use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, Stream, StreamConfig};
use speech_application::CancelFlag;
use speech_application::ports::{AudioOutput, PlaybackError};
use speech_domain::AudioClip;

const IDLE_POLL: Duration = Duration::from_millis(15);
const DRAIN_TAIL: Duration = Duration::from_millis(120);
const RETIRE_POLL: Duration = Duration::from_millis(250);

type Queue = Arc<Mutex<VecDeque<f32>>>;
type Broken = Arc<AtomicBool>;
type Ready = Sender<Result<DeviceFormat, PlaybackError>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeviceFormat {
    rate: u32,
    channels: u16,
}

/// An opened output stream: its format, and the flag that marks it failed. Each stream has
/// its own flag; once set, the stream's thread drops it and a new one is opened on demand.
#[derive(Debug, Clone)]
struct OpenDevice {
    format: DeviceFormat,
    broken: Broken,
}

/// The default speakers or headphones, through `cpal`.
///
/// The device is opened on first use on a dedicated thread that keeps the stream alive;
/// queued audio is resampled to the device rate and played on every channel. If opening
/// fails (no device), the next [`AudioOutput::enqueue`] tries again. If the device fails while
/// playing (unplugged headphones, default device changed), queued audio is dropped so waiters
/// return, the failed stream is shut down, and the next [`AudioOutput::enqueue`] opens the
/// default device again.
#[derive(Default)]
pub struct SpeakerOutput {
    queue: Queue,
    device: Mutex<Option<OpenDevice>>,
}

impl fmt::Debug for SpeakerOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpeakerOutput").finish_non_exhaustive()
    }
}

impl SpeakerOutput {
    fn device(&self) -> Result<OpenDevice, PlaybackError> {
        let mut device = lock(&self.device);
        if let Some(open) = device
            .as_ref()
            .filter(|open| !open.broken.load(Ordering::SeqCst))
        {
            return Ok(open.clone());
        }
        let broken = Broken::default();
        let format = open_device(Arc::clone(&self.queue), Arc::clone(&broken))?;
        let open = OpenDevice { format, broken };
        *device = Some(open.clone());
        Ok(open)
    }
}

impl AudioOutput for SpeakerOutput {
    fn enqueue(&self, clip: AudioClip) -> Result<(), PlaybackError> {
        let device = self.device()?;
        lock(&self.queue).extend(clip.resampled(device.format.rate).into_samples());
        if device.broken.load(Ordering::SeqCst) {
            lock(&self.queue).clear();
            return Err(PlaybackError(
                "the speakers stopped working; try again to reopen them".into(),
            ));
        }
        Ok(())
    }

    fn wait_until_idle(&self, cancel: &CancelFlag) {
        wait_for_drain(&self.queue, cancel, IDLE_POLL);
        thread::sleep(DRAIN_TAIL);
    }

    fn stop(&self) {
        lock(&self.queue).clear();
    }
}

fn wait_for_drain(queue: &Queue, cancel: &CancelFlag, poll: Duration) {
    while !cancel.is_cancelled() && !lock(queue).is_empty() {
        thread::sleep(poll);
    }
}

fn fill<T: Sample + FromSample<f32>>(output: &mut [T], channels: u16, queue: &Queue) {
    let mut pending = lock(queue);
    for frame in output.chunks_mut(usize::from(channels.max(1))) {
        frame.fill(T::from_sample(pending.pop_front().unwrap_or(0.0)));
    }
}

fn open_device(queue: Queue, broken: Broken) -> Result<DeviceFormat, PlaybackError> {
    let (ready, opened) = mpsc::channel();
    thread::Builder::new()
        .name("speaker-output".into())
        .spawn(move || run_device(queue, &broken, &ready))
        .map_err(|error| PlaybackError(error.to_string()))?;
    opened
        .recv()
        .map_err(|_| PlaybackError("the audio thread stopped before opening the device".into()))?
}

fn run_device(queue: Queue, broken: &Broken, ready: &Ready) {
    match start_stream(queue, Arc::clone(broken)) {
        Ok(opened) => keep_alive(opened, ready, broken),
        Err(error) => report(ready, Err(error)),
    }
}

fn keep_alive((stream, format): (Stream, DeviceFormat), ready: &Ready, broken: &AtomicBool) {
    if ready.send(Ok(format)).is_ok() {
        hold_until_broken(stream, broken);
    }
}

fn hold_until_broken(stream: Stream, broken: &AtomicBool) {
    while !broken.load(Ordering::SeqCst) {
        thread::park_timeout(RETIRE_POLL);
    }
    drop(stream);
}

fn report(ready: &Ready, result: Result<DeviceFormat, PlaybackError>) {
    if ready.send(result).is_err() {
        tracing::warn!("nobody was waiting for the audio device result");
    }
}

fn start_stream(queue: Queue, broken: Broken) -> Result<(Stream, DeviceFormat), PlaybackError> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or_else(|| PlaybackError("no speakers or headphones are available".into()))?;
    let supported = device.default_output_config().map_err(playback_error)?;
    if supported.sample_format() != SampleFormat::F32 {
        return Err(PlaybackError(format!(
            "the output device uses {} samples; only 32-bit float is supported",
            supported.sample_format()
        )));
    }
    let config: StreamConfig = supported.config();
    let format = DeviceFormat {
        rate: config.sample_rate,
        channels: config.channels,
    };
    let channels = format.channels;
    let pending = Arc::clone(&queue);
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| fill(data, channels, &queue),
            move |error| {
                tracing::warn!(%error, "speaker stream error; it will be reopened");
                broken.store(true, Ordering::SeqCst);
                lock(&pending).clear();
            },
            None,
        )
        .map_err(playback_error)?;
    stream.play().map_err(playback_error)?;
    Ok((stream, format))
}

fn playback_error(error: impl fmt::Display) -> PlaybackError {
    PlaybackError(error.to_string())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    fn queue(samples: &[f32]) -> Queue {
        Arc::new(Mutex::new(samples.iter().copied().collect()))
    }

    fn speaker_at(rate: u32) -> SpeakerOutput {
        SpeakerOutput {
            queue: Queue::default(),
            device: Mutex::new(Some(OpenDevice {
                format: DeviceFormat { rate, channels: 2 },
                broken: Broken::default(),
            })),
        }
    }

    #[test]
    fn fill_copies_each_sample_to_every_channel_then_silence() {
        let pending = queue(&[0.5, -0.5]);
        let mut output = [9.0_f32; 6];
        fill(&mut output, 2, &pending);
        assert_eq!(output, [0.5, 0.5, -0.5, -0.5, 0.0, 0.0]);
        assert!(lock(&pending).is_empty(), "everything queued was played");
    }

    #[test]
    fn fill_converts_to_integer_samples_and_survives_zero_channels() {
        let mut ints = [0_i16; 2];
        fill(&mut ints, 0, &queue(&[0.5, -1.0]));
        assert_eq!(ints, [16_384, -32_768]);
    }

    #[test]
    fn enqueue_resamples_to_the_device_rate() {
        let speaker = speaker_at(2_000);
        speaker
            .enqueue(AudioClip::from_samples(1_000, vec![0.0, 1.0]).unwrap())
            .unwrap();
        assert_eq!(
            lock(&speaker.queue).iter().copied().collect::<Vec<_>>(),
            [0.0, 0.5, 1.0, 1.0]
        );
    }

    #[test]
    fn enqueue_drops_the_audio_and_fails_when_the_stream_breaks_while_it_queues() {
        let speaker = Arc::new(speaker_at(1_000));
        let broken = Arc::clone(&lock(&speaker.device).as_ref().unwrap().broken);
        let held = lock(&speaker.queue);
        let writer = Arc::clone(&speaker);
        let enqueue = thread::spawn(move || {
            writer.enqueue(AudioClip::from_samples(1_000, vec![0.1; 10]).unwrap())
        });
        thread::sleep(Duration::from_millis(300));
        broken.store(true, Ordering::SeqCst);
        drop(held);
        assert_eq!(
            enqueue.join().unwrap(),
            Err(PlaybackError(
                "the speakers stopped working; try again to reopen them".into()
            ))
        );
        assert!(lock(&speaker.queue).is_empty(), "the audio was dropped");
    }

    #[test]
    fn a_healthy_stream_is_reused_and_keeps_its_audio() {
        let speaker = speaker_at(1_000);
        let clip = AudioClip::from_samples(1_000, vec![0.1; 4]).unwrap();
        speaker.enqueue(clip.clone()).unwrap();
        speaker.enqueue(clip).unwrap();
        assert_eq!(lock(&speaker.queue).len(), 8);
        assert!(
            !lock(&speaker.device)
                .as_ref()
                .unwrap()
                .broken
                .load(Ordering::SeqCst)
        );
    }

    #[test]
    fn stop_drops_everything_queued() {
        let speaker = speaker_at(1_000);
        speaker
            .enqueue(AudioClip::from_samples(1_000, vec![0.1; 10]).unwrap())
            .unwrap();
        speaker.stop();
        assert!(lock(&speaker.queue).is_empty(), "stop clears the queue");
    }

    fn finishes_quickly(work: impl FnOnce() + Send + 'static) -> bool {
        let (done, finished) = mpsc::channel();
        thread::spawn(move || {
            work();
            let _ = done.send(());
        });
        finished.recv_timeout(Duration::from_secs(1)).is_ok()
    }

    #[test]
    fn waiting_returns_at_once_when_nothing_is_queued() {
        assert!(finishes_quickly(|| {
            wait_for_drain(&queue(&[]), &CancelFlag::new(), Duration::from_millis(1));
        }));
    }

    #[test]
    fn waiting_returns_at_once_when_cancelled_with_audio_still_queued() {
        let cancel = CancelFlag::new();
        cancel.cancel();
        assert!(finishes_quickly(move || {
            wait_for_drain(&queue(&[0.1]), &cancel, Duration::from_millis(1));
        }));
    }

    #[test]
    fn waiting_polls_until_the_device_drains_the_queue() {
        let pending = queue(&[0.1]);
        let player = Arc::clone(&pending);
        let drain = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            lock(&player).clear();
        });
        let waiter = Arc::clone(&pending);
        assert!(finishes_quickly(move || {
            wait_for_drain(&waiter, &CancelFlag::new(), Duration::from_millis(2));
        }));
        assert!(lock(&pending).is_empty(), "returned only after draining");
        drain.join().unwrap();
    }

    #[test]
    fn going_idle_waits_for_the_drain_and_then_the_tail() {
        let speaker = Arc::new(speaker_at(1_000));
        lock(&speaker.queue).push_back(0.1);
        let player = Arc::clone(&speaker.queue);
        let drain = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            lock(&player).clear();
        });
        let waiter = Arc::clone(&speaker);
        let started = Instant::now();
        assert!(finishes_quickly(move || {
            waiter.wait_until_idle(&CancelFlag::new());
        }));
        assert!(lock(&speaker.queue).is_empty(), "drained before returning");
        assert!(
            started.elapsed() >= Duration::from_millis(150),
            "{:?}",
            started.elapsed()
        );
        drain.join().unwrap();
    }

    #[test]
    fn going_idle_with_nothing_queued_still_waits_out_the_tail() {
        let started = Instant::now();
        assert!(finishes_quickly(|| {
            speaker_at(1_000).wait_until_idle(&CancelFlag::new());
        }));
        assert!(started.elapsed() >= Duration::from_millis(100));
    }

    #[test]
    fn debug_does_not_dump_the_queue() {
        assert_eq!(
            format!("{:?}", SpeakerOutput::default()),
            "SpeakerOutput { .. }"
        );
    }

    #[test]
    fn errors_keep_the_device_message() {
        assert_eq!(
            playback_error("device lost"),
            PlaybackError("device lost".into())
        );
    }
}
