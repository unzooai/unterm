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
            // 2.05× cell_height — combined with the stats bar's 1.15×
            // this lands at ~3.2× total, lining up with Warp's ~100 px
            // retina chrome instead of overshooting to 130 px.
            Ok((font.metrics().cell_height.get() as f32 * 2.05).ceil())
        } else {
            Ok(render_metrics.cell_size.height as f32)
        }
    }

    pub fn tab_bar_pixel_height(&self) -> anyhow::Result<f32> {
        // Returned value is the *total* chrome-above-panes height —
        // tab strip + per-pane stats bar. Other code paths already
        // treat it as "the height of everything between the OS chrome
        // and the panes", so folding the stats bar in here lets
        // every existing pane-layout site (resize, mouse mapping,
        // popups) shift the pane top down without separate threading.
        Ok(Self::tab_bar_pixel_height_impl(&self.config, &self.fonts, &self.render_metrics)?
            + self.top_stats_bar_pixel_height())
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
        // 1.15× — one line of text + tight padding. Combined with the
        // tab strip's 2.05× this caps total chrome at ~3.2× cell height
        // (~100 px retina at 13 pt), matching Warp's chrome posture.
        let font = match fonts.title_font() {
            Ok(f) => f,
            Err(_) => return 0.,
        };
        (font.metrics().cell_height.get() as f32 * 1.15).ceil()
    }
}
