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
use crate::termwindow::render::corners::*;
use crate::termwindow::{UIItem, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::ui_tokens;
use config::{Dimension, DimensionContext};
use mux::Mux;
use wezterm_term::color::ColorAttribute;
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
}

/// One row's snapshot, captured from the mux ahead of element building
/// so we don't hold borrows across rendering. Pulled straight from the
/// mux (not the top tab bar) so the sidebar stays populated even when a
/// single tab would hide the top strip.
struct RowInfo {
    tab_idx: usize,
    active: bool,
    title: String,
    /// AI agent currently driving the tab's active pane, if any.
    agent: Option<String>,
    /// Last component of the active pane's working directory.
    dir: Option<String>,
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
        self.left_tab_bar.borrow_mut().width_pts = Some(clamped);
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
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    /// Paint the bar and register its UI items. Painted into the gutter
    /// the panes have already been shifted out of, below the top bar.
    pub fn paint_left_tab_bar(&mut self) -> anyhow::Result<()> {
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
        // The sidebar is chrome, not terminal content. Its surface and
        // scrollbar must start exactly at the top bar's bottom edge; adding
        // terminal padding here creates a visible vertical gap.
        let top = top_bar_height + border.top.get() as f32;
        let status_h = if self.config.show_unterm_status_bar {
            self.status_bar_pixel_height()
        } else {
            0.
        };
        // Subtract padding_bottom as well as the status bar so the sidebar
        // ends level with the terminal content, leaving the full-width
        // status bar (and its bottom info) uncovered. Without it the panel
        // ran flush to the status bar's top edge and, with sub-pixel
        // rounding, bled over it on macOS (user: "边栏压住底部信息区域").
        let bottom = self.dimensions.pixel_height as f32
            - status_h
            - self.padding_bottom_px()
            - border.bottom.get() as f32;

        // Sidebar surface: keep it close to the terminal background.
        // A previous 10% lift toward the foreground made dark themes read
        // as a pale grey slab, especially on Windows where the sidebar can
        // consume a large fraction of the window. Use a small lift plus a
        // dark bias so the panel separates quietly without shouting.
        let bg = palette.background.to_linear();
        let fgc = palette.foreground.to_linear();
        let luma = 0.2126 * bg.0 + 0.7152 * bg.1 + 0.0722 * bg.2;
        let is_light = luma > 0.48;
        let mix = |a: LinearRgba, b: LinearRgba, t: f32| {
            LinearRgba::with_components(
                a.0 * (1. - t) + b.0 * t,
                a.1 * (1. - t) + b.1 * t,
                a.2 * (1. - t) + b.2 * t,
                1.,
            )
        };
        let bar_bg = if is_light {
            mix(bg, fgc, 0.045)
        } else {
            let lifted = mix(bg, fgc, 0.028);
            LinearRgba::with_components(lifted.0 * 0.965, lifted.1 * 0.965, lifted.2 * 0.965, 1.)
        };
        let divider = fgc.mul_alpha(if is_light { 0.18 } else { 0.10 });
        let row_pad = ui_tokens::ROW_PADDING * pt;
        let content_top_gap = 10. * pt;
        let radius = Dimension::Pixels(ui_tokens::CORNER_RADIUS * pt);
        let footer_pad_v = 10. * pt;
        let footer_pad_h = 12. * pt;
        let footer_height = (metrics.cell_size.height as f32 + 2.0 * footer_pad_v).ceil();
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
        let dim = fg.mul_alpha(if is_light { 0.76 } else { 0.72 }); // subtitle / directory
        let sel_bg = if is_light {
            mix(bg, fg, 0.20)
        } else {
            mix(bar_bg, fg, 0.155)
        };
        let hover_bg = if is_light {
            mix(bg, fg, 0.11)
        } else {
            mix(bar_bg, fg, 0.07)
        };

        // Snapshot rows straight from the mux (not the top tab bar, which
        // empties out when a lone tab hides the top strip).
        let rows: Vec<RowInfo> = {
            let mux = Mux::get();
            let window = match mux.get_window(self.mux_window_id) {
                Some(w) => w,
                None => return Ok(()),
            };
            let active_idx = window.get_active_idx();
            let collected: Vec<RowInfo> = window
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
                        Some(p) => {
                            let proc_info =
                                p.get_foreground_process_info(mux::pane::CachePolicy::AllowStale);
                            let agent = crate::mcp::handler::detect_agent_for_pane(
                                p.pane_id() as u64,
                                proc_info.as_ref(),
                            );
                            let dir = super::pane_cwd_path(p).and_then(|pp| {
                                pp.file_name().map(|n| n.to_string_lossy().to_string())
                            });
                            (agent, dir)
                        }
                        None => (None, None),
                    };
                    RowInfo {
                        tab_idx: idx,
                        active: idx == active_idx,
                        title,
                        agent,
                        dir,
                    }
                })
                .collect();
            collected
        };

        // Uniform two-line rows make the scroll window arithmetic exact.
        let row_h = metrics.cell_size.height as f32 * 2.0 + 2.0 * row_pad + 4.0 * pt;
        let content_bottom = (bottom - footer_height).max(top + content_top_gap + row_h);
        let visible_rows = ((content_bottom - top - content_top_gap) / row_h)
            .floor()
            .max(1.0) as usize;
        // Keep the active row inside the visible window. Otherwise a
        // newly-created active tab can exist below the current sidebar
        // viewport and look like it was never added.
        let active_idx = rows.iter().position(|r| r.active).unwrap_or(0);
        let scroll_top = {
            let mut bar = self.left_tab_bar.borrow_mut();
            let next = scroll_top_after_active_change(
                bar.scroll_top,
                rows.len(),
                visible_rows,
                active_idx,
                bar.last_active_idx,
            );
            bar.row_count = rows.len();
            bar.visible_rows = visible_rows;
            bar.scroll_top = next;
            bar.last_active_idx = rows.get(active_idx).map(|_| active_idx);
            next
        };

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

        for row in rows.iter().skip(scroll_top).take(visible_rows) {
            let title_fg = if row.active {
                fg
            } else {
                fg.mul_alpha(if is_light { 0.9 } else { 0.82 })
            };

            // Row-leading indicator. AI-driven panes show a saturated
            // bullet `●` in the agent's accent color — same encoding
            // as before, just stripped of the box/letter scaffolding
            // that read as "messy" in user feedback. Idle rows keep
            // the wider `→` arrow.
            let dot = if row.agent.is_some() {
                Element::new(&font, ElementContent::Text("●".to_string()))
                    .vertical_align(VerticalAlign::Middle)
                    .margin(BoxDimension {
                        left: Dimension::Pixels(0.),
                        right: Dimension::Pixels(8. * pt),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: if row.active {
                            fg.into()
                        } else {
                            agent_color.into()
                        },
                    })
            } else {
                Element::new(&font, ElementContent::Text("→".to_string()))
                    .vertical_align(VerticalAlign::Middle)
                    .margin(BoxDimension {
                        left: Dimension::Pixels(0.),
                        right: Dimension::Pixels(8. * pt),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: fg.mul_alpha(0.6).into(),
                    })
            };

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
                row.title.clone()
            };
            let primary_color = if row.active {
                fg
            } else if row.agent.is_some() {
                agent_color
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
                    top: Dimension::Pixels(5. * pt),
                    bottom: Dimension::Pixels(5. * pt),
                });

            // No inline close button — Warp's vertical-tab rows have none;
            // closing is via the right-click context menu.

            // Selected row: deeper neutral fill only. No outline; the
            // active marker and stronger fill carry selection state.
            let row_bg = if row.active {
                sel_bg
            } else {
                LinearRgba::TRANSPARENT
            };
            let row_border = BorderColor {
                left: if row.active {
                    agent_color
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
                        left: Dimension::Pixels(2. * pt),
                        right: Dimension::Pixels(0.),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
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
            // Horizontal padding insets the rows from the panel edges and
            // the divider, so the rounded selection sits inside the panel
            // instead of overflowing into the terminal.
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
            // Content box shrinks by the padding + divider so the panel
            // total still fills exactly the reserved `width` gutter.
            .min_width(Some(Dimension::Pixels(width - 14. * pt - 1.)))
            .min_height(Some(Dimension::Pixels(
                content_bottom - top - content_top_gap - 1. * pt,
            )));

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

        let mut ui_items = computed.ui_items();
        {
            let gl_state = self.render_state.as_ref().unwrap();
            self.render_element(&computed, gl_state, None)?;
        }
        self.ui_items.append(&mut ui_items);

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
        if rows.len() > visible_rows {
            let track_top = top + content_top_gap + 1. * pt;
            let track_bottom = content_bottom - 1. * pt;
            let track_h = (track_bottom - track_top).max(1.);
            let scrollbar_w = (5. * pt).round().max(6.);
            let bar_right = border.left.get() as f32 + width;
            let scrollbar_x = bar_right - scrollbar_w;
            let thumb_h = (track_h * (visible_rows as f32) / (rows.len() as f32))
                .max(28. * pt)
                .min(track_h);
            let max_top = rows.len().saturating_sub(visible_rows).max(1) as f32;
            let thumb_y = track_top + (track_h - thumb_h) * (scroll_top as f32 / max_top);

            let track = Element::new(&font, ElementContent::Text(String::new()))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(if is_light { 0.10 } else { 0.16 }).into(),
                    text: LinearRgba::TRANSPARENT.into(),
                })
                .min_width(Some(Dimension::Pixels(scrollbar_w)))
                .min_height(Some(Dimension::Pixels(track_h)));
            let track_layout = LayoutContext {
                height: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: track_h,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: scrollbar_w,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(scrollbar_x, track_top, scrollbar_w, track_h),
                metrics: &metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 21,
            };
            let track_computed = self.compute_element(&track_layout, &track)?;
            {
                let gl_state = self.render_state.as_ref().unwrap();
                self.render_element(&track_computed, gl_state, None)?;
            }

            let thumb = Element::new(&font, ElementContent::Text(String::new()))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(if is_light { 0.62 } else { 0.74 }).into(),
                    text: LinearRgba::TRANSPARENT.into(),
                })
                .min_width(Some(Dimension::Pixels(scrollbar_w)))
                .min_height(Some(Dimension::Pixels(thumb_h)));
            let thumb_layout = LayoutContext {
                height: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: thumb_h,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: scrollbar_w,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(scrollbar_x, thumb_y, scrollbar_w, thumb_h),
                metrics: &metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 22,
            };
            let thumb_computed = self.compute_element(&thumb_layout, &thumb)?;
            let gl_state = self.render_state.as_ref().unwrap();
            self.render_element(&thumb_computed, gl_state, None)?;

            let hit_w = (18. * pt).round().max(scrollbar_w) as usize;
            let hit_right = bar_right.max(border.left.get() as f32).round() as usize;
            let hit_x = hit_right.saturating_sub(hit_w);
            self.ui_items.push(UIItem {
                x: hit_x,
                width: hit_w,
                y: track_top as usize,
                height: track_h.max(1.0) as usize,
                item_type: UIItemType::LeftTabBarScrollTrack {
                    row_count: rows.len(),
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
                    row_count: rows.len(),
                    visible_rows,
                    track_top: track_top as usize,
                    track_height: track_h.max(1.0) as usize,
                },
                pane_id: None,
            });
        }

        // Author footer — pinned to the absolute bottom of the sidebar.
        // Painted in the palette teal accent (same hue as the row status
        // dots / agent names) + a ↗ external-link arrow so it unmistakably
        // reads as a clickable hyperlink — the user asked for the link
        // affordance to be obvious, so this deliberately drops the old
        // low-key dim grey. Click anywhere on the row opens
        // https://doaipm.com.
        let caption = format!("{} ↗", crate::i18n::t("sidebar.author_caption"));
        let link_color = agent_color;
        // Back to the title font (SF Pro) — JetBrains Mono's open `O`
        // didn't sit visually with the rest of the all-caps run.
        // SF Pro's caps are designed as a single optical family, so
        // even at small size the `O` shares stroke width and round-
        // ness with `D`, `B`, `M`. All-caps + the title-font
        // metrics give the uniform letter band the user asked for.
        let footer = Element::new(&font, ElementContent::Text(caption))
            .item_type(UIItemType::LeftTabBarAuthorLink)
            .display(DisplayType::Block)
            .min_width(Some(Dimension::Pixels(width - 14. * pt - 1.)))
            .padding(BoxDimension {
                left: Dimension::Pixels(footer_pad_h),
                right: Dimension::Pixels(footer_pad_h),
                top: Dimension::Pixels(footer_pad_v),
                bottom: Dimension::Pixels(footer_pad_v),
            })
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: link_color.into(),
            })
            .hover_colors(Some(ElementColors {
                border: BorderColor::default(),
                bg: hover_bg.into(),
                text: fg.into(),
            }));
        let footer_top = bottom - footer_height;
        let footer_layout = LayoutContext {
            height: DimensionContext {
                dpi: self.dimensions.dpi as f32,
                pixel_max: footer_height,
                pixel_cell: metrics.cell_size.height as f32,
            },
            width: DimensionContext {
                dpi: self.dimensions.dpi as f32,
                pixel_max: width,
                pixel_cell: metrics.cell_size.width as f32,
            },
            bounds: euclid::rect(border.left.get() as f32, footer_top, width, footer_height),
            metrics: &metrics,
            gl_state: self.render_state.as_ref().unwrap(),
            zindex: 19,
        };
        let footer_computed = self.compute_element(&footer_layout, &footer)?;
        {
            let gl_state = self.render_state.as_ref().unwrap();
            self.render_element(&footer_computed, gl_state, None)?;
        }
        let mut footer_items = footer_computed.ui_items();
        self.ui_items.append(&mut footer_items);

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
