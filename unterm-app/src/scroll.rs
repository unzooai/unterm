//! Turning wheel and key events into a number of lines.
//!
//! Kept separate from the event loop so the arithmetic can be checked without
//! a window. It is all arithmetic, and all of it is the kind that is wrong by
//! a sign or a factor of three until someone tries it.

/// How many lines one wheel notch moves.
///
/// Three is what every desktop uses; one is too slow to read with and a whole
/// screen is too far to keep your place.
const LINES_PER_NOTCH: f32 = 3.0;

/// Lines to scroll for a wheel event.
///
/// Positive is toward older output, matching how the engine numbers a
/// scrollback: rolling the wheel away from you goes back in time.
pub fn lines_for_wheel(delta: WheelDelta, cell_height: f32) -> isize {
    let lines = match delta {
        WheelDelta::Lines(lines) => lines * LINES_PER_NOTCH,
        // A trackpad reports pixels. Dividing by the cell height is what makes
        // a flick move the same distance as the text it is moving.
        WheelDelta::Pixels(pixels) => pixels / cell_height.max(1.0),
    };
    // Round away from zero: a small trackpad movement that truncates to zero
    // feels like the scroll is broken.
    if lines > 0.0 {
        lines.max(1.0).round() as isize
    } else if lines < 0.0 {
        lines.min(-1.0).round() as isize
    } else {
        0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelDelta {
    Lines(f32),
    Pixels(f32),
}

/// Lines for a page key.
///
/// A page is a screen less one line, so the line you were reading stays on
/// screen and gives you somewhere to pick up.
pub fn lines_for_page(rows: usize) -> isize {
    (rows.max(2) - 1) as isize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wheel_notch_moves_three_lines() {
        // One line is too slow to read with; a whole screen loses your place.
        assert_eq!(lines_for_wheel(WheelDelta::Lines(1.0), 20.0), 3);
        assert_eq!(lines_for_wheel(WheelDelta::Lines(-1.0), 20.0), -3);
    }

    #[test]
    fn a_trackpad_moves_the_text_it_is_pushing() {
        // Sixty pixels over a twenty-pixel cell is three lines, whatever the
        // font size happens to be.
        assert_eq!(lines_for_wheel(WheelDelta::Pixels(60.0), 20.0), 3);
        assert_eq!(lines_for_wheel(WheelDelta::Pixels(60.0), 30.0), 2);
    }

    #[test]
    fn a_small_trackpad_movement_still_moves() {
        // Truncating to zero reads as a broken scroll, and a trackpad produces
        // a lot of small deltas.
        assert_eq!(lines_for_wheel(WheelDelta::Pixels(3.0), 20.0), 1);
        assert_eq!(lines_for_wheel(WheelDelta::Pixels(-3.0), 20.0), -1);
    }

    #[test]
    fn no_movement_scrolls_nothing() {
        assert_eq!(lines_for_wheel(WheelDelta::Pixels(0.0), 20.0), 0);
        assert_eq!(lines_for_wheel(WheelDelta::Lines(0.0), 20.0), 0);
    }

    #[test]
    fn a_zero_height_cell_does_not_divide_by_zero() {
        // Metrics come from a font that may not have loaded yet.
        assert_eq!(lines_for_wheel(WheelDelta::Pixels(60.0), 0.0), 60);
    }

    #[test]
    fn a_page_keeps_one_line_of_context() {
        // Paging by the full height leaves nothing to reconnect the screens.
        assert_eq!(lines_for_page(24), 23);
        assert_eq!(lines_for_page(2), 1);
    }

    #[test]
    fn a_page_on_a_tiny_screen_still_moves() {
        assert_eq!(lines_for_page(1), 1);
        assert_eq!(lines_for_page(0), 1);
    }
}
