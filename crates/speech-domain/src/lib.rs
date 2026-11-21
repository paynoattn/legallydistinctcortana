//! Speech value objects. Pure: no I/O, no async runtime, no audio devices.
//!
//! - [`ReadingText`]: validated text to read aloud, split into sentences for streaming.
//! - [`AudioClip`]: mono samples at a sample rate, with resampling, speed, tempo, gain, and
//!   silence.
//! - [`Prosody`]: how a [`kernel::Mood`] is approximated by speed, loudness, and pauses, and
//!   the listener's reading speed.
//! - [`OverlayState`]: what the corner indicator shows, and how events move it.
//! - [`ConnectionStatus`]: whether the voice servers answer, in one sentence.

mod audio;
mod connection;
mod dsp;
mod overlay;
mod prosody;
mod tempo;
mod text;
mod voice;

pub use audio::{AudioClip, AudioError, MAX_SAMPLE_RATE};
pub use connection::{ConnectionKind, ConnectionStatus};
pub use overlay::{
    ERROR_LINGER, Fix, INSERTED_LINGER, LOUD_DB, OverlayError, OverlayEvent, OverlayState,
    QUIET_DB, Stage, level_from_rms,
};
pub use prosody::{MAX_TEMPO_PER_MILLE, MIN_TEMPO_PER_MILLE, Prosody};
pub use text::{MAX_READING_CHARS, MAX_SENTENCE_CHARS, ReadingText, Sentence, TextError};
pub use voice::{Voice, VoiceChoice};
