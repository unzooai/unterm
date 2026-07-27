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
