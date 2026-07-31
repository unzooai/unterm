//! Who the mouse belongs to.
//!
//! A terminal's mouse has two owners and they take turns. Normally it is the
//! terminal's: dragging selects text, the wheel scrolls the scrollback. But a
//! program can ask for it -- vim, htop, less all do -- and from then on a
//! click is the program's to interpret, and the terminal must keep its hands
//! off or the user gets a selection they did not ask for on top of a click
//! that did work.
//!
//! Shift is the way back. Every terminal treats a shifted drag as "I want the
//! terminal, not the program", which is how you copy text out of vim without
//! leaving it.

use termwiz::input::Modifiers;
use unterm_engine::next_core::mouse_encoding::{
    MouseButton, MouseEvent, MouseEventKind, MouseModes, MouseTracking,
};

/// What the front end should do with a mouse event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// Send it to the program.
    ToProgram(MouseEvent),
    /// Keep it: selection, scrollback, the front end's own business.
    ToTerminal,
}

/// Which modifiers were held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Held {
    fn modifiers(self) -> Modifiers {
        let mut modifiers = Modifiers::NONE;
        if self.shift {
            modifiers |= Modifiers::SHIFT;
        }
        if self.ctrl {
            modifiers |= Modifiers::CTRL;
        }
        if self.alt {
            modifiers |= Modifiers::ALT;
        }
        modifiers
    }
}

/// Decide where a mouse event goes.
///
/// `button` is the button pressed or released, or the one held during a
/// motion. `None` on a motion with nothing held.
pub fn route(
    modes: MouseModes,
    kind: MouseEventKind,
    button: Option<MouseButton>,
    column: usize,
    row: usize,
    held: Held,
) -> Route {
    // Shift always means the user wants the terminal, whatever the program
    // asked for. Without this there is no way to select text inside a
    // full-screen program.
    if held.shift {
        return Route::ToTerminal;
    }

    let wanted = match modes.tracking {
        MouseTracking::None => false,
        // Press only: a release or a motion is still the terminal's, and
        // sending one would be a byte the program never asked for.
        MouseTracking::X10 => kind == MouseEventKind::Press,
        MouseTracking::ButtonEvent => kind != MouseEventKind::Motion,
        // Motion, but only while a button is down.
        MouseTracking::ButtonMotion => kind != MouseEventKind::Motion || button.is_some(),
        MouseTracking::AnyEvent => true,
    };
    if !wanted {
        return Route::ToTerminal;
    }

    Route::ToProgram(MouseEvent {
        kind,
        button,
        column,
        row,
        modifiers: held.modifiers(),
    })
}

/// Wheel notches as button presses, which is how a terminal reports them.
///
/// A wheel has no release: xterm sends a press for each notch and nothing
/// else, so a program that counts releases would wait forever.
pub fn wheel_button(lines: f32) -> Option<MouseButton> {
    if lines < 0.0 {
        Some(MouseButton::WheelUp)
    } else if lines > 0.0 {
        Some(MouseButton::WheelDown)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modes(tracking: MouseTracking) -> MouseModes {
        MouseModes {
            tracking,
            sgr: true,
            urxvt: false,
            utf8: false,
        }
    }

    fn press(modes: MouseModes, held: Held) -> Route {
        route(
            modes,
            MouseEventKind::Press,
            Some(MouseButton::Left),
            3,
            4,
            held,
        )
    }

    #[test]
    fn with_reporting_off_the_terminal_keeps_the_mouse() {
        assert_eq!(
            press(modes(MouseTracking::None), Held::default()),
            Route::ToTerminal
        );
    }

    #[test]
    fn a_program_that_asked_for_clicks_gets_them() {
        match press(modes(MouseTracking::ButtonEvent), Held::default()) {
            Route::ToProgram(event) => {
                assert_eq!(event.column, 3);
                assert_eq!(event.row, 4);
                assert_eq!(event.button, Some(MouseButton::Left));
            }
            other => panic!("expected the program to get it, got {other:?}"),
        }
    }

    #[test]
    fn shift_takes_the_mouse_back_from_the_program() {
        // Without this there is no way to select text inside vim.
        let held = Held {
            shift: true,
            ..Default::default()
        };
        assert_eq!(
            press(modes(MouseTracking::AnyEvent), held),
            Route::ToTerminal
        );
    }

    #[test]
    fn x10_gets_presses_only() {
        let m = modes(MouseTracking::X10);
        assert!(matches!(press(m, Held::default()), Route::ToProgram(_)));
        assert_eq!(
            route(
                m,
                MouseEventKind::Release,
                Some(MouseButton::Left),
                0,
                0,
                Held::default()
            ),
            Route::ToTerminal,
            "X10 has no release; sending one is a byte nobody asked for"
        );
    }

    #[test]
    fn button_event_mode_does_not_get_bare_motion() {
        let m = modes(MouseTracking::ButtonEvent);
        assert_eq!(
            route(m, MouseEventKind::Motion, None, 1, 1, Held::default()),
            Route::ToTerminal
        );
    }

    #[test]
    fn button_motion_mode_gets_motion_only_while_dragging() {
        let m = modes(MouseTracking::ButtonMotion);
        assert_eq!(
            route(m, MouseEventKind::Motion, None, 1, 1, Held::default()),
            Route::ToTerminal,
            "nothing held is not a drag"
        );
        assert!(matches!(
            route(
                m,
                MouseEventKind::Motion,
                Some(MouseButton::Left),
                1,
                1,
                Held::default()
            ),
            Route::ToProgram(_)
        ));
    }

    #[test]
    fn any_event_mode_gets_bare_motion() {
        assert!(matches!(
            route(
                modes(MouseTracking::AnyEvent),
                MouseEventKind::Motion,
                None,
                1,
                1,
                Held::default()
            ),
            Route::ToProgram(_)
        ));
    }

    #[test]
    fn modifiers_travel_with_the_event() {
        let held = Held {
            shift: false,
            ctrl: true,
            alt: true,
        };
        match press(modes(MouseTracking::ButtonEvent), held) {
            Route::ToProgram(event) => {
                assert!(event.modifiers.contains(Modifiers::CTRL));
                assert!(event.modifiers.contains(Modifiers::ALT));
            }
            other => panic!("expected the program to get it, got {other:?}"),
        }
    }

    #[test]
    fn the_wheel_reports_a_direction_and_nothing_for_no_movement() {
        assert_eq!(wheel_button(-1.0), Some(MouseButton::WheelUp));
        assert_eq!(wheel_button(1.0), Some(MouseButton::WheelDown));
        assert_eq!(wheel_button(0.0), None);
    }
}

/// What a right-click does.
///
/// A gesture, not a menu. With something selected it copies and lets go of the
/// selection -- the two things anyone does next, in one press. With nothing
/// selected it pastes. Both are one motion where a context menu is three, and
/// the terminal it replaces is the one people are used to on Windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightClick {
    CopyAndClear,
    Paste,
}

pub fn right_click(has_selection: bool) -> RightClick {
    if has_selection {
        RightClick::CopyAndClear
    } else {
        RightClick::Paste
    }
}

#[cfg(test)]
mod right_click_tests {
    use super::*;

    #[test]
    fn a_selection_is_copied_and_let_go_of() {
        assert_eq!(right_click(true), RightClick::CopyAndClear);
    }

    /// The clearing matters: leaving the selection up means the next
    /// right-click copies the same text again instead of pasting, which is
    /// the opposite of what the second press was for.
    #[test]
    fn with_nothing_selected_it_pastes() {
        assert_eq!(right_click(false), RightClick::Paste);
    }
}
