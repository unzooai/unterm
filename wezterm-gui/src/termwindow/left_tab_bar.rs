//! Left vertical tab bar (`tab_bar_position = "Left"`).
//!
//! Tabs become rows in a sidebar docked at the window's left edge: a
//! title line (same auto-title as the top bar) over a dim subtitle of
//! `agent · directory` — which MCP agent is currently driving the pane,
//! and the last component of its working directory. The top bar stays,
//! but only carries window buttons, the active title and quick actions.
//!
//! Same plumbing as the directory-tree sidebar: the bar's width is
//! injected at every window_padding.left evaluation site (panes, splits,
//! mouse mapping and terminal cols all shift together), rendering is
//! box-model and theme-driven, and interactions route via UIItemType.
//!
//! Interactions (wired in mouseevent.rs):
//!   single-click row → activate tab; keep dragging to reorder
//!   double-click row → rename tab (inline prompt overlay)
//!   right-click row  → tab context menu
//!   ✕ on a row       → close tab (existing CloseTab routing)
//!   + row            → new tab (existing NewTabButton routing)
//!   right-edge grip  → drag to resize, clamped to [200pt, 50% window]
//!   wheel            → scroll

use crate::termwindow::box_model::*;
use crate::termwindow::chrome_colors;
use crate::termwindow::render::corners::*;
use crate::termwindow::{UIItem, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::ui_tokens;
use config::{Dimension, DimensionContext};
use mux::Mux;
use std::time::{Duration, Instant};
use wezterm_term::color::ColorAttribute;
use wezterm_term::terminal::Progress;
use window::color::LinearRgba;
use window::WindowOps;

/// Per-window state. Lives on TermWindow regardless of mode; it only
/// takes effect while `tab_bar_position = "Left"`.
#[derive(Default)]
pub struct LeftTabBar {
    /// Hidden via the View-menu toggle. Width collapses to zero and the
    /// terminal reflows; tabs are still reachable via keys/top-bar menu.
    pub hidden: bool,
    /// User-resized width in points; None → ui_tokens default.
    pub width_pts: Option<f32>,
    /// First visible row index (wheel scrolling).
    pub scroll_top: usize,
    /// Last painted tab-row count, used to clamp wheel/drag scrolling.
    pub row_count: usize,
    /// Last painted number of visible tab rows.
    pub visible_rows: usize,
    /// Last active tab index seen by paint. Active changes auto-scroll
    /// into view; ordinary repaints preserve the user's manual scroll.
    pub last_active_idx: Option<usize>,
    /// Cached layout for the main left-tab-bar container. Hover state is
    /// resolved during render, so the computed tree can be reused briefly
    /// across animation/paint ticks when the visible tab rows are unchanged.
    cached: Option<ComputedElement>,
    cached_key: Option<LeftTabBarCacheKey>,
    cached_at: Option<Instant>,
    /// Last time a live resize-drag ran the expensive PTY reflow, used to
    /// throttle it to ~25fps so dragging the divider stays smooth.
    last_reflow: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LeftTabBarCacheKey {
    pixel_width: usize,
    pixel_height: usize,
    dpi: usize,
    bar_width_bits: u32,
    top_bits: u32,
    content_bottom_bits: u32,
    active_idx: usize,
    scroll_top: usize,
    row_count: usize,
    visible_rows: usize,
    visible_rows_sig: Vec<String>,
}

// `compute_element` shapes every visible row. On windows with many restored
// tabs that first compute can take hundreds of milliseconds, so a 250ms TTL
// expires before the next paint and makes the cache effectively useless.
// The key already captures geometry, active row, scroll position and visible
// row text; keep the TTL only as a fallback for theme/color changes.
const LEFT_TAB_BAR_CACHE_TTL: Duration = Duration::from_secs(5);

/// One row's snapshot, captured from the mux ahead of element building
/// so we don't hold borrows across rendering. Pulled straight from the
/// mux (not the top tab bar) so the sidebar stays populated even when a
/// single tab would hide the top strip.
#[derive(Clone)]
struct RowInfo {
    tab_idx: usize,
    active: bool,
    title: String,
    /// AI agent currently driving the tab's active pane, if any.
    agent: Option<String>,
    /// Last component of the active pane's working directory.
    dir: Option<String>,
    /// New output since the tab was last focused — drives the unread dot.
    has_unseen: bool,
    /// OSC 9;4 progress state (running / error), when the program reports it.
    progress: Progress,
}

/// A single rendered line in the sidebar's scroll window. Tabs are grouped
/// by project (their cwd basename); when more than one project is present a
/// `GroupHeader` is emitted before each project's tabs, giving the Warp-style
/// grouped list — except the grouping here is automatic (derived from the
/// directory each pane is in) rather than something the user has to set up.
enum DisplayRow {
    GroupHeader { label: String, count: usize },
    Tab(RowInfo),
}

impl LeftTabBar {
    fn invalidate_cache(&mut self) {
        self.cached = None;
        self.cached_key = None;
        self.cached_at = None;
    }
}

fn scroll_top_for_active(
    current_scroll_top: usize,
    row_count: usize,
    visible_rows: usize,
    active_idx: usize,
) -> usize {
    let mut scroll_top = clamp_scroll_top(current_scroll_top, row_count, visible_rows);
    if row_count == 0 {
        return 0;
    }

    let visible_rows = visible_rows.max(1);
    let active_idx = active_idx.min(row_count - 1);
    let max_top = row_count.saturating_sub(visible_rows);

    if active_idx < scroll_top {
        scroll_top = active_idx;
    } else if active_idx >= scroll_top.saturating_add(visible_rows) {
        scroll_top = active_idx.saturating_add(1).saturating_sub(visible_rows);
    }

    scroll_top.min(max_top)
}

fn clamp_scroll_top(scroll_top: usize, row_count: usize, visible_rows: usize) -> usize {
    if row_count == 0 {
        return 0;
    }
    scroll_top.min(row_count.saturating_sub(visible_rows.max(1)))
}

fn scroll_top_after_active_change(
    current_scroll_top: usize,
    row_count: usize,
    visible_rows: usize,
    active_idx: usize,
    last_active_idx: Option<usize>,
) -> usize {
    if last_active_idx == Some(active_idx) {
        clamp_scroll_top(current_scroll_top, row_count, visible_rows)
    } else {
        scroll_top_for_active(current_scroll_top, row_count, visible_rows, active_idx)
    }
}

fn scroll_top_for_delta(
    current_scroll_top: usize,
    row_count: usize,
    visible_rows: usize,
    delta: isize,
) -> usize {
    let current_scroll_top = clamp_scroll_top(current_scroll_top, row_count, visible_rows);
    let max_top = row_count.saturating_sub(visible_rows.max(1));
    (current_scroll_top as isize + delta).clamp(0, max_top as isize) as usize
}

fn scroll_top_for_thumb_top(
    thumb_top: usize,
    track_top: usize,
    track_height: usize,
    thumb_height: usize,
    row_count: usize,
    visible_rows: usize,
) -> usize {
    let max_top = row_count.saturating_sub(visible_rows.max(1));
    if max_top == 0 {
        return 0;
    }

    let max_thumb_top = track_height.saturating_sub(thumb_height);
    if max_thumb_top == 0 {
        return 0;
    }

    let thumb_top = thumb_top.saturating_sub(track_top).min(max_thumb_top);
    ((thumb_top as f32 / max_thumb_top as f32) * max_top as f32).round() as usize
}

fn gutter_limited_width_pts(
    window_pts: f32,
    desired_pts: f32,
    other_width_pts: Option<f32>,
    own_min_pts: f32,
    other_min_pts: f32,
    total_max_ratio: f32,
) -> f32 {
    let Some(other_width_pts) = other_width_pts else {
        return desired_pts.max(own_min_pts);
    };
    let total_max = (window_pts * total_max_ratio).max(own_min_pts + other_min_pts);
    (total_max - other_width_pts).max(own_min_pts)
}

/// Display-friendly process name for a tab row: drop the noisy executable
/// extension (`powershell.exe` → `powershell`), the way Warp and iTerm show
/// shells. Anything without a known suffix is returned unchanged.
fn prettify_proc_title(title: &str) -> String {
    let lower = title.to_ascii_lowercase();
    for ext in [".exe", ".com", ".bat", ".cmd"] {
        if lower.ends_with(ext) {
            return title[..title.len() - ext.len()].to_string();
        }
    }
    title.to_string()
}

impl crate::TermWindow {
    /// Physical pixels the left tab bar occupies (0 when not in Left
    /// mode or hidden). Clamped to [MIN, MAX_RATIO × window width].
    pub(crate) fn left_tab_bar_pixel_width(&self) -> f32 {
        let Some(raw_width_pts) = self.left_tab_bar_raw_width_pts() else {
            return 0.0;
        };
        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let max =
            self.left_tab_bar_max_width_pts(window_pts)
                .min(self.left_gutter_limited_width_pts(
                    raw_width_pts,
                    self.tree_sidebar_raw_width_pts(),
                    ui_tokens::LEFT_TAB_BAR_MIN_WIDTH,
                    ui_tokens::TREE_SIDEBAR_MIN_WIDTH,
                ));
        (raw_width_pts.clamp(ui_tokens::LEFT_TAB_BAR_MIN_WIDTH, max) * pt).round()
    }

    pub(crate) fn left_tab_bar_raw_width_pts(&self) -> Option<f32> {
        if self.config.tab_bar_position != config::TabBarPosition::Left {
            return None;
        }
        let bar = self.left_tab_bar.borrow();
        if bar.hidden {
            return None;
        }
        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let max = self.left_tab_bar_max_width_pts(window_pts);
        Some(
            bar.width_pts
                .unwrap_or(ui_tokens::LEFT_TAB_BAR_WIDTH)
                .clamp(ui_tokens::LEFT_TAB_BAR_MIN_WIDTH, max),
        )
    }

    fn left_tab_bar_max_width_pts(&self, window_pts: f32) -> f32 {
        (window_pts * ui_tokens::LEFT_TAB_BAR_MAX_RATIO).max(ui_tokens::LEFT_TAB_BAR_MIN_WIDTH)
    }

    pub(crate) fn left_gutter_limited_width_pts(
        &self,
        desired_pts: f32,
        other_width_pts: Option<f32>,
        own_min_pts: f32,
        other_min_pts: f32,
    ) -> f32 {
        let Some(other_width_pts) = other_width_pts else {
            return desired_pts.max(own_min_pts);
        };
        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        gutter_limited_width_pts(
            window_pts,
            desired_pts,
            Some(other_width_pts),
            own_min_pts,
            other_min_pts,
            ui_tokens::LEFT_GUTTER_MAX_RATIO,
        )
    }

    /// Total left gutter: tree sidebar + left tab bar. This is the value
    /// injected at every window_padding.left evaluation site.
    pub(crate) fn left_gutter_pixel_width(&self) -> f32 {
        self.left_tab_bar_pixel_width() + self.tree_sidebar_pixel_width()
    }

    /// Apply a resize-grip drag. `x_px` is the cursor x in window
    /// physical pixels; the bar's left edge is the os border.
    pub(crate) fn resize_left_tab_bar(&mut self, x_px: f32) {
        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let x_px = if pt > 1.0 && x_px <= window_pts + 2.0 {
            x_px * pt
        } else {
            x_px
        };
        let border = self.get_os_border();
        let w_pts = (x_px - border.left.get() as f32) / pt;
        let max =
            self.left_tab_bar_max_width_pts(window_pts)
                .min(self.left_gutter_limited_width_pts(
                    w_pts,
                    self.tree_sidebar_raw_width_pts(),
                    ui_tokens::LEFT_TAB_BAR_MIN_WIDTH,
                    ui_tokens::TREE_SIDEBAR_MIN_WIDTH,
                ));
        let clamped = w_pts.clamp(ui_tokens::LEFT_TAB_BAR_MIN_WIDTH, max);
        {
            let mut bar = self.left_tab_bar.borrow_mut();
            if bar.width_pts != Some(clamped) {
                bar.width_pts = Some(clamped);
                bar.invalidate_cache();
            }
        }
        if let Some(window) = self.window.as_ref().cloned() {
            // Throttle the expensive PTY reflow to ~25fps during a live drag.
            // Reflowing every mouse-move (~125Hz) resized every pane and
            // re-gathered every tab's metadata each frame, which is what made
            // the divider drag feel laggy. The sidebar still repaints every
            // frame so it tracks the cursor; the drag-release does a final
            // reflow to land on the exact width.
            let now = Instant::now();
            let due = {
                let bar = self.left_tab_bar.borrow();
                bar.last_reflow
                    .map_or(true, |t| now.duration_since(t) >= Duration::from_millis(40))
            };
            if due {
                self.left_tab_bar.borrow_mut().last_reflow = Some(now);
                let dims = self.dimensions;
                self.apply_dimensions(&dims, None, &window);
            }
            window.invalidate();
        }
    }

    /// Settle the terminal at the exact final width after a throttled
    /// resize-drag ends (see `resize_left_tab_bar`).
    pub(crate) fn finish_left_tab_bar_resize(&mut self) {
        self.left_tab_bar.borrow_mut().last_reflow = None;
        if let Some(window) = self.window.as_ref().cloned() {
            let dims = self.dimensions;
            self.apply_dimensions(&dims, None, &window);
            window.invalidate();
        }
    }

    /// View-menu / key toggle. Mirrors toggle_tree_sidebar: flip, then
    /// reflow the terminal around the changed gutter width.
    pub(crate) fn toggle_left_tab_bar(&mut self) {
        {
            let mut bar = self.left_tab_bar.borrow_mut();
            bar.hidden = !bar.hidden;
            bar.invalidate_cache();
        }
        if let Some(window) = self.window.as_ref().cloned() {
            let dims = self.dimensions;
            self.apply_dimensions(&dims, None, &window);
            window.invalidate();
        }
    }

    pub(crate) fn left_tab_bar_scroll_by(&mut self, delta: isize) {
        let mut bar = self.left_tab_bar.borrow_mut();
        let next = scroll_top_for_delta(bar.scroll_top, bar.row_count, bar.visible_rows, delta);
        if next != bar.scroll_top {
            bar.scroll_top = next;
            bar.invalidate_cache();
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }
    }

    pub(crate) fn left_tab_bar_scroll_to_thumb_top(
        &mut self,
        thumb_top: usize,
        track_top: usize,
        track_height: usize,
        thumb_height: usize,
        row_count: usize,
        visible_rows: usize,
    ) {
        let next = scroll_top_for_thumb_top(
            thumb_top,
            track_top,
            track_height,
            thumb_height,
            row_count,
            visible_rows,
        );
        let mut bar = self.left_tab_bar.borrow_mut();
        if next != bar.scroll_top {
            bar.scroll_top = next;
            bar.invalidate_cache();
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    /// Paint the bar and register its UI items. Painted into the gutter
    /// the panes have already been shifted out of, below the top bar.
    pub fn paint_left_tab_bar(&mut self) -> anyhow::Result<()> {
        let trace_start = Instant::now();
        let mut trace_last = trace_start;
        let mut trace_steps: Vec<(&'static str, std::time::Duration)> = Vec::with_capacity(10);
        let mut trace_mark = |name: &'static str| {
            let now = Instant::now();
            trace_steps.push((name, now.saturating_duration_since(trace_last)));
            trace_last = now;
        };

        let width = self.left_tab_bar_pixel_width();
        if width <= 0.0 {
            return Ok(());
        }

        // UI font (SF Pro on macOS) for tab rows, 12pt — Warp renders its
        // vertical-tab text with the proportional UI font, not monospace.
        let font = self.fonts.title_font()?;
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let pt = self.dimensions.dpi as f32 / 72.0;
        let palette = self.palette().clone();

        let border = self.get_os_border();
        let top_bar_height = if self.show_tab_bar && !self.config.tab_bar_at_bottom {
            self.tab_bar_pixel_height().unwrap_or(0.)
        } else {
            0.
        };
        // The sidebar is chrome, not terminal content. Its surface starts at
        // the top bar's bottom edge; list content and scrollbar get their own
        // small breathing room below that chrome boundary.
        // No vertical inset: the surface runs flush from the top bar straight
        // down to the status bar (same full height as the directory-tree
        // sidebar), so there is no top/bottom gap to be uneven.
        let vgap = 0. * pt;
        let top = top_bar_height + border.top.get() as f32 + vgap;
        let status_h = if self.config.show_unterm_status_bar {
            self.status_bar_pixel_height()
        } else {
            0.
        };
        // The sidebar surface runs flush down to the top edge of the status
        // bar (no terminal-padding gap), so the panel meets the bottom info
        // bar with no seam. The status bar fills its own rect on a higher
        // layer *after* the sidebar paints, so it always stays on top — the
        // surface can reach its edge without covering the info text.
        let bottom =
            self.dimensions.pixel_height as f32 - status_h - border.bottom.get() as f32 - vgap;

        // Sidebar surface: keep it close to the terminal background.
        // A previous 10% lift toward the foreground made dark themes read
        // as a pale grey slab, especially on Windows where the sidebar can
        // consume a large fraction of the window. Use a small lift plus a
        // dark bias so the panel separates quietly without shouting.
        let bg = palette.background.to_linear();
        let fgc = palette.foreground.to_linear();
        let chrome = chrome_colors::sidebar(bg, fgc);
        let bar_bg = chrome.surface;
        let divider = chrome.divider;
        let is_light = chrome.is_light;
        let row_pad = ui_tokens::ROW_PADDING * pt;
        let content_top_gap = 12. * pt;
        let radius = Dimension::Pixels(ui_tokens::CORNER_RADIUS * pt);
        // The surface runs the full height down to the status bar (no reserved
        // gap), so the panel is one continuous fill that meets the bottom info
        // bar. A leftover gap here used to expose the window background beneath
        // the surface as a second, slightly different shade — the "two layers"
        // seam. The empty area below the "+" row is part of the surface (one
        // colour), not a gap.
        let content_bottom_gap = 0. * pt;
        let rounded = || {
            Some(Corners {
                top_left: SizedPoly {
                    width: radius,
                    height: radius,
                    poly: TOP_LEFT_ROUNDED_CORNER,
                },
                top_right: SizedPoly {
                    width: radius,
                    height: radius,
                    poly: TOP_RIGHT_ROUNDED_CORNER,
                },
                bottom_left: SizedPoly {
                    width: radius,
                    height: radius,
                    poly: BOTTOM_LEFT_ROUNDED_CORNER,
                },
                bottom_right: SizedPoly {
                    width: radius,
                    height: radius,
                    poly: BOTTOM_RIGHT_ROUNDED_CORNER,
                },
            })
        };
        trace_mark("setup");

        // Color model mirrors Warp's: rows are greyscale foreground-overlay
        // (5/10/15% opacity); color comes ONLY from the status dot and the
        // agent name, so it reads clean against any user theme. Title is
        // full-contrast foreground; subtitle was 60% but read too faint on
        // dark schemes (rows came across as a single line of title with a
        // ghost underneath) — 75% holds up as a clear second tier.
        let agent_color = palette
            .resolve_fg(ColorAttribute::PaletteIndex(14))
            .to_linear();
        let fg = palette.foreground.to_linear();
        let dim = chrome.dim_text; // subtitle / directory
        let sel_bg = chrome.selected_bg;
        // The active row owns the strong fill + accent bar; a merely-hovered
        // row gets only a whisper of fill. Keeping hover well below the
        // selected fill means a hover that lingers under the cursor after a
        // keyboard tab-switch can never be mistaken for a second selection.
        let hover_bg = chrome_colors::mix(bar_bg, fg, if is_light { 0.07 } else { 0.045 });

        // Snapshot just the tab handles (cheap Arc clones) and the active
        // index. The expensive per-tab metadata (title / agent detection /
        // cwd) is gathered further down for ONLY the visible rows, so the
        // sidebar costs O(visible rows) per paint instead of O(total tabs).
        // The previous all-tabs gather ran on every repaint and bogged the
        // main thread once a window held dozens of tabs (laggy menus etc.).
        let (tabs, active_idx) = {
            let mux = Mux::get();
            let window = match mux.get_window(self.mux_window_id) {
                Some(w) => w,
                None => return Ok(()),
            };
            (
                window.iter().cloned().collect::<Vec<_>>(),
                window.get_active_idx(),
            )
        };
        trace_mark("tabs");
        // Resolve metadata for EVERY tab (cheap now: `agent_and_cwd_for_pane`
        // is a cached, non-blocking lookup that refreshes off-thread). We need
        // every tab's project to group them, so the old "visible rows only"
        // gather no longer applies.
        let metas: Vec<RowInfo> = tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| {
                let pane = tab.get_active_pane();
                let title = {
                    let t = tab.get_title();
                    if !t.is_empty() {
                        t
                    } else {
                        pane.as_ref().map(|p| p.get_title()).unwrap_or_default()
                    }
                };
                let (agent, dir) = match &pane {
                    Some(p) => crate::mcp::handler::agent_and_cwd_for_pane(p.pane_id() as u64),
                    None => (None, None),
                };
                let has_unseen = pane.as_ref().map(|p| p.has_unseen_output()).unwrap_or(false);
                let progress = pane
                    .as_ref()
                    .map(|p| p.get_progress())
                    .unwrap_or_default();
                RowInfo {
                    tab_idx: idx,
                    active: idx == active_idx,
                    title,
                    agent,
                    dir,
                    has_unseen,
                    progress,
                }
            })
            .collect();
        trace_mark("metadata");

        // Auto-group tabs by project (cwd basename), preserving the order in
        // which each project first appears. A tab with no known cwd falls
        // under "~". Group headers are only emitted when more than one project
        // is open — a single-project window stays a clean flat list.
        let mut groups: Vec<(String, Vec<&RowInfo>)> = Vec::new();
        for meta in &metas {
            let key = meta.dir.as_deref().unwrap_or("~");
            if let Some((_, members)) = groups.iter_mut().find(|(group, _)| group == key) {
                members.push(meta);
            } else {
                groups.push((key.to_string(), vec![meta]));
            }
        }
        let group_order: Vec<String> = groups.iter().map(|(group, _)| group.clone()).collect();
        let multi_group = group_order.len() > 1;

        let mut display: Vec<DisplayRow> = vec![];
        let mut active_pos = 0usize;
        for (g, members) in groups {
            if multi_group {
                display.push(DisplayRow::GroupHeader {
                    label: g.clone(),
                    count: members.len(),
                });
            }
            for m in members {
                if m.active {
                    active_pos = display.len();
                }
                display.push(DisplayRow::Tab((*m).clone()));
            }
        }
        let row_count = display.len();
        trace_mark("group");

        // Uniform rows make the scroll window arithmetic exact (headers share
        // the tab row height). Must match the actual rendered row height
        // below, or scrolling drifts.
        let row_text_pad_v = 9.0 * pt;
        let row_h = metrics.cell_size.height as f32 + 2.0 * row_text_pad_v + 2.0 * pt;
        let content_bottom = (bottom - content_bottom_gap).max(top + content_top_gap + row_h);
        let visible_rows = ((content_bottom - top - content_top_gap) / row_h)
            .floor()
            .max(1.0) as usize;
        // Keep the active row inside the visible window. Otherwise a
        // newly-created active tab can exist below the current sidebar
        // viewport and look like it was never added.
        let scroll_top = {
            let mut bar = self.left_tab_bar.borrow_mut();
            let next = scroll_top_after_active_change(
                bar.scroll_top,
                row_count,
                visible_rows,
                active_pos,
                bar.last_active_idx,
            );
            bar.row_count = row_count;
            bar.visible_rows = visible_rows;
            bar.scroll_top = next;
            bar.last_active_idx = (active_pos < row_count).then_some(active_pos);
            next
        };

        let visible: Vec<&DisplayRow> =
            display.iter().skip(scroll_top).take(visible_rows).collect();
        trace_mark("scroll");

        let cache_key = LeftTabBarCacheKey {
            pixel_width: self.dimensions.pixel_width,
            pixel_height: self.dimensions.pixel_height,
            dpi: self.dimensions.dpi,
            bar_width_bits: width.to_bits(),
            top_bits: top.to_bits(),
            content_bottom_bits: content_bottom.to_bits(),
            active_idx,
            scroll_top,
            row_count,
            visible_rows,
            visible_rows_sig: visible
                .iter()
                .map(|row| match row {
                    DisplayRow::GroupHeader { label, count } => {
                        format!("g:{label}:{count}")
                    }
                    DisplayRow::Tab(row) => format!(
                        "t:{}:{}:{}:{}:{}:{}:{:?}",
                        row.tab_idx,
                        row.active,
                        row.title,
                        row.agent.as_deref().unwrap_or(""),
                        row.dir.as_deref().unwrap_or(""),
                        row.has_unseen,
                        row.progress
                    ),
                })
                .collect(),
        };
        let cached = {
            let bar = self.left_tab_bar.borrow();
            match (&bar.cached_key, &bar.cached, bar.cached_at) {
                (Some(key), Some(computed), Some(cached_at))
                    if key == &cache_key && cached_at.elapsed() <= LEFT_TAB_BAR_CACHE_TTL =>
                {
                    Some(computed.clone())
                }
                _ => None,
            }
        };

        if let Some(computed) = cached {
            trace_mark("cache_hit");
            let mut ui_items = computed.ui_items();
            {
                let gl_state = self.render_state.as_ref().unwrap();
                self.render_element(&computed, gl_state, None)?;
            }
            trace_mark("render_cached");
            self.ui_items.append(&mut ui_items);
        } else {
            let mut children: Vec<Element> = vec![];
            children.push(
                Element::new(&font, ElementContent::Text(String::new()))
                    .display(DisplayType::Block)
                    .min_width(Some(Dimension::Percent(1.)))
                    .min_height(Some(Dimension::Pixels(content_top_gap)))
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: LinearRgba::TRANSPARENT.into(),
                    }),
            );

            for disp in visible.iter() {
                // Project group header: a quiet uppercase caption with the tab
                // count, on the same row pitch as the tabs so the scroll window
                // arithmetic stays exact.
                let row = match disp {
                    DisplayRow::GroupHeader { label, count } => {
                        let label_disp = if label.as_str() == "~" {
                            "HOME".to_string()
                        } else {
                            label.to_uppercase()
                        };
                        // Project name on the left; the tab count is demoted to a
                        // faint right-floated number instead of an inline
                        // "LABEL   10" string that read as debug clutter.
                        let header_kids = vec![
                            Element::new(&font, ElementContent::Text(label_disp)).colors(
                                ElementColors {
                                    border: BorderColor::default(),
                                    bg: LinearRgba::TRANSPARENT.into(),
                                    text: dim.mul_alpha(0.72).into(),
                                },
                            ),
                            Element::new(&font, ElementContent::Text(count.to_string()))
                                .float(Float::Right)
                                .colors(ElementColors {
                                    border: BorderColor::default(),
                                    bg: LinearRgba::TRANSPARENT.into(),
                                    text: dim.mul_alpha(0.4).into(),
                                }),
                        ];
                        children.push(
                            Element::new(&font, ElementContent::Children(header_kids))
                                .display(DisplayType::Block)
                                .min_width(Some(Dimension::Percent(1.)))
                                .margin(BoxDimension {
                                    left: Dimension::Pixels(0.),
                                    right: Dimension::Pixels(0.),
                                    top: Dimension::Pixels(1. * pt),
                                    bottom: Dimension::Pixels(1. * pt),
                                })
                                .padding(BoxDimension {
                                    left: Dimension::Pixels(9. * pt),
                                    right: Dimension::Pixels(10. * pt),
                                    top: Dimension::Pixels(9. * pt),
                                    bottom: Dimension::Pixels(9. * pt),
                                })
                                .colors(ElementColors {
                                    border: BorderColor::default(),
                                    bg: LinearRgba::TRANSPARENT.into(),
                                    text: dim.mul_alpha(0.72).into(),
                                }),
                        );
                        continue;
                    }
                    DisplayRow::Tab(r) => r,
                };

                let title_fg = if row.active {
                    fg
                } else {
                    fg.mul_alpha(if is_light { 0.9 } else { 0.82 })
                };

                // Row-leading indicator: a real per-program glyph (Nerd Font) —
                // the PowerShell / cmd / bash terminal mark for shells, a robot
                // for AI-agent panes — the same icon language as the top tab bar
                // and as Warp's vertical tabs. The accent color on an agent icon
                // keeps AI panes recognizable at a glance; shell icons sit at a
                // calm mid tier. Selection is carried by the row's left accent
                // bar, not the icon.
                let icon_glyph =
                    crate::tabbar::detect_shell_icon(row.agent.as_deref().unwrap_or(&row.title));
                let icon_color = if row.agent.is_some() {
                    agent_color
                } else if row.active {
                    fg.mul_alpha(if is_light { 0.85 } else { 0.78 })
                } else {
                    fg.mul_alpha(if is_light { 0.62 } else { 0.55 })
                };
                let dot = Element::new(&font, ElementContent::Text(icon_glyph.to_string()))
                    .vertical_align(VerticalAlign::Middle)
                    .margin(BoxDimension {
                        left: Dimension::Pixels(0.),
                        right: Dimension::Pixels(9. * pt),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: icon_color.into(),
                    });

                // Row label: when an AI agent is bound, the agent name
                // *is* the title — pane.title is usually "Claude Code" or
                // similar and just duplicates the agent name. Drop the
                // duplicate; the bullet's accent color encodes "agent",
                // the agent name carries the wordmark, the cwd trails dim.
                // Idle rows fall back to the pane title.
                let primary_text = if let Some(agent) = &row.agent {
                    agent.clone()
                } else if row.title.is_empty() {
                    "shell".to_string()
                } else {
                    prettify_proc_title(&row.title)
                };
                // Agent rows carry the agent's accent color on the name in every
                // state (the wordmark IS the identity); plain shells use full
                // foreground when active and a dimmed tier otherwise.
                let primary_color = if row.agent.is_some() {
                    agent_color
                } else if row.active {
                    fg
                } else {
                    title_fg
                };
                let title_el =
                    Element::new(&font, ElementContent::Text(primary_text)).colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: primary_color.into(),
                    });
                let mut line_kids: Vec<Element> = vec![];
                line_kids.push(dot);
                line_kids.push(title_el);
                // When the list is grouped, the project already shows in the group
                // header, so repeating a (truncated) cwd on every row is just
                // noise — drop it. Ungrouped (single-project) windows keep the cwd
                // trailing the name so each tab still carries its location.
                if !multi_group {
                    if let Some(dir) = &row.dir {
                        line_kids.push(
                            Element::new(&font, ElementContent::Text(format!("  · {dir}"))).colors(
                                ElementColors {
                                    border: BorderColor::default(),
                                    bg: LinearRgba::TRANSPARENT.into(),
                                    text: dim.into(),
                                },
                            ),
                        );
                    }
                }

                // Right-aligned activity indicator (Otty-style): a running/error
                // state the program reported (OSC 9;4), else an unread-output dot.
                // The active row shows nothing — you're already looking at it.
                let indicator: Option<LinearRgba> = if row.active {
                    None
                } else {
                    match row.progress {
                        Progress::Error(_) => {
                            Some(palette.resolve_fg(ColorAttribute::PaletteIndex(9)).to_linear())
                        }
                        Progress::Percentage(_) | Progress::Indeterminate => {
                            Some(palette.resolve_fg(ColorAttribute::PaletteIndex(10)).to_linear())
                        }
                        Progress::None if row.has_unseen => {
                            Some(palette.resolve_fg(ColorAttribute::PaletteIndex(12)).to_linear())
                        }
                        Progress::None => None,
                    }
                };
                if let Some(color) = indicator {
                    line_kids.push(
                        Element::new(&font, ElementContent::Text("\u{25cf}".to_string()))
                            .float(Float::Right)
                            .vertical_align(VerticalAlign::Middle)
                            .colors(ElementColors {
                                border: BorderColor::default(),
                                bg: LinearRgba::TRANSPARENT.into(),
                                text: color.into(),
                            }),
                    );
                }
                // Insets live on the content (not the row) because row padding
                // would create a second visible grey layer around the selected
                // fill. Keep the row as one flat block and align text by giving
                // every row the same transparent/active left border width.
                let title_line = Element::new(&font, ElementContent::Children(line_kids))
                    .display(DisplayType::Block)
                    .min_width(Some(Dimension::Percent(1.)))
                    .padding(BoxDimension {
                        left: Dimension::Pixels(7. * pt),
                        right: Dimension::Pixels(8. * pt),
                        // Taller rows (was 5pt) so the sidebar breathes instead of
                        // reading cramped. Mirrored in `row_text_pad_v` above so
                        // the scroll-window arithmetic stays exact.
                        top: Dimension::Pixels(9. * pt),
                        bottom: Dimension::Pixels(9. * pt),
                    });

                // No inline close button — Warp's vertical-tab rows have none;
                // closing is via the right-click context menu.

                // Active row carries TWO distinct cues: a stronger neutral fill
                // AND a saturated left accent bar (the agent's color for AI
                // panes, a neutral foreground tint for plain shells). Hover, by
                // contrast, is only a faint fill with no bar — so an active row
                // never reads the same as a merely-hovered one. This is what
                // keeps a lingering hover from looking like a second selection.
                // Accent: agent panes carry the agent's color (bright cyan);
                // plain shells carry the theme's bright-blue (ANSI 12). Both are
                // real hues from the user's palette — a grey cursor-white or a
                // dimmed foreground is exactly what made the active tab read as
                // a lifeless grey slab. Blue stays distinct from the agent cyan.
                let accent = if row.agent.is_some() {
                    agent_color
                } else {
                    palette
                        .resolve_fg(ColorAttribute::PaletteIndex(12))
                        .to_linear()
                };
                let row_bg = if row.active {
                    // Tie the active fill to the accent so the selection reads
                    // as an intentional colored panel item, not a grey block.
                    chrome_colors::mix(sel_bg, accent, 0.12)
                } else {
                    LinearRgba::TRANSPARENT
                };
                let row_border = BorderColor {
                    left: if row.active {
                        accent
                    } else {
                        LinearRgba::TRANSPARENT
                    },
                    right: LinearRgba::TRANSPARENT,
                    top: LinearRgba::TRANSPARENT,
                    bottom: LinearRgba::TRANSPARENT,
                };
                children.push(
                    Element::new(&font, ElementContent::Children(vec![title_line]))
                        .item_type(UIItemType::LeftTabBarTab(row.tab_idx))
                        .display(DisplayType::Block)
                        .min_width(Some(Dimension::Percent(1.)))
                        .margin(BoxDimension {
                            left: Dimension::Pixels(0.),
                            right: Dimension::Pixels(0.),
                            top: Dimension::Pixels(1. * pt),
                            bottom: Dimension::Pixels(1. * pt),
                        })
                        .padding(BoxDimension {
                            left: Dimension::Pixels(0.),
                            right: Dimension::Pixels(0.),
                            top: Dimension::Pixels(0.),
                            bottom: Dimension::Pixels(0.),
                        })
                        .border(BoxDimension {
                            left: Dimension::Pixels(3. * pt),
                            right: Dimension::Pixels(0.),
                            top: Dimension::Pixels(0.),
                            bottom: Dimension::Pixels(0.),
                        })
                        // Otty-style: the active row reads as a clean rounded
                        // selection rather than a sharp full-bleed rectangle.
                        .border_corners(if row.active { rounded() } else { None })
                        .colors(ElementColors {
                            border: row_border,
                            bg: row_bg.into(),
                            text: title_fg.into(),
                        })
                        .hover_colors(if row.active {
                            None
                        } else {
                            Some(ElementColors {
                                border: BorderColor::new(LinearRgba::TRANSPARENT),
                                bg: hover_bg.into(),
                                text: fg.into(),
                            })
                        }),
                );
            }

            // Trailing "+   ▾" row → the "+" spawns a default-shell tab, the
            // chevron opens the shell selector. Two elements share the row
            // so the picker is visually discoverable instead of buried behind
            // right-click muscle memory.
            // Bigger hit targets — single-glyph cells with the original
            // tight padding gave ~16-20px click areas, well under the
            // 32 px tap-target ergonomics target. Doubled vertical
            // padding + min_width so each half is a comfortable button.
            let btn_pad_v = row_pad * 1.4;
            let btn_pad_h = row_pad * 1.8;
            let plus_cell = Element::new(&font, ElementContent::Text("+".to_string()))
                .item_type(UIItemType::TabBar(crate::tabbar::TabBarItem::NewTabButton))
                .vertical_align(VerticalAlign::Middle)
                .min_width(Some(Dimension::Pixels(40. * pt)))
                .padding(BoxDimension {
                    left: Dimension::Pixels(btn_pad_h),
                    right: Dimension::Pixels(btn_pad_h / 2.),
                    top: Dimension::Pixels(btn_pad_v),
                    bottom: Dimension::Pixels(btn_pad_v),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                })
                .hover_colors(Some(ElementColors {
                    border: BorderColor::default(),
                    bg: hover_bg.into(),
                    text: fg.into(),
                }));
            let chevron_cell = Element::new(&font, ElementContent::Text("▾".to_string()))
                .item_type(UIItemType::NewTabShellSelector)
                .vertical_align(VerticalAlign::Middle)
                .min_width(Some(Dimension::Pixels(40. * pt)))
                .padding(BoxDimension {
                    left: Dimension::Pixels(btn_pad_h / 2.),
                    right: Dimension::Pixels(btn_pad_h),
                    top: Dimension::Pixels(btn_pad_v),
                    bottom: Dimension::Pixels(btn_pad_v),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                })
                .hover_colors(Some(ElementColors {
                    border: BorderColor::default(),
                    bg: hover_bg.into(),
                    text: fg.into(),
                }));
            children.push(
                Element::new(
                    &font,
                    ElementContent::Children(vec![plus_cell, chevron_cell]),
                )
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .margin(BoxDimension {
                    left: Dimension::Pixels(0.),
                    right: Dimension::Pixels(0.),
                    top: Dimension::Pixels(2. * pt),
                    bottom: Dimension::Pixels(2. * pt),
                })
                .border(BoxDimension::new(Dimension::Pixels(1.)))
                .border_corners(rounded())
                .colors(ElementColors {
                    border: BorderColor::new(LinearRgba::TRANSPARENT),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                }),
            );

            let container = Element::new(&font, ElementContent::Children(children))
                .item_type(UIItemType::LeftTabBarBg)
                // Horizontal padding insets the rows from the panel edges so the
                // selection fill doesn't bleed to the divider.
                .padding(BoxDimension {
                    left: Dimension::Pixels(7. * pt),
                    right: Dimension::Pixels(7. * pt),
                    top: Dimension::Pixels(0.),
                    bottom: Dimension::Pixels(0.),
                })
                .border(BoxDimension {
                    left: Dimension::Pixels(0.),
                    right: Dimension::Pixels(1.),
                    top: Dimension::Pixels(0.),
                    bottom: Dimension::Pixels(0.),
                })
                .colors(ElementColors {
                    border: BorderColor {
                        left: LinearRgba::TRANSPARENT,
                        right: divider,
                        top: LinearRgba::TRANSPARENT,
                        bottom: LinearRgba::TRANSPARENT,
                    },
                    bg: bar_bg.into(),
                    text: fg.into(),
                })
                // Full-height panel: the surface fills from the top bar down to the
                // bottom status bar, so the sidebar reads as one continuous left
                // panel that meets the info bar — not a short floating card with a
                // dead band beneath it. The reserved `width` gutter is unchanged
                // (the terminal still reflows around it).
                .min_width(Some(Dimension::Pixels(width - 14. * pt - 1.)))
                // Fill the exact span top → content_bottom (no padding/border on
                // top or bottom), so the window-background gap above the surface
                // (`vgap` below the top bar) equals the gap below it (`vgap` above
                // the status bar). content_top_gap is an internal spacer child, so
                // it must NOT be subtracted here or the surface stops short and the
                // bottom gap grows.
                .min_height(Some(Dimension::Pixels(content_bottom - top)));

            let layout = LayoutContext {
                height: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: self.dimensions.pixel_height as f32,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: self.dimensions.pixel_width as f32,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(border.left.get() as f32, top, width, content_bottom - top),
                metrics: &metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 18,
            };
            let computed = self.compute_element(&layout, &container)?;
            trace_mark("compute");

            let mut ui_items = computed.ui_items();
            trace_mark("ui_items");
            {
                let mut bar = self.left_tab_bar.borrow_mut();
                bar.cached_key = Some(cache_key);
                bar.cached_at = Some(Instant::now());
                bar.cached = Some(computed.clone());
            }
            {
                let gl_state = self.render_state.as_ref().unwrap();
                self.render_element(&computed, gl_state, None)?;
            }
            trace_mark("render");
            self.ui_items.append(&mut ui_items);
        }

        // Resize grip: a thin strip on the bar's right edge. It is
        // registered before the scrollbar hit items so that, when a
        // scrollbar is visible, dragging the edge-most thumb still scrolls
        // instead of being intercepted as a sidebar resize.
        let grip_w = (ui_tokens::LEFT_TAB_BAR_GRIP * pt).round() as usize;
        let bar_right = (border.left.get() as f32 + width) as usize;
        self.ui_items.push(UIItem {
            x: bar_right.saturating_sub(grip_w),
            width: grip_w,
            y: top as usize,
            height: (bottom - top).max(0.) as usize,
            item_type: UIItemType::LeftTabBarResize,
            pane_id: None,
        });

        // Scrollbar thumb on the right edge of the sidebar — visible only
        // when there are more rows than fit. Until this landed the user
        // had no way to tell that a newly-spawned tab existed at all when
        // the sidebar was already full ("最多那多个 tab"): with the thumb,
        // the auto-scroll's reposition is immediately legible.
        if row_count > visible_rows {
            let track_top = top + content_top_gap;
            let track_bottom = content_bottom - 1. * pt;
            let track_h = (track_bottom - track_top).max(1.);
            let scrollbar_w = (ui_tokens::CHROME_SCROLLBAR_WIDTH * pt)
                .round()
                .max(ui_tokens::CHROME_SCROLLBAR_MIN_WIDTH);
            let bar_right = border.left.get() as f32 + width;
            let scrollbar_x = bar_right - scrollbar_w;
            let thumb_h = (track_h * (visible_rows as f32) / (row_count as f32))
                .max(ui_tokens::CHROME_SCROLLBAR_MIN_THUMB_HEIGHT * pt)
                .min(track_h);
            let max_top = row_count.saturating_sub(visible_rows).max(1) as f32;
            let thumb_y = track_top + (track_h - thumb_h) * (scroll_top as f32 / max_top);

            let thumb_color = palette
                .scrollbar_thumb
                .to_linear()
                .mul_alpha(ui_tokens::CHROME_SCROLLBAR_THUMB_ALPHA);
            let track_color = palette
                .scrollbar_thumb
                .to_linear()
                .mul_alpha(ui_tokens::CHROME_SCROLLBAR_TRACK_ALPHA);

            {
                let gl_state = self.render_state.as_ref().unwrap();
                let track_layer = gl_state.layer_for_zindex(23)?;
                let mut layers = track_layer.quad_allocator();
                self.filled_rectangle(
                    &mut layers,
                    0,
                    euclid::rect(scrollbar_x, track_top, scrollbar_w, track_h),
                    track_color,
                )?;
            }
            {
                let gl_state = self.render_state.as_ref().unwrap();
                let thumb_layer = gl_state.layer_for_zindex(24)?;
                let mut layers = thumb_layer.quad_allocator();
                self.filled_rectangle(
                    &mut layers,
                    0,
                    euclid::rect(scrollbar_x, thumb_y, scrollbar_w, thumb_h),
                    thumb_color,
                )?;
            }

            let hit_w = (20. * pt).round().max(scrollbar_w) as usize;
            let hit_right = bar_right.max(border.left.get() as f32).round() as usize;
            let hit_x = hit_right.saturating_sub(hit_w);
            self.ui_items.push(UIItem {
                x: hit_x,
                width: hit_w,
                y: track_top as usize,
                height: track_h.max(1.0) as usize,
                item_type: UIItemType::LeftTabBarScrollTrack {
                    row_count: row_count,
                    visible_rows,
                    thumb_height: thumb_h.max(1.0) as usize,
                },
                pane_id: None,
            });
            self.ui_items.push(UIItem {
                x: hit_x,
                width: hit_w,
                y: thumb_y as usize,
                height: thumb_h.max(1.0) as usize,
                item_type: UIItemType::LeftTabBarScrollThumb {
                    row_count: row_count,
                    visible_rows,
                    track_top: track_top as usize,
                    track_height: track_h.max(1.0) as usize,
                },
                pane_id: None,
            });
        }
        trace_mark("scrollbar");

        let total = trace_start.elapsed();
        if total.as_millis() >= 40 {
            let slowest = trace_steps
                .iter()
                .max_by_key(|(_, duration)| *duration)
                .map(|(name, duration)| format!("{name}:{duration:?}"))
                .unwrap_or_else(|| "none:0ns".to_string());
            let steps = trace_steps
                .iter()
                .map(|(name, duration)| format!("{name}={duration:?}"))
                .collect::<Vec<_>>()
                .join(" ");
            log::info!(
                "left-tab-bar-slow total={total:?} slowest={slowest} tabs={} rows={} visible_rows={} {steps}",
                tabs.len(),
                row_count,
                visible_rows,
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        gutter_limited_width_pts, scroll_top_after_active_change, scroll_top_for_active,
        scroll_top_for_delta, scroll_top_for_thumb_top,
    };

    #[test]
    fn scrolls_down_to_keep_active_row_visible() {
        assert_eq!(scroll_top_for_active(0, 10, 4, 9), 6);
    }

    #[test]
    fn scrolls_up_to_keep_active_row_visible() {
        assert_eq!(scroll_top_for_active(6, 10, 4, 2), 2);
    }

    #[test]
    fn keeps_scroll_top_when_active_row_is_already_visible() {
        assert_eq!(scroll_top_for_active(3, 10, 4, 5), 3);
    }

    #[test]
    fn clamps_stale_scroll_after_rows_shrink() {
        assert_eq!(scroll_top_for_active(99, 3, 10, 1), 0);
    }

    #[test]
    fn handles_empty_rows() {
        assert_eq!(scroll_top_for_active(5, 0, 4, 0), 0);
    }

    #[test]
    fn preserves_manual_scroll_when_active_tab_is_unchanged() {
        assert_eq!(scroll_top_after_active_change(0, 20, 5, 19, Some(19)), 0);
    }

    #[test]
    fn auto_scrolls_only_when_active_tab_changes() {
        assert_eq!(scroll_top_after_active_change(0, 20, 5, 19, Some(18)), 15);
    }

    #[test]
    fn wheel_scroll_clamps_to_visible_window() {
        assert_eq!(scroll_top_for_delta(0, 20, 5, 3), 3);
        assert_eq!(scroll_top_for_delta(14, 20, 5, 3), 15);
        assert_eq!(scroll_top_for_delta(2, 20, 5, -5), 0);
    }

    #[test]
    fn thumb_top_maps_to_scroll_top() {
        assert_eq!(scroll_top_for_thumb_top(0, 0, 100, 20, 20, 5), 0);
        assert_eq!(scroll_top_for_thumb_top(80, 0, 100, 20, 20, 5), 15);
        assert_eq!(scroll_top_for_thumb_top(40, 0, 100, 20, 20, 5), 8);
    }

    #[test]
    fn gutter_limit_caps_combined_sidebars() {
        assert_eq!(
            gutter_limited_width_pts(1000.0, 300.0, Some(260.0), 112.0, 112.0, 0.42),
            160.0
        );
    }

    #[test]
    fn gutter_limit_keeps_minimums_on_narrow_windows() {
        assert_eq!(
            gutter_limited_width_pts(480.0, 260.0, Some(180.0), 112.0, 112.0, 0.42),
            112.0
        );
    }

    #[test]
    fn gutter_limit_does_not_cap_single_sidebar() {
        assert_eq!(
            gutter_limited_width_pts(1000.0, 260.0, None, 112.0, 112.0, 0.42),
            260.0
        );
    }
}
