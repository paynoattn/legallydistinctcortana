use std::process;
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use platform_ports::{SelectionError, SelectionReader};

/// How long the focused app gets to answer the copy request.
pub const COPY_TIMEOUT: Duration = Duration::from_millis(600);

/// How often the clipboard is checked while waiting for the copy.
pub const POLL_INTERVAL: Duration = Duration::from_millis(20);

const CF_BITMAP: u32 = 2;
const CF_METAFILEPICT: u32 = 3;
const CF_PALETTE: u32 = 9;
const CF_ENHMETAFILE: u32 = 14;
const CF_OWNERDISPLAY: u32 = 0x80;
const CF_DSPBITMAP: u32 = 0x82;
const CF_DSPMETAFILEPICT: u32 = 0x83;
const CF_DSPENHMETAFILE: u32 = 0x8E;
const CF_DIB: u32 = 8;
const CF_DIBV5: u32 = 17;

/// Handle formats Windows rebuilds from a DIB that is put back.
const REBUILT_FROM_DIB: [u32; 2] = [CF_BITMAP, CF_PALETTE];

/// Formats that carry GDI handles instead of bytes, so they cannot be copied. A bitmap and its
/// palette are rebuilt by Windows from a DIB that is put back; any other handle format means
/// the clipboard cannot be restored exactly.
const HANDLE_FORMATS: [u32; 8] = [
    CF_BITMAP,
    CF_METAFILEPICT,
    CF_PALETTE,
    CF_ENHMETAFILE,
    CF_OWNERDISPLAY,
    CF_DSPBITMAP,
    CF_DSPMETAFILEPICT,
    CF_DSPENHMETAFILE,
];

/// How long to wait for the copy, and how often to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// Give up after this long: the app had nothing selected.
    pub timeout: Duration,
    /// Check the clipboard this often.
    pub poll: Duration,
}

impl Timing {
    /// [`COPY_TIMEOUT`] and [`POLL_INTERVAL`].
    pub const DEFAULT: Self = Self {
        timeout: COPY_TIMEOUT,
        poll: POLL_INTERVAL,
    };
}

/// One clipboard format as read from the system: its id, and its bytes or why they could not
/// be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFormat {
    /// The clipboard format id (standard or registered).
    pub id: u32,
    /// The format's bytes, or the reason they could not be read.
    pub bytes: Result<Vec<u8>, String>,
}

/// Every format that was on the clipboard, byte for byte, so it can be put back exactly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardSnapshot {
    formats: Vec<(u32, Vec<u8>)>,
}

impl ClipboardSnapshot {
    /// Keeps every byte format, and skips the handle formats Windows rebuilds from a DIB.
    ///
    /// # Errors
    ///
    /// A description when a byte format could not be read, or when a handle format has no DIB
    /// to be rebuilt from (metafiles, owner-drawn data). Either way the clipboard cannot be put
    /// back exactly, so it must not be touched.
    pub fn from_raw(raw: Vec<RawFormat>) -> Result<Self, String> {
        let (handles, readable): (Vec<RawFormat>, Vec<RawFormat>) = raw
            .into_iter()
            .partition(|format| HANDLE_FORMATS.contains(&format.id));
        let formats = readable
            .into_iter()
            .map(|format| {
                let id = format.id;
                format
                    .bytes
                    .map(|bytes| (id, bytes))
                    .map_err(|reason| format!("format {id}: {reason}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let has_dib = formats
            .iter()
            .any(|(id, _)| [CF_DIB, CF_DIBV5].contains(id));
        handles
            .iter()
            .find(|format| !(has_dib && REBUILT_FROM_DIB.contains(&format.id)))
            .map_or(Ok(Self { formats }), |lost| {
                Err(format!(
                    "format {} is a graphics handle that cannot be copied",
                    lost.id
                ))
            })
    }

    /// The saved formats, in clipboard order.
    #[must_use]
    pub fn formats(&self) -> &[(u32, Vec<u8>)] {
        &self.formats
    }
}

/// The system clipboard.
pub trait ClipboardAccess: Send + Sync {
    /// Every format currently on the clipboard, each read in full.
    ///
    /// # Errors
    ///
    /// A description when the clipboard cannot be opened.
    fn read_formats(&self) -> Result<Vec<RawFormat>, String>;
    /// Empties the clipboard and puts `formats` on it, kept out of clipboard history.
    ///
    /// # Errors
    ///
    /// A description of the clipboard failure.
    fn write_formats(&self, formats: &[(u32, Vec<u8>)]) -> Result<(), String>;
    /// The current text, if the clipboard holds text.
    fn text(&self) -> Option<String>;
    /// Replaces the contents with `text`, kept out of clipboard history where supported.
    ///
    /// # Errors
    ///
    /// A description of the clipboard failure.
    fn set_text(&self, text: &str) -> Result<(), String>;
}

/// Synthetic keyboard input.
pub trait KeyboardAccess: Send + Sync {
    /// Releases held modifiers (the shortcut's keys may still be down), then sends the
    /// platform's copy shortcut to the focused app.
    ///
    /// # Errors
    ///
    /// A description of the input failure.
    fn send_copy(&self) -> Result<(), String>;

    /// Releases held modifiers, then sends the platform's paste shortcut to the focused app.
    ///
    /// # Errors
    ///
    /// A description of the input failure.
    fn send_paste(&self) -> Result<(), String>;
}

/// Reads the selection in any app: saves every clipboard format, puts a marker on the
/// clipboard, asks the app to copy, waits until the clipboard changes, then puts every saved
/// format back. If the clipboard cannot be saved exactly, it is left untouched and the read
/// fails. One read runs at a time.
#[derive(Debug)]
pub struct ClipboardSelectionReader<C, K> {
    clipboard: C,
    keyboard: K,
    timing: Timing,
    transaction: Mutex<()>,
}

impl<C: ClipboardAccess, K: KeyboardAccess> ClipboardSelectionReader<C, K> {
    /// A reader with [`Timing::DEFAULT`].
    #[must_use]
    pub const fn new(clipboard: C, keyboard: K) -> Self {
        Self::with_timing(clipboard, keyboard, Timing::DEFAULT)
    }

    /// A reader with custom timing.
    #[must_use]
    pub const fn with_timing(clipboard: C, keyboard: K, timing: Timing) -> Self {
        Self {
            clipboard,
            keyboard,
            timing,
            transaction: Mutex::new(()),
        }
    }

    fn copy_and_wait(&self, marker: &str) -> Result<String, SelectionError> {
        self.keyboard
            .send_copy()
            .map_err(SelectionError::Unavailable)?;
        let deadline = Instant::now() + self.timing.timeout;
        loop {
            if let Some(text) = self.clipboard.text().filter(|text| text != marker) {
                return Ok(text);
            }
            if Instant::now() >= deadline {
                return Err(SelectionError::NothingSelected);
            }
            thread::sleep(self.timing.poll);
        }
    }

    fn restore(&self, snapshot: &ClipboardSnapshot) {
        if let Err(error) = self.clipboard.write_formats(snapshot.formats()) {
            tracing::warn!(%error, "could not restore the clipboard after reading the selection");
        }
    }
}

impl<C: ClipboardAccess, K: KeyboardAccess> SelectionReader for ClipboardSelectionReader<C, K> {
    fn read_selection(&self) -> Result<String, SelectionError> {
        let _one_at_a_time = self
            .transaction
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let snapshot = self
            .clipboard
            .read_formats()
            .and_then(ClipboardSnapshot::from_raw)
            .map_err(|reason| unpreservable(&reason))?;
        let marker = marker();
        let copied = self
            .clipboard
            .set_text(&marker)
            .map_err(SelectionError::Unavailable)
            .and_then(|()| self.copy_and_wait(&marker));
        self.restore(&snapshot);
        copied
    }
}

fn unpreservable(reason: &str) -> SelectionError {
    SelectionError::Unavailable(format!(
        "the clipboard was left untouched because it could not be put back afterwards \
         ({reason}). Paste or replace what is on it, then try again."
    ))
}

fn marker() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!("\u{2063}read-aloud-marker-{}-{nanos}", process::id())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};

    use super::*;

    const TEXT: u32 = 13;
    const HTML: u32 = 49_161;
    const DIB: u32 = 8;

    type Formats = Vec<(u32, Vec<u8>)>;

    #[derive(Default)]
    struct Board {
        contents: Mutex<Formats>,
        log: Mutex<Vec<String>>,
        refuse: &'static [&'static str],
        unreadable: Option<u32>,
    }

    impl Board {
        fn holding(formats: Formats) -> Arc<Self> {
            Arc::new(Self {
                contents: Mutex::new(formats),
                ..Self::default()
            })
        }

        fn contents(&self) -> Formats {
            self.contents
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }

        fn write(&self, entry: &str, formats: Formats) -> Result<(), String> {
            self.log.lock().unwrap().push(entry.to_owned());
            if self.refuse.contains(&entry) {
                return Err("clipboard locked by another app".into());
            }
            *self.contents.lock().unwrap() = formats;
            Ok(())
        }

        fn raw(&self, id: u32, bytes: Vec<u8>) -> RawFormat {
            let bytes = if self.unreadable == Some(id) {
                Err("not memory".into())
            } else {
                Ok(bytes)
            };
            RawFormat { id, bytes }
        }
    }

    impl ClipboardAccess for Arc<Board> {
        fn read_formats(&self) -> Result<Vec<RawFormat>, String> {
            Ok(self
                .contents()
                .into_iter()
                .map(|(id, bytes)| self.raw(id, bytes))
                .collect())
        }

        fn write_formats(&self, formats: &[(u32, Vec<u8>)]) -> Result<(), String> {
            self.write("write_formats", formats.to_vec())
        }

        fn text(&self) -> Option<String> {
            self.contents()
                .into_iter()
                .find(|(id, _)| *id == TEXT)
                .and_then(|(_, bytes)| String::from_utf8(bytes).ok())
        }

        fn set_text(&self, text: &str) -> Result<(), String> {
            self.write("set_text", vec![(TEXT, text.as_bytes().to_vec())])
        }
    }

    struct App {
        board: Arc<Board>,
        selection: Option<&'static str>,
        broken: bool,
    }

    impl KeyboardAccess for App {
        fn send_copy(&self) -> Result<(), String> {
            if self.broken {
                return Err("input blocked".into());
            }
            if let Some(selection) = self.selection {
                *self.board.contents.lock().unwrap() = vec![(TEXT, selection.as_bytes().to_vec())];
            }
            Ok(())
        }

        fn send_paste(&self) -> Result<(), String> {
            Err("not used".into())
        }
    }

    fn reader(board: &Arc<Board>, app: App) -> ClipboardSelectionReader<Arc<Board>, App> {
        ClipboardSelectionReader::with_timing(
            Arc::clone(board),
            app,
            Timing {
                timeout: Duration::from_millis(30),
                poll: Duration::from_millis(5),
            },
        )
    }

    fn app(board: &Arc<Board>, selection: Option<&'static str>) -> App {
        App {
            board: Arc::clone(board),
            selection,
            broken: false,
        }
    }

    fn rich() -> Formats {
        vec![
            (TEXT, b"the user text".to_vec()),
            (HTML, b"<b>the user text</b>".to_vec()),
        ]
    }

    #[test]
    fn returns_the_selection_and_puts_every_format_back() {
        let board = Board::holding(rich());
        let selection = reader(&board, app(&board, Some("Read me."))).read_selection();
        assert_eq!(selection, Ok("Read me.".into()));
        assert_eq!(board.contents(), rich());
    }

    #[test]
    fn an_empty_clipboard_is_put_back_empty() {
        let board = Board::holding(vec![]);
        reader(&board, app(&board, Some("x")))
            .read_selection()
            .unwrap();
        assert!(board.contents().is_empty(), "nothing was on it before");
    }

    #[test]
    fn nothing_selected_when_the_app_does_not_copy_in_time() {
        let board = Board::holding(rich());
        let started = Instant::now();
        let selection = reader(&board, app(&board, None)).read_selection();
        assert_eq!(selection, Err(SelectionError::NothingSelected));
        assert!(
            started.elapsed() >= Duration::from_millis(30),
            "waits for the full timeout"
        );
        assert_eq!(board.contents(), rich());
    }

    #[test]
    fn a_failed_copy_still_restores_the_clipboard() {
        let board = Board::holding(rich());
        let broken = App {
            broken: true,
            ..app(&board, Some("x"))
        };
        assert_eq!(
            reader(&board, broken).read_selection(),
            Err(SelectionError::Unavailable("input blocked".into()))
        );
        assert_eq!(board.contents(), rich());
    }

    #[test]
    fn an_unreadable_format_leaves_the_clipboard_untouched() {
        let board = Arc::new(Board {
            contents: Mutex::new(rich()),
            unreadable: Some(HTML),
            ..Board::default()
        });
        let result = reader(&board, app(&board, Some("x"))).read_selection();
        assert!(
            matches!(&result, Err(SelectionError::Unavailable(message)) if message.contains("format 49161: not memory")),
            "{result:?}"
        );
        assert!(board.log().is_empty(), "nothing was written");
        assert_eq!(board.contents(), rich());
    }

    #[test]
    fn a_failed_marker_write_still_attempts_the_restore() {
        let board = Arc::new(Board {
            contents: Mutex::new(rich()),
            refuse: &["set_text"],
            ..Board::default()
        });
        assert_eq!(
            reader(&board, app(&board, None)).read_selection(),
            Err(SelectionError::Unavailable(
                "clipboard locked by another app".into()
            ))
        );
        assert_eq!(board.log(), ["set_text", "write_formats"]);
    }

    #[test]
    fn a_failed_restore_does_not_hide_the_selection() {
        let board = Arc::new(Board {
            refuse: &["write_formats"],
            ..Board::default()
        });
        let selection = reader(&board, app(&board, Some("Hi"))).read_selection();
        assert_eq!(selection, Ok("Hi".into()));
    }

    fn bytes(id: u32) -> RawFormat {
        RawFormat {
            id,
            bytes: Ok(vec![1]),
        }
    }

    fn handle(id: u32) -> RawFormat {
        RawFormat {
            id,
            bytes: Err("handle".into()),
        }
    }

    fn refused(id: u32) -> Result<ClipboardSnapshot, String> {
        Err(format!(
            "format {id} is a graphics handle that cannot be copied"
        ))
    }

    #[test]
    fn a_bitmap_or_palette_is_rebuilt_from_a_dib_so_it_is_skipped() {
        for dib in [DIB, CF_DIBV5] {
            for id in [CF_BITMAP, CF_PALETTE] {
                let snapshot =
                    ClipboardSnapshot::from_raw(vec![bytes(TEXT), handle(id), bytes(dib)]).unwrap();
                assert_eq!(
                    snapshot.formats(),
                    [(TEXT, vec![1]), (dib, vec![1])],
                    "format {id} with DIB format {dib}"
                );
            }
        }
        let both =
            ClipboardSnapshot::from_raw(vec![handle(CF_BITMAP), handle(CF_PALETTE), bytes(DIB)]);
        assert_eq!(both.unwrap().formats(), [(DIB, vec![1])]);
    }

    #[test]
    fn readable_bytes_in_a_handle_format_are_never_copied() {
        let snapshot =
            ClipboardSnapshot::from_raw(vec![bytes(CF_BITMAP), bytes(TEXT), bytes(DIB)]).unwrap();
        assert_eq!(snapshot.formats(), [(TEXT, vec![1]), (DIB, vec![1])]);
    }

    #[test]
    fn a_bitmap_or_palette_without_a_dib_cannot_be_put_back() {
        for id in [CF_BITMAP, CF_PALETTE] {
            assert_eq!(
                ClipboardSnapshot::from_raw(vec![bytes(TEXT), handle(id)]),
                refused(id)
            );
        }
    }

    #[test]
    fn other_handle_formats_cannot_be_put_back_even_with_a_dib() {
        let handle_ids = [3, 14, 0x80, 0x82, 0x83, 0x8E];
        for id in handle_ids {
            assert_eq!(
                ClipboardSnapshot::from_raw(vec![bytes(DIB), handle(id)]),
                refused(id)
            );
        }
        assert_eq!(
            HANDLE_FORMATS,
            [2, 3, 9, 14, 0x80, 0x82, 0x83, 0x8E],
            "the handle formats"
        );
    }

    #[test]
    fn the_first_handle_that_cannot_be_rebuilt_is_named() {
        assert_eq!(
            ClipboardSnapshot::from_raw(vec![
                handle(CF_BITMAP),
                handle(CF_METAFILEPICT),
                handle(CF_ENHMETAFILE),
                bytes(DIB)
            ]),
            refused(CF_METAFILEPICT)
        );
    }

    #[test]
    fn an_unreadable_byte_format_is_reported_before_any_handle() {
        assert_eq!(
            ClipboardSnapshot::from_raw(vec![
                handle(CF_ENHMETAFILE),
                RawFormat {
                    id: HTML,
                    bytes: Err("locked".into())
                }
            ]),
            Err("format 49161: locked".into())
        );
    }

    #[test]
    fn an_unreadable_dib_does_not_count_as_a_dib() {
        let result = ClipboardSnapshot::from_raw(vec![handle(CF_BITMAP), handle(DIB)]);
        assert_eq!(result, Err("format 8: handle".into()));
    }

    #[test]
    fn a_clipboard_with_a_lost_handle_is_refused_and_left_untouched() {
        let board = Board::holding(vec![(CF_ENHMETAFILE, vec![1])]);
        let result = reader(&board, app(&board, Some("x"))).read_selection();
        assert!(
            matches!(&result, Err(SelectionError::Unavailable(message)) if message.contains("(format 14 is a graphics handle that cannot be copied)")),
            "{result:?}"
        );
        assert!(board.log().is_empty(), "nothing was written");
        assert_eq!(board.contents(), vec![(CF_ENHMETAFILE, vec![1])]);
    }

    #[test]
    fn an_empty_snapshot_has_no_formats() {
        assert_eq!(
            ClipboardSnapshot::from_raw(vec![]),
            Ok(ClipboardSnapshot::default())
        );
    }

    #[test]
    fn the_first_unreadable_byte_format_is_named_in_the_error() {
        let result = ClipboardSnapshot::from_raw(vec![
            RawFormat {
                id: TEXT,
                bytes: Ok(vec![1]),
            },
            RawFormat {
                id: HTML,
                bytes: Err("locked".into()),
            },
        ]);
        assert_eq!(result, Err("format 49161: locked".into()));
    }

    #[test]
    fn the_unpreservable_message_tells_the_person_what_to_do() {
        let board = Arc::new(Board {
            contents: Mutex::new(rich()),
            unreadable: Some(HTML),
            ..Board::default()
        });
        assert_eq!(
            reader(&board, app(&board, Some("x"))).read_selection(),
            Err(SelectionError::Unavailable(
                "the clipboard was left untouched because it could not be put back afterwards \
                 (format 49161: not memory). Paste or replace what is on it, then try again."
                    .into()
            ))
        );
    }

    #[test]
    fn a_failed_marker_write_does_not_wait_for_a_copy() {
        let board = Arc::new(Board {
            contents: Mutex::new(rich()),
            refuse: &["set_text"],
            ..Board::default()
        });
        let started = Instant::now();
        let result = reader(&board, app(&board, Some("x"))).read_selection();
        assert_eq!(
            result,
            Err(SelectionError::Unavailable(
                "clipboard locked by another app".into()
            ))
        );
        assert!(started.elapsed() < Duration::from_millis(25));
        assert_eq!(board.contents(), rich());
    }

    struct CountingApp {
        board: Arc<Board>,
        copies: AtomicUsize,
    }

    impl KeyboardAccess for CountingApp {
        fn send_copy(&self) -> Result<(), String> {
            let number = self.copies.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(10));
            *self.board.contents.lock().unwrap() =
                vec![(TEXT, format!("selection {number}").into_bytes())];
            Ok(())
        }

        fn send_paste(&self) -> Result<(), String> {
            Err("not used".into())
        }
    }

    #[test]
    fn two_reads_at_once_each_get_their_own_selection_and_the_clipboard_ends_as_it_began() {
        let board = Board::holding(rich());
        let shared = ClipboardSelectionReader::with_timing(
            Arc::clone(&board),
            CountingApp {
                board: Arc::clone(&board),
                copies: AtomicUsize::new(0),
            },
            Timing {
                timeout: Duration::from_secs(2),
                poll: Duration::from_millis(2),
            },
        );
        let mut results = thread::scope(|scope| {
            let first = scope.spawn(|| shared.read_selection().unwrap());
            let second = scope.spawn(|| shared.read_selection().unwrap());
            [first.join().unwrap(), second.join().unwrap()]
        });
        results.sort();
        assert_eq!(results, ["selection 0", "selection 1"]);
        assert_eq!(board.contents(), rich());
        let writes = board.log();
        assert_eq!(
            writes,
            ["set_text", "write_formats", "set_text", "write_formats"],
            "the second read starts only after the first restored the clipboard"
        );
    }

    #[test]
    fn a_read_after_a_panicking_one_still_works() {
        let board = Board::holding(rich());
        let shared = Arc::new(reader(&board, app(&board, Some("ok"))));
        let poisoner = Arc::clone(&shared);
        let died = thread::spawn(move || {
            let _held = poisoner.transaction.lock().unwrap();
            panic!("a reader died");
        })
        .join();
        assert!(died.is_err());
        assert_eq!(shared.read_selection(), Ok("ok".into()));
    }

    #[test]
    fn markers_are_invisible_and_unique_per_call() {
        let first = marker();
        assert!(
            first.starts_with('\u{2063}'),
            "starts with an invisible separator"
        );
        assert!(
            first.contains(&process::id().to_string()),
            "names the process"
        );
        thread::sleep(Duration::from_millis(1));
        assert_ne!(first, marker());
    }

    #[test]
    fn default_timing_is_the_documented_one() {
        assert_eq!(
            Timing::DEFAULT,
            Timing {
                timeout: COPY_TIMEOUT,
                poll: POLL_INTERVAL
            }
        );
    }
}
