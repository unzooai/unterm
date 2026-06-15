use crate::quad::TripleLayerQuadAllocator;
use crate::termwindow::render::RenderScreenLineParams;
use crate::utilsprites::RenderMetrics;
use config::ConfigHandle;
use mux::renderable::RenderableDimensions;
use wezterm_term::color::ColorAttribute;
use window::color::LinearRgba;

impl crate::TermWindow {
    pub fn paint_tab_bar(&mut self, layers: &mut TripleLayerQuadAllocator) -> anyhow::Result<()> {
        if self.config.use_fancy_tab_bar {
            if self.fancy_tab_bar.is_none() {
                let palette = self.palette().clone();
                let tab_bar = self.build_fancy_tab_bar(&palette)?;
                self.fancy_tab_bar.replace(tab_bar);
            }

            self.ui_items.append(&mut self.paint_fancy_tab_bar()?);
            return Ok(());
        }

        let border = self.get_os_border();

        let palette = self.palette().clone();
        let tab_bar_height = self.tab_bar_pixel_height()?;
        let tab_bar_y = if self.config.tab_bar_at_bottom {
            ((self.dimensions.pixel_height as f32) - (tab_bar_height + border.bottom.get() as f32))
                .max(0.)
        } else {
            border.top.get() as f32
        };

        // Register the tab bar location
        self.ui_items.append(&mut self.tab_bar.compute_ui_items(
            tab_bar_y as usize,
            self.render_metrics.cell_size.height as usize,
            self.render_metrics.cell_size.width as usize,
        ));

        let window_is_transparent =
            !self.window_background.is_empty() || self.config.window_background_opacity != 1.0;
        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();
        let default_bg = palette
            .resolve_bg(ColorAttribute::Default)
            .to_linear()
            .mul_alpha(if window_is_transparent {
                0.
            } else {
                self.config.text_background_opacity
            });

        self.render_screen_line(
            RenderScreenLineParams {
                top_pixel_y: tab_bar_y,
                left_pixel_x: 0.,
                pixel_width: self.dimensions.pixel_width as f32,
                stable_line_idx: None,
                line: self.tab_bar.line(),
                selection: 0..0,
                cursor: &Default::default(),
                palette: &palette,
                dims: &RenderableDimensions {
                    cols: self.dimensions.pixel_width
                        / self.render_metrics.cell_size.width as usize,
                    physical_top: 0,
                    scrollback_rows: 0,
                    scrollback_top: 0,
                    viewport_rows: 1,
                    dpi: self.terminal_size.dpi,
                    pixel_height: self.render_metrics.cell_size.height as usize,
                    pixel_width: self.terminal_size.pixel_width,
                    reverse_video: false,
                },
                config: &self.config,
                cursor_border_color: LinearRgba::default(),
                foreground: palette.foreground.to_linear(),
                pane: None,
                is_active: true,
                selection_fg: LinearRgba::default(),
                selection_bg: LinearRgba::default(),
                cursor_fg: LinearRgba::default(),
                cursor_bg: LinearRgba::default(),
                cursor_is_default_color: true,
                white_space,
                filled_box,
                window_is_transparent,
                default_bg,
                style: None,
                font: None,
                use_pixel_positioning: self.config.experimental_pixel_positioning,
                render_metrics: self.render_metrics,
                shape_key: None,
                password_input: false,
            },
            layers,
        )?;

        Ok(())
    }

    pub fn tab_bar_pixel_height_impl(
        config: &ConfigHandle,
        fontconfig: &wezterm_font::FontConfiguration,
        render_metrics: &RenderMetrics,
    ) -> anyhow::Result<f32> {
        if config.use_fancy_tab_bar {
            let font = fontconfig.title_font()?;
            // 1.0× cell_height (~31 px @ 144 dpi 13 pt). Tightened from
            // 1.6× because macOS anchors the integrated traffic lights
            // to a fixed y-offset from the window's top edge (~14 px on
            // current macOS). At 1.6× the chrome was 50 px and the
            // lights ended up at y=14–28 with a visible 22 px dead band
            // underneath; the user diagnosed it as "边框压了顶栏" — the
            // chrome was carrying empty bottom padding the OS lights
            // couldn't fill. 1.0× brings the chrome down to roughly the
            // lights' natural row so codicons sit at the same y as the
            // lights, and the sidebar's first tab lands flush against
            // the chrome's bottom edge instead of below a gap.
            Ok((font.metrics().cell_height.get() as f32 * 1.0).ceil())
        } else {
            Ok(render_metrics.cell_size.height as f32)
        }
    }

    pub fn tab_bar_pixel_height(&self) -> anyhow::Result<f32> {
        // Stats are now rendered INSIDE the tab bar's middle gap (so
        // they sit on the same row as the traffic lights / icons,
        // vertically centered), not as a separate strip beneath the
        // bar. No extra height contribution from the stats bar.
        Self::tab_bar_pixel_height_impl(&self.config, &self.fonts, &self.render_metrics)
    }

    /// Height of the per-pane stats strip (git / cpu / tokens / last
    /// command). 0 when disabled or in classic-tab-bar mode (the
    /// strip is rendered as part of the integrated chrome only).
    pub fn top_stats_bar_pixel_height(&self) -> f32 {
        if !self.config.use_fancy_tab_bar || !self.show_tab_bar {
            return 0.;
        }
        Self::top_stats_bar_pixel_height_impl(&self.fonts)
    }

    /// Static height computation for code paths that don't yet have a
    /// `&self` (startup mux setup before TermWindow exists). Same
    /// formula as the instance method.
    pub fn top_stats_bar_pixel_height_impl(
        fonts: &wezterm_font::FontConfiguration,
    ) -> f32 {
        // 1.0× — just one line of text, no extra padding. Combined
        // with the tab strip's 1.8× this caps total chrome at ~2.8×
        // cell height, tighter than Warp.
        let font = match fonts.title_font() {
            Ok(f) => f,
            Err(_) => return 0.,
        };
        (font.metrics().cell_height.get() as f32 * 1.0).ceil()
    }
}
