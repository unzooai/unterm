use crate::ai::models::{ai_state, ChatRole, InsightType};
use crate::quad::TripleLayerQuadAllocator;
use crate::termwindow::render::RenderScreenLineParams;
use crate::termwindow::{UIItem, UIItemType};
use mux::renderable::RenderableDimensions;
use termwiz::cell::CellAttributes;
use termwiz::color::SrgbaTuple;
use termwiz::surface::line::Line;
use wezterm_term::color::ColorAttribute;
use window::color::LinearRgba;

impl crate::TermWindow {
    /// Width of the AI panel in cells.
    fn ai_panel_cols(&self) -> usize {
        let total_cols = self.dimensions.pixel_width / self.render_metrics.cell_size.width as usize;
        // Panel is ~30% of width, at least 30 cols, at most 60 cols
        let panel_cols = (total_cols * 30 / 100).max(30).min(60);
        panel_cols
    }

    /// Pixel x position where the AI panel starts.
    pub fn ai_panel_pixel_x(&self) -> f32 {
        let panel_cols = self.ai_panel_cols();
        let panel_pixel_width = panel_cols as f32 * self.render_metrics.cell_size.width as f32;
        self.dimensions.pixel_width as f32 - panel_pixel_width
    }

    pub fn paint_ai_panel(&mut self, layers: &mut TripleLayerQuadAllocator) -> anyhow::Result<()> {
        if !ai_state().panel_visible() {
            return Ok(());
        }

        let panel_cols = self.ai_panel_cols();
        let panel_x = self.ai_panel_pixel_x();
        let cell_height = self.render_metrics.cell_size.height as f32;

        let border = self.get_os_border();
        let tab_bar_height = if self.show_tab_bar {
            self.tab_bar_pixel_height().unwrap_or(0.)
        } else {
            0.
        };
        let top_y = border.top.get() as f32 + tab_bar_height;

        let panel_pixel_width = panel_cols as f32 * self.render_metrics.cell_size.width as f32;
        let panel_height = self.dimensions.pixel_height as f32 - top_y - border.bottom.get() as f32;
        let visible_rows = (panel_height / cell_height) as usize;

        // Background: #181818 deep dark
        let panel_bg = LinearRgba::with_components(
            0x18 as f32 / 255.0,
            0x18 as f32 / 255.0,
            0x18 as f32 / 255.0,
            1.0,
        );

        // Panel background
        self.filled_rectangle(
            layers,
            0,
            euclid::rect(panel_x, top_y, panel_pixel_width, panel_height),
            panel_bg,
        )?;

        // Separator line (left edge, 2px, accent blue)
        let sep_bg = LinearRgba::with_components(
            0x56 as f32 / 255.0,
            0x9c as f32 / 255.0,
            0xd6 as f32 / 255.0,
            0.6,
        );
        self.filled_rectangle(
            layers,
            0,
            euclid::rect(panel_x, top_y, 2.0, panel_height),
            sep_bg,
        )?;

        // Register AI panel as UI item for mouse hit testing
        let input_row_height = (cell_height * 3.0) as usize; // 3 rows for input area
        let panel_body_height = panel_height as usize - input_row_height;

        // Panel body area (scrollable)
        self.ui_items.push(UIItem {
            x: panel_x as usize,
            y: top_y as usize,
            width: panel_pixel_width as usize,
            height: panel_body_height,
            item_type: UIItemType::AiPanel,
        });

        // Input area at bottom
        self.ui_items.push(UIItem {
            x: panel_x as usize,
            y: (top_y as usize) + panel_body_height,
            width: panel_pixel_width as usize,
            height: input_row_height,
            item_type: UIItemType::AiPanelInput,
        });

        // Build content lines and collect execute button positions
        let (lines, execute_buttons) = self.build_ai_panel_lines(panel_cols, visible_rows);

        // Register execute buttons as clickable UIItems
        for (row, cmd) in &execute_buttons {
            let btn_y = top_y + *row as f32 * cell_height;
            self.ui_items.push(UIItem {
                x: panel_x as usize,
                y: btn_y as usize,
                width: panel_pixel_width as usize,
                height: cell_height as usize,
                item_type: UIItemType::AiPanelExecute(cmd.clone()),
            });
        }

        // Create a palette for the panel
        let palette = self.palette().clone();
        let window_is_transparent =
            !self.window_background.is_empty() || self.config.window_background_opacity != 1.0;
        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();

        let fg = LinearRgba::with_components(
            0xe0 as f32 / 255.0,
            0xe0 as f32 / 255.0,
            0xe0 as f32 / 255.0,
            1.0,
        );

        // Render each line
        for (row, line) in lines.iter().enumerate().take(visible_rows) {
            let y = top_y + row as f32 * cell_height;

            self.render_screen_line(
                RenderScreenLineParams {
                    top_pixel_y: y,
                    left_pixel_x: panel_x + 4.0, // small padding after separator
                    pixel_width: panel_pixel_width - 4.0,
                    stable_line_idx: None,
                    line,
                    selection: 0..0,
                    cursor: &Default::default(),
                    palette: &palette,
                    dims: &RenderableDimensions {
                        cols: panel_cols.saturating_sub(1),
                        physical_top: 0,
                        scrollback_rows: 0,
                        scrollback_top: 0,
                        viewport_rows: 1,
                        dpi: self.terminal_size.dpi,
                        pixel_height: self.render_metrics.cell_size.height as usize,
                        pixel_width: panel_pixel_width as usize,
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
                    default_bg: panel_bg,
                    style: None,
                    font: None,
                    use_pixel_positioning: self.config.experimental_pixel_positioning,
                    render_metrics: self.render_metrics,
                    shape_key: None,
                    password_input: false,
                },
                layers,
            )?;
        }

        Ok(())
    }

    /// Returns (lines, execute_button_positions) where each button is (row_index, command).
    fn build_ai_panel_lines(
        &self,
        panel_cols: usize,
        visible_rows: usize,
    ) -> (Vec<Line>, Vec<(usize, String)>) {
        let mut lines: Vec<Line> = Vec::new();
        let mut execute_buttons: Vec<(usize, String)> = Vec::new();
        let usable_cols = panel_cols.saturating_sub(2);

        // Title: bright white, bold
        let mut title_attrs = CellAttributes::blank();
        title_attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            1.0, 1.0, 1.0, 1.0,
        )));

        // Body text: bright white-gray (high contrast on dark bg)
        let mut body_attrs = CellAttributes::blank();
        body_attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            0xe0 as f32 / 255.0,
            0xe0 as f32 / 255.0,
            0xe0 as f32 / 255.0,
            1.0,
        )));

        // Command/user input: bright green
        let mut cmd_attrs = CellAttributes::blank();
        cmd_attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            0x98 as f32 / 255.0,
            0xc3 as f32 / 255.0,
            0x79 as f32 / 255.0,
            1.0,
        )));

        // Error: bright red
        let mut err_attrs = CellAttributes::blank();
        err_attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            0xe0 as f32 / 255.0,
            0x6c as f32 / 255.0,
            0x75 as f32 / 255.0,
            1.0,
        )));

        // Dim text: medium gray (but still readable)
        let mut dim_attrs = CellAttributes::blank();
        dim_attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            0x80 as f32 / 255.0,
            0x80 as f32 / 255.0,
            0x80 as f32 / 255.0,
            1.0,
        )));

        // Accent: bright blue
        let mut accent_attrs = CellAttributes::blank();
        accent_attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            0x61 as f32 / 255.0,
            0xaf as f32 / 255.0,
            0xef as f32 / 255.0,
            1.0,
        )));

        // Header
        lines.push(Line::from_text("", &CellAttributes::blank(), 0, None));
        let header = " Unterm AI";
        lines.push(Line::from_text(header, &title_attrs, 0, None));

        // Separator
        let sep = format!(
            " {}",
            "\u{2500}".repeat(usable_cols.saturating_sub(2).min(38))
        );
        lines.push(Line::from_text(&sep, &dim_attrs, 0, None));

        lines.push(Line::from_text("", &CellAttributes::blank(), 0, None));

        // Show active model
        let state = ai_state();
        let model = state.active_model();
        let provider = state.provider();
        let model_line = format!(" {} {}", provider.display_icon(), model);
        lines.push(Line::from_text(&model_line, &accent_attrs, 0, None));

        lines.push(Line::from_text("", &CellAttributes::blank(), 0, None));

        // Show insight card if any
        if let Some(card) = state.get_insight() {
            let type_icon = match &card.card_type {
                InsightType::Error => "\u{2717}",
                InsightType::Suggestion => "\u{25b8}",
                InsightType::Info => "\u{2139}",
                InsightType::Chat => "\u{25c6}",
            };

            let card_title_attrs = match &card.card_type {
                InsightType::Error => &err_attrs,
                _ => &title_attrs,
            };

            let title = format!(" {} {}", type_icon, card.title);
            lines.push(Line::from_text(&title, card_title_attrs, 0, None));
            lines.push(Line::from_text(
                &format!(" {}", "\u{2500}".repeat(usable_cols.min(40))),
                &dim_attrs,
                0,
                None,
            ));

            // Word-wrap content
            for content_line in card.content.lines() {
                let wrapped = word_wrap(content_line, usable_cols);
                for w in wrapped {
                    lines.push(Line::from_text(&format!(" {}", w), &body_attrs, 0, None));
                }
            }

            if let Some(cmd) = &card.command {
                lines.push(Line::from_text("", &CellAttributes::blank(), 0, None));
                lines.push(Line::from_text(
                    &format!("  $ {}", cmd),
                    &cmd_attrs,
                    0,
                    None,
                ));
                // Execute button (clickable)
                let exec_label = " ▶ Execute in Terminal";
                lines.push(Line::from_text(exec_label, &accent_attrs, 0, None));
                execute_buttons.push((lines.len() - 1, cmd.clone()));
            }
        } else {
            lines.push(Line::from_text(" No insights yet", &dim_attrs, 0, None));
            lines.push(Line::from_text("", &CellAttributes::blank(), 0, None));
            lines.push(Line::from_text(
                " Click here or Ctrl+Shift+U to chat",
                &dim_attrs,
                0,
                None,
            ));
        }

        // Chat history
        let history = state.chat_history();
        if !history.is_empty() {
            lines.push(Line::from_text("", &CellAttributes::blank(), 0, None));
            let sep = format!(
                " {}",
                "\u{2500}".repeat(usable_cols.saturating_sub(2).min(38))
            );
            lines.push(Line::from_text(&sep, &dim_attrs, 0, None));
            lines.push(Line::from_text(" Chat", &accent_attrs, 0, None));
            lines.push(Line::from_text("", &CellAttributes::blank(), 0, None));

            // Show all messages, apply scroll offset
            // Each entry: (line, is_user, optional_command_for_execute_button)
            let mut chat_lines: Vec<(Line, bool, Option<String>)> = Vec::new();
            for msg in &history {
                match msg.role {
                    ChatRole::User => {
                        for content_line in msg.content.lines() {
                            let text = format!(" > {}", content_line);
                            let wrapped = word_wrap(&text, usable_cols);
                            for w in wrapped {
                                chat_lines.push((
                                    Line::from_text(&w, &cmd_attrs, 0, None),
                                    true,
                                    None,
                                ));
                            }
                        }
                    }
                    ChatRole::Assistant => {
                        // Parse code blocks for execute buttons
                        let mut in_code_block = false;
                        let mut code_block = String::new();
                        for content_line in msg.content.lines() {
                            if content_line.trim_start().starts_with("```") {
                                if in_code_block {
                                    // End of code block — add execute button
                                    if !code_block.trim().is_empty() {
                                        let cmd = code_block.trim().to_string();
                                        chat_lines.push((
                                            Line::from_text(" ▶ Execute", &accent_attrs, 0, None),
                                            false,
                                            Some(cmd),
                                        ));
                                    }
                                    code_block.clear();
                                    in_code_block = false;
                                } else {
                                    in_code_block = true;
                                }
                                continue;
                            }
                            if in_code_block {
                                let text = format!("   {}", content_line);
                                chat_lines.push((
                                    Line::from_text(&text, &cmd_attrs, 0, None),
                                    false,
                                    None,
                                ));
                                if !code_block.is_empty() {
                                    code_block.push('\n');
                                }
                                code_block.push_str(content_line);
                            } else {
                                let text = format!("   {}", content_line);
                                let wrapped = word_wrap(&text, usable_cols);
                                for w in wrapped {
                                    chat_lines.push((
                                        Line::from_text(&w, &body_attrs, 0, None),
                                        false,
                                        None,
                                    ));
                                }
                            }
                        }
                        // Handle unclosed code block
                        if in_code_block && !code_block.trim().is_empty() {
                            let cmd = code_block.trim().to_string();
                            chat_lines.push((
                                Line::from_text(" ▶ Execute", &accent_attrs, 0, None),
                                false,
                                Some(cmd),
                            ));
                        }
                    }
                }
                chat_lines.push((
                    Line::from_text("", &CellAttributes::blank(), 0, None),
                    false,
                    None,
                ));
            }

            // Apply scroll: scroll_offset=0 means show latest (bottom)
            let scroll_offset = state.scroll_offset();
            let available_rows = visible_rows.saturating_sub(lines.len() + 3); // reserve for input
            let total_chat = chat_lines.len();
            let end = total_chat.saturating_sub(scroll_offset);
            let start = end.saturating_sub(available_rows);

            for (line, _, exec_cmd) in &chat_lines[start..end] {
                lines.push(line.clone());
                if let Some(cmd) = exec_cmd {
                    execute_buttons.push((lines.len() - 1, cmd.clone()));
                }
            }

            // Show scroll indicator if not at bottom
            if scroll_offset > 0 {
                lines.push(Line::from_text(
                    &format!(" [{} more below]", scroll_offset.min(total_chat)),
                    &dim_attrs,
                    0,
                    None,
                ));
            }
        }

        // Chat input
        lines.push(Line::from_text("", &CellAttributes::blank(), 0, None));
        let sep = format!(
            " {}",
            "\u{2500}".repeat(usable_cols.saturating_sub(2).min(38))
        );
        lines.push(Line::from_text(&sep, &dim_attrs, 0, None));

        let chat_input = state.chat_input();
        let focused = state.chat_focused();

        // Input box background highlight
        let mut input_bg_attrs = CellAttributes::blank();
        if focused {
            input_bg_attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(
                SrgbaTuple(
                    0x98 as f32 / 255.0,
                    0xc3 as f32 / 255.0,
                    0x79 as f32 / 255.0,
                    1.0,
                ),
            ));
        } else {
            input_bg_attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(
                SrgbaTuple(
                    0x80 as f32 / 255.0,
                    0x80 as f32 / 255.0,
                    0x80 as f32 / 255.0,
                    1.0,
                ),
            ));
        }

        let prompt = if focused {
            format!(" > {}\u{258f}", chat_input)
        } else {
            " Click to chat...".to_string()
        };
        lines.push(Line::from_text(&prompt, &input_bg_attrs, 0, None));

        (lines, execute_buttons)
    }
}

fn word_wrap(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            result.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}
