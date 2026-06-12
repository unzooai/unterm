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
use wezterm_term::Line;
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
}

/// One row's snapshot, captured from mux + tab bar state ahead of
/// element building so we don't hold borrows across rendering.
struct RowInfo {
    tab_idx: usize,
    active: bool,
    title: Line,
    subtitle: String,
}

impl crate::TermWindow {
    /// Physical pixels the left tab bar occupies (0 when not in Left
    /// mode or hidden). Clamped to [MIN, MAX_RATIO × window width].
    pub(crate) fn left_tab_bar_pixel_width(&self) -> f32 {
        if self.config.tab_bar_position != config::TabBarPosition::Left {
            return 0.0;
        }
        let bar = self.left_tab_bar.borrow();
        if bar.hidden {
            return 0.0;
        }
        let pt = self.dimensions.dpi as f32 / 72.0;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let max = (window_pts * ui_tokens::LEFT_TAB_BAR_MAX_RATIO)
            .max(ui_tokens::LEFT_TAB_BAR_MIN_WIDTH);
        let w = bar
            .width_pts
            .unwrap_or(ui_tokens::LEFT_TAB_BAR_WIDTH)
            .clamp(ui_tokens::LEFT_TAB_BAR_MIN_WIDTH, max);
        (w * pt).round()
    }

    /// Total left gutter: tree sidebar + left tab bar. This is the value
    /// injected at every window_padding.left evaluation site.
    pub(crate) fn left_gutter_pixel_width(&self) -> f32 {
        self.left_tab_bar_pixel_width() + self.tree_sidebar_pixel_width()
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

    /// Apply a resize-grip drag. `x_px` is the cursor x in window
    /// physical pixels; the bar's left edge is the os border.
    pub(crate) fn resize_left_tab_bar(&mut self, x_px: f32) {
        let pt = self.dimensions.dpi as f32 / 72.0;
        let border = self.get_os_border();
        let w_pts = (x_px - border.left.get() as f32) / pt;
        let window_pts = self.dimensions.pixel_width as f32 / pt;
        let max = (window_pts * ui_tokens::LEFT_TAB_BAR_MAX_RATIO)
            .max(ui_tokens::LEFT_TAB_BAR_MIN_WIDTH);
        let clamped = w_pts.clamp(ui_tokens::LEFT_TAB_BAR_MIN_WIDTH, max);
        self.left_tab_bar.borrow_mut().width_pts = Some(clamped);
        if let Some(window) = self.window.as_ref().cloned() {
            let dims = self.dimensions;
            self.apply_dimensions(&dims, None, &window);
            window.invalidate();
        }
    }

    pub(crate) fn left_tab_bar_scroll_by(&mut self, delta: isize) {
        let row_count = self.tab_bar.items().iter().filter(|e| {
            matches!(e.item, crate::tabbar::TabBarItem::Tab { .. })
        }).count();
        let mut bar = self.left_tab_bar.borrow_mut();
        let max_top = row_count.saturating_sub(1);
        bar.scroll_top =
            (bar.scroll_top as isize + delta).clamp(0, max_top as isize) as usize;
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    /// `agent · dir` subtitle for a tab: the MCP agent that most
    /// recently drove the tab's active pane, and the last component of
    /// that pane's cwd. Either half may be absent.
    fn left_tab_bar_subtitle(&self, tab_idx: usize) -> String {
        let mux = Mux::get();
        let Some(window) = mux.get_window(self.mux_window_id) else {
            return String::new();
        };
        let Some(tab) = window.get_by_idx(tab_idx) else {
            return String::new();
        };
        let Some(pane) = tab.get_active_pane() else {
            return String::new();
        };
        let agent = crate::mcp::handler::agent_for_pane(pane.pane_id() as u64);
        let dir = super::pane_cwd_path(&pane).and_then(|p| {
            p.file_name().map(|n| n.to_string_lossy().to_string())
        });
        match (agent, dir) {
            (Some(a), Some(d)) => format!("{a} · {d}"),
            (None, Some(d)) => d,
            (Some(a), None) => a,
            (None, None) => String::new(),
        }
    }

    /// Paint the bar and register its UI items. Painted into the gutter
    /// the panes have already been shifted out of, below the top bar.
    pub fn paint_left_tab_bar(&mut self) -> anyhow::Result<()> {
        let width = self.left_tab_bar_pixel_width();
        if width <= 0.0 {
            return Ok(());
        }

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
        let top = top_bar_height + border.top.get() as f32;
        let status_h = if self.config.show_unterm_status_bar {
            self.status_bar_pixel_height()
        } else {
            0.
        };
        let bottom =
            self.dimensions.pixel_height as f32 - status_h - border.bottom.get() as f32;

        // Theme-driven colors: the bar shares the titlebar surface so the
        // chrome reads as one piece; rows use the tab palette.
        let colors = self
            .config
            .colors
            .as_ref()
            .and_then(|c| c.tab_bar.as_ref())
            .cloned()
            .unwrap_or_else(config::TabBarColors::default);
        let bar_bg = if self.focused.is_some() {
            self.config.window_frame.active_titlebar_bg
        } else {
            self.config.window_frame.inactive_titlebar_bg
        }
        .to_linear();
        let active_tab = colors.active_tab();
        let inactive_tab = colors.inactive_tab();
        let hover_tab = colors.inactive_tab_hover();
        let edge = colors.inactive_tab_edge().to_linear();

        let row_pad = ui_tokens::ROW_PADDING * pt;
        let radius = Dimension::Pixels(ui_tokens::CORNER_RADIUS * pt);
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

        // Snapshot rows before any element borrows.
        let rows: Vec<RowInfo> = self
            .tab_bar
            .items()
            .iter()
            .filter_map(|entry| match entry.item {
                crate::tabbar::TabBarItem::Tab { tab_idx, active } => Some(RowInfo {
                    tab_idx,
                    active,
                    title: entry.title.clone(),
                    subtitle: self.left_tab_bar_subtitle(tab_idx),
                }),
                _ => None,
            })
            .collect();

        let scroll_top = self.left_tab_bar.borrow().scroll_top.min(
            rows.len().saturating_sub(1),
        );
        // Uniform two-line rows make the scroll window arithmetic exact.
        let row_h = metrics.cell_size.height as f32 * 2.0 + 2.0 * row_pad + 4.0 * pt;
        let visible_rows = ((bottom - top - row_h) / row_h).floor().max(1.0) as usize;

        let mut children: Vec<Element> = vec![];

        for row in rows.iter().skip(scroll_top).take(visible_rows) {
            let title_fg = if row.active {
                active_tab.fg_color.to_linear()
            } else {
                inactive_tab.fg_color.to_linear()
            };
            let mut kids = vec![Element::with_line(&font, &row.title, &palette)
                .display(DisplayType::Block)
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: title_fg.into(),
                })];
            let subtitle = if row.subtitle.is_empty() {
                " ".to_string()
            } else {
                row.subtitle.clone()
            };
            kids.push(
                Element::new(&font, ElementContent::Text(subtitle))
                    .display(DisplayType::Block)
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: title_fg.mul_alpha(0.6).into(),
                    }),
            );
            if self.config.show_close_tab_button_in_tabs {
                kids.push(
                    crate::termwindow::render::fancy_tab_bar::make_x_button(
                        &font,
                        &metrics,
                        &colors,
                        row.tab_idx,
                        row.active,
                    ),
                );
            }

            let (row_bg, row_border) = if row.active {
                (active_tab.bg_color.to_linear(), edge)
            } else {
                (LinearRgba::TRANSPARENT, LinearRgba::TRANSPARENT)
            };
            children.push(
                Element::new(&font, ElementContent::Children(kids))
                    .item_type(UIItemType::LeftTabBarTab(row.tab_idx))
                    .display(DisplayType::Block)
                    .min_width(Some(Dimension::Percent(1.)))
                    .margin(BoxDimension {
                        left: Dimension::Pixels(6. * pt),
                        right: Dimension::Pixels(6. * pt),
                        top: Dimension::Pixels(2. * pt),
                        bottom: Dimension::Pixels(2. * pt),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Pixels(row_pad),
                        right: Dimension::Pixels(row_pad),
                        top: Dimension::Pixels(row_pad / 2.),
                        bottom: Dimension::Pixels(row_pad / 2.),
                    })
                    .border(BoxDimension::new(Dimension::Pixels(1.)))
                    .border_corners(rounded())
                    .colors(ElementColors {
                        border: BorderColor::new(row_border),
                        bg: row_bg.into(),
                        text: title_fg.into(),
                    })
                    .hover_colors(if row.active {
                        None
                    } else {
                        Some(ElementColors {
                            border: BorderColor::new(
                                hover_tab.bg_color.to_linear(),
                            ),
                            bg: hover_tab.bg_color.to_linear().into(),
                            text: hover_tab.fg_color.to_linear().into(),
                        })
                    }),
            );
        }

        // Trailing "+" row → existing NewTabButton routing spawns a tab.
        let new_tab = colors.new_tab();
        let new_tab_hover = colors.new_tab_hover();
        children.push(
            Element::new(&font, ElementContent::Text("+".to_string()))
                .item_type(UIItemType::TabBar(crate::tabbar::TabBarItem::NewTabButton))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .margin(BoxDimension {
                    left: Dimension::Pixels(6. * pt),
                    right: Dimension::Pixels(6. * pt),
                    top: Dimension::Pixels(2. * pt),
                    bottom: Dimension::Pixels(2. * pt),
                })
                .padding(BoxDimension {
                    left: Dimension::Pixels(row_pad),
                    right: Dimension::Pixels(row_pad),
                    top: Dimension::Pixels(row_pad / 2.),
                    bottom: Dimension::Pixels(row_pad / 2.),
                })
                .border(BoxDimension::new(Dimension::Pixels(1.)))
                .border_corners(rounded())
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: new_tab.bg_color.to_linear().into(),
                    text: new_tab.fg_color.to_linear().into(),
                })
                .hover_colors(Some(ElementColors {
                    border: BorderColor::default(),
                    bg: new_tab_hover.bg_color.to_linear().into(),
                    text: new_tab_hover.fg_color.to_linear().into(),
                })),
        );

        let container = Element::new(&font, ElementContent::Children(children))
            .item_type(UIItemType::LeftTabBarBg)
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: bar_bg.into(),
                text: inactive_tab.fg_color.to_linear().into(),
            })
            .min_width(Some(Dimension::Pixels(width)))
            .min_height(Some(Dimension::Pixels(bottom - top)));

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
            bounds: euclid::rect(border.left.get() as f32, top, width, bottom - top),
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

        // Resize grip: a thin strip on the bar's right edge. Registered
        // after the rows so it wins hit-testing (resolve_ui_item picks
        // the most recently added item).
        let grip_w = (ui_tokens::LEFT_TAB_BAR_GRIP * pt).round() as usize;
        self.ui_items.push(UIItem {
            x: (border.left.get() as f32 + width) as usize - grip_w,
            width: grip_w,
            y: top as usize,
            height: (bottom - top).max(0.) as usize,
            item_type: UIItemType::LeftTabBarResize,
            pane_id: None,
        });

        Ok(())
    }
}
