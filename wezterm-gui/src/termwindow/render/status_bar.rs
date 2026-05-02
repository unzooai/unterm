use crate::quad::TripleLayerQuadAllocator;
use crate::termwindow::render::RenderScreenLineParams;
use crate::termwindow::{UIItem, UIItemType};
use mux::renderable::RenderableDimensions;
use termwiz::cell::{unicode_column_width, CellAttributes};
use termwiz::color::SrgbaTuple;
use termwiz::surface::line::Line;
use wezterm_term::color::ColorAttribute;
use window::color::LinearRgba;

impl crate::TermWindow {
    /// Height of the status bar in pixels (1 row of terminal font).
    pub fn status_bar_pixel_height(&self) -> f32 {
        if self.config.show_unterm_status_bar {
            self.render_metrics.cell_size.height as f32
        } else {
            0.0
        }
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

        self.filled_rectangle(
            layers,
            0,
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
            0,
            euclid::rect(0., bar_y, bar_width, 1.0),
            sep_color,
        )?;

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
                item_type: region.item_type,
            });
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

        let proxy = if unterm_proxy_enabled() {
            crate::i18n::t("status_bar.proxy_on")
        } else {
            crate::i18n::t("status_bar.proxy_off")
        };
        let theme = crate::overlay::theme_selector::read_theme_id();

        let project_part = crate::i18n::t_args(
            "status_bar.project",
            &[("name", &self.active_project_label())],
        );

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
        let exclude_part = crate::i18n::t("status_bar.screenshot_exclude");
        text.push_str(&exclude_part);
        text.push_str(" | ");
        let include_offset = cw(&text);
        let include_part = crate::i18n::t("status_bar.screenshot_include");
        text.push_str(&include_part);
        text.push_str(" | ");
        let proxy_offset = cw(&text);
        text.push_str(&proxy);
        text.push_str(" | ");
        let theme_offset = cw(&text);
        let theme_part = crate::i18n::t_args("status_bar.theme", &[("name", &theme)]);
        text.push_str(&theme_part);
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
                    offset: theme_offset,
                    len: cw(&theme_part),
                    item_type: UIItemType::StatusBarTheme,
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
                cwd.to_file_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| cwd.to_string())
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

fn unterm_proxy_enabled() -> bool {
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

fn status_bar_theme_colors() -> ((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)) {
    match crate::overlay::theme_selector::read_theme_id().as_str() {
        "midnight" => ((0x12, 0x18, 0x24), (0x2f, 0x45, 0x68), (0xb8, 0xc7, 0xe0)),
        "daylight" => ((0xee, 0xec, 0xdd), (0x93, 0xa1, 0xa1), (0x58, 0x6e, 0x75)),
        "classic" => ((0x20, 0x20, 0x20), (0x55, 0x55, 0x55), (0xd0, 0xd0, 0xd0)),
        _ => ((0x1e, 0x1e, 0x1e), (0x3a, 0x3a, 0x3a), (0xa0, 0xa0, 0xa0)),
    }
}
