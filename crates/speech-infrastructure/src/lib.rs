//! Speech adapters.
//!
//! - [`WyomingSynthesizer`]: Piper (or any Wyoming TTS server) over TCP, for example the
//!   `rhasspy/wyoming-piper` container on another machine on your LAN or tailnet.
//! - [`WyomingTranscriber`]: Whisper (or any Wyoming STT server) over TCP, for example the
//!   `rhasspy/wyoming-whisper` container next to Piper.
//! - [`SpeakerOutput`]: the default output device through `cpal`.
//! - [`MicrophoneInput`]: the default input device through `cpal`, open only while recording.
//! - [`WyomingConfig`] and [`WyomingSttConfig`]: server addresses from the same environment
//!   variables as `voice.py` (`VOICE_HOST`, `VOICE_TTS_PORT`, `VOICE_STT_PORT`, plus
//!   `VOICE_TTS_VOICE` and `VOICE_STT_LANGUAGE`).

mod config;
#[cfg(test)]
mod dictation_flow;
mod microphone;
#[cfg(test)]
mod read_aloud_flow;
mod speaker;
mod wyoming;

pub use config::{
    AUTO_LANGUAGE, ConfigError, DEFAULT_HOST, DEFAULT_STT_LANGUAGE, DEFAULT_STT_PORT,
    DEFAULT_TTS_PORT, WyomingConfig, WyomingSttConfig,
};
pub use microphone::{MAX_RECORDING, MicrophoneInput};
pub use speaker::SpeakerOutput;
pub use wyoming::{WyomingSynthesizer, WyomingTranscriber};
