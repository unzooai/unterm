//! Where you are in the scrollback, and how to get somewhere else.
//!
//! A wheel tells you nothing about how much is above you. A terminal with ten
//! thousand lines of build output and no scrollbar leaves the user guessing
//! whether they are near the start, and dragging blind.
//!
//! The arithmetic is here rather than in the window because every one of these
//! is off by a bit until someone drags to the very bottom and checks: the
//! thumb has to reach the end, a click has to land where the user pointed, and
//! a drag has to track the pointer rather than drift.

/// How wide the bar is, in pixels.
pub const WIDTH: f32 = 8.0;

/// Where the thumb sits and how tall it is, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thumb {
    pub top: f32,
    pub height: f32,
}

/// The scrollbar's thumb for a viewport over some history.
///
/// `top_row` is the first row on screen, counting from the oldest line kept.
/// `None` when everything fits: a bar that fills its whole track tells the
/// user nothing and takes a column to say it.
pub fn thumb(total_rows: usize, viewport_rows: usize, top_row: usize, track: f32) -> Option<Thumb> {
    if total_rows <= viewport_rows || viewport_rows == 0 || track <= 0.0 {
        return None;
    }
    let total = total_rows as f32;
    let visible = viewport_rows as f32;

    // A floor, so the thumb stays grabbable over a very long scrollback.
    const MIN_HEIGHT: f32 = 16.0;
    let height = (track * visible / total).max(MIN_HEIGHT).min(track);

    let max_top_row = (total_rows - viewport_rows) as f32;
    let progress = (top_row as f32 / max_top_row).clamp(0.0, 1.0);
    Some(Thumb {
        // Against the remaining track, so dragging to the bottom puts the
        // thumb at the bottom rather than one thumb-height short of it.
        top: progress * (track - height),
        height,
    })
}

/// Which row a pointer at `y` is asking for.
///
/// The thumb's *centre* follows the pointer, which is what makes a drag feel
/// like it is holding the thumb rather than pushing its top edge.
pub fn row_at(total_rows: usize, viewport_rows: usize, y: f32, track: f32) -> usize {
    if total_rows <= viewport_rows || track <= 0.0 {
        return 0;
    }
    let height = thumb(total_rows, viewport_rows, 0, track)
        .map(|thumb| thumb.height)
        .unwrap_or(0.0);
    let usable = (track - height).max(1.0);
    let progress = ((y - height / 2.0) / usable).clamp(0.0, 1.0);
    let max_top_row = total_rows - viewport_rows;
    (progress * max_top_row as f32).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_fitting_needs_no_bar() {
        assert!(thumb(24, 24, 0, 480.0).is_none());
        assert!(thumb(10, 24, 0, 480.0).is_none());
    }

    #[test]
    fn the_thumb_is_the_visible_share_of_the_history() {
        let thumb = thumb(100, 25, 0, 400.0).expect("a bar");
        assert_eq!(thumb.height, 100.0, "a quarter visible is a quarter tall");
        assert_eq!(thumb.top, 0.0, "at the top of the history");
    }

    #[test]
    fn scrolled_to_the_bottom_puts_the_thumb_at_the_bottom() {
        // The one every scrollbar gets wrong: the thumb has to reach the end
        // of the track, not stop one thumb-height short of it.
        let track = 400.0;
        let thumb = thumb(100, 25, 75, track).expect("a bar");
        assert_eq!(thumb.top + thumb.height, track);
    }

    #[test]
    fn a_very_long_history_still_leaves_something_to_grab() {
        let thumb = thumb(1_000_000, 25, 0, 400.0).expect("a bar");
        assert!(thumb.height >= 16.0, "a one-pixel thumb cannot be grabbed");
    }

    #[test]
    fn the_thumb_never_leaves_its_track() {
        let track = 400.0;
        for top_row in [0, 1, 37, 74, 75, 999] {
            let thumb = thumb(100, 25, top_row, track).expect("a bar");
            assert!(thumb.top >= 0.0);
            assert!(thumb.top + thumb.height <= track + 0.01, "at row {top_row}");
        }
    }

    #[test]
    fn clicking_the_top_of_the_track_goes_to_the_oldest_line() {
        assert_eq!(row_at(100, 25, 0.0, 400.0), 0);
    }

    #[test]
    fn clicking_the_bottom_of_the_track_goes_to_the_newest() {
        assert_eq!(row_at(100, 25, 400.0, 400.0), 75);
    }

    #[test]
    fn a_drag_and_the_thumb_it_produces_agree() {
        // Drag to a position, ask where the thumb lands, and the pointer
        // should be inside it. Without this a drag drifts away from the
        // pointer the further it goes.
        let track = 400.0;
        for y in [20.0, 100.0, 200.0, 300.0, 380.0] {
            let row = row_at(1000, 25, y, track);
            let thumb = thumb(1000, 25, row, track).expect("a bar");
            assert!(
                y >= thumb.top - 1.0 && y <= thumb.top + thumb.height + 1.0,
                "dragging to {y} put the thumb at {}..{}",
                thumb.top,
                thumb.top + thumb.height
            );
        }
    }

    #[test]
    fn a_pointer_past_the_track_is_clamped_rather_than_wrapping() {
        assert_eq!(row_at(100, 25, -50.0, 400.0), 0);
        assert_eq!(row_at(100, 25, 9999.0, 400.0), 75);
    }

    #[test]
    fn nothing_to_scroll_asks_for_the_first_row() {
        assert_eq!(row_at(10, 25, 200.0, 400.0), 0);
    }
}

#[cfg(test)]
mod live_shape_tests {
    use super::*;

    /// The numbers a live pane actually reports produce a bar.
    ///
    /// Taken from a running terminal with 200 lines of output in a 45-row
    /// window: 156 rows above the viewport. If the arithmetic says "nothing
    /// to scroll" for that, the bar never appears no matter how much history
    /// there is.
    #[test]
    fn a_pane_with_history_above_it_gets_a_bar() {
        let scrollback_rows = 156;
        let viewport_rows = 45;
        let total = scrollback_rows + viewport_rows;
        let bar = thumb(total, viewport_rows, scrollback_rows, 540.0)
            .expect("156 rows above a 45-row window is a scrollbar");
        assert!(bar.height < 540.0);
        assert!(
            (bar.top + bar.height - 540.0).abs() < 0.01,
            "scrolled to the newest line, the thumb sits at the bottom"
        );
    }
}
