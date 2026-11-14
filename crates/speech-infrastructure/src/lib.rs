//! Speech adapters.
//!
//! - [`WyomingSynthesizer`]: Piper (or any Wyoming TTS server) over TCP, for example the
//!   `rhasspy/wyoming-piper` container on another machine on your LAN or tailnet.
//! - [`SpeakerOutput`]: the default output device through `cpal`.
//! - [`WyomingConfig`]: server address from the same environment variables as `voice.py`
//!   (`VOICE_HOST`, `VOICE_TTS_PORT`, plus `VOICE_TTS_VOICE`).

mod config;
#[cfg(test)]
mod read_aloud_flow;
mod speaker;
mod wyoming;

pub use config::{ConfigError, DEFAULT_HOST, DEFAULT_TTS_PORT, WyomingConfig};
pub use speaker::SpeakerOutput;
pub use wyoming::WyomingSynthesizer;
