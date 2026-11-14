//! Desktop adapters for `platform-ports`, built on cross-platform crates with safe APIs.
//!
//! [`ClipboardSelectionReader`] reads the selection in any app by asking it to copy, and
//! [`ClipboardTextInserter`] types text into any app by pasting it. Both put every format of
//! the user's clipboard back afterwards, or refuse to start if they cannot. [`ArboardClipboard`]
//! and [`EnigoKeyboard`] are the thin system bindings they run on; everything with a decision
//! in it is tested against fakes.

mod clipboard_insertion;
mod clipboard_selection;
mod os;

pub use clipboard_insertion::{ClipboardTextInserter, PASTE_SETTLE};
pub use clipboard_selection::{
    COPY_TIMEOUT, ClipboardAccess, ClipboardSelectionReader, ClipboardSnapshot, KeyboardAccess,
    POLL_INTERVAL, RawFormat, Timing,
};
pub use os::{ArboardClipboard, EnigoKeyboard};
