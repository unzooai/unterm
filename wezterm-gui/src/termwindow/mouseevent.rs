use crate::tabbar::TabBarItem;
use crate::termwindow::{
    GuiWin, MouseCapture, PositionedSplit, ScrollHit, TermWindowNotif, UIItem, UIItemType, TMB,
};
use ::window::{
    MouseButtons as WMB, MouseCursor, MouseEvent, MouseEventKind as WMEK, MousePress,
    WindowDecorations, WindowOps, WindowState,
};
use config::keyassignment::{KeyAssignment, MouseEventTrigger, SpawnCommand};
use config::MouseEventAltScreen;
use mux::pane::{Pane, WithPaneLines};
use mux::tab::SplitDirection;
use mux::Mux;
use mux_lua::MuxPane;
use std::convert::TryInto;
use std::ops::Sub;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use termwiz::hyperlink::Hyperlink;
use termwiz::surface::Line;
use wezterm_dynamic::ToDynamic;
use wezterm_term::input::{MouseButton, MouseEventKind as TMEK};
use wezterm_term::{ClickPosition, LastMouseClick, StableRowIndex};

impl super::TermWindow {
    fn resolve_ui_item(&self, event: &MouseEvent) -> Option<UIItem> {
        let x = event.coords.x;
        let y = event.coords.y;
        self.ui_items
            .iter()
            .rev()
            .find(|item| item.hit_test(x, y))
            .cloned()
    }

    fn leave_ui_item(&mut self, item: &UIItem) {
        match item.item_type {
            UIItemType::TabBar(_) => {
                self.update_title_post_status();
            }
            UIItemType::CloseTab(_)
            | UIItemType::AboveScrollThumb
            | UIItemType::BelowScrollThumb
            | UIItemType::ScrollThumb
            | UIItemType::Split(_)
            | UIItemType::AiPanel
            | UIItemType::AiPanelInput
            | UIItemType::AiPanelExecute(_)
            | UIItemType::StatusBarProject
            | UIItemType::StatusBarTheme
            | UIItemType::StatusBarCapture
            | UIItemType::StatusBarAdmin
            | UIItemType::StatusBarProxy
            | UIItemType::StatusBarCommand => {}
        }
    }

    fn enter_ui_item(&mut self, item: &UIItem) {
        match item.item_type {
            UIItemType::TabBar(_) => {}
            UIItemType::CloseTab(_)
            | UIItemType::AboveScrollThumb
            | UIItemType::BelowScrollThumb
            | UIItemType::ScrollThumb
            | UIItemType::Split(_)
            | UIItemType::AiPanel
            | UIItemType::AiPanelInput
            | UIItemType::AiPanelExecute(_)
            | UIItemType::StatusBarProject
            | UIItemType::StatusBarTheme
            | UIItemType::StatusBarCapture
            | UIItemType::StatusBarAdmin
            | UIItemType::StatusBarProxy
            | UIItemType::StatusBarCommand => {}
        }
    }

    pub fn mouse_event_impl(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        log::trace!("{:?}", event);
        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        self.current_mouse_event.replace(event.clone());

        let border = self.get_os_border();

        let first_line_offset = if self.show_tab_bar && !self.config.tab_bar_at_bottom {
            self.tab_bar_pixel_height().unwrap_or(0.) as isize
        } else {
            0
        } + border.top.get() as isize;

        let (padding_left, padding_top) = self.padding_left_top();

        let y = (event
            .coords
            .y
            .sub(padding_top as isize)
            .sub(first_line_offset)
            .max(0)
            / self.render_metrics.cell_size.height) as i64;

        let x = (event
            .coords
            .x
            .sub((padding_left + border.left.get() as f32) as isize)
            .max(0) as f32)
            / self.render_metrics.cell_size.width as f32;
        let x = if !pane.is_mouse_grabbed() {
            // Round the x coordinate so that we're a bit more forgiving of
            // the horizontal position when selecting cells
            x.round()
        } else {
            x
        }
        .trunc() as usize;

        let mut y_pixel_offset = event
            .coords
            .y
            .sub(padding_top as isize)
            .sub(first_line_offset);
        if y > 0 {
            y_pixel_offset = y_pixel_offset.max(0) % self.render_metrics.cell_size.height;
        }

        let mut x_pixel_offset = event
            .coords
            .x
            .sub((padding_left + border.left.get() as f32) as isize);
        if x > 0 {
            x_pixel_offset = x_pixel_offset.max(0) % self.render_metrics.cell_size.width;
        }

        self.last_mouse_coords = (x, y);

        let mut capture_mouse = false;

        match event.kind {
            WMEK::Release(ref press) => {
                self.current_mouse_capture = None;
                self.current_mouse_buttons.retain(|p| p != press);
                if press == &MousePress::Left && self.window_drag_position.take().is_some() {
                    // Completed a window drag
                    return;
                }
                if press == &MousePress::Left && self.dragging.take().is_some() {
                    // Completed a drag
                    return;
                }
            }

            WMEK::Press(ref press) => {
                capture_mouse = true;

                // Perform click counting
                let button = mouse_press_to_tmb(press);

                let click_position = ClickPosition {
                    column: x,
                    row: y,
                    x_pixel_offset,
                    y_pixel_offset,
                };

                let click = match self.last_mouse_click.take() {
                    None => LastMouseClick::new(button, click_position),
                    Some(click) => click.add(button, click_position),
                };
                self.last_mouse_click = Some(click);
                self.current_mouse_buttons.retain(|p| p != press);
                self.current_mouse_buttons.push(*press);
            }

            WMEK::Move => {
                if let Some(start) = self.window_drag_position.as_ref() {
                    // Dragging the window
                    // Compute the distance since the initial event
                    let delta_x = start.screen_coords.x - event.screen_coords.x;
                    let delta_y = start.screen_coords.y - event.screen_coords.y;

                    // Now compute a new window position.
                    // We don't have a direct way to get the position,
                    // but we can infer it by comparing the mouse coords
                    // with the screen coords in the initial event.
                    // This computes the original top_left position,
                    // and applies the total drag delta to it.
                    let top_left = ::window::ScreenPoint::new(
                        (start.screen_coords.x - start.coords.x) - delta_x,
                        (start.screen_coords.y - start.coords.y) - delta_y,
                    );
                    // and now tell the window to go there
                    context.set_window_position(top_left);
                    return;
                }

                if let Some((item, start_event)) = self.dragging.take() {
                    self.drag_ui_item(item, start_event, x, y, event, context);
                    return;
                }
            }
            _ => {}
        }

        let prior_ui_item = self.last_ui_item.clone();

        let ui_item = if matches!(self.current_mouse_capture, None | Some(MouseCapture::UI)) {
            let ui_item = self.resolve_ui_item(&event);

            match (self.last_ui_item.take(), &ui_item) {
                (Some(prior), Some(item)) => {
                    if prior != *item || !self.config.use_fancy_tab_bar {
                        self.leave_ui_item(&prior);
                        self.enter_ui_item(item);
                        context.invalidate();
                    }
                }
                (Some(prior), None) => {
                    self.leave_ui_item(&prior);
                    context.invalidate();
                }
                (None, Some(item)) => {
                    self.enter_ui_item(item);
                    context.invalidate();
                }
                (None, None) => {}
            }

            ui_item
        } else {
            None
        };

        if let Some(item) = ui_item.clone() {
            if capture_mouse {
                self.current_mouse_capture = Some(MouseCapture::UI);
            }
            self.mouse_event_ui_item(item, pane, y, event, context);
        } else if matches!(
            self.current_mouse_capture,
            None | Some(MouseCapture::TerminalPane(_))
        ) {
            self.mouse_event_terminal(
                pane,
                ClickPosition {
                    column: x,
                    row: y,
                    x_pixel_offset,
                    y_pixel_offset,
                },
                event,
                context,
                capture_mouse,
            );
        }

        if prior_ui_item != ui_item {
            self.update_title_post_status();
        }
    }

    pub fn mouse_leave_impl(&mut self, context: &dyn WindowOps) {
        self.current_mouse_event = None;
        self.update_title();
        context.set_cursor(Some(MouseCursor::Arrow));
        context.invalidate();
    }

    fn drag_split(
        &mut self,
        mut item: UIItem,
        split: PositionedSplit,
        start_event: MouseEvent,
        x: usize,
        y: i64,
        context: &dyn WindowOps,
    ) {
        let mux = Mux::get();
        let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
            Some(tab) => tab,
            None => return,
        };
        let delta = match split.direction {
            SplitDirection::Horizontal => (x as isize).saturating_sub(split.left as isize),
            SplitDirection::Vertical => (y as isize).saturating_sub(split.top as isize),
        };

        if delta != 0 {
            tab.resize_split_by(split.index, delta);
            if let Some(split) = tab.iter_splits().into_iter().nth(split.index) {
                item.item_type = UIItemType::Split(split);
                context.invalidate();
            }
        }
        self.dragging.replace((item, start_event));
    }

    fn drag_scroll_thumb(
        &mut self,
        item: UIItem,
        start_event: MouseEvent,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let dims = pane.get_dimensions();
        let current_viewport = self.get_viewport(pane.pane_id());

        let tab_bar_height = if self.show_tab_bar {
            self.tab_bar_pixel_height().unwrap_or(0.)
        } else {
            0.
        };
        let (top_bar_height, bottom_bar_height) = if self.config.tab_bar_at_bottom {
            (0.0, tab_bar_height)
        } else {
            (tab_bar_height, 0.0)
        };

        let border = self.get_os_border();
        let y_offset = top_bar_height + border.top.get() as f32;

        let from_top = start_event.coords.y.saturating_sub(item.y as isize);
        let effective_thumb_top = event
            .coords
            .y
            .saturating_sub(y_offset as isize + from_top)
            .max(0) as usize;

        // Convert thumb top into a row index by reversing the math
        // in ScrollHit::thumb
        let row = ScrollHit::thumb_top_to_scroll_top(
            effective_thumb_top,
            &*pane,
            current_viewport,
            self.dimensions.pixel_height.saturating_sub(
                y_offset as usize + border.bottom.get() + bottom_bar_height as usize,
            ),
            self.min_scroll_bar_height() as usize,
        );
        self.set_viewport(pane.pane_id(), Some(row), dims);
        context.invalidate();
        self.dragging.replace((item, start_event));
    }

    fn drag_ui_item(
        &mut self,
        item: UIItem,
        start_event: MouseEvent,
        x: usize,
        y: i64,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        match item.item_type {
            UIItemType::Split(split) => {
                self.drag_split(item, split, start_event, x, y, context);
            }
            UIItemType::ScrollThumb => {
                self.drag_scroll_thumb(item, start_event, event, context);
            }
            _ => {
                log::error!("drag not implemented for {:?}", item);
            }
        }
    }

    fn mouse_event_ui_item(
        &mut self,
        item: UIItem,
        pane: Arc<dyn Pane>,
        _y: i64,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        self.last_ui_item.replace(item.clone());
        match item.item_type {
            UIItemType::TabBar(item) => {
                self.mouse_event_tab_bar(item, event, context);
            }
            UIItemType::AboveScrollThumb => {
                self.mouse_event_above_scroll_thumb(item, pane, event, context);
            }
            UIItemType::ScrollThumb => {
                self.mouse_event_scroll_thumb(item, pane, event, context);
            }
            UIItemType::BelowScrollThumb => {
                self.mouse_event_below_scroll_thumb(item, pane, event, context);
            }
            UIItemType::Split(split) => {
                self.mouse_event_split(item, split, event, context);
            }
            UIItemType::CloseTab(idx) => {
                self.mouse_event_close_tab(idx, event, context);
            }
            UIItemType::AiPanel => {
                self.mouse_event_ai_panel(event, context);
            }
            UIItemType::AiPanelInput => {
                self.mouse_event_ai_panel_input(event, context);
            }
            UIItemType::AiPanelExecute(ref cmd) => {
                self.mouse_event_ai_panel_execute(cmd.clone(), event, context);
            }
            UIItemType::StatusBarProject => {
                self.mouse_event_status_bar_project(event, context);
            }
            UIItemType::StatusBarTheme => {
                self.mouse_event_theme_selector(event, context);
            }
            UIItemType::StatusBarCapture => {
                self.mouse_event_status_bar_capture(event, context);
            }
            UIItemType::StatusBarAdmin => {
                self.mouse_event_status_bar_admin(event, context);
            }
            UIItemType::StatusBarProxy => {
                self.mouse_event_status_bar_proxy(event, context);
            }
            UIItemType::StatusBarCommand => {
                self.mouse_event_status_bar_command(event, context);
            }
        }
    }

    fn mouse_event_ai_panel(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        use crate::ai::models::ai_state;
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                log::info!("AI panel body clicked — unfocusing chat");
                ai_state().set_chat_focused(false);
                context.invalidate();
            }
            WMEK::VertWheel(amount) => {
                let state = ai_state();
                if amount > 0 {
                    state.scroll_up(3);
                } else {
                    state.scroll_down(3);
                }
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Arrow));
    }

    fn mouse_event_ai_panel_input(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        use crate::ai::models::ai_state;
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                log::info!("AI panel input clicked — focusing chat");
                ai_state().set_chat_focused(true);
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Text));
    }

    fn mouse_event_ai_panel_execute(
        &mut self,
        cmd: String,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                log::info!("AI panel execute clicked: {}", cmd);
                // Send the command to the active pane
                if let Some(pane) = self.get_active_pane_or_overlay() {
                    let mut writer = pane.writer();
                    // Send the command followed by Enter
                    let _ = writer.write_all(cmd.as_bytes());
                    let _ = writer.write_all(b"\r");
                }
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Hand));
    }

    fn mouse_event_theme_selector(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                let message = match crate::overlay::theme_selector::cycle_theme() {
                    Ok((name, scheme)) => {
                        if let Err(err) = self.apply_client_theme_palette(&scheme) {
                            log::error!("theme palette apply failed: {err:#}");
                            format!("Theme saved as {name}, palette apply failed: {err:#}")
                        } else {
                            format!("Theme switched to {name}.")
                        }
                    }
                    Err(err) => {
                        log::error!("theme cycle failed: {err:#}");
                        format!("Theme switch failed: {err:#}")
                    }
                };
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    write_unterm_status_to_pane(&pane, &message);
                }
                context.invalidate();
            }
            WMEK::Press(MousePress::Right) => {
                self.show_theme_selector();
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Hand));
    }

    fn apply_client_theme_palette(&mut self, scheme: &str) -> anyhow::Result<()> {
        let mut theme_config: config::Config = (*self.config).clone();
        theme_config.color_scheme = Some(scheme.to_string());
        let Some(palette) = theme_config.resolve_color_scheme().cloned() else {
            anyhow::bail!("unknown color scheme: {scheme}");
        };
        let palette: wezterm_term::color::ColorPalette = palette.into();
        self.palette.replace(palette.clone());

        let term_config = Arc::new(config::TermConfig::with_config(self.config.clone()));
        term_config.set_client_palette(palette);
        let term_config: Arc<dyn wezterm_term::config::TerminalConfiguration> = term_config;

        let mux = Mux::get();
        if let Some(window) = mux.get_window(self.mux_window_id) {
            for tab in window.iter() {
                for pane in tab.iter_panes_ignoring_zoom() {
                    pane.pane.set_config(Arc::clone(&term_config));
                }
            }
        }
        Ok(())
    }

    fn mouse_event_status_bar_capture(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    write_unterm_status_to_pane(
                        &pane,
                        "Hidden-window region screenshot started. Drag a region or press Esc to cancel.",
                    );
                    std::thread::spawn(move || {
                        let message = match capture_selected_region_to_file(true) {
                            Ok(path) => format!(
                                "Hidden-window region screenshot saved and copied: {}",
                                path.display()
                            ),
                            Err(err) => {
                                log::error!("status bar hidden region capture failed: {err:#}");
                                format!("Hidden-window region screenshot failed: {err:#}")
                            }
                        };
                        write_unterm_status_to_pane(&pane, &message);
                    });
                }
                context.invalidate();
            }
            WMEK::Press(MousePress::Right) => {
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    write_unterm_status_to_pane(
                        &pane,
                        "Visible-window region screenshot started. Drag a region or press Esc to cancel.",
                    );
                    std::thread::spawn(move || {
                        let message = match capture_selected_region_to_file(false) {
                            Ok(path) => format!(
                                "Visible-window region screenshot saved and copied: {}",
                                path.display()
                            ),
                            Err(err) => {
                                log::error!("status bar visible region capture failed: {err:#}");
                                format!("Visible-window region screenshot failed: {err:#}")
                            }
                        };
                        write_unterm_status_to_pane(&pane, &message);
                    });
                }
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Hand));
    }

    fn mouse_event_status_bar_admin(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                let message = match launch_admin_window() {
                    Ok(()) => "Admin PowerShell requested. Accept the UAC prompt.".to_string(),
                    Err(err) => {
                        log::error!("status bar admin launch failed: {err:#}");
                        format!("Admin launch failed: {err:#}")
                    }
                };
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    write_unterm_status_to_pane(&pane, &message);
                }
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Hand));
    }

    fn mouse_event_status_bar_proxy(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                let message = match toggle_unterm_proxy_enabled() {
                    Ok(true) => "Proxy enabled for new shells.".to_string(),
                    Ok(false) => "Proxy disabled for new shells.".to_string(),
                    Err(err) => {
                        log::error!("proxy toggle failed: {err:#}");
                        format!("Proxy toggle failed: {err:#}")
                    }
                };
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    write_unterm_status_to_pane(&pane, &message);
                }
                context.invalidate();
            }
            WMEK::Press(MousePress::Right) => {
                self.show_proxy_settings();
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Hand));
    }

    fn mouse_event_status_bar_project(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                let Some(pane) = self.get_active_pane_no_overlay() else {
                    return;
                };
                let pane_id = pane.pane_id();
                let Some(window) = self.window.as_ref().cloned() else {
                    return;
                };
                write_unterm_status_to_pane(
                    &pane,
                    "Select a project directory to open in a new tab.",
                );
                std::thread::spawn(move || {
                    open_project_directory_in_new_tab(window, pane_id);
                });
                context.invalidate();
            }
            WMEK::Press(MousePress::Right) => {
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    if let Some(cwd) =
                        pane.get_current_working_dir(mux::pane::CachePolicy::AllowStale)
                    {
                        write_unterm_status_to_pane(&pane, &format!("Current project: {cwd}"));
                    }
                }
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Hand));
    }

    fn mouse_event_status_bar_command(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                self.show_context_menu();
                context.invalidate();
            }
            WMEK::Press(MousePress::Right) => {
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    let _ =
                        self.perform_key_assignment(&pane, &KeyAssignment::ActivateCommandPalette);
                }
                context.invalidate();
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Hand));
    }

    pub fn mouse_event_close_tab(
        &mut self,
        idx: usize,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                log::debug!("Should close tab {}", idx);
                self.close_specific_tab(idx, true);
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Arrow));
    }

    fn do_new_tab_button_click(&mut self, button: MousePress) {
        let pane = match self.get_active_pane_or_overlay() {
            Some(pane) => pane,
            None => return,
        };
        let action = match button {
            MousePress::Left => Some(KeyAssignment::ShowShellSelector),
            MousePress::Right => Some(KeyAssignment::ShowLauncher),
            MousePress::Middle => None,
        };

        async fn dispatch_new_tab_button(
            lua: Option<Rc<mlua::Lua>>,
            window: GuiWin,
            pane: MuxPane,
            button: MousePress,
            action: Option<KeyAssignment>,
        ) -> anyhow::Result<()> {
            let default_action = match lua {
                Some(lua) => {
                    let args = lua.pack_multi((
                        window.clone(),
                        pane,
                        format!("{button:?}"),
                        action.clone(),
                    ))?;
                    config::lua::emit_event(&lua, ("new-tab-button-click".to_string(), args))
                        .await
                        .map_err(|e| {
                            log::error!("while processing new-tab-button-click event: {:#}", e);
                            e
                        })?
                }
                None => true,
            };
            if let (true, Some(assignment)) = (default_action, action) {
                window.window.notify(TermWindowNotif::PerformAssignment {
                    pane_id: pane.0,
                    assignment,
                    tx: None,
                });
            }
            Ok(())
        }
        let window = GuiWin::new(self);
        let pane = MuxPane(pane.pane_id());
        promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
            dispatch_new_tab_button(lua, window, pane, button, action)
        }))
        .detach();
    }

    pub fn mouse_event_tab_bar(
        &mut self,
        item: TabBarItem,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        match event.kind {
            WMEK::Press(MousePress::Left) => match item {
                TabBarItem::Tab { tab_idx, .. } => {
                    self.activate_tab(tab_idx as isize).ok();
                }
                TabBarItem::NewTabButton { .. } => {
                    self.do_new_tab_button_click(MousePress::Left);
                }
                TabBarItem::MenuButton => {
                    self.show_context_menu();
                }
                TabBarItem::None | TabBarItem::LeftStatus | TabBarItem::RightStatus => {
                    let maximized = self
                        .window_state
                        .intersects(WindowState::MAXIMIZED | WindowState::FULL_SCREEN);
                    if let Some(ref window) = self.window {
                        if self.config.window_decorations
                            == WindowDecorations::INTEGRATED_BUTTONS | WindowDecorations::RESIZE
                        {
                            if self.last_mouse_click.as_ref().map(|c| c.streak) == Some(2) {
                                if maximized {
                                    window.restore();
                                } else {
                                    window.maximize();
                                }
                            }
                        }
                    }
                    // Potentially starting a drag by the tab bar
                    if !maximized {
                        self.window_drag_position.replace(event.clone());
                    }
                    context.request_drag_move();
                }
                TabBarItem::WindowButton(button) => {
                    use window::IntegratedTitleButton as Button;
                    if let Some(ref window) = self.window {
                        match button {
                            Button::Hide => window.hide(),
                            Button::Maximize => {
                                let maximized = self
                                    .window_state
                                    .intersects(WindowState::MAXIMIZED | WindowState::FULL_SCREEN);
                                if maximized {
                                    window.restore();
                                } else {
                                    window.maximize();
                                }
                            }
                            Button::Close => self.close_requested(&window.clone()),
                        }
                    }
                }
            },
            WMEK::Press(MousePress::Middle) => match item {
                TabBarItem::Tab { tab_idx, .. } => {
                    self.close_specific_tab(tab_idx, true);
                }
                TabBarItem::NewTabButton { .. } => {
                    self.do_new_tab_button_click(MousePress::Middle);
                }
                TabBarItem::None
                | TabBarItem::LeftStatus
                | TabBarItem::RightStatus
                | TabBarItem::MenuButton
                | TabBarItem::WindowButton(_) => {}
            },
            WMEK::Press(MousePress::Right) => match item {
                TabBarItem::Tab { tab_idx, .. } => {
                    self.show_tab_context_menu(tab_idx);
                }
                TabBarItem::NewTabButton { .. } => {
                    self.do_new_tab_button_click(MousePress::Right);
                }
                TabBarItem::MenuButton => {
                    self.show_context_menu();
                }
                TabBarItem::None
                | TabBarItem::LeftStatus
                | TabBarItem::RightStatus
                | TabBarItem::WindowButton(_) => {}
            },
            WMEK::Move => match item {
                TabBarItem::None | TabBarItem::LeftStatus | TabBarItem::RightStatus => {
                    context.set_window_drag_position(event.screen_coords);
                }
                TabBarItem::WindowButton(window::IntegratedTitleButton::Maximize) => {
                    let item = self.last_ui_item.clone().unwrap();
                    let bounds: ::window::ScreenRect = euclid::rect(
                        item.x as isize - (event.coords.x as isize - event.screen_coords.x),
                        item.y as isize - (event.coords.y as isize - event.screen_coords.y),
                        item.width as isize,
                        item.height as isize,
                    );
                    context.set_maximize_button_position(bounds);
                }
                TabBarItem::WindowButton(_)
                | TabBarItem::Tab { .. }
                | TabBarItem::NewTabButton { .. }
                | TabBarItem::MenuButton => {}
            },
            WMEK::VertWheel(n) => {
                if self.config.mouse_wheel_scrolls_tabs {
                    self.activate_tab_relative(if n < 1 { 1 } else { -1 }, true)
                        .ok();
                }
            }
            _ => {}
        }
        context.set_cursor(Some(MouseCursor::Arrow));
    }

    pub fn mouse_event_above_scroll_thumb(
        &mut self,
        _item: UIItem,
        pane: Arc<dyn Pane>,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        if let WMEK::Press(MousePress::Left) = event.kind {
            let dims = pane.get_dimensions();
            let current_viewport = self.get_viewport(pane.pane_id());
            // Page up
            self.set_viewport(
                pane.pane_id(),
                Some(
                    current_viewport
                        .unwrap_or(dims.physical_top)
                        .saturating_sub(self.terminal_size.rows.try_into().unwrap()),
                ),
                dims,
            );
            context.invalidate();
        }
        context.set_cursor(Some(MouseCursor::Arrow));
    }

    pub fn mouse_event_below_scroll_thumb(
        &mut self,
        _item: UIItem,
        pane: Arc<dyn Pane>,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        if let WMEK::Press(MousePress::Left) = event.kind {
            let dims = pane.get_dimensions();
            let current_viewport = self.get_viewport(pane.pane_id());
            // Page down
            self.set_viewport(
                pane.pane_id(),
                Some(
                    current_viewport
                        .unwrap_or(dims.physical_top)
                        .saturating_add(self.terminal_size.rows.try_into().unwrap()),
                ),
                dims,
            );
            context.invalidate();
        }
        context.set_cursor(Some(MouseCursor::Arrow));
    }

    pub fn mouse_event_scroll_thumb(
        &mut self,
        item: UIItem,
        _pane: Arc<dyn Pane>,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        if let WMEK::Press(MousePress::Left) = event.kind {
            // Start a scroll drag
            // self.scroll_drag_start = Some(from_top);
            self.dragging = Some((item, event));
        }
        context.set_cursor(Some(MouseCursor::Arrow));
    }

    pub fn mouse_event_split(
        &mut self,
        item: UIItem,
        split: PositionedSplit,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        context.set_cursor(Some(match &split.direction {
            SplitDirection::Horizontal => MouseCursor::SizeLeftRight,
            SplitDirection::Vertical => MouseCursor::SizeUpDown,
        }));

        if event.kind == WMEK::Press(MousePress::Left) {
            self.dragging.replace((item, event));
        }
    }

    fn mouse_event_terminal(
        &mut self,
        mut pane: Arc<dyn Pane>,
        position: ClickPosition,
        event: MouseEvent,
        context: &dyn WindowOps,
        capture_mouse: bool,
    ) {
        let mut is_click_to_focus_pane = false;

        let ClickPosition {
            mut column,
            mut row,
            mut x_pixel_offset,
            mut y_pixel_offset,
        } = position;

        let is_already_captured = matches!(
            self.current_mouse_capture,
            Some(MouseCapture::TerminalPane(_))
        );

        for pos in self.get_panes_to_render() {
            if !is_already_captured
                && row >= pos.top as i64
                && row <= (pos.top + pos.height) as i64
                && column >= pos.left
                && column <= pos.left + pos.width
            {
                if pane.pane_id() != pos.pane.pane_id() {
                    // We're over a pane that isn't active
                    match &event.kind {
                        WMEK::Press(_) => {
                            let mux = Mux::get();
                            mux.get_active_tab_for_window(self.mux_window_id)
                                .map(|tab| tab.set_active_idx(pos.index));

                            pane = Arc::clone(&pos.pane);
                            is_click_to_focus_pane = true;
                        }
                        WMEK::Move => {
                            if self.config.pane_focus_follows_mouse {
                                let mux = Mux::get();
                                mux.get_active_tab_for_window(self.mux_window_id)
                                    .map(|tab| tab.set_active_idx(pos.index));

                                pane = Arc::clone(&pos.pane);
                                context.invalidate();
                            }
                        }
                        WMEK::Release(_) | WMEK::HorzWheel(_) => {}
                        WMEK::VertWheel(_) => {
                            // Let wheel events route to the hovered pane,
                            // even if it doesn't have focus
                            pane = Arc::clone(&pos.pane);
                            context.invalidate();
                        }
                    }
                }
                column = column.saturating_sub(pos.left);
                row = row.saturating_sub(pos.top as i64);
                break;
            } else if is_already_captured && pane.pane_id() == pos.pane.pane_id() {
                column = column.saturating_sub(pos.left);
                row = row.saturating_sub(pos.top as i64).max(0);

                if position.column < pos.left {
                    x_pixel_offset -= self.render_metrics.cell_size.width
                        * (pos.left as isize - position.column as isize);
                }
                if position.row < pos.top as i64 {
                    y_pixel_offset -= self.render_metrics.cell_size.height
                        * (pos.top as isize - position.row as isize);
                }

                break;
            }
        }

        if capture_mouse {
            self.current_mouse_capture = Some(MouseCapture::TerminalPane(pane.pane_id()));
        }

        let is_focused = if let Some(focused) = self.focused.as_ref() {
            !self.config.swallow_mouse_click_on_window_focus
                || (focused.elapsed() > Duration::from_millis(200))
        } else {
            false
        };

        if self.focused.is_some() && !is_focused {
            if matches!(&event.kind, WMEK::Press(_))
                && self.config.swallow_mouse_click_on_window_focus
            {
                // Entering click to focus state
                self.is_click_to_focus_window = true;
                context.invalidate();
                log::trace!("enter click to focus");
                return;
            }
        }
        if self.is_click_to_focus_window && matches!(&event.kind, WMEK::Release(_)) {
            // Exiting click to focus state
            self.is_click_to_focus_window = false;
            context.invalidate();
            log::trace!("exit click to focus");
            return;
        }

        let allow_action = if self.is_click_to_focus_window || !is_focused {
            matches!(&event.kind, WMEK::VertWheel(_) | WMEK::HorzWheel(_))
        } else {
            true
        };

        log::trace!(
            "is_focused={} allow_action={} event={:?}",
            is_focused,
            allow_action,
            event
        );

        let dims = pane.get_dimensions();
        let stable_row = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top)
            + row as StableRowIndex;

        self.pane_state(pane.pane_id())
            .mouse_terminal_coords
            .replace((
                ClickPosition {
                    column,
                    row,
                    x_pixel_offset,
                    y_pixel_offset,
                },
                stable_row,
            ));

        pane.apply_hyperlinks(stable_row..stable_row + 1, &self.config.hyperlink_rules);

        struct FindCurrentLink {
            current: Option<Arc<Hyperlink>>,
            stable_row: StableRowIndex,
            column: usize,
        }

        impl WithPaneLines for FindCurrentLink {
            fn with_lines_mut(&mut self, stable_top: StableRowIndex, lines: &mut [&mut Line]) {
                if stable_top == self.stable_row {
                    if let Some(line) = lines.get(0) {
                        if let Some(cell) = line.get_cell(self.column) {
                            self.current = cell.attrs().hyperlink().cloned();
                        }
                    }
                }
            }
        }

        let mut find_link = FindCurrentLink {
            current: None,
            stable_row,
            column,
        };
        pane.with_lines_mut(stable_row..stable_row + 1, &mut find_link);
        let new_highlight = find_link.current;

        match (self.current_highlight.as_ref(), new_highlight) {
            (Some(old_link), Some(new_link)) if Arc::ptr_eq(&old_link, &new_link) => {
                // Unchanged
            }
            (None, None) => {
                // Unchanged
            }
            (_, rhs) => {
                // We're hovering over a different URL, so invalidate and repaint
                // so that we render the underline correctly
                self.current_highlight = rhs;
                context.invalidate();
            }
        };

        let outside_window = event.coords.x < 0
            || event.coords.x as usize > self.dimensions.pixel_width
            || event.coords.y < 0
            || event.coords.y as usize > self.dimensions.pixel_height;

        context.set_cursor(Some(if self.current_highlight.is_some() {
            // When hovering over a hyperlink, show an appropriate
            // mouse cursor to give the cue that it is clickable
            MouseCursor::Hand
        } else if pane.is_mouse_grabbed() || outside_window {
            MouseCursor::Arrow
        } else {
            MouseCursor::Text
        }));

        let event_trigger_type = match &event.kind {
            WMEK::Press(press) => {
                let press = mouse_press_to_tmb(press);
                match self.last_mouse_click.as_ref() {
                    Some(LastMouseClick { streak, button, .. }) if *button == press => {
                        Some(MouseEventTrigger::Down {
                            streak: *streak,
                            button: press,
                        })
                    }
                    _ => None,
                }
            }
            WMEK::Release(press) => {
                let press = mouse_press_to_tmb(press);
                match self.last_mouse_click.as_ref() {
                    Some(LastMouseClick { streak, button, .. }) if *button == press => {
                        Some(MouseEventTrigger::Up {
                            streak: *streak,
                            button: press,
                        })
                    }
                    _ => None,
                }
            }
            WMEK::Move => {
                if !self.current_mouse_buttons.is_empty() {
                    if let Some(LastMouseClick { streak, button, .. }) =
                        self.last_mouse_click.as_ref()
                    {
                        if Some(*button)
                            == self.current_mouse_buttons.last().map(mouse_press_to_tmb)
                        {
                            Some(MouseEventTrigger::Drag {
                                streak: *streak,
                                button: *button,
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            WMEK::VertWheel(amount) => Some(match *amount {
                0 => return,
                1.. => MouseEventTrigger::Down {
                    streak: 1,
                    button: MouseButton::WheelUp(*amount as usize),
                },
                _ => MouseEventTrigger::Down {
                    streak: 1,
                    button: MouseButton::WheelDown(-amount as usize),
                },
            }),
            WMEK::HorzWheel(amount) => Some(match *amount {
                0 => return,
                1.. => MouseEventTrigger::Down {
                    streak: 1,
                    button: MouseButton::WheelLeft(*amount as usize),
                },
                _ => MouseEventTrigger::Down {
                    streak: 1,
                    button: MouseButton::WheelRight(-amount as usize),
                },
            }),
        };

        if allow_action {
            if let Some(mut event_trigger_type) = event_trigger_type {
                self.current_event = Some(event_trigger_type.to_dynamic());
                let mut modifiers = event.modifiers;

                // Since we use shift to force assessing the mouse bindings, pretend
                // that shift is not one of the mods when the mouse is grabbed.
                let mut mouse_reporting = pane.is_mouse_grabbed();
                if mouse_reporting {
                    if modifiers.contains(self.config.bypass_mouse_reporting_modifiers) {
                        modifiers.remove(self.config.bypass_mouse_reporting_modifiers);
                        mouse_reporting = false;
                    }
                }

                if mouse_reporting {
                    // If they were scrolled back prior to launching an
                    // application that captures the mouse, then mouse based
                    // scrolling assignments won't have any effect.
                    // Ensure that we scroll to the bottom if they try to
                    // use the mouse so that things are less surprising
                    self.scroll_to_bottom(&pane);
                }

                // normalize delta and streak to make mouse assignment
                // easier to wrangle
                match event_trigger_type {
                    MouseEventTrigger::Down {
                        ref mut streak,
                        button:
                            MouseButton::WheelUp(ref mut delta)
                            | MouseButton::WheelDown(ref mut delta)
                            | MouseButton::WheelLeft(ref mut delta)
                            | MouseButton::WheelRight(ref mut delta),
                    }
                    | MouseEventTrigger::Up {
                        ref mut streak,
                        button:
                            MouseButton::WheelUp(ref mut delta)
                            | MouseButton::WheelDown(ref mut delta)
                            | MouseButton::WheelLeft(ref mut delta)
                            | MouseButton::WheelRight(ref mut delta),
                    }
                    | MouseEventTrigger::Drag {
                        ref mut streak,
                        button:
                            MouseButton::WheelUp(ref mut delta)
                            | MouseButton::WheelDown(ref mut delta)
                            | MouseButton::WheelLeft(ref mut delta)
                            | MouseButton::WheelRight(ref mut delta),
                    } => {
                        *streak = 1;
                        *delta = 1;
                    }
                    _ => {}
                };

                let mouse_mods = config::MouseEventTriggerMods {
                    mods: modifiers,
                    mouse_reporting,
                    alt_screen: if pane.is_alt_screen_active() {
                        MouseEventAltScreen::True
                    } else {
                        MouseEventAltScreen::False
                    },
                };

                // Unterm: intercept right-click for native context menu
                // before the input_map can handle it (WezTerm default: paste)
                #[cfg(windows)]
                if !pane.is_mouse_grabbed() {
                    if let WMEK::Press(MousePress::Right) = &event.kind {
                        log::info!("Unterm: right-click detected, showing context menu");
                        self.show_context_menu();
                        context.invalidate();
                        return;
                    }
                }

                if let Some(action) = self.input_map.lookup_mouse(event_trigger_type, mouse_mods) {
                    self.perform_key_assignment(&pane, &action).ok();
                    return;
                }
            }
        }

        let mouse_event = wezterm_term::MouseEvent {
            kind: match event.kind {
                WMEK::Move => TMEK::Move,
                WMEK::VertWheel(_) | WMEK::HorzWheel(_) | WMEK::Press(_) => TMEK::Press,
                WMEK::Release(_) => TMEK::Release,
            },
            button: match event.kind {
                WMEK::Release(ref press) | WMEK::Press(ref press) => mouse_press_to_tmb(press),
                WMEK::Move => {
                    if event.mouse_buttons == WMB::LEFT {
                        TMB::Left
                    } else if event.mouse_buttons == WMB::RIGHT {
                        TMB::Right
                    } else if event.mouse_buttons == WMB::MIDDLE {
                        TMB::Middle
                    } else {
                        TMB::None
                    }
                }
                WMEK::VertWheel(amount) => {
                    if amount > 0 {
                        TMB::WheelUp(amount as usize)
                    } else {
                        TMB::WheelDown((-amount) as usize)
                    }
                }
                WMEK::HorzWheel(amount) => {
                    if amount > 0 {
                        TMB::WheelLeft(amount as usize)
                    } else {
                        TMB::WheelRight((-amount) as usize)
                    }
                }
            },
            x: column,
            y: row,
            x_pixel_offset,
            y_pixel_offset,
            modifiers: event.modifiers,
        };

        if allow_action
            && !(self.config.swallow_mouse_click_on_pane_focus && is_click_to_focus_pane)
        {
            pane.mouse_event(mouse_event).ok();
        }

        match event.kind {
            WMEK::Move => {}
            _ => {
                context.invalidate();
            }
        }
    }
}

fn mouse_press_to_tmb(press: &MousePress) -> TMB {
    match press {
        MousePress::Left => TMB::Left,
        MousePress::Right => TMB::Right,
        MousePress::Middle => TMB::Middle,
    }
}

pub(crate) fn write_unterm_status_to_pane(pane: &Arc<dyn Pane>, message: &str) {
    let mut writer = pane.writer();
    let message = message.replace('\'', "''");
    let _ = writer.write_all(format!("\r\nWrite-Host '[Unterm] {message}'\r\n").as_bytes());
}

fn toggle_unterm_proxy_enabled() -> anyhow::Result<bool> {
    let path = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("proxy.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "enabled": false,
                "current_node": "local",
                "http_proxy": "http://127.0.0.1:7890",
                "socks_proxy": "socks5://127.0.0.1:7890",
                "no_proxy": "localhost,127.0.0.1,::1",
                "nodes": [
                    {"name": "local", "url": "http://127.0.0.1:7890", "kind": "http"}
                ]
            })
        });
    let next = !value
        .get("enabled")
        .and_then(|enabled| enabled.as_bool())
        .unwrap_or(false);
    value["enabled"] = serde_json::Value::Bool(next);
    std::fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(next)
}

#[cfg(windows)]
pub(crate) fn open_project_directory_in_new_tab(window: ::window::Window, pane_id: mux::pane::PaneId) {
    match pick_project_directory() {
        Ok(path) => {
            if let Err(err) = save_recent_project_directory(&path) {
                log::warn!("failed to save recent project directory: {err:#}");
            }
            window.notify(TermWindowNotif::PerformAssignment {
                pane_id,
                assignment: KeyAssignment::SpawnCommandInNewTab(SpawnCommand {
                    cwd: Some(path),
                    ..SpawnCommand::default()
                }),
                tx: None,
            });
        }
        Err(err) => {
            log::warn!("project directory picker did not return a directory: {err:#}");
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn open_project_directory_in_new_tab(_window: ::window::Window, _pane_id: mux::pane::PaneId) {
    log::warn!("project directory picker is only supported on Windows");
}

#[cfg(windows)]
fn pick_project_directory() -> anyhow::Result<std::path::PathBuf> {
    use base64::Engine as _;

    let script = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$dlg = New-Object System.Windows.Forms.FolderBrowserDialog
$dlg.Description = 'Select project directory'
$dlg.ShowNewFolderButton = $true
if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  [Console]::Out.Write($dlg.SelectedPath)
}
"#;
    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut command = std::process::Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-STA",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &encoded,
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output()?;
    if !output.status.success() {
        anyhow::bail!("PowerShell folder picker returned {}", output.status);
    }
    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        anyhow::bail!("project directory selection was canceled");
    }
    let path = std::path::PathBuf::from(selected);
    if !path.is_dir() {
        anyhow::bail!("selected path is not a directory: {}", path.display());
    }
    Ok(path)
}

#[cfg(windows)]
fn save_recent_project_directory(path: &std::path::Path) -> anyhow::Result<()> {
    let config_path = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("projects.json");
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut projects = std::fs::read_to_string(&config_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Vec<String>>(&content).ok())
        .unwrap_or_default();
    let path = path.display().to_string();
    projects.retain(|existing| existing != &path);
    projects.insert(0, path);
    projects.truncate(12);
    std::fs::write(config_path, serde_json::to_string_pretty(&projects)?)?;
    Ok(())
}

#[cfg(windows)]
fn launch_admin_window() -> anyhow::Result<()> {
    let gui_exe = std::env::current_exe()?;
    let launch_exe = admin_launcher_exe(&gui_exe);
    let cwd = gui_exe
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let pwsh = "C:\\Program Files\\PowerShell\\7\\pwsh.exe";
    let shell = if std::path::Path::new(pwsh).exists() {
        pwsh
    } else {
        "powershell.exe"
    };
    let exe = powershell_single_quote(&launch_exe.display().to_string());
    let cwd = powershell_single_quote(&cwd.display().to_string());
    let arg_list = powershell_single_quote(&format!(
        "start --always-new-process -- \"{}\" -NoLogo",
        shell
    ));
    let script = format!(
        "Start-Process -Verb RunAs -FilePath '{exe}' -WorkingDirectory '{cwd}' -ArgumentList '{arg_list}'"
    );
    let mut command = std::process::Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &script,
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("PowerShell Start-Process returned {status}");
    }
    Ok(())
}

#[cfg(windows)]
fn admin_launcher_exe(gui_exe: &std::path::Path) -> std::path::PathBuf {
    let Some(dir) = gui_exe.parent() else {
        return gui_exe.to_path_buf();
    };
    let launcher = dir.join("Unterm.exe");
    let should_copy = match (std::fs::metadata(gui_exe), std::fs::metadata(&launcher)) {
        (Ok(src), Ok(dst)) => src.len() != dst.len() || src.modified().ok() != dst.modified().ok(),
        (Ok(_), Err(_)) => true,
        _ => false,
    };

    if should_copy {
        if let Err(err) = std::fs::copy(gui_exe, &launcher) {
            log::warn!(
                "failed to prepare Unterm.exe admin launcher at {}: {err:#}",
                launcher.display()
            );
        }
    }

    if launcher.exists() {
        launcher
    } else {
        gui_exe.to_path_buf()
    }
}

#[cfg(not(windows))]
fn launch_admin_window() -> anyhow::Result<()> {
    anyhow::bail!("administrator launch is only supported on Windows")
}

#[cfg(windows)]
pub(crate) fn capture_selected_region_to_file(hide_window: bool) -> anyhow::Result<std::path::PathBuf> {
    use base64::Engine as _;

    let pid = std::process::id();
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("screenshots");
    std::fs::create_dir_all(&dir)?;
    let prefix = if hide_window {
        "region_hidden"
    } else {
        "region_visible"
    };
    let output_path = dir.join(format!(
        "{}_{}.png",
        prefix,
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));
    let path = powershell_single_quote(&output_path.display().to_string());
    let hide_script = if hide_window {
        "foreach ($win in $windows) { [UntermStatusCapture]::ShowWindow($win, 0) | Out-Null }\nStart-Sleep -Milliseconds 350"
    } else {
        "[UntermStatusCapture]::SetForegroundWindow($hwnd) | Out-Null\nStart-Sleep -Milliseconds 120"
    };
    let restore_script = if hide_window {
        "foreach ($win in $windows) { [UntermStatusCapture]::ShowWindow($win, 5) | Out-Null }\n  [UntermStatusCapture]::SetForegroundWindow($hwnd) | Out-Null"
    } else {
        "[UntermStatusCapture]::SetForegroundWindow($hwnd) | Out-Null"
    };
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class UntermStatusCapture {{
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
  public static IntPtr[] WindowsForPid(uint pid) {{
    var windows = new List<IntPtr>();
    EnumWindows((hWnd, lParam) => {{
      uint windowPid;
      GetWindowThreadProcessId(hWnd, out windowPid);
      if (windowPid == pid && IsWindowVisible(hWnd)) {{
        windows.Add(hWnd);
      }}
      return true;
    }}, IntPtr.Zero);
    return windows.ToArray();
  }}
}}
"@
$proc = Get-Process -Id {pid} -ErrorAction Stop
$windows = [UntermStatusCapture]::WindowsForPid([uint32]$proc.Id)
if ($windows.Count -eq 0) {{ throw "No visible window handle" }}
$hwnd = $windows[0]
{hide_script}
try {{
  [System.Windows.Forms.Clipboard]::Clear()
  Start-Process "ms-screenclip:"
  $deadline = [DateTime]::Now.AddSeconds(90)
  $image = $null
  while ([DateTime]::Now -lt $deadline) {{
    Start-Sleep -Milliseconds 250
    if ([System.Windows.Forms.Clipboard]::ContainsImage()) {{
      $image = [System.Windows.Forms.Clipboard]::GetImage()
      break
    }}
  }}
  if ($image -eq $null) {{ throw "Screenshot canceled or timed out" }}
  $image.Save('{path}', [System.Drawing.Imaging.ImageFormat]::Png)
  $clipboardImage = [System.Drawing.Image]::FromFile('{path}')
  $pngBytes = [System.IO.File]::ReadAllBytes('{path}')
  $pngStream = New-Object System.IO.MemoryStream
  $pngStream.Write($pngBytes, 0, $pngBytes.Length)
  $pngStream.Position = 0
  $fileDrop = New-Object System.Collections.Specialized.StringCollection
  [void]$fileDrop.Add('{path}')
  $data = New-Object System.Windows.Forms.DataObject
  $data.SetImage($clipboardImage)
  $data.SetFileDropList($fileDrop)
  $data.SetText('{path}')
  $data.SetData('PNG', $false, $pngStream)
  $data.SetData('image/png', $false, $pngStream)
  try {{
    $set = $false
    for ($i = 0; $i -lt 10 -and -not $set; $i++) {{
      try {{
        [System.Windows.Forms.Clipboard]::SetDataObject($data, $true)
        $set = $true
      }} catch {{
        Start-Sleep -Milliseconds 120
      }}
    }}
    if (-not $set) {{ throw "Clipboard is busy" }}
  }} finally {{
    $clipboardImage.Dispose()
    $pngStream.Dispose()
  }}
  $image.Dispose()
}} finally {{
  {restore_script}
}}
"#
    );
    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut command = std::process::Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-STA",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &encoded,
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("PowerShell screenshot returned {status}");
    }
    if !output_path.exists() {
        anyhow::bail!("screenshot file was not created: {}", output_path.display());
    }
    Ok(output_path)
}

#[cfg(not(windows))]
pub(crate) fn capture_selected_region_to_file(_hide_window: bool) -> anyhow::Result<std::path::PathBuf> {
    anyhow::bail!("region capture is only supported on Windows")
}

#[cfg(windows)]
fn powershell_single_quote(text: &str) -> String {
    text.replace('\'', "''")
}
