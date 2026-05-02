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
            | UIItemType::StatusBarProject
            | UIItemType::StatusBarCwd
            | UIItemType::StatusBarTheme
            | UIItemType::StatusBarCaptureExclude
            | UIItemType::StatusBarCaptureInclude
            | UIItemType::StatusBarProxy
            | UIItemType::CloseSplitPane(_) => {}
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
            | UIItemType::StatusBarProject
            | UIItemType::StatusBarCwd
            | UIItemType::StatusBarTheme
            | UIItemType::StatusBarCaptureExclude
            | UIItemType::StatusBarCaptureInclude
            | UIItemType::StatusBarProxy
            | UIItemType::CloseSplitPane(_) => {}
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
            UIItemType::StatusBarProject => {
                self.mouse_event_status_bar_project(event, context);
            }
            UIItemType::StatusBarCwd => {
                self.mouse_event_status_bar_cwd(event, context);
            }
            UIItemType::StatusBarTheme => {
                self.mouse_event_theme_selector(event, context);
            }
            UIItemType::StatusBarCaptureExclude => {
                self.mouse_event_status_bar_capture(event, context, true);
            }
            UIItemType::StatusBarCaptureInclude => {
                self.mouse_event_status_bar_capture(event, context, false);
            }
            UIItemType::StatusBarProxy => {
                self.mouse_event_status_bar_proxy(event, context);
            }
            UIItemType::CloseSplitPane(pane_id) => {
                self.mouse_event_close_split_pane(pane_id, event, context);
            }
        }
    }

    fn mouse_event_close_split_pane(
        &mut self,
        pane_id: mux::pane::PaneId,
        event: MouseEvent,
        context: &dyn WindowOps,
    ) {
        if matches!(event.kind, WMEK::Press(MousePress::Left)) {
            self.close_pane_by_id(pane_id);
            context.invalidate();
        }
        context.set_cursor(Some(MouseCursor::Arrow));
    }

    fn mouse_event_theme_selector(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                let message = match crate::overlay::theme_selector::cycle_theme() {
                    Ok((name, scheme)) => {
                        if let Err(err) = self.apply_client_theme_palette(&scheme) {
                            log::error!("theme palette apply failed: {err:#}");
                            crate::i18n::t_args(
                                "theme.saved_palette_failed",
                                &[("name", &name), ("err", &format!("{err:#}"))],
                            )
                        } else {
                            crate::i18n::t_args(
                                "theme.switched_to",
                                &[("name", &name)],
                            )
                        }
                    }
                    Err(err) => {
                        log::error!("theme cycle failed: {err:#}");
                        crate::i18n::t_args(
                            "theme.switch_failed",
                            &[("err", &format!("{err:#}"))],
                        )
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

    fn mouse_event_status_bar_capture(
        &mut self,
        event: MouseEvent,
        context: &dyn WindowOps,
        hide_window: bool,
    ) {
        if let WMEK::Press(MousePress::Left) = event.kind {
            if let Some(pane) = self.get_active_pane_no_overlay() {
                capture_and_announce(&pane, hide_window);
            }
            context.invalidate();
        }
        context.set_cursor(Some(MouseCursor::Hand));
    }

    fn mouse_event_status_bar_admin(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        match event.kind {
            WMEK::Press(MousePress::Left) => {
                let message = match launch_admin_window() {
                    Ok(()) => crate::i18n::t("admin.requested"),
                    Err(err) => {
                        log::error!("status bar admin launch failed: {err:#}");
                        crate::i18n::t_args(
                            "admin.failed",
                            &[("err", &format!("{err:#}"))],
                        )
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
                    Ok(true) => crate::i18n::t("proxy.enabled_for_new_shells"),
                    Ok(false) => crate::i18n::t("proxy.disabled_for_new_shells"),
                    Err(err) => {
                        log::error!("proxy toggle failed: {err:#}");
                        crate::i18n::t_args(
                            "proxy.toggle_failed",
                            &[("err", &format!("{err:#}"))],
                        )
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

    /// Click on the CWD segment: copy the active pane's full cwd to the
    /// clipboard. Useful when you've navigated deep into a project tree and
    /// want to paste the path into another tool. No prompt — silent copy,
    /// the path is already shown in the bar so the user knows what landed.
    fn mouse_event_status_bar_cwd(&mut self, event: MouseEvent, context: &dyn WindowOps) {
        if !matches!(event.kind, WMEK::Press(MousePress::Left)) {
            return;
        }
        let Some(pane) = self.get_active_pane_no_overlay() else {
            return;
        };
        let Some(cwd) = pane.get_current_working_dir(mux::pane::CachePolicy::AllowStale) else {
            return;
        };
        let path = cwd
            .to_file_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| cwd.to_string());
        self.copy_to_clipboard(
            config::keyassignment::ClipboardCopyDestination::ClipboardAndPrimarySelection,
            path,
        );
        context.invalidate();
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
                    &crate::i18n::t("project.prompt_picker"),
                );
                std::thread::spawn(move || {
                    open_project_directory_in_new_tab(window, pane_id, None);
                });
                context.invalidate();
            }
            WMEK::Press(MousePress::Right) => {
                if let Some(pane) = self.get_active_pane_no_overlay() {
                    if let Some(cwd) =
                        pane.get_current_working_dir(mux::pane::CachePolicy::AllowStale)
                    {
                        write_unterm_status_to_pane(
                            &pane,
                            &crate::i18n::t_args(
                                "project.current",
                                &[("path", &cwd.to_string())],
                            ),
                        );
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
                    self.show_settings_menu();
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
                    self.show_settings_menu();
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

/// Run a region screenshot from the status bar and announce results back to
/// the user. After saving the PNG, copy the file path as plain text to the
/// system clipboard so the user can paste it into the terminal (or anywhere
/// else) directly.  The PNG bytes themselves were already copied to the
/// system *image* clipboard by `capture_selected_region_to_file` — that
/// covers image-aware paste targets like editors and chat windows.
///
/// We deliberately do *not* render the image inline via OSC 1337.  Forcing
/// hundreds of KB of escape data through the terminal's escape parser was
/// found to wedge the GUI on large captures, and the user's actual need is
/// "be able to send the screenshot back to the terminal" — pasting a path
/// satisfies that with no parser involvement.
pub(crate) fn capture_and_announce(pane: &Arc<dyn Pane>, hide_window: bool) {
    let mode_label = if hide_window {
        crate::i18n::t("screenshot.mode.hidden")
    } else {
        crate::i18n::t("screenshot.mode.visible")
    };
    write_unterm_status_to_pane(
        pane,
        &crate::i18n::t_args("screenshot.started", &[("mode", &mode_label)]),
    );
    let pane = pane.clone();
    let mode_label_thread = mode_label.clone();
    std::thread::spawn(move || match capture_selected_region_to_file(hide_window) {
        Ok(path) => {
            let path_str = path.display().to_string();
            if let Err(err) = copy_text_to_clipboard(&path_str) {
                log::warn!("could not copy screenshot path to clipboard: {err:#}");
            }
            write_unterm_status_to_pane(
                &pane,
                &crate::i18n::t_args(
                    "screenshot.saved",
                    &[("mode", &mode_label_thread), ("path", &path_str)],
                ),
            );
        }
        Err(err) => {
            log::error!("status bar region capture failed: {err:#}");
            write_unterm_status_to_pane(
                &pane,
                &crate::i18n::t_args(
                    "screenshot.failed",
                    &[("mode", &mode_label_thread), ("err", &format!("{err:#}"))],
                ),
            );
        }
    });
}

/// Put plain text on the system clipboard so the user can paste it as the
/// next command (or argument). Each platform has its own command.
#[cfg(target_os = "macos")]
pub(crate) fn copy_text_to_clipboard(text: &str) -> anyhow::Result<()> {
    use std::process::{Command, Stdio};
    let mut child = Command::new("/usr/bin/pbcopy")
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        std::io::Write::write_all(stdin, text.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("pbcopy exited with {status}");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn copy_text_to_clipboard(text: &str) -> anyhow::Result<()> {
    use std::process::{Command, Stdio};
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("clip");
    cmd.stdin(Stdio::piped());
    cmd.creation_flags(0x08000000);
    let mut child = cmd.spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        std::io::Write::write_all(stdin, text.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("clip exited with {status}");
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn copy_text_to_clipboard(text: &str) -> anyhow::Result<()> {
    use std::process::{Command, Stdio};
    // Try wl-copy first (Wayland), fall back to xclip / xsel (X11).
    for (bin, args) in &[
        ("wl-copy", &[][..]),
        ("xclip", &["-selection", "clipboard"][..]),
        ("xsel", &["--clipboard", "--input"][..]),
    ] {
        let mut cmd = Command::new(bin);
        cmd.args(*args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut child = child;
        if let Some(stdin) = child.stdin.as_mut() {
            std::io::Write::write_all(stdin, text.as_bytes())?;
        }
        if child.wait()?.success() {
            return Ok(());
        }
    }
    anyhow::bail!("no usable clipboard tool (tried wl-copy, xclip, xsel)")
}

pub(crate) fn write_unterm_status_to_pane(_pane: &Arc<dyn Pane>, _message: &str) {
    // Intentionally a no-op.
    //
    // The earlier implementation injected `[Unterm] ...` banner lines
    // straight into the pane's output stream via perform_actions(). On
    // a naive shell that's harmless — text just lands above the prompt.
    // But Claude Code, vim, btop, lazygit, k9s, and basically every
    // ncurses/TUI app uses absolute cursor positioning and full-frame
    // repaints; an injected CRLF sequence in the middle of their render
    // shifts every subsequent absolute-positioned cell by one row, so
    // the user sees fragments of the next frame interleaved with stale
    // text from the previous frame ("内容多了以后开始乱序"). 2026-05-01
    // bug report attached a screenshot of Claude Code self-corruption.
    //
    // Subtraction principle: the action's effect is already visible
    // through the status bar, the system clipboard, and the file on
    // disk — yelling about it inside the pane was redundant and now
    // we know it's destructive. Keep the function symbol so the 19
    // existing call sites still compile, but it does nothing.
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

pub(crate) fn open_project_directory_in_new_tab(
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
    start_at: Option<std::path::PathBuf>,
) {
    #[cfg(windows)]
    let picked = pick_project_directory_starting_at(start_at.as_deref());
    #[cfg(not(windows))]
    let picked = pick_project_directory_unix_starting_at(start_at.as_deref());
    match picked {
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

#[cfg(target_os = "macos")]
pub(crate) fn pick_project_directory_unix() -> anyhow::Result<std::path::PathBuf> {
    pick_project_directory_unix_starting_at(None)
}

/// Pop the macOS folder picker, optionally pre-pointed at `start_at` so the
/// dialog opens at the user's current pane cwd (instead of always Documents).
/// User can immediately accept the start_at directory, or navigate elsewhere.
#[cfg(target_os = "macos")]
pub(crate) fn pick_project_directory_unix_starting_at(
    start_at: Option<&std::path::Path>,
) -> anyhow::Result<std::path::PathBuf> {
    let default_clause = match start_at {
        Some(p) if p.is_dir() => {
            // Escape embedded double quotes / backslashes for AppleScript string.
            let escaped = p
                .display()
                .to_string()
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            format!(" default location (POSIX file \"{}\")", escaped)
        }
        _ => String::new(),
    };
    let script = format!(
        r#"try
  POSIX path of (choose folder with prompt "Select project directory"{default_clause})
on error
  ""
end try"#
    );
    let output = std::process::Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(&script)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("osascript exited with {}", output.status);
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

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn pick_project_directory_unix() -> anyhow::Result<std::path::PathBuf> {
    pick_project_directory_unix_starting_at(None)
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn pick_project_directory_unix_starting_at(
    start_at: Option<&std::path::Path>,
) -> anyhow::Result<std::path::PathBuf> {
    let start_str: String = start_at
        .filter(|p| p.is_dir())
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    // zenity supports `--filename=<path>` to open the picker at <path>.
    // kdialog takes the start dir as positional arg. yad mirrors zenity.
    let mut zenity_filename = String::new();
    if !start_str.is_empty() {
        zenity_filename = format!("--filename={}/", start_str);
    }
    let kdialog_start: &str = if start_str.is_empty() { "." } else { &start_str };
    let candidates: Vec<(&str, Vec<String>)> = vec![
        (
            "zenity",
            vec![
                "--file-selection".into(),
                "--directory".into(),
                "--title=Select project directory".into(),
                zenity_filename.clone(),
            ],
        ),
        ("kdialog", vec!["--getexistingdirectory".into(), kdialog_start.to_string()]),
        (
            "yad",
            vec![
                "--file".into(),
                "--directory".into(),
                "--title=Select project directory".into(),
                zenity_filename,
            ],
        ),
    ];
    for (bin, args) in &candidates {
        let args: Vec<&str> = args.iter().filter(|s| !s.is_empty()).map(|s| s.as_str()).collect();
        let output = match std::process::Command::new(bin).args(&args).output() {
            Ok(o) => o,
            Err(_) => continue,
        };
        if !output.status.success() {
            continue;
        }
        let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if selected.is_empty() {
            anyhow::bail!("project directory selection was canceled");
        }
        let path = std::path::PathBuf::from(selected);
        if !path.is_dir() {
            anyhow::bail!("selected path is not a directory: {}", path.display());
        }
        return Ok(path);
    }
    anyhow::bail!("no usable directory picker (install one of: zenity, kdialog, yad)")
}

#[cfg(not(windows))]
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
pub(crate) fn pick_project_directory() -> anyhow::Result<std::path::PathBuf> {
    pick_project_directory_starting_at(None)
}

/// Pop the FolderBrowserDialog pre-pointed at `start_at` so the picker opens
/// at the current pane cwd instead of My Computer / Desktop.
#[cfg(windows)]
pub(crate) fn pick_project_directory_starting_at(
    start_at: Option<&std::path::Path>,
) -> anyhow::Result<std::path::PathBuf> {
    use base64::Engine as _;

    // Single-quote string literal in PowerShell — escape ' by doubling it.
    let initial_dir = start_at
        .filter(|p| p.is_dir())
        .map(|p| p.display().to_string().replace('\'', "''"))
        .unwrap_or_default();
    let initial_clause = if initial_dir.is_empty() {
        String::new()
    } else {
        format!("$dlg.SelectedPath = '{}'\n", initial_dir)
    };
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$dlg = New-Object System.Windows.Forms.FolderBrowserDialog
$dlg.Description = 'Select project directory'
$dlg.ShowNewFolderButton = $true
{initial_clause}if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
  [Console]::Out.Write($dlg.SelectedPath)
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
fn powershell_single_quote(text: &str) -> String {
    text.replace('\'', "''")
}

/// Returns the directory where region screenshots are saved (creates it if missing).
fn screenshot_output_dir() -> anyhow::Result<std::path::PathBuf> {
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("screenshots");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(windows)]
pub(crate) fn capture_selected_region_to_file(
    hide_window: bool,
) -> anyhow::Result<std::path::PathBuf> {
    use base64::Engine as _;

    let pid = std::process::id();
    let dir = screenshot_output_dir()?;
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

/// macOS region screenshot via `screencapture -i`.
///
/// `hide_window=true` hides our app first using `osascript` to ask System
/// Events to hide the frontmost process, runs the interactive picker, then
/// reactivates Unterm. ESC cancels the picker.
#[cfg(target_os = "macos")]
pub(crate) fn capture_selected_region_to_file(
    hide_window: bool,
) -> anyhow::Result<std::path::PathBuf> {
    let dir = screenshot_output_dir()?;
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

    if hide_window {
        let _ = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to set visible of process \"unterm\" to false",
            ])
            .status();
        // Brief delay so the window finishes hiding before the picker UI shows.
        std::thread::sleep(std::time::Duration::from_millis(250));
    }

    // -i = interactive selection, -t png = explicit format
    // We do NOT pass -x so the picker chrome and shutter sound stay (matches Win UX).
    let status = std::process::Command::new("/usr/sbin/screencapture")
        .args(["-i", "-t", "png"])
        .arg(&output_path)
        .status();

    if hide_window {
        // Always try to bring our window back, even on cancel/error.
        let _ = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"unterm\" to activate",
            ])
            .status();
    }

    let status = status?;
    if !status.success() {
        anyhow::bail!("screencapture exited with {status}");
    }

    if !output_path.exists() {
        anyhow::bail!(
            "Screenshot canceled or file not created: {}",
            output_path.display()
        );
    }

    // Copy to clipboard so the user can paste it elsewhere (parity with Win path).
    let _ = std::process::Command::new("osascript")
        .args([
            "-e",
            &format!(
                "set the clipboard to (read (POSIX file \"{}\") as «class PNGf»)",
                output_path.display()
            ),
        ])
        .status();

    Ok(output_path)
}

/// Linux region screenshot. Probes available tools in order and uses the first
/// one that exists.
///
/// `hide_window=true` is best-effort — most Linux screenshot tools take a
/// short delay flag, but minimizing the window cleanly across X11/Wayland
/// without window-server-specific code is fragile, so we currently skip it
/// and just rely on the tool's own region picker UI.
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn capture_selected_region_to_file(
    hide_window: bool,
) -> anyhow::Result<std::path::PathBuf> {
    let dir = screenshot_output_dir()?;
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

    // Try grim+slurp (Wayland), then gnome-screenshot, spectacle, scrot, maim.
    let path_str = output_path.display().to_string();
    let attempts: &[(&str, &[&str])] = &[
        ("grim", &[]), // grim handled specially below because slurp is piped
        ("gnome-screenshot", &["-a", "-f"]),
        ("spectacle", &["-bn", "-r", "-o"]),
        ("scrot", &["-s"]),
        ("maim", &["-s"]),
    ];

    let mut last_err: Option<String> = None;
    for (tool, args) in attempts {
        if !command_exists(tool) {
            continue;
        }

        let status = if *tool == "grim" {
            // grim -g "$(slurp)" <output>
            if !command_exists("slurp") {
                last_err = Some("grim found but slurp is required for region selection".into());
                continue;
            }
            // Run `slurp` to pick a region, capture stdout, pass to grim.
            let slurp = std::process::Command::new("slurp").output();
            let slurp = match slurp {
                Ok(o) if o.status.success() => o,
                Ok(o) => {
                    last_err = Some(format!(
                        "slurp exited with {} (selection cancelled?)",
                        o.status
                    ));
                    continue;
                }
                Err(e) => {
                    last_err = Some(format!("slurp failed: {e}"));
                    continue;
                }
            };
            let geom = String::from_utf8_lossy(&slurp.stdout).trim().to_string();
            std::process::Command::new("grim")
                .args(["-g", &geom])
                .arg(&output_path)
                .status()
        } else {
            let mut cmd = std::process::Command::new(tool);
            cmd.args(*args);
            cmd.arg(&path_str);
            cmd.status()
        };

        match status {
            Ok(s) if s.success() => {
                if output_path.exists() {
                    // Try to copy to clipboard via xclip / wl-copy — best effort.
                    let _ = copy_image_to_clipboard_unix(&output_path);
                    return Ok(output_path);
                } else {
                    last_err = Some(format!("{tool} reported success but no file was created"));
                }
            }
            Ok(s) => {
                last_err = Some(format!("{tool} exited with {s}"));
            }
            Err(e) => {
                last_err = Some(format!("failed to run {tool}: {e}"));
            }
        }
    }

    let _ = hide_window; // currently unused on Linux
    let msg = last_err.unwrap_or_else(|| {
        "No screenshot tool found. Install one of: grim+slurp, gnome-screenshot, spectacle, scrot, or maim".into()
    });
    anyhow::bail!("{}", msg)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn command_exists(name: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return true;
        }
    }
    false
}

#[cfg(all(unix, not(target_os = "macos")))]
fn copy_image_to_clipboard_unix(path: &std::path::Path) -> anyhow::Result<()> {
    use std::io::Write;
    if command_exists("wl-copy") {
        let mut child = std::process::Command::new("wl-copy")
            .args(["--type", "image/png"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        let bytes = std::fs::read(path)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&bytes)?;
        }
        child.wait()?;
        return Ok(());
    }
    if command_exists("xclip") {
        let mut child = std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-t", "image/png", "-i"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        let bytes = std::fs::read(path)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&bytes)?;
        }
        child.wait()?;
        return Ok(());
    }
    Ok(())
}

