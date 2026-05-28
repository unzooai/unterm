use crate::quad::TripleLayerQuadAllocator;
use crate::termwindow::render::RenderScreenLineParams;
use crate::termwindow::{UIItem, UIItemType};
use mux::renderable::RenderableDimensions;
use termwiz::cell::{unicode_column_width, CellAttributes};
use termwiz::color::SrgbaTuple;
use termwiz::surface::line::Line;
use wezterm_term::color::ColorAttribute;
use window::WindowOps;
use window::color::LinearRgba;

static DEFER_FIRST_STATUS_TEXT_RENDER: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

impl crate::TermWindow {
    /// Height of the status bar in pixels (1 row of terminal font).
    pub fn status_bar_pixel_height(&self) -> f32 {
        if self.config.show_unterm_status_bar {
            self.render_metrics.cell_size.height as f32
        } else {
            0.0
        }
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

    fn active_pane_first_pending_suggestion(
        &self,
    ) -> Option<crate::mcp::handler::Suggestion> {
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
        let (bar_bg_rgb, sep_rgb, fg_rgb) = suggest_bar_theme_colors();
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
        let pad_cols = total_cols
            .saturating_sub(unicode_column_width(&text, None) + hint_cols);
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
                top_pixel_y: bar_y + 1.0,
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

        let (bar_bg_rgb, sep_rgb, fg_rgb) = status_bar_theme_colors();
        let bar_bg = LinearRgba::with_components(
            bar_bg_rgb.0 as f32 / 255.0,
            bar_bg_rgb.1 as f32 / 255.0,
            bar_bg_rgb.2 as f32 / 255.0,
            1.0,
        );

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

        // Top separator line (1px, subtle)
        let sep_color = LinearRgba::with_components(
            sep_rgb.0 as f32 / 255.0,
            sep_rgb.1 as f32 / 255.0,
            sep_rgb.2 as f32 / 255.0,
            1.0,
        );
        self.filled_rectangle(
            layers,
            1,
            euclid::rect(0., bar_y, bar_width, 1.0),
            sep_color,
        )?;

        if DEFER_FIRST_STATUS_TEXT_RENDER.swap(false, std::sync::atomic::Ordering::AcqRel) {
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
            return Ok(());
        }

        let (line, regions) = self.build_status_line();
        let total_cols = (bar_width / cell_width) as usize;

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

        self.render_screen_line(
            RenderScreenLineParams {
                top_pixel_y: bar_y + 1.0, // below separator
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

        let (bar_bg_rgb, sep_rgb, fg_rgb) = confirm_banner_theme_colors();
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
        let pad_cols = total_cols
            .saturating_sub(unicode_column_width(&text, None) + hint_cols);
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
                top_pixel_y: bar_y + 1.0,
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

    fn build_status_line(&self) -> (Line, Vec<StatusRegion>) {
        // Status bar text color
        let mut attrs = CellAttributes::blank();
        let (_, _, fg_rgb) = status_bar_theme_colors();
        attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            fg_rgb.0 as f32 / 255.0,
            fg_rgb.1 as f32 / 255.0,
            fg_rgb.2 as f32 / 255.0,
            1.0,
        )));

        // 1. Shell type (with version distinction)
        let shell_name = if let Some(pane) = self.get_active_pane_no_overlay() {
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
                    if lower.starts_with("/") {
                        "bash (wsl)".to_string()
                    } else {
                        "bash".to_string()
                    }
                } else if lower.contains("zsh") {
                    if lower.starts_with("/") {
                        "zsh (wsl)".to_string()
                    } else {
                        "zsh".to_string()
                    }
                } else if lower.contains("fish") {
                    if lower.starts_with("/") {
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

        let proxy_enabled = unterm_proxy_enabled();
        let proxy = if proxy_enabled {
            "proxy:on".to_string()
        } else {
            "proxy:off".to_string()
        };
        let theme = crate::overlay::theme_selector::read_theme_id();

        // MCP activity chip. Always rendered so the position is stable
        // (a chip that appears/disappears would shift every neighboring
        // segment's click hit-test). `⚡` suffix marks "writes recently"
        // so the user notices a flash without having to compare counts.
        let mcp_activity = crate::mcp::handler::recent_mcp_input_activity();
        let mcp_part = {
            let flash = mcp_activity
                .seconds_since_last
                .map(|s| s < 5.0)
                .unwrap_or(false);
            format!(
                "mcp:{}{}",
                mcp_activity.count,
                if flash { "⚡" } else { "" }
            )
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
        let profile_label = current_profile_display_name();
        let profile_part = format!("profile:{profile_label}");

        let project_part = format!("project:{}", self.active_project_label());

        // Use *cell width* (not char count) for offsets so the click hit-test
        // lines up with the rendered glyph. Wide CJK chars take 2 cells.
        let cw = |s: &str| unicode_column_width(s, None);

        let cwd_part = self.active_pane_cwd_for_status();

        let mut text = format!(" {} | ", shell_name);
        let cwd_offset = cw(&text);
        text.push_str(&cwd_part);
        text.push_str(" | ");
        text.push_str(&format!("{}x{} | ", cols, rows));
        let project_offset = cw(&text);
        text.push_str(&project_part);
        text.push_str(" | ");
        let exclude_offset = cw(&text);
        let exclude_part = "capture:exclude".to_string();
        text.push_str(&exclude_part);
        text.push_str(" | ");
        let include_offset = cw(&text);
        let include_part = "capture:include".to_string();
        text.push_str(&include_part);
        text.push_str(" | ");
        let proxy_offset = cw(&text);
        text.push_str(&proxy);
        text.push_str(" | ");
        let mcp_offset = cw(&text);
        text.push_str(&mcp_part);
        text.push_str(" | ");
        let theme_offset = cw(&text);
        let theme_part = format!("theme:{theme}");
        text.push_str(&theme_part);
        text.push_str(" | ");
        let profile_offset = cw(&text);
        text.push_str(&profile_part);
        text.push(' ');

        (
            Line::from_text(&text, &attrs, 0, None),
            vec![
                StatusRegion {
                    offset: cwd_offset,
                    len: cw(&cwd_part),
                    item_type: UIItemType::StatusBarCwd,
                },
                StatusRegion {
                    offset: project_offset,
                    len: cw(&project_part),
                    item_type: UIItemType::StatusBarProject,
                },
                StatusRegion {
                    offset: exclude_offset,
                    len: cw(&exclude_part),
                    item_type: UIItemType::StatusBarCaptureExclude,
                },
                StatusRegion {
                    offset: include_offset,
                    len: cw(&include_part),
                    item_type: UIItemType::StatusBarCaptureInclude,
                },
                StatusRegion {
                    offset: proxy_offset,
                    len: cw(&proxy),
                    item_type: UIItemType::StatusBarProxy,
                },
                StatusRegion {
                    offset: mcp_offset,
                    len: cw(&mcp_part),
                    item_type: UIItemType::StatusBarMcpAudit,
                },
                StatusRegion {
                    offset: theme_offset,
                    len: cw(&theme_part),
                    item_type: UIItemType::StatusBarTheme,
                },
                StatusRegion {
                    offset: profile_offset,
                    len: cw(&profile_part),
                    item_type: UIItemType::StatusBarProfile,
                },
            ],
        )
    }

    /// Active pane's cwd, formatted for the bottom status bar:
    ///   - Resolved to a local path when possible (drops the `file://` scheme
    ///     and the host component for remote URIs we can't visit anyway).
    ///   - $HOME prefix replaced with `~` so common project paths
    ///     stay short.
    ///   - Truncated to ~48 display columns by elision in the *middle*
    ///     (`/Users/me/code/.../wezterm-gui/src`) — keeps both project
    ///     root context and current-directory tail visible.
    fn active_pane_cwd_for_status(&self) -> String {
        let raw: Option<String> = self
            .get_active_pane_no_overlay()
            .and_then(|pane| pane.get_current_working_dir(mux::pane::CachePolicy::AllowStale))
            .map(|cwd| {
                // OSC 7 carries the hostname; on the local machine that's
                // typically "localhost", but multiplexer-mode and remote
                // panes (Linux container / SSH host) report a real
                // hostname like "ubuntu". `to_file_path()` only succeeds
                // when host is empty/localhost — for everything else it
                // returns Err and we previously fell back to the raw URL
                // (ugly: `file://ubuntu/home/alexlee` showing up in the
                // status bar). Strip to just the path component instead.
                if let Ok(p) = cwd.to_file_path() {
                    p.display().to_string()
                } else {
                    let s = cwd.as_str();
                    s.strip_prefix("file://")
                        .and_then(|rest| rest.split_once('/').map(|(_host, path)| format!("/{}", path)))
                        .unwrap_or_else(|| s.to_string())
                }
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
        let mut tail_chars: Vec<char> =
            chars.iter().rev().take(20).copied().collect();
        tail_chars.reverse();
        let tail: String = tail_chars.into_iter().collect();
        format!("{} ... {}", head, tail)
    }

    fn active_project_label(&self) -> String {
        let Some(pane) = self.get_active_pane_no_overlay() else {
            return "~".to_string();
        };
        let Some(cwd) = pane.get_current_working_dir(mux::pane::CachePolicy::AllowStale) else {
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

#[derive(Default)]
struct ProxyStatusCache {
    value: bool,
    loaded: bool,
    loading: bool,
}

lazy_static::lazy_static! {
    static ref PROXY_STATUS_CACHE: std::sync::Mutex<ProxyStatusCache> =
        std::sync::Mutex::new(ProxyStatusCache::default());
}

fn unterm_proxy_enabled() -> bool {
    let mut cache = PROXY_STATUS_CACHE.lock().unwrap();
    if cache.loaded || cache.loading {
        return cache.value;
    }

    cache.loading = true;
    let value = cache.value;
    drop(cache);

    std::thread::spawn(|| {
        let enabled = read_unterm_proxy_enabled();
        let mut cache = PROXY_STATUS_CACHE.lock().unwrap();
        cache.value = enabled;
        cache.loaded = true;
        cache.loading = false;
    });

    value
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

fn status_bar_theme_colors() -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)) {
    match crate::overlay::theme_selector::read_theme_id().as_str() {
        "midnight" => ((0x12, 0x18, 0x24), (0x2f, 0x45, 0x68), (0xb8, 0xc7, 0xe0)),
        "daylight" => ((0xee, 0xec, 0xdd), (0x93, 0xa1, 0xa1), (0x58, 0x6e, 0x75)),
        "classic" => ((0x20, 0x20, 0x20), (0x55, 0x55, 0x55), (0xd0, 0xd0, 0xd0)),
        _ => ((0x1e, 0x1e, 0x1e), (0x3a, 0x3a, 0x3a), (0xa0, 0xa0, 0xa0)),
    }
}

/// Alert palette for the **confirmation** banner. Hotter / more
/// saturated than the suggest bar — a worker thread is parked and
/// the user *has* to decide. Same shape as the other theme tuples:
/// (background, separator, foreground).
fn confirm_banner_theme_colors() -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)) {
    match crate::overlay::theme_selector::read_theme_id().as_str() {
        "midnight" => ((0x3a, 0x18, 0x18), (0xc8, 0x44, 0x44), (0xff, 0xe2, 0xc0)),
        "daylight" => ((0xff, 0xe0, 0xd0), (0xc8, 0x35, 0x10), (0x40, 0x18, 0x0a)),
        "classic" => ((0x40, 0x20, 0x18), (0xb0, 0x40, 0x20), (0xff, 0xd6, 0x88)),
        _ => ((0x33, 0x14, 0x0d), (0xa8, 0x3a, 0x20), (0xff, 0xc8, 0x78)),
    }
}

/// Accent palette for the suggest bar. Deliberately warmer than the
/// regular status bar so a pending suggestion catches the eye without
/// being alarming.
fn suggest_bar_theme_colors() -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)) {
    match crate::overlay::theme_selector::read_theme_id().as_str() {
        "midnight" => ((0x1a, 0x24, 0x3a), (0x4a, 0x6f, 0xa8), (0xc8, 0xdb, 0xf5)),
        "daylight" => ((0xff, 0xf3, 0xc4), (0xd0, 0xa0, 0x33), (0x40, 0x32, 0x10)),
        "classic" => ((0x2a, 0x24, 0x18), (0x88, 0x66, 0x33), (0xff, 0xd6, 0x88)),
        _ => ((0x24, 0x1f, 0x14), (0x70, 0x55, 0x2a), (0xff, 0xc6, 0x70)),
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
