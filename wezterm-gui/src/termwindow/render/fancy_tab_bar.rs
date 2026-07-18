use crate::customglyph::*;
use crate::tabbar::{TabBarItem, TabEntry};
use crate::termwindow::box_model::*;
use crate::termwindow::render::corners::*;

use crate::termwindow::render::window_buttons::window_button_element;
use crate::termwindow::{UIItem, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::{Dimension, DimensionContext, TabBarColors};
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use wezterm_font::LoadedFont;
use wezterm_term::color::{ColorAttribute, ColorPalette};
use window::{IntegratedTitleButtonAlignment, IntegratedTitleButtonStyle};

const X_BUTTON: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::One, BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::Zero, BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Zero, BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

const PLUS_BUTTON: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::Frac(1, 2), BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Zero, BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

/// Vertical geometry of the chrome-row buttons (quick actions / menu).
/// `tab_bar_pixel_height_impl` derives the reserved chrome height from
/// these same constants — if the buttons grow, the reserved height grows
/// with them instead of the first terminal row disappearing under the bar.
pub(crate) const QUICK_BTN_VMARGIN_CELLS: f32 = 0.22;
pub(crate) const QUICK_BTN_VPAD_CELLS: f32 = 0.28;
pub(crate) const QUICK_BTN_BORDER_PX: f32 = 1.0;
/// The 1 px `chrome_bottom_divider` drawn as the chrome element's bottom
/// border. Part of the painted chrome height, so part of the reserved
/// height too.
pub(crate) const CHROME_BOTTOM_DIVIDER_PX: f32 = 1.0;

impl crate::TermWindow {
    pub fn invalidate_fancy_tab_bar(&mut self) {
        self.fancy_tab_bar.take();
    }

    pub fn build_fancy_tab_bar(&self, palette: &ColorPalette) -> anyhow::Result<ComputedElement> {
        let tab_bar_height = self.tab_bar_pixel_height()?;
        let font = self.fonts.title_font()?;
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let items = self.tab_bar.items();
        let colors = self
            .config
            .colors
            .as_ref()
            .and_then(|c| c.tab_bar.as_ref())
            .cloned()
            .unwrap_or_else(TabBarColors::default);

        let mut left_status = vec![];
        let mut left_eles = vec![];
        let mut right_eles = vec![];
        // Top bar bg/fg follow the active color scheme by default. The bar
        // gets a subtle 5% lighten over the scheme background so the
        // chrome reads as a distinct panel from the pane content (a
        // user pinged this as "too flat" when chrome and content were
        // pixel-identical) — same ratio the bundled unterm.lua applies
        // via `theme_bg:lighten(0.05)`. The hardcoded window_frame
        // defaults only fire when the user has explicitly set those
        // fields.
        // Chrome bg = content color (the terminal background), no lift.
        // The top bar deliberately matches the pane bg so it reads as part
        // of the content surface; the lifted-grey LEFT SIDEBAR is what now
        // carries the visual "panel" layer. Previously the chrome got a
        // +5% lift too, which made it the same grey as the sidebar — the
        // two merged at their shared edge and the sidebar looked like it
        // was covering the top bar (user's "sidebar 压住顶栏" report). With
        // the chrome held at content color, the dark bar and the grey
        // sidebar sit at clearly different tones; the 1px
        // `chrome_bottom_divider` still seals the bar's bottom edge.
        let scheme_bg = palette.background.to_linear();
        let scheme_fg = palette.foreground.to_linear();
        // Scheme A: the top bar shares the sidebar's lifted chrome tone so the
        // whole chrome (top + left sidebar + bottom) forms one continuous frame
        // and the sidebar↔top-bar corner meets seamlessly, with the darker pane
        // content inset inside it.
        let chrome_surface =
            crate::termwindow::chrome_colors::sidebar(scheme_bg, scheme_fg).surface;
        let bar_bg = if self.focused.is_some() {
            if self.config.window_frame.active_bg_is_default() {
                chrome_surface
            } else {
                self.config.window_frame.active_titlebar_bg.to_linear()
            }
        } else if self.config.window_frame.inactive_bg_is_default() {
            chrome_surface
        } else {
            self.config.window_frame.inactive_titlebar_bg.to_linear()
        };
        let bar_fg = if self.focused.is_some() {
            if self.config.window_frame.active_fg_is_default() {
                scheme_fg
            } else {
                self.config.window_frame.active_titlebar_fg.to_linear()
            }
        } else if self.config.window_frame.inactive_fg_is_default() {
            scheme_fg.mul_alpha(0.7)
        } else {
            self.config.window_frame.inactive_titlebar_fg.to_linear()
        };
        // 1 px bottom divider so the chrome / sidebar / pane seam is
        // visible — without it, the chrome bg and the sidebar bg
        // resolve to similar greys and the entire left column reads as
        // one continuous panel (user's recurring "sidebar 压住顶栏"
        // complaint). 0.4 alpha was chosen after a 0.12 trial in v0.44.3
        // — that line was visible in code but invisible on screen.
        // Scheme A: the chrome (top bar + sidebar + bottom) is now one tone, so
        // the old 0.4-alpha bottom line just cut across the unified frame. Drop
        // it — the chrome↔content tone contrast already defines the edge.
        let chrome_bottom_divider = window::color::LinearRgba::TRANSPARENT;
        let bar_colors = ElementColors {
            border: BorderColor {
                left: window::color::LinearRgba::TRANSPARENT,
                right: window::color::LinearRgba::TRANSPARENT,
                top: window::color::LinearRgba::TRANSPARENT,
                bottom: chrome_bottom_divider,
            },
            bg: bar_bg.into(),
            text: bar_fg.into(),
        };

        let item_to_elem = |item: &TabEntry| -> Element {
            let element = Element::with_line(&font, &item.title, palette);

            let bg_color = item
                .title
                .get_cell(0)
                .and_then(|c| match c.attrs().background() {
                    ColorAttribute::Default => None,
                    col => Some(palette.resolve_bg(col)),
                });
            let fg_color = item
                .title
                .get_cell(0)
                .and_then(|c| match c.attrs().foreground() {
                    ColorAttribute::Default => None,
                    col => Some(palette.resolve_fg(col)),
                });

            let new_tab_hover = colors.new_tab_hover();
            let active_tab = colors.active_tab();

            match item.item {
                TabBarItem::RightStatus
                | TabBarItem::LeftStatus
                | TabBarItem::None
                | TabBarItem::MenuButton => element
                    .item_type(UIItemType::TabBar(TabBarItem::None))
                    .line_height(Some(2.0))
                    .margin(BoxDimension {
                        left: Dimension::Cells(0.),
                        right: Dimension::Cells(0.),
                        top: Dimension::Cells(0.1),
                        bottom: Dimension::Cells(0.),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Cells(0.5),
                        right: Dimension::Cells(0.),
                        top: Dimension::Cells(0.),
                        bottom: Dimension::Cells(0.),
                    })
                    .border(BoxDimension::new(Dimension::Pixels(0.)))
                    .colors(bar_colors.clone()),
                TabBarItem::NewTabButton => Element::new(
                    &font,
                    ElementContent::Poly {
                        line_width: metrics.underline_height.max(2),
                        poly: SizedPoly {
                            poly: PLUS_BUTTON,
                            width: Dimension::Pixels(metrics.cell_size.height as f32 / 2.),
                            height: Dimension::Pixels(metrics.cell_size.height as f32 / 2.),
                        },
                    },
                )
                .vertical_align(VerticalAlign::Middle)
                .item_type(UIItemType::TabBar(item.item.clone()))
                .margin(BoxDimension {
                    left: Dimension::Cells(0.5),
                    right: Dimension::Cells(0.),
                    top: Dimension::Cells(0.),
                    bottom: Dimension::Cells(0.),
                })
                .padding(BoxDimension {
                    left: Dimension::Cells(0.5),
                    right: Dimension::Cells(0.5),
                    top: Dimension::Cells(0.35),
                    bottom: Dimension::Cells(0.35),
                })
                .border(BoxDimension::new(Dimension::Pixels(1.)))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    // Frameless idle (transparent) so the row stays clean on
                    // both light and dark chrome, but a clearly brighter icon
                    // colour than the muted new-tab grey so the buttons are
                    // legible. Hover fills a solid chip for feedback.
                    bg: window::color::LinearRgba::TRANSPARENT.into(),
                    text: colors.inactive_tab().fg_color.to_linear().into(),
                })
                .hover_colors(Some(ElementColors {
                    border: BorderColor::default(),
                    bg: new_tab_hover.bg_color.to_linear().into(),
                    text: new_tab_hover.fg_color.to_linear().into(),
                })),
                TabBarItem::Tab { active, .. } if active => element
                    .vertical_align(VerticalAlign::Bottom)
                    .item_type(UIItemType::TabBar(item.item.clone()))
                    .margin(BoxDimension {
                        left: Dimension::Cells(0.),
                        right: Dimension::Cells(0.),
                        top: Dimension::Cells(0.),
                        bottom: Dimension::Cells(0.),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Cells(0.5),
                        right: Dimension::Cells(0.5),
                        top: Dimension::Cells(0.3),
                        bottom: Dimension::Cells(0.25),
                    })
                    .border(BoxDimension::new(Dimension::Pixels(1.)))
                    .border_corners(Some(Corners {
                        top_left: SizedPoly {
                            width: Dimension::Cells(0.5),
                            height: Dimension::Cells(0.5),
                            poly: TOP_LEFT_ROUNDED_CORNER,
                        },
                        top_right: SizedPoly {
                            width: Dimension::Cells(0.5),
                            height: Dimension::Cells(0.5),
                            poly: TOP_RIGHT_ROUNDED_CORNER,
                        },
                        bottom_left: SizedPoly::none(),
                        bottom_right: SizedPoly::none(),
                    }))
                    .colors(ElementColors {
                        border: BorderColor::new(
                            bg_color
                                .unwrap_or_else(|| active_tab.bg_color.into())
                                .to_linear(),
                        ),
                        bg: bg_color
                            .unwrap_or_else(|| active_tab.bg_color.into())
                            .to_linear()
                            .into(),
                        text: fg_color
                            .unwrap_or_else(|| active_tab.fg_color.into())
                            .to_linear()
                            .into(),
                    }),
                TabBarItem::Tab { .. } => element
                    .vertical_align(VerticalAlign::Bottom)
                    .item_type(UIItemType::TabBar(item.item.clone()))
                    .margin(BoxDimension {
                        left: Dimension::Cells(0.),
                        right: Dimension::Cells(0.),
                        top: Dimension::Cells(0.),
                        bottom: Dimension::Cells(0.),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Cells(0.5),
                        right: Dimension::Cells(0.5),
                        top: Dimension::Cells(0.3),
                        bottom: Dimension::Cells(0.25),
                    })
                    .border(BoxDimension::new(Dimension::Pixels(1.)))
                    .border_corners(Some(Corners {
                        top_left: SizedPoly {
                            width: Dimension::Cells(0.5),
                            height: Dimension::Cells(0.5),
                            poly: TOP_LEFT_ROUNDED_CORNER,
                        },
                        top_right: SizedPoly {
                            width: Dimension::Cells(0.5),
                            height: Dimension::Cells(0.5),
                            poly: TOP_RIGHT_ROUNDED_CORNER,
                        },
                        bottom_left: SizedPoly {
                            width: Dimension::Cells(0.),
                            height: Dimension::Cells(0.33),
                            poly: &[],
                        },
                        bottom_right: SizedPoly {
                            width: Dimension::Cells(0.),
                            height: Dimension::Cells(0.33),
                            poly: &[],
                        },
                    }))
                    .colors({
                        let inactive_tab = colors.inactive_tab();
                        let bg = bg_color
                            .unwrap_or_else(|| inactive_tab.bg_color.into())
                            .to_linear();
                        let edge = colors.inactive_tab_edge().to_linear();
                        ElementColors {
                            border: BorderColor {
                                left: bg,
                                right: edge,
                                top: bg,
                                bottom: bg,
                            },
                            bg: bg.into(),
                            text: fg_color
                                .unwrap_or_else(|| inactive_tab.fg_color.into())
                                .to_linear()
                                .into(),
                        }
                    })
                    .hover_colors({
                        let inactive_tab_hover = colors.inactive_tab_hover();
                        Some(ElementColors {
                            border: BorderColor::new(
                                bg_color
                                    .unwrap_or_else(|| inactive_tab_hover.bg_color.into())
                                    .to_linear(),
                            ),
                            bg: bg_color
                                .unwrap_or_else(|| inactive_tab_hover.bg_color.into())
                                .to_linear()
                                .into(),
                            text: fg_color
                                .unwrap_or_else(|| inactive_tab_hover.fg_color.into())
                                .to_linear()
                                .into(),
                        })
                    }),
                TabBarItem::WindowButton(button) => window_button_element(
                    button,
                    self.window_state.contains(window::WindowState::MAXIMIZED),
                    &font,
                    &metrics,
                    &self.config,
                ),
            }
        };

        let num_tabs: f32 = items
            .iter()
            .map(|item| match item.item {
                TabBarItem::NewTabButton | TabBarItem::Tab { .. } => 1.,
                _ => 0.,
            })
            .sum();
        let max_tab_width = ((self.dimensions.pixel_width as f32 / num_tabs)
            - (1.5 * metrics.cell_size.width as f32))
            .max(0.);

        // Reserve space for the native titlebar buttons
        if self
            .config
            .window_decorations
            .contains(::window::WindowDecorations::INTEGRATED_BUTTONS)
            && self.config.integrated_title_button_style == IntegratedTitleButtonStyle::MacOsNative
            && !self.window_state.contains(window::WindowState::FULL_SCREEN)
        {
            left_status.push(
                Element::new(&font, ElementContent::Text("".to_string())).margin(BoxDimension {
                    // Reserve the actual width of the traffic-light cluster —
                    // shared with the left_eles padding at line ~515 — instead
                    // of the older 4-cell guess, which under-reserved at large
                    // font sizes and over-reserved at small ones.
                    // Points (DPI-scaled) so the reserved width tracks the
                    // now DPI-scaled traffic-light cluster; Pixels under-
                    // reserved by half on Retina and let the cluster collide
                    // with the title.
                    left: Dimension::Points(config::ui_tokens::MACOS_TRAFFIC_LIGHT_RESERVE),
                    right: Dimension::Cells(0.),
                    top: Dimension::Cells(0.),
                    bottom: Dimension::Cells(0.),
                }),
            );
        }

        let menu_button = || {
            let new_tab_hover = colors.new_tab_hover();
            // ▾ chevron — codicon `cod_chevron_down`, rendered through the
            // bundled SymbolsNerdFontMono so CoreText's rasterizer does the
            // anti-aliasing (same path Warp uses for SF Symbols).
            Element::new(&font, ElementContent::Text("\u{eab4}".to_string()))
                .vertical_align(VerticalAlign::Middle)
                .item_type(UIItemType::TabBar(TabBarItem::MenuButton))
                // Same margin/padding as quick_button so the menu button is the
                // same height and inset from the bar edges (vertical breathing
                // room) instead of filling the chrome top-to-bottom.
                .margin(BoxDimension {
                    left: Dimension::Cells(0.5),
                    right: Dimension::Cells(0.),
                    top: Dimension::Cells(QUICK_BTN_VMARGIN_CELLS),
                    bottom: Dimension::Cells(QUICK_BTN_VMARGIN_CELLS),
                })
                .padding(BoxDimension {
                    left: Dimension::Cells(0.5),
                    right: Dimension::Cells(0.5),
                    top: Dimension::Cells(QUICK_BTN_VPAD_CELLS),
                    bottom: Dimension::Cells(QUICK_BTN_VPAD_CELLS),
                })
                .border(BoxDimension::new(Dimension::Pixels(QUICK_BTN_BORDER_PX)))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    // Frameless idle (transparent) so the row stays clean on
                    // both light and dark chrome, but a clearly brighter icon
                    // colour than the muted new-tab grey so the buttons are
                    // legible. Hover fills a solid chip for feedback.
                    bg: window::color::LinearRgba::TRANSPARENT.into(),
                    text: colors.inactive_tab().fg_color.to_linear().into(),
                })
                .hover_colors(Some(ElementColors {
                    border: BorderColor::default(),
                    bg: new_tab_hover.bg_color.to_linear().into(),
                    text: new_tab_hover.fg_color.to_linear().into(),
                }))
        };

        // Quick actions on the right of the top bar. Each button is a
        // single Nerd Font glyph (Codicons) rendered through the bundled
        // SymbolsNerdFontMono — CoreText/FreeType does the AA on the
        // vector outline, so we get Warp-grade crispness instead of the
        // jaggy geometric polylines we shipped before.
        let quick_button = |glyph: &'static str, action: crate::termwindow::QuickAction| {
            let new_tab_hover = colors.new_tab_hover();
            Element::new(&font, ElementContent::Text(glyph.to_string()))
                .vertical_align(VerticalAlign::Middle)
                .item_type(UIItemType::QuickAction(action))
                // Margin doubled (0.25 → 0.5 cells) so icons breathe instead
                // of clumping at the right edge; padding bumped vertically
                // so the glyph sits visually centered in the taller bar.
                // Top/bottom margin insets the button from the bar edges so it
                // no longer fills the chrome top-to-bottom; padding shrinks to
                // keep the glyph the same size while opening up the gap.
                .margin(BoxDimension {
                    left: Dimension::Cells(0.5),
                    right: Dimension::Cells(0.),
                    top: Dimension::Cells(QUICK_BTN_VMARGIN_CELLS),
                    bottom: Dimension::Cells(QUICK_BTN_VMARGIN_CELLS),
                })
                .padding(BoxDimension {
                    left: Dimension::Cells(0.5),
                    right: Dimension::Cells(0.5),
                    top: Dimension::Cells(QUICK_BTN_VPAD_CELLS),
                    bottom: Dimension::Cells(QUICK_BTN_VPAD_CELLS),
                })
                .border(BoxDimension::new(Dimension::Pixels(QUICK_BTN_BORDER_PX)))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    // Frameless idle (transparent) so the row stays clean on
                    // both light and dark chrome, but a clearly brighter icon
                    // colour than the muted new-tab grey so the buttons are
                    // legible. Hover fills a solid chip for feedback.
                    bg: window::color::LinearRgba::TRANSPARENT.into(),
                    text: colors.inactive_tab().fg_color.to_linear().into(),
                })
                .hover_colors(Some(ElementColors {
                    border: BorderColor::default(),
                    bg: new_tab_hover.bg_color.to_linear().into(),
                    text: new_tab_hover.fg_color.to_linear().into(),
                }))
        };
        {
            use crate::termwindow::QuickAction as QA;
            // Per-pane stats text rides on the same row as the quick
            // actions — vertically centered with the icons, sitting
            // just to the left of the action cluster. Rendering it
            // here (instead of in a separate strip) means the icons
            // never compete for zindex with the strip and the chrome
            // stays single-row.
            let stats_text = compose_top_stats_text(self);
            if !stats_text.is_empty() {
                // Palette-derived accent so the stats text picks up the
                // scheme's bright-cyan slot instead of a hardcoded
                // #6fccb8 (which clashed with warm-toned light themes).
                let stats_teal = palette
                    .resolve_fg(ColorAttribute::PaletteIndex(14))
                    .to_linear();
                // Render in the terminal font (JetBrains Mono) instead
                // of the chrome title font (SF Pro on macOS). The stats
                // string mixes letters, digits, Powerline icons and
                // punctuation — proportional SF Pro hits a different
                // fallback font per glyph class so the baseline visibly
                // drifts (`⎇ master 3.7% CPU · 2M` read as misaligned).
                // JBM carries the whole run at one cell + one baseline.
                let stats_font = self.fonts.default_font()?;
                right_eles.push(
                    Element::new(&stats_font, ElementContent::Text(stats_text))
                        .vertical_align(VerticalAlign::Middle)
                        .margin(BoxDimension {
                            left: Dimension::Cells(0.),
                            right: Dimension::Cells(0.75),
                            top: Dimension::Cells(0.),
                            bottom: Dimension::Cells(0.),
                        })
                        .colors(ElementColors {
                            border: BorderColor::default(),
                            bg: window::color::LinearRgba::TRANSPARENT.into(),
                            text: stats_teal.into(),
                        }),
                );
            }
            // Agent Cockpit chip: aggregate agent activity across ALL
            // windows (⚡N working, ✋M waiting). Hidden when no agent is
            // tracked; waiting > 0 switches to the warning accent so a
            // blocked agent is visible from any tab. Click opens the
            // Inbox palette.
            if self.config.cockpit_enabled {
                let (working, waiting) = crate::cockpit::summary();
                if working + waiting > 0 {
                    // Codicons (same family the quick-action buttons use, so
                    // the glyphs are guaranteed present): pulse = working,
                    // bell = waiting. The terminal font's emoji fallback
                    // can't be relied on for ⚡/✋ here.
                    let mut chip = String::new();
                    if working > 0 {
                        chip.push_str(&format!("\u{eb30} {working}"));
                    }
                    if waiting > 0 {
                        if !chip.is_empty() {
                            chip.push_str("  ");
                        }
                        chip.push_str(&format!("\u{eaa2} {waiting}"));
                    }
                    let accent = if waiting > 0 {
                        // Palette bright-yellow slot: "an agent needs you".
                        palette.resolve_fg(ColorAttribute::PaletteIndex(11)).to_linear()
                    } else {
                        palette.resolve_fg(ColorAttribute::PaletteIndex(14)).to_linear()
                    };
                    let chip_font = font.clone();
                    let new_tab_hover = colors.new_tab_hover();
                    right_eles.push(
                        Element::new(&chip_font, ElementContent::Text(chip))
                            .vertical_align(VerticalAlign::Middle)
                            .item_type(UIItemType::QuickAction(QA::Inbox))
                            .margin(BoxDimension {
                                left: Dimension::Cells(0.5),
                                right: Dimension::Cells(0.),
                                top: Dimension::Cells(QUICK_BTN_VMARGIN_CELLS),
                                bottom: Dimension::Cells(QUICK_BTN_VMARGIN_CELLS),
                            })
                            .padding(BoxDimension {
                                left: Dimension::Cells(0.5),
                                right: Dimension::Cells(0.5),
                                top: Dimension::Cells(QUICK_BTN_VPAD_CELLS),
                                bottom: Dimension::Cells(QUICK_BTN_VPAD_CELLS),
                            })
                            .border(BoxDimension::new(Dimension::Pixels(QUICK_BTN_BORDER_PX)))
                            .colors(ElementColors {
                                border: BorderColor::default(),
                                bg: window::color::LinearRgba::TRANSPARENT.into(),
                                text: accent.into(),
                            })
                            .hover_colors(Some(ElementColors {
                                border: BorderColor::default(),
                                bg: new_tab_hover.bg_color.to_linear().into(),
                                text: new_tab_hover.fg_color.to_linear().into(),
                            })),
                    );
                }
            }
            // Codicons — VS Code's single-weight icon family. Same visual
            // language as Warp's SF Symbols, but bundled cross-platform via
            // SymbolsNerdFontMono so Win/Linux look identical.
            right_eles.push(quick_button("\u{eb6a}", QA::CommandPalette)); // cod_three_bars
            right_eles.push(quick_button("\u{ebf3}", QA::TreeSidebar)); // cod_layout_sidebar_left
            right_eles.push(quick_button("\u{eb56}", QA::SplitRight)); // cod_split_horizontal
            right_eles.push(quick_button("\u{ea83}", QA::DirJump)); // cod_folder
            right_eles.push(quick_button("\u{ea6d}", QA::Search)); // cod_search
            right_eles.push(quick_button("\u{eb51}", QA::Settings)); // cod_settings_gear
                                                                     // The ▾ menu sits at the far right after the quick actions —
                                                                     // the design doc's "右侧一排动作 + 一个菜单". The menu still
                                                                     // routes through TabBarItem::MenuButton → show_settings_menu.
            right_eles.push(menu_button());
        }

        // With the left vertical tab bar active, tabs and the + button
        // live in the sidebar; the top bar keeps window buttons, the
        // active title, the menu and the quick actions.
        let tabs_at_left = self.config.tab_bar_position == config::TabBarPosition::Left;

        for item in items {
            match item.item {
                TabBarItem::LeftStatus => left_status.push(item_to_elem(item)),
                TabBarItem::None | TabBarItem::RightStatus => right_eles.push(item_to_elem(item)),
                TabBarItem::WindowButton(_) => {
                    // MacOsCustom dots always live on the left, the
                    // way macOS users expect — irrespective of the
                    // `integrated_title_button_alignment` default
                    // (which is `Right`, set for Windows-style chrome).
                    let force_left = self.config.integrated_title_button_style
                        == IntegratedTitleButtonStyle::MacOsCustom;
                    if force_left
                        || self.config.integrated_title_button_alignment
                            == IntegratedTitleButtonAlignment::Left
                    {
                        left_eles.push(item_to_elem(item))
                    } else {
                        right_eles.push(item_to_elem(item))
                    }
                }
                TabBarItem::Tab { .. } if tabs_at_left => {
                    // Sidebar mode: the tab title already lives in the
                    // sidebar row, so duplicating it in the top bar made
                    // the chrome read like a hold-over from the top-tab
                    // design ("标题栏已经不需要那个 zsh 标记了啊"). Skip
                    // the push — top bar is now just traffic lights +
                    // window buttons + quick actions.
                }
                TabBarItem::Tab { tab_idx, active } => {
                    let mut elem = item_to_elem(item);
                    elem.max_width = Some(Dimension::Pixels(max_tab_width));
                    elem.content = match elem.content {
                        ElementContent::Text(_) => unreachable!(),
                        ElementContent::Poly { .. } => unreachable!(),
                        ElementContent::Children(mut kids) => {
                            if self.config.show_close_tab_button_in_tabs {
                                kids.push(make_x_button(&font, &metrics, &colors, tab_idx, active));
                            }
                            ElementContent::Children(kids)
                        }
                    };
                    left_eles.push(elem);
                }
                TabBarItem::NewTabButton => {
                    if !tabs_at_left {
                        left_eles.push(item_to_elem(item));
                    }
                    // Menu moved to the right-side action bar (right_eles).
                }
                _ => left_eles.push(item_to_elem(item)),
            }
        }

        let mut children = vec![];

        if !left_status.is_empty() {
            children.push(
                Element::new(&font, ElementContent::Children(left_status))
                    .colors(bar_colors.clone()),
            );
        }

        let window_buttons_at_left = self
            .config
            .window_decorations
            .contains(window::WindowDecorations::INTEGRATED_BUTTONS)
            && (self.config.integrated_title_button_alignment
                == IntegratedTitleButtonAlignment::Left
                || self.config.integrated_title_button_style
                    == IntegratedTitleButtonStyle::MacOsNative
                || self.config.integrated_title_button_style
                    == IntegratedTitleButtonStyle::MacOsCustom);

        let left_padding = if window_buttons_at_left {
            if self.config.integrated_title_button_style == IntegratedTitleButtonStyle::MacOsNative
            {
                if !self.window_state.contains(window::WindowState::FULL_SCREEN) {
                    // Points (DPI-scaled): the native traffic lights are ~24 px
                    // caps on Retina, so a raw-pixel reserve of 76 was half the
                    // cluster's real width and the title crowded them.
                    Dimension::Points(config::ui_tokens::MACOS_TRAFFIC_LIGHT_RESERVE)
                } else {
                    Dimension::Cells(0.5)
                }
            } else {
                Dimension::Pixels(0.0)
            }
        } else {
            Dimension::Cells(0.5)
        };

        // Brand mark at the far left of the info (top) bar: a compact UT
        // wordmark that mirrors the Shared Rail app icon. Skipped on macOS (the
        // traffic lights own that corner) and whenever window buttons are
        // left-aligned, so the mark never collides with them.
        #[cfg(not(target_os = "macos"))]
        if !window_buttons_at_left {
            // Use bar_fg, not palette.foreground: on themes that override the
            // titlebar bg/fg (e.g. a dark info bar over a light scheme), the
            // scheme foreground is tuned for the light content area and renders
            // near-invisible on the dark bar. bar_fg is the bar's own text
            // color and always contrasts with bar_bg.
            let brand_fg = bar_fg;
            let brand_accent = palette
                .resolve_fg(ColorAttribute::PaletteIndex(14))
                .to_linear();
            let brand_kids = vec![
                Element::new(&font, ElementContent::Text("U".to_string()))
                    .vertical_align(VerticalAlign::Middle)
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: window::color::LinearRgba::TRANSPARENT.into(),
                        text: brand_fg.into(),
                    }),
                Element::new(&font, ElementContent::Text("T".to_string()))
                    .vertical_align(VerticalAlign::Middle)
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: window::color::LinearRgba::TRANSPARENT.into(),
                        text: brand_accent.into(),
                    }),
                Element::new(&font, ElementContent::Text(" Unterm".to_string()))
                    .vertical_align(VerticalAlign::Middle)
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: window::color::LinearRgba::TRANSPARENT.into(),
                        text: brand_fg.into(),
                    }),
            ];
            left_eles.insert(
                0,
                Element::new(&font, ElementContent::Children(brand_kids))
                    .vertical_align(VerticalAlign::Middle)
                    .margin(BoxDimension {
                        left: Dimension::Cells(0.3),
                        right: Dimension::Cells(0.7),
                        top: Dimension::Cells(0.),
                        bottom: Dimension::Cells(0.),
                    }),
            );
        }

        children.push(
            Element::new(&font, ElementContent::Children(left_eles))
                // Middle, not Bottom — traffic lights sat against the bottom
                // edge of the 2.4× bar leaving a top gap that read as the
                // bar being too tall, which it isn't. Center keeps the
                // whole top strip visually balanced.
                .vertical_align(VerticalAlign::Middle)
                .colors(bar_colors.clone())
                .padding(BoxDimension {
                    left: left_padding,
                    right: Dimension::Cells(0.),
                    top: Dimension::Cells(0.),
                    bottom: Dimension::Cells(0.),
                })
                .zindex(1),
        );
        children.push(
            Element::new(&font, ElementContent::Children(right_eles))
                .vertical_align(VerticalAlign::Middle)
                .colors(bar_colors.clone())
                .float(Float::Right)
                // zindex ABOVE the left tab cluster (which is zindex 1).
                // When the window is narrowed, the left tabs extend toward
                // the right edge and overlap the floated right cluster; a
                // lower zindex let the tabs paint OVER the window
                // min/maximize/close buttons on Windows, hiding controls
                // the user must always be able to reach. The right cluster
                // now wins that overlap.
                .zindex(2),
        );

        let content = ElementContent::Children(children);

        let tabs = Element::new(&font, content)
            .display(DisplayType::Block)
            .item_type(UIItemType::TabBar(TabBarItem::None))
            .min_width(Some(Dimension::Pixels(self.dimensions.pixel_width as f32)))
            // min_height constrains the content box; the bottom divider
            // border is drawn outside it, so subtract it here to keep the
            // painted total equal to tab_bar_pixel_height().
            .min_height(Some(Dimension::Pixels(
                tab_bar_height - CHROME_BOTTOM_DIVIDER_PX,
            )))
            .vertical_align(VerticalAlign::Middle)
            // 1 px bottom border carries `chrome_bottom_divider`. The
            // other three edges stay zero-width so we don't paint a
            // frame around the whole top strip.
            .border(BoxDimension {
                left: Dimension::Pixels(0.),
                right: Dimension::Pixels(0.),
                top: Dimension::Pixels(0.),
                bottom: Dimension::Pixels(CHROME_BOTTOM_DIVIDER_PX),
            })
            .colors(bar_colors);

        let border = self.get_os_border();

        let mut computed = self.compute_element(
            &LayoutContext {
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
                bounds: euclid::rect(
                    border.left.get() as f32,
                    0.,
                    self.dimensions.pixel_width as f32 - (border.left + border.right).get() as f32,
                    tab_bar_height,
                ),
                metrics: &metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                // Chrome zindex bumped above the sidebar's 18 so the
                // chrome strip wins in any vertical overlap with the
                // sidebar's bg paint — user wanted "顶栏压住边框，
                // 而不是边框压顶栏". The chrome's bottom edge therefore
                // reads as a hard horizontal boundary even when the
                // sidebar's grey panel paints right up to chrome.y_bottom.
                zindex: 20,
            },
            &tabs,
        )?;

        log::debug!(
            "fancy-tab-bar: computed height {} vs tab_bar_pixel_height {} (title cell {})",
            computed.bounds.height(),
            tab_bar_height,
            metrics.cell_size.height,
        );
        computed.translate(euclid::vec2(
            0.,
            if self.config.tab_bar_at_bottom {
                self.dimensions.pixel_height as f32
                    - (computed.bounds.height() + border.bottom.get() as f32)
            } else {
                border.top.get() as f32
            },
        ));

        Ok(computed)
    }

    pub fn paint_fancy_tab_bar(&self) -> anyhow::Result<Vec<UIItem>> {
        let computed = self.fancy_tab_bar.as_ref().ok_or_else(|| {
            anyhow::anyhow!("paint_fancy_tab_bar called but fancy_tab_bar is None")
        })?;
        let ui_items = computed.ui_items();

        let gl_state = self.render_state.as_ref().unwrap();
        self.render_element(&computed, gl_state, None)?;

        Ok(ui_items)
    }
}

pub(crate) fn make_x_button(
    font: &Rc<LoadedFont>,
    metrics: &RenderMetrics,
    colors: &TabBarColors,
    tab_idx: usize,
    active: bool,
) -> Element {
    Element::new(
        &font,
        ElementContent::Poly {
            line_width: metrics.underline_height.max(2),
            poly: SizedPoly {
                poly: X_BUTTON,
                width: Dimension::Pixels(metrics.cell_size.height as f32 * 0.42),
                height: Dimension::Pixels(metrics.cell_size.height as f32 * 0.42),
            },
        },
    )
    // Ensure that we draw our background over the
    // top of the rest of the tab contents
    .zindex(1)
    .vertical_align(VerticalAlign::Middle)
    .float(Float::Right)
    .padding(BoxDimension {
        left: Dimension::Cells(0.2),
        right: Dimension::Cells(0.2),
        top: Dimension::Cells(0.2),
        bottom: Dimension::Cells(0.2),
    })
    .item_type(UIItemType::CloseTab(tab_idx))
    .hover_colors({
        let inactive_tab_hover = colors.inactive_tab_hover();
        let active_tab = colors.active_tab();

        Some(ElementColors {
            border: BorderColor::default(),
            bg: (if active {
                inactive_tab_hover.bg_color
            } else {
                active_tab.bg_color
            })
            .to_linear()
            .into(),
            text: (if active {
                inactive_tab_hover.fg_color
            } else {
                active_tab.fg_color
            })
            .to_linear()
            .into(),
        })
    })
    .padding(BoxDimension {
        left: Dimension::Cells(0.25),
        right: Dimension::Cells(0.25),
        top: Dimension::Cells(0.25),
        bottom: Dimension::Cells(0.25),
    })
    .margin(BoxDimension {
        left: Dimension::Cells(0.5),
        right: Dimension::Cells(0.),
        top: Dimension::Cells(0.),
        bottom: Dimension::Cells(0.),
    })
}

/// Compose the per-pane stats text rendered in the top bar. Mirrors
/// what the old paint_top_stats_bar produced — agent, git, CPU/MEM,
/// pane title — joined by 4-space gaps, with an "—" placeholder when
/// every segment is empty so the slot stays visible while the async
/// caches warm.
pub(crate) fn compose_top_stats_text(win: &crate::TermWindow) -> String {
    const TOP_STATS_TEXT_TTL: Duration = Duration::from_millis(250);

    #[derive(Clone, Default)]
    struct CachedTopStatsText {
        pane_id: Option<u64>,
        width_bucket: u32,
        value: String,
        updated: Option<Instant>,
    }

    fn cache() -> &'static Mutex<CachedTopStatsText> {
        static CACHE: OnceLock<Mutex<CachedTopStatsText>> = OnceLock::new();
        CACHE.get_or_init(|| Mutex::new(CachedTopStatsText::default()))
    }

    let width = win.dimensions.pixel_width as f32;
    if width < 900.0 {
        return String::new();
    }

    let active = win.get_active_pane_or_overlay();
    let pane_id = active.as_ref().map(|p| p.pane_id() as u64);
    let width_bucket = (width / 50.0) as u32;

    {
        let cached = cache().lock().unwrap();
        if cached.pane_id == pane_id
            && cached.width_bucket == width_bucket
            && cached
                .updated
                .map(|at| at.elapsed() < TOP_STATS_TEXT_TTL)
                .unwrap_or(false)
        {
            return cached.value.clone();
        }
    }

    let value = compute_top_stats_text(active);
    {
        let mut cached = cache().lock().unwrap();
        cached.pane_id = pane_id;
        cached.width_bucket = width_bucket;
        cached.value = value.clone();
        cached.updated = Some(Instant::now());
    }
    value
}

fn compute_top_stats_text(active: Option<std::sync::Arc<dyn mux::pane::Pane>>) -> String {
    use crate::termwindow::top_stats_bar;

    let cwd = active
        .as_ref()
        .and_then(|p| crate::termwindow::pane_cwd_path(p));
    let proc_info = active
        .as_ref()
        .and_then(|p| p.get_foreground_process_info(mux::pane::CachePolicy::AllowStale));
    let git_text = cwd
        .and_then(|c| top_stats_bar::git_status_for(&c))
        .map(|s| top_stats_bar::render_git_segment(&Some(s)))
        .unwrap_or_default();
    let proc_text = proc_info
        .as_ref()
        .and_then(|p| top_stats_bar::proc_status_for(p.pid))
        .map(|s| top_stats_bar::render_proc_segment(&Some(s)))
        .unwrap_or_default();
    let pane_id = active.as_ref().map(|p| p.pane_id() as u64);
    let agent_text = pane_id
        .and_then(|id| crate::mcp::handler::detect_agent_for_pane(id, proc_info.as_ref()))
        .map(|name| format!("⚡ {name}"))
        .unwrap_or_default();
    let title_text = active
        .as_ref()
        .map(|p| p.get_title())
        .map(|t| {
            let trimmed = t.trim().to_string();
            if trimmed.is_empty()
                || matches!(trimmed.as_str(), "zsh" | "bash" | "fish" | "nu" | "sh")
            {
                String::new()
            } else {
                format!("▶ {trimmed}")
            }
        })
        .unwrap_or_default();
    let segments: Vec<String> = vec![agent_text, git_text, proc_text, title_text]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    if segments.is_empty() {
        String::new()
    } else {
        segments.join("    ")
    }
}
