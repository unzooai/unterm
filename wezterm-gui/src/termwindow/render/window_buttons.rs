use crate::customglyph::*;
use crate::termwindow::box_model::*;
use crate::termwindow::render::corners::*;
use crate::termwindow::{TabBarItem, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::{ui_tokens, ConfigHandle, Dimension, IntegratedTitleButtonColor};
use std::rc::Rc;
use wezterm_font::LoadedFont;
use window::color::LinearRgba;
use window::{IntegratedTitleButton, IntegratedTitleButtonStyle as Style};

pub struct WindowButtonColors {
    pub colors: ElementColors,
    pub hover_colors: ElementColors,
}

fn auto_button_color(
    background_lightness: f64,
    foreground: IntegratedTitleButtonColor,
) -> LinearRgba {
    match foreground {
        IntegratedTitleButtonColor::Custom(color) => color.to_linear(),
        IntegratedTitleButtonColor::Auto => {
            if background_lightness > 0.5 {
                LinearRgba(0.0, 0.0, 0.0, 1.0)
            } else {
                LinearRgba(1.0, 1.0, 1.0, 1.0)
            }
        }
    }
}

fn hover_fill(background_lightness: f64, foreground: LinearRgba) -> LinearRgba {
    let alpha = if background_lightness > 0.5 {
        0.14
    } else {
        0.11
    };
    foreground.mul_alpha(alpha)
}

fn titlebar_lightness(bg: LinearRgba) -> f64 {
    (0.2126 * bg.0 + 0.7152 * bg.1 + 0.0722 * bg.2) as f64
}

mod windows {
    use super::*;

    pub const CLOSE: &[Poly] = &[Poly {
        path: &[
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::One),
            PolyCommand::MoveTo(BlockCoord::One, BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::Zero, BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::OutlineThin,
    }];

    pub const HIDE: &[Poly] = &[Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Zero, BlockCoord::Frac(6, 10)),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::Frac(6, 10)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::OutlineThin,
    }];

    pub const MAXIMIZE: &[Poly] = &[Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(2, 10), BlockCoord::Frac(1, 10)),
            PolyCommand::LineTo(BlockCoord::Frac(9, 10), BlockCoord::Frac(1, 10)),
            PolyCommand::LineTo(BlockCoord::Frac(10, 10), BlockCoord::Frac(2, 10)),
            PolyCommand::LineTo(BlockCoord::Frac(10, 10), BlockCoord::Frac(9, 10)),
            PolyCommand::LineTo(BlockCoord::Frac(9, 10), BlockCoord::Frac(10, 10)),
            PolyCommand::LineTo(BlockCoord::Frac(2, 10), BlockCoord::Frac(10, 10)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 10), BlockCoord::Frac(9, 10)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 10), BlockCoord::Frac(2, 10)),
            PolyCommand::LineTo(BlockCoord::Frac(2, 10), BlockCoord::Frac(1, 10)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::OutlineThin,
    }];

    pub const RESTORE: &[Poly] = &[
        Poly {
            path: &[
                PolyCommand::MoveTo(BlockCoord::Frac(5, 20), BlockCoord::Frac(1, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(8, 10), BlockCoord::Frac(1, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(10, 10), BlockCoord::Frac(3, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(10, 10), BlockCoord::Frac(15, 20)),
            ],
            intensity: BlockAlpha::Full,
            style: PolyStyle::OutlineThin,
        },
        Poly {
            path: &[
                PolyCommand::MoveTo(BlockCoord::Frac(2, 10), BlockCoord::Frac(3, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(7, 10), BlockCoord::Frac(3, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(8, 10), BlockCoord::Frac(4, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(8, 10), BlockCoord::Frac(9, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(7, 10), BlockCoord::Frac(10, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(2, 10), BlockCoord::Frac(10, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(1, 10), BlockCoord::Frac(9, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(1, 10), BlockCoord::Frac(4, 10)),
                PolyCommand::LineTo(BlockCoord::Frac(2, 10), BlockCoord::Frac(3, 10)),
            ],
            intensity: BlockAlpha::Full,
            style: PolyStyle::OutlineThin,
        },
    ];

    pub fn sized_poly(poly: &'static [Poly]) -> SizedPoly {
        let scale = 72.0 / 96.0;
        let size = Dimension::Points(10. * scale);
        SizedPoly {
            poly,
            width: size,
            height: size,
        }
    }

    pub fn window_button_colors(
        background_lightness: f64,
        foreground: config::IntegratedTitleButtonColor,
        window_button: IntegratedTitleButton,
    ) -> WindowButtonColors {
        let foreground = auto_button_color(background_lightness, foreground);
        let colors = ElementColors {
            border: BorderColor::new(LinearRgba::TRANSPARENT),
            bg: LinearRgba::TRANSPARENT.into(),
            text: foreground.into(),
        };

        let hover_colors = if window_button == IntegratedTitleButton::Close {
            ElementColors {
                border: BorderColor::new(LinearRgba::TRANSPARENT),
                bg: LinearRgba(0.88, 0.05, 0.04, 1.0).into(),
                text: LinearRgba(1.0, 1.0, 1.0, 1.0).into(),
            }
        } else {
            ElementColors {
                border: BorderColor::new(LinearRgba::TRANSPARENT),
                bg: hover_fill(background_lightness, foreground).into(),
                text: foreground.into(),
            }
        };

        WindowButtonColors {
            colors,
            hover_colors,
        }
    }
}

mod gnome {
    use super::*;

    pub const CLOSE: &[Poly] = &[Poly {
        path: &[
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::One),
            PolyCommand::MoveTo(BlockCoord::One, BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::Zero, BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    }];

    pub const HIDE: &[Poly] = &[Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Zero, BlockCoord::Frac(15, 16)),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::Frac(15, 16)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    }];

    pub const MAXIMIZE: &[Poly] = &[Poly {
        path: &[
            PolyCommand::LineTo(BlockCoord::Frac(1, 16), BlockCoord::Frac(15, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(15, 16), BlockCoord::Frac(15, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(15, 16), BlockCoord::Frac(1, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 16), BlockCoord::Frac(1, 16)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    }];

    pub const RESTORE: &[Poly] = &[Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(3, 16), BlockCoord::Frac(3, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(3, 16), BlockCoord::Frac(13, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(13, 16), BlockCoord::Frac(13, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(13, 16), BlockCoord::Frac(3, 16)),
            PolyCommand::LineTo(BlockCoord::Frac(3, 16), BlockCoord::Frac(3, 16)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    }];

    pub fn sized_poly(poly: &'static [Poly]) -> SizedPoly {
        let size = Dimension::Pixels(8.);
        SizedPoly {
            poly,
            width: size,
            height: size,
        }
    }

    pub fn window_button_colors(
        background_lightness: f64,
        foreground: config::IntegratedTitleButtonColor,
        _window_button: IntegratedTitleButton,
    ) -> WindowButtonColors {
        let foreground = auto_button_color(background_lightness, foreground);
        let bg = hover_fill(background_lightness, foreground);
        let hover_bg = hover_fill(background_lightness, foreground).mul_alpha(1.35);
        WindowButtonColors {
            colors: ElementColors {
                border: BorderColor::new(bg),
                bg: bg.into(),
                text: foreground.into(),
            },
            hover_colors: ElementColors {
                border: BorderColor::new(hover_bg),
                bg: hover_bg.into(),
                text: foreground.into(),
            },
        }
    }
}

mod macos {
    //! Custom-drawn macOS-style traffic lights. Three filled circles
    //! that ride in our own box-model so `VerticalAlign::Middle`
    //! actually centers them in the chrome (the OS native lights
    //! can't be vertically centered because AppKit anchors them to
    //! a fixed offset from the window top).
    //!
    //! Trade-off: no OS hover glyphs (X / − / +). The dots stay
    //! solid on hover; the click → close / hide / zoom routing is
    //! identical to the other custom styles.
    use super::*;

    /// One filled circle. The circle is drawn at the center of the
    /// element's bounding box at radius 0.5 in BlockCoord space — the
    /// rasterizer scales it to whatever `sized_poly` reports.
    pub const DOT: &[Poly] = &[Poly {
        path: &[PolyCommand::Circle {
            center: (BlockCoord::Frac(1, 2), BlockCoord::Frac(1, 2)),
            radius: BlockCoord::Frac(1, 2),
        }],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Fill,
    }];

    pub fn sized_poly(poly: &'static [Poly]) -> SizedPoly {
        // 12 *points* matches the macOS traffic-light cap diameter. It must be
        // Points, not Pixels: `Dimension::Pixels` is raw device pixels, so on a
        // 2× Retina display Pixels(12) rendered a 12 px dot — half the native
        // 24 px cap — which read as tiny traffic lights. Points scales by DPI.
        let size = Dimension::Points(ui_tokens::MACOS_TRAFFIC_LIGHT_DOT);
        SizedPoly {
            poly,
            width: size,
            height: size,
        }
    }

    pub fn window_button_colors(
        _background_lightness: f64,
        _foreground: config::IntegratedTitleButtonColor,
        window_button: IntegratedTitleButton,
    ) -> WindowButtonColors {
        // Apple-design palette for the three caps. Values are the
        // commonly-cited "macOS Big Sur+" hex set, slightly desaturated
        // so the dots don't out-shout the chrome content.
        let (r, g, b) = match window_button {
            IntegratedTitleButton::Close => (0xed, 0x6a, 0x5e),
            IntegratedTitleButton::Hide => (0xf4, 0xbf, 0x4f),
            IntegratedTitleButton::Maximize => (0x61, 0xc5, 0x54),
        };
        let dot = LinearRgba(
            (r as f32 / 255.0).powf(2.2),
            (g as f32 / 255.0).powf(2.2),
            (b as f32 / 255.0).powf(2.2),
            1.0,
        );

        WindowButtonColors {
            colors: ElementColors {
                border: BorderColor::new(LinearRgba::TRANSPARENT),
                bg: LinearRgba::TRANSPARENT.into(),
                text: dot.into(),
            },
            hover_colors: ElementColors {
                border: BorderColor::new(LinearRgba::TRANSPARENT),
                bg: LinearRgba::TRANSPARENT.into(),
                text: dot.into(),
            },
        }
    }
}

pub fn window_button_element(
    window_button: IntegratedTitleButton,
    is_maximized: bool,
    font: &Rc<LoadedFont>,
    metrics: &RenderMetrics,
    config: &ConfigHandle,
    titlebar_bg: LinearRgba,
) -> Element {
    let style = config.integrated_title_button_style;

    if style == Style::MacOsNative {
        return Element::new(font, ElementContent::Text(String::new()));
    }

    let poly = {
        let (close, hide, maximize, restore) = match style {
            Style::Windows => {
                use self::windows::{CLOSE, HIDE, MAXIMIZE, RESTORE};
                (CLOSE, HIDE, MAXIMIZE, RESTORE)
            }
            Style::Gnome => {
                use self::gnome::{CLOSE, HIDE, MAXIMIZE, RESTORE};
                (CLOSE, HIDE, MAXIMIZE, RESTORE)
            }
            Style::MacOsCustom => {
                // All three caps share the same filled-circle poly;
                // the color is picked at `window_button_colors` time.
                (
                    self::macos::DOT,
                    self::macos::DOT,
                    self::macos::DOT,
                    self::macos::DOT,
                )
            }
            Style::MacOsNative => unreachable!(),
        };
        let poly = match window_button {
            IntegratedTitleButton::Hide => hide,
            IntegratedTitleButton::Maximize => {
                if is_maximized {
                    restore
                } else {
                    maximize
                }
            }
            IntegratedTitleButton::Close => close,
        };

        match style {
            Style::Windows => self::windows::sized_poly(poly),
            Style::Gnome => self::gnome::sized_poly(poly),
            Style::MacOsCustom => self::macos::sized_poly(poly),
            Style::MacOsNative => unreachable!(),
        }
    };

    let element = Element::new(
        &font,
        ElementContent::Poly {
            line_width: metrics.underline_height.max(2),
            poly,
        },
    );

    let element = match style {
        Style::Windows => {
            let left_padding = match window_button {
                IntegratedTitleButton::Hide => 17.0,
                _ => 18.0,
            };
            let scale = 72.0 / 96.0;

            element
                .zindex(1)
                .vertical_align(VerticalAlign::Middle)
                .padding(BoxDimension {
                    left: Dimension::Points(left_padding * scale),
                    right: Dimension::Points(18. * scale),
                    top: Dimension::Points(10. * scale),
                    bottom: Dimension::Points(10. * scale),
                })
        }
        Style::Gnome => {
            let dim = Dimension::Pixels(7.);
            let border_corners_size = Dimension::Pixels(12.);
            element
                .zindex(1)
                .vertical_align(VerticalAlign::Middle)
                .padding(BoxDimension {
                    left: dim,
                    right: dim,
                    top: dim,
                    bottom: dim,
                })
                .border(BoxDimension::new(Dimension::Pixels(1.)))
                .border_corners(Some(Corners {
                    top_left: SizedPoly {
                        width: border_corners_size,
                        height: border_corners_size,
                        poly: TOP_LEFT_ROUNDED_CORNER,
                    },
                    top_right: SizedPoly {
                        width: border_corners_size,
                        height: border_corners_size,
                        poly: TOP_RIGHT_ROUNDED_CORNER,
                    },
                    bottom_left: SizedPoly {
                        width: border_corners_size,
                        height: border_corners_size,
                        poly: BOTTOM_LEFT_ROUNDED_CORNER,
                    },
                    bottom_right: SizedPoly {
                        width: border_corners_size,
                        height: border_corners_size,
                        poly: BOTTOM_RIGHT_ROUNDED_CORNER,
                    },
                }))
                .margin(BoxDimension {
                    left: dim,
                    right: dim,
                    top: dim,
                    bottom: dim,
                })
        }
        Style::MacOsCustom => {
            let side = ((ui_tokens::MACOS_TRAFFIC_LIGHT_RESERVE / 3.0
                - ui_tokens::MACOS_TRAFFIC_LIGHT_DOT)
                / 2.0)
                .max(0.0);
            element
                .zindex(1)
                .vertical_align(VerticalAlign::Middle)
                .padding(BoxDimension {
                    // Points, not Pixels, so the inter-dot gap scales with DPI
                    // exactly like the dot diameter (both derive from the same
                    // point-valued tokens) — otherwise the cluster stayed
                    // half-size on Retina.
                    left: Dimension::Points(side),
                    right: Dimension::Points(side),
                    top: Dimension::Points(0.),
                    bottom: Dimension::Points(0.),
                })
        }
        Style::MacOsNative => unreachable!(),
    };

    let foreground = config.integrated_title_button_color.clone();
    // Use the background that the title bar is actually painting this frame.
    // Reading `window_frame.active_titlebar_bg` here can be stale or defaulted
    // while a live light theme (Notion/Daylight) supplies a different palette,
    // which previously left white controls invisible on a near-white bar.
    let background_lightness = titlebar_lightness(titlebar_bg);

    let window_button_colors_fn = match style {
        Style::Windows => self::windows::window_button_colors,
        Style::Gnome => self::gnome::window_button_colors,
        Style::MacOsCustom => self::macos::window_button_colors,
        Style::MacOsNative => unreachable!(),
    };

    let colors = window_button_colors_fn(background_lightness, foreground, window_button);

    let element = element
        .item_type(UIItemType::TabBar(TabBarItem::WindowButton(window_button)))
        .colors(colors.colors)
        .hover_colors(Some(colors.hover_colors));

    element
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_controls_contrast_with_live_light_and_dark_titlebars() {
        let notion_light = LinearRgba::with_srgba(0xf7, 0xf6, 0xf3, 0xff);
        let midnight = LinearRgba::with_srgba(0x22, 0x28, 0x30, 0xff);

        let on_light = auto_button_color(
            titlebar_lightness(notion_light),
            IntegratedTitleButtonColor::Auto,
        );
        let on_dark = auto_button_color(
            titlebar_lightness(midnight),
            IntegratedTitleButtonColor::Auto,
        );

        assert!(on_light.0 < 0.01 && on_light.1 < 0.01 && on_light.2 < 0.01);
        assert!(on_dark.0 > 0.99 && on_dark.1 > 0.99 && on_dark.2 > 0.99);
    }
}
