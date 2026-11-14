//! Speech value objects. Pure: no I/O, no async runtime, no audio devices.
//!
//! - [`ReadingText`]: validated text to read aloud, split into sentences for streaming.
//! - [`AudioClip`]: mono samples at a sample rate, with resampling, speed, gain, and silence.
//! - [`Prosody`]: how a [`kernel::Mood`] is approximated by speed, loudness, and pauses.

mod audio;
mod dsp;
mod prosody;
mod text;
mod voice;

pub use audio::{AudioClip, AudioError, MAX_SAMPLE_RATE};
pub use prosody::Prosody;
pub use text::{MAX_READING_CHARS, MAX_SENTENCE_CHARS, ReadingText, TextError};
pub use voice::{Voice, VoiceChoice};
