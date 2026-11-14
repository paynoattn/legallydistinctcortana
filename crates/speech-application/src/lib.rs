//! Speech use cases: read selected text aloud ([`ReadAloudService`]) and push-to-talk
//! dictation ([`DictationService`]). Talks to the outside world only through the ports in
//! [`ports`] and the [`platform_ports::SelectionReader`] and [`platform_ports::TextInserter`].

mod cancel;
mod dictation;
pub mod ports;
mod service;

pub use cancel::CancelFlag;
pub use dictation::{
    DictationError, DictationOutcome, DictationService, MIN_SPEECH, PressOutcome, Utterance,
};
pub use service::{
    PREVIEW_CHARS, ReadAloudError, ReadAloudService, ReadOutcome, ReadingTicket, SelectionPreview,
    ShortcutOutcome,
};
pub use speech_domain::{Voice, VoiceChoice};
