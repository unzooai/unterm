//! Chrome typography & spacing tokens.
//!
//! One scale for every piece of window chrome (tab bar, sidebars, status
//! bar, popups) so new UI picks up consistent sizing instead of growing
//! per-site magic numbers. Pixel-ish values are in points; multiply by
//! `dpi / 72.0` at the render site.

/// Chrome text: tabs, sidebar rows, status bar. Keep this close to the
/// terminal font while using the title/UI font's natural weight; oversizing
/// chrome makes the app read less precise than Warp. Nudged to 13.5 so the
/// sidebar/tab labels read a touch larger and crisper without ballooning the
/// chrome.
pub const UI_FONT_SIZE: f64 = 13.5;
/// Command palette / modal body text.
pub const PALETTE_FONT_SIZE: f64 = 14.0;
/// Small overline / badge text.
pub const OVERLINE_FONT_SIZE: f64 = 10.0;
/// Modal / section header text.
pub const HEADER_FONT_SIZE: f64 = 18.0;
/// Line-height ratio for chrome text. A touch loose (vs 1.2) so sidebar/tab
/// rows breathe instead of reading cramped.
pub const UI_LINE_HEIGHT: f64 = 1.35;
/// Corner radius for selectable rows and buttons.
pub const CORNER_RADIUS: f32 = 4.0;
/// Padding inside selectable rows. Generous so sidebar/tab rows don't read
/// cramped, especially at the slightly larger UI font size.
pub const ROW_PADDING: f32 = 10.0;
/// Width reserved for the macOS traffic-light cluster in the tab bar.
pub const MACOS_TRAFFIC_LIGHT_RESERVE: f32 = 76.0;
/// Diameter of the custom-drawn macOS traffic-light dots.
pub const MACOS_TRAFFIC_LIGHT_DOT: f32 = 12.0;
/// Extra vertical breathing room around the bottom status-bar text.
pub const STATUS_BAR_VERTICAL_PADDING: f32 = 2.0;
/// Visual baseline compensation for one-line chrome text. Terminal cells
/// include descender space, so geometric centering reads slightly low.
pub const CHROME_TEXT_BASELINE_NUDGE: f32 = -2.0;
/// Width of chrome scrollbars; rendered with a minimum physical width.
pub const CHROME_SCROLLBAR_WIDTH: f32 = 5.0;
/// Minimum physical scrollbar width, to keep HiDPI and low-DPI output aligned.
pub const CHROME_SCROLLBAR_MIN_WIDTH: f32 = 6.0;
/// Minimum scrollbar thumb height.
pub const CHROME_SCROLLBAR_MIN_THUMB_HEIGHT: f32 = 28.0;
/// Faint track opacity behind chrome/pane scrollbars.
pub const CHROME_SCROLLBAR_TRACK_ALPHA: f32 = 0.16;
/// Thumb opacity for theme-provided scrollbar colors.
pub const CHROME_SCROLLBAR_THUMB_ALPHA: f32 = 0.74;

/// Left tab bar geometry.
pub const LEFT_TAB_BAR_WIDTH: f32 = 164.0;
pub const LEFT_TAB_BAR_MIN_WIDTH: f32 = 112.0;
/// Max width as a fraction of the window width.
pub const LEFT_TAB_BAR_MAX_RATIO: f32 = 0.30;
/// Width of the resize grip on the bar's right edge.
pub const LEFT_TAB_BAR_GRIP: f32 = 12.0;

/// Directory tree sidebar geometry.
pub const TREE_SIDEBAR_WIDTH: f32 = 152.0;
pub const TREE_SIDEBAR_MIN_WIDTH: f32 = 112.0;
pub const TREE_SIDEBAR_MAX_RATIO: f32 = 0.30;
pub const TREE_SIDEBAR_GRIP: f32 = 12.0;
/// Combined left chrome should never dominate the terminal area.
pub const LEFT_GUTTER_MAX_RATIO: f32 = 0.42;

/// Right-docked source-control (git) panel geometry.
pub const GIT_PANEL_WIDTH: f32 = 232.0;
pub const GIT_PANEL_MIN_WIDTH: f32 = 160.0;
/// Max width as a fraction of the window width.
pub const GIT_PANEL_MAX_RATIO: f32 = 0.32;
