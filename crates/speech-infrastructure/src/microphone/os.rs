use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, Stream, StreamConfig};
use speech_application::ports::CaptureError;

use super::{Capture, Recording, Shared, capture_error, lock, open_with, require_float};

pub(super) fn open_default_input() -> Result<Capture, CaptureError> {
    open_with(start_stream)
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
