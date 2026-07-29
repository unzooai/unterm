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
/// Named after the sequence that sets each one.
///
/// They used to be named one number off -- `X10` meant `CSI ? 1000 h` -- and
/// the mapping had to be explained in a comment wherever it was read. A name
/// that lies is a bug waiting for someone to "fix" the mapping.
///
/// Real X10 tracking (`CSI ? 9 h`) is not parsed. It is ignored rather than
/// approximated: guessing would send a program bytes it never asked for, and
/// nothing written this century uses it.
pub(super) enum MouseTrackingMode {
    #[default]
    None,
    /// `CSI ? 1000 h` -- press and release.
    ButtonEvent,
    /// `CSI ? 1002 h` -- plus motion while a button is held.
    ButtonMotion,
    /// `CSI ? 1003 h` -- plus free motion.
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

#[cfg(test)]
mod mouse_mode_tests {
    use crate::next_core::mouse_encoding::MouseTracking;
    use crate::next_core::NextCoreScreen;

    /// What a program asks for reaches the front end.
    ///
    /// A front end has to know whether a click belongs to the program before
    /// it acts on one itself. It learns that from the screen snapshot, so the
    /// path from the escape sequence to that snapshot is the thing to check --
    /// a mode parsed but not reported is the same as a mode not parsed.
    #[test]
    fn a_program_asking_for_the_mouse_is_reported_to_the_front_end() {
        let mut screen = NextCoreScreen::new(40, 6);
        assert_eq!(
            screen.mouse_modes().tracking,
            MouseTracking::None,
            "nothing has asked yet"
        );

        // What vim sends: button events, SGR encoding.
        screen.feed("\x1b[?1000h");
        screen.feed("\x1b[?1006h");
        let modes = screen.mouse_modes();
        assert_eq!(modes.tracking, MouseTracking::ButtonEvent);
        assert!(modes.sgr, "SGR is what survives past column 223");

        // And what it sends on the way out.
        screen.feed("\x1b[?1000l");
        screen.feed("\x1b[?1006l");
        assert_eq!(
            screen.mouse_modes().tracking,
            MouseTracking::None,
            "leaving the program gives the mouse back to the terminal"
        );
    }

    #[test]
    fn each_tracking_mode_is_understood() {
        let cases: [(&str, MouseTracking); 4] = [
            ("[?1000h", MouseTracking::ButtonEvent),
            ("[?1002h", MouseTracking::ButtonMotion),
            ("[?1003h", MouseTracking::AnyEvent),
            // Real X10 (`CSI ? 9 h`) is not parsed. Ignoring it is the safe
            // reading: approximating it would send a program bytes it never
            // asked for, and nothing written this century uses it.
            ("[?9h", MouseTracking::None),
        ];
        for (sequence, expected) in cases {
            let mut screen = NextCoreScreen::new(40, 6);
            screen.feed(sequence);
            assert_eq!(
                screen.mouse_modes().tracking,
                expected,
                "{sequence:?} should ask for {expected:?}"
            );
        }
    }
}

#[cfg(test)]
mod bell_tests {
    use crate::next_core::NextCoreScreen;

    /// A bell is counted, and counting is what makes it not get lost.
    ///
    /// A flag would be missed whenever two bells landed between two frames,
    /// or shown twice if a reader forgot to clear it. A number that only goes
    /// up is one both sides can compare.
    #[test]
    fn ringing_the_bell_is_counted_rather_than_flagged() {
        let mut screen = NextCoreScreen::new(20, 4);
        assert_eq!(screen.bells, 0);

        screen.feed("hi\x07there");
        assert_eq!(screen.bells, 1);
        assert_eq!(
            screen.lines[0].iter().map(|cell| cell.ch).collect::<String>().trim_end(),
            "hithere",
            "the bell is not a character and takes no cell"
        );

        screen.feed("\x07\x07");
        assert_eq!(screen.bells, 3, "two bells between reads are two bells");
    }

    #[test]
    fn a_bell_inside_an_osc_string_terminates_it_rather_than_ringing() {
        // BEL is how an OSC ends. Counting that one would ring on every
        // window-title change, which is most of what a shell does.
        let mut screen = NextCoreScreen::new(20, 4);
        screen.feed("\x1b]0;a title\x07");
        assert_eq!(screen.bells, 0);
        assert_eq!(screen.title(), Some("a title".to_string()));
    }
}
