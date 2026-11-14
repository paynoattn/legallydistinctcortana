/// Distance in physical pixels between the cursor and the popup's nearest corner.
pub const POPUP_CURSOR_GAP: i32 = 12;

/// A point in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    /// Pixels from the left of the virtual screen.
    pub x: i32,
    /// Pixels from the top of the virtual screen.
    pub y: i32,
}

/// A size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// A rectangle in physical screen pixels, such as a monitor's work area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Top-left corner.
    pub origin: Point,
    /// Width and height.
    pub size: Size,
}

/// Top-left position for a popup of `popup` size near `cursor`, inside `area`.
///
/// The popup opens below and to the right of the cursor. It flips to the left or above when
/// it would cross the area's right or bottom edge, and is clamped inside the area as a last
/// resort (for a popup larger than the area, its top-left corner stays visible).
#[must_use]
pub fn popup_origin(cursor: Point, popup: Size, area: Rect) -> Point {
    Point {
        x: axis(
            cursor.x,
            popup.width,
            Span::new(area.origin.x, area.size.width),
        ),
        y: axis(
            cursor.y,
            popup.height,
            Span::new(area.origin.y, area.size.height),
        ),
    }
}

#[derive(Clone, Copy)]
struct Span {
    start: i64,
    end: i64,
}

impl Span {
    fn new(start: i32, extent: u32) -> Self {
        let start = i64::from(start);
        Self {
            start,
            end: start + i64::from(extent),
        }
    }
}

fn axis(cursor: i32, length: u32, span: Span) -> i32 {
    let gap = i64::from(POPUP_CURSOR_GAP);
    let (cursor, length) = (i64::from(cursor), i64::from(length));
    let Span { start, end } = span;
    let forward = cursor + gap;
    let preferred = if forward + length <= end {
        forward
    } else {
        cursor - gap - length
    };
    let clamped = preferred.min(end - length).max(start);
    i32::try_from(clamped).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect {
        origin: Point { x: 0, y: 0 },
        size: Size {
            width: 1920,
            height: 1040,
        },
    };
    const POPUP: Size = Size {
        width: 300,
        height: 400,
    };

    #[test]
    fn opens_below_right_of_the_cursor_when_there_is_room() {
        assert_eq!(
            popup_origin(Point { x: 100, y: 100 }, POPUP, AREA),
            Point { x: 112, y: 112 }
        );
    }

    #[test]
    fn a_cursor_past_the_right_edge_is_clamped_to_the_edge() {
        assert_eq!(
            popup_origin(Point { x: 3000, y: 100 }, POPUP, AREA),
            Point { x: 1620, y: 112 }
        );
    }

    #[test]
    fn fits_exactly_against_the_right_and_bottom_edges() {
        assert_eq!(
            popup_origin(Point { x: 1608, y: 628 }, POPUP, AREA),
            Point { x: 1620, y: 640 }
        );
    }

    #[test]
    fn flips_left_and_up_near_the_bottom_right_corner() {
        assert_eq!(
            popup_origin(Point { x: 1900, y: 1000 }, POPUP, AREA),
            Point { x: 1588, y: 588 }
        );
    }

    #[test]
    fn clamps_inside_the_area_when_neither_side_fits() {
        let narrow = Rect {
            origin: Point { x: -1280, y: 0 },
            size: Size {
                width: 320,
                height: 420,
            },
        };
        assert_eq!(
            popup_origin(Point { x: -1100, y: 200 }, POPUP, narrow),
            Point { x: -1280, y: 0 }
        );
    }

    #[test]
    fn a_popup_larger_than_the_area_keeps_its_top_left_visible() {
        let tiny = Rect {
            origin: Point { x: 10, y: 10 },
            size: Size {
                width: 100,
                height: 100,
            },
        };
        assert_eq!(
            popup_origin(Point { x: 50, y: 50 }, POPUP, tiny),
            Point { x: 10, y: 10 }
        );
    }
}
