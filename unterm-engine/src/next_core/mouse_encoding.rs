//! Mouse reporting for next-core input.
//!
//! Unlike keys, mouse reporting is mode-dependent all the way down: whether to
//! report at all, and which of four incompatible byte formats to use, both
//! come from the modes the application negotiated. So this function takes the
//! modes explicitly rather than pretending to be mode-free — the session owns
//! that state and passes it in, keeping this pure and testable.
//!
//! Returning `None` means "the application did not ask to hear about this",
//! which is the common case: with tracking off, the terminal handles the mouse
//! itself (selection, scrollback) and sends nothing to the PTY.

use termwiz::input::Modifiers;

/// Which mouse events the application asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub enum MouseTracking {
    /// No reporting. The terminal keeps the mouse for itself.
    #[default]
    None,
    /// `CSI ? 9 h` — press only, no release, no motion.
    X10,
    /// `CSI ? 1000 h` — press and release.
    ButtonEvent,
    /// `CSI ? 1002 h` — press, release, and motion while a button is held.
    ButtonMotion,
    /// `CSI ? 1003 h` — press, release, and all motion.
    AnyEvent,
}

/// The mouse-related modes of a session, as the encoder needs to see them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct MouseModes {
    pub tracking: MouseTracking,
    /// `CSI ? 1006 h` — SGR encoding. Takes precedence; it is the only format
    /// that reports releases per-button and survives past column 223.
    pub sgr: bool,
    /// `CSI ? 1015 h` — urxvt encoding.
    pub urxvt: bool,
    /// `CSI ? 1005 h` — UTF-8 extended coordinates on the legacy format.
    pub utf8: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEventKind {
    Press,
    Release,
    /// Pointer moved. `button` is the button held down, if any.
    Motion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    /// The button involved. `None` on a motion event with nothing held.
    pub button: Option<MouseButton>,
    /// Zero-based cell coordinates. The wire format is one-based; the
    /// conversion happens here so callers can stay in screen coordinates.
    pub column: usize,
    pub row: usize,
    pub modifiers: Modifiers,
}

/// Legacy encodings bias every byte by 32 and cannot express a coordinate
/// past 223 (255 - 32). Beyond that xterm simply stops reporting.
const LEGACY_COORD_BIAS: usize = 32;
const LEGACY_COORD_MAX: usize = 223;

fn modifier_bits(modifiers: Modifiers) -> u8 {
    let mut bits = 0;
    if modifiers.contains(Modifiers::SHIFT) {
        bits |= 4;
    }
    if modifiers.contains(Modifiers::ALT) {
        bits |= 8;
    }
    if modifiers.contains(Modifiers::CTRL) {
        bits |= 16;
    }
    bits
}

/// The button field of the report, before modifier and motion bits.
fn button_bits(event: &MouseEvent, sgr: bool) -> Option<u8> {
    let base = match event.button {
        Some(MouseButton::Left) => 0,
        Some(MouseButton::Middle) => 1,
        Some(MouseButton::Right) => 2,
        Some(MouseButton::WheelUp) => 64,
        Some(MouseButton::WheelDown) => 65,
        Some(MouseButton::WheelLeft) => 66,
        Some(MouseButton::WheelRight) => 67,
        // Motion with no button held is reported as button 3 ("no button").
        None => 3,
    };

    match event.kind {
        MouseEventKind::Press | MouseEventKind::Motion => Some(base),
        MouseEventKind::Release => {
            // SGR reports which button was released via the final byte, so it
            // keeps the real button code. The legacy formats cannot, and use
            // the same "button 3" for every release.
            if sgr {
                Some(base)
            } else {
                Some(3)
            }
        }
    }
}

fn wants_event(modes: MouseModes, event: &MouseEvent) -> bool {
    match modes.tracking {
        MouseTracking::None => false,
        // X10 reports presses only — no releases, no motion.
        MouseTracking::X10 => matches!(event.kind, MouseEventKind::Press),
        MouseTracking::ButtonEvent => !matches!(event.kind, MouseEventKind::Motion),
        // Motion only while a button is held.
        MouseTracking::ButtonMotion => {
            !matches!(event.kind, MouseEventKind::Motion) || event.button.is_some()
        }
        MouseTracking::AnyEvent => true,
    }
}

/// Encode a mouse event for the PTY, or `None` if it should not be reported.
pub fn encode_mouse(event: MouseEvent, modes: MouseModes) -> Option<String> {
    if !wants_event(modes, &event) {
        return None;
    }

    let mut code = button_bits(&event, modes.sgr)?;
    // The motion bit rides on top of the button code.
    if matches!(event.kind, MouseEventKind::Motion) {
        code += 32;
    }
    code += modifier_bits(event.modifiers);

    // Wire coordinates are one-based.
    let column = event.column.saturating_add(1);
    let row = event.row.saturating_add(1);

    if modes.sgr {
        let final_byte = if matches!(event.kind, MouseEventKind::Release) {
            'm'
        } else {
            'M'
        };
        return Some(format!("\x1b[<{code};{column};{row}{final_byte}"));
    }

    if modes.urxvt {
        // urxvt keeps the +32 button bias but writes coordinates as decimal.
        let code = code as usize + LEGACY_COORD_BIAS;
        return Some(format!("\x1b[{code};{column};{row}M"));
    }

    if modes.utf8 {
        let button = char::from_u32(code as u32 + LEGACY_COORD_BIAS as u32)?;
        let column = char::from_u32((column + LEGACY_COORD_BIAS) as u32)?;
        let row = char::from_u32((row + LEGACY_COORD_BIAS) as u32)?;
        return Some(format!("\x1b[M{button}{column}{row}"));
    }

    // Legacy X10 format: single bytes, and silently unreportable past 223.
    if column > LEGACY_COORD_MAX || row > LEGACY_COORD_MAX {
        return None;
    }
    let button = (code as usize + LEGACY_COORD_BIAS) as u8 as char;
    let column = (column + LEGACY_COORD_BIAS) as u8 as char;
    let row = (row + LEGACY_COORD_BIAS) as u8 as char;
    Some(format!("\x1b[M{button}{column}{row}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(button: MouseButton, column: usize, row: usize) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Press,
            button: Some(button),
            column,
            row,
            modifiers: Modifiers::NONE,
        }
    }

    fn modes(tracking: MouseTracking) -> MouseModes {
        MouseModes {
            tracking,
            ..Default::default()
        }
    }

    #[test]
    fn tracking_off_reports_nothing() {
        // With tracking off the terminal owns the mouse: selection and
        // scrollback, nothing on the wire.
        assert_eq!(
            encode_mouse(press(MouseButton::Left, 0, 0), MouseModes::default()),
            None
        );
    }

    #[test]
    fn legacy_press_biases_every_field_by_32() {
        // Button 0, column 1, row 1 -> ' ', '!', '!'
        assert_eq!(
            encode_mouse(press(MouseButton::Left, 0, 0), modes(MouseTracking::X10)).as_deref(),
            Some("\x1b[M \x21\x21")
        );
        assert_eq!(
            encode_mouse(press(MouseButton::Right, 2, 4), modes(MouseTracking::X10)).as_deref(),
            Some("\x1b[M\x22\x23\x25")
        );
    }

    #[test]
    fn legacy_releases_lose_the_button_but_sgr_keeps_it() {
        let release = MouseEvent {
            kind: MouseEventKind::Release,
            button: Some(MouseButton::Right),
            column: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        };

        // Legacy: button 3 ("some button went up"), final byte M.
        assert_eq!(
            encode_mouse(release, modes(MouseTracking::ButtonEvent)).as_deref(),
            Some("\x1b[M\x23\x21\x21")
        );
        // SGR: the real button, final byte m.
        let sgr = MouseModes {
            tracking: MouseTracking::ButtonEvent,
            sgr: true,
            ..Default::default()
        };
        assert_eq!(encode_mouse(release, sgr).as_deref(), Some("\x1b[<2;1;1m"));
    }

    #[test]
    fn sgr_press_uses_decimal_one_based_coordinates() {
        let sgr = MouseModes {
            tracking: MouseTracking::ButtonEvent,
            sgr: true,
            ..Default::default()
        };

        assert_eq!(
            encode_mouse(press(MouseButton::Left, 0, 0), sgr).as_deref(),
            Some("\x1b[<0;1;1M")
        );
        // Well past the 223-column limit of the legacy format.
        assert_eq!(
            encode_mouse(press(MouseButton::Left, 499, 299), sgr).as_deref(),
            Some("\x1b[<0;500;300M")
        );
    }

    #[test]
    fn legacy_format_gives_up_past_its_coordinate_limit() {
        // 223 is the last reportable one-based coordinate; 224 cannot be
        // expressed in a single biased byte, and xterm reports nothing.
        assert!(
            encode_mouse(press(MouseButton::Left, 222, 0), modes(MouseTracking::X10)).is_some()
        );
        assert_eq!(
            encode_mouse(press(MouseButton::Left, 223, 0), modes(MouseTracking::X10)),
            None
        );
        assert_eq!(
            encode_mouse(press(MouseButton::Left, 0, 223), modes(MouseTracking::X10)),
            None
        );
    }

    #[test]
    fn utf8_mode_encodes_large_coordinates_as_chars() {
        let utf8 = MouseModes {
            tracking: MouseTracking::ButtonEvent,
            utf8: true,
            ..Default::default()
        };

        let encoded = encode_mouse(press(MouseButton::Left, 300, 0), utf8).expect("utf8 encodes");
        assert!(encoded.starts_with("\x1b[M"));
        // 301 + 32 = 333, beyond one byte, so it must arrive as a char.
        assert!(encoded.chars().any(|c| c as u32 == 333));
    }

    #[test]
    fn urxvt_mode_writes_decimal_fields() {
        let urxvt = MouseModes {
            tracking: MouseTracking::ButtonEvent,
            urxvt: true,
            ..Default::default()
        };

        assert_eq!(
            encode_mouse(press(MouseButton::Left, 0, 0), urxvt).as_deref(),
            Some("\x1b[32;1;1M")
        );
    }

    #[test]
    fn wheel_buttons_use_the_high_button_codes() {
        let sgr = MouseModes {
            tracking: MouseTracking::ButtonEvent,
            sgr: true,
            ..Default::default()
        };

        assert_eq!(
            encode_mouse(press(MouseButton::WheelUp, 0, 0), sgr).as_deref(),
            Some("\x1b[<64;1;1M")
        );
        assert_eq!(
            encode_mouse(press(MouseButton::WheelDown, 0, 0), sgr).as_deref(),
            Some("\x1b[<65;1;1M")
        );
    }

    #[test]
    fn modifiers_add_their_xterm_bits() {
        let sgr = MouseModes {
            tracking: MouseTracking::ButtonEvent,
            sgr: true,
            ..Default::default()
        };
        let with = |modifiers| MouseEvent {
            modifiers,
            ..press(MouseButton::Left, 0, 0)
        };

        assert_eq!(
            encode_mouse(with(Modifiers::SHIFT), sgr).as_deref(),
            Some("\x1b[<4;1;1M")
        );
        assert_eq!(
            encode_mouse(with(Modifiers::ALT), sgr).as_deref(),
            Some("\x1b[<8;1;1M")
        );
        assert_eq!(
            encode_mouse(with(Modifiers::CTRL), sgr).as_deref(),
            Some("\x1b[<16;1;1M")
        );
        assert_eq!(
            encode_mouse(with(Modifiers::CTRL | Modifiers::SHIFT), sgr).as_deref(),
            Some("\x1b[<20;1;1M")
        );
    }

    #[test]
    fn each_tracking_mode_reports_only_what_it_asked_for() {
        let motion_held = MouseEvent {
            kind: MouseEventKind::Motion,
            button: Some(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        };
        let motion_free = MouseEvent {
            button: None,
            ..motion_held
        };
        let release = MouseEvent {
            kind: MouseEventKind::Release,
            ..motion_held
        };

        // X10: presses only.
        assert!(encode_mouse(press(MouseButton::Left, 0, 0), modes(MouseTracking::X10)).is_some());
        assert!(encode_mouse(release, modes(MouseTracking::X10)).is_none());
        assert!(encode_mouse(motion_held, modes(MouseTracking::X10)).is_none());

        // ButtonEvent: press and release, never motion.
        assert!(encode_mouse(release, modes(MouseTracking::ButtonEvent)).is_some());
        assert!(encode_mouse(motion_held, modes(MouseTracking::ButtonEvent)).is_none());

        // ButtonMotion: motion only while a button is held.
        assert!(encode_mouse(motion_held, modes(MouseTracking::ButtonMotion)).is_some());
        assert!(encode_mouse(motion_free, modes(MouseTracking::ButtonMotion)).is_none());

        // AnyEvent: everything, including free motion.
        assert!(encode_mouse(motion_free, modes(MouseTracking::AnyEvent)).is_some());
    }

    #[test]
    fn motion_sets_the_motion_bit_and_free_motion_uses_button_three() {
        let sgr = MouseModes {
            tracking: MouseTracking::AnyEvent,
            sgr: true,
            ..Default::default()
        };
        let motion_held = MouseEvent {
            kind: MouseEventKind::Motion,
            button: Some(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        };

        // 0 (left) + 32 (motion) = 32
        assert_eq!(
            encode_mouse(motion_held, sgr).as_deref(),
            Some("\x1b[<32;1;1M")
        );
        // 3 (no button) + 32 (motion) = 35
        assert_eq!(
            encode_mouse(
                MouseEvent {
                    button: None,
                    ..motion_held
                },
                sgr
            )
            .as_deref(),
            Some("\x1b[<35;1;1M")
        );
    }

    #[test]
    fn sgr_takes_precedence_over_the_other_extensions() {
        // Applications sometimes enable several at once; SGR is the one that
        // actually round-trips releases and large coordinates.
        let all = MouseModes {
            tracking: MouseTracking::ButtonEvent,
            sgr: true,
            urxvt: true,
            utf8: true,
        };

        assert_eq!(
            encode_mouse(press(MouseButton::Left, 0, 0), all).as_deref(),
            Some("\x1b[<0;1;1M")
        );
    }
}
