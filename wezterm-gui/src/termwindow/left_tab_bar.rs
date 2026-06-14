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

use crate::customglyph::*;
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
        let top = top_bar_height + border.top.get() as f32;
        let status_h = if self.config.show_unterm_status_bar {
            self.status_bar_pixel_height()
        } else {
            0.
        };
        let bottom =
            self.dimensions.pixel_height as f32 - status_h - border.bottom.get() as f32;

        // Sidebar surface: the terminal background lifted ~5% toward the
        // foreground (Warp's fg_overlay_1). Enough to read as a distinct
        // full-height panel without floating jarringly like the old
        // titlebar color did. A 1px divider on the right seals the edge.
        let bg = palette.background.to_linear();
        let fgc = palette.foreground.to_linear();
        let lift = 0.05;
        let bar_bg = LinearRgba::with_components(
            bg.0 * (1. - lift) + fgc.0 * lift,
            bg.1 * (1. - lift) + fgc.1 * lift,
            bg.2 * (1. - lift) + fgc.2 * lift,
            1.,
        );
        let divider = fgc.mul_alpha(0.12);
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
        let dim = fg.mul_alpha(0.75); // subtitle / directory
        let sel_bg = fg.mul_alpha(0.10); // fg_overlay_2 — selected fill
        let sel_border = fg.mul_alpha(0.15); // fg_overlay_3 — selected border
        let hover_bg = fg.mul_alpha(0.08);

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
                        Some(p) => (
                            crate::mcp::handler::agent_for_pane(p.pane_id() as u64),
                            super::pane_cwd_path(p)
                                .and_then(|pp| pp.file_name().map(|n| n.to_string_lossy().to_string())),
                        ),
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

        let scroll_top = self.left_tab_bar.borrow().scroll_top.min(
            rows.len().saturating_sub(1),
        );
        // Uniform two-line rows make the scroll window arithmetic exact.
        let row_h = metrics.cell_size.height as f32 * 2.0 + 2.0 * row_pad + 4.0 * pt;
        let visible_rows = ((bottom - top - row_h) / row_h).floor().max(1.0) as usize;

        let mut children: Vec<Element> = vec![];

        for row in rows.iter().skip(scroll_top).take(visible_rows) {
            let title_fg = if row.active { fg } else { fg.mul_alpha(0.82) };

            // Row-leading indicator. AI-driven panes keep the colored
            // chip with the agent's initial — that's the row's accent
            // and the at-a-glance "this one is busy" signal. Idle rows
            // drop the chip background entirely (it read as a stray "+"
            // against the dim glyph at small sizes) and just show a
            // longer "→" arrow, like a list bullet pointing at the
            // title.
            let chip_px = 16. * pt;
            let chip_radius = Dimension::Pixels(chip_px / 2.);
            let circle = || {
                Some(Corners {
                    top_left: SizedPoly {
                        width: chip_radius,
                        height: chip_radius,
                        poly: TOP_LEFT_ROUNDED_CORNER,
                    },
                    top_right: SizedPoly {
                        width: chip_radius,
                        height: chip_radius,
                        poly: TOP_RIGHT_ROUNDED_CORNER,
                    },
                    bottom_left: SizedPoly {
                        width: chip_radius,
                        height: chip_radius,
                        poly: BOTTOM_LEFT_ROUNDED_CORNER,
                    },
                    bottom_right: SizedPoly {
                        width: chip_radius,
                        height: chip_radius,
                        poly: BOTTOM_RIGHT_ROUNDED_CORNER,
                    },
                })
            };
            let dot = if let Some(agent) = &row.agent {
                let initial = agent
                    .chars()
                    .find(|c| c.is_alphanumeric())
                    .map(|c| c.to_ascii_uppercase().to_string())
                    .unwrap_or_else(|| "•".to_string());
                Element::new(&font, ElementContent::Text(initial))
                    .vertical_align(VerticalAlign::Middle)
                    .min_width(Some(Dimension::Pixels(chip_px)))
                    .min_height(Some(Dimension::Pixels(chip_px)))
                    .border_corners(circle())
                    .margin(BoxDimension {
                        left: Dimension::Pixels(0.),
                        right: Dimension::Pixels(8. * pt),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Pixels(5. * pt),
                        right: Dimension::Pixels(0.),
                        top: Dimension::Pixels(1.5 * pt),
                        bottom: Dimension::Pixels(0.),
                    })
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: agent_color.into(),
                        text: bg.into(),
                    })
            } else {
                Element::new(&font, ElementContent::Text("→".to_string()))
                    .vertical_align(VerticalAlign::Middle)
                    .min_width(Some(Dimension::Pixels(chip_px)))
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

            // Title line: dot + title flow inline; the line itself is a
            // block so the subtitle drops beneath it.
            let title_text = if row.title.is_empty() {
                "shell".to_string()
            } else {
                row.title.clone()
            };
            let title_el = Element::new(&font, ElementContent::Text(title_text)).colors(
                ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: title_fg.into(),
                },
            );
            // Insets live on the content (not the row) because this box
            // model's rounded background fills only the content box — row
            // padding would leave a gap inside the border. 8px left + 6px
            // top reproduce Warp's uniform-8 breathing room.
            let title_line = Element::new(&font, ElementContent::Children(vec![dot, title_el]))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .padding(BoxDimension {
                    left: Dimension::Pixels(8. * pt),
                    right: Dimension::Pixels(8. * pt),
                    top: Dimension::Pixels(6. * pt),
                    bottom: Dimension::Pixels(0.),
                });

            // Subtitle line: "agent · dir", agent in cyan, dir dimmed.
            // Left-inset to sit under the title text (panel pad 8 + chip 16
            // + gap 8 = 32).
            let indent = 32. * pt;
            let mut sub_kids = vec![];
            if let Some(agent) = &row.agent {
                sub_kids.push(
                    Element::new(&font, ElementContent::Text(agent.clone())).colors(
                        ElementColors {
                            border: BorderColor::default(),
                            bg: LinearRgba::TRANSPARENT.into(),
                            text: agent_color.into(),
                        },
                    ),
                );
                if row.dir.is_some() {
                    sub_kids.push(
                        Element::new(&font, ElementContent::Text(" · ".to_string())).colors(
                            ElementColors {
                                border: BorderColor::default(),
                                bg: LinearRgba::TRANSPARENT.into(),
                                text: dim.into(),
                            },
                        ),
                    );
                }
            }
            sub_kids.push(
                Element::new(
                    &font,
                    ElementContent::Text(row.dir.clone().unwrap_or_else(|| " ".to_string())),
                )
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                }),
            );
            let subtitle_line = Element::new(&font, ElementContent::Children(sub_kids))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .padding(BoxDimension {
                    left: Dimension::Pixels(indent),
                    right: Dimension::Pixels(8. * pt),
                    top: Dimension::Pixels(2. * pt),
                    bottom: Dimension::Pixels(6. * pt),
                });

            // No inline close button — Warp's vertical-tab rows have none;
            // closing is via the right-click context menu.
            let kids = vec![title_line, subtitle_line];

            // Selected row: fg_overlay_2 fill + fg_overlay_3 1px border,
            // rounded — Warp's restrained greyscale selection, not a
            // saturated block.
            let (row_bg, row_border) = if row.active {
                (sel_bg, sel_border)
            } else {
                (LinearRgba::TRANSPARENT, LinearRgba::TRANSPARENT)
            };
            children.push(
                Element::new(&font, ElementContent::Children(kids))
                    .item_type(UIItemType::LeftTabBarTab(row.tab_idx))
                    .display(DisplayType::Block)
                    .min_width(Some(Dimension::Percent(1.)))
                    .margin(BoxDimension {
                        left: Dimension::Pixels(0.),
                        right: Dimension::Pixels(0.),
                        top: Dimension::Pixels(2. * pt),
                        bottom: Dimension::Pixels(2. * pt),
                    })
                    // No row padding — insets live on the content so the
                    // rounded background fills the whole border box with no
                    // gap (see title_line/subtitle_line padding above).
                    .padding(BoxDimension {
                        left: Dimension::Pixels(0.),
                        right: Dimension::Pixels(0.),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
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
        let plus_cell = Element::new(&font, ElementContent::Text("+".to_string()))
            .item_type(UIItemType::TabBar(crate::tabbar::TabBarItem::NewTabButton))
            .padding(BoxDimension {
                left: Dimension::Pixels(row_pad),
                right: Dimension::Pixels(row_pad / 2.),
                top: Dimension::Pixels(row_pad / 2.),
                bottom: Dimension::Pixels(row_pad / 2.),
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
            .padding(BoxDimension {
                left: Dimension::Pixels(row_pad / 2.),
                right: Dimension::Pixels(row_pad),
                top: Dimension::Pixels(row_pad / 2.),
                bottom: Dimension::Pixels(row_pad / 2.),
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
            Element::new(&font, ElementContent::Children(vec![plus_cell, chevron_cell]))
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
                top: Dimension::Pixels(6. * pt),
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
            .min_height(Some(Dimension::Pixels(bottom - top - 6. * pt)));

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
