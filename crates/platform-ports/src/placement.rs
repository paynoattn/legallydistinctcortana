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

/// Top-left position for a window of `window` size in the bottom-right corner of `area`,
/// `margin` pixels in from its right and bottom edges.
///
/// A window wider or taller than the area keeps its top-left corner inside the area.
#[must_use]
pub fn corner_origin(window: Size, area: Rect, margin: u32) -> Point {
    Point {
        x: far_edge(
            window.width,
            Span::new(area.origin.x, area.size.width),
            margin,
        ),
        y: far_edge(
            window.height,
            Span::new(area.origin.y, area.size.height),
            margin,
        ),
    }
}

/// Top-left position for a window of `window` size centred in `area`.
///
/// A window wider or taller than the area keeps its top-left corner inside the area.
#[must_use]
pub fn centred_origin(window: Size, area: Rect) -> Point {
    Point {
        x: centre(window.width, Span::new(area.origin.x, area.size.width)),
        y: centre(window.height, Span::new(area.origin.y, area.size.height)),
    }
}

fn centre(length: u32, span: Span) -> i32 {
    let Span { start, end } = span;
    let preferred = start + (end - start - i64::from(length)) / 2;
    i32::try_from(preferred.max(start)).unwrap_or(i32::MAX)
}

fn far_edge(length: u32, span: Span, margin: u32) -> i32 {
    let Span { start, end } = span;
    let preferred = end - i64::from(margin) - i64::from(length);
    i32::try_from(preferred.max(start)).unwrap_or(i32::MAX)
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
    fn the_corner_window_sits_inside_the_margin_of_the_bottom_right() {
        let window = Size {
            width: 160,
            height: 160,
        };
        assert_eq!(corner_origin(window, AREA, 16), Point { x: 1744, y: 864 });
    }

    #[test]
    fn the_corner_follows_a_monitor_left_of_the_primary_and_a_top_taskbar() {
        let left = Rect {
            origin: Point { x: -2560, y: 48 },
            size: Size {
                width: 2560,
                height: 1392,
            },
        };
        let window = Size {
            width: 320,
            height: 320,
        };
        assert_eq!(corner_origin(window, left, 32), Point { x: -352, y: 1088 });
    }

    #[test]
    fn a_corner_window_larger_than_the_area_keeps_its_top_left_inside() {
        let tiny = Rect {
            origin: Point { x: 10, y: 20 },
            size: Size {
                width: 100,
                height: 100,
            },
        };
        assert_eq!(corner_origin(POPUP, tiny, 16), Point { x: 10, y: 20 });
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

    #[test]
    fn a_window_is_centred_in_the_area() {
        let second_monitor = Rect {
            origin: Point { x: 1920, y: -100 },
            size: Size {
                width: 2560,
                height: 1400,
            },
        };
        let reader = Size {
            width: 1290,
            height: 991,
        };
        assert_eq!(
            centred_origin(reader, second_monitor),
            Point { x: 2555, y: 104 }
        );
    }

    #[test]
    fn a_window_larger_than_the_area_keeps_its_top_left_inside() {
        let tiny = Rect {
            origin: Point { x: 10, y: 20 },
            size: Size {
                width: 100,
                height: 100,
            },
        };
        assert_eq!(centred_origin(POPUP, tiny), Point { x: 10, y: 20 });
    }
}
