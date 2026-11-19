//! Settings value types. Pure: no I/O.
//!
//! - [`Settings`]: every setting, each valid, built from [`Settings::defaults`] (which
//!   reads the same environment variables the app used before it had settings) and changed
//!   with [`SettingsInput`].
//! - [`Chord`]: a keyboard shortcut such as `Ctrl+Alt+R`, refusing chords Windows reserves.

mod chord;
mod settings;

pub use chord::{Chord, ChordError};
pub use settings::{
    DEFAULT_DICTATE_SHORTCUT, DEFAULT_READ_ALOUD_SHORTCUT, DEFAULT_STT_LANGUAGE, DEFAULT_STT_PORT,
    DEFAULT_TTS_PORT, DEFAULT_VOICE_HOST, Field, FieldError, MAX_HOST_CHARS, Settings,
    SettingsInput,
};
