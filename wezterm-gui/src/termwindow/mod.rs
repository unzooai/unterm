#![allow(clippy::range_plus_one)]
use super::renderstate::*;
use super::utilsprites::RenderMetrics;
use crate::colorease::ColorEase;
use crate::engine::{
    CreateSessionRequest, EngineRenderBufferBatch, EngineRenderConsumerSet,
    EngineRenderViewportPlacement, InputEngine, LaunchPolicySnapshot, NextCorePaneBindings,
    RenderCellMetrics, SessionEngine,
};
use crate::frontend::{front_end, try_front_end};
use crate::inputmap::InputMap;
use crate::overlay::{
    confirm_close_pane, confirm_close_tab, confirm_close_window, confirm_quit_program, launcher,
    start_overlay, start_overlay_pane, CopyModeParams, CopyOverlay, LauncherArgs, LauncherFlags,
    QuickSelectOverlay,
};
use crate::resize_increment_calculator::ResizeIncrementCalculator;
use crate::scripting::guiwin::GuiWin;
use crate::scrollbar::*;
use crate::selection::Selection;
use crate::shapecache::*;
use crate::tabbar::{detect_shell_icon, TabBarItem, TabBarState};
use crate::termwindow::background::{
    load_background_image, reload_background_image, LoadedBackgroundLayer,
};
use crate::termwindow::keyevent::{KeyTableArgs, KeyTableState};
use crate::termwindow::modal::Modal;
use crate::termwindow::render::paint::AllowImage;
use crate::termwindow::render::{
    CachedLineState, LineQuadCacheKey, LineQuadCacheValue, LineToEleShapeCacheKey,
    LineToElementShapeItem,
};
use crate::termwindow::webgpu::{NextCoreWebGpuPaneDrawFrame, WebGpuState};
use ::wezterm_term::input::{ClickPosition, MouseButton as TMB};
use ::window::*;
use anyhow::{anyhow, ensure, Context};
use config::keyassignment::{
    Confirmation, KeyAssignment, LauncherActionArgs, PaneDirection, Pattern, PromptInputLine,
    QuickSelectArguments, RotationDirection, SpawnCommand, SplitSize,
};
use config::window::WindowLevel;
use config::{
    configuration, AudibleBell, ConfigHandle, Dimension, DimensionContext, FrontEndSelection,
    GeometryOrigin, GuiPosition, TermConfig, WindowCloseConfirmation,
};
use lfucache::*;
use mlua::{FromLua, LuaSerdeExt, UserData, UserDataFields};
use mux::pane::{
    CachePolicy, CloseReason, Pane, PaneId, Pattern as MuxPattern, PerformAssignmentResult,
};
use mux::renderable::RenderableDimensions;
use mux::tab::{
    PositionedPane, PositionedSplit, SplitDirection, SplitRequest, SplitSize as MuxSplitSize, Tab,
    TabId,
};
use mux::window::WindowId as MuxWindowId;
use mux::{Mux, MuxNotification};
use mux_lua::MuxPane;
use smol::channel::Sender;
use smol::Timer;
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, LinkedList};
use std::io::Write as _;
use std::ops::Add;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use termwiz::hyperlink::Hyperlink;
use termwiz::surface::SequenceNo;
use wezterm_dynamic::Value;
use wezterm_font::FontConfiguration;
use wezterm_term::color::ColorPalette;
use wezterm_term::input::LastMouseClick;
use wezterm_term::{Alert, Progress, StableRowIndex, TerminalConfiguration, TerminalSize};

pub mod background;
pub mod box_model;
pub mod charselect;
pub mod chrome_colors;
pub mod clipboard;
pub mod composer;
pub mod cockpit_inbox;
pub mod fleet_palette;
pub mod dir_jump;
pub mod keyevent;
pub mod left_tab_bar;
pub mod modal;
pub(crate) mod mouseevent;
pub mod palette;
pub mod paneselect;
pub mod popup_menu;
mod prevcursor;
pub mod render;
pub mod resize;
mod selection;
pub mod git_panel;
pub(crate) mod sidebar_text;
pub mod spawn;
pub mod top_stats_bar;
pub mod tree_sidebar;
pub mod ui_icons;
pub mod webgpu;
use crate::spawn::SpawnWhere;
use prevcursor::PrevCursorPos;

const ATLAS_SIZE: usize = 256;

lazy_static::lazy_static! {
    static ref WINDOW_CLASS: Mutex<String> = Mutex::new(wezterm_gui_subcommands::DEFAULT_WINDOW_CLASS.to_owned());
    static ref POSITION: Mutex<Option<GuiPosition>> = Mutex::new(None);
    /// Saved window dimensions (width, height in pixels) from last session.
    static ref SAVED_DIMENSIONS: Mutex<Option<(usize, usize)>> = Mutex::new(None);
}

pub const ICON_DATA: &'static [u8] = include_bytes!("../../../assets/icon/terminal.png");

pub fn set_window_position(pos: GuiPosition) {
    POSITION.lock().unwrap().replace(pos);
}

/// Set saved window dimensions (pixel width, pixel height) to be applied
/// on the next window creation. Used for session state restoration.
pub fn set_saved_dimensions(width: usize, height: usize) {
    SAVED_DIMENSIONS.lock().unwrap().replace((width, height));
}

pub fn set_window_class(cls: &str) {
    *WINDOW_CLASS.lock().unwrap() = cls.to_owned();
}

pub fn get_window_class() -> String {
    WINDOW_CLASS.lock().unwrap().clone()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MouseCapture {
    UI,
    TerminalPane(PaneId),
}

/// Type used together with Window::notify to do something in the
/// context of the window-specific event loop
pub enum TermWindowNotif {
    InvalidateShapeCache,
    PerformAssignment {
        pane_id: PaneId,
        assignment: KeyAssignment,
        tx: Option<Sender<anyhow::Result<()>>>,
    },
    SetLeftStatus(String),
    SetRightStatus(String),
    GetDimensions(Sender<(Dimensions, WindowState)>),
    GetSelectionForPane {
        pane_id: PaneId,
        tx: Sender<String>,
    },
    GetEffectiveConfig(Sender<ConfigHandle>),
    FinishWindowEvent {
        name: String,
        again: bool,
    },
    GetConfigOverrides(Sender<wezterm_dynamic::Value>),
    SetConfigOverrides(wezterm_dynamic::Value),
    CancelOverlayForPane(PaneId),
    CancelOverlayForTab {
        tab_id: TabId,
        pane_id: Option<PaneId>,
    },
    MuxNotification(MuxNotification),
    EmitStatusUpdate,
    Apply(Box<dyn FnOnce(&mut TermWindow) + Send + Sync>),
    SwitchToMuxWindow(MuxWindowId),
    SetInnerSize {
        width: usize,
        height: usize,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum QuickAction {
    CommandPalette,
    TreeSidebar,
    SplitRight,
    DirJump,
    Search,
    Settings,
    Inbox,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RightClickAction {
    CopySelection,
    PasteClipboard,
}

fn right_click_action(has_selection: bool) -> RightClickAction {
    if has_selection {
        RightClickAction::CopySelection
    } else {
        RightClickAction::PasteClipboard
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UIItemType {
    TabBar(TabBarItem),
    CloseTab(usize),
    AboveScrollThumb,
    ScrollThumb,
    BelowScrollThumb,
    Split(PositionedSplit),
    StatusBarProject,
    /// Current working directory of the active pane (left-click = copy to clipboard).
    StatusBarCwd,
    StatusBarTheme,
    /// Identity profile bound to this window. Left-click spawns a new
    /// Unterm window in the next profile (cycling through `index.toml`
    /// order). Right-click would open a picker overlay — TBD in v0.14.
    StatusBarProfile,
    /// Region screenshot, hide-Unterm-window mode (left-click = trigger).
    StatusBarCaptureExclude,
    /// Region screenshot, include-Unterm-window mode (left-click = trigger).
    StatusBarCaptureInclude,
    StatusBarProxy,
    /// MCP activity indicator: shows `mcp:N` (and a `⚡` flash for ~5s
    /// after each write). Click opens the audit log overlay so the
    /// user can see *what* an AI client just wrote to a pane.
    StatusBarMcpAudit,
    /// `×` button rendered in the top-right corner of every pane when the
    /// active tab has 2+ panes. Click closes the specific pane.
    CloseSplitPane(mux::pane::PaneId),
    /// A selectable row inside the mouse-operable popup menu (v0.40).
    PopupMenuRow(usize),
    /// A selectable row inside the directory-jump palette (v0.40).
    DirJumpRow(usize),
    CockpitInboxRow(usize),
    FleetPaletteRow(usize),
    ScrollToBottom(PaneId),
    /// Scroll track inside the directory-jump palette.
    DirJumpScrollTrack {
        thumb_height: usize,
    },
    /// Draggable scroll thumb inside the directory-jump palette.
    DirJumpScrollThumb {
        track_top: usize,
        track_height: usize,
    },
    /// A row in the left directory-tree sidebar (v0.40).
    TreeSidebarRow(usize),
    /// The tree sidebar's background (swallows clicks, accepts wheel).
    TreeSidebarBg,
    /// The right-docked git panel's background (swallows clicks so they
    /// don't reach the pane; read-only MVP has no interactive rows yet).
    GitPanelBg,
    /// Resize grip on the directory tree sidebar's right edge.
    TreeSidebarResize,
    /// Scroll track inside the directory tree sidebar.
    TreeSidebarScrollTrack {
        row_count: usize,
        visible_rows: usize,
        thumb_height: usize,
    },
    /// Draggable scroll thumb inside the directory tree sidebar.
    TreeSidebarScrollThumb {
        row_count: usize,
        visible_rows: usize,
        track_top: usize,
        track_height: usize,
    },
    /// A tab row in the left vertical tab bar. Click activates; keep
    /// dragging to reorder; right-click menus.
    LeftTabBarTab(usize),
    /// A project group header in the left vertical tab bar. Click toggles its
    /// collapsed state; the string is the normalized full project identity.
    LeftTabBarGroup(String),
    /// Resize grip on the left tab bar's right edge.
    LeftTabBarResize,
    /// Scroll track inside the left tab bar. Click jumps the thumb;
    /// wheel scrolls rows.
    LeftTabBarScrollTrack {
        row_count: usize,
        visible_rows: usize,
        thumb_height: usize,
    },
    /// Draggable scroll thumb inside the left tab bar.
    LeftTabBarScrollThumb {
        row_count: usize,
        visible_rows: usize,
        track_top: usize,
        track_height: usize,
    },
    /// The left tab bar's background (swallows clicks, accepts wheel).
    LeftTabBarBg,
    /// Search all tabs/projects with the fuzzy tab navigator.
    LeftTabBarSearch,
    /// The tree sidebar's header (root name); click re-anchors the root to
    /// the active pane's cwd.
    TreeSidebarHeader,
    /// Top-bar quick action buttons (v0.40 "C").
    QuickAction(QuickAction),
    /// The ▾ chevron next to the sidebar's "+" row. Click opens the shell
    /// selector — same surface as right-click on "+" / Ctrl+Shift+N, just
    /// visually discoverable. Left-click stays bound to "+" itself, which
    /// keeps default-shell-new-tab a single click.
    NewTabShellSelector,
    /// "DO AI PM" footer row at the bottom of the left tab bar.
    /// Click opens the author site (doaipm.com) in the OS browser.
    LeftTabBarAuthorLink,
    /// The popup menu card itself — swallows clicks that miss every row so
    /// they don't fall through to the pane below.
    PopupMenuCard,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UIItem {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub pane_id: Option<PaneId>,
    pub item_type: UIItemType,
}

impl UIItem {
    pub fn hit_test(&self, x: isize, y: isize) -> bool {
        x >= self.x as isize
            && x <= (self.x + self.width) as isize
            && y >= self.y as isize
            && y <= (self.y + self.height) as isize
    }
}

#[derive(Clone, Default)]
pub struct SemanticZoneCache {
    seqno: SequenceNo,
    zones: Vec<StableRowIndex>,
}

pub struct OverlayState {
    pub pane: Arc<dyn Pane>,
    pub key_table_state: KeyTableState,
}

#[derive(Default)]
pub struct PaneState {
    /// If is_some(), the top row of the visible screen.
    /// Otherwise, the viewport is at the bottom of the
    /// scrollback.
    viewport: Option<StableRowIndex>,
    selection: Selection,
    /// If is_some(), rather than display the actual tab
    /// contents, we're overlaying a little internal application
    /// tab.  We'll also route input to it.
    pub overlay: Option<OverlayState>,

    bell_start: Option<Instant>,
    pub mouse_terminal_coords: Option<(ClickPosition, StableRowIndex)>,
}

/// Data used when synchronously formatting pane and window titles
#[derive(Debug, Clone)]
pub struct TabInformation {
    pub tab_id: TabId,
    pub tab_index: usize,
    pub is_active: bool,
    pub is_last_active: bool,
    pub active_pane: Option<PaneInformation>,
    pub window_id: MuxWindowId,
    pub tab_title: String,
}

impl UserData for TabInformation {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("tab_id", |_, this| Ok(this.tab_id));
        fields.add_field_method_get("tab_index", |_, this| Ok(this.tab_index));
        fields.add_field_method_get("is_active", |_, this| Ok(this.is_active));
        fields.add_field_method_get("is_last_active", |_, this| Ok(this.is_last_active));
        fields.add_field_method_get("active_pane", |_, this| {
            if let Some(pane) = &this.active_pane {
                Ok(Some(pane.clone()))
            } else {
                Ok(None)
            }
        });
        fields.add_field_method_get("panes", |_, this| {
            let mux = Mux::get();
            let mut panes = vec![];
            if let Some(tab) = mux.get_tab(this.tab_id) {
                panes = tab
                    .iter_panes()
                    .iter()
                    .map(TermWindow::pos_pane_to_pane_info)
                    .collect();
            }
            Ok(panes)
        });
        fields.add_field_method_get("window_id", |_, this| Ok(this.window_id));
        fields.add_field_method_get("tab_title", |_, this| Ok(this.tab_title.clone()));
        fields.add_field_method_get("window_title", |_, this| {
            let mux = Mux::get();
            let window = mux.get_window(this.window_id).ok_or_else(|| {
                mlua::Error::external(format!("window {} not found", this.window_id))
            })?;
            Ok(window.get_title().to_string())
        });
    }
}

/// Data used when synchronously formatting pane and window titles
#[derive(Debug, Clone)]
pub struct PaneInformation {
    pub pane_id: PaneId,
    pub pane_index: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
    pub has_unseen_output: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
    pub title: String,
    pub user_vars: HashMap<String, String>,
    pub progress: Progress,
}

impl UserData for PaneInformation {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("pane_id", |_, this| Ok(this.pane_id));
        fields.add_field_method_get("pane_index", |_, this| Ok(this.pane_index));
        fields.add_field_method_get("is_active", |_, this| Ok(this.is_active));
        fields.add_field_method_get("is_zoomed", |_, this| Ok(this.is_zoomed));
        fields.add_field_method_get("has_unseen_output", |_, this| Ok(this.has_unseen_output));
        fields.add_field_method_get("left", |_, this| Ok(this.left));
        fields.add_field_method_get("top", |_, this| Ok(this.top));
        fields.add_field_method_get("width", |_, this| Ok(this.width));
        fields.add_field_method_get("height", |_, this| Ok(this.height));
        fields.add_field_method_get("pixel_width", |_, this| Ok(this.pixel_width));
        fields.add_field_method_get("pixel_height", |_, this| Ok(this.pixel_height));
        fields.add_field_method_get("progress", |lua, this| lua.to_value(&this.progress));
        fields.add_field_method_get("title", |_, this| Ok(this.title.clone()));
        fields.add_field_method_get("user_vars", |_, this| Ok(this.user_vars.clone()));
        fields.add_field_method_get("foreground_process_name", |_, this| {
            let mut name = None;
            if let Some(mux) = Mux::try_get() {
                if let Some(pane) = mux.get_pane(this.pane_id) {
                    name = pane.get_foreground_process_name(CachePolicy::AllowStale);
                }
            }
            match name {
                Some(name) => Ok(name),
                None => Ok("".to_string()),
            }
        });
        fields.add_field_method_get("tty_name", |_, this| {
            let mut name = None;
            if let Some(mux) = Mux::try_get() {
                if let Some(pane) = mux.get_pane(this.pane_id) {
                    name = pane.tty_name();
                }
            }
            Ok(name)
        });
        fields.add_field_method_get("current_working_dir", |_, this| {
            if let Some(mux) = Mux::try_get() {
                if let Some(pane) = mux.get_pane(this.pane_id) {
                    return Ok(pane
                        .get_current_working_dir(CachePolicy::AllowStale)
                        .map(|url| url_funcs::Url { url }));
                }
            }
            Ok(None)
        });
        fields.add_field_method_get("domain_name", |_, this| {
            let mut name = None;
            if let Some(mux) = Mux::try_get() {
                if let Some(pane) = mux.get_pane(this.pane_id) {
                    let domain_id = pane.domain_id();
                    name = mux
                        .get_domain(domain_id)
                        .map(|dom| dom.domain_name().to_string());
                }
            }
            match name {
                Some(name) => Ok(name),
                None => Ok("".to_string()),
            }
        });
    }
}

#[derive(Default)]
pub struct TabState {
    /// If is_some(), rather than display the actual tab
    /// contents, we're overlaying a little internal application
    /// tab.  We'll also route input to it.
    pub overlay: Option<OverlayState>,
}

/// Manages the state/queue of lua based event handlers.
/// We don't want to queue more than 1 event at a time,
/// so we use this enum to allow for at most 1 executing
/// and 1 pending event.
#[derive(Copy, Clone, Debug)]
enum EventState {
    /// The event is not running
    None,
    /// The event is running
    InProgress,
    /// The event is running, and we have another one ready to
    /// run once it completes
    InProgressWithQueued(Option<PaneId>),
}

pub struct TermWindow {
    pub window: Option<Window>,
    pub config: ConfigHandle,
    pub config_overrides: wezterm_dynamic::Value,
    os_parameters: Option<parameters::Parameters>,
    /// When we most recently received keyboard focus
    pub focused: Option<Instant>,
    fonts: Rc<FontConfiguration>,
    /// Window dimensions and dpi
    pub dimensions: Dimensions,
    pub window_state: WindowState,
    pub resizes_pending: usize,
    is_repaint_pending: bool,
    pending_scale_changes: LinkedList<resize::ScaleChange>,
    /// Terminal dimensions
    terminal_size: TerminalSize,
    pub mux_window_id: MuxWindowId,
    pub mux_window_id_for_subscriptions: Arc<Mutex<MuxWindowId>>,
    pub render_metrics: RenderMetrics,
    render_state: Option<RenderState>,
    input_map: InputMap,
    /// If is_some, the LEADER modifier is active until the specified instant.
    leader_is_down: Option<std::time::Instant>,
    dead_key_status: DeadKeyStatus,
    key_table_state: KeyTableState,
    show_tab_bar: bool,
    show_scroll_bar: bool,
    tab_bar: TabBarState,
    fancy_tab_bar: Option<box_model::ComputedElement>,
    /// Top-stats text (git/cpu/mem/uptime) as of the last periodic
    /// title update. It renders inside the fancy tab bar but is not
    /// part of TabBarState, so the `new_tab_bar != self.tab_bar`
    /// check can't see it change; without tracking it separately the
    /// chrome stats freeze at whatever was last painted.
    last_top_stats_text: String,
    pub right_status: String,
    pub left_status: String,
    /// Short, non-destructive feedback rendered in the existing bottom bar.
    /// Unlike the old PTY-injected messages this cannot corrupt full-screen
    /// TUIs and expires without adding another permanent chrome row.
    pub(crate) ui_notice: RefCell<Option<(String, Instant)>>,
    last_ui_item: Option<UIItem>,
    /// Tracks whether the current mouse-down event is part of click-focus.
    /// If so, we ignore mouse events until released
    is_click_to_focus_window: bool,
    last_mouse_coords: (usize, i64),
    window_drag_position: Option<MouseEvent>,
    current_mouse_event: Option<MouseEvent>,
    prev_cursor: PrevCursorPos,
    last_scroll_info: RenderableDimensions,

    tab_state: RefCell<HashMap<TabId, TabState>>,
    pane_state: RefCell<HashMap<PaneId, PaneState>>,
    next_core_render_consumers: RefCell<EngineRenderConsumerSet>,
    /// GUI pane id -> next-core session id. Pane ids and next-core session ids
    /// come from independent allocators and overlap numerically, so the render
    /// and input paths must resolve through this map rather than passing a raw
    /// pane id to the engine.
    next_core_pane_bindings: RefCell<NextCorePaneBindings>,
    semantic_zones: HashMap<PaneId, SemanticZoneCache>,

    /// True after a Ctrl+Left press was consumed as a macOS secondary click;
    /// the remaining Drag/Release of that physical gesture is swallowed so it
    /// cannot also trigger the default Left-button selection/link bindings.
    swallow_left_gesture_after_secondary_click: bool,

    window_background: Vec<LoadedBackgroundLayer>,

    current_modifier_and_leds: (Modifiers, KeyboardLedStatus),
    current_mouse_buttons: Vec<MousePress>,
    current_mouse_capture: Option<MouseCapture>,

    opengl_info: Option<String>,

    /// Keeps track of double and triple clicks
    last_mouse_click: Option<LastMouseClick>,

    /// The URL over which we are currently hovering
    current_highlight: Option<Arc<Hyperlink>>,

    quad_generation: usize,
    shape_generation: usize,
    shape_cache: RefCell<LfuCache<ShapeCacheKey, anyhow::Result<Rc<Vec<ShapedInfo>>>>>,
    line_to_ele_shape_cache: RefCell<LfuCache<LineToEleShapeCacheKey, LineToElementShapeItem>>,

    line_state_cache: RefCell<LfuCacheU64<Arc<CachedLineState>>>,
    next_line_state_id: u64,

    line_quad_cache: RefCell<LfuCache<LineQuadCacheKey, LineQuadCacheValue>>,

    last_status_call: Instant,
    cursor_blink_state: RefCell<ColorEase>,
    blink_state: RefCell<ColorEase>,
    rapid_blink_state: RefCell<ColorEase>,

    palette: Option<ColorPalette>,

    ui_items: Vec<UIItem>,
    dragging: Option<(UIItem, MouseEvent)>,

    modal: RefCell<Option<Rc<dyn Modal>>>,
    /// Composer + prompt-queue state; persists across opening/closing the
    /// overlay within a session (see `composer.rs`).
    composer: RefCell<crate::termwindow::composer::ComposerState>,
    prewarmed_settings_menu: RefCell<Option<Rc<crate::termwindow::popup_menu::PopupMenu>>>,
    /// v0.40: left directory-tree sidebar; None = closed.
    pub(crate) tree_sidebar: RefCell<Option<crate::termwindow::tree_sidebar::TreeSidebar>>,
    /// Right-docked source-control (git) panel; None = closed.
    pub(crate) git_panel: RefCell<Option<crate::termwindow::git_panel::GitPanel>>,
    /// Left vertical tab bar state; active when tab_bar_position = Left.
    pub(crate) left_tab_bar: RefCell<crate::termwindow::left_tab_bar::LeftTabBar>,
    /// Scrollbar fills deferred until after the splits are painted, so the
    /// divider-riding inner-pane bar isn't overdrawn by the split line
    /// (same GL layer — later draw wins).
    pub(crate) deferred_scrollbar: RefCell<Vec<(::window::RectF, ::window::color::LinearRgba)>>,

    event_states: HashMap<String, EventState>,
    pub current_event: Option<Value>,
    has_animation: RefCell<Option<Instant>>,
    /// We use this to attempt to do something reasonable
    /// if we run out of texture space
    allow_images: AllowImage,
    scheduled_animation: RefCell<Option<Instant>>,

    created: Instant,

    pub last_frame_duration: Duration,
    last_fps_check_time: Instant,
    num_frames: usize,
    pub fps: f32,

    connection_name: String,

    gl: Option<Rc<glium::backend::Context>>,
    webgpu: Option<Rc<WebGpuState>>,
    config_subscription: Option<config::ConfigSubscription>,
}

/// Read the pane's current working directory and turn it into a local
/// filesystem PathBuf — used as the starting location for folder pickers,
/// so the dialog opens at where the user is *now*, not at Documents/My Computer.
/// Returns None when cwd is unknown (not a file:// URL, or remote SSH pane).
fn pane_cwd_path(pane: &Arc<dyn mux::pane::Pane>) -> Option<std::path::PathBuf> {
    let url = pane.get_current_working_dir(mux::pane::CachePolicy::AllowStale)?;
    url.to_file_path().ok()
}

/// Build the next-core session request for a GUI pane.
///
/// The pane's own geometry drives cols/rows so next-core wraps at the width
/// the user actually sees, and the proxy env matches what `spawn` injects into
/// a WezTerm pane — a next-core-backed pane must not silently bypass the
/// user's proxy configuration. No command is set: next-core falls back to the
/// platform default shell, the same as an MCP `session.create` without one.
fn next_core_pane_session_request(
    cols: usize,
    rows: usize,
    cwd: Option<String>,
) -> CreateSessionRequest {
    CreateSessionRequest {
        cols: cols.max(1),
        rows: rows.max(1),
        command_dir: cwd,
        command: None,
        env: crate::spawn::read_unterm_proxy_env().unwrap_or_default(),
        launch_policy: LaunchPolicySnapshot::default(),
    }
}

/// Translate a GUI mouse event into next-core's input form.
///
/// Returns `None` for events next-core has no encoding for, which the caller
/// treats as "nothing to report" rather than as a reason to fall back to the
/// legacy pane.
pub(crate) fn next_core_mouse_event(
    event: &wezterm_term::MouseEvent,
) -> Option<unterm_engine::next_core::mouse_encoding::MouseEvent> {
    use unterm_engine::next_core::mouse_encoding::{MouseButton, MouseEventKind};
    use wezterm_term::{MouseButton as TMB, MouseEventKind as TMEK};

    let kind = match event.kind {
        TMEK::Press => MouseEventKind::Press,
        TMEK::Release => MouseEventKind::Release,
        TMEK::Move => MouseEventKind::Motion,
    };
    let button = match event.button {
        TMB::Left => Some(MouseButton::Left),
        TMB::Middle => Some(MouseButton::Middle),
        TMB::Right => Some(MouseButton::Right),
        TMB::WheelUp(_) => Some(MouseButton::WheelUp),
        TMB::WheelDown(_) => Some(MouseButton::WheelDown),
        TMB::WheelLeft(_) => Some(MouseButton::WheelLeft),
        TMB::WheelRight(_) => Some(MouseButton::WheelRight),
        // Motion with nothing held; the encoder reports it as "no button".
        TMB::None => None,
    };

    Some(unterm_engine::next_core::mouse_encoding::MouseEvent {
        kind,
        button,
        // Both sides are zero-based, pane-relative cell coordinates.
        column: event.x,
        row: event.y.max(0) as usize,
        modifiers: event.modifiers,
    })
}

fn posix_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(windows)]
fn powershell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[cfg(windows)]
fn cmd_double_quote(s: &str) -> String {
    // Windows paths cannot contain `"`, so quote for spaces and escape `%`
    // to avoid accidental environment expansion in cmd.exe.
    format!("\"{}\"", s.replace('%', "^%"))
}

fn cd_command_for_pane(pane: &Arc<dyn mux::pane::Pane>, path: &std::path::Path) -> String {
    let raw = path.display().to_string();
    #[cfg(not(windows))]
    let _ = pane;

    #[cfg(windows)]
    {
        let shell = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .or_else(|| pane.get_foreground_process_name(mux::pane::CachePolicy::FetchImmediate))
            .unwrap_or_default()
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();

        // End with a bare CR (`\r`) — exactly the byte the Enter key sends.
        // A trailing `\n` makes PowerShell's PSReadLine treat the write as
        // multi-line *pasted* text and insert it literally instead of running
        // it, so the user had to press Enter to actually cd.
        if shell == "cmd.exe" || shell == "cmd" {
            return format!("cd /d {}\r", cmd_double_quote(&raw));
        }

        if shell == "powershell.exe"
            || shell == "powershell"
            || shell == "pwsh.exe"
            || shell == "pwsh"
        {
            return format!("Set-Location -LiteralPath {}\r", powershell_single_quote(&raw));
        }

        if shell == "nu.exe" || shell == "nu" {
            return format!("cd {}\r", powershell_single_quote(&raw));
        }
    }

    format!("cd {}\r", posix_single_quote(&raw))
}

impl TermWindow {
    fn load_os_parameters(&mut self) {
        if let Some(ref window) = self.window {
            self.os_parameters = match window.get_os_parameters(&self.config, self.window_state) {
                Ok(os_parameters) => os_parameters,
                Err(err) => {
                    log::warn!("Error while getting OS parameters: {:#}", err);
                    None
                }
            };
        }
    }

    /// Save window geometry and tab state to `~/.unterm/last_session.json`.
    fn save_session_state(&self, window: &Window) {
        let pos = window.get_window_position();
        let dims = &self.dimensions;

        // Collect tab CWDs
        let mux = Mux::get();
        let mut tabs = Vec::new();
        if let Some(mux_window) = mux.get_window(self.mux_window_id) {
            for tab in mux_window.iter() {
                let cwd = tab.get_active_pane().and_then(|pane| {
                    pane.get_current_working_dir(CachePolicy::AllowStale)
                        .map(|u| u.to_string())
                });
                let title = tab.get_title();
                tabs.push(crate::session_state::TabState { cwd, title });
            }
        }

        let (x, y) = match pos {
            Some(p) => (p.x as i32, p.y as i32),
            None => (0, 0),
        };

        let state = crate::session_state::SessionState {
            x,
            y,
            width: dims.pixel_width,
            height: dims.pixel_height,
            dpi: dims.dpi,
            tabs,
            saved_at: chrono::Local::now().to_rfc3339(),
        };

        if let Err(e) = crate::session_state::save_session_state(&state) {
            log::error!("Failed to save session state: {:#}", e);
        }
    }

    fn close_requested(&mut self, window: &Window) {
        // Save session state before the window is destroyed
        self.save_session_state(window);

        let mux = Mux::get();
        match self.config.window_close_confirmation {
            WindowCloseConfirmation::NeverPrompt => {
                // Immediately kill the tabs and allow the window to close
                mux.kill_window(self.mux_window_id);
                window.close();
                front_end().forget_known_window(window);
            }
            WindowCloseConfirmation::AlwaysPrompt => {
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => {
                        mux.kill_window(self.mux_window_id);
                        window.close();
                        front_end().forget_known_window(window);
                        return;
                    }
                };

                let mux_window_id = self.mux_window_id;

                let can_close = mux
                    .get_window(mux_window_id)
                    .map_or(false, |w| w.can_close_without_prompting());
                if can_close {
                    mux.kill_window(self.mux_window_id);
                    window.close();
                    front_end().forget_known_window(window);
                    return;
                }
                let window = self.window.clone().unwrap();
                let (overlay, future) = start_overlay(self, &tab, move |tab_id, term| {
                    confirm_close_window(term, mux_window_id, window, tab_id)
                });
                self.assign_overlay(tab.tab_id(), overlay);
                promise::spawn::spawn(future).detach();

                // Don't close right now; let the close happen from
                // the confirmation overlay
            }
        }
    }

    fn focus_changed(&mut self, focused: bool, window: &Window) {
        log::trace!("Setting focus to {:?}", focused);
        self.focused = if focused { Some(Instant::now()) } else { None };
        self.quad_generation += 1;
        self.load_os_parameters();

        if self.focused.is_none() {
            self.last_mouse_click = None;
            self.current_mouse_buttons.clear();
            self.current_mouse_capture = None;
            self.is_click_to_focus_window = false;

            for state in self.pane_state.borrow_mut().values_mut() {
                state.mouse_terminal_coords.take();
            }
        }

        // Reset the cursor blink phase
        self.prev_cursor.bump();

        // force cursor to be repainted
        window.invalidate();

        if let Some(pane) = self.get_active_pane_or_overlay() {
            pane.focus_changed(focused);
        }

        self.update_title();
        self.emit_window_event("window-focus-changed", None);
    }

    fn created(&mut self, ctx: RenderContext) -> anyhow::Result<()> {
        self.render_state = None;

        let render_info = ctx.renderer_info();
        self.opengl_info.replace(render_info.clone());

        match RenderState::new(ctx, &self.fonts, &self.render_metrics, ATLAS_SIZE) {
            Ok(render_state) => {
                log::debug!(
                    "OpenGL initialized! {} wezterm version: {}",
                    render_info,
                    config::wezterm_version(),
                );
                self.render_state.replace(render_state);
            }
            Err(err) => {
                log::error!("failed to create RenderState: {}", err);
            }
        }

        if self.render_state.is_none() {
            panic!("No OpenGL");
        }

        Ok(())
    }
}

impl TermWindow {
    pub async fn new_window(mux_window_id: MuxWindowId) -> anyhow::Result<()> {
        crate::startup_timing::mark("TermWindow::new_window enter");
        let startup_span = std::time::Instant::now();
        let config = configuration();
        let dpi = config.dpi.unwrap_or_else(|| ::window::default_dpi()) as usize;
        let fontconfig = Rc::new(FontConfiguration::new(Some(config.clone()), dpi)?);
        log::debug!(
            "startup-span: font config ready {:?}",
            startup_span.elapsed()
        );

        let mux = Mux::get();
        let size = match mux.get_active_tab_for_window(mux_window_id) {
            Some(tab) => tab.get_size(),
            None => {
                log::debug!("new_window has no tabs... yet?");
                Default::default()
            }
        };
        let physical_rows = size.rows as usize;
        let physical_cols = size.cols as usize;

        let render_metrics = RenderMetrics::new(&fontconfig)?;
        log::trace!("using render_metrics {:#?}", render_metrics);
        log::debug!("startup-span: render metrics {:?}", startup_span.elapsed());

        // Initially we have only a single tab, so take that into account
        // for the tab bar state.
        let show_tab_bar = config.enable_tab_bar && !config.hide_tab_bar_if_only_one_tab;
        let tab_bar_height = if show_tab_bar {
            Self::tab_bar_pixel_height_impl(&config, &fontconfig, &render_metrics)? as usize
        } else {
            0
        };

        let terminal_size = TerminalSize {
            rows: physical_rows,
            cols: physical_cols,
            pixel_width: (render_metrics.cell_size.width as usize * physical_cols),
            pixel_height: (render_metrics.cell_size.height as usize * physical_rows),
            dpi: dpi as u32,
        };

        if terminal_size != size {
            // DPI is different from the default assumed DPI when the mux
            // created the pty. We need to inform the kernel of the revised
            // pixel geometry now
            log::trace!(
                "Initial geometry was {:?} but dpi-adjusted geometry \
                        is {:?}; update the kernel pixel geometry for the ptys!",
                size,
                terminal_size,
            );
            if let Some(window) = mux.get_window(mux_window_id) {
                for tab in window.iter() {
                    tab.resize(terminal_size);
                }
            };
        }

        let h_context = DimensionContext {
            dpi: dpi as f32,
            pixel_max: terminal_size.pixel_width as f32,
            pixel_cell: render_metrics.cell_size.width as f32,
        };
        let padding_left = config.window_padding.left.evaluate_as_pixels(h_context) as usize;
        let padding_right = resize::effective_right_padding(&config, h_context) as usize;
        let v_context = DimensionContext {
            dpi: dpi as f32,
            pixel_max: terminal_size.pixel_height as f32,
            pixel_cell: render_metrics.cell_size.height as f32,
        };
        let padding_top = config.window_padding.top.evaluate_as_pixels(v_context) as usize;
        let padding_bottom = config.window_padding.bottom.evaluate_as_pixels(v_context) as usize;

        let mut dimensions = Dimensions {
            pixel_width: (terminal_size.pixel_width + padding_left + padding_right) as usize,
            pixel_height: ((terminal_size.rows * render_metrics.cell_size.height as usize)
                + padding_top
                + padding_bottom) as usize
                + tab_bar_height,
            dpi,
        };

        let border = Self::get_os_border_impl(&None, &config, &dimensions, &render_metrics);

        dimensions.pixel_height += (border.top + border.bottom).get() as usize;
        dimensions.pixel_width += (border.left + border.right).get() as usize;

        let window_background = load_background_image(&config, &dimensions, &render_metrics);

        log::trace!(
            "TermWindow::new_window called with mux_window_id {} {:?} {:?}",
            mux_window_id,
            terminal_size,
            dimensions
        );

        let render_state = None;

        let connection_name = Connection::get().unwrap().name();

        let myself = Self {
            created: Instant::now(),
            connection_name,
            last_fps_check_time: Instant::now(),
            num_frames: 0,
            last_frame_duration: Duration::ZERO,
            fps: 0.,
            config_subscription: None,
            os_parameters: None,
            gl: None,
            webgpu: None,
            window: None,
            window_background,
            config: config.clone(),
            config_overrides: wezterm_dynamic::Value::default(),
            palette: None,
            focused: None,
            mux_window_id,
            mux_window_id_for_subscriptions: Arc::new(Mutex::new(mux_window_id)),
            fonts: Rc::clone(&fontconfig),
            render_metrics,
            dimensions,
            window_state: WindowState::default(),
            resizes_pending: 0,
            is_repaint_pending: false,
            pending_scale_changes: LinkedList::new(),
            terminal_size,
            render_state,
            input_map: InputMap::new(&config),
            leader_is_down: None,
            dead_key_status: DeadKeyStatus::None,
            show_tab_bar,
            show_scroll_bar: config.enable_scroll_bar,
            tab_bar: TabBarState::default(),
            fancy_tab_bar: None,
            last_top_stats_text: String::new(),
            right_status: Self::default_right_status(&config, None),
            left_status: String::new(),
            ui_notice: RefCell::new(None),
            last_mouse_coords: (0, -1),
            window_drag_position: None,
            current_mouse_event: None,
            current_modifier_and_leds: Default::default(),
            prev_cursor: PrevCursorPos::new(),
            last_scroll_info: RenderableDimensions::default(),
            tab_state: RefCell::new(HashMap::new()),
            pane_state: RefCell::new(HashMap::new()),
            next_core_render_consumers: RefCell::new(EngineRenderConsumerSet::new()),
            next_core_pane_bindings: RefCell::new(NextCorePaneBindings::new()),
            swallow_left_gesture_after_secondary_click: false,
            current_mouse_buttons: vec![],
            current_mouse_capture: None,
            last_mouse_click: None,
            current_highlight: None,
            quad_generation: 0,
            shape_generation: 0,
            shape_cache: RefCell::new(LfuCache::new(
                "shape_cache.hit.rate",
                "shape_cache.miss.rate",
                |config| config.shape_cache_size,
                &config,
            )),
            line_state_cache: RefCell::new(LfuCacheU64::new(
                "line_state_cache.hit.rate",
                "line_state_cache.miss.rate",
                |config| config.line_state_cache_size,
                &config,
            )),
            next_line_state_id: 0,
            line_quad_cache: RefCell::new(LfuCache::new(
                "line_quad_cache.hit.rate",
                "line_quad_cache.miss.rate",
                |config| config.line_quad_cache_size,
                &config,
            )),
            line_to_ele_shape_cache: RefCell::new(LfuCache::new(
                "line_to_ele_shape_cache.hit.rate",
                "line_to_ele_shape_cache.miss.rate",
                |config| config.line_to_ele_shape_cache_size,
                &config,
            )),
            last_status_call: Instant::now(),
            cursor_blink_state: RefCell::new(ColorEase::new(
                config.cursor_blink_rate,
                config.cursor_blink_ease_in,
                config.cursor_blink_rate,
                config.cursor_blink_ease_out,
                None,
            )),
            blink_state: RefCell::new(ColorEase::new(
                config.text_blink_rate,
                config.text_blink_ease_in,
                config.text_blink_rate,
                config.text_blink_ease_out,
                None,
            )),
            rapid_blink_state: RefCell::new(ColorEase::new(
                config.text_blink_rate_rapid,
                config.text_blink_rapid_ease_in,
                config.text_blink_rate_rapid,
                config.text_blink_rapid_ease_out,
                None,
            )),
            event_states: HashMap::new(),
            current_event: None,
            has_animation: RefCell::new(None),
            scheduled_animation: RefCell::new(None),
            allow_images: AllowImage::Yes,
            semantic_zones: HashMap::new(),
            ui_items: vec![],
            dragging: None,
            last_ui_item: None,
            is_click_to_focus_window: false,
            key_table_state: KeyTableState::default(),
            modal: RefCell::new(None),
            composer: RefCell::new(Default::default()),
            prewarmed_settings_menu: RefCell::new(None),
            tree_sidebar: RefCell::new(None),
            git_panel: RefCell::new(None),
            left_tab_bar: RefCell::new(left_tab_bar::LeftTabBar::default()),
            deferred_scrollbar: RefCell::new(Vec::new()),
            opengl_info: None,
        };

        let tw = Rc::new(RefCell::new(myself));
        let tw_event = Rc::clone(&tw);

        let mut x = None;
        let mut y = None;
        let mut origin = GeometryOrigin::default();

        if let Some(position) = mux
            .get_window(mux_window_id)
            .and_then(|window| window.get_initial_position().clone())
            .or_else(|| POSITION.lock().unwrap().take())
        {
            x.replace(position.x);
            y.replace(position.y);
            origin = position.origin;
        }

        // Use saved dimensions from last session if available,
        // otherwise use the computed dimensions from config.
        let (geo_width, geo_height) =
            if let Some((saved_w, saved_h)) = SAVED_DIMENSIONS.lock().unwrap().take() {
                log::info!("Restoring saved window dimensions: {}x{}", saved_w, saved_h);
                (saved_w, saved_h)
            } else {
                (dimensions.pixel_width, dimensions.pixel_height)
            };

        let geometry = RequestedWindowGeometry {
            width: Dimension::Pixels(geo_width as f32),
            height: Dimension::Pixels(geo_height as f32),
            x,
            y,
            origin,
        };
        log::trace!("{:?}", geometry);

        log::debug!(
            "startup-span: pre Window::new_window {:?}",
            startup_span.elapsed()
        );
        let window = Window::new_window(
            &get_window_class(),
            "unterm",
            geometry,
            Some(&config),
            Rc::clone(&fontconfig),
            move |event, window| {
                let mut tw = tw_event.borrow_mut();
                if let Err(err) = tw.dispatch_window_event(event, window) {
                    log::error!("dispatch_window_event: {:#}", err);
                }
            },
        )
        .await?;
        log::debug!(
            "startup-span: os window created {:?}",
            startup_span.elapsed()
        );
        crate::startup_timing::mark("os window created");
        tw.borrow_mut().window.replace(window.clone());

        // Pre-warm the search bar's localized glyphs at idle. Their first
        // font-fallback resolution costs a few hundred ms and otherwise
        // lands on the user's first search open, which reads as "the
        // search box didn't open". One string per slice, staggered, so
        // the warm-up itself never blocks the GUI thread noticeably.
        for (i, text) in crate::overlay::copy::search_bar_prewarm_strings()
            .into_iter()
            .enumerate()
        {
            let window = window.clone();
            promise::spawn::spawn(async move {
                smol::Timer::after(Duration::from_millis(1500 + i as u64 * 250)).await;
                window.notify(TermWindowNotif::Apply(Box::new(move |tw| {
                    tw.prewarm_search_bar_glyphs(&text);
                })));
            })
            .detach();
        }

        {
            let window = window.clone();
            promise::spawn::spawn(async move {
                smol::Timer::after(Duration::from_millis(700)).await;
                window.notify(TermWindowNotif::Apply(Box::new(move |tw| {
                    tw.prewarm_settings_menu();
                })));
            })
            .detach();
        }

        Self::apply_icon(&window)?;

        let config_subscription = config::subscribe_to_config_reload({
            let window = window.clone();
            move || {
                window.notify(TermWindowNotif::Apply(Box::new(|tw| {
                    tw.config_was_reloaded()
                })));
                true
            }
        });

        let gl = match config.front_end {
            FrontEndSelection::WebGpu => None,
            _ => Some(window.enable_opengl().await?),
        };
        crate::startup_timing::mark("gpu context ready");

        {
            let mut myself = tw.borrow_mut();
            let webgpu = match config.front_end {
                FrontEndSelection::WebGpu => Some(Rc::new(
                    WebGpuState::new(&window, dimensions, &config).await?,
                )),
                _ => None,
            };
            myself.config_subscription.replace(config_subscription);
            if config.use_resize_increments {
                window.set_resize_increments(
                    ResizeIncrementCalculator {
                        x: myself.render_metrics.cell_size.width as u16,
                        y: myself.render_metrics.cell_size.height as u16,
                        padding_left: padding_left,
                        padding_top: padding_top,
                        padding_right: padding_right,
                        padding_bottom: padding_bottom,
                        border: border,
                        tab_bar_height: tab_bar_height,
                    }
                    .into(),
                );
            }

            if let Some(gl) = gl {
                myself.gl.replace(Rc::clone(&gl));
                myself.created(RenderContext::Glium(Rc::clone(&gl)))?;
            }
            if let Some(webgpu) = webgpu {
                myself.webgpu.replace(Rc::clone(&webgpu));
                myself.created(RenderContext::WebGpu(Rc::clone(&webgpu)))?;
            }
            myself.load_os_parameters();

            crate::startup_timing::mark("render state created");
            window.show();
            crate::startup_timing::mark("window.show() called");
            myself.subscribe_to_pane_updates();
            myself.emit_window_event("window-config-reloaded", None);
            myself.emit_status_event();
        }

        crate::update::start_update_checker();
        front_end().record_known_window(window, mux_window_id);

        Ok(())
    }

    fn dispatch_window_event(
        &mut self,
        event: WindowEvent,
        window: &Window,
    ) -> anyhow::Result<bool> {
        log::debug!("{event:?}");
        match event {
            WindowEvent::Destroyed => {
                // Ensure that we cancel any overlays we had running, so
                // that the mux can empty out, otherwise the mux keeps
                // the TermWindow alive via the frontend even though
                // the window is gone and we'll linger forever.
                // <https://github.com/wezterm/wezterm/issues/3522>
                self.clear_all_overlays();
                Ok(false)
            }
            WindowEvent::CloseRequested => {
                self.close_requested(window);
                Ok(true)
            }
            WindowEvent::AppearanceChanged(appearance) => {
                log::debug!("Appearance is now {:?}", appearance);
                // This is a bit fugly; we get per-window notifications
                // for appearance changes which successfully updates the
                // per-window config, but we need to explicitly tell the
                // global config to reload, otherwise things that acces
                // the config via config::configuration() will see the
                // prior version of the config.
                // What's fugly about this is that we'll reload the
                // global config here once per window, which could
                // be nasty for folks with a lot of windows.
                // <https://github.com/wezterm/wezterm/issues/2295>
                config::reload();
                self.config_was_reloaded();
                Ok(true)
            }
            WindowEvent::PerformKeyAssignment(action) => {
                if let Some(pane) = self.get_active_pane_or_overlay() {
                    self.perform_key_assignment(&pane, &action)?;
                    window.invalidate();
                }
                Ok(true)
            }
            WindowEvent::FocusChanged(focused) => {
                self.focus_changed(focused, window);
                Ok(true)
            }
            WindowEvent::MouseEvent(event) => {
                self.mouse_event_impl(event, window);
                Ok(true)
            }
            WindowEvent::MouseLeave => {
                self.mouse_leave_impl(window);
                Ok(true)
            }
            WindowEvent::Resized {
                dimensions,
                window_state,
                live_resizing,
            } => {
                self.resize(dimensions, window_state, window, live_resizing);
                Ok(true)
            }
            WindowEvent::SetInnerSizeCompleted => {
                self.resizes_pending -= 1;
                if self.is_repaint_pending {
                    self.is_repaint_pending = false;
                    if self.webgpu.is_some() {
                        self.do_paint_webgpu()?;
                    } else {
                        self.do_paint(window);
                    }
                }
                self.apply_pending_scale_changes();
                Ok(true)
            }
            WindowEvent::AdviseModifiersLedStatus(modifiers, leds) => {
                self.current_modifier_and_leds = (modifiers, leds);
                self.update_title();
                window.invalidate();
                Ok(true)
            }
            WindowEvent::RawKeyEvent(event) => {
                self.raw_key_event_impl(event, window);
                Ok(true)
            }
            WindowEvent::KeyEvent(event) => {
                self.key_event_impl(event, window);
                Ok(true)
            }
            WindowEvent::AdviseDeadKeyStatus(status) => {
                if self.config.debug_key_events {
                    log::info!("DeadKeyStatus now: {:?}", status);
                } else {
                    log::trace!("DeadKeyStatus now: {:?}", status);
                }
                // A modal with a text input takes over the composition
                // preview; storing None here keeps the pane renderer from
                // painting the marked text at the pane cursor behind the
                // modal card.
                let consumed = {
                    let modal = self.modal.borrow().clone();
                    modal.map(|m| m.advise_compose(&status)).unwrap_or(false)
                };
                self.dead_key_status = if consumed {
                    DeadKeyStatus::None
                } else {
                    status
                };
                self.update_title();
                // Ensure that we repaint so that any composing
                // text is updated
                window.invalidate();
                Ok(true)
            }
            WindowEvent::NeedRepaint => {
                if self.resizes_pending > 0 {
                    self.is_repaint_pending = true;
                    Ok(true)
                } else if self.webgpu.is_some() {
                    self.do_paint_webgpu()
                } else {
                    Ok(self.do_paint(window))
                }
            }
            WindowEvent::Notification(item) => {
                if item.is::<TermWindowNotif>() {
                    if let Ok(notif) = item.downcast::<TermWindowNotif>() {
                        self.dispatch_notif(*notif, window)
                            .context("dispatch_notif")?;
                    }
                }
                Ok(true)
            }
            WindowEvent::DroppedString(text) => {
                let pane = match self.get_active_pane_or_overlay() {
                    Some(pane) => pane,
                    None => return Ok(true),
                };
                pane.send_paste(text.as_str())?;
                Ok(true)
            }
            WindowEvent::DroppedUrl(urls) => {
                let pane = match self.get_active_pane_or_overlay() {
                    Some(pane) => pane,
                    None => return Ok(true),
                };
                let urls = urls
                    .iter()
                    .map(|url| self.config.quote_dropped_files.escape(&url.to_string()))
                    .collect::<Vec<_>>()
                    .join(" ")
                    + " ";
                pane.send_paste(urls.as_str())?;
                Ok(true)
            }
            WindowEvent::DroppedFile(paths) => {
                let pane = match self.get_active_pane_or_overlay() {
                    Some(pane) => pane,
                    None => return Ok(true),
                };
                let paths = paths
                    .iter()
                    .map(|path| {
                        self.config
                            .quote_dropped_files
                            .escape(&path.to_string_lossy())
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
                    + " ";
                pane.send_paste(&paths)?;
                Ok(true)
            }
            WindowEvent::DraggedFile(_) => Ok(true),
        }
    }

    fn do_paint(&mut self, window: &Window) -> bool {
        let gl = match self.gl.as_ref() {
            Some(gl) => gl,
            None => return false,
        };

        if gl.is_context_lost() {
            log::error!("opengl context was lost; should reinit");
            window.close();
            front_end().forget_known_window(window);
            return false;
        }

        let mut frame = glium::Frame::new(
            Rc::clone(&gl),
            (
                self.dimensions.pixel_width as u32,
                self.dimensions.pixel_height as u32,
            ),
        );
        self.paint_impl(&mut RenderFrame::Glium(&mut frame));
        window.finish_frame(frame).is_ok()
    }

    fn do_paint_webgpu(&mut self) -> anyhow::Result<bool> {
        self.webgpu.as_mut().unwrap().resize(self.dimensions);
        match self.do_paint_webgpu_impl() {
            Ok(ok) => Ok(ok),
            Err(err) => {
                match err.downcast_ref::<wgpu::SurfaceError>() {
                    Some(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.webgpu.as_mut().unwrap().resize(self.dimensions);
                        return self.do_paint_webgpu_impl();
                    }
                    _ => {}
                }
                Err(err)
            }
        }
    }

    fn do_paint_webgpu_impl(&mut self) -> anyhow::Result<bool> {
        self.paint_impl(&mut RenderFrame::WebGpu);
        Ok(true)
    }

    fn dispatch_notif(&mut self, notif: TermWindowNotif, window: &Window) -> anyhow::Result<()> {
        fn chan_err<T>(e: smol::channel::TrySendError<T>) -> anyhow::Error {
            anyhow::anyhow!("{}", e)
        }

        match notif {
            TermWindowNotif::InvalidateShapeCache => {
                self.shape_generation += 1;
                self.shape_cache.borrow_mut().clear();
                self.invalidate_modal();
                window.invalidate();
            }
            TermWindowNotif::PerformAssignment {
                pane_id,
                assignment,
                tx,
            } => {
                let mux = Mux::get();
                let result = || -> anyhow::Result<()> {
                    // The CopyMode overlay doesn't exist in the mux, but aliases
                    // itself with the overlaid pane's pane_id.
                    // So we do a bit of fancy footwork here to resolve the overlay
                    // and use that if it has the same pane_id, but otherwise fall
                    // back to what we get from the mux.
                    // <https://github.com/wezterm/wezterm/issues/3209>
                    let active_pane = self
                        .get_active_pane_or_overlay()
                        .ok_or_else(|| anyhow!("there is no active pane!?"))?;
                    let pane = if active_pane.pane_id() == pane_id {
                        active_pane
                    } else {
                        mux.get_pane(pane_id)
                            .ok_or_else(|| anyhow!("pane id {} is not valid", pane_id))?
                    };
                    self.perform_key_assignment(&pane, &assignment)
                        .context("perform_key_assignment")?;
                    Ok(())
                }();
                window.invalidate();
                if let Some(tx) = tx {
                    tx.try_send(result).ok();
                }
            }
            TermWindowNotif::SetRightStatus(status) => {
                if status != self.right_status {
                    self.right_status = status;
                    self.update_title_post_status();
                } else {
                    self.schedule_next_status_update();
                }
            }
            TermWindowNotif::SetLeftStatus(status) => {
                if status != self.left_status {
                    self.left_status = status;
                    self.update_title_post_status();
                } else {
                    self.schedule_next_status_update();
                }
            }
            TermWindowNotif::GetDimensions(tx) => {
                tx.try_send((self.dimensions, self.window_state))
                    .map_err(chan_err)
                    .context("send GetDimensions response")?;
            }
            TermWindowNotif::GetEffectiveConfig(tx) => {
                tx.try_send(self.config.clone())
                    .map_err(chan_err)
                    .context("send GetEffectiveConfig response")?;
            }
            TermWindowNotif::FinishWindowEvent { name, again } => {
                self.finish_window_event(&name, again);
            }
            TermWindowNotif::GetConfigOverrides(tx) => {
                tx.try_send(self.config_overrides.clone())
                    .map_err(chan_err)
                    .context("send GetConfigOverrides response")?;
            }
            TermWindowNotif::SetConfigOverrides(value) => {
                if value != self.config_overrides {
                    self.config_overrides = value;
                    self.config_was_reloaded();
                }
            }
            TermWindowNotif::CancelOverlayForPane(pane_id) => {
                self.cancel_overlay_for_pane(pane_id);
            }
            TermWindowNotif::CancelOverlayForTab { tab_id, pane_id } => {
                self.cancel_overlay_for_tab(tab_id, pane_id);
            }
            TermWindowNotif::MuxNotification(n) => match n {
                MuxNotification::Alert {
                    alert: Alert::SetUserVar { name, value },
                    pane_id,
                } => {
                    self.emit_user_var_event(pane_id, name, value);
                }
                MuxNotification::Alert {
                    alert: Alert::WindowTitleChanged(title),
                    pane_id,
                } => {
                    crate::cockpit::on_title_change(pane_id as u64, &title);
                    self.update_title();
                }
                MuxNotification::Alert {
                    alert: Alert::IconTitleChanged(title),
                    pane_id,
                } => {
                    if let Some(title) = &title {
                        crate::cockpit::on_title_change(pane_id as u64, title);
                    }
                    self.update_title();
                }
                MuxNotification::Alert {
                    alert: Alert::Progress(progress),
                    pane_id,
                } => {
                    crate::cockpit::on_progress(
                        pane_id as u64,
                        !matches!(progress, wezterm_term::Progress::None),
                    );
                    self.update_title();
                }
                MuxNotification::WindowTitleChanged { .. }
                | MuxNotification::Alert {
                    alert:
                        Alert::OutputSinceFocusLost
                        | Alert::CurrentWorkingDirectoryChanged
                        | Alert::TabTitleChanged(_),
                    ..
                } => {
                    self.update_title();
                }
                MuxNotification::Alert {
                    alert: Alert::PaletteChanged,
                    pane_id,
                } => {
                    // Shape cache includes color information, so
                    // ensure that we invalidate that as part of
                    // this overall invalidation for the palette
                    self.dispatch_notif(TermWindowNotif::InvalidateShapeCache, window)?;
                    self.mux_pane_output_event(pane_id);
                }
                MuxNotification::Alert {
                    alert: Alert::Bell,
                    pane_id,
                } => {
                    if !self.window_contains_pane(pane_id) {
                        return Ok(());
                    }

                    match self.config.audible_bell {
                        AudibleBell::SystemBeep => {
                            Connection::get().expect("on main thread").beep();
                        }
                        AudibleBell::Disabled => {}
                    }

                    log::trace!("Ding! (this is the bell) in pane {}", pane_id);
                    self.emit_window_event("bell", Some(pane_id));
                    crate::cockpit::on_bell(pane_id as u64);

                    let mut per_pane = self.pane_state(pane_id);
                    per_pane.bell_start.replace(Instant::now());
                    window.invalidate();
                }
                MuxNotification::Alert {
                    alert: Alert::ToastNotification { title, body, .. },
                    pane_id,
                } => {
                    crate::cockpit::on_notification(pane_id as u64, title.as_deref(), &body);
                }
                MuxNotification::TabAddedToWindow {
                    window_id: _,
                    tab_id,
                } => {
                    let mux = Mux::get();
                    let mut size = self.terminal_size;
                    if let Some(tab) = mux.get_tab(tab_id) {
                        // If we attached to a remote domain and loaded in
                        // a tab async, we need to fixup its size, either
                        // by resizing it or resizes ourselves.
                        // The strategy here is to adjust both by taking
                        // the maximal size in both horizontal and vertical
                        // dimensions and applying that. In practice that
                        // means that a new local client will resize larger
                        // to adjust to the size of an existing client.
                        let tab_size = tab.get_size();
                        size.rows = size.rows.max(tab_size.rows);
                        size.cols = size.cols.max(tab_size.cols);

                        if size.rows != self.terminal_size.rows
                            || size.cols != self.terminal_size.cols
                            || size.pixel_width != self.terminal_size.pixel_width
                            || size.pixel_height != self.terminal_size.pixel_height
                        {
                            self.set_window_size(size, window)?;
                        } else if tab_size.dpi == 0 {
                            log::debug!("fixup dpi in newly added tab");
                            tab.resize(self.terminal_size);
                        }
                    }
                }
                MuxNotification::PaneOutput(pane_id) => {
                    self.mux_pane_output_event(pane_id);
                }
                MuxNotification::WindowInvalidated(_) => {
                    window.invalidate();
                    self.update_title_post_status();
                }
                MuxNotification::WindowRemoved(_window_id) => {
                    // Handled by frontend
                }
                MuxNotification::AssignClipboard { .. } => {
                    // Handled by frontend
                }
                MuxNotification::SaveToDownloads { .. } => {
                    // Handled by frontend
                }
                MuxNotification::PaneFocused(_) => {
                    // Also handled by clientpane
                    self.update_title_post_status();
                }
                MuxNotification::TabResized(_) => {
                    // Also handled by wezterm-client
                    self.update_title_post_status();
                }
                MuxNotification::TabTitleChanged { .. } => {
                    self.update_title_post_status();
                }
                MuxNotification::PaneAdded(_)
                | MuxNotification::WorkspaceRenamed { .. }
                | MuxNotification::PaneRemoved(_)
                | MuxNotification::WindowWorkspaceChanged(_)
                | MuxNotification::ActiveWorkspaceChanged(_)
                | MuxNotification::Empty
                | MuxNotification::WindowCreated(_) => {}
            },
            TermWindowNotif::EmitStatusUpdate => {
                self.emit_status_event();
                // Feed the cockpit's process-detection layer and run its
                // decay pass on the same cadence as the stats refresh.
                // The probe reads the mcp handler's non-blocking cache,
                // so this never touches the process table on this thread.
                {
                    let mux = Mux::get();
                    let panes = mux.iter_panes();
                    let ids: Vec<u64> = panes.iter().map(|p| p.pane_id() as u64).collect();
                    crate::cockpit::poll(&ids, |id| {
                        crate::mcp::handler::agent_and_cwd_for_pane(id).0
                    });
                    crate::cockpit::status::retain_panes(&ids.iter().copied().collect());
                    // Layer-4 screen-text heuristics for tracked panes
                    // that have no OSC/hook signal (e.g. Aider). Reads
                    // only the last 3 viewport lines of agent panes.
                    for status in crate::cockpit::snapshot() {
                        if let Some(pane) = mux.get_pane(status.pane_id as mux::pane::PaneId) {
                            let dims = pane.get_dimensions();
                            let last = dims.physical_top + dims.viewport_rows as isize;
                            let start = (last - 3).max(dims.physical_top);
                            let (_first, lines) = pane.get_lines(start..last);
                            let tail: Vec<String> =
                                lines.iter().map(|l| l.as_str().to_string()).collect();
                            crate::cockpit::status::on_screen_tail(status.pane_id, &tail);
                        }
                    }
                    // Auto-checkpoint: an agent just started working in
                    // these panes — snapshot their repos off-thread.
                    for pane_id in crate::cockpit::status::take_checkpoint_requests() {
                        let Some(status) = crate::cockpit::status_for_pane(pane_id) else {
                            continue;
                        };
                        // Fleet worktrees already carry their start commit
                        // as the review baseline; don't double-checkpoint.
                        if status.fleet_id.is_some() {
                            continue;
                        }
                        let cwd = mux
                            .get_pane(pane_id as mux::pane::PaneId)
                            .and_then(|p| {
                                p.get_current_working_dir(mux::pane::CachePolicy::AllowStale)
                            })
                            .and_then(|url| url.to_file_path().ok());
                        let Some(cwd) = cwd else { continue };
                        let agent = status.agent.clone();
                        std::thread::Builder::new()
                            .name("cockpit-checkpoint".into())
                            .spawn(move || {
                                match crate::cockpit::review::record_auto_checkpoint(
                                    &cwd, &agent, pane_id,
                                ) {
                                    Ok(Some(sha)) => log::info!(
                                        "cockpit checkpoint {} for pane {pane_id}",
                                        &sha[..12.min(sha.len())]
                                    ),
                                    Ok(None) => {}
                                    // Not-a-git-repo is the common,
                                    // uninteresting failure.
                                    Err(err) => {
                                        log::debug!("cockpit checkpoint skipped: {err:#}")
                                    }
                                }
                            })
                            .ok();
                    }
                }
                // Drive the periodic refresh directly. The Lua status
                // events above only lead back to update_title_impl when
                // a handler calls window:set_*_status(); without one the
                // status timer (re-armed at the end of update_title_impl)
                // would never re-arm after this tick, freezing the
                // top-stats chrome at its last painted values.
                self.update_title_post_status();
            }
            TermWindowNotif::GetSelectionForPane { pane_id, tx } => {
                let mux = Mux::get();
                let pane = mux
                    .get_pane(pane_id)
                    .ok_or_else(|| anyhow!("pane id {} is not valid", pane_id))?;

                tx.try_send(self.selection_text(&pane))
                    .map_err(chan_err)
                    .context("send GetSelectionForPane response")?;
            }
            TermWindowNotif::Apply(func) => {
                func(self);
            }
            TermWindowNotif::SwitchToMuxWindow(mux_window_id) => {
                self.mux_window_id = mux_window_id;
                *self.mux_window_id_for_subscriptions.lock().unwrap() = mux_window_id;

                self.clear_all_overlays();
                self.current_highlight.take();
                self.invalidate_fancy_tab_bar();
                self.invalidate_modal();

                let mux = Mux::get();
                if let Some(window) = mux.get_window(self.mux_window_id) {
                    for tab in window.iter() {
                        tab.resize(self.terminal_size);
                    }
                };
                self.update_title();
                window.invalidate();
            }
            TermWindowNotif::SetInnerSize { width, height } => {
                self.set_inner_size(window, width, height);
            }
        }

        Ok(())
    }

    fn set_inner_size(&mut self, window: &Window, width: usize, height: usize) {
        self.resizes_pending += 1;
        window.set_inner_size(width, height);
    }

    /// Take care to remove our panes from the mux, otherwise
    /// we can leave the mux with no windows but some panes
    /// and it won't believe that we are empty.
    fn clear_all_overlays(&mut self) {
        let overlay_panes_to_cancel = self
            .pane_state
            .borrow()
            .iter()
            .filter_map(|(_, state)| state.overlay.as_ref().map(|overlay| overlay.pane.pane_id()))
            .collect::<Vec<_>>();

        for pane_id in overlay_panes_to_cancel {
            self.cancel_overlay_for_pane(pane_id);
        }

        let tab_overlays_to_cancel = self
            .tab_state
            .borrow()
            .iter()
            .filter_map(|(tab_id, state)| state.overlay.as_ref().map(|_| *tab_id))
            .collect::<Vec<_>>();

        for tab_id in tab_overlays_to_cancel {
            self.cancel_overlay_for_tab(tab_id, None);
        }

        self.pane_state.borrow_mut().clear();
        self.next_core_render_consumers.borrow_mut().clear();
        self.destroy_all_next_core_pane_bindings();
        self.tab_state.borrow_mut().clear();
    }

    fn apply_icon(window: &Window) -> anyhow::Result<()> {
        let image = image::load_from_memory(ICON_DATA)?.into_rgba8();
        let (width, height) = image.dimensions();
        window.set_icon(Image::with_rgba32(
            width as usize,
            height as usize,
            width as usize * 4,
            image.as_raw(),
        ));
        Ok(())
    }

    fn schedule_status_update(&self) {
        if let Some(window) = self.window.as_ref() {
            window.notify(TermWindowNotif::EmitStatusUpdate);
        }
    }

    fn is_pane_visible(&mut self, pane_id: PaneId) -> bool {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return false,
        };

        let tab_id = tab.tab_id();
        if let Some(tab_overlay) = self
            .tab_state(tab_id)
            .overlay
            .as_ref()
            .map(|overlay| overlay.pane.clone())
        {
            return tab_overlay.pane_id() == pane_id;
        }

        tab.contains_pane(pane_id)
    }

    fn mux_pane_output_event(&mut self, pane_id: PaneId) {
        metrics::histogram!("mux.pane_output_event.rate").record(1.);
        if self.is_pane_visible(pane_id) {
            if let Some(ref win) = self.window {
                win.invalidate();
            }
        }
    }

    fn mux_pane_output_event_callback(
        n: MuxNotification,
        window: &Window,
        mux_window_id: MuxWindowId,
        dead: &Arc<AtomicBool>,
    ) -> bool {
        if dead.load(Ordering::Relaxed) {
            // Subscription cancelled asynchronously
            return false;
        }

        match n {
            MuxNotification::Alert {
                pane_id,
                alert:
                    Alert::OutputSinceFocusLost
                    | Alert::CurrentWorkingDirectoryChanged
                    | Alert::WindowTitleChanged(_)
                    | Alert::TabTitleChanged(_)
                    | Alert::IconTitleChanged(_)
                    | Alert::Progress(_)
                    | Alert::SetUserVar { .. }
                    // The cockpit consumes agent notifications (OSC 9 /
                    // OSC 777) for waiting/done detection; the OS toast
                    // is raised separately by the frontend subscriber.
                    | Alert::ToastNotification { .. }
                    | Alert::Bell,
            }
            | MuxNotification::PaneFocused(pane_id)
            | MuxNotification::PaneRemoved(pane_id)
            | MuxNotification::PaneOutput(pane_id) => {
                // Ideally we'd check to see if pane_id is part of this window,
                // but overlays may not be 100% associated with the window
                // in the mux and we don't want to lose the invalidation
                // signal for that case, so we just check window validity
                // here and propagate to the window event handler that
                // will then do the check with full context.
                let mux = Mux::get();
                if mux.get_window(mux_window_id).is_none() {
                    // Something inconsistent: cancel subscription
                    log::debug!(
                        "PaneOutput: wanted mux_window_id={} from mux, but \
                         was not found, cancel mux subscription",
                        mux_window_id
                    );
                    return false;
                }
                let _ = pane_id;
            }
            MuxNotification::PaneAdded(_pane_id) => {
                // If some other client spawns a pane inside this window, this
                // gives us an opportunity to attach it to the clipboard.
                let mux = Mux::get();
                return mux.get_window(mux_window_id).is_some();
            }
            MuxNotification::TabAddedToWindow { window_id, .. }
            | MuxNotification::WindowTitleChanged { window_id, .. }
            | MuxNotification::WindowInvalidated(window_id) => {
                if window_id != mux_window_id {
                    return true;
                }
            }
            MuxNotification::WindowRemoved(window_id) => {
                if window_id != mux_window_id {
                    return true;
                }
                // Set the window as dead to unsubscribe from further notifications
                dead.store(true, Ordering::Relaxed);
                return false;
            }
            MuxNotification::TabResized(tab_id)
            | MuxNotification::TabTitleChanged { tab_id, .. } => {
                let mux = Mux::get();
                if mux.window_containing_tab(tab_id) == Some(mux_window_id) {
                    // fall through
                } else {
                    return true;
                }
            }
            MuxNotification::AssignClipboard { .. }
            | MuxNotification::SaveToDownloads { .. }
            | MuxNotification::WindowCreated(_)
            | MuxNotification::ActiveWorkspaceChanged(_)
            | MuxNotification::WorkspaceRenamed { .. }
            | MuxNotification::Empty
            | MuxNotification::WindowWorkspaceChanged(_) => return true,
            MuxNotification::Alert {
                alert: Alert::PaletteChanged { .. },
                ..
            } => {
                // fall through
            }
        }

        window.notify(TermWindowNotif::MuxNotification(n));

        true
    }

    fn subscribe_to_pane_updates(&self) {
        let window = self.window.clone().expect("window to be valid on startup");
        let mux_window_id = Arc::clone(&self.mux_window_id_for_subscriptions);
        let mux = Mux::get();
        let dead = Arc::new(AtomicBool::new(false));
        mux.subscribe(move |n| {
            if dead.load(Ordering::Relaxed) {
                return false;
            }
            let mux_window_id = *mux_window_id.lock().unwrap();
            let window = window.clone();
            let dead = dead.clone();
            promise::spawn::spawn_into_main_thread(async move {
                Self::mux_pane_output_event_callback(n, &window, mux_window_id, &dead)
            })
            .detach();
            true
        });
    }

    fn emit_status_event(&mut self) {
        self.emit_window_event("update-right-status", None);
        self.emit_window_event("update-status", None);
    }

    fn schedule_window_event(&mut self, name: &str, pane_id: Option<PaneId>) {
        let window = GuiWin::new(self);
        let pane = match pane_id {
            Some(pane_id) => Mux::get().get_pane(pane_id),
            None => None,
        };
        let pane = match pane {
            Some(pane) => pane,
            None => match self.get_active_pane_or_overlay() {
                Some(pane) => pane,
                None => return,
            },
        };
        let pane = MuxPane(pane.pane_id());
        let name = name.to_string();

        async fn do_event(
            lua: Option<Rc<mlua::Lua>>,
            name: String,
            window: GuiWin,
            pane: MuxPane,
        ) -> anyhow::Result<()> {
            let again = if let Some(lua) = lua {
                let args = lua.pack_multi((window.clone(), pane))?;

                if let Err(err) = config::lua::emit_event(&lua, (name.clone(), args)).await {
                    log::error!("while processing {} event: {:#}", name, err);
                }
                true
            } else {
                false
            };

            window
                .window
                .notify(TermWindowNotif::FinishWindowEvent { name, again });

            Ok(())
        }

        promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
            do_event(lua, name, window, pane)
        }))
        .detach();
    }

    /// Called as part of finishing up a callout to lua.
    /// If again==false it means that there isn't a lua config
    /// to execute against, so we should just mark as done.
    /// Otherwise, if there is a queued item, schedule it now.
    fn finish_window_event(&mut self, name: &str, again: bool) {
        let state = self
            .event_states
            .entry(name.to_string())
            .or_insert(EventState::None);
        if again {
            match state {
                EventState::InProgress => {
                    *state = EventState::None;
                }
                EventState::InProgressWithQueued(pane) => {
                    let pane = *pane;
                    *state = EventState::InProgress;
                    self.schedule_window_event(name, pane);
                }
                EventState::None => {}
            }
        } else {
            *state = EventState::None;
        }
    }

    pub fn emit_window_event(&mut self, name: &str, pane_id: Option<PaneId>) {
        if self.get_active_pane_or_overlay().is_none() || self.window.is_none() {
            return;
        }

        let state = self
            .event_states
            .entry(name.to_string())
            .or_insert(EventState::None);
        match state {
            EventState::InProgress => {
                // Flag that we want to run again when the currently
                // executing event calls finish_window_event().
                *state = EventState::InProgressWithQueued(pane_id);
                return;
            }
            EventState::InProgressWithQueued(other_pane) => {
                // We've already got one copy executing and another
                // pending dispatch, so don't queue another.
                if pane_id != *other_pane {
                    log::warn!(
                        "Cannot queue {} event for pane {:?}, as \
                         there is already an event queued for pane {:?} \
                         in the same window",
                        name,
                        pane_id,
                        other_pane
                    );
                }
                return;
            }
            EventState::None => {
                // Nothing pending, so schedule a call now
                *state = EventState::InProgress;
                self.schedule_window_event(name, pane_id);
            }
        }
    }

    fn check_for_dirty_lines_and_invalidate_selection(&mut self, pane: &Arc<dyn Pane>) {
        let dims = pane.get_dimensions();
        let viewport = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top);
        let visible_range = viewport..viewport + dims.viewport_rows as StableRowIndex;
        let seqno = self.selection(pane.pane_id()).seqno;
        let dirty = pane.get_changed_since(visible_range, seqno);

        if dirty.is_empty() {
            return;
        }
        if pane.downcast_ref::<CopyOverlay>().is_none()
            && pane.downcast_ref::<QuickSelectOverlay>().is_none()
        {
            // If any of the changed lines intersect with the
            // selection, then we need to clear the selection, but not
            // when the search overlay is active; the search overlay
            // marks lines as dirty to force invalidate them for
            // highlighting purpose but also manipulates the selection
            // and we want to allow it to retain the selection it made!

            let clear_selection =
                if let Some(selection_range) = self.selection(pane.pane_id()).range.as_ref() {
                    let selection_rows = selection_range.rows();
                    selection_rows.into_iter().any(|row| dirty.contains(row))
                } else {
                    false
                };

            if clear_selection {
                self.selection(pane.pane_id()).range.take();
                self.selection(pane.pane_id()).origin.take();
                self.selection(pane.pane_id()).seqno = pane.get_current_seqno();
            }
        }
    }
}

impl TermWindow {
    fn palette(&mut self) -> &ColorPalette {
        if self.palette.is_none() {
            self.palette
                .replace(config::TermConfig::with_config(self.config.clone()).color_palette());
        }
        self.palette.as_ref().unwrap()
    }

    pub(crate) fn apply_theme_palette(&mut self, scheme: &str) -> anyhow::Result<()> {
        let config = self.config.with_runtime_color_scheme(scheme)?;
        let palette: wezterm_term::color::ColorPalette = config.resolved_palette.clone().into();
        self.config = config.clone();
        self.palette.replace(palette.clone());
        self.quad_generation += 1;
        self.line_quad_cache.borrow_mut().clear();
        self.line_to_ele_shape_cache.borrow_mut().clear();
        self.fancy_tab_bar.take();
        self.invalidate_fancy_tab_bar();
        self.invalidate_modal();
        self.invalidate_left_tab_bar();
        self.render_state.as_mut().map(|rs| rs.config_changed());

        let term_config = Arc::new(config::TermConfig::with_config(config));
        term_config.set_client_palette(palette);
        let term_config: Arc<dyn wezterm_term::config::TerminalConfiguration> = term_config;

        let mux = Mux::get();
        if let Some(window) = mux.get_window(self.mux_window_id) {
            for tab in window.iter() {
                for pane in tab.iter_panes_ignoring_zoom() {
                    pane.pane.set_config(Arc::clone(&term_config));
                }
            }
        }
        for state in self.pane_state.borrow().values() {
            if let Some(overlay) = &state.overlay {
                overlay.pane.set_config(Arc::clone(&term_config));
            }
        }
        for state in self.tab_state.borrow().values() {
            if let Some(overlay) = &state.overlay {
                overlay.pane.set_config(Arc::clone(&term_config));
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
        Ok(())
    }

    pub fn config_was_reloaded(&mut self) {
        log::debug!(
            "config was reloaded, overrides: {:?}",
            self.config_overrides
        );
        self.key_table_state.clear_stack();
        self.connection_name = Connection::get().unwrap().name();
        let config = match config::overridden_config(&self.config_overrides) {
            Ok(config) => config,
            Err(err) => {
                log::error!(
                    "Failed to apply config overrides to window: {:#}: {:?}",
                    err,
                    self.config_overrides
                );
                configuration()
            }
        };
        self.config = config.clone();
        self.palette.take();

        let mux = Mux::get();
        let window = match mux.get_window(self.mux_window_id) {
            Some(window) => window,
            _ => return,
        };
        if window.len() == 1 {
            self.show_tab_bar = config.enable_tab_bar && !config.hide_tab_bar_if_only_one_tab;
        } else {
            self.show_tab_bar = config.enable_tab_bar;
        }
        *self.cursor_blink_state.borrow_mut() = ColorEase::new(
            config.cursor_blink_rate,
            config.cursor_blink_ease_in,
            config.cursor_blink_rate,
            config.cursor_blink_ease_out,
            None,
        );
        *self.blink_state.borrow_mut() = ColorEase::new(
            config.text_blink_rate,
            config.text_blink_ease_in,
            config.text_blink_rate,
            config.text_blink_ease_out,
            None,
        );
        *self.rapid_blink_state.borrow_mut() = ColorEase::new(
            config.text_blink_rate_rapid,
            config.text_blink_rapid_ease_in,
            config.text_blink_rate_rapid,
            config.text_blink_rapid_ease_out,
            None,
        );

        self.show_scroll_bar = config.enable_scroll_bar;
        self.shape_generation += 1;
        {
            let mut shape_cache = self.shape_cache.borrow_mut();
            shape_cache.update_config(&config);
            shape_cache.clear();
        }
        self.line_state_cache.borrow_mut().update_config(&config);
        self.line_quad_cache.borrow_mut().update_config(&config);
        self.line_to_ele_shape_cache
            .borrow_mut()
            .update_config(&config);
        self.fancy_tab_bar.take();
        self.invalidate_fancy_tab_bar();
        self.invalidate_modal();
        self.input_map = InputMap::new(&config);
        self.leader_is_down = None;
        self.render_state.as_mut().map(|rs| rs.config_changed());
        let dimensions = self.dimensions;

        if let Err(err) = self.fonts.config_changed(&config) {
            log::error!("Failed to load font configuration: {:#}", err);
        }

        if let Some(window) = mux.get_window(self.mux_window_id) {
            let term_config: Arc<dyn TerminalConfiguration> =
                Arc::new(TermConfig::with_config(config.clone()));
            for tab in window.iter() {
                for pane in tab.iter_panes_ignoring_zoom() {
                    pane.pane.set_config(Arc::clone(&term_config));
                }
            }
            for state in self.pane_state.borrow().values() {
                if let Some(overlay) = &state.overlay {
                    overlay.pane.set_config(Arc::clone(&term_config));
                }
            }
            for state in self.tab_state.borrow().values() {
                if let Some(overlay) = &state.overlay {
                    overlay.pane.set_config(Arc::clone(&term_config));
                }
            }
        }

        if let Some(window) = self.window.as_ref().map(|w| w.clone()) {
            self.load_os_parameters();
            self.apply_scale_change(&dimensions, self.fonts.get_font_scale());
            self.apply_dimensions(&dimensions, None, &window);
            window.config_did_change(&config);
            window.invalidate();
        }

        // Do this after we've potentially adjusted scaling based on config/padding
        // and window size
        self.window_background = reload_background_image(
            &config,
            &self.window_background,
            &self.dimensions,
            &self.render_metrics,
        );

        self.invalidate_modal();
        self.emit_window_event("window-config-reloaded", None);
    }

    fn invalidate_modal(&mut self) {
        if let Some(modal) = self.get_modal() {
            modal.reconfigure(self);
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }
    }

    pub fn cancel_modal(&self) {
        self.modal.borrow_mut().take();
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn set_modal(&self, modal: Rc<dyn Modal>) {
        self.modal.borrow_mut().replace(modal);
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    fn get_modal(&self) -> Option<Rc<dyn Modal>> {
        self.modal.borrow().as_ref().map(|m| Rc::clone(&m))
    }

    fn update_scrollbar(&mut self) {
        if !self.show_scroll_bar {
            return;
        }

        let tab = match self.get_active_pane_or_overlay() {
            Some(tab) => tab,
            None => return,
        };

        let render_dims = tab.get_dimensions();
        if render_dims == self.last_scroll_info {
            return;
        }

        self.last_scroll_info = render_dims;

        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    /// Called by various bits of code to update the title bar.
    /// Let's also trigger the status event so that it can choose
    /// to update the right-status.
    fn update_title(&mut self) {
        self.schedule_status_update();
        self.update_title_impl();
    }

    fn window_contains_pane(&mut self, pane_id: PaneId) -> bool {
        let mux = Mux::get();

        let (_domain, window_id, _tab_id) = match mux.resolve_pane_id(pane_id) {
            Some(tuple) => tuple,
            None => return false,
        };

        return window_id == self.mux_window_id;
    }

    fn emit_user_var_event(&mut self, pane_id: PaneId, name: String, value: String) {
        if !self.window_contains_pane(pane_id) {
            return;
        }

        let mux = Mux::get();
        let window = GuiWin::new(self);
        let pane = match mux.get_pane(pane_id) {
            Some(pane) => mux_lua::MuxPane(pane.pane_id()),
            None => return,
        };

        async fn do_event(
            lua: Option<Rc<mlua::Lua>>,
            name: String,
            value: String,
            window: GuiWin,
            pane: MuxPane,
        ) -> anyhow::Result<()> {
            if let Some(lua) = lua {
                let args = lua.pack_multi((window.clone(), pane, name, value))?;
                if let Err(err) =
                    config::lua::emit_event(&lua, ("user-var-changed".to_string(), args)).await
                {
                    log::error!("while processing user-var-changed event: {:#}", err);
                }
            }

            window
                .window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    term_window.update_title();
                })));

            Ok(())
        }

        promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
            do_event(lua, name, value, window, pane)
        }))
        .detach();
    }

    fn default_right_status(
        _config: &config::ConfigHandle,
        active_pane: Option<&PaneInformation>,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Shell type from pane title
        if let Some(pane) = active_pane {
            let shell = detect_shell_icon(&pane.title);
            if !shell.is_empty() {
                parts.push(shell.to_string());
            }

            // Terminal dimensions (cols x rows)
            parts.push(format!("{}×{}", pane.width, pane.height));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!(" {} ", parts.join(" · "))
        }
    }

    /// Called by window:set_right_status after the status has
    /// been updated; let's update the bar
    pub fn update_title_post_status(&mut self) {
        self.update_title_impl();
    }

    fn update_title_impl(&mut self) {
        let mux = Mux::get();
        let window = match mux.get_window(self.mux_window_id) {
            Some(window) => window,
            _ => return,
        };
        let tabs = self.get_tab_information();
        let panes = self.get_pane_information();
        let active_tab = tabs.iter().find(|t| t.is_active).cloned();
        let active_pane_no_overlay = self.get_active_pane_no_overlay().map(|pane| pane.pane_id());
        let active_pane = active_pane_no_overlay
            .and_then(|pane_id| panes.iter().find(|p| p.pane_id == pane_id).cloned())
            .or_else(|| panes.iter().find(|p| p.is_active).cloned());

        // Keep the integrated titlebar aligned with Windows Terminal / PowerShell.
        // Richer details live in the compact bottom status bar.
        let new_status = String::new();
        if new_status != self.right_status {
            self.right_status = new_status;
        }

        // Refresh this instance's stored cwd so peers that enumerate
        // `instance.list` see up-to-date paths. set_cwd is a no-op
        // when the value matches what's already on disk, so calling
        // it on every title update is cheap.
        let cwd_for_storage = self
            .get_active_pane_no_overlay()
            .and_then(|p| p.get_current_working_dir(mux::pane::CachePolicy::AllowStale))
            .map(|c| {
                c.to_file_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| c.to_string())
            });
        let _ = crate::server_info::set_cwd(cwd_for_storage.clone());

        let border = self.get_os_border();
        let tab_bar_height = self.tab_bar_pixel_height().unwrap_or(0.);
        let tab_bar_y = if self.config.tab_bar_at_bottom {
            ((self.dimensions.pixel_height as f32) - (tab_bar_height + border.bottom.get() as f32))
                .max(0.)
        } else {
            border.top.get() as f32
        };

        let tab_bar_height = self.tab_bar_pixel_height().unwrap_or(0.);

        let hovering_in_tab_bar = match &self.current_mouse_event {
            Some(event) => {
                let mouse_y = event.coords.y as f32;
                mouse_y >= tab_bar_y as f32 && mouse_y < tab_bar_y as f32 + tab_bar_height
            }
            None => false,
        };

        let new_tab_bar = TabBarState::new(
            self.dimensions.pixel_width / self.render_metrics.cell_size.width as usize,
            if hovering_in_tab_bar {
                Some(self.last_mouse_coords.0)
            } else {
                None
            },
            &tabs,
            &panes,
            self.config.resolved_palette.tab_bar.as_ref(),
            &self.config,
            &self.left_status,
            &self.right_status,
        );
        if new_tab_bar != self.tab_bar {
            self.tab_bar = new_tab_bar;
            self.invalidate_fancy_tab_bar();
            self.invalidate_modal();
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }

        // The per-pane stats text (git/cpu/mem/uptime) renders in the
        // fancy tab bar but lives outside TabBarState, so the check
        // above never notices it changing. Compare it explicitly on
        // the same cadence, otherwise the chrome keeps showing the
        // first sample forever on panes whose tabs/title stay quiet.
        let stats_text = crate::termwindow::render::fancy_tab_bar::compose_top_stats_text(self);
        if stats_text != self.last_top_stats_text {
            self.last_top_stats_text = stats_text;
            self.invalidate_fancy_tab_bar();
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }

        let num_tabs = window.len();
        if num_tabs == 0 {
            return;
        }
        drop(window);

        let title = match config::run_immediate_with_lua_config(|lua| {
            if let Some(lua) = lua {
                let tabs = lua.create_sequence_from(tabs.clone().into_iter())?;
                let panes = lua.create_sequence_from(panes.clone().into_iter())?;

                let v = config::lua::emit_sync_callback(
                    &*lua,
                    (
                        "format-window-title".to_string(),
                        (
                            active_tab.clone(),
                            active_pane.clone(),
                            tabs,
                            panes,
                            (*self.config).clone(),
                        ),
                    ),
                )?;
                match &v {
                    mlua::Value::Nil => Ok(None),
                    _ => Ok(Some(String::from_lua(v, &*lua)?)),
                }
            } else {
                Ok(None)
            }
        }) {
            Ok(s) => s,
            Err(err) => {
                log::warn!("format-window-title: {}", err);
                None
            }
        };

        // Window title — optimized for Dock / Cmd-Tab / Mission Control
        // distinguishability when multiple Unterm windows are open.
        //
        // Pattern (Apple convention `<Document> — <App>`):
        //     [Z] [i/N] <project> — Unterm (<instance>)
        //
        // Why <project> leads:
        //   The user (and the user's eyes scanning the Dock window-list
        //   or Cmd-Tab thumbnails) thinks in project names, not in
        //   "Unterm window #3". Leading the title with the cwd basename
        //   lets you tell `~/code/unterm` apart from `~/code/solomd`
        //   at a glance.
        //
        // Document segment resolution order:
        //   1. `info.title` — explicit user-set override
        //      (`unterm-cli instance set-title …` / set in MCP).
        //   2. cwd basename — the "project name" for vibe-coders.
        //   3. `pane.title` — shell-supplied title (OSC 0/2), used as
        //      a last resort when we can't compute a cwd (e.g. very
        //      early startup or a remote pane).
        //
        // Instance segment (NATO phonetic id) is kept as a trailing
        // `(alpha)` so AI agents doing screenshot-OCR can still map
        // a window back to `~/.unterm/instances/<id>.json` metadata.
        //
        // Lua `format-window-title` callbacks still win and bypass
        // all of this — power users can format however they like.
        let info = crate::server_info::read_current();
        let user_title_override = info.title.clone().filter(|t| !t.is_empty());
        let instance_id_segment = Some(info.id).filter(|s| !s.is_empty());

        let project_segment: Option<String> = cwd_for_storage
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let title = match title {
            Some(title) => title,
            None => {
                let zoom = active_pane
                    .as_ref()
                    .map(|p| if p.is_zoomed { "[Z] " } else { "" })
                    .unwrap_or("");
                let tabs_prefix = match &active_tab {
                    Some(tab) if num_tabs > 1 => {
                        format!("[{}/{}] ", tab.tab_index + 1, num_tabs)
                    }
                    _ => String::new(),
                };
                let doc = user_title_override
                    .clone()
                    .or_else(|| project_segment.clone())
                    .or_else(|| active_pane.as_ref().map(|p| p.title.clone()))
                    .unwrap_or_default();

                let head = format!("{}{}{}", zoom, tabs_prefix, doc);
                let body = if head.trim().is_empty() {
                    "Unterm".to_string()
                } else {
                    format!("{} — Unterm", head)
                };
                match instance_id_segment.as_deref() {
                    Some(seg) => format!("{} ({})", body, seg),
                    None => body,
                }
            }
        };

        if let Some(window) = self.window.as_ref() {
            window.set_title(&title);

            let show_tab_bar = if num_tabs == 1 {
                self.config.enable_tab_bar && !self.config.hide_tab_bar_if_only_one_tab
            } else {
                self.config.enable_tab_bar
            };

            // If the number of tabs changed and caused the tab bar to
            // hide/show, then we'll need to resize things.  It is simplest
            // to piggy back on the config reloading code for that, so that
            // is what we're doing.
            if show_tab_bar != self.show_tab_bar {
                self.config_was_reloaded();
            }
        }

        self.schedule_next_status_update();
    }

    fn schedule_next_status_update(&mut self) {
        if let Some(window) = self.window.as_ref() {
            let now = Instant::now();
            if self.last_status_call <= now {
                let interval = Duration::from_millis(self.config.status_update_interval);
                let target = now + interval;
                self.last_status_call = target;

                let window = window.clone();
                promise::spawn::spawn(async move {
                    Timer::at(target).await;
                    window.notify(TermWindowNotif::EmitStatusUpdate);
                })
                .detach();
            }
        }
    }

    fn update_text_cursor(&mut self, pos: &PositionedPane) {
        // While a modal with a text input is up, the IME candidate window
        // belongs next to the modal's caret, not the pane cursor.
        {
            let modal = self.modal.borrow().clone();
            if let Some(modal) = modal {
                if let Some(rect) = modal.ime_cursor_rect(self) {
                    if let Some(win) = self.window.as_ref() {
                        win.set_text_cursor_position(rect);
                    }
                    return;
                }
            }
        }
        if let Some(win) = self.window.as_ref() {
            let cursor = pos.pane.get_cursor_position();
            let top = pos.pane.get_dimensions().physical_top;
            let tab_bar_height = if self.show_tab_bar && !self.config.tab_bar_at_bottom {
                self.tab_bar_pixel_height().unwrap()
            } else {
                0.0
            };
            let (padding_left, padding_top) = self.padding_left_top();

            let r = Rect::new(
                Point::new(
                    (((cursor.x + pos.left) as isize).max(0) * self.render_metrics.cell_size.width)
                        .add(padding_left as isize),
                    ((cursor.y + pos.top as isize - top).max(0)
                        * self.render_metrics.cell_size.height)
                        .add(tab_bar_height as isize)
                        .add(padding_top as isize),
                ),
                self.render_metrics.cell_size,
            );
            win.set_text_cursor_position(r);
        }
    }

    fn activate_window(&mut self, window_idx: usize) -> anyhow::Result<()> {
        let windows = front_end().gui_windows();
        if let Some(win) = windows.get(window_idx) {
            win.window.focus();
        }
        Ok(())
    }

    fn activate_window_relative(&mut self, delta: isize, wrap: bool) -> anyhow::Result<()> {
        let windows = front_end().gui_windows();
        let my_idx = windows
            .iter()
            .position(|w| Some(&w.window) == self.window.as_ref())
            .ok_or_else(|| anyhow!("I'm not in the window list!?"))?;

        let idx = my_idx as isize + delta;

        let idx = if wrap {
            let idx = if idx < 0 {
                windows.len() as isize + idx
            } else {
                idx
            };
            idx as usize % windows.len()
        } else {
            if idx < 0 {
                0
            } else if idx >= windows.len() as isize {
                windows.len().saturating_sub(1)
            } else {
                idx as usize
            }
        };

        if let Some(win) = windows.get(idx) {
            win.window.focus();
        }

        Ok(())
    }

    fn activate_tab(&mut self, tab_idx: isize) -> anyhow::Result<()> {
        let mux = Mux::get();
        let mut window = mux
            .get_window_mut(self.mux_window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        // This logic is coupled with the CliSubCommand::ActivateTab
        // logic in wezterm/src/main.rs. If you update this, update that!
        let max = window.len();

        let tab_idx = if tab_idx < 0 {
            max.saturating_sub(tab_idx.abs() as usize)
        } else {
            tab_idx as usize
        };

        if tab_idx < max {
            window.save_and_then_set_active(tab_idx);

            drop(window);

            if let Some(tab) = self.get_active_pane_or_overlay() {
                tab.focus_changed(true);
            }

            self.update_title();
            self.update_scrollbar();
        }
        Ok(())
    }

    fn activate_tab_relative(&mut self, delta: isize, wrap: bool) -> anyhow::Result<()> {
        let mux = Mux::get();
        let window = mux
            .get_window(self.mux_window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        let max = window.len();
        ensure!(max > 0, "no more tabs");

        // This logic is coupled with the CliSubCommand::ActivateTab
        // logic in wezterm/src/main.rs. If you update this, update that!
        let active = window.get_active_idx() as isize;
        let tab = active + delta;
        let tab = if wrap {
            let tab = if tab < 0 { max as isize + tab } else { tab };
            (tab as usize % max) as isize
        } else {
            if tab < 0 {
                0
            } else if tab >= max as isize {
                max as isize - 1
            } else {
                tab
            }
        };
        drop(window);
        self.activate_tab(tab)
    }

    fn activate_last_tab(&mut self) -> anyhow::Result<()> {
        let mux = Mux::get();
        let window = mux
            .get_window(self.mux_window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        let last_idx = window.get_last_active_idx();
        drop(window);
        match last_idx {
            Some(idx) => self.activate_tab(idx as isize),
            None => Ok(()),
        }
    }

    fn move_tab(&mut self, tab_idx: usize) -> anyhow::Result<()> {
        let mux = Mux::get();
        let mut window = mux
            .get_window_mut(self.mux_window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        let max = window.len();
        ensure!(max > 0, "no more tabs");

        let active = window.get_active_idx();

        ensure!(tab_idx < max, "cannot move a tab out of range");

        let tab_inst = window.remove_by_idx(active);
        window.insert(tab_idx, &tab_inst);
        window.set_active_without_saving(tab_idx);

        drop(window);
        self.update_title();
        self.update_scrollbar();

        Ok(())
    }

    fn show_input_selector(&mut self, args: &config::keyassignment::InputSelector) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        // Ignore any current overlay: we're going to cancel it out below
        // and we don't want this new one to reference that cancelled pane
        let pane = match self.get_active_pane_no_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane = MuxPane(pane.pane_id());

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::selector::selector(term, args, gui_win, pane)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    fn show_prompt_input_line(&mut self, args: &PromptInputLine) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane = MuxPane(pane.pane_id());

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::prompt::show_line_prompt_overlay(term, args, gui_win, pane)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    fn show_confirmation(&mut self, args: &Confirmation) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane = MuxPane(pane.pane_id());

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::confirm::show_confirmation_overlay(term, args, gui_win, pane)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    fn show_shell_selector(&mut self) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let window = self.window.as_ref().unwrap().clone();
        let pane_id = pane.pane_id();

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, mut term| {
            crate::overlay::shell_selector::shell_selector(&mut term, window, pane_id)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    pub fn show_proxy_settings(&mut self) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, mut term| {
            crate::overlay::proxy_settings::proxy_settings(&mut term)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    pub fn show_theme_selector(&mut self) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, mut term| {
            crate::overlay::theme_selector::theme_selector(&mut term)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    pub fn show_tab_context_menu(&mut self, tab_idx: usize) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let window = self.window.as_ref().unwrap().clone();
        let pane_id = pane.pane_id();

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, mut term| {
            crate::overlay::tab_context_menu::tab_context_menu(&mut term, window, pane_id, tab_idx)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    /// Right-click is a direct gesture, not a menu trigger:
    ///   * with selection → copy + clear selection
    ///   * without selection → paste from clipboard
    ///
    /// Settings entry points (Themes / Proxy / Project Directory / Shell
    /// Selector) are reached via the status bar buttons, the OS app menu bar,
    /// and keyboard shortcuts — never by right-click.
    fn show_context_menu(&mut self) {
        let Some(pane) = self.get_active_pane_or_overlay() else {
            return;
        };
        self.right_click_copy_or_paste(&pane);
    }

    /// Run the right-click action against the pane that was actually clicked.
    /// This matters for split layouts: the active-pane snapshot can lag the
    /// mouse press, which previously made copy/paste appear to do nothing (or
    /// act on the neighboring split).
    pub(crate) fn right_click_copy_or_paste(&mut self, pane: &Arc<dyn Pane>) {
        use config::keyassignment::{
            ClipboardCopyDestination, ClipboardPasteSource, KeyAssignment,
        };
        let has_selection = !self.selection_text(pane).is_empty();

        let action = right_click_action(has_selection);
        match action {
            RightClickAction::CopySelection => {
                let assignment = KeyAssignment::CopyTo(ClipboardCopyDestination::Clipboard);
                match self.perform_key_assignment(pane, &assignment) {
                    Ok(_) => self.show_ui_notice(crate::i18n::t("interaction.copied")),
                    Err(err) => log::warn!("right-click copy failed: {err:#}"),
                }
            }
            RightClickAction::PasteClipboard => {
                // Clipboard reads are asynchronous. The success/failure notice
                // is emitted only after the clipboard has been read and the
                // pane has accepted the paste.
                self.paste_from_clipboard_with_feedback(
                    pane,
                    ClipboardPasteSource::Clipboard,
                    true,
                );
            }
        }
        if has_selection {
            // Clear the selection so the user gets a clean prompt back.
            let pane_id = pane.pane_id();
            self.selection(pane_id).clear();
        }
    }

    /// Open the settings menu — the Tab bar's `▼` button is the GUI entry
    /// point for everything configuration-related (Themes, Proxy, Project
    /// Directory, Shell Selector). The right-click gesture is reserved for
    /// direct copy/paste, so this is the *only* place a menu surfaces.
    /// Width in physical pixels the tree sidebar currently occupies (0 when
    /// closed). Injected at every window_padding.left evaluation so panes,
    /// splits, terminal cols and mouse mapping all shift together.
    pub(crate) fn tree_sidebar_pixel_width(&self) -> f32 {
        let Some(raw_width_pts) = self.tree_sidebar_raw_width_pts() else {
            return 0.0;
        };
        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let max =
            self.tree_sidebar_max_width_pts(window_pts)
                .min(self.left_gutter_limited_width_pts(
                    raw_width_pts,
                    self.left_tab_bar_raw_width_pts(),
                    config::ui_tokens::TREE_SIDEBAR_MIN_WIDTH,
                    config::ui_tokens::LEFT_TAB_BAR_MIN_WIDTH,
                ));
        (raw_width_pts.clamp(config::ui_tokens::TREE_SIDEBAR_MIN_WIDTH, max) * pt).round()
    }

    pub(crate) fn tree_sidebar_raw_width_pts(&self) -> Option<f32> {
        let tree = self.tree_sidebar.borrow();
        let Some(tree) = tree.as_ref() else {
            return None;
        };

        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let max = self.tree_sidebar_max_width_pts(window_pts);
        Some(
            tree.width_pts
                .unwrap_or(config::ui_tokens::TREE_SIDEBAR_WIDTH)
                .clamp(config::ui_tokens::TREE_SIDEBAR_MIN_WIDTH, max),
        )
    }

    fn tree_sidebar_max_width_pts(&self, window_pts: f32) -> f32 {
        (window_pts * config::ui_tokens::TREE_SIDEBAR_MAX_RATIO)
            .max(config::ui_tokens::TREE_SIDEBAR_MIN_WIDTH)
    }

    pub(crate) fn resize_tree_sidebar(&mut self, x_px: f32) {
        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let x_px = if pt > 1.0 && x_px <= window_pts + 2.0 {
            x_px * pt
        } else {
            x_px
        };
        let border = self.get_os_border();
        let left = border.left.get() as f32 + self.left_tab_bar_pixel_width();
        let w_pts = (x_px - left) / pt;
        let max =
            self.tree_sidebar_max_width_pts(window_pts)
                .min(self.left_gutter_limited_width_pts(
                    w_pts,
                    self.left_tab_bar_raw_width_pts(),
                    config::ui_tokens::TREE_SIDEBAR_MIN_WIDTH,
                    config::ui_tokens::LEFT_TAB_BAR_MIN_WIDTH,
                ));
        let clamped = w_pts.clamp(config::ui_tokens::TREE_SIDEBAR_MIN_WIDTH, max);
        if let Some(tree) = self.tree_sidebar.borrow_mut().as_mut() {
            tree.width_pts = Some(clamped);
        }
        if let Some(window) = self.window.as_ref().cloned() {
            let dims = self.dimensions;
            self.apply_dimensions(&dims, None, &window);
            window.invalidate();
        }
    }

    /// Toggle the directory-tree sidebar, rooted at the active pane's cwd.
    pub(crate) fn toggle_tree_sidebar(&mut self) {
        let is_open = self.tree_sidebar.borrow().is_some();
        if is_open {
            self.tree_sidebar.borrow_mut().take();
        } else {
            let root = self
                .get_active_pane_or_overlay()
                .and_then(|pane| pane_cwd_path(&pane))
                .or_else(dirs_next::home_dir)
                .unwrap_or_else(|| std::path::PathBuf::from("/"));
            self.tree_sidebar
                .borrow_mut()
                .replace(crate::termwindow::tree_sidebar::TreeSidebar::new(root));
        }
        // Reflow the terminal around the new gutter width.
        if let Some(window) = self.window.as_ref().cloned() {
            let dims = self.dimensions;
            self.apply_dimensions(&dims, None, &window);
            window.invalidate();
        }
    }

    /// Physical pixels the right-docked git panel occupies (0 when closed).
    /// Injected into `window_padding.right` at every terminal-size evaluation
    /// so the panes reflow to leave a reserved gutter on the right edge.
    pub(crate) fn git_panel_pixel_width(&self) -> f32 {
        if self.git_panel.borrow().is_none() {
            return 0.0;
        }
        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let max = (window_pts * config::ui_tokens::GIT_PANEL_MAX_RATIO)
            .max(config::ui_tokens::GIT_PANEL_MIN_WIDTH);
        (config::ui_tokens::GIT_PANEL_WIDTH
            .clamp(config::ui_tokens::GIT_PANEL_MIN_WIDTH, max)
            * pt)
            .round()
    }

    /// Toggle the right-docked source-control panel, anchored at the active
    /// pane's cwd.
    pub(crate) fn toggle_git_panel(&mut self) {
        let is_open = self.git_panel.borrow().is_some();
        if is_open {
            self.git_panel.borrow_mut().take();
        } else {
            let cwd = self
                .get_active_pane_or_overlay()
                .and_then(|pane| pane_cwd_path(&pane))
                .or_else(dirs_next::home_dir)
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            self.git_panel
                .borrow_mut()
                .replace(crate::termwindow::git_panel::GitPanel::new(cwd));
        }
        // Reflow the terminal around the new gutter width.
        if let Some(window) = self.window.as_ref().cloned() {
            let dims = self.dimensions;
            self.apply_dimensions(&dims, None, &window);
            window.invalidate();
        }
    }

    /// Toggle the Composer + prompt-queue overlay. The queue itself lives on
    /// `self.composer` and is preserved across toggles within a session.
    pub(crate) fn toggle_composer(&mut self) {
        let is_open = self
            .get_modal()
            .map_or(false, |m| m.downcast_ref::<composer::Composer>().is_some());
        if is_open {
            self.cancel_modal();
        } else {
            self.set_modal(std::rc::Rc::new(composer::Composer::new()));
        }
    }

    /// Rebuild the composer overlay's cached element and repaint.
    pub(crate) fn invalidate_composer(&mut self) {
        self.invalidate_modal();
    }

    /// Move the enqueued draft (if non-empty) onto the back of the queue.
    pub(crate) fn composer_enqueue(&mut self) {
        {
            let mut c = self.composer.borrow_mut();
            let draft = std::mem::take(&mut c.draft);
            let trimmed = draft.trim();
            if trimmed.is_empty() {
                c.draft = draft;
                return;
            }
            c.queue.push(trimmed.to_string());
            c.selected = c.queue.len().saturating_sub(1);
            c.status = None;
        }
        self.invalidate_composer();
    }

    pub(crate) fn composer_move_selection(&mut self, delta: isize) {
        {
            let mut c = self.composer.borrow_mut();
            if c.queue.is_empty() {
                return;
            }
            let len = c.queue.len() as isize;
            let next = (c.selected as isize + delta).rem_euclid(len);
            c.selected = next as usize;
        }
        self.invalidate_composer();
    }

    pub(crate) fn composer_remove_selected(&mut self) {
        {
            let mut c = self.composer.borrow_mut();
            // Don't yank the prompt out from under an in-flight run.
            if c.is_running() {
                return;
            }
            let idx = c.selected;
            if idx < c.queue.len() {
                c.queue.remove(idx);
                if c.selected >= c.queue.len() {
                    c.selected = c.queue.len().saturating_sub(1);
                }
            }
        }
        self.invalidate_composer();
    }

    pub(crate) fn composer_clear(&mut self) {
        {
            let mut c = self.composer.borrow_mut();
            if c.is_running() {
                return;
            }
            c.queue.clear();
            c.selected = 0;
            c.status = None;
        }
        self.invalidate_composer();
    }

    /// Begin dispatching the queue to the active pane, one prompt at a time.
    pub(crate) fn composer_run_start(&mut self) {
        let pane_id = match self.get_active_pane_or_overlay() {
            Some(pane) => pane.pane_id(),
            None => return,
        };
        {
            let mut c = self.composer.borrow_mut();
            if c.is_running() || c.queue.is_empty() {
                return;
            }
            c.generation = c.generation.wrapping_add(1);
            let now = std::time::Instant::now();
            c.run = Some(composer::RunState {
                generation: c.generation,
                pane_id,
                last_seqno: 0,
                last_change: now,
                sent_at: now,
                phase: composer::RunPhase::Running,
            });
            c.status = Some("Running…".to_string());
        }
        self.composer_send_current();
        self.invalidate_composer();
    }

    /// Write the front-of-queue prompt to the run's pane, followed by Enter,
    /// then arm the idle poll. If the queue is empty, the run is finished.
    fn composer_send_current(&mut self) {
        let (pane_id, generation, text) = {
            let c = self.composer.borrow();
            match &c.run {
                Some(run) => (run.pane_id, run.generation, c.queue.first().cloned()),
                None => return,
            }
        };
        let Some(text) = text else {
            self.composer_finish(Some("Queue complete".to_string()));
            return;
        };
        let pane = match mux::Mux::get().get_pane(pane_id) {
            Some(pane) => pane,
            None => {
                self.composer_finish(Some("Run stopped: pane closed".to_string()));
                return;
            }
        };
        {
            let mut writer = pane.writer();
            let mut bytes = text.into_bytes();
            bytes.push(b'\r');
            if let Err(err) = writer.write_all(&bytes) {
                log::warn!("composer: failed to write prompt: {err:#}");
                self.composer_finish(Some("Run stopped: write error".to_string()));
                return;
            }
        }
        let seqno = pane.get_current_seqno();
        {
            let mut c = self.composer.borrow_mut();
            if let Some(run) = c.run.as_mut() {
                let now = std::time::Instant::now();
                run.last_seqno = seqno;
                run.last_change = now;
                run.sent_at = now;
                run.phase = composer::RunPhase::Running;
            }
        }
        self.schedule_composer_poll(generation);
    }

    /// Poll callback: detect pane idle and route to a mode-specific decision.
    pub(crate) fn composer_poll_idle(&mut self, generation: u64) {
        use composer::RunPhase;
        enum Next {
            Reschedule,
            /// Idle detected in an actionable phase; make an advance decision.
            Decide,
            Abort,
        }
        let next = {
            let mut c = self.composer.borrow_mut();
            let Some(run) = c.run.as_mut() else {
                return;
            };
            if run.generation != generation {
                return;
            }
            match mux::Mux::get().get_pane(run.pane_id) {
                None => Next::Abort,
                Some(pane) => {
                    let seq = pane.get_current_seqno();
                    let now = std::time::Instant::now();
                    if seq != run.last_seqno {
                        run.last_seqno = seq;
                        run.last_change = now;
                        // New output while auto-paused means the user is
                        // operating the pane; arm re-evaluation on next idle.
                        if let RunPhase::AutoPaused { moved } = &mut run.phase {
                            *moved = true;
                        }
                    }
                    let idle = now.duration_since(run.last_change) >= composer::IDLE_DEBOUNCE
                        && now.duration_since(run.sent_at) >= composer::MIN_GRACE;
                    if !idle {
                        Next::Reschedule
                    } else {
                        match run.phase {
                            // Wait for the pane to move before re-inspecting.
                            RunPhase::AutoPaused { moved: false } => Next::Reschedule,
                            // ManualPaused stops rescheduling from the decision
                            // point, so we should never see it here.
                            RunPhase::ManualPaused => Next::Reschedule,
                            _ => Next::Decide,
                        }
                    }
                }
            }
        };
        match next {
            Next::Reschedule => self.schedule_composer_poll(generation),
            Next::Abort => self.composer_finish(Some("Run stopped: pane closed".to_string())),
            Next::Decide => self.composer_decide_advance(generation),
        }
    }

    /// At an idle point, decide what to do based on the current advance mode.
    fn composer_decide_advance(&mut self, generation: u64) {
        use composer::{AdvanceMode, PromptKind, RunPhase};
        let mode = self.composer.borrow().advance_mode;
        match mode {
            AdvanceMode::AutoNext => self.composer_advance(),
            AdvanceMode::Manual => {
                {
                    let mut c = self.composer.borrow_mut();
                    if let Some(run) = c.run.as_mut() {
                        run.phase = RunPhase::ManualPaused;
                    }
                    c.status = Some("Paused — Ctrl+Enter to send next".to_string());
                }
                self.invalidate_composer();
                // Do not reschedule: the run resumes on the user's keystroke.
            }
            AdvanceMode::AutoApprove => match self.composer_inspect_pane() {
                PromptKind::Done => self.composer_advance(),
                PromptKind::ConfirmEnter => self.composer_approve(generation, b"\r", "Yes"),
                PromptKind::ConfirmYes => self.composer_approve(generation, b"y\r", "y"),
                PromptKind::Pause => {
                    {
                        let mut c = self.composer.borrow_mut();
                        if let Some(run) = c.run.as_mut() {
                            run.phase = RunPhase::AutoPaused { moved: false };
                        }
                        c.status =
                            Some("Paused — choose in the pane (auto-resumes)".to_string());
                    }
                    self.invalidate_composer();
                    // Keep polling so we notice the pane moving and re-inspect.
                    self.schedule_composer_poll(generation);
                }
            },
        }
    }

    /// Read the active run pane's bottom ~12 visible lines as plain strings and
    /// classify them. Called only at the idle-advance decision point, never per
    /// frame.
    fn composer_inspect_pane(&self) -> composer::PromptKind {
        let pane_id = match &self.composer.borrow().run {
            Some(run) => run.pane_id,
            None => return composer::PromptKind::Done,
        };
        let pane = match mux::Mux::get().get_pane(pane_id) {
            Some(pane) => pane,
            None => return composer::PromptKind::Done,
        };
        let dims = pane.get_dimensions();
        let last_row = dims.physical_top + dims.viewport_rows as isize;
        let first_row = (last_row - 12).max(dims.physical_top);
        let (_first, lines) = pane.get_lines(first_row..last_row);
        let text: Vec<String> = lines
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect();
        composer::classify_prompt(&text)
    }

    /// Answer a detected confirmation by writing `bytes` to the pane WITHOUT
    /// popping the queue: this replies to a mid-task question, then keeps
    /// polling so the run continues once the pane settles again.
    fn composer_approve(&mut self, generation: u64, bytes: &[u8], label: &str) {
        let pane_id = match &self.composer.borrow().run {
            Some(run) => run.pane_id,
            None => return,
        };
        let pane = match mux::Mux::get().get_pane(pane_id) {
            Some(pane) => pane,
            None => {
                self.composer_finish(Some("Run stopped: pane closed".to_string()));
                return;
            }
        };
        if let Err(err) = pane.writer().write_all(bytes) {
            log::warn!("composer: failed to write approval: {err:#}");
            self.composer_finish(Some("Run stopped: write error".to_string()));
            return;
        }
        let seqno = pane.get_current_seqno();
        {
            let mut c = self.composer.borrow_mut();
            if let Some(run) = c.run.as_mut() {
                let now = std::time::Instant::now();
                run.last_seqno = seqno;
                run.last_change = now;
                run.sent_at = now;
                run.phase = composer::RunPhase::Running;
            }
            c.status = Some(format!("Auto-approved ({label})…"));
        }
        self.invalidate_composer();
        self.schedule_composer_poll(generation);
    }

    /// Pop the finished prompt and send the next, or finish if the queue is
    /// empty. Shared by every "advance to the next prompt" path.
    fn composer_advance(&mut self) {
        let empty = {
            let mut c = self.composer.borrow_mut();
            if !c.queue.is_empty() {
                c.queue.remove(0);
            }
            c.selected = 0;
            c.queue.is_empty()
        };
        if empty {
            self.composer_finish(Some("Queue complete".to_string()));
        } else {
            self.composer_send_current();
            self.invalidate_composer();
        }
    }

    /// Ctrl+Enter while paused (Manual, or an auto-pause the user wants to skip)
    /// force-advances to the next queued prompt.
    pub(crate) fn composer_force_advance(&mut self) {
        let paused = self
            .composer
            .borrow()
            .run
            .as_ref()
            .map_or(false, |r| r.phase.is_paused());
        if paused {
            self.composer_advance();
        }
    }

    /// Stop an in-flight run, leaving any remaining prompts in the queue.
    pub(crate) fn composer_stop(&mut self) {
        let was_running = self.composer.borrow().is_running();
        if was_running {
            self.composer_finish(Some("Stopped".to_string()));
        }
    }

    fn composer_finish(&mut self, status: Option<String>) {
        {
            let mut c = self.composer.borrow_mut();
            // Bump the generation so any already-scheduled poll is ignored.
            c.generation = c.generation.wrapping_add(1);
            c.run = None;
            c.status = status;
        }
        self.invalidate_composer();
    }

    /// Arm a one-shot timer that re-enters `composer_poll_idle` on the main
    /// thread after `POLL_INTERVAL`.
    fn schedule_composer_poll(&self, generation: u64) {
        let Some(window) = self.window.clone() else {
            return;
        };
        promise::spawn::spawn(async move {
            smol::Timer::after(composer::POLL_INTERVAL).await;
            window.notify(TermWindowNotif::Apply(Box::new(move |tw| {
                tw.composer_poll_idle(generation);
            })));
        })
        .detach();
    }

    /// Open the directory-jump palette (v0.40 "B"): fuzzy go-to-directory
    /// rooted at the active pane's cwd.
    pub(crate) fn show_dir_jump(&mut self) {
        self.show_dir_jump_with_action(crate::termwindow::dir_jump::DirJumpAction::ChangeCwd);
    }

    /// Open the Agent Inbox palette (cockpit).
    pub(crate) fn show_cockpit_inbox(&mut self) {
        let modal = crate::termwindow::cockpit_inbox::CockpitInbox::new();
        self.set_modal(std::rc::Rc::new(modal));
    }

    /// Open the fleet-launch palette (cockpit).
    pub(crate) fn show_fleet_palette(&mut self) {
        let Some(pane) = self.get_active_pane_or_overlay() else {
            return;
        };
        let base = pane_cwd_path(&pane)
            .or_else(dirs_next::home_dir)
            .unwrap_or_else(|| std::path::PathBuf::from("/"));
        let modal = crate::termwindow::fleet_palette::FleetPalette::new(base);
        self.set_modal(std::rc::Rc::new(modal));
    }

    pub(crate) fn show_dir_jump_with_action(
        &mut self,
        action: crate::termwindow::dir_jump::DirJumpAction,
    ) {
        let Some(pane) = self.get_active_pane_or_overlay() else {
            return;
        };
        let pane_id = pane.pane_id();
        let base = pane_cwd_path(&pane)
            .or_else(dirs_next::home_dir)
            .unwrap_or_else(|| std::path::PathBuf::from("/"));
        let modal = crate::termwindow::dir_jump::DirJump::with_action(pane_id, base, action);
        self.set_modal(std::rc::Rc::new(modal));
    }

    /// Shape one of the search bar's strings so the font-fallback caches
    /// are hot before the user first opens search. Called at idle shortly
    /// after window creation, one string per call to keep each slice of
    /// main-thread work small.
    fn prewarm_search_bar_glyphs(&mut self, text: &str) {
        let start = std::time::Instant::now();
        // Warm BOTH the terminal font (search bar) and the title font: the
        // dir-jump / folder overlay renders its labels with the title font, so
        // warming only the default font left its CJK/⌕ fallback cold and the
        // picker flashed tofu on its first frame.
        // IntoIterator::into_iter form: this crate is edition 2018, where
        // `array.into_iter()` resolves to iterating references and trips
        // the `array_into_iter` lint under CI's -D warnings.
        for font in
            IntoIterator::into_iter([self.fonts.default_font(), self.fonts.title_font()]).flatten()
        {
            let _ = font.shape(
                text,
                || {},
                crate::customglyph::BlockKey::filter_out_synthetic,
                None,
                wezterm_bidi::Direction::LeftToRight,
                None,
                None,
            );
        }
        log::debug!(
            "glyph prewarm ({text:?}) took {:?}",
            start.elapsed()
        );
    }

    fn prewarm_settings_menu(&mut self) {
        if self.prewarmed_settings_menu.borrow().is_some() {
            return;
        }
        let Some(pane) = self.get_active_pane_or_overlay() else {
            return;
        };
        let menu = Rc::new(crate::termwindow::popup_menu::PopupMenu::build_default(
            pane.pane_id(),
        ));
        let start = std::time::Instant::now();
        let mut result = menu.precompute(self);
        if let Err(err) = &result {
            // The menu's glyphs (codicons, CJK labels) can outgrow the
            // startup atlas. Grow it here, at idle, the same way the
            // paint pass does — that also spares the first real menu
            // open from paying the atlas-growth hitch.
            if let Some(&::window::bitmaps::atlas::OutOfTextureSpace {
                size: Some(size), ..
            }) = err.root_cause().downcast_ref()
            {
                log::debug!("settings menu prewarm: growing atlas to {size}");
                if let Err(err) = self.recreate_texture_atlas(Some(size)) {
                    log::warn!("settings menu prewarm: atlas grow failed: {err:#}");
                } else {
                    self.invalidate_fancy_tab_bar();
                    self.invalidate_modal();
                    self.invalidate_left_tab_bar();
                    result = menu.precompute(self);
                }
            }
        }
        match result {
            Ok(()) => {
                // Prewarm glyph/font fallback, but do not reuse the computed
                // element. Reusing the first layout can briefly show stale
                // fallback glyphs or stale theme colors when the menu opens.
                menu.clear_precomputed();
                self.prewarmed_settings_menu.borrow_mut().replace(menu);
                if let Some(window) = self.window.as_ref() {
                    window.invalidate();
                }
                log::debug!("settings menu prewarm took {:?}", start.elapsed());
            }
            Err(err) => {
                log::warn!("settings menu prewarm failed: {err:#}");
            }
        }
    }

    fn show_settings_menu(&mut self) {
        // v0.40: mouse-operable floating menu (popup_menu.rs) replaces the
        // keyboard-only cell-grid overlay that used to live here.
        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };
        let pane_id = pane.pane_id();
        let cached_menu = self
            .prewarmed_settings_menu
            .borrow_mut()
            .take()
            .filter(|menu| menu.pane_id() == pane_id);
        let menu = cached_menu.unwrap_or_else(|| {
            Rc::new(crate::termwindow::popup_menu::PopupMenu::build_default(
                pane_id,
            ))
        });
        menu.clear_precomputed();
        self.set_modal(menu);
    }

    pub(crate) fn open_project_directory_from_menu(&mut self) {
        self.show_dir_jump_with_action(crate::termwindow::dir_jump::DirJumpAction::NewTab);
    }

    pub(crate) fn open_project_directory_from_system_picker(&mut self) {
        let Some(pane) = self.get_active_pane_no_overlay() else {
            return;
        };
        let pane_id = pane.pane_id();
        let Some(window) = self.window.as_ref().cloned() else {
            return;
        };
        let start_at = pane_cwd_path(&pane);
        crate::termwindow::mouseevent::write_unterm_status_to_pane(
            &pane,
            &crate::i18n::t("project.prompt_new_tab"),
        );
        std::thread::spawn(move || {
            crate::termwindow::mouseevent::open_project_directory_in_new_tab(
                window, pane_id, start_at,
            );
        });
    }

    /// Pop a folder picker, then split the current pane horizontally and
    /// spawn a shell in the picked directory in the new right-side pane.
    pub(crate) fn open_folder_in_split(&mut self, pane_id: mux::pane::PaneId) {
        let _ = pane_id;
        self.show_dir_jump_with_action(crate::termwindow::dir_jump::DirJumpAction::SplitRight);
    }

    /// Pop a native folder picker, then split the current pane horizontally
    /// and spawn a shell in the picked directory in the new right-side pane.
    pub(crate) fn open_folder_in_split_system_picker(&mut self, pane_id: mux::pane::PaneId) {
        use config::keyassignment::{KeyAssignment, SpawnCommand};
        let Some(pane) = self.get_active_pane_no_overlay() else {
            return;
        };
        let Some(window) = self.window.as_ref().cloned() else {
            return;
        };
        let start_at = pane_cwd_path(&pane);
        crate::termwindow::mouseevent::write_unterm_status_to_pane(
            &pane,
            &crate::i18n::t("project.prompt_split"),
        );
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            let picked = crate::termwindow::mouseevent::pick_project_directory_starting_at(
                start_at.as_deref(),
            );
            #[cfg(not(target_os = "windows"))]
            let picked = crate::termwindow::mouseevent::pick_project_directory_unix_starting_at(
                start_at.as_deref(),
            );
            match picked {
                Ok(path) => {
                    window.notify(crate::termwindow::TermWindowNotif::PerformAssignment {
                        pane_id,
                        assignment: KeyAssignment::SplitHorizontal(SpawnCommand {
                            cwd: Some(path),
                            ..SpawnCommand::default()
                        }),
                        tx: None,
                    });
                }
                Err(err) => {
                    log::warn!("open-folder-in-split canceled: {err:#}");
                }
            }
        });
    }

    /// Pop a folder picker, then `cd` the chosen path in the *current* pane —
    /// no new tab. The cd is shell-quoted and written to the pane's PTY input,
    /// so the user sees and confirms it.
    pub(crate) fn change_working_directory_for_pane(&mut self, _pane_id: mux::pane::PaneId) {
        self.show_dir_jump_with_action(crate::termwindow::dir_jump::DirJumpAction::ChangeCwd);
    }

    /// Native folder picker fallback for Cmd+O from the directory jump palette.
    pub(crate) fn change_working_directory_for_pane_system_picker(
        &mut self,
        _pane_id: mux::pane::PaneId,
    ) {
        let Some(pane) = self.get_active_pane_no_overlay() else {
            return;
        };
        let start_at = pane_cwd_path(&pane);
        crate::termwindow::mouseevent::write_unterm_status_to_pane(
            &pane,
            &crate::i18n::t("cwd.prompt"),
        );
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            let picked = crate::termwindow::mouseevent::pick_project_directory_starting_at(
                start_at.as_deref(),
            );
            #[cfg(not(target_os = "windows"))]
            let picked = crate::termwindow::mouseevent::pick_project_directory_unix_starting_at(
                start_at.as_deref(),
            );

            match picked {
                Ok(path) => {
                    let cmd = cd_command_for_pane(&pane, &path);
                    if let Err(err) = pane.writer().write_all(cmd.as_bytes()) {
                        log::warn!("could not inject cd command: {err:#}");
                    }
                }
                Err(err) => {
                    log::warn!("change-cwd canceled: {err:#}");
                }
            }
        });
    }

    /// Toggle session recording on/off for the given pane. Status is
    /// announced inline in the pane so the user always knows whether the
    /// red dot they see corresponds to an active recording.
    pub(crate) fn toggle_session_recording(&mut self, pane_id: mux::pane::PaneId) {
        let Some(pane) = self.get_active_pane_no_overlay() else {
            return;
        };
        if crate::recording::recorder::current_session(pane_id).is_some() {
            match crate::recording::recorder::stop_recording(pane_id) {
                Ok(stop) => {
                    crate::termwindow::mouseevent::write_unterm_status_to_pane(
                        &pane,
                        &crate::i18n::t_args(
                            "recording.stopped",
                            &[
                                ("blocks", &stop.block_count.to_string()),
                                ("path", &stop.md_path),
                            ],
                        ),
                    );
                }
                Err(err) => {
                    log::error!("recording stop failed: {err:#}");
                    crate::termwindow::mouseevent::write_unterm_status_to_pane(
                        &pane,
                        &crate::i18n::t_args(
                            "recording.stop_failed",
                            &[("err", &format!("{err:#}"))],
                        ),
                    );
                }
            }
        } else {
            match crate::recording::recorder::start_recording(pane_id) {
                Ok(start) => {
                    crate::termwindow::mouseevent::write_unterm_status_to_pane(
                        &pane,
                        &crate::i18n::t_args("recording.started", &[("path", &start.log_path)]),
                    );
                }
                Err(err) => {
                    log::error!("recording start failed: {err:#}");
                    crate::termwindow::mouseevent::write_unterm_status_to_pane(
                        &pane,
                        &crate::i18n::t_args(
                            "recording.start_failed",
                            &[("err", &format!("{err:#}"))],
                        ),
                    );
                }
            }
        }
    }

    /// One-shot dump of the current pane's scrollback to a markdown file
    /// under `~/.unterm/sessions/`. Independent of recording state.
    pub(crate) fn export_current_session(&mut self, pane_id: mux::pane::PaneId) {
        let Some(pane) = self.get_active_pane_no_overlay() else {
            return;
        };
        match crate::recording::recorder::export_pane_markdown(pane_id, None) {
            Ok((path, _output)) => {
                let path_str = path.display().to_string();
                if let Err(err) = crate::termwindow::mouseevent::copy_text_to_clipboard(&path_str) {
                    log::warn!("could not copy export path to clipboard: {err:#}");
                }
                crate::termwindow::mouseevent::write_unterm_status_to_pane(
                    &pane,
                    &crate::i18n::t_args("recording.exported", &[("path", &path_str)]),
                );
            }
            Err(err) => {
                log::error!("session export failed: {err:#}");
                crate::termwindow::mouseevent::write_unterm_status_to_pane(
                    &pane,
                    &crate::i18n::t_args(
                        "recording.export_failed",
                        &[("err", &format!("{err:#}"))],
                    ),
                );
            }
        }
    }

    /// Open the Unterm Web Settings UI in the user's default browser.
    /// Uses THIS instance's port (via `read_current`), not whichever
    /// instance owns active.json — otherwise clicking the chip in
    /// bravo's status bar opens alpha's web settings, which is
    /// confusing and reaches the wrong process's state.
    pub(crate) fn open_web_settings(&mut self) {
        self.open_web_settings_fragment(None);
    }

    pub(crate) fn open_web_settings_fragment(&mut self, fragment: Option<&str>) {
        let info = crate::server_info::read_current();
        if info.http_port == 0 {
            log::warn!("web settings: http_port not yet bound; cannot open browser");
            return;
        }
        let url = match fragment {
            Some(fragment) => format!("http://127.0.0.1:{}#{}", info.http_port, fragment),
            None => format!("http://127.0.0.1:{}", info.http_port),
        };

        #[cfg(target_os = "macos")]
        let opener = "open";
        #[cfg(target_os = "windows")]
        let opener = "explorer";
        #[cfg(all(unix, not(target_os = "macos")))]
        let opener = "xdg-open";

        if let Err(err) = std::process::Command::new(opener).arg(&url).spawn() {
            log::warn!("could not open web settings in browser: {err:#}");
        }
    }

    /// Reveal `~/.unterm/sessions/` in the platform's file manager.
    pub(crate) fn open_sessions_folder(&mut self) {
        let Some(home) = dirs_next::home_dir() else {
            return;
        };
        let sessions_dir = home.join(".unterm").join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);

        #[cfg(target_os = "macos")]
        let opener = "open";
        #[cfg(target_os = "windows")]
        let opener = "explorer";
        #[cfg(all(unix, not(target_os = "macos")))]
        let opener = "xdg-open";

        if let Err(err) = std::process::Command::new(opener)
            .arg(&sessions_dir)
            .spawn()
        {
            log::warn!("could not open sessions folder: {err:#}");
        }
    }

    fn show_insights_overlay(&mut self) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };
        let Some(pane) = self.get_active_pane_no_overlay() else {
            return;
        };

        // Compute the snapshot on the GUI thread BEFORE spawning
        // the overlay. The overlay runs in a separate task and
        // shouldn't reach back into Mux state — keeping the
        // snapshot self-contained avoids cross-thread fragility.
        let pane_id = pane.pane_id();
        let shell_name = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .unwrap_or_else(|| "(unknown)".to_string());
        let cwd = pane
            .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
            .map(|c| {
                c.to_file_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| c.as_str().to_string())
            });
        let dims = pane.get_dimensions();
        let recent_commits = crate::ghost_text::recent_global_commits(10);
        let top_commits = crate::ghost_text::commit_frequency(5);
        let activity = crate::mcp::handler::recent_mcp_input_activity();
        let mcp = crate::mcp::handler::insights_mcp_snapshot(8);

        let snap = crate::overlay::InsightsSnapshot {
            pane_id: pane_id as u64,
            shell_name,
            cwd,
            cols: dims.cols,
            rows: dims.viewport_rows,
            recent_commits,
            top_commits,
            mcp_input_count: activity.count,
            seconds_since_last_input: activity.seconds_since_last,
            recent_audit: mcp.recent_audit,
            agents_seen: mcp.agents_seen,
            pending_suggestions: mcp.pending_suggestions,
            pending_confirmations: mcp.pending_confirmations,
        };

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::show_insights_overlay(term, snap)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    fn show_debug_overlay(&mut self) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        let gui_win = GuiWin::new(self);

        let opengl_info = self.opengl_info.as_deref().unwrap_or("Unknown").to_string();
        let connection_info = self.connection_name.clone();

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::show_debug_overlay(term, gui_win, opengl_info, connection_info)
        });
        self.assign_overlay(tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    pub(crate) fn show_tab_navigator(&mut self) {
        let mux = Mux::get();
        let active_tab_idx = match mux.get_window(self.mux_window_id) {
            Some(mux_window) => mux_window.get_active_idx(),
            None => return,
        };
        let title = "Tab Navigator".to_string();
        let args = LauncherActionArgs {
            title: Some(title),
            flags: LauncherFlags::TABS | LauncherFlags::FUZZY,
            help_text: Some(
                "Type a project, path, agent or command · Enter=switch · Esc=cancel".to_string(),
            ),
            fuzzy_help_text: Some("Find window: ".to_string()),
            alphabet: None,
        };
        self.show_launcher_impl(args, active_tab_idx);
    }

    fn show_launcher(&mut self) {
        let title = "Launcher".to_string();
        let args = LauncherActionArgs {
            title: Some(title),
            flags: LauncherFlags::LAUNCH_MENU_ITEMS
                | LauncherFlags::WORKSPACES
                | LauncherFlags::DOMAINS
                | LauncherFlags::KEY_ASSIGNMENTS
                | LauncherFlags::COMMANDS,
            help_text: None,
            fuzzy_help_text: None,
            alphabet: None,
        };
        self.show_launcher_impl(args, 0);
    }

    fn show_launcher_impl(&mut self, args: LauncherActionArgs, initial_choice_idx: usize) {
        let mux_window_id = self.mux_window_id;
        let window = self.window.as_ref().unwrap().clone();

        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };

        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let domain_id_of_current_pane = tab
            .get_active_pane()
            .expect("tab has no panes!")
            .domain_id();
        let pane_id = pane.pane_id();
        let tab_id = tab.tab_id();
        let title = args.title.unwrap();
        let flags = args.flags;
        let help_text = args.help_text.unwrap_or(
            "Select an item and press Enter=launch  \
             Esc=cancel  /=filter"
                .to_string(),
        );
        let fuzzy_help_text = args
            .fuzzy_help_text
            .unwrap_or("Fuzzy matching: ".to_string());

        let config = &self.config;
        let alphabet = args.alphabet.unwrap_or(config.launcher_alphabet.clone());

        promise::spawn::spawn(async move {
            let args = LauncherArgs::new(
                &title,
                flags,
                mux_window_id,
                pane_id,
                domain_id_of_current_pane,
                &help_text,
                &fuzzy_help_text,
                &alphabet,
            )
            .await;

            let win = window.clone();
            win.notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                let mux = Mux::get();
                if let Some(tab) = mux.get_tab(tab_id) {
                    let window = window.clone();
                    let (overlay, future) =
                        start_overlay(term_window, &tab, move |_tab_id, term| {
                            launcher(args, term, window, initial_choice_idx)
                        });

                    term_window.assign_overlay(tab_id, overlay);
                    promise::spawn::spawn(future).detach();
                }
            })));
        })
        .detach();
    }

    /// Returns the Prompt semantic zones
    fn get_semantic_prompt_zones(&mut self, pane: &Arc<dyn Pane>) -> &[StableRowIndex] {
        let cache = self
            .semantic_zones
            .entry(pane.pane_id())
            .or_insert_with(SemanticZoneCache::default);

        let seqno = pane.get_current_seqno();
        if cache.seqno != seqno {
            let zones = pane.get_semantic_zones().unwrap_or_else(|_| vec![]);
            let mut zones: Vec<StableRowIndex> = zones
                .into_iter()
                .filter_map(|zone| {
                    if zone.semantic_type == wezterm_term::SemanticType::Prompt {
                        Some(zone.start_y)
                    } else {
                        None
                    }
                })
                .collect();
            // dedup to avoid issues where both left and right prompts are
            // defined: we only care if there were 1+ prompts on a line,
            // not about how many prompts are on a line.
            // <https://github.com/wezterm/wezterm/issues/1121>
            zones.dedup();
            cache.zones = zones;
            cache.seqno = seqno;
        }
        &cache.zones
    }

    fn scroll_to_prompt(&mut self, amount: isize, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top);
        let zone = {
            let zones = self.get_semantic_prompt_zones(&pane);
            let idx = match zones.binary_search(&position) {
                Ok(idx) | Err(idx) => idx,
            };
            let idx = ((idx as isize) + amount).max(0) as usize;
            zones.get(idx).cloned()
        };
        if let Some(zone) = zone {
            self.set_viewport(pane.pane_id(), Some(zone), dims);
        }

        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn scroll_by_page(&mut self, amount: f64, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top) as f64
            + (amount * dims.viewport_rows as f64);
        self.set_viewport(pane.pane_id(), Some(position as isize), dims);
        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn scroll_by_current_event_wheel_delta(&mut self, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        if let Some(event) = &self.current_mouse_event {
            let amount = match event.kind {
                MouseEventKind::VertWheel(amount) => -amount,
                _ => return Ok(()),
            };
            self.scroll_by_line(amount.into(), pane)?;
        }
        Ok(())
    }

    fn scroll_by_line(&mut self, amount: isize, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        // A next-core-replaced pane shows next-core's viewport, so scrolling
        // the legacy pane's would move a buffer nobody is looking at.
        if self.scroll_next_core_pane_by_line(pane.pane_id(), amount) {
            if let Some(win) = self.window.as_ref() {
                win.invalidate();
            }
            return Ok(());
        }
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top)
            .saturating_add(amount);
        self.set_viewport(pane.pane_id(), Some(position), dims);
        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn move_tab_relative(&mut self, delta: isize) -> anyhow::Result<()> {
        let mux = Mux::get();
        let window = mux
            .get_window(self.mux_window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        let max = window.len();
        ensure!(max > 0, "no more tabs");

        let active = window.get_active_idx();
        let tab = active as isize + delta;
        let tab = if tab < 0 {
            0usize
        } else if tab >= max as isize {
            max - 1
        } else {
            tab as usize
        };

        drop(window);
        self.move_tab(tab)
    }

    pub fn perform_key_assignment(
        &mut self,
        pane: &Arc<dyn Pane>,
        assignment: &KeyAssignment,
    ) -> anyhow::Result<PerformAssignmentResult> {
        use KeyAssignment::*;

        if let Some(modal) = self.get_modal() {
            if modal.perform_assignment(assignment, self) {
                return Ok(PerformAssignmentResult::Handled);
            }
        }

        match pane.perform_assignment(assignment) {
            PerformAssignmentResult::Unhandled => {}
            result => return Ok(result),
        }

        let window = self.window.as_ref().map(|w| w.clone());

        match assignment {
            ActivateKeyTable {
                name,
                timeout_milliseconds,
                replace_current,
                one_shot,
                until_unknown,
                prevent_fallback,
            } => {
                anyhow::ensure!(
                    self.input_map.has_table(name),
                    "ActivateKeyTable: no key_table named {}",
                    name
                );
                self.key_table_state.activate(KeyTableArgs {
                    name,
                    timeout_milliseconds: *timeout_milliseconds,
                    replace_current: *replace_current,
                    one_shot: *one_shot,
                    until_unknown: *until_unknown,
                    prevent_fallback: *prevent_fallback,
                });
                self.update_title();
            }
            PopKeyTable => {
                self.key_table_state.pop();
                self.update_title();
            }
            ClearKeyTableStack => {
                self.key_table_state.clear_stack();
                self.update_title();
            }
            Multiple(actions) => {
                for a in actions {
                    self.perform_key_assignment(pane, a)?;
                }
            }
            SpawnTab(spawn_where) => {
                self.spawn_tab(spawn_where);
            }
            SpawnWindow => {
                self.spawn_command(&SpawnCommand::default(), SpawnWhere::NewWindow);
            }
            SpawnCommandInNewTab(spawn) => {
                self.spawn_command(spawn, SpawnWhere::NewTab);
            }
            SpawnCommandInNewWindow(spawn) => {
                self.spawn_command(spawn, SpawnWhere::NewWindow);
            }
            SplitHorizontal(spawn) => {
                log::trace!("SplitHorizontal {:?}", spawn);
                self.spawn_command(
                    spawn,
                    SpawnWhere::SplitPane(SplitRequest {
                        direction: SplitDirection::Horizontal,
                        target_is_second: true,
                        size: MuxSplitSize::Percent(50),
                        top_level: false,
                    }),
                );
            }
            SplitVertical(spawn) => {
                log::trace!("SplitVertical {:?}", spawn);
                self.spawn_command(
                    spawn,
                    SpawnWhere::SplitPane(SplitRequest {
                        direction: SplitDirection::Vertical,
                        target_is_second: true,
                        size: MuxSplitSize::Percent(50),
                        top_level: false,
                    }),
                );
            }
            ToggleFullScreen => {
                self.window.as_ref().unwrap().toggle_fullscreen();
            }
            ToggleAlwaysOnTop => {
                let window = self.window.clone().unwrap();
                let current_level = self.window_state.as_window_level();

                match current_level {
                    WindowLevel::AlwaysOnTop => {
                        window.set_window_level(WindowLevel::Normal);
                    }
                    WindowLevel::AlwaysOnBottom | WindowLevel::Normal => {
                        window.set_window_level(WindowLevel::AlwaysOnTop);
                    }
                }
            }
            ToggleAlwaysOnBottom => {
                let window = self.window.clone().unwrap();
                let current_level = self.window_state.as_window_level();

                match current_level {
                    WindowLevel::AlwaysOnBottom => {
                        window.set_window_level(WindowLevel::Normal);
                    }
                    WindowLevel::AlwaysOnTop | WindowLevel::Normal => {
                        window.set_window_level(WindowLevel::AlwaysOnBottom);
                    }
                }
            }
            SetWindowLevel(level) => {
                let window = self.window.clone().unwrap();
                window.set_window_level(level.clone());
            }
            CopyTo(dest) => {
                let text = self.selection_text(pane);
                self.copy_to_clipboard(*dest, text);
            }
            CopyToOrSendKey(dest) => {
                // Unterm: Windows Terminal Ctrl+C behavior —
                // copy selection if any, otherwise let key pass through to PTY
                let text = self.selection_text(pane);
                if !text.is_empty() {
                    self.copy_to_clipboard(*dest, text);
                    self.clear_selection(pane);
                } else {
                    return Ok(PerformAssignmentResult::Unhandled);
                }
            }
            CopyTextTo { text, destination } => {
                self.copy_to_clipboard(*destination, text.clone());
            }
            PasteFrom(source) => {
                self.paste_from_clipboard(pane, *source);
            }
            ActivateTabRelative(n) => {
                self.activate_tab_relative(*n, true)?;
            }
            ActivateTabRelativeNoWrap(n) => {
                self.activate_tab_relative(*n, false)?;
            }
            ActivateLastTab => self.activate_last_tab()?,
            DecreaseFontSize => self.decrease_font_size(),
            IncreaseFontSize => self.increase_font_size(),
            ResetFontSize => self.reset_font_size(),
            ResetFontAndWindowSize => {
                if let Some(w) = window.as_ref() {
                    self.reset_font_and_window_size(&w)?
                }
            }
            ActivateTab(n) => {
                self.activate_tab(*n)?;
            }
            ActivateWindow(n) => {
                self.activate_window(*n)?;
            }
            ActivateWindowRelative(n) => {
                self.activate_window_relative(*n, true)?;
            }
            ActivateWindowRelativeNoWrap(n) => {
                self.activate_window_relative(*n, false)?;
            }
            SendString(s) => pane.writer().write_all(s.as_bytes())?,
            SendKey(key) => {
                use keyevent::Key;
                let mods = key.mods;
                if let Key::Code(key) = self.win_key_code_to_termwiz_key_code(
                    &key.key.resolve(self.config.key_map_preference),
                ) {
                    pane.key_down(key, mods)?;
                }
            }
            Hide => {
                if let Some(w) = window.as_ref() {
                    w.hide();
                }
            }
            Show => {
                if let Some(w) = window.as_ref() {
                    w.show();
                }
            }
            CloseCurrentTab { confirm } => self.close_current_tab(*confirm),
            CloseCurrentPane { confirm } => self.close_current_pane(*confirm),
            Nop | DisableDefaultAssignment => {}
            ReloadConfiguration => config::reload(),
            MoveTab(n) => self.move_tab(*n)?,
            MoveTabRelative(n) => self.move_tab_relative(*n)?,
            ScrollByPage(n) => self.scroll_by_page(**n, pane)?,
            ScrollByLine(n) => self.scroll_by_line(*n, pane)?,
            ScrollByCurrentEventWheelDelta => self.scroll_by_current_event_wheel_delta(pane)?,
            ScrollToPrompt(n) => self.scroll_to_prompt(*n, pane)?,
            ScrollToTop => self.scroll_to_top(pane),
            ScrollToBottom => self.scroll_to_bottom(pane),
            ShowTabNavigator => self.show_tab_navigator(),
            ShowDebugOverlay => self.show_debug_overlay(),
            ShowShellSelector => self.show_shell_selector(),
            ShowDirJump => self.show_dir_jump(),
            ShowCockpitInbox => self.show_cockpit_inbox(),
            ToggleComposer => self.toggle_composer(),
            ToggleTreeSidebar => self.toggle_tree_sidebar(),
            ToggleGitPanel => self.toggle_git_panel(),
            ToggleLeftTabBar => self.toggle_left_tab_bar(),
            ShowContextMenu => self.right_click_copy_or_paste(pane),
            ToggleSessionRecording => self.toggle_session_recording(pane.pane_id()),
            ExportSessionMarkdown => self.export_current_session(pane.pane_id()),
            CaptureScrollbackPng => mouseevent::capture_scrollback_and_announce(pane),
            OpenWebSettings => self.open_web_settings(),
            OpenAiAgentsSettings => self.open_web_settings_fragment(Some("agents")),
            OpenRecordingSettings => self.open_web_settings_fragment(Some("recording")),
            ShowLauncher => self.show_launcher(),
            ShowLauncherArgs(args) => {
                let title = args.title.clone().unwrap_or("Launcher".to_string());
                let args = LauncherActionArgs {
                    title: Some(title),
                    flags: args.flags,
                    help_text: args.help_text.clone(),
                    fuzzy_help_text: args.fuzzy_help_text.clone(),
                    alphabet: args.alphabet.clone(),
                };
                self.show_launcher_impl(args, 0);
            }
            HideApplication => {
                let con = Connection::get().expect("call on gui thread");
                con.hide_application();
            }
            QuitApplication => {
                let mux = Mux::get();
                let config = &self.config;
                log::info!("QuitApplication over here (window)");

                match config.window_close_confirmation {
                    WindowCloseConfirmation::NeverPrompt => {
                        let con = Connection::get().expect("call on gui thread");
                        con.terminate_message_loop();
                    }
                    WindowCloseConfirmation::AlwaysPrompt => {
                        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                            Some(tab) => tab,
                            None => anyhow::bail!("no active tab!?"),
                        };

                        let window = self.window.clone().unwrap();
                        let (overlay, future) = start_overlay(self, &tab, move |tab_id, term| {
                            confirm_quit_program(term, window, tab_id)
                        });
                        self.assign_overlay(tab.tab_id(), overlay);
                        promise::spawn::spawn(future).detach();
                    }
                }
            }
            SelectTextAtMouseCursor(mode) => self.select_text_at_mouse_cursor(*mode, pane),
            ExtendSelectionToMouseCursor(mode) => {
                self.extend_selection_at_mouse_cursor(*mode, pane)
            }
            ClearSelection => {
                self.clear_selection(pane);
            }
            StartWindowDrag => {
                self.window_drag_position = self.current_mouse_event.clone();
            }
            OpenLinkAtMouseCursor => {
                self.do_open_link_at_mouse_cursor(pane);
            }
            EmitEvent(name) => {
                self.emit_window_event(name, None);
            }
            CompleteSelectionOrOpenLinkAtMouseCursor(dest) => {
                let text = self.selection_text(pane);
                if !text.is_empty() {
                    self.copy_to_clipboard(*dest, text);
                    let window = self.window.as_ref().unwrap();
                    window.invalidate();
                } else {
                    self.do_open_link_at_mouse_cursor(pane);
                }
            }
            CompleteSelection(dest) => {
                let text = self.selection_text(pane);
                if !text.is_empty() {
                    self.copy_to_clipboard(*dest, text);
                    let window = self.window.as_ref().unwrap();
                    window.invalidate();
                }
            }
            ClearScrollback(erase_mode) => {
                pane.erase_scrollback(*erase_mode);
                let window = self.window.as_ref().unwrap();
                window.invalidate();
            }
            Search(pattern) => {
                log::info!("search-open: Search assignment dispatched");
                if let Some(pane) = self.get_active_pane_or_overlay() {
                    let mut replace_current = false;
                    if let Some(existing) = pane.downcast_ref::<CopyOverlay>() {
                        let mut params = existing.get_params();
                        params.editing_search = true;
                        if !pattern.is_empty() {
                            params.pattern = self.resolve_search_pattern(pattern.clone(), &pane);
                        }
                        existing.apply_params(params);
                        replace_current = true;
                    } else {
                        let search = CopyOverlay::with_pane(
                            self,
                            &pane,
                            CopyModeParams {
                                pattern: self.resolve_search_pattern(pattern.clone(), &pane),
                                editing_search: true,
                            },
                        )?;
                        self.assign_overlay_for_pane(pane.pane_id(), search);
                    }
                    log::info!("search-open: overlay assigned");
                    self.pane_state(pane.pane_id())
                        .overlay
                        .as_mut()
                        .map(|overlay| {
                            overlay.key_table_state.activate(KeyTableArgs {
                                name: "search_mode",
                                timeout_milliseconds: None,
                                replace_current,
                                one_shot: false,
                                until_unknown: false,
                                prevent_fallback: false,
                            });
                        });
                }
            }
            QuickSelect => {
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    let qa = QuickSelectOverlay::with_pane(
                        self,
                        &pane,
                        &QuickSelectArguments::default(),
                    );
                    self.assign_overlay_for_pane(pane.pane_id(), qa);
                }
            }
            QuickSelectArgs(args) => {
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    let qa = QuickSelectOverlay::with_pane(self, &pane, args);
                    self.assign_overlay_for_pane(pane.pane_id(), qa);
                }
            }
            ActivateCopyMode => {
                if let Some(pane) = self.get_active_pane_or_overlay() {
                    let mut replace_current = false;
                    if let Some(existing) = pane.downcast_ref::<CopyOverlay>() {
                        let mut params = existing.get_params();
                        params.editing_search = false;
                        existing.apply_params(params);
                        replace_current = true;
                    } else {
                        let copy = CopyOverlay::with_pane(
                            self,
                            &pane,
                            CopyModeParams {
                                pattern: MuxPattern::default(),
                                editing_search: false,
                            },
                        )?;
                        self.assign_overlay_for_pane(pane.pane_id(), copy);
                    }
                    self.pane_state(pane.pane_id())
                        .overlay
                        .as_mut()
                        .map(|overlay| {
                            overlay.key_table_state.activate(KeyTableArgs {
                                name: "copy_mode",
                                timeout_milliseconds: None,
                                replace_current,
                                one_shot: false,
                                until_unknown: false,
                                prevent_fallback: false,
                            });
                        });
                }
            }
            AdjustPaneSize(direction, amount) => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };

                let tab_id = tab.tab_id();

                if self.tab_state(tab_id).overlay.is_none() {
                    tab.adjust_pane_size(*direction, *amount);
                }
            }
            ActivatePaneByIndex(index) => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };

                let tab_id = tab.tab_id();

                if self.tab_state(tab_id).overlay.is_none() {
                    let panes = tab.iter_panes();
                    if panes.iter().position(|p| p.index == *index).is_some() {
                        tab.set_active_idx(*index);
                    }
                }
            }
            ActivatePaneDirection(direction) => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };

                let tab_id = tab.tab_id();

                if self.tab_state(tab_id).overlay.is_none() {
                    tab.activate_pane_direction(*direction);
                }
            }
            TogglePaneZoomState => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };
                tab.toggle_zoom();
            }
            SetPaneZoomState(zoomed) => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };
                tab.set_zoomed(*zoomed);
            }
            SwitchWorkspaceRelative(delta) => {
                let mux = Mux::get();
                let workspace = mux.active_workspace();
                let workspaces = mux.iter_workspaces();
                let idx = workspaces.iter().position(|w| *w == workspace).unwrap_or(0);
                let new_idx = idx as isize + delta;
                let new_idx = if new_idx < 0 {
                    workspaces.len() as isize + new_idx
                } else {
                    new_idx
                };
                let new_idx = new_idx as usize % workspaces.len();
                if let Some(w) = workspaces.get(new_idx) {
                    front_end().switch_workspace(w);
                }
            }
            SwitchToWorkspace { name, spawn } => {
                let activity = crate::Activity::new();
                let mux = Mux::get();
                let name = name
                    .as_ref()
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| mux.generate_workspace_name());
                let switcher = crate::frontend::WorkspaceSwitcher::new(&name);
                mux.set_active_workspace(&name);

                if mux.iter_windows_in_workspace(&name).is_empty() {
                    let spawn = spawn.as_ref().map(|s| s.clone()).unwrap_or_default();
                    let size = self.terminal_size;
                    let term_config = Arc::new(TermConfig::with_config(self.config.clone()));
                    let src_window_id = self.mux_window_id;

                    promise::spawn::spawn(async move {
                        if let Err(err) = crate::spawn::spawn_command_internal(
                            spawn,
                            SpawnWhere::NewWindow,
                            size,
                            Some(src_window_id),
                            term_config,
                        )
                        .await
                        {
                            log::error!("Failed to spawn: {:#}", err);
                        }
                        switcher.do_switch();
                        drop(activity);
                    })
                    .detach();
                } else {
                    switcher.do_switch();
                }
            }
            DetachDomain(domain) => {
                let domain = Mux::get().resolve_spawn_tab_domain(Some(pane.pane_id()), domain)?;
                domain.detach()?;
            }
            AttachDomain(domain) => {
                let window = self.mux_window_id;
                let domain = domain.to_string();
                let dpi = self.dimensions.dpi as u32;

                promise::spawn::spawn(async move {
                    let mux = Mux::get();
                    let domain = mux
                        .get_domain_by_name(&domain)
                        .ok_or_else(|| anyhow!("{} is not a valid domain name", domain))?;
                    domain.attach(Some(window)).await?;

                    let have_panes_in_domain = mux
                        .iter_panes()
                        .iter()
                        .any(|p| p.domain_id() == domain.domain_id());

                    if !have_panes_in_domain {
                        let config = config::configuration();
                        let _tab = domain
                            .spawn(
                                config.initial_size(
                                    dpi,
                                    Some(crate::cell_pixel_dims(&config, dpi as f64)?),
                                ),
                                None,
                                None,
                                window,
                            )
                            .await?;
                    }

                    Result::<(), anyhow::Error>::Ok(())
                })
                .detach();
            }
            CopyMode(_) => {
                // NOP here; handled by the overlay directly
            }
            RotatePanes(direction) => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };
                match direction {
                    RotationDirection::Clockwise => tab.rotate_clockwise(),
                    RotationDirection::CounterClockwise => tab.rotate_counter_clockwise(),
                }
            }
            SplitPane(split) => {
                log::trace!("SplitPane {:?}", split);
                self.spawn_command(
                    &split.command,
                    SpawnWhere::SplitPane(SplitRequest {
                        direction: match split.direction {
                            PaneDirection::Down | PaneDirection::Up => SplitDirection::Vertical,
                            PaneDirection::Left | PaneDirection::Right => {
                                SplitDirection::Horizontal
                            }
                            PaneDirection::Next | PaneDirection::Prev => {
                                log::error!(
                                    "Invalid direction {:?} for SplitPane",
                                    split.direction
                                );
                                return Ok(PerformAssignmentResult::Handled);
                            }
                        },
                        target_is_second: match split.direction {
                            PaneDirection::Down | PaneDirection::Right => true,
                            PaneDirection::Up | PaneDirection::Left => false,
                            PaneDirection::Next | PaneDirection::Prev => unreachable!(),
                        },
                        size: match split.size {
                            SplitSize::Percent(n) => MuxSplitSize::Percent(n),
                            SplitSize::Cells(n) => MuxSplitSize::Cells(n),
                        },
                        top_level: split.top_level,
                    }),
                );
            }
            PaneSelect(args) => {
                let modal = crate::termwindow::paneselect::PaneSelector::new(self, args);
                self.set_modal(Rc::new(modal));
            }
            CharSelect(args) => {
                let modal = crate::termwindow::charselect::CharSelector::new(self, args);
                self.set_modal(Rc::new(modal));
            }
            ResetTerminal => {
                pane.perform_actions(vec![termwiz::escape::Action::Esc(
                    termwiz::escape::Esc::Code(termwiz::escape::EscCode::FullReset),
                )]);
            }
            OpenUri(link) => {
                wezterm_open_url::open_url(link);
            }
            ActivateCommandPalette => {
                let modal = crate::termwindow::palette::CommandPalette::new(self);
                self.set_modal(Rc::new(modal));
            }
            PromptInputLine(args) => self.show_prompt_input_line(args),
            InputSelector(args) => self.show_input_selector(args),
            Confirmation(args) => self.show_confirmation(args),
            AcceptSuggestion { run_immediately } => {
                // Suggest bindings are conditional: if no suggestion
                // is pending on the active pane, fall through to the
                // pane's default handling so Tab still triggers shell
                // completion, Esc still goes to vim, etc.
                let pane_id = pane.pane_id() as u64;
                let suggestions = crate::mcp::handler::pending_suggestions_for_pane(pane_id);
                let Some(first) = suggestions.first() else {
                    return Ok(PerformAssignmentResult::Unhandled);
                };
                match crate::mcp::handler::accept_suggestion(&first.id, *run_immediately) {
                    Ok(mut text) => {
                        if *run_immediately {
                            text.push('\n');
                        }
                        if let Err(e) = pane.writer().write_all(text.as_bytes()) {
                            log::error!("accept_suggestion write_all failed: {e:#}");
                        }
                    }
                    Err(e) => {
                        log::warn!("accept_suggestion failed: {e}");
                    }
                }
            }
            DismissSuggestion => {
                // Esc has two jobs on a single key when MCP UI is up:
                // 1) Block a pending confirmation banner (more urgent
                //    — a worker thread is parked on it).
                // 2) Dismiss the oldest pending suggestion.
                // Try (1) first. If neither is pending, fall through
                // so Esc reaches vim / less / etc.
                if let Some(view) = crate::mcp::handler::pending_confirmation_view() {
                    crate::mcp::handler::resolve_confirmation(
                        view.id,
                        crate::mcp::handler::ConfirmationDecision::Block,
                    );
                } else {
                    let pane_id = pane.pane_id() as u64;
                    let suggestions = crate::mcp::handler::pending_suggestions_for_pane(pane_id);
                    let Some(first) = suggestions.first() else {
                        return Ok(PerformAssignmentResult::Unhandled);
                    };
                    if let Err(e) = crate::mcp::handler::dismiss_suggestion(&first.id) {
                        log::warn!("dismiss_suggestion failed: {e}");
                    }
                }
            }
            McpConfirmAllow => {
                let Some(view) = crate::mcp::handler::pending_confirmation_view() else {
                    return Ok(PerformAssignmentResult::Unhandled);
                };
                crate::mcp::handler::resolve_confirmation(
                    view.id,
                    crate::mcp::handler::ConfirmationDecision::Allow,
                );
            }
            McpConfirmAlwaysAllow => {
                let Some(view) = crate::mcp::handler::pending_confirmation_view() else {
                    return Ok(PerformAssignmentResult::Unhandled);
                };
                crate::mcp::handler::resolve_confirmation(
                    view.id,
                    crate::mcp::handler::ConfirmationDecision::AlwaysAllow,
                );
            }
            AcceptGhostText => {
                let pane_id = pane.pane_id() as u64;
                if !crate::ghost_text::has_pending_ghost(pane_id) {
                    return Ok(PerformAssignmentResult::Unhandled);
                }
                let Some(continuation) = crate::ghost_text::accept(pane_id) else {
                    return Ok(PerformAssignmentResult::Unhandled);
                };
                if let Err(e) = pane.writer().write_all(continuation.as_bytes()) {
                    log::error!("AcceptGhostText write_all failed: {e:#}");
                } else {
                    crate::cockpit::on_user_input(pane_id);
                    self.maybe_scroll_to_bottom_for_input(pane);
                }
            }
            ShowInsights => self.show_insights_overlay(),
        };
        Ok(PerformAssignmentResult::Handled)
    }

    fn do_open_link_at_mouse_cursor(&self, pane: &Arc<dyn Pane>) {
        // They clicked on a link, so let's open it!
        // We need to ensure that we spawn the `open` call outside of the context
        // of our window loop; on Windows it can cause a panic due to
        // triggering our WndProc recursively.
        // We get that assurance for free as part of the async dispatch that we
        // perform below; here we allow the user to define an `open-uri` event
        // handler that can bypass the normal `open_url` functionality.
        if let Some(link) = self.current_highlight.as_ref().cloned() {
            let window = GuiWin::new(self);
            let pane = MuxPane(pane.pane_id());

            async fn open_uri(
                lua: Option<Rc<mlua::Lua>>,
                window: GuiWin,
                pane: MuxPane,
                link: String,
            ) -> anyhow::Result<()> {
                let default_click = match lua {
                    Some(lua) => {
                        let args = lua.pack_multi((window, pane, link.clone()))?;
                        config::lua::emit_event(&lua, ("open-uri".to_string(), args))
                            .await
                            .map_err(|e| {
                                log::error!("while processing open-uri event: {:#}", e);
                                e
                            })?
                    }
                    None => true,
                };
                if default_click {
                    log::info!("clicking {}", link);
                    wezterm_open_url::open_url(&link);
                }
                Ok(())
            }

            promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
                open_uri(lua, window, pane, link.uri().to_string())
            }))
            .detach();
        }
    }
    fn close_current_pane(&mut self, confirm: bool) {
        let mux_window_id = self.mux_window_id;
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(mux_window_id) {
            Some(tab) => tab,
            None => return,
        };
        let pane = match tab.get_active_pane() {
            Some(p) => p,
            None => return,
        };

        let pane_id = pane.pane_id();
        if confirm && !pane.can_close_without_prompting(CloseReason::Pane) {
            let window = self.window.clone().unwrap();
            let (overlay, future) = start_overlay_pane(self, &pane, move |pane_id, term| {
                confirm_close_pane(pane_id, term, mux_window_id, window)
            });
            self.assign_overlay_for_pane(pane_id, overlay);
            promise::spawn::spawn(future).detach();
        } else {
            self.remove_next_core_render_consumer(pane_id);
            mux.remove_pane(pane_id);
        }
    }

    /// Close the pane identified by `pane_id` (the pane behind the `×` button
    /// on a split). Skips the confirmation overlay — splits are cheap and the
    /// user explicitly clicked the kill button, so we trust them. This matches
    /// `WindowCloseConfirmation::NeverPrompt` from the global default.
    pub fn close_pane_by_id(&mut self, pane_id: mux::pane::PaneId) {
        self.remove_next_core_render_consumer(pane_id);
        Mux::get().remove_pane(pane_id);
    }

    fn close_specific_tab(&mut self, tab_idx: usize, confirm: bool) {
        let mux = Mux::get();
        let mux_window_id = self.mux_window_id;
        let mux_window = match mux.get_window(mux_window_id) {
            Some(w) => w,
            None => return,
        };

        let tab = match mux_window.get_by_idx(tab_idx) {
            Some(tab) => Arc::clone(tab),
            None => return,
        };
        drop(mux_window);

        let tab_id = tab.tab_id();
        if confirm && !tab.can_close_without_prompting(CloseReason::Tab) {
            if self.activate_tab(tab_idx as isize).is_err() {
                return;
            }

            let window = self.window.clone().unwrap();
            let (overlay, future) = start_overlay(self, &tab, move |tab_id, term| {
                confirm_close_tab(tab_id, term, mux_window_id, window)
            });
            self.assign_overlay(tab_id, overlay);
            promise::spawn::spawn(future).detach();
        } else {
            mux.remove_tab(tab_id);
        }
    }

    fn close_current_tab(&mut self, confirm: bool) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };
        let tab_id = tab.tab_id();
        let mux_window_id = self.mux_window_id;
        if confirm && !tab.can_close_without_prompting(CloseReason::Tab) {
            let window = self.window.clone().unwrap();
            let (overlay, future) = start_overlay(self, &tab, move |tab_id, term| {
                confirm_close_tab(tab_id, term, mux_window_id, window)
            });
            self.assign_overlay(tab_id, overlay);
            promise::spawn::spawn(future).detach();
        } else {
            mux.remove_tab(tab_id);
        }
    }

    pub fn pane_state(&self, pane_id: PaneId) -> RefMut<'_, PaneState> {
        RefMut::map(self.pane_state.borrow_mut(), |state| {
            state.entry(pane_id).or_insert_with(PaneState::default)
        })
    }

    fn remove_next_core_render_consumer(&self, pane_id: PaneId) {
        // Render consumers and glyph atlases are keyed by next-core session
        // id, so resolve before unbinding — after `destroy_next_core_pane_binding`
        // the mapping is gone and the entries would leak.
        if let Ok(session_id) = self.next_core_pane_session(pane_id) {
            self.next_core_render_consumers
                .borrow_mut()
                .remove_pane(session_id);
            if let Some(webgpu) = &self.webgpu {
                webgpu.remove_next_core_glyph_atlas_pane(session_id);
            }
        }
        self.destroy_next_core_pane_binding(pane_id);
    }

    /// Drop this pane's next-core session, if it has one. Called from every
    /// pane-close path so a closed pane never leaves its PTY running.
    fn destroy_next_core_pane_binding(&self, pane_id: PaneId) {
        // A session the pane itself owns is destroyed by the pane on drop;
        // destroying it here too would kill a live session out from under a
        // pane the mux has not finished releasing.
        let owns = self
            .next_core_pane_bindings
            .borrow()
            .owns_session(pane_id as usize);
        let session_id = self
            .next_core_pane_bindings
            .borrow_mut()
            .unbind_pane(pane_id as usize);
        if !owns {
            return;
        }
        if let Some(session_id) = session_id {
            if let Err(err) = crate::engine::next_core().destroy_session(session_id) {
                log::debug!(
                    "next-core session {session_id} for pane {pane_id} \
                     was already gone at unbind: {err:#}"
                );
            }
        }
    }

    /// Drop every next-core session this window owns. Used by the paths that
    /// discard all pane state at once, where per-pane close never runs.
    ///
    /// Sessions owned by their pane are left alone; the pane destroys them.
    fn destroy_all_next_core_pane_bindings(&self) {
        let bindings: Vec<(usize, usize)> = {
            let borrowed = self.next_core_pane_bindings.borrow();
            borrowed
                .bindings()
                .into_iter()
                .filter(|(pane_id, _)| borrowed.owns_session(*pane_id))
                .collect()
        };
        self.next_core_pane_bindings.borrow_mut().clear();
        for (pane_id, session_id) in bindings {
            if let Err(err) = crate::engine::next_core().destroy_session(session_id) {
                log::debug!(
                    "next-core session {session_id} for pane {pane_id} \
                     was already gone at teardown: {err:#}"
                );
            }
        }
    }

    /// Resolve `pane_id` to its next-core session, creating and binding one on
    /// first use. The session is sized to the pane and inherits the same proxy
    /// env the spawn path uses, so a next-core-backed pane starts the same way
    /// an MCP-created session does.
    fn ensure_next_core_pane_binding(
        &self,
        pane_id: PaneId,
        cols: usize,
        rows: usize,
        cwd: Option<String>,
    ) -> anyhow::Result<usize> {
        if let Some(session_id) = self
            .next_core_pane_bindings
            .borrow()
            .session_for(pane_id as usize)
        {
            return Ok(session_id);
        }

        let request = next_core_pane_session_request(cols, rows, cwd);
        let (request_cols, request_rows) = (request.cols, request.rows);
        let session = crate::engine::next_core()
            .create_session(request)
            .with_context(|| format!("creating next-core session for pane {pane_id}"))?;
        let replaced = self.next_core_pane_bindings.borrow_mut().bind(
            pane_id as usize,
            session.id,
            request_cols,
            request_rows,
        );
        if let Some(replaced) = replaced {
            // Defensive: `session_for` said the pane was unbound, so this can
            // only happen if a concurrent bind raced us. Don't leak its PTY.
            let _ = crate::engine::next_core().destroy_session(replaced);
        }
        Ok(session.id)
    }

    /// Ensure `pane` has a next-core session, sizing it from the pane's own
    /// viewport geometry and starting it in the pane's current directory.
    fn ensure_next_core_pane_binding_for(
        &self,
        pane: &Arc<dyn mux::pane::Pane>,
    ) -> anyhow::Result<usize> {
        let dims = pane.get_dimensions();
        // A pane that already *is* a next-core session must bind to that
        // session, not get a second one spawned behind it.
        if let Some(native) = pane.downcast_ref::<crate::engine::next_core_pane::NextCorePane>() {
            let session_id = native.session_id();
            if self
                .next_core_pane_bindings
                .borrow()
                .session_for(pane.pane_id() as usize)
                != Some(session_id)
            {
                self.next_core_pane_bindings.borrow_mut().bind_borrowed(
                    pane.pane_id() as usize,
                    session_id,
                    dims.cols,
                    dims.viewport_rows,
                );
            }
            // Resize still flows through the registry so the size tracking
            // stays in one place, but the pane owns the session's lifetime.
            self.resize_next_core_pane_binding(pane.pane_id(), dims.cols, dims.viewport_rows);
            return Ok(session_id);
        }

        let cwd = pane_cwd_path(pane).map(|path| path.display().to_string());
        let session_id =
            self.ensure_next_core_pane_binding(pane.pane_id(), dims.cols, dims.viewport_rows, cwd)?;
        self.resize_next_core_pane_binding(pane.pane_id(), dims.cols, dims.viewport_rows);
        Ok(session_id)
    }

    fn next_core_pane_session(&self, pane_id: PaneId) -> anyhow::Result<usize> {
        Ok(self
            .next_core_pane_bindings
            .borrow()
            .resolve_session(pane_id as usize)?)
    }

    /// Whether next-core owns `pane_id`'s keyboard input.
    ///
    /// True only when the pane is bound to a next-core session *and* next-core
    /// is replacing the pane on screen. Input must follow the pixels: routing
    /// keystrokes to a session the user cannot see makes the visible pane look
    /// dead.
    pub fn next_core_owns_pane_input(&self, pane_id: PaneId) -> bool {
        crate::termwindow::render::draw::next_core_webgpu_pane_mode().owns_pane_input()
            && self.next_core_pane_session(pane_id).is_ok()
    }

    /// Route a key press to the pane's next-core session.
    ///
    /// Returns `true` when next-core owns the pane, whether or not the key
    /// produced bytes — a modifier keypress legitimately encodes to nothing,
    /// and falling through would type it into the hidden legacy pane instead.
    pub fn send_next_core_key(
        &self,
        pane_id: PaneId,
        key: ::termwiz::input::KeyCode,
        modifiers: ::termwiz::input::Modifiers,
    ) -> bool {
        if !self.next_core_owns_pane_input(pane_id) {
            return false;
        }
        if let Some(encoded) =
            unterm_engine::next_core::key_encoding::encode_key(key, modifiers)
        {
            self.write_next_core_pane_input(pane_id, &encoded);
        }
        true
    }

    /// Scroll the next-core session backing `pane_id` by `amount` rows.
    ///
    /// Returns `false` when next-core does not own the pane, so the caller
    /// scrolls the legacy pane instead.
    fn scroll_next_core_pane_by_line(&self, pane_id: PaneId, amount: isize) -> bool {
        if !self.next_core_owns_pane_input(pane_id) {
            return false;
        }
        let Ok(session_id) = self.next_core_pane_session(pane_id) else {
            return false;
        };
        if let Err(err) = crate::engine::next_core().scroll_viewport_by(session_id, amount) {
            log::debug!(
                "next-core scroll for pane {pane_id} (session {session_id}) failed: {err:#}"
            );
        }
        true
    }

    /// Offer a mouse event to the pane's next-core session.
    ///
    /// Returns `true` when next-core owns the pane, whether or not the event
    /// was reported: an application with mouse tracking off legitimately wants
    /// nothing, and falling through would send the event to the hidden legacy
    /// pane instead.
    pub fn send_next_core_mouse(&self, pane_id: PaneId, event: &wezterm_term::MouseEvent) -> bool {
        if !self.next_core_owns_pane_input(pane_id) {
            return false;
        }
        let Ok(session_id) = self.next_core_pane_session(pane_id) else {
            return false;
        };
        let Some(event) = next_core_mouse_event(event) else {
            return true;
        };
        if let Err(err) = crate::engine::next_core().report_mouse(session_id, event) {
            log::warn!(
                "next-core mouse report for pane {pane_id} (session {session_id}) failed: {err:#}"
            );
        }
        true
    }

    /// Route already-encoded bytes (win32-input-mode, kitty, IME-composed
    /// text) to the pane's next-core session.
    pub fn send_next_core_encoded_input(&self, pane_id: PaneId, encoded: &str) -> bool {
        if !self.next_core_owns_pane_input(pane_id) {
            return false;
        }
        self.write_next_core_pane_input(pane_id, encoded);
        true
    }

    /// Send `input` to the next-core session backing `pane_id`.
    ///
    /// Returns `false` when the pane has no next-core session, which is the
    /// signal for the caller to fall back to the legacy WezTerm pane writer.
    fn write_next_core_pane_input(&self, pane_id: PaneId, input: &str) -> bool {
        let Ok(session_id) = self.next_core_pane_session(pane_id) else {
            return false;
        };
        match crate::engine::next_core().write_input(session_id, input) {
            Ok(()) => true,
            Err(err) => {
                log::warn!(
                    "next-core input write for pane {pane_id} (session {session_id}) failed: {err:#}"
                );
                false
            }
        }
    }

    /// Paste `text` into the next-core session backing `pane_id`.
    ///
    /// Returns `false` when the pane has no next-core session, which is the
    /// signal for the caller to fall back to the legacy WezTerm pane writer.
    pub fn paste_next_core_pane_input(&self, pane_id: PaneId, text: &str) -> bool {
        let Ok(session_id) = self.next_core_pane_session(pane_id) else {
            return false;
        };
        match crate::engine::next_core().paste_input(session_id, text) {
            Ok(()) => true,
            Err(err) => {
                log::warn!(
                    "next-core paste for pane {pane_id} (session {session_id}) failed: {err:#}"
                );
                false
            }
        }
    }

    /// Keep the next-core session backing `pane_id` sized to the pane.
    ///
    /// A next-core-backed pane that misses a resize keeps wrapping at the old
    /// width. This is driven from the render path rather than from
    /// `apply_dimensions` so it covers every source of geometry change —
    /// window resize, split drag, font size change, zoom — without each one
    /// having to remember to call it. `sync_size` returns `None` when nothing
    /// moved, so a steady-state frame costs one comparison.
    fn resize_next_core_pane_binding(&self, pane_id: PaneId, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let session_id = self
            .next_core_pane_bindings
            .borrow_mut()
            .sync_size(pane_id as usize, cols, rows);
        let Some(session_id) = session_id else {
            return;
        };
        if let Err(err) = crate::engine::next_core().resize_session(session_id, cols, rows) {
            log::debug!(
                "next-core resize for pane {pane_id} (session {session_id}) \
                 to {cols}x{rows} failed: {err:#}"
            );
        }
    }

    #[allow(dead_code)]
    fn prepare_next_core_render_buffer_plan(
        &self,
        pane_id: PaneId,
    ) -> anyhow::Result<EngineRenderBufferBatch> {
        let engine = crate::engine::current();
        let metrics = RenderCellMetrics {
            cell_width_px: self.render_metrics.cell_size.width as usize,
            cell_height_px: self.render_metrics.cell_size.height as usize,
        };
        // Read by next-core session id, not by pane id: the two id spaces
        // overlap, and indexing the engine by pane id paints whichever session
        // happens to share the number.
        let session_id = self.next_core_pane_session(pane_id)?;
        self.next_core_render_consumers
            .borrow_mut()
            .read_buffer_plan(&engine, session_id, metrics)
    }

    /// Pane ids belonging to overlays (copy mode, launcher, debug output).
    ///
    /// `get_panes_to_render` substitutes an overlay's pane in place of the
    /// real one, so a caller that wants only real terminal panes cannot just
    /// ask whether the pane it was handed *has* an overlay — by then it *is*
    /// the overlay. next-core must skip these: binding one would spawn a shell
    /// and paint it over the overlay the user is looking at.
    pub fn overlay_pane_ids(&self) -> std::collections::HashSet<PaneId> {
        let mut ids: std::collections::HashSet<PaneId> = self
            .pane_state
            .borrow()
            .values()
            .filter_map(|state| state.overlay.as_ref().map(|overlay| overlay.pane.pane_id()))
            .collect();
        ids.extend(
            self.tab_state
                .borrow()
                .values()
                .filter_map(|state| state.overlay.as_ref().map(|overlay| overlay.pane.pane_id())),
        );
        ids
    }

    /// Where `pos` lands in the window, as a next-core render placement.
    ///
    /// next-core builds each frame with the pane's own top-left at the origin,
    /// so a split has to shift it to the pane's corner of the shared render
    /// target. Without this every pane would draw over the top-left one.
    pub fn next_core_pane_viewport(
        &self,
        pos: &mux::tab::PositionedPane,
    ) -> EngineRenderViewportPlacement {
        let (origin_x_px, origin_y_px) = self.pane_origin_pixels(pos);
        EngineRenderViewportPlacement::at(
            origin_x_px,
            origin_y_px,
            self.dimensions.pixel_width.max(1) as f32,
            self.dimensions.pixel_height.max(1) as f32,
        )
    }

    #[allow(dead_code)]
    pub fn prepare_next_core_webgpu_pane_frame(
        &self,
        pane_id: PaneId,
        replace_requested: bool,
        viewport: EngineRenderViewportPlacement,
    ) -> anyhow::Result<NextCoreWebGpuPaneDrawFrame> {
        let webgpu = self
            .webgpu
            .as_ref()
            .ok_or_else(|| anyhow!("next-core WebGPU pane frame requested without WebGPU state"))?;
        let batch = self.prepare_next_core_render_buffer_plan(pane_id)?;
        let font = match self.fonts.default_font() {
            Ok(font) => Some(font),
            Err(err) => {
                log::debug!("next-core WebGPU font raster source skipped: {err:#}");
                None
            }
        };

        Ok(webgpu.prepare_next_core_pane_frame(batch, font, replace_requested, viewport))
    }

    pub fn tab_state(&self, tab_id: TabId) -> RefMut<'_, TabState> {
        RefMut::map(self.tab_state.borrow_mut(), |state| {
            state.entry(tab_id).or_insert_with(TabState::default)
        })
    }

    /// Resize overlays to match their corresponding tab/pane dimensions
    pub fn resize_overlays(&self) {
        let mux = Mux::get();
        for (_, state) in self.tab_state.borrow().iter() {
            if let Some(overlay) = state.overlay.as_ref().map(|o| &o.pane) {
                overlay.resize(self.terminal_size).ok();
            }
        }
        for (pane_id, state) in self.pane_state.borrow().iter() {
            if let Some(overlay) = state.overlay.as_ref().map(|o| &o.pane) {
                if let Some(pane) = mux.get_pane(*pane_id) {
                    let dims = pane.get_dimensions();
                    overlay
                        .resize(TerminalSize {
                            cols: dims.cols,
                            rows: dims.viewport_rows,
                            dpi: self.terminal_size.dpi,
                            pixel_height: (self.terminal_size.pixel_height
                                / self.terminal_size.rows)
                                * dims.viewport_rows,
                            pixel_width: (self.terminal_size.pixel_width / self.terminal_size.cols)
                                * dims.cols,
                        })
                        .ok();
                }
            }
        }
    }

    pub fn get_viewport(&self, pane_id: PaneId) -> Option<StableRowIndex> {
        self.pane_state(pane_id).viewport
    }

    pub fn set_viewport(
        &mut self,
        pane_id: PaneId,
        position: Option<StableRowIndex>,
        dims: RenderableDimensions,
    ) {
        let pos = match position {
            Some(pos) => {
                // Drop out of scrolling mode if we're off the bottom
                if pos >= dims.physical_top {
                    None
                } else {
                    Some(pos.max(dims.scrollback_top))
                }
            }
            None => None,
        };

        let mut state = self.pane_state(pane_id);
        if pos != state.viewport {
            state.viewport = pos;

            // This is a bit gross.  If we add other overlays that need this information,
            // this should get extracted out into a trait
            if let Some(overlay) = state.overlay.as_ref() {
                if let Some(copy) = overlay.pane.downcast_ref::<CopyOverlay>() {
                    copy.viewport_changed(pos);
                } else if let Some(qs) = overlay.pane.downcast_ref::<QuickSelectOverlay>() {
                    qs.viewport_changed(pos);
                }
            }
        }
        self.window.as_ref().unwrap().invalidate();
    }

    fn maybe_scroll_to_bottom_for_input(&mut self, pane: &Arc<dyn Pane>) {
        if self.config.scroll_to_bottom_on_input {
            self.scroll_to_bottom(pane);
        }
    }

    fn scroll_to_top(&mut self, pane: &Arc<dyn Pane>) {
        let dims = pane.get_dimensions();
        self.set_viewport(pane.pane_id(), Some(dims.scrollback_top), dims);
    }

    fn scroll_to_bottom(&mut self, pane: &Arc<dyn Pane>) {
        self.pane_state(pane.pane_id()).viewport = None;
    }

    pub fn get_active_pane_no_overlay(&self) -> Option<Arc<dyn Pane>> {
        let mux = Mux::get();
        mux.get_active_tab_for_window(self.mux_window_id)
            .and_then(|tab| tab.get_active_pane())
    }

    /// Returns a Pane that we can interact with; this will typically be
    /// the active tab for the window, but if the window has a tab-wide
    /// overlay (such as the launcher / tab navigator),
    /// then that will be returned instead.  Otherwise, if the pane has
    /// an active overlay (such as search or copy mode) then that will
    /// be returned.
    pub fn get_active_pane_or_overlay(&self) -> Option<Arc<dyn Pane>> {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return None,
        };

        let tab_id = tab.tab_id();

        if let Some(tab_overlay) = self
            .tab_state(tab_id)
            .overlay
            .as_ref()
            .map(|overlay| overlay.pane.clone())
        {
            Some(tab_overlay)
        } else {
            let pane = tab.get_active_pane()?;
            let pane_id = pane.pane_id();
            self.pane_state(pane_id)
                .overlay
                .as_ref()
                .map(|overlay| overlay.pane.clone())
                .or_else(|| Some(pane))
        }
    }

    pub fn get_pane_by_id(&self, pane_id: PaneId) -> Option<Arc<dyn Pane>> {
        let mux = Mux::get();
        let tab = mux.get_active_tab_for_window(self.mux_window_id)?;
        let tab_id = tab.tab_id();

        if let Some(tab_overlay) = self
            .tab_state(tab_id)
            .overlay
            .as_ref()
            .map(|overlay| overlay.pane.clone())
        {
            if tab_overlay.pane_id() == pane_id {
                return Some(tab_overlay);
            }
        }

        for pos in tab.iter_panes() {
            if pos.pane.pane_id() == pane_id {
                if let Some(overlay) = self.pane_state(pane_id).overlay.as_ref() {
                    return Some(overlay.pane.clone());
                }
                return Some(pos.pane);
            }
        }

        None
    }

    fn get_splits(&mut self) -> Vec<PositionedSplit> {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return vec![],
        };

        let tab_id = tab.tab_id();

        if self.tab_state(tab_id).overlay.is_some() {
            vec![]
        } else {
            tab.iter_splits()
        }
    }

    fn pos_pane_to_pane_info(pos: &PositionedPane) -> PaneInformation {
        PaneInformation {
            pane_id: pos.pane.pane_id(),
            pane_index: pos.index,
            is_active: pos.is_active,
            is_zoomed: pos.is_zoomed,
            has_unseen_output: pos.pane.has_unseen_output(),
            left: pos.left,
            top: pos.top,
            width: pos.width,
            height: pos.height,
            pixel_width: pos.pixel_width,
            pixel_height: pos.pixel_height,
            title: pos.pane.get_title(),
            user_vars: pos.pane.copy_user_vars(),
            progress: pos.pane.get_progress(),
        }
    }

    fn get_tab_information(&mut self) -> Vec<TabInformation> {
        let mux = Mux::get();
        let window = match mux.get_window(self.mux_window_id) {
            Some(window) => window,
            _ => return vec![],
        };
        let tab_index = window.get_active_idx();

        window
            .iter()
            .enumerate()
            .map(|(idx, tab)| {
                let active_pane_no_overlay = tab.get_active_pane();

                TabInformation {
                    tab_index: idx,
                    tab_id: tab.tab_id(),
                    is_active: tab_index == idx,
                    is_last_active: window
                        .get_last_active_idx()
                        .map(|last_active| last_active == idx)
                        .unwrap_or(false),
                    window_id: self.mux_window_id,
                    tab_title: tab.get_title(),
                    active_pane: active_pane_no_overlay.as_ref().map(|pane| {
                        let size = pane.get_dimensions();
                        PaneInformation {
                            pane_id: pane.pane_id(),
                            pane_index: 0,
                            is_active: true,
                            is_zoomed: false,
                            has_unseen_output: false,
                            left: 0,
                            top: 0,
                            width: size.cols,
                            height: size.viewport_rows,
                            pixel_width: size.pixel_width,
                            pixel_height: size.pixel_height,
                            title: pane.get_title(),
                            user_vars: pane.copy_user_vars(),
                            progress: pane.get_progress(),
                        }
                    }),
                }
            })
            .collect()
    }

    fn get_pane_information(&self) -> Vec<PaneInformation> {
        self.get_panes_to_render()
            .iter()
            .map(Self::pos_pane_to_pane_info)
            .collect()
    }

    fn get_pos_panes_for_tab(&self, tab: &Arc<Tab>) -> Vec<PositionedPane> {
        let tab_id = tab.tab_id();

        if let Some(pane) = self
            .tab_state(tab_id)
            .overlay
            .as_ref()
            .map(|overlay| overlay.pane.clone())
        {
            let size = tab.get_size();
            vec![PositionedPane {
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 0,
                top: 0,
                width: size.cols as _,
                height: size.rows as _,
                pixel_width: size.cols as usize * self.render_metrics.cell_size.width as usize,
                pixel_height: size.rows as usize * self.render_metrics.cell_size.height as usize,
                pane,
            }]
        } else {
            let mut panes = tab.iter_panes();
            for p in &mut panes {
                if let Some(overlay) = self.pane_state(p.pane.pane_id()).overlay.as_ref() {
                    p.pane = Arc::clone(&overlay.pane);
                }
            }
            panes
        }
    }

    fn get_panes_to_render(&self) -> Vec<PositionedPane> {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return vec![],
        };

        self.get_pos_panes_for_tab(&tab)
    }

    /// if pane_id.is_none(), removes any overlay for the specified tab.
    /// Otherwise: if the overlay is the specified pane for that tab, remove it.
    fn cancel_overlay_for_tab(&mut self, tab_id: TabId, pane_id: Option<PaneId>) {
        if pane_id.is_some() {
            let current = self
                .tab_state(tab_id)
                .overlay
                .as_ref()
                .map(|o| o.pane.pane_id());
            if current != pane_id {
                return;
            }
        }
        if let Some(overlay) = self.tab_state(tab_id).overlay.take() {
            Mux::get().remove_pane(overlay.pane.pane_id());
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn schedule_cancel_overlay(window: Window, tab_id: TabId, pane_id: Option<PaneId>) {
        window.notify(TermWindowNotif::CancelOverlayForTab { tab_id, pane_id });
    }

    fn cancel_overlay_for_pane(&mut self, pane_id: PaneId) {
        if let Some(overlay) = self.pane_state(pane_id).overlay.take() {
            // Ungh, when I built the CopyOverlay, its pane doesn't get
            // added to the mux and instead it reports the overlaid
            // pane id.  Take care to avoid killing ourselves off
            // when closing the CopyOverlay
            if pane_id != overlay.pane.pane_id() {
                Mux::get().remove_pane(overlay.pane.pane_id());
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn schedule_cancel_overlay_for_pane(window: Window, pane_id: PaneId) {
        window.notify(TermWindowNotif::CancelOverlayForPane(pane_id));
    }

    pub fn assign_overlay_for_pane(&mut self, pane_id: PaneId, pane: Arc<dyn Pane>) {
        self.cancel_overlay_for_pane(pane_id);
        self.pane_state(pane_id).overlay.replace(OverlayState {
            pane,
            key_table_state: KeyTableState::default(),
        });
        self.update_title();
        // Paint the overlay NOW. Without this, the overlay (search bar,
        // quick select, …) only appears on the next incidental repaint —
        // cursor blink, pane output — which reads as the UI being slow.
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn assign_overlay(&mut self, tab_id: TabId, overlay: Arc<dyn Pane>) {
        self.cancel_overlay_for_tab(tab_id, None);
        self.tab_state(tab_id).overlay.replace(OverlayState {
            pane: overlay,
            key_table_state: KeyTableState::default(),
        });
        self.update_title();
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    fn resolve_search_pattern(&self, pattern: Pattern, pane: &Arc<dyn Pane>) -> MuxPattern {
        match pattern {
            Pattern::CaseSensitiveString(s) => MuxPattern::CaseSensitiveString(s),
            Pattern::CaseInSensitiveString(s) => MuxPattern::CaseInSensitiveString(s),
            Pattern::Regex(s) => MuxPattern::Regex(s),
            Pattern::CurrentSelectionOrEmptyString => {
                let text = self.selection_text(pane);
                let first_line = text
                    .lines()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                // Ignore-case is the friendlier default for interactive
                // search (toolbar button / Ctrl+Shift+F); Ctrl+R in the
                // search bar still cycles to exact / regex matching.
                MuxPattern::CaseInSensitiveString(first_line)
            }
        }
    }
}

impl Drop for TermWindow {
    fn drop(&mut self) {
        self.clear_all_overlays();
        if let Some(window) = self.window.take() {
            if let Some(fe) = try_front_end() {
                fe.forget_known_window(&window);
            }
        }
    }
}

#[cfg(test)]
mod interaction_contract_tests {
    use super::{right_click_action, RightClickAction};

    #[test]
    fn right_click_copies_only_when_a_selection_exists() {
        assert_eq!(right_click_action(true), RightClickAction::CopySelection);
        assert_eq!(right_click_action(false), RightClickAction::PasteClipboard);
    }
}
