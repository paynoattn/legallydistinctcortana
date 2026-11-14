//! Desktop adapters for `platform-ports`, built on cross-platform crates with safe APIs.
//!
//! [`ClipboardSelectionReader`] reads the selection in any app by asking it to copy, then
//! puts every format of the user's clipboard back, or refuses to start if it cannot. [`ArboardClipboard`] and [`EnigoKeyboard`] are the thin
//! system bindings it runs on; everything with a decision in it is tested against fakes.

mod clipboard_selection;
mod os;

pub use clipboard_selection::{
    COPY_TIMEOUT, ClipboardAccess, ClipboardSelectionReader, ClipboardSnapshot, KeyboardAccess,
    POLL_INTERVAL, RawFormat, Timing,
};
pub use os::{ArboardClipboard, EnigoKeyboard};
