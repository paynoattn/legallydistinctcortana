mod os;

use std::fmt;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::SampleFormat;
use speech_application::ports::{AudioInput, CaptureError};
use speech_domain::AudioClip;

/// Longest recording kept. Speech after this is dropped, so a key that never reports its
/// release cannot fill memory.
pub const MAX_RECORDING: Duration = Duration::from_secs(300);

/// Mono samples recorded so far from an input of `channels` interleaved channels.
#[derive(Debug)]
struct Recording {
    rate: u32,
    channels: u16,
    samples: Vec<f32>,
    limit: usize,
}

impl Recording {
    fn new(rate: u32, channels: u16) -> Self {
        let limit = u64::from(rate).saturating_mul(MAX_RECORDING.as_secs());
        Self {
            rate,
            channels,
            samples: Vec::new(),
            limit: usize::try_from(limit).unwrap_or(usize::MAX),
        }
    }

    /// Adds interleaved input, averaging each frame's channels, up to the length limit.
    fn push(&mut self, interleaved: &[f32]) {
        let room = self.limit.saturating_sub(self.samples.len());
        let frames = interleaved.chunks(usize::from(self.channels.max(1)));
        self.samples.extend(frames.map(mean).take(room));
    }

    /// Everything recorded so far, leaving the recording empty.
    fn take_clip(&mut self) -> Result<AudioClip, CaptureError> {
        AudioClip::from_samples(self.rate, std::mem::take(&mut self.samples))
            .map_err(|error| CaptureError(error.to_string()))
    }
}

fn mean(frame: &[f32]) -> f32 {
    let count = u16::try_from(frame.len()).map_or(f32::from(u16::MAX), f32::from);
    frame.iter().sum::<f32>() / count.max(1.0)
}

type Shared = Arc<Mutex<Recording>>;

type Ready = Sender<Result<Shared, CaptureError>>;

/// Opens an input stream with `start` on a dedicated thread, which keeps the stream (not
/// `Send` on every platform) alive until the capture is finished.
fn open_with<S>(
    start: impl FnOnce() -> Result<(S, Shared), CaptureError> + Send + 'static,
) -> Result<Capture, CaptureError> {
    let (ready, opened) = mpsc::channel();
    let (stop, stopped) = mpsc::channel();
    let holder = thread::Builder::new()
        .name("microphone-input".into())
        .spawn(move || run_device(start, &stopped, &ready))
        .map_err(capture_error)?;
    let recording = opened.recv().map_err(|_| {
        CaptureError("the microphone thread stopped before opening the device".into())
    })??;
    Ok(Capture {
        recording,
        stop,
        holder,
    })
}

fn run_device<S>(
    start: impl FnOnce() -> Result<(S, Shared), CaptureError>,
    stopped: &Receiver<()>,
    ready: &Ready,
) {
    match start() {
        Ok((stream, recording)) => {
            if ready.send(Ok(recording)).is_ok() {
                hold_until_stopped(stream, stopped);
            }
        }
        Err(error) => drop(ready.send(Err(error))),
    }
}

/// Keeps `stream` open until the capture's stop sender is dropped.
fn hold_until_stopped<S>(stream: S, stopped: &Receiver<()>) {
    stopped.recv().unwrap_or_default();
    drop(stream);
}

/// An open input stream: the recording it fills, and the thread that holds it open until
/// `stop` is dropped.
#[derive(Debug)]
struct Capture {
    recording: Shared,
    stop: Sender<()>,
    holder: JoinHandle<()>,
}

impl Capture {
    /// Closes the stream, waits until its last buffer is in, and returns the recording.
    fn finish(self) -> Result<AudioClip, CaptureError> {
        let Self {
            recording,
            stop,
            holder,
        } = self;
        drop(stop);
        holder
            .join()
            .map_err(|_| CaptureError("the microphone thread stopped unexpectedly".into()))?;
        lock(&recording).take_clip()
    }
}

type Opener = fn() -> Result<Capture, CaptureError>;

/// The default microphone, through `cpal`.
///
/// The device is opened on [`AudioInput::start`] and closed on [`AudioInput::stop`], so the
/// microphone is only in use while the user dictates. Channels are averaged to mono at the
/// device's own rate; recordings are capped at [`MAX_RECORDING`].
pub struct MicrophoneInput {
    open: Opener,
    active: Mutex<Option<Capture>>,
}

impl Default for MicrophoneInput {
    fn default() -> Self {
        Self::with_opener(os::open_default_input)
    }
}

impl fmt::Debug for MicrophoneInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MicrophoneInput")
            .field("recording", &lock(&self.active).is_some())
            .finish_non_exhaustive()
    }
}

impl MicrophoneInput {
    const fn with_opener(open: Opener) -> Self {
        Self {
            open,
            active: Mutex::new(None),
        }
    }
}

impl AudioInput for MicrophoneInput {
    fn start(&self) -> Result<(), CaptureError> {
        let mut active = lock(&self.active);
        if let Some(stale) = active.take() {
            drop(stale.finish());
        }
        *active = Some((self.open)()?);
        Ok(())
    }

    fn stop(&self) -> Result<AudioClip, CaptureError> {
        lock(&self.active)
            .take()
            .ok_or_else(|| CaptureError("the microphone was not recording".into()))?
            .finish()
    }
}

/// Only 32-bit float input is read; that is what shared-mode Windows audio delivers.
fn require_float(format: SampleFormat) -> Result<(), CaptureError> {
    if format == SampleFormat::F32 {
        return Ok(());
    }
    Err(CaptureError(format!(
        "the microphone uses {format} samples; only 32-bit float is supported"
    )))
}

fn capture_error(error: impl fmt::Display) -> CaptureError {
    CaptureError(error.to_string())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;

    fn recorded(mut recording: Recording) -> Vec<f32> {
        recording.take_clip().unwrap().into_samples()
    }

    #[test]
    fn frames_are_averaged_to_mono() {
        let mut recording = Recording::new(48_000, 2);
        recording.push(&[0.5, 0.25, -1.0, 0.0]);
        recording.push(&[1.0, 1.0]);
        assert_eq!(recorded(recording), [0.375, -0.5, 1.0]);
    }

    #[test]
    fn mono_input_is_kept_as_it_is() {
        let mut recording = Recording::new(16_000, 1);
        recording.push(&[0.1, 0.2, 0.3]);
        let clip = recording.take_clip().unwrap();
        assert_eq!(clip.sample_rate(), 16_000);
        assert!(
            recording.take_clip().unwrap().samples().is_empty(),
            "taken once"
        );
        assert_eq!(clip.samples(), [0.1, 0.2, 0.3]);
    }

    #[test]
    fn a_frame_cut_short_averages_what_arrived() {
        let mut recording = Recording::new(1_000, 3);
        recording.push(&[0.3, 0.3, 0.3, 0.6]);
        assert_eq!(recorded(recording), [0.3, 0.6]);
    }

    #[test]
    fn recordings_stop_growing_at_five_minutes() {
        assert_eq!(MAX_RECORDING, Duration::from_secs(300));
        let mut recording = Recording::new(10, 1);
        assert_eq!(recording.limit, 3_000);
        recording.push(&vec![0.5; 2_999]);
        recording.push(&[0.1, 0.2, 0.3]);
        let samples = recorded(recording);
        assert_eq!(samples.len(), 3_000);
        assert_eq!(samples.last(), Some(&0.1));
    }

    #[test]
    fn a_device_without_a_rate_cannot_make_a_recording() {
        assert_eq!(
            Recording::new(0, 1).take_clip(),
            Err(CaptureError("sample rate must be above zero".into()))
        );
    }

    #[test]
    fn only_float_input_is_accepted() {
        assert_eq!(require_float(SampleFormat::F32), Ok(()));
        assert_eq!(
            require_float(SampleFormat::I16),
            Err(CaptureError(
                "the microphone uses i16 samples; only 32-bit float is supported".into()
            ))
        );
    }

    static OPENED: AtomicUsize = AtomicUsize::new(0);

    struct FakeStream {
        sink: Shared,
        dropped: Arc<AtomicBool>,
    }

    impl Drop for FakeStream {
        fn drop(&mut self) {
            lock(&self.sink).push(&[0.25, 0.25]);
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it stands in for opening a stream, which can fail"
    )]
    fn fake_stream(dropped: Arc<AtomicBool>) -> Result<(FakeStream, Shared), CaptureError> {
        let recording = Arc::new(Mutex::new(Recording::new(1_000, 2)));
        lock(&recording).push(&[0.5, 0.5]);
        let stream = FakeStream {
            sink: Arc::clone(&recording),
            dropped,
        };
        Ok((stream, recording))
    }

    fn fake_device() -> Result<Capture, CaptureError> {
        OPENED.fetch_add(1, Ordering::SeqCst);
        open_with(|| fake_stream(Arc::default()))
    }

    #[test]
    fn the_stream_stays_open_until_the_capture_is_finished() {
        let dropped = Arc::new(AtomicBool::new(false));
        let watched = Arc::clone(&dropped);
        let capture = open_with(move || fake_stream(watched)).unwrap();
        thread::sleep(Duration::from_millis(50));
        assert!(!dropped.load(Ordering::SeqCst), "still recording");
        assert_eq!(capture.finish().unwrap().samples(), [0.5, 0.25]);
        assert!(dropped.load(Ordering::SeqCst), "closed when finished");
    }

    #[test]
    fn a_device_that_fails_to_open_reports_why() {
        let opened = open_with(|| -> Result<((), Shared), CaptureError> {
            Err(CaptureError("no microphone is available".into()))
        });
        assert_eq!(
            opened.unwrap_err(),
            CaptureError("no microphone is available".into())
        );
    }

    #[test]
    fn a_device_thread_that_dies_while_opening_is_reported() {
        let opened =
            open_with(|| -> Result<((), Shared), CaptureError> { panic!("driver crashed") });
        assert_eq!(
            opened.unwrap_err(),
            CaptureError("the microphone thread stopped before opening the device".into())
        );
    }

    fn no_device() -> Result<Capture, CaptureError> {
        Err(CaptureError("no microphone is available".into()))
    }

    #[test]
    fn stop_closes_the_stream_and_keeps_its_last_buffer() {
        let microphone = MicrophoneInput::with_opener(fake_device);
        microphone.start().unwrap();
        let clip = microphone.stop().unwrap();
        assert_eq!(clip.sample_rate(), 1_000);
        assert_eq!(clip.samples(), [0.5, 0.25]);
    }

    #[test]
    fn stopping_twice_reports_that_nothing_is_recording() {
        let microphone = MicrophoneInput::with_opener(fake_device);
        assert_eq!(
            microphone.stop(),
            Err(CaptureError("the microphone was not recording".into()))
        );
        microphone.start().unwrap();
        microphone.stop().unwrap();
        assert!(microphone.stop().is_err(), "the recording was handed out");
    }

    #[test]
    fn starting_again_drops_the_old_recording() {
        let microphone = MicrophoneInput::with_opener(fake_device);
        let before = OPENED.load(Ordering::SeqCst);
        microphone.start().unwrap();
        microphone.start().unwrap();
        assert!(OPENED.load(Ordering::SeqCst) >= before + 2);
        assert_eq!(microphone.stop().unwrap().samples(), [0.5, 0.25]);
    }

    #[test]
    fn a_missing_microphone_fails_the_start() {
        let microphone = MicrophoneInput::with_opener(no_device);
        assert_eq!(
            microphone.start(),
            Err(CaptureError("no microphone is available".into()))
        );
        assert_eq!(
            format!("{microphone:?}"),
            "MicrophoneInput { recording: false, .. }"
        );
    }

    #[test]
    fn debug_output_shows_whether_it_is_recording() {
        let microphone = MicrophoneInput::with_opener(fake_device);
        microphone.start().unwrap();
        assert_eq!(
            format!("{microphone:?}"),
            "MicrophoneInput { recording: true, .. }"
        );
        microphone.stop().unwrap();
    }

    #[test]
    fn a_holder_thread_that_panics_is_reported() {
        let recording = Arc::new(Mutex::new(Recording::new(1_000, 1)));
        let capture = Capture {
            recording,
            stop: mpsc::channel().0,
            holder: thread::spawn(|| panic!("device lost")),
        };
        assert_eq!(
            capture.finish(),
            Err(CaptureError(
                "the microphone thread stopped unexpectedly".into()
            ))
        );
    }

    #[test]
    fn errors_are_described_by_their_message() {
        assert_eq!(
            capture_error("device busy"),
            CaptureError("device busy".into())
        );
    }
}
