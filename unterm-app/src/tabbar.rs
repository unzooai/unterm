//! The strip along the top that says which tabs exist.
//!
//! Deliberately plain: numbers and the active one highlighted. A tab bar that
//! cannot be seen is worse than none, and one that tries to be clever before
//! the terminal underneath it works is effort in the wrong place.
//!
//! The part that matters for correctness is that it takes its height *out of*
//! the terminal area. A bar drawn over the grid hides the last row while the
//! shell still believes it is there, so the cursor ends up somewhere the user
//! cannot see.

use unterm_render::quads::{CellMetrics, FrameColors, Quad};

/// How tall the bar is, in cells.
///
/// One row: enough to read, and every row it takes is a row of output the user
/// does not get.
pub const ROWS: usize = 1;

/// The terminal area left after the bar takes its share.
pub fn terminal_height(window_height: f32, metrics: CellMetrics, tabs: usize) -> f32 {
    if tabs <= 1 {
        // One tab needs no bar. Reserving a row for something not drawn is a
        // row of output thrown away.
        return window_height;
    }
    (window_height - metrics.height * ROWS as f32).max(metrics.height)
}

/// Where the terminal area starts, in pixels from the top.
pub fn terminal_top(metrics: CellMetrics, tabs: usize) -> f32 {
    if tabs <= 1 {
        0.0
    } else {
        metrics.height * ROWS as f32
    }
}

/// The bar's quads: one per tab, the active one in the foreground colour.
pub fn quads(
    tab_count: usize,
    active_index: usize,
    window_width: f32,
    metrics: CellMetrics,
    colors: FrameColors,
) -> Vec<Quad> {
    if tab_count <= 1 {
        return Vec::new();
    }

    let width = (window_width / tab_count as f32).max(metrics.width);
    (0..tab_count)
        .map(|index| Quad {
            left: index as f32 * width,
            top: 0.0,
            // A hair narrower, so neighbouring tabs have a visible seam rather
            // than reading as one continuous bar.
            width: (width - 1.0).max(1.0),
            height: metrics.height,
            color: if index == active_index {
                colors.foreground
            } else {
                // Halfway between: present enough to count, quiet enough that
                // the active one is obvious at a glance.
                [
                    (colors.foreground[0] + colors.background[0]) / 2.0,
                    (colors.foreground[1] + colors.background[1]) / 2.0,
                    (colors.foreground[2] + colors.background[2]) / 2.0,
                    1.0,
                ]
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> CellMetrics {
        CellMetrics {
            width: 10.0,
            height: 20.0,
            baseline: 16.0,
        }
    }

    fn colors() -> FrameColors {
        FrameColors {
            foreground: [1.0, 1.0, 1.0, 1.0],
            background: [0.0, 0.0, 0.0, 1.0],
        }
    }

    #[test]
    fn one_tab_needs_no_bar() {
        // Reserving a row for something not drawn is a row of output thrown
        // away on every window that never opens a second tab.
        assert_eq!(terminal_height(400.0, metrics(), 1), 400.0);
        assert_eq!(terminal_top(metrics(), 1), 0.0);
        assert!(quads(1, 0, 800.0, metrics(), colors()).is_empty());
    }

    #[test]
    fn the_bar_takes_its_height_from_the_terminal() {
        // Drawing over the grid instead hides the last row while the shell
        // still believes it is there.
        assert_eq!(terminal_height(400.0, metrics(), 2), 380.0);
        assert_eq!(terminal_top(metrics(), 2), 20.0);
    }

    #[test]
    fn a_window_too_short_for_both_still_leaves_a_row_of_terminal() {
        assert_eq!(terminal_height(20.0, metrics(), 3), 20.0);
    }

    #[test]
    fn every_tab_gets_a_piece_of_the_bar() {
        let bar = quads(4, 0, 800.0, metrics(), colors());

        assert_eq!(bar.len(), 4);
        assert_eq!(bar[0].left, 0.0);
        assert_eq!(bar[3].left, 600.0);
    }

    #[test]
    fn the_active_tab_is_the_one_that_stands_out() {
        let bar = quads(3, 1, 900.0, metrics(), colors());

        assert_eq!(bar[1].color, colors().foreground);
        assert_ne!(bar[0].color, colors().foreground);
        assert_ne!(bar[0].color, colors().background);
    }

    #[test]
    fn tabs_have_a_seam_between_them() {
        let bar = quads(2, 0, 800.0, metrics(), colors());

        // Without it the bar reads as one block and the tab count is invisible.
        assert!(bar[0].left + bar[0].width < bar[1].left);
    }

    #[test]
    fn many_tabs_in_a_narrow_window_still_each_get_a_cell() {
        let bar = quads(200, 0, 100.0, metrics(), colors());

        assert!(bar.iter().all(|quad| quad.width >= 1.0));
    }
}
