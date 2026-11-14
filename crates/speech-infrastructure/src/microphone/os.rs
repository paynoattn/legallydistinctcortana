use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, Stream, StreamConfig};
use speech_application::ports::CaptureError;

use super::{Capture, Recording, Shared, capture_error, lock, require_float};

const STOP_POLL: Duration = Duration::from_millis(250);

type Ready = Sender<Result<Shared, CaptureError>>;

pub(super) fn open_default_input() -> Result<Capture, CaptureError> {
    let (ready, opened) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let holder = thread::Builder::new()
        .name("microphone-input".into())
        .spawn(move || run_device(&flag, &ready))
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

fn run_device(stop: &AtomicBool, ready: &Ready) {
    match start_stream() {
        Ok(opened) => keep_alive(opened, ready, stop),
        Err(error) => report(ready, Err(error)),
    }
}

fn keep_alive((stream, recording): (Stream, Shared), ready: &Ready, stop: &AtomicBool) {
    if ready.send(Ok(recording)).is_ok() {
        hold_until_stopped(stream, stop);
    }
}

fn hold_until_stopped(stream: Stream, stop: &AtomicBool) {
    while !stop.load(Ordering::SeqCst) {
        thread::park_timeout(STOP_POLL);
    }
    drop(stream);
}

fn report(ready: &Ready, result: Result<Shared, CaptureError>) {
    if ready.send(result).is_err() {
        tracing::warn!("nobody was waiting for the microphone result");
    }
}

fn start_stream() -> Result<(Stream, Shared), CaptureError> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| CaptureError("no microphone is available".into()))?;
    let supported = device.default_input_config().map_err(capture_error)?;
    require_float(supported.sample_format())?;
    let config: StreamConfig = supported.config();
    let recording = Arc::new(Mutex::new(Recording::new(
        config.sample_rate,
        config.channels,
    )));
    let sink = Arc::clone(&recording);
    let stream = device
        .build_input_stream(
            config,
            move |data: &[f32], _: &InputCallbackInfo| lock(&sink).push(data),
            |error| tracing::warn!(%error, "microphone stream error"),
            None,
        )
        .map_err(capture_error)?;
    stream.play().map_err(capture_error)?;
    Ok((stream, recording))
}
