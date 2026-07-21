use crate::quad::TripleLayerQuadAllocator;
use crate::termwindow::render::RenderScreenLineParams;
use crate::termwindow::{UIItem, UIItemType};
use config::ui_tokens;
use mux::renderable::RenderableDimensions;
use termwiz::cell::{unicode_column_width, CellAttributes};
use termwiz::color::SrgbaTuple;
use termwiz::surface::line::Line;
use wezterm_term::color::ColorAttribute;
use window::color::LinearRgba;
use window::WindowOps;

static DEFER_FIRST_STATUS_TEXT_RENDER: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

impl crate::TermWindow {
    pub(crate) fn show_ui_notice(&self, message: impl Into<String>) {
        let expires = std::time::Instant::now() + std::time::Duration::from_millis(2400);
        self.ui_notice.replace(Some((message.into(), expires)));
        self.update_next_frame_time(Some(expires));
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    fn active_ui_notice(&self) -> Option<String> {
        let now = std::time::Instant::now();
        let mut notice = self.ui_notice.borrow_mut();
        match notice.as_ref() {
            Some((message, expires)) if *expires > now => Some(message.clone()),
            Some(_) => {
                notice.take();
                None
            }
            None => None,
        }
    }

    fn status_bar_vertical_padding_px(&self) -> f32 {
        let pt = self.dimensions.dpi as f32 / 72.0;
        (ui_tokens::STATUS_BAR_VERTICAL_PADDING * pt)
            .round()
            .max(2.0)
    }

    pub fn status_bar_pixel_height_for_cell_height(&self, cell_height: f32) -> f32 {
        if self.config.show_unterm_status_bar {
            cell_height + self.status_bar_vertical_padding_px() * 2.0
        } else {
            0.0
        }
    }

    /// Height of the status bar in pixels.
    pub fn status_bar_pixel_height(&self) -> f32 {
        self.status_bar_pixel_height_for_cell_height(self.render_metrics.cell_size.height as f32)
    }

    fn status_bar_text_top_pixel_y(&self, bar_y: f32, bar_height: f32) -> f32 {
        let cell_height = self.render_metrics.cell_size.height as f32;
        let pt = self.dimensions.dpi as f32 / 72.0;
        let nudge = ui_tokens::CHROME_TEXT_BASELINE_NUDGE * pt;
        (bar_y + ((bar_height - cell_height) * 0.5).round() + nudge).max(bar_y)
    }

    /// Height of the MCP banner row in pixels. Zero unless either a
    /// pending confirmation OR a pending suggestion is showing for
    /// the active pane. The pane layout subtracts this so the banner
    /// doesn't overlap the terminal output.
    pub fn suggest_bar_pixel_height(&self) -> f32 {
        if crate::mcp::handler::pending_confirmation_count() > 0
            || self.active_pane_first_pending_suggestion().is_some()
        {
            self.render_metrics.cell_size.height as f32
        } else {
            0.0
        }
    }

    fn active_pane_first_pending_suggestion(&self) -> Option<crate::mcp::handler::Suggestion> {
        let pane = self.get_active_pane_no_overlay()?;
        crate::mcp::handler::pending_suggestions_for_pane(pane.pane_id() as u64)
            .into_iter()
            .next()
    }

    /// Render a one-row banner above the status bar. Two modes share
    /// the row, with confirmation winning when both are pending:
    /// * MCP **confirmation** banner — a worker thread is parked
    ///   waiting for the user to allow/block a PTY-writing call.
    /// * MCP **suggestion** bar — non-blocking proposal the user can
    ///   accept (Tab/Alt+Enter) or dismiss (Esc).
    pub fn paint_suggest_bar(
        &mut self,
        layers: &mut TripleLayerQuadAllocator,
    ) -> anyhow::Result<()> {
        if let Some(view) = crate::mcp::handler::pending_confirmation_view() {
            return self.paint_confirmation_banner(layers, &view);
        }
        let Some(suggestion) = self.active_pane_first_pending_suggestion() else {
            return Ok(());
        };

        let cell_height = self.render_metrics.cell_size.height as f32;
        let cell_width = self.render_metrics.cell_size.width as f32;
        let border = self.get_os_border();

        let bar_height = self.suggest_bar_pixel_height();
        if bar_height <= 0.0 {
            return Ok(());
        }
        // Sit directly above the status bar — single row, full width.
        let status_height = self.status_bar_pixel_height();
        let bar_y = self.dimensions.pixel_height as f32
            - status_height
            - bar_height
            - border.bottom.get() as f32;
        let bar_width = self.dimensions.pixel_width as f32;

        // Distinct accent palette so the suggest bar visually reads as
        // "AI is asking" rather than blending into the chrome.
        let theme = crate::overlay::theme_selector::read_theme_id();
        let (bar_bg_rgb, sep_rgb, fg_rgb) = suggest_bar_theme_colors(&theme);
        let bar_bg = LinearRgba::with_components(
            bar_bg_rgb.0 as f32 / 255.0,
            bar_bg_rgb.1 as f32 / 255.0,
            bar_bg_rgb.2 as f32 / 255.0,
            0.92,
        );

        self.filled_rectangle(
            layers,
            0,
            euclid::rect(0., bar_y, bar_width, bar_height),
            bar_bg,
        )?;

        let sep_color = LinearRgba::with_components(
            sep_rgb.0 as f32 / 255.0,
            sep_rgb.1 as f32 / 255.0,
            sep_rgb.2 as f32 / 255.0,
            1.0,
        );
        self.filled_rectangle(
            layers,
            0,
            euclid::rect(0., bar_y, bar_width, 1.0),
            sep_color,
        )?;

        // Compose the bar text. Newlines in the suggestion text are
        // collapsed to `␤` so a multi-line snippet still renders on
        // one row — the full text appears once the user accepts and
        // it actually lands in the PTY.
        let agent = if suggestion.posted_by_agent == "anonymous" {
            "agent".to_string()
        } else {
            suggestion.posted_by_agent.clone()
        };
        let one_line: String = suggestion
            .text
            .chars()
            .map(|c| match c {
                '\n' => '␤',
                '\r' => '␍',
                '\t' => ' ',
                c if c.is_control() => '·',
                c => c,
            })
            .collect();
        let main = format!(" ✨ {}: {}", agent, one_line);
        let hint = "  [Tab] accept   [Esc] dismiss   [Alt+Enter] accept & run ";

        // Truncate the main segment so the hint always fits.
        let total_cols = (bar_width / cell_width) as usize;
        let hint_cols = unicode_column_width(hint, None);
        let max_main = total_cols.saturating_sub(hint_cols + 2);
        let truncated_main = truncate_to_width(&main, max_main);

        let mut text = String::new();
        text.push_str(&truncated_main);
        let pad_cols = total_cols.saturating_sub(unicode_column_width(&text, None) + hint_cols);
        for _ in 0..pad_cols {
            text.push(' ');
        }
        text.push_str(hint);

        let mut attrs = CellAttributes::blank();
        attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            fg_rgb.0 as f32 / 255.0,
            fg_rgb.1 as f32 / 255.0,
            fg_rgb.2 as f32 / 255.0,
            1.0,
        )));

        let palette = self.palette().clone();
        let window_is_transparent =
            !self.window_background.is_empty() || self.config.window_background_opacity != 1.0;
        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();
        let fg = LinearRgba::with_components(
            fg_rgb.0 as f32 / 255.0,
            fg_rgb.1 as f32 / 255.0,
            fg_rgb.2 as f32 / 255.0,
            1.0,
        );

        let line = Line::from_text(&text, &attrs, 0, None);

        self.render_screen_line(
            RenderScreenLineParams {
                top_pixel_y: self.status_bar_text_top_pixel_y(bar_y, bar_height),
                left_pixel_x: 0.0,
                pixel_width: bar_width,
                stable_line_idx: None,
                line: &line,
                selection: 0..0,
                cursor: &Default::default(),
                palette: &palette,
                dims: &RenderableDimensions {
                    cols: total_cols,
                    physical_top: 0,
                    scrollback_rows: 0,
                    scrollback_top: 0,
                    viewport_rows: 1,
                    dpi: self.terminal_size.dpi,
                    pixel_height: cell_height as usize,
                    pixel_width: bar_width as usize,
                    reverse_video: false,
                },
                config: &self.config,
                cursor_border_color: LinearRgba::default(),
                foreground: fg,
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
                default_bg: bar_bg,
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

    pub fn paint_status_bar(
        &mut self,
        layers: &mut TripleLayerQuadAllocator,
    ) -> anyhow::Result<()> {
        if !self.config.show_unterm_status_bar {
            return Ok(());
        }

        let cell_height = self.render_metrics.cell_size.height as f32;
        let cell_width = self.render_metrics.cell_size.width as f32;
        let border = self.get_os_border();

        let bar_height = self.status_bar_pixel_height();
        let bar_y = self.dimensions.pixel_height as f32 - bar_height - border.bottom.get() as f32;
        let bar_width = self.dimensions.pixel_width as f32;

        let theme = crate::overlay::theme_selector::read_theme_id();
        let (_bar_bg_rgb, sep_rgb, _fg_rgb) = status_bar_theme_colors(&theme);
        // Scheme A: match the bottom bar to the shared chrome tone (sidebar +
        // top bar) so the sidebar↔bottom-bar corner meets seamlessly instead
        // of stepping to a slightly different grey.
        let pal = self.palette().clone();
        let chrome = crate::termwindow::chrome_colors::sidebar(
            pal.background.to_linear(),
            pal.foreground.to_linear(),
        );
        let bar_bg = chrome.surface;

        // Draw the bar background on layer 1 (not 0) so it fully occludes
        // everything beneath it at the bottom of the window — in particular
        // the faint 1px seam where two adjacent pane backgrounds overlap by
        // half a cell at the split column. On layer 0 that seam bleeds up
        // through the status-bar text ("t|heme:classic"); a layer-1 fill
        // composites cleanly over it. The bar text renders after, on top.
        self.filled_rectangle(
            layers,
            1,
            euclid::rect(0., bar_y, bar_width, bar_height),
            bar_bg,
        )?;

        // Close the bottom of the shared chrome frame with the same subtle
        // optical edge used by the title bar and sidebar.
        let _ = sep_rgb;
        let pt = self.dimensions.dpi as f32 / 72.0;
        let divider = chrome.outer_edge;
        self.filled_rectangle(
            layers,
            2,
            euclid::rect(0., bar_y, bar_width, (1.0 * pt).max(1.0)),
            divider,
        )?;

        if DEFER_FIRST_STATUS_TEXT_RENDER.swap(false, std::sync::atomic::Ordering::AcqRel) {
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
            return Ok(());
        }

        let (line, regions) = self.build_status_line(&theme);
        let total_cols = (bar_width / cell_width) as usize;

        let palette = self.palette().clone();
        let window_is_transparent =
            !self.window_background.is_empty() || self.config.window_background_opacity != 1.0;
        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();

        let fg = pal.foreground.to_linear();

        self.render_screen_line(
            RenderScreenLineParams {
                top_pixel_y: self.status_bar_text_top_pixel_y(bar_y, bar_height),
                left_pixel_x: 0.0,
                pixel_width: bar_width,
                stable_line_idx: None,
                line: &line,
                selection: 0..0,
                cursor: &Default::default(),
                palette: &palette,
                dims: &RenderableDimensions {
                    cols: total_cols,
                    physical_top: 0,
                    scrollback_rows: 0,
                    scrollback_top: 0,
                    viewport_rows: 1,
                    dpi: self.terminal_size.dpi,
                    pixel_height: cell_height as usize,
                    pixel_width: bar_width as usize,
                    reverse_video: false,
                },
                config: &self.config,
                cursor_border_color: LinearRgba::default(),
                foreground: fg,
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
                default_bg: bar_bg,
                style: None,
                font: None,
                use_pixel_positioning: self.config.experimental_pixel_positioning,
                render_metrics: self.render_metrics,
                shape_key: None,
                password_input: false,
            },
            layers,
        )?;

        for region in regions {
            self.ui_items.push(UIItem {
                x: (region.offset as f32 * cell_width) as usize,
                y: bar_y as usize,
                width: (region.len as f32 * cell_width) as usize,
                height: bar_height as usize,
                pane_id: None,
                item_type: region.item_type,
            });
        }

        Ok(())
    }

    /// Render the MCP **confirmation** banner — a worker thread is
    /// blocked waiting for the user to allow/block this. Same row +
    /// width as the suggestion bar but uses a warning palette so the
    /// user notices a *blocking* prompt vs a passive suggestion.
    fn paint_confirmation_banner(
        &mut self,
        layers: &mut TripleLayerQuadAllocator,
        view: &crate::mcp::handler::ConfirmationView,
    ) -> anyhow::Result<()> {
        let cell_height = self.render_metrics.cell_size.height as f32;
        let cell_width = self.render_metrics.cell_size.width as f32;
        let border = self.get_os_border();

        let bar_height = cell_height;
        let status_height = self.status_bar_pixel_height();
        let bar_y = self.dimensions.pixel_height as f32
            - status_height
            - bar_height
            - border.bottom.get() as f32;
        let bar_width = self.dimensions.pixel_width as f32;

        let theme = crate::overlay::theme_selector::read_theme_id();
        let (bar_bg_rgb, sep_rgb, fg_rgb) = confirm_banner_theme_colors(&theme);
        let bar_bg = LinearRgba::with_components(
            bar_bg_rgb.0 as f32 / 255.0,
            bar_bg_rgb.1 as f32 / 255.0,
            bar_bg_rgb.2 as f32 / 255.0,
            0.96,
        );

        self.filled_rectangle(
            layers,
            0,
            euclid::rect(0., bar_y, bar_width, bar_height),
            bar_bg,
        )?;

        let sep_color = LinearRgba::with_components(
            sep_rgb.0 as f32 / 255.0,
            sep_rgb.1 as f32 / 255.0,
            sep_rgb.2 as f32 / 255.0,
            1.0,
        );
        self.filled_rectangle(
            layers,
            0,
            euclid::rect(0., bar_y, bar_width, 1.0),
            sep_color,
        )?;

        // Compose banner text.
        let agent_label = if view.agent == "anonymous" {
            "agent".to_string()
        } else {
            view.agent.clone()
        };
        let main = format!(
            " ⚠ {} wants to write to pane #{}: {}",
            agent_label, view.pane_id, view.input_preview
        );
        let hint = "  [Enter] allow   [Esc] block   [Alt+A] always allow ";

        let total_cols = (bar_width / cell_width) as usize;
        let hint_cols = unicode_column_width(hint, None);
        let max_main = total_cols.saturating_sub(hint_cols + 2);
        let truncated_main = truncate_to_width(&main, max_main);

        let mut text = String::new();
        text.push_str(&truncated_main);
        let pad_cols = total_cols.saturating_sub(unicode_column_width(&text, None) + hint_cols);
        for _ in 0..pad_cols {
            text.push(' ');
        }
        text.push_str(hint);

        let mut attrs = CellAttributes::blank();
        attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            fg_rgb.0 as f32 / 255.0,
            fg_rgb.1 as f32 / 255.0,
            fg_rgb.2 as f32 / 255.0,
            1.0,
        )));

        let palette = self.palette().clone();
        let window_is_transparent =
            !self.window_background.is_empty() || self.config.window_background_opacity != 1.0;
        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();
        let fg = LinearRgba::with_components(
            fg_rgb.0 as f32 / 255.0,
            fg_rgb.1 as f32 / 255.0,
            fg_rgb.2 as f32 / 255.0,
            1.0,
        );

        let line = Line::from_text(&text, &attrs, 0, None);

        self.render_screen_line(
            RenderScreenLineParams {
                top_pixel_y: self.status_bar_text_top_pixel_y(bar_y, bar_height),
                left_pixel_x: 0.0,
                pixel_width: bar_width,
                stable_line_idx: None,
                line: &line,
                selection: 0..0,
                cursor: &Default::default(),
                palette: &palette,
                dims: &RenderableDimensions {
                    cols: total_cols,
                    physical_top: 0,
                    scrollback_rows: 0,
                    scrollback_top: 0,
                    viewport_rows: 1,
                    dpi: self.terminal_size.dpi,
                    pixel_height: cell_height as usize,
                    pixel_width: bar_width as usize,
                    reverse_video: false,
                },
                config: &self.config,
                cursor_border_color: LinearRgba::default(),
                foreground: fg,
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
                default_bg: bar_bg,
                style: None,
                font: None,
                use_pixel_positioning: self.config.experimental_pixel_positioning,
                render_metrics: self.render_metrics,
                shape_key: None,
                password_input: false,
            },
            layers,
        )?;

        // Keep repainting while the banner is up so any slow-arriving
        // animation can register; otherwise the banner will sit
        // statically (acceptable) until the user keys something.
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }

        Ok(())
    }

    fn build_status_line(&self, theme: &str) -> (Line, Vec<StatusRegion>) {
        // Status bar text color
        let mut attrs = CellAttributes::blank();
        let (_, _, fg_rgb) = status_bar_theme_colors(theme);
        let status_fg: SrgbaTuple = self
            .config
            .resolved_palette
            .foreground
            .map(Into::into)
            .unwrap_or(SrgbaTuple(
                fg_rgb.0 as f32 / 255.0,
                fg_rgb.1 as f32 / 255.0,
                fg_rgb.2 as f32 / 255.0,
                1.0,
            ));
        attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(status_fg));

        if let Some(message) = self.active_ui_notice() {
            let total_cols = (self.dimensions.pixel_width as f32
                / self.render_metrics.cell_size.width.max(1) as f32)
                .floor() as usize;
            let notice = truncate_to_width(&format!("  ✓ {message}"), total_cols);
            return (Line::from_text(&notice, &attrs, 0, None), Vec::new());
        }

        let active_pane = self.get_active_pane_no_overlay();
        let active_cwd = active_pane
            .as_ref()
            .and_then(|pane| pane.get_current_working_dir(mux::pane::CachePolicy::AllowStale));

        // 1. Shell type (with version distinction)
        let shell_name = if let Some(pane) = active_pane.as_ref() {
            if let Some(name) = pane.get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            {
                let lower = name.to_lowercase();
                if lower.contains("pwsh") {
                    "pwsh 7".to_string()
                } else if lower.contains("powershell") {
                    "pwsh 5.1".to_string()
                } else if lower.contains("cmd") {
                    "cmd".to_string()
                } else if lower.contains("nu") {
                    "nu".to_string()
                } else if lower.contains("wsl") {
                    "wsl".to_string()
                } else if lower.contains("bash") {
                    if cfg!(windows) && lower.starts_with("/") {
                        "bash (wsl)".to_string()
                    } else {
                        "bash".to_string()
                    }
                } else if lower.contains("zsh") {
                    if cfg!(windows) && lower.starts_with("/") {
                        "zsh (wsl)".to_string()
                    } else {
                        "zsh".to_string()
                    }
                } else if lower.contains("fish") {
                    if cfg!(windows) && lower.starts_with("/") {
                        "fish (wsl)".to_string()
                    } else {
                        "fish".to_string()
                    }
                } else {
                    "shell".to_string()
                }
            } else {
                "shell".to_string()
            }
        } else {
            "shell".to_string()
        };

        // 2. Terminal size
        let cols = self.terminal_size.cols;
        let rows = self.terminal_size.rows;
        let total_cols = (self.dimensions.pixel_width as f32
            / self.render_metrics.cell_size.width.max(1) as f32)
            .floor() as usize;
        let density = status_bar_density(total_cols);

        // Hidden segments are zero-cost. In particular, avoid the profile
        // registry read and proxy-file refresh on compact windows where those
        // values cannot be displayed.
        let proxy = if !density.show_telemetry {
            String::new()
        } else if unterm_proxy_enabled() {
            "proxy:on".to_string()
        } else {
            "proxy:off".to_string()
        };
        // MCP activity chip. Always rendered so the position is stable
        // (a chip that appears/disappears would shift every neighboring
        // segment's click hit-test). `⚡` suffix marks "writes recently"
        // so the user notices a flash without having to compare counts.
        let mcp_part = if density.show_telemetry {
            let mcp_activity = crate::mcp::handler::recent_mcp_input_activity();
            let flash = mcp_activity
                .seconds_since_last
                .map(|s| s < 5.0)
                .unwrap_or(false);
            format!(
                "mcp:{}{}",
                mcp_activity.count,
                if flash { "⚡" } else { "" }
            )
        } else {
            String::new()
        };

        // Identity profile chip (window=identity model). The chip lives
        // in the bottom status bar rather than the top tabbar so it
        // shares space with the other context indicators (cwd / project /
        // theme); they're all "current window state" signals that the
        // user wants to see at a glance. Display `—` when no profile
        // is bound so the chip stays visible but visibly empty —
        // important because the click action (cycle profile) doubles
        // as the "I haven't set up profiles yet, what's this?"
        // discoverability hook.
        let profile_part = if density.show_profile {
            let profile_label = crate::termwindow::sidebar_text::ellipsize_middle(
                &current_profile_display_name(),
                18,
            );
            format!("profile:{profile_label}")
        } else {
            String::new()
        };

        let project_label = crate::termwindow::sidebar_text::ellipsize_middle(
            &Self::project_label_for_cwd(active_cwd.as_ref()),
            18,
        );
        let project_part = format!("project:{}", project_label);

        let cwd_part = crate::termwindow::sidebar_text::ellipsize_middle(
            &Self::cwd_for_status(active_cwd.as_ref()),
            density.cwd_cols,
        );

        let mut text = format!(" {}", shell_name);
        let mut regions = vec![];
        let cwd_offset = append_status_part(
            &mut text,
            &mut regions,
            &cwd_part,
            Some(UIItemType::StatusBarCwd),
        );
        append_status_part(&mut text, &mut regions, &format!("{}x{}", cols, rows), None);
        let project_offset = density.show_project.then(|| {
            append_status_part(
                &mut text,
                &mut regions,
                &project_part,
                Some(UIItemType::StatusBarProject),
            )
        });
        let exclude_part = "capture:exclude".to_string();
        let include_part = "capture:include".to_string();
        let exclude_offset = density.show_capture.then(|| {
            append_status_part(
                &mut text,
                &mut regions,
                &exclude_part,
                Some(UIItemType::StatusBarCaptureExclude),
            )
        });
        let include_offset = density.show_capture.then(|| {
            append_status_part(
                &mut text,
                &mut regions,
                &include_part,
                Some(UIItemType::StatusBarCaptureInclude),
            )
        });
        let proxy_offset = density.show_telemetry.then(|| {
            append_status_part(
                &mut text,
                &mut regions,
                &proxy,
                Some(UIItemType::StatusBarProxy),
            )
        });
        let mcp_offset = density.show_telemetry.then(|| {
            append_status_part(
                &mut text,
                &mut regions,
                &mcp_part,
                Some(UIItemType::StatusBarMcpAudit),
            )
        });
        let theme_part = format!("theme:{theme}");
        let theme_offset = density.show_theme.then(|| {
            append_status_part(
                &mut text,
                &mut regions,
                &theme_part,
                Some(UIItemType::StatusBarTheme),
            )
        });
        let profile_offset = density.show_profile.then(|| {
            append_status_part(
                &mut text,
                &mut regions,
                &profile_part,
                Some(UIItemType::StatusBarProfile),
            )
        });
        text.push(' ');

        let mut line = Line::from_text(&text, &attrs, 0, None);
        {
            // 分层排版:标签保持基础暗色,值用主题前景提亮;proxy:on 与
            // mcp 活动值用品牌 teal 点睛(2026-06-10 状态栏精修)。
            let bright_color: SrgbaTuple = self
                .config
                .resolved_palette
                .foreground
                .map(|c| c.into())
                .unwrap_or(SrgbaTuple::from((0xe6u8, 0xe6u8, 0xe4u8)));
            // Palette-derived accent: ANSI bright cyan (index 14 =
            // brights[6]) so the actionable chips pick up whatever the
            // scheme defines for "bright cyan" instead of a hardcoded
            // #6fccb8 — keeps Solarized Light's warm-teal, Tokyo
            // Night's icy blue-cyan, etc., in sync with the rest of
            // the chrome.
            let teal: SrgbaTuple = self
                .config
                .resolved_palette
                .brights
                .and_then(|b| b.get(6).copied())
                .map(|c| c.into())
                .unwrap_or_else(|| SrgbaTuple::from((0x6fu8, 0xccu8, 0xb8u8)));
            let mk = |c: SrgbaTuple| {
                let mut a = attrs.clone();
                a.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(c));
                a
            };
            let bright = mk(bright_color);
            let teal_a = mk(teal);
            let mut paint = |offset: usize, txt: &str, a: &CellAttributes| {
                line.overlay_text_with_attribute(offset, txt, a.clone(), 0);
            };
            // shell 名 + cwd 提亮(主信息)
            paint(1, &shell_name, &bright);
            // cwd is clickable (copy on left-click) — teal flag it as actionable.
            paint(cwd_offset, &cwd_part, &teal_a);
            // Status bar values fall into three tiers now:
            //   bright   — read-only display ("89x32", shell name)
            //   teal     — clickable affordance (everything with a UI handler:
            //              project, capture modes, theme, profile, cwd)
            //   active   — currently-on toggles (proxy:on, mcp count > 0)
            //              kept as teal so the chip still pops, but the same
            //              hue as plain clickables so the color difference
            //              doesn't read as a category change.
            // This gives a single uniform cue for "you can click this".
            let val = |part: &str| -> (usize, String) {
                match part.find(':') {
                    Some(i) => (i + 1, part[i + 1..].to_string()),
                    None => (0, part.to_string()),
                }
            };
            let mut paint_value = |offset: Option<usize>, part: &str| {
                if let Some(offset) = offset {
                    let (value_offset, value) = val(part);
                    paint(offset + value_offset, &value, &teal_a);
                }
            };
            paint_value(project_offset, &project_part);
            paint_value(exclude_offset, &exclude_part);
            paint_value(include_offset, &include_part);
            paint_value(proxy_offset, &proxy);
            paint_value(mcp_offset, &mcp_part);
            paint_value(theme_offset, &theme_part);
            let (_, profile_value) = val(&profile_part);
            if profile_value != "—" {
                paint_value(profile_offset, &profile_part);
            }
        }
        (line, regions)
    }

    /// Active pane's cwd, formatted for the bottom status bar:
    ///   - Resolved to a local path when possible (drops the `file://` scheme
    ///     and the host component for remote URIs we can't visit anyway).
    ///   - $HOME prefix replaced with `~` so common project paths
    ///     stay short.
    ///   - Truncated to ~48 display columns by elision in the *middle*
    ///     (`/Users/me/code/.../wezterm-gui/src`) — keeps both project
    ///     root context and current-directory tail visible.
    fn cwd_for_status(cwd: Option<&url::Url>) -> String {
        let raw: Option<String> = cwd.map(|cwd| {
            // OSC 7 carries the hostname; on the local machine that's
            // typically "localhost", but multiplexer-mode and remote
            // panes (Linux container / SSH host) report a real
            // hostname like "ubuntu". For those we want just the path
            // component (`/home/alexlee`), not `file://ubuntu/home/...`.
            //
            // Check for a real remote host BEFORE `to_file_path()`: on
            // Windows that call treats `file://ubuntu/home` as the UNC
            // path `\\ubuntu\home` and succeeds, so the host would
            // survive as `//ubuntu/home` after backslash normalization.
            // Only fall through to `to_file_path()` for local URLs.
            let has_remote_host = cwd
                .host_str()
                .map_or(false, |h| !h.is_empty() && h != "localhost");
            if !has_remote_host {
                if let Ok(p) = cwd.to_file_path() {
                    return p.display().to_string();
                }
            }
            let s = cwd.as_str();
            s.strip_prefix("file://")
                .and_then(|rest| {
                    rest.split_once('/')
                        .map(|(_host, path)| format!("/{}", path))
                })
                .unwrap_or_else(|| s.to_string())
        });
        let Some(path) = raw else {
            return "~".to_string();
        };

        // Normalize Windows backslashes to forward slashes for display
        // consistency. The on-disk path doesn't change; this is purely UI.
        let display = path.replace('\\', "/");

        let home = dirs_next::home_dir()
            .and_then(|h| Some(h.display().to_string().replace('\\', "/")))
            .unwrap_or_default();
        let with_tilde = if !home.is_empty() && display.starts_with(&home) {
            format!("~{}", &display[home.len()..])
        } else {
            display
        };

        // Truncate by display *width* (CJK chars are 2 cells), not byte
        // length. Aim for ~48 columns; if longer, keep first 24 and last
        // 20 with " ... " in the middle.
        const MAX: usize = 48;
        let width = unicode_column_width(&with_tilde, None);
        if width <= MAX {
            return with_tilde;
        }
        let chars: Vec<char> = with_tilde.chars().collect();
        let head: String = chars.iter().take(24).collect();
        let mut tail_chars: Vec<char> = chars.iter().rev().take(20).copied().collect();
        tail_chars.reverse();
        let tail: String = tail_chars.into_iter().collect();
        format!("{} ... {}", head, tail)
    }

    fn project_label_for_cwd(cwd: Option<&url::Url>) -> String {
        let Some(cwd) = cwd else {
            return "~".to_string();
        };
        if let Ok(path) = cwd.to_file_path() {
            return path
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| path.display().to_string());
        }
        cwd.as_str()
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or(cwd.as_str())
            .to_string()
    }
}

struct StatusRegion {
    offset: usize,
    len: usize,
    item_type: UIItemType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusBarDensity {
    cwd_cols: usize,
    show_project: bool,
    show_theme: bool,
    show_telemetry: bool,
    show_profile: bool,
    show_capture: bool,
}

fn status_bar_density(total_cols: usize) -> StatusBarDensity {
    StatusBarDensity {
        cwd_cols: match total_cols {
            0..=79 => 18,
            80..=111 => 26,
            112..=143 => 34,
            _ => 48,
        },
        show_project: total_cols >= 72,
        show_theme: total_cols >= 96,
        show_telemetry: total_cols >= 128,
        show_profile: total_cols >= 160,
        show_capture: total_cols >= 208,
    }
}

fn append_status_part(
    text: &mut String,
    regions: &mut Vec<StatusRegion>,
    part: &str,
    item_type: Option<UIItemType>,
) -> usize {
    text.push_str("   ");
    let offset = unicode_column_width(text, None);
    text.push_str(part);
    if let Some(item_type) = item_type {
        regions.push(StatusRegion {
            offset,
            len: unicode_column_width(part, None),
            item_type,
        });
    }
    offset
}

#[derive(Default)]
struct ProxyStatusCache {
    value: bool,
    loaded_at: Option<std::time::Instant>,
    loading: bool,
}

#[derive(Default)]
struct ProfileDisplayCache {
    value: String,
    loaded_at: Option<std::time::Instant>,
    loading: bool,
}

const PROFILE_DISPLAY_TTL: std::time::Duration = std::time::Duration::from_secs(5);
const PROXY_STATUS_TTL: std::time::Duration = std::time::Duration::from_secs(2);

lazy_static::lazy_static! {
    static ref PROXY_STATUS_CACHE: std::sync::Mutex<ProxyStatusCache> =
        std::sync::Mutex::new(ProxyStatusCache::default());
    static ref PROFILE_DISPLAY_CACHE: std::sync::Mutex<ProfileDisplayCache> =
        std::sync::Mutex::new(ProfileDisplayCache::default());
}

fn unterm_proxy_enabled() -> bool {
    {
        let mut cache = PROXY_STATUS_CACHE.lock().unwrap();
        if cache
            .loaded_at
            .map(|at| at.elapsed() < PROXY_STATUS_TTL)
            .unwrap_or(false)
            || cache.loading
        {
            return cache.value;
        }
        cache.loading = true;
    }

    if std::thread::Builder::new()
        .name("proxy-status-refresh".into())
        .spawn(|| {
            let refreshed = std::panic::catch_unwind(read_unterm_proxy_enabled);
            let mut cache = PROXY_STATUS_CACHE.lock().unwrap();
            if let Ok(enabled) = refreshed {
                cache.value = enabled;
                cache.loaded_at = Some(std::time::Instant::now());
            }
            cache.loading = false;
        })
        .is_err()
    {
        PROXY_STATUS_CACHE.lock().unwrap().loading = false;
    }

    PROXY_STATUS_CACHE.lock().unwrap().value
}

fn read_unterm_proxy_enabled() -> bool {
    let path = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("proxy.json");
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return false;
    };
    value
        .get("enabled")
        .and_then(|enabled| enabled.as_bool())
        .unwrap_or(false)
}

/// Display name of the profile this window is bound to, suffixed with
/// `⚠` if any of the profile's tracked secrets expires within 7 days.
/// Returns `—` when no profile is set.
///
/// Read off the per-instance JSON so the chip reflects the same value
/// `apply_unterm_profile_env` uses at spawn time — never a stale
/// registry-default. Failures are silent: a missing instance file or
/// registry-load error renders as `—`, same as "no profile" — there's
/// no useful distinction to draw for the user, and the chip should
/// never visibly error.
fn current_profile_display_name() -> String {
    let cached = {
        let mut cache = PROFILE_DISPLAY_CACHE.lock().unwrap();
        let fresh = cache
            .loaded_at
            .map(|at| at.elapsed() < PROFILE_DISPLAY_TTL)
            .unwrap_or(false);
        if fresh || cache.loading {
            return if cache.value.is_empty() {
                "—".to_string()
            } else {
                cache.value.clone()
            };
        }
        cache.loading = true;
        if cache.value.is_empty() {
            "—".to_string()
        } else {
            cache.value.clone()
        }
    };

    if std::thread::Builder::new()
        .name("profile-display-refresh".into())
        .spawn(|| {
            let refreshed = std::panic::catch_unwind(compute_current_profile_display_name);
            let mut cache = PROFILE_DISPLAY_CACHE.lock().unwrap();
            if let Ok(value) = refreshed {
                cache.value = value;
                cache.loaded_at = Some(std::time::Instant::now());
            }
            cache.loading = false;
        })
        .is_err()
    {
        PROFILE_DISPLAY_CACHE.lock().unwrap().loading = false;
    }

    cached
}

fn compute_current_profile_display_name() -> String {
    let info = crate::server_info::read_current();
    let Some(id) = info.profile.as_deref() else {
        return "—".to_string();
    };
    if id.is_empty() {
        return "—".to_string();
    }
    let Ok(registry) = unterm_profile::ProfileRegistry::load() else {
        return id.to_string();
    };
    let Some(profile) = registry.get(id) else {
        return id.to_string();
    };

    // Check whether any tracked secret expires within 7 days. The
    // warning glyph is a soft signal; the canonical reason live on
    // `unterm-cli profile audit` (and the equivalent MCP method),
    // both of which show date + days-remaining per secret. The chip
    // only needs to surface that something is wrong, not what.
    let today = chrono::Local::now().date_naive();
    let has_expiry_warning = profile.expiration.values().any(|date_str| {
        chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map(|d| (d - today).num_days() <= 7)
            .unwrap_or(false)
    });

    if has_expiry_warning {
        format!("{} ⚠", profile.display_name)
    } else {
        profile.display_name.clone()
    }
}

fn status_bar_theme_colors(theme: &str) -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)) {
    match theme {
        "midnight" => ((0x11, 0x18, 0x25), (0x43, 0x53, 0x70), (0xe2, 0xea, 0xf7)),
        "daylight" => ((0xd9, 0xdf, 0xd9), (0x82, 0x8d, 0x83), (0x0b, 0x0f, 0x14)),
        "classic" => ((0x18, 0x18, 0x18), (0x60, 0x60, 0x60), (0xf0, 0xf0, 0xf0)),
        "notion-dark" => ((0x21, 0x21, 0x1f), (0x5f, 0x5c, 0x54), (0xee, 0xec, 0xe6)),
        "notion-light" => ((0xde, 0xda, 0xd0), (0x8e, 0x87, 0x7a), (0x17, 0x16, 0x15)),
        _ => ((0x17, 0x17, 0x17), (0x52, 0x52, 0x52), (0xf0, 0xf0, 0xf0)),
    }
}

/// Alert palette for the **confirmation** banner. Hotter / more
/// saturated than the suggest bar — a worker thread is parked and
/// the user *has* to decide. Same shape as the other theme tuples:
/// (background, separator, foreground).
fn confirm_banner_theme_colors(theme: &str) -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)) {
    match theme {
        "midnight" => ((0x42, 0x20, 0x2e), (0xff, 0x7a, 0x8a), (0xff, 0xed, 0xe8)),
        "daylight" => ((0xff, 0xd0, 0xc4), (0xa5, 0x1d, 0x30), (0x24, 0x0c, 0x12)),
        "classic" => ((0x42, 0x20, 0x18), (0xff, 0x5f, 0x57), (0xff, 0xea, 0xd2)),
        "notion-dark" => ((0x3d, 0x20, 0x1b), (0xff, 0x78, 0x69), (0xff, 0xed, 0xe6)),
        "notion-light" => ((0xff, 0xd2, 0xc8), (0x9b, 0x24, 0x24), (0x25, 0x0e, 0x0c)),
        _ => ((0x3c, 0x18, 0x12), (0xff, 0x6b, 0x62), (0xff, 0xe8, 0xd0)),
    }
}

/// Accent palette for the suggest bar. Deliberately warmer than the
/// regular status bar so a pending suggestion catches the eye without
/// being alarming.
fn suggest_bar_theme_colors(theme: &str) -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)) {
    match theme {
        "midnight" => ((0x20, 0x2f, 0x4c), (0x8c, 0xb6, 0xff), (0xec, 0xf4, 0xff)),
        "daylight" => ((0xff, 0xe6, 0xa3), (0x75, 0x4b, 0x00), (0x22, 0x18, 0x02)),
        "classic" => ((0x31, 0x2a, 0x18), (0xf2, 0xc2, 0x3d), (0xff, 0xec, 0xc4)),
        "notion-dark" => ((0x33, 0x2d, 0x1d), (0xf0, 0xc2, 0x58), (0xff, 0xec, 0xc0)),
        "notion-light" => ((0xff, 0xe4, 0xa8), (0x70, 0x48, 0x0a), (0x22, 0x17, 0x04)),
        _ => ((0x2d, 0x26, 0x15), (0xee, 0xcf, 0x70), (0xff, 0xe9, 0xbc)),
    }
}

/// Truncate `text` so its display *width* (CJK = 2 cells, etc.) does
/// not exceed `max_cols`. Adds `…` when truncation happens. Used by
/// the suggest bar to keep the hint segment visible no matter how
/// long the suggestion text is.
fn truncate_to_width(text: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    if unicode_column_width(text, None) <= max_cols {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for c in text.chars() {
        let w = unicode_column_width(&c.to_string(), None);
        if used + w + 1 > max_cols {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bar_cwd_strips_remote_file_host() {
        let cwd = url::Url::parse("file://ubuntu/home/alex/project/src").unwrap();

        assert_eq!(
            crate::TermWindow::cwd_for_status(Some(&cwd)),
            "/home/alex/project/src"
        );
        assert_eq!(crate::TermWindow::project_label_for_cwd(Some(&cwd)), "src");
    }

    #[test]
    fn status_bar_cwd_defaults_without_active_cwd() {
        assert_eq!(crate::TermWindow::cwd_for_status(None), "~");
        assert_eq!(crate::TermWindow::project_label_for_cwd(None), "~");
    }

    #[test]
    fn truncate_to_width_preserves_short_text() {
        assert_eq!(truncate_to_width("agent:kimi-code", 32), "agent:kimi-code");
    }

    #[test]
    fn status_bar_density_progressively_reveals_secondary_state() {
        let narrow = status_bar_density(80);
        assert!(narrow.show_project);
        assert!(!narrow.show_theme);
        assert!(!narrow.show_telemetry);
        assert!(!narrow.show_capture);

        let common = status_bar_density(115);
        assert!(common.show_theme);
        assert!(!common.show_telemetry);
        assert!(!common.show_profile);
        assert!(!common.show_capture);

        let wide = status_bar_density(220);
        assert!(wide.show_project);
        assert!(wide.show_theme);
        assert!(wide.show_telemetry);
        assert!(wide.show_profile);
        assert!(wide.show_capture);
    }

    #[test]
    fn status_bar_parts_track_wide_text_offsets() {
        let mut text = " shell".to_string();
        let mut regions = vec![];
        let offset = append_status_part(
            &mut text,
            &mut regions,
            "项目:终端",
            Some(UIItemType::StatusBarProject),
        );
        assert_eq!(offset, unicode_column_width(" shell   ", None));
        assert_eq!(regions[0].offset, offset);
        assert_eq!(regions[0].len, unicode_column_width("项目:终端", None));
    }
}
