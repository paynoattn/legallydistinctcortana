mod os;

use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use cpal::{FromSample, Sample};
use speech_application::CancelFlag;
use speech_application::ports::{AudioOutput, PlaybackError};
use speech_domain::AudioClip;

const IDLE_POLL: Duration = Duration::from_millis(15);
const DRAIN_TAIL: Duration = Duration::from_millis(120);
const MICROS_PER_SECOND: u64 = 1_000_000;

type Queue = Arc<Mutex<VecDeque<f32>>>;
type Broken = Arc<AtomicBool>;

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
        let format = os::open_device(Arc::clone(&self.queue), Arc::clone(&broken))?;
        let open = OpenDevice { format, broken };
        *device = Some(open.clone());
        Ok(open)
    }
}

impl AudioOutput for SpeakerOutput {
    fn enqueue(&self, clip: AudioClip) -> Result<Duration, PlaybackError> {
        let device = self.device()?;
        let samples = clip.resampled(device.format.rate).into_samples();
        let ahead = {
            let mut queue = lock(&self.queue);
            let ahead = queued_for(queue.len(), device.format.rate);
            queue.extend(samples);
            ahead
        };
        if device.broken.load(Ordering::SeqCst) {
            lock(&self.queue).clear();
            return Err(PlaybackError(
                "the speakers stopped working; try again to reopen them".into(),
            ));
        }
        Ok(ahead)
    }

    fn wait_until_idle(&self, cancel: &CancelFlag) {
        wait_for_drain(&self.queue, cancel, IDLE_POLL);
        thread::sleep(DRAIN_TAIL);
    }

    fn stop(&self) {
        lock(&self.queue).clear();
    }
}

/// How long `samples` mono samples last at `rate` samples a second, to the microsecond.
fn queued_for(samples: usize, rate: u32) -> Duration {
    let samples = u64::try_from(samples).unwrap_or(u64::MAX);
    Duration::from_micros(samples.saturating_mul(MICROS_PER_SECOND) / u64::from(rate.max(1)))
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

fn playback_error(error: impl fmt::Display) -> PlaybackError {
    PlaybackError(error.to_string())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
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
    fn enqueue_reports_the_audio_already_queued_at_the_device_rate() {
        let speaker = speaker_at(2_000);
        let clip = AudioClip::from_samples(1_000, vec![0.1; 1_000]).unwrap();
        assert_eq!(speaker.enqueue(clip.clone()), Ok(Duration::ZERO));
        assert_eq!(speaker.enqueue(clip), Ok(Duration::from_secs(1)));
    }

    #[test]
    fn queued_audio_is_measured_to_the_microsecond() {
        assert_eq!(queued_for(3, 2_000), Duration::from_micros(1_500));
        assert_eq!(queued_for(44_100, 44_100), Duration::from_secs(1));
        assert_eq!(queued_for(5, 0), Duration::from_secs(5));
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
