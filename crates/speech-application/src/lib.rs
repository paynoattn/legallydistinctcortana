//! Read-aloud use cases. Talks to the outside world only through the ports in [`ports`] and
//! the [`platform_ports::SelectionReader`].

mod cancel;
pub mod ports;
mod service;

pub use cancel::CancelFlag;
pub use service::{
    PREVIEW_CHARS, ReadAloudError, ReadAloudService, ReadOutcome, ReadingTicket, SelectionPreview,
    ShortcutOutcome,
};
pub use speech_domain::{Voice, VoiceChoice};
