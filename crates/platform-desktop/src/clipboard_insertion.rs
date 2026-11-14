use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use platform_ports::{InsertError, TextInserter};

use crate::{ClipboardAccess, ClipboardSnapshot, KeyboardAccess};

/// How long the focused app gets to read the clipboard after the paste, before it is put back.
pub const PASTE_SETTLE: Duration = Duration::from_millis(150);

/// Types text into any app by pasting it: saves every clipboard format, puts the text on the
/// clipboard, sends the paste shortcut, waits [`PASTE_SETTLE`], then puts every saved format
/// back. If the clipboard cannot be saved exactly, nothing is pasted and it is left untouched.
/// One insertion runs at a time.
#[derive(Debug)]
pub struct ClipboardTextInserter<C, K> {
    clipboard: C,
    keyboard: K,
    settle: Duration,
    transaction: Mutex<()>,
}

impl<C: ClipboardAccess, K: KeyboardAccess> ClipboardTextInserter<C, K> {
    /// An inserter that waits [`PASTE_SETTLE`] before restoring the clipboard.
    #[must_use]
    pub const fn new(clipboard: C, keyboard: K) -> Self {
        Self::with_settle(clipboard, keyboard, PASTE_SETTLE)
    }

    /// An inserter that waits `settle` before restoring the clipboard.
    #[must_use]
    pub const fn with_settle(clipboard: C, keyboard: K, settle: Duration) -> Self {
        Self {
            clipboard,
            keyboard,
            settle,
            transaction: Mutex::new(()),
        }
    }

    fn paste(&self, text: &str) -> Result<(), InsertError> {
        self.clipboard.set_text(text).map_err(InsertError)?;
        self.keyboard.send_paste().map_err(InsertError)?;
        thread::sleep(self.settle);
        Ok(())
    }

    fn restore(&self, snapshot: &ClipboardSnapshot) {
        if let Err(error) = self.clipboard.write_formats(snapshot.formats()) {
            tracing::warn!(%error, "could not restore the clipboard after inserting text");
        }
    }
}

impl<C: ClipboardAccess, K: KeyboardAccess> TextInserter for ClipboardTextInserter<C, K> {
    fn insert(&self, text: &str) -> Result<(), InsertError> {
        let _one_at_a_time = self
            .transaction
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let snapshot = self
            .clipboard
            .read_formats()
            .and_then(ClipboardSnapshot::from_raw)
            .map_err(|reason| unpreservable(&reason))?;
        let pasted = self.paste(text);
        self.restore(&snapshot);
        pasted
    }
}

fn unpreservable(reason: &str) -> InsertError {
    InsertError(format!(
        "the clipboard was left untouched because it could not be put back afterwards \
         ({reason}). Paste or replace what is on it, then try again."
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use super::*;
    use crate::RawFormat;

    const TEXT: u32 = 13;
    const HTML: u32 = 49_161;
    const ENHMETAFILE: u32 = 14;

    type Formats = Vec<(u32, Vec<u8>)>;

    #[derive(Default)]
    struct Desk {
        clipboard: Mutex<Formats>,
        pasted: Mutex<Vec<String>>,
        log: Mutex<Vec<&'static str>>,
        fails: Option<&'static str>,
    }

    impl Desk {
        fn holding(formats: Formats) -> Self {
            Self {
                clipboard: Mutex::new(formats),
                ..Self::default()
            }
        }

        fn failing(step: &'static str) -> Self {
            Self {
                fails: Some(step),
                ..Self::holding(rich())
            }
        }

        fn note(&self, entry: &'static str, error: &str) -> Result<(), String> {
            self.log.lock().unwrap().push(entry);
            if self.fails == Some(entry) {
                return Err(error.into());
            }
            Ok(())
        }

        fn clipboard(&self) -> Formats {
            self.clipboard.lock().unwrap().clone()
        }

        fn pasted(&self) -> Vec<String> {
            self.pasted.lock().unwrap().clone()
        }

        fn log(&self) -> Vec<&'static str> {
            self.log.lock().unwrap().clone()
        }
    }

    #[derive(Clone)]
    struct Shared(Arc<Desk>);

    impl ClipboardAccess for Shared {
        fn read_formats(&self) -> Result<Vec<RawFormat>, String> {
            self.0.note("read_formats", "clipboard locked")?;
            Ok(self
                .0
                .clipboard()
                .into_iter()
                .map(|(id, bytes)| RawFormat {
                    id,
                    bytes: Ok(bytes),
                })
                .collect())
        }

        fn write_formats(&self, formats: &[(u32, Vec<u8>)]) -> Result<(), String> {
            self.0.note("write_formats", "clipboard locked")?;
            *self.0.clipboard.lock().unwrap() = formats.to_vec();
            Ok(())
        }

        fn text(&self) -> Option<String> {
            self.0
                .clipboard()
                .into_iter()
                .find(|(id, _)| *id == TEXT)
                .and_then(|(_, bytes)| String::from_utf8(bytes).ok())
        }

        fn set_text(&self, text: &str) -> Result<(), String> {
            self.0.note("set_text", "clipboard busy")?;
            *self.0.clipboard.lock().unwrap() = vec![(TEXT, text.as_bytes().to_vec())];
            Ok(())
        }
    }

    impl KeyboardAccess for Shared {
        fn send_copy(&self) -> Result<(), String> {
            Err("not used".into())
        }

        fn send_paste(&self) -> Result<(), String> {
            self.0.note("send_paste", "input blocked")?;
            let text = self.text().unwrap_or_default();
            self.0.pasted.lock().unwrap().push(text);
            Ok(())
        }
    }

    fn rich() -> Formats {
        vec![(TEXT, b"mine".to_vec()), (HTML, b"<b>mine</b>".to_vec())]
    }

    fn inserter(desk: &Arc<Desk>) -> ClipboardTextInserter<Shared, Shared> {
        let shared = Shared(Arc::clone(desk));
        ClipboardTextInserter::with_settle(shared.clone(), shared, Duration::ZERO)
    }

    #[test]
    fn pastes_the_text_and_puts_every_format_back() {
        let desk = Arc::new(Desk::holding(rich()));
        assert_eq!(inserter(&desk).insert("Hello, world. "), Ok(()));
        assert_eq!(desk.pasted(), ["Hello, world. "]);
        assert_eq!(desk.clipboard(), rich());
        assert_eq!(
            desk.log(),
            ["read_formats", "set_text", "send_paste", "write_formats"]
        );
    }

    #[test]
    fn an_empty_clipboard_is_left_empty() {
        let desk = Arc::new(Desk::default());
        assert_eq!(inserter(&desk).insert("hi"), Ok(()));
        assert_eq!(desk.pasted(), ["hi"]);
        assert_eq!(desk.clipboard(), Formats::new());
    }

    #[test]
    fn a_clipboard_that_cannot_be_saved_is_not_touched() {
        let desk = Arc::new(Desk::failing("read_formats"));
        let error = inserter(&desk).insert("hi").unwrap_err();
        assert_eq!(
            error.to_string(),
            "could not type into the focused app: the clipboard was left untouched because it \
             could not be put back afterwards (clipboard locked). Paste or replace what is on \
             it, then try again."
        );
        assert_eq!(desk.log(), ["read_formats"]);
        assert_eq!(desk.clipboard(), rich());
    }

    #[test]
    fn a_clipboard_with_a_graphics_handle_is_not_touched() {
        let desk = Arc::new(Desk::holding(vec![(ENHMETAFILE, vec![1])]));
        let error = inserter(&desk).insert("hi").unwrap_err();
        assert!(
            error.0.contains("format 14 is a graphics handle"),
            "{error}"
        );
        assert_eq!(desk.log(), ["read_formats"]);
        assert!(desk.pasted().is_empty(), "nothing was pasted");
    }

    #[test]
    fn a_failed_paste_still_restores_the_clipboard() {
        let desk = Arc::new(Desk::failing("send_paste"));
        assert_eq!(
            inserter(&desk).insert("hi"),
            Err(InsertError("input blocked".into()))
        );
        assert_eq!(desk.clipboard(), rich());
        assert_eq!(desk.log().last(), Some(&"write_formats"));
    }

    #[test]
    fn a_refused_clipboard_write_sends_no_paste_and_restores() {
        let desk = Arc::new(Desk::failing("set_text"));
        assert_eq!(
            inserter(&desk).insert("hi"),
            Err(InsertError("clipboard busy".into()))
        );
        assert_eq!(desk.log(), ["read_formats", "set_text", "write_formats"]);
    }

    #[test]
    fn a_failed_restore_does_not_fail_the_insertion() {
        let desk = Arc::new(Desk::failing("write_formats"));
        assert_eq!(inserter(&desk).insert("hi"), Ok(()));
        assert_eq!(desk.pasted(), ["hi"]);
    }

    #[test]
    fn waits_for_the_app_to_read_the_clipboard_before_restoring() {
        let desk = Arc::new(Desk::holding(rich()));
        let shared = Shared(Arc::clone(&desk));
        let slow =
            ClipboardTextInserter::with_settle(shared.clone(), shared, Duration::from_millis(60));
        let started = Instant::now();
        slow.insert("hi").unwrap();
        assert!(started.elapsed() >= Duration::from_millis(60));
    }

    #[test]
    fn the_default_settle_is_150_ms() {
        assert_eq!(PASTE_SETTLE, Duration::from_millis(150));
        let shared = Shared(Arc::new(Desk::default()));
        assert_eq!(
            ClipboardTextInserter::new(shared.clone(), shared).settle,
            PASTE_SETTLE
        );
    }

    #[test]
    fn one_insertion_runs_at_a_time() {
        let desk = Arc::new(Desk::holding(rich()));
        let shared = Shared(Arc::clone(&desk));
        let inserter = Arc::new(ClipboardTextInserter::with_settle(
            shared.clone(),
            shared,
            Duration::from_millis(20),
        ));
        let workers: Vec<_> = ["one", "two", "three"]
            .into_iter()
            .map(|text| {
                let inserter = Arc::clone(&inserter);
                thread::spawn(move || inserter.insert(text))
            })
            .collect();
        for worker in workers {
            assert_eq!(worker.join().unwrap(), Ok(()));
        }
        let mut pasted = desk.pasted();
        pasted.sort();
        assert_eq!(pasted, ["one", "three", "two"]);
        assert_eq!(desk.clipboard(), rich());
        assert_eq!(
            desk.log()
                .chunks(4)
                .map(<[&str]>::to_vec)
                .collect::<Vec<_>>(),
            vec![vec!["read_formats", "set_text", "send_paste", "write_formats"]; 3]
        );
    }
}
