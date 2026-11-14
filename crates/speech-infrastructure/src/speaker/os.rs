use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use speech_application::ports::PlaybackError;

use super::{Broken, DeviceFormat, Queue, fill, lock, playback_error};

const RETIRE_POLL: Duration = Duration::from_millis(250);

type Ready = Sender<Result<DeviceFormat, PlaybackError>>;

pub(super) fn open_device(queue: Queue, broken: Broken) -> Result<DeviceFormat, PlaybackError> {
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
