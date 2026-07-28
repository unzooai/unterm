use std::collections::BTreeSet;

use super::cell::{CellAttributes, ScreenCell};

#[derive(Default)]
pub(super) struct ScreenState {
    pub(super) cols: usize,
    pub(super) scrollback: Vec<Vec<ScreenCell>>,
    pub(super) lines: Vec<Vec<ScreenCell>>,
    pub(super) viewport_top: Option<usize>,
    pub(super) cursor_x: usize,
    pub(super) cursor_y: usize,
    pub(super) cursor_visible: bool,
    pub(super) cursor_blinking: bool,
    pub(super) cursor_shape: String,
    pub(super) column_132_mode: bool,
    pub(super) auto_wrap: bool,
    pub(super) reverse_video: bool,
    pub(super) application_cursor_keys: bool,
    pub(super) application_keypad: bool,
    pub(super) focus_event_reporting: bool,
    pub(super) mouse_tracking: MouseTrackingMode,
    pub(super) utf8_mouse: bool,
    pub(super) urxvt_mouse: bool,
    pub(super) sgr_mouse: bool,
    pub(super) alternate_scroll: bool,
    pub(super) sgr_pixel_mouse: bool,
    pub(super) meta_sends_escape: bool,
    pub(super) synchronized_output: bool,
    pub(super) alternate_screen_modes: BTreeSet<usize>,
    pub(super) origin_mode: bool,
    pub(super) insert_mode: bool,
    pub(super) left_right_margin_mode: bool,
    pub(super) tab_stops: BTreeSet<usize>,
    pub(super) bracketed_paste: bool,
    pub(super) current_attr: CellAttributes,
    pub(super) scroll_top: usize,
    pub(super) scroll_bottom: usize,
    pub(super) left_margin: usize,
    pub(super) right_margin: usize,
    pub(super) saved_cursor_x: usize,
    pub(super) saved_cursor_y: usize,
    pub(super) saved_cursor_attr: CellAttributes,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum MouseTrackingMode {
    #[default]
    None,
    X10,
    ButtonEvent,
    AnyEvent,
}

#[cfg(test)]
mod resize_cursor_tests {
    use crate::next_core::NextCoreScreen;

    /// After a resize the cursor must still be on the line it was on.
    ///
    /// A window that gains or loses a row -- a tab bar appearing, a split, a
    /// drag -- resizes every pane under it. If the text moves and the cursor
    /// does not, what the user sees is a caret floating above their prompt,
    /// which is the sort of thing that reads as "this terminal is broken".
    #[test]
    fn the_cursor_stays_on_its_line_across_a_resize() {
        const PROMPT: &str = "C:/Users/me>";
        let row_of = |screen: &NextCoreScreen| {
            screen
                .lines
                .iter()
                .position(|line| {
                    line.iter().map(|cell| cell.ch).collect::<String>().contains(PROMPT)
                })
                .expect("the prompt is on screen")
        };

        let mut screen = NextCoreScreen::new(40, 6);
        screen.feed("\r\n");
        screen.feed(PROMPT);
        let before_x = screen.cursor_x;
        assert_eq!(
            screen.cursor_y,
            row_of(&screen),
            "the cursor starts on the prompt's line"
        );

        screen.resize(40, 5);
        assert_eq!(
            screen.cursor_y,
            row_of(&screen),
            "the cursor followed the prompt when a row was taken away"
        );
        assert_eq!(screen.cursor_x, before_x);

        screen.resize(40, 6);
        assert_eq!(
            screen.cursor_y,
            row_of(&screen),
            "the cursor followed the prompt when the row came back"
        );
    }

    /// The same, on a screen that has already scrolled.
    ///
    /// This is the case a real shell is in: enough output has gone by that
    /// the line buffer is as tall as the screen, so taking a row away drops
    /// one off the top and everything below it moves.
    #[test]
    fn the_cursor_stays_on_its_line_when_shrinking_scrolls_the_top_away() {
        const PROMPT: &str = "C:/Users/me>";
        let row_of = |screen: &NextCoreScreen| {
            screen
                .lines
                .iter()
                .position(|line| {
                    line.iter().map(|cell| cell.ch).collect::<String>().contains(PROMPT)
                })
                .expect("the prompt is on screen")
        };

        let mut screen = NextCoreScreen::new(40, 6);
        for n in 0..8 {
            screen.feed(&format!("line {n}\r\n"));
        }
        screen.feed(PROMPT);
        assert_eq!(
            screen.cursor_y,
            row_of(&screen),
            "the cursor starts on the prompt's line"
        );

        screen.resize(40, 5);
        assert_eq!(
            screen.cursor_y,
            row_of(&screen),
            "a row taken off the top must move the cursor with the text"
        );

        screen.resize(40, 3);
        assert_eq!(
            screen.cursor_y,
            row_of(&screen),
            "several rows at once is the same rule"
        );
    }

    /// The reported cursor row is the row the text is actually drawn on.
    ///
    /// `cursor_y` counts live lines, but the viewport does not always start
    /// at the first of those. Two cases where it does not, and both used to
    /// put the caret on the wrong row.
    #[test]
    fn the_reported_cursor_row_matches_the_row_the_prompt_is_drawn_on() {
        const PROMPT: &str = "C:/Users/me>";
        let drawn_row = |screen: &NextCoreScreen| {
            screen
                .styled_viewport_lines(0)
                .iter()
                .position(|line| {
                    line.cells.iter().map(|cell| cell.ch).collect::<String>().contains(PROMPT)
                })
                .map(|row| row as isize)
        };

        let mut screen = NextCoreScreen::new(40, 6);
        for n in 0..10 {
            screen.feed(&format!("line {n}\r\n"));
        }
        screen.feed(PROMPT);
        assert_eq!(screen.cursor_snapshot().y, drawn_row(&screen).unwrap());

        // Grown: the rows above the live lines are filled from scrollback,
        // so every drawn row is further down than its live index.
        screen.resize(40, 9);
        assert_eq!(
            screen.cursor_snapshot().y,
            drawn_row(&screen).unwrap(),
            "growing pulls scrollback in above the live lines"
        );

        // Scrolled back: the prompt has left the viewport, and so must the
        // cursor -- a caret parked in the middle of old output is worse than
        // no caret at all.
        screen.scroll_viewport_by(-4);
        assert!(
            drawn_row(&screen).is_none(),
            "the prompt should be off-screen after scrolling back"
        );
        assert!(
            screen.cursor_snapshot().y >= screen.rows as isize,
            "the cursor should be below the viewport, not inside it"
        );
    }
}
