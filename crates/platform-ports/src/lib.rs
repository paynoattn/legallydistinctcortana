//! OS capabilities as ports. Adapters live in `platform-desktop` (and later `platform-windows`).
//!
//! - [`SelectionReader`]: the text the user has selected in the focused app.
//! - [`TextInserter`]: put text at the cursor of the focused app.
//! - [`popup_origin`]: where to put a small window next to the cursor so it stays on screen.

mod insertion;
mod placement;
mod selection;

pub use insertion::{InsertError, TextInserter};
pub use placement::{POPUP_CURSOR_GAP, Point, Rect, Size, popup_origin};
pub use selection::{SelectionError, SelectionReader};
