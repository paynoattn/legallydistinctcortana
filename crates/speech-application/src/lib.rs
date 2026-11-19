//! Speech use cases: read selected text aloud ([`ReadAloudService`]) and push-to-talk
//! dictation ([`DictationService`]). Talks to the outside world only through the ports in
//! [`ports`] and the [`platform_ports::SelectionReader`] and [`platform_ports::TextInserter`].

mod cancel;
mod dictation;
mod overlay;
pub mod ports;
mod push_to_talk;
mod service;

pub use cancel::CancelFlag;
pub use dictation::{
    DictationError, DictationOutcome, DictationService, MIN_SPEECH, PressOutcome, Utterance,
};
pub use overlay::{
    Delay, OverlayController, OverlaySurface, finish_event, overlay_error, press_event,
};
pub use push_to_talk::{DictationHost, PushToTalk};
pub use service::{
    PREVIEW_CHARS, ReadAloudError, ReadAloudService, ReadOutcome, ReadingTicket, SelectionPreview,
    ShortcutOutcome,
};
pub use speech_domain::{
    ConnectionKind, ConnectionStatus, Fix, OverlayError, OverlayEvent, OverlayState, Stage, Voice,
    VoiceChoice,
};
