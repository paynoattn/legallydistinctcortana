use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::{ClipboardAccess, KeyboardAccess, RawFormat};

const MODIFIER_SETTLE: Duration = Duration::from_millis(15);

/// The system clipboard: text through `arboard`, every format through `clipboard-win`.
/// Opens the clipboard per call, so it is cheap to share and never holds the clipboard open.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArboardClipboard;

impl ClipboardAccess for ArboardClipboard {
    fn read_formats(&self) -> Result<Vec<RawFormat>, String> {
        native::read_formats()
    }

    fn write_formats(&self, formats: &[(u32, Vec<u8>)]) -> Result<(), String> {
        native::write_formats(formats)
    }

    fn text(&self) -> Option<String> {
        Clipboard::new()
            .and_then(|mut clipboard| clipboard.get_text())
            .ok()
    }

    fn set_text(&self, text: &str) -> Result<(), String> {
        Clipboard::new()
            .and_then(|mut clipboard| set_hidden_text(&mut clipboard, text))
            .map_err(|error| error.to_string())
    }
}

#[cfg(windows)]
fn set_hidden_text(clipboard: &mut Clipboard, text: &str) -> Result<(), arboard::Error> {
    use arboard::SetExtWindows;
    clipboard.set().exclude_from_history().text(text)
}

#[cfg(not(windows))]
fn set_hidden_text(clipboard: &mut Clipboard, text: &str) -> Result<(), arboard::Error> {
    clipboard.set_text(text)
}

#[cfg(windows)]
mod native {
    use clipboard_win::{Clipboard, raw};

    use crate::RawFormat;

    const OPEN_ATTEMPTS: usize = 10;
    const KEEP_OUT_OF_HISTORY: [&str; 2] =
        ["CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"];

    pub(super) fn read_formats() -> Result<Vec<RawFormat>, String> {
        let _open = Clipboard::new_attempts(OPEN_ATTEMPTS).map_err(|error| error.to_string())?;
        Ok(raw::EnumFormats::new().map(read_format).collect())
    }

    fn read_format(id: u32) -> RawFormat {
        let mut bytes = Vec::new();
        let read = raw::get_vec(id, &mut bytes);
        RawFormat {
            id,
            bytes: read.map(|_| bytes).map_err(|error| error.to_string()),
        }
    }

    pub(super) fn write_formats(formats: &[(u32, Vec<u8>)]) -> Result<(), String> {
        let _open = Clipboard::new_attempts(OPEN_ATTEMPTS).map_err(|error| error.to_string())?;
        raw::empty().map_err(|error| error.to_string())?;
        formats
            .iter()
            .try_for_each(|(id, bytes)| raw::set_without_clear(*id, bytes))
            .map_err(|error| error.to_string())?;
        KEEP_OUT_OF_HISTORY
            .iter()
            .filter_map(|name| raw::register_format(name))
            .try_for_each(|id| raw::set_without_clear(id.get(), &0_u32.to_le_bytes()))
            .map_err(|error| error.to_string())
    }
}

#[cfg(not(windows))]
mod native {
    use crate::RawFormat;

    const UNSUPPORTED: &str = "keeping every clipboard format is only implemented on Windows";

    pub(super) fn read_formats() -> Result<Vec<RawFormat>, String> {
        Err(UNSUPPORTED.into())
    }

    pub(super) fn write_formats(_formats: &[(u32, Vec<u8>)]) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
}

/// Synthetic key presses through `enigo`.
#[derive(Debug, Clone, Copy, Default)]
pub struct EnigoKeyboard;

impl KeyboardAccess for EnigoKeyboard {
    fn send_copy(&self) -> Result<(), String> {
        let mut enigo = Enigo::new(&Settings::default()).map_err(|error| error.to_string())?;
        release_modifiers(&mut enigo).map_err(|error| error.to_string())?;
        thread::sleep(MODIFIER_SETTLE);
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|error| error.to_string())?;
        let copied = enigo.key(Key::Unicode('c'), Direction::Click);
        let released = enigo.key(Key::Control, Direction::Release);
        copied.and(released).map_err(|error| error.to_string())
    }
}

fn release_modifiers(enigo: &mut Enigo) -> enigo::InputResult<()> {
    enigo.key(Key::Alt, Direction::Release)?;
    enigo.key(Key::Shift, Direction::Release)?;
    enigo.key(Key::Meta, Direction::Release)?;
    enigo.key(Key::Control, Direction::Release)
}
