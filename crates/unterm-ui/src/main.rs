//! unterm-ui: Unterm GUI 渲染进程

mod bridge;
mod client;
mod clipboard;
mod config;
mod input;
mod layout;
mod menu;
mod mouse;
mod render;

use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowId, WindowAttributes};

use std::collections::HashMap;
use crate::bridge::CoreBridge;
use crate::clipboard::{ClipboardContent, ClipboardManager};
use crate::config::AppConfig;
use crate::layout::{LayoutManager, SplitDirection};
use crate::mouse::MouseState;
use crate::menu::{MenuAction, MenuState};
use crate::mouse::{TabBarHit, TabBarLayout};
use crate::render::{MenuRenderData, MenuRenderItem, PaneContent, PaneRect, RenderCell, RenderCursor, StructuredContent, TabInfo};

/// 字体度量常量（逻辑像素，与 layout/mod.rs 和 render/mod.rs 保持一致）
const FONT_SIZE: f32 = 16.0;
const LINE_HEIGHT: f32 = 20.0;
const FONT_WIDTH: f32 = FONT_SIZE * 0.6;

rust_i18n::i18n!("locales", fallback = "en");

/// 应用状态
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<render::Renderer>,
    input_handler: input::InputHandler,
    layout: Option<LayoutManager>,

    /// 配置
    config: AppConfig,

    /// Core 通信桥
    bridge: Option<CoreBridge>,

    /// 当前修饰键状态
    modifiers: ModifiersState,

    /// 上次重绘时间（用于控制刷新频率）
    last_redraw: Instant,

    /// 鼠标状态
    mouse: MouseState,

    /// 剪贴板管理器
    clipboard: ClipboardManager,

    /// 菜单状态
    menu: MenuState,

    /// Tab 栏各区域（每帧更新，用于鼠标 hit-test）
    tab_bar_layout: TabBarLayout,

    /// 每个 pane 的滚动偏移（0 = 底部实时，正数 = 向上滚了多少行）
    viewport_offsets: HashMap<u64, usize>,
}

impl App {
    fn new(config: AppConfig) -> Self {
        Self {
            window: None,
            renderer: None,
            input_handler: input::InputHandler::new(),
            layout: None,
            config,
            bridge: None,
            modifiers: ModifiersState::empty(),
            last_redraw: Instant::now(),
            mouse: MouseState::new(),
            clipboard: ClipboardManager::new(),
            menu: MenuState::new(),
            tab_bar_layout: TabBarLayout::default(),
            viewport_offsets: HashMap::new(),
        }
    }

    /// 为新 pane 创建 session
    fn create_session_for_pane(&self, pane_id: u64) {
        if let Some(bridge) = &self.bridge {
            bridge.create_session_for_pane(
                pane_id,
                Some(self.config.default_shell.clone()),
                Some(self.config.effective_cwd()),
            );
        }
    }

    /// 粘贴剪贴板内容到当前激活 pane
    fn paste_to_active_pane(&mut self) {
        let content = self.clipboard.read();
        let text = match content {
            ClipboardContent::Text(t) => t,
            ClipboardContent::ImagePath(path) => path,
            ClipboardContent::Empty => return,
        };
        if let Some(layout) = &self.layout {
            let active_pane_id = layout.active_tab().active_pane;
            if let Some(bridge) = &self.bridge {
                bridge.send_input_to_pane(active_pane_id, text.into_bytes());
            }
        }
    }

    /// 复制选区文本到剪贴板
    ///
    /// 目前选区文本提取是简化实现：从 pane 的屏幕内容中按行列范围截取。
    fn copy_selection_to_clipboard(&mut self) {
        if !self.mouse.has_selection() {
            return;
        }
        let (start_col, start_row, pane_id) = match self.mouse.selection_start {
            Some(s) => s,
            None => return,
        };
        let (end_col, end_row) = match self.mouse.selection_end {
            Some(e) => e,
            None => return,
        };

        // 从 bridge 获取 pane 内容
        let content = self
            .bridge
            .as_ref()
            .and_then(|b| b.get_pane_content(pane_id))
            .unwrap_or("");

        // 简化选区提取：按行分割，截取选区范围内的文本
        let lines: Vec<&str> = content.lines().collect();
        let (r0, c0, r1, c1) = if (start_row, start_col) <= (end_row, end_col) {
            (start_row, start_col, end_row, end_col)
        } else {
            (end_row, end_col, start_row, start_col)
        };

        let mut selected = String::new();
        for row in r0..=r1 {
            if let Some(line) = lines.get(row as usize) {
                let chars: Vec<char> = line.chars().collect();
                let from = if row == r0 { c0 as usize } else { 0 };
                let to = if row == r1 {
                    (c1 as usize + 1).min(chars.len())
                } else {
                    chars.len()
                };
                if from < chars.len() {
                    selected.extend(&chars[from..to.min(chars.len())]);
                }
                if row < r1 {
                    selected.push('\n');
                }
            }
        }

        if !selected.is_empty() {
            self.clipboard.write_text(&selected);
        }
        self.mouse.clear_selection();
    }

    /// 处理快捷键，返回 true 表示该按键已被快捷键消费
    fn handle_shortcut(&mut self, key_code: KeyCode, event_loop: &ActiveEventLoop) -> bool {
        let ctrl = self.modifiers.control_key();
        let shift = self.modifiers.shift_key();
        let alt = self.modifiers.alt_key();

        // Ctrl+V — 粘贴（在 layout 借用之前处理，避免 borrow 冲突）
        if ctrl && !shift && !alt && key_code == KeyCode::KeyV {
            self.paste_to_active_pane();
            return true;
        }

        // Ctrl+C — 有选区时复制，否则不消费（交给终端发 \x03）
        if ctrl && !shift && !alt && key_code == KeyCode::KeyC {
            if self.mouse.has_selection() {
                self.copy_selection_to_clipboard();
                self.request_redraw();
                return true;
            }
            // 无选区，不消费，让后续逻辑发送 \x03
        }

        let layout = match self.layout.as_mut() {
            Some(l) => l,
            None => return false,
        };

        // ── 快捷键（对齐 Windows Terminal 风格）──

        // Ctrl+Shift 组合键
        if ctrl && shift && !alt {
            match key_code {
                // Ctrl+Shift+T — 新建 Tab
                KeyCode::KeyT => {
                    let tab_count = layout.tab_infos().len();
                    let (_tab_id, pane_id) =
                        layout.add_tab(format!("Tab {}", tab_count + 1));
                    info!("新建 Tab, Pane ID: {}", pane_id);
                    if self.config.auto_create_session {
                        self.create_session_for_pane(pane_id);
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+W — 关闭当前 Pane（最后一个 pane 则关闭 Tab）
                KeyCode::KeyW => {
                    let active_pane_id = layout.active_tab().active_pane;
                    if let Some(bridge) = &mut self.bridge {
                        bridge.destroy_pane_session(active_pane_id);
                    }
                    let tab_closed = layout.close_pane();
                    if tab_closed && layout.tab_infos().is_empty() {
                        event_loop.exit();
                    }
                    // 恢复窗口焦点
                    if let Some(w) = &self.window {
                        w.focus_window();
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+Tab — 上一个 Tab
                KeyCode::Tab => {
                    layout.prev_tab();
                    self.request_redraw();
                    return true;
                }
                _ => {}
            }
        }

        // Alt+Shift 组合键
        if alt && shift && !ctrl {
            match key_code {
                // Alt+Shift+D — 垂直分屏（左右）
                KeyCode::KeyD => {
                    if let Some(new_pane_id) = layout.split_pane(SplitDirection::Horizontal) {
                        info!("水平分屏（左右），新 Pane ID: {}", new_pane_id);
                        if self.config.auto_create_session {
                            self.create_session_for_pane(new_pane_id);
                        }
                    }
                    self.request_redraw();
                    return true;
                }
                // Alt+Shift+Minus — 水平分屏（上下）
                KeyCode::Minus => {
                    if let Some(new_pane_id) = layout.split_pane(SplitDirection::Vertical) {
                        info!("垂直分屏（上下），新 Pane ID: {}", new_pane_id);
                        if self.config.auto_create_session {
                            self.create_session_for_pane(new_pane_id);
                        }
                    }
                    self.request_redraw();
                    return true;
                }
                _ => {}
            }
        }

        // Ctrl+Tab — 下一个 Tab
        if ctrl && !shift && !alt && key_code == KeyCode::Tab {
            layout.next_tab();
            self.request_redraw();
            return true;
        }

        // Alt+Arrow — 切换 Pane 焦点
        if alt && !ctrl && !shift {
            match key_code {
                KeyCode::ArrowLeft | KeyCode::ArrowUp => {
                    layout.focus_prev_pane();
                    self.request_redraw();
                    return true;
                }
                KeyCode::ArrowRight | KeyCode::ArrowDown => {
                    layout.focus_next_pane();
                    self.request_redraw();
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    /// 执行菜单操作
    fn execute_menu_action(&mut self, action: MenuAction, event_loop: &ActiveEventLoop) {
        match action {
            MenuAction::NewTab => {
                if let Some(layout) = &mut self.layout {
                    let tab_count = layout.tab_infos().len();
                    let (_tab_id, pane_id) = layout.add_tab(format!("PowerShell {}", tab_count + 1));
                    if self.config.auto_create_session {
                        self.create_session_for_pane(pane_id);
                    }
                }
            }
            MenuAction::CloseTab => {
                if let Some(layout) = &mut self.layout {
                    let tab_id = layout.active_tab().id;
                    let pane_ids = layout.tab_pane_ids(tab_id);
                    if let Some(bridge) = &mut self.bridge {
                        for pid in &pane_ids {
                            bridge.destroy_pane_session(*pid);
                        }
                    }
                    layout.close_tab(tab_id);
                    if layout.tab_infos().is_empty() {
                        event_loop.exit();
                    }
                }
            }
            MenuAction::HorizontalSplit => {
                if let Some(layout) = &mut self.layout {
                    if let Some(new_pane_id) = layout.split_pane(SplitDirection::Horizontal) {
                        if self.config.auto_create_session {
                            self.create_session_for_pane(new_pane_id);
                        }
                    }
                }
            }
            MenuAction::VerticalSplit => {
                if let Some(layout) = &mut self.layout {
                    if let Some(new_pane_id) = layout.split_pane(SplitDirection::Vertical) {
                        if self.config.auto_create_session {
                            self.create_session_for_pane(new_pane_id);
                        }
                    }
                }
            }
            MenuAction::ClosePane => {
                if let Some(layout) = &mut self.layout {
                    let active_pane_id = layout.active_tab().active_pane;
                    if let Some(bridge) = &mut self.bridge {
                        bridge.destroy_pane_session(active_pane_id);
                    }
                    let tab_closed = layout.close_pane();
                    if tab_closed && layout.tab_infos().is_empty() {
                        event_loop.exit();
                    }
                }
                // 恢复窗口焦点
                if let Some(w) = &self.window {
                    w.focus_window();
                }
            }
            MenuAction::Copy => {
                self.copy_selection_to_clipboard();
            }
            MenuAction::Paste => {
                self.paste_to_active_pane();
            }
            MenuAction::Settings | MenuAction::About | MenuAction::None => {
                // TODO
            }
        }
        self.request_redraw();
    }

    /// 请求窗口重绘
    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            info!("unterm-ui 正在初始化...");
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let (min_w, min_h) = LayoutManager::min_window_size();
        let attrs = WindowAttributes::default()
            .with_title("Unterm - Windows PowerShell")
            .with_inner_size(LogicalSize::new(1024, 768))
            .with_min_inner_size(LogicalSize::new(min_w, min_h));

        match event_loop.create_window(attrs) {
            Ok(window) => {
                // 启用 IME 输入法支持
                window.set_ime_allowed(true);

                let window = Arc::new(window);
                let size = window.inner_size();

                // 初始化渲染器
                match render::Renderer::new(window.clone()) {
                    Ok(renderer) => {
                        self.renderer = Some(renderer);
                    }
                    Err(e) => {
                        tracing::error!("渲染器初始化失败: {}", e);
                        event_loop.exit();
                        return;
                    }
                }

                // 初始化布局管理器（使用逻辑像素，与 winit 鼠标坐标一致）
                let scale = window.scale_factor() as f32;
                let layout = LayoutManager::new(
                    size.width as f32 / scale,
                    size.height as f32 / scale,
                );

                // 获取初始 pane 的 ID
                let initial_pane_id = layout.active_tab().active_pane;

                self.layout = Some(layout);
                self.window = Some(window);

                // 启动 Core 通信桥
                let bridge = CoreBridge::start(
                    self.config.core_address.clone(),
                    self.config.screen_poll_interval_ms,
                );

                // 为初始 pane 创建 session
                if self.config.auto_create_session {
                    bridge.create_session_for_pane(
                        initial_pane_id,
                        Some(self.config.default_shell.clone()),
                        Some(self.config.effective_cwd()),
                    );
                }

                self.bridge = Some(bridge);

                info!("unterm-ui 窗口已创建，正在连接 core...");
            }
            Err(e) => {
                tracing::error!("窗口创建失败: {}", e);
                event_loop.exit();
            }
        }
    }

    /// 事件循环空闲时调用 — 用于轮询 bridge 事件并触发重绘
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // 轮询 bridge 事件
        let mut needs_redraw = false;
        if let Some(bridge) = &mut self.bridge {
            let events = bridge.poll_events();
            if !events.is_empty() {
                needs_redraw = true;
                for event in &events {
                    match event {
                        bridge::CoreEvent::Connected => {
                            info!("已连接到 unterm-core");
                        }
                        bridge::CoreEvent::Disconnected => {
                            tracing::warn!("与 unterm-core 断开连接");
                        }
                        bridge::CoreEvent::SessionCreated { pane_id, session_id } => {
                            info!("Session 已创建: pane={} session={}", pane_id, session_id);
                        }
                        bridge::CoreEvent::Error(msg) => {
                            tracing::error!("Core 错误: {}", msg);
                        }
                        bridge::CoreEvent::ScreenUpdate { .. } => {
                            // 屏幕内容更新，需要重绘
                        }
                    }
                }
            }
        }

        // 限制最大刷新频率为 60fps（~16ms）
        let now = Instant::now();
        if needs_redraw && now.duration_since(self.last_redraw) > Duration::from_millis(16) {
            self.last_redraw = now;
            self.request_redraw();
        }

        // 设置下次唤醒时间（确保持续轮询）
        if let Some(bridge) = &self.bridge {
            if bridge.connected {
                _event_loop.set_control_flow(ControlFlow::wait_duration(Duration::from_millis(16)));
            } else {
                // 未连接时降低频率
                _event_loop.set_control_flow(ControlFlow::wait_duration(Duration::from_millis(100)));
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("unterm-ui 正在关闭...");
                event_loop.exit();
            }

            // 跟踪修饰键状态
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                // 布局使用逻辑像素
                let scale = self.renderer.as_ref().map(|r| r.scale_factor).unwrap_or(1.0);
                if let Some(layout) = &mut self.layout {
                    layout.resize(size.width as f32 / scale, size.height as f32 / scale);

                    // 通知 core 调整所有可见 pane 的 session 尺寸
                    if let Some(bridge) = &self.bridge {
                        let pane_layouts = layout.compute_pane_layouts();
                        for pl in &pane_layouts {
                            if let Some(session_id) = bridge.get_session_id(pl.pane_id) {
                                bridge.send_command(bridge::UiCommand::ResizeSession {
                                    session_id: session_id.to_string(),
                                    cols: pl.cols,
                                    rows: pl.rows,
                                });
                            }
                        }
                    }
                }
                self.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                // 从 bridge 获取连接状态
                let connected = self.bridge.as_ref().map_or(false, |b| b.connected);

                // 收集渲染数据
                let render_data = if let Some(layout) = &self.layout {
                    let tab_infos: Vec<TabInfo> = layout
                        .tab_infos()
                        .iter()
                        .map(|(id, title, active)| TabInfo {
                            id: *id,
                            title: title.to_string(),
                            is_active: *active,
                        })
                        .collect();

                    let pane_layouts = layout.compute_pane_layouts();
                    let pane_contents: Vec<PaneContent> = pane_layouts
                        .iter()
                        .map(|pl| {
                            // 优先从 bridge 获取实时屏幕内容（JSON 结构化数据）
                            let (text, structured) = self
                                .bridge
                                .as_ref()
                                .and_then(|b| b.get_pane_content(pl.pane_id))
                                .map(|s| parse_screen_content(s))
                                .unwrap_or_else(|| {
                                    let fallback = if connected {
                                        format!("Pane {} ({}x{})\n正在加载...", pl.pane_id, pl.cols, pl.rows)
                                    } else {
                                        format!("Pane {} ({}x{})\n等待连接 unterm-core...", pl.pane_id, pl.cols, pl.rows)
                                    };
                                    (fallback, None)
                                });
                            let vp_offset = self.viewport_offsets.get(&pl.pane_id).copied().unwrap_or(0);
                            PaneContent {
                                pane_id: pl.pane_id,
                                rect: PaneRect {
                                    x: pl.rect.x,
                                    y: pl.rect.y,
                                    width: pl.rect.width,
                                    height: pl.rect.height,
                                },
                                text,
                                is_active: pl.is_active,
                                title: None,
                                structured,
                                viewport_offset: vp_offset,
                            }
                        })
                        .collect();

                    let status_text = if connected {
                        rust_i18n::t!("ui.status_connected").to_string()
                    } else {
                        rust_i18n::t!("ui.status_disconnected").to_string()
                    };

                    let sr = layout.status_bar_rect();
                    let status_rect = PaneRect {
                        x: sr.x,
                        y: sr.y,
                        width: sr.width,
                        height: sr.height,
                    };

                    let tr = layout.tab_bar_rect();
                    let tab_bar_rect = PaneRect {
                        x: tr.x,
                        y: tr.y,
                        width: tr.width,
                        height: tr.height,
                    };

                    // 计算 TabBarLayout（逻辑像素，用于鼠标 hit-test）
                    // layout 已使用逻辑像素，winit 鼠标坐标也是逻辑像素
                    let logical_cw = FONT_WIDTH; // 逻辑字符宽度
                    let mut tab_bar_layout = TabBarLayout::default();
                    {
                        let mut tx = tr.x + 8.0;
                        for ti in &tab_infos {
                            let label = format!(" {}  \u{00D7} ", ti.title);
                            let tw = render::display_width(&label) as f32 * logical_cw + 8.0;
                            let close_w = 3.0 * logical_cw; // × 按钮区域
                            tab_bar_layout.tabs.push(mouse::TabBarRegion {
                                tab_id: ti.id,
                                tab_rect: layout::Rect { x: tx, y: tr.y, width: tw, height: tr.height },
                                close_rect: layout::Rect { x: tx + tw - close_w, y: tr.y, width: close_w, height: tr.height },
                            });
                            tx += tw + 2.0;
                        }
                        let plus_w = 4.0 * logical_cw;
                        tab_bar_layout.new_btn_rect = layout::Rect { x: tx, y: tr.y, width: plus_w, height: tr.height };
                        let dropdown_w = 36.0;
                        tab_bar_layout.dropdown_rect = layout::Rect {
                            x: tr.x + tr.width - dropdown_w,
                            y: tr.y,
                            width: dropdown_w,
                            height: tr.height,
                        };
                    }

                    Some((tab_infos, pane_contents, status_text, status_rect, tab_bar_rect, tab_bar_layout))
                } else {
                    None
                };

                if let (Some(renderer), Some((tabs, panes, status, sr, tr, tbl))) =
                    (&mut self.renderer, render_data)
                {
                    // 构建菜单渲染数据
                    let menu_data = if self.menu.is_open() {
                        Some(MenuRenderData {
                            x: self.menu.x,
                            y: self.menu.y,
                            width: menu::MENU_WIDTH,
                            items: self.menu.items.iter().map(|item| MenuRenderItem {
                                label: item.label.clone(),
                                shortcut: item.shortcut.clone(),
                                is_separator: item.is_separator,
                            }).collect(),
                            hovered: self.menu.hovered,
                        })
                    } else {
                        None
                    };

                    if let Err(e) = renderer.draw_frame(&tabs, &panes, &status, sr, tr, menu_data.as_ref()) {
                        tracing::error!("渲染错误: {}", e);
                    }

                    // 更新 tab_bar_layout 用于后续鼠标交互
                    self.tab_bar_layout = tbl;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                // winit 0.30 CursorMoved 给出物理像素坐标，需转为逻辑坐标
                let scale = self.window.as_ref()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.0);
                self.mouse.x = (position.x / scale) as f32;
                self.mouse.y = (position.y / scale) as f32;

                // 菜单 hover 更新
                if self.menu.is_open() {
                    self.menu.hit_test(self.mouse.x, self.mouse.y);
                    self.request_redraw();
                }

                // 如果左键按下，更新选区终点并重绘
                if self.mouse.left_pressed {
                    if let Some(layout) = &self.layout {
                        let pane_layouts = layout.compute_pane_layouts();
                        if let Some(pane) = self.mouse.hit_test_pane(&pane_layouts) {
                            let (col, row) = self.mouse.pixel_to_cell(pane, FONT_WIDTH, LINE_HEIGHT);
                            self.mouse.selection_end = Some((col, row));
                        }
                    }
                    self.request_redraw();
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                match (button, state) {
                    // 左键按下
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.mouse.left_pressed = true;

                        // 如果菜单打开，检测是否点击菜单项
                        if self.menu.is_open() {
                            if let Some(_idx) = self.menu.hit_test(self.mouse.x, self.mouse.y) {
                                if let Some(action) = self.menu.hovered_action() {
                                    self.menu.close();
                                    self.execute_menu_action(action, event_loop);
                                    return;
                                }
                            }
                            // 点在菜单外面 → 关闭菜单
                            self.menu.close();
                            self.request_redraw();
                            return;
                        }

                        // Tab 栏 hit-test
                        let tab_hit = self.mouse.hit_test_tab_bar(&self.tab_bar_layout);
                        match tab_hit {
                            TabBarHit::Tab(tab_id) => {
                                if let Some(layout) = &mut self.layout {
                                    layout.switch_tab(tab_id);
                                }
                                self.request_redraw();
                                return;
                            }
                            TabBarHit::CloseTab(tab_id) => {
                                if let Some(layout) = &mut self.layout {
                                    // 销毁该 Tab 的所有 pane session
                                    let pane_ids = layout.tab_pane_ids(tab_id);
                                    if let Some(bridge) = &mut self.bridge {
                                        for pid in &pane_ids {
                                            bridge.destroy_pane_session(*pid);
                                        }
                                    }
                                    layout.close_tab(tab_id);
                                    if layout.tab_infos().is_empty() {
                                        event_loop.exit();
                                    }
                                }
                                // 恢复窗口焦点
                                if let Some(w) = &self.window {
                                    w.focus_window();
                                }
                                self.request_redraw();
                                return;
                            }
                            TabBarHit::NewTab => {
                                self.execute_menu_action(MenuAction::NewTab, event_loop);
                                return;
                            }
                            TabBarHit::Dropdown => {
                                let dr = &self.tab_bar_layout.dropdown_rect;
                                // 防止菜单超出窗口右边界
                                let win_w = self.layout.as_ref()
                                    .map(|l| l.tab_bar_rect().width)
                                    .unwrap_or(1024.0);
                                let menu_x = (dr.x + dr.width).min(win_w) - menu::MENU_WIDTH;
                                let menu_x = menu_x.max(0.0);
                                self.menu.open_dropdown(menu_x, dr.y + dr.height);
                                self.request_redraw();
                                return;
                            }
                            TabBarHit::None => {}
                        }

                        // Pane 区域 — 检查 × 按钮 / 切换焦点 + 开始选区
                        if let Some(layout) = &mut self.layout {
                            let pane_layouts = layout.compute_pane_layouts();
                            let multi_pane = pane_layouts.len() > 1;
                            if let Some(pane) = self.mouse.hit_test_pane(&pane_layouts) {
                                let pane_id = pane.pane_id;

                                // 多窗格时检查 × 关闭按钮（右上角 20×20 逻辑像素）
                                if multi_pane {
                                    let btn_size = 20.0_f32;
                                    let btn_margin = 4.0_f32;
                                    let btn_x = pane.rect.x + pane.rect.width - btn_size - btn_margin;
                                    let btn_y = pane.rect.y + btn_margin;
                                    if self.mouse.x >= btn_x && self.mouse.x < btn_x + btn_size
                                        && self.mouse.y >= btn_y && self.mouse.y < btn_y + btn_size
                                    {
                                        // 关闭该 pane
                                        if let Some(bridge) = &mut self.bridge {
                                            bridge.destroy_pane_session(pane_id);
                                        }
                                        layout.active_tab_mut().active_pane = pane_id;
                                        let tab_closed = layout.close_pane();
                                        if tab_closed && layout.tab_infos().is_empty() {
                                            event_loop.exit();
                                        }
                                        if let Some(w) = &self.window {
                                            w.focus_window();
                                        }
                                        self.request_redraw();
                                        return;
                                    }
                                }

                                let (col, row) = self.mouse.pixel_to_cell(pane, FONT_WIDTH, LINE_HEIGHT);
                                layout.active_tab_mut().active_pane = pane_id;
                                self.mouse.selection_start = Some((col, row, pane_id));
                                self.mouse.selection_end = Some((col, row));
                            }
                        }
                        self.request_redraw();
                    }
                    // 左键松开
                    (MouseButton::Left, ElementState::Released) => {
                        self.mouse.left_pressed = false;
                        if self.mouse.has_selection() {
                            let has_range = match (self.mouse.selection_start, self.mouse.selection_end) {
                                (Some((c0, r0, _)), Some((c1, r1))) => c0 != c1 || r0 != r1,
                                _ => false,
                            };
                            if has_range {
                                self.copy_selection_to_clipboard();
                            } else {
                                self.mouse.clear_selection();
                            }
                        }
                    }
                    // 右键按下 — 打开右键菜单
                    (MouseButton::Right, ElementState::Pressed) => {
                        if self.menu.is_open() {
                            self.menu.close();
                        } else {
                            // 防止菜单超出窗口边界
                            let (win_w, win_h) = self.layout.as_ref()
                                .map(|l| {
                                    let cr = l.content_rect();
                                    (cr.x + cr.width, cr.y + cr.height)
                                })
                                .unwrap_or((1024.0, 768.0));
                            let mx = self.mouse.x.min(win_w - menu::MENU_WIDTH).max(0.0);
                            let menu_h = self.menu.estimate_context_height();
                            let my = self.mouse.y.min(win_h - menu_h).max(0.0);
                            self.menu.open_context(mx, my);
                        }
                        self.request_redraw();
                    }
                    _ => {}
                }
            }

            // 鼠标滚轮 — 滚动回看
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        y as i32
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.y / 40.0) as i32
                    }
                };

                if lines != 0 {
                    if let Some(layout) = &self.layout {
                        let active_pane_id = layout.active_tab().active_pane;
                        // 获取当前 pane 的 scrollback 行数来限制 offset
                        let max_offset = self.bridge.as_ref()
                            .and_then(|b| b.get_pane_content(active_pane_id))
                            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                            .and_then(|v| v.get("scrollback_len").and_then(|n| n.as_u64()))
                            .unwrap_or(0) as usize;

                        let offset = self.viewport_offsets.entry(active_pane_id).or_insert(0);
                        if lines > 0 {
                            *offset = offset.saturating_add(lines as usize).min(max_offset);
                        } else {
                            *offset = offset.saturating_sub(lines.unsigned_abs() as usize);
                        }
                        self.request_redraw();
                    }
                }
            }

            // IME 输入法事件（中文/日文/韩文等）
            WindowEvent::Ime(ime_event) => {
                match ime_event {
                    winit::event::Ime::Commit(text) => {
                        // 输入法确认文本，发送到终端
                        if let Some(layout) = &self.layout {
                            let active_pane_id = layout.active_tab().active_pane;
                            self.viewport_offsets.insert(active_pane_id, 0);
                            if let Some(bridge) = &self.bridge {
                                bridge.send_input_to_pane(active_pane_id, text.into_bytes());
                            }
                        }
                    }
                    winit::event::Ime::Preedit(_, _) => {
                        // TODO: 显示输入法预编辑文本
                    }
                    winit::event::Ime::Enabled | winit::event::Ime::Disabled => {}
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                // 先尝试快捷键
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    if self.handle_shortcut(key_code, event_loop) {
                        return;
                    }
                }

                // 非快捷键：转换为终端输入序列，发送到当前激活 pane
                let text = event.text.as_ref().map(|s| s.as_str());
                if let Some(bytes) = self.input_handler.key_to_sequence(
                    &event.physical_key,
                    event.state,
                    text,
                ) {
                    if let Some(layout) = &self.layout {
                        let active_pane_id = layout.active_tab().active_pane;
                        // 任何键盘输入自动回到底部
                        self.viewport_offsets.insert(active_pane_id, 0);
                        if let Some(bridge) = &self.bridge {
                            bridge.send_input_to_pane(active_pane_id, bytes);
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

/// 解析 core 返回的屏幕内容
///
/// 尝试将 JSON 结构化数据（cells 数组 + cursor + cols + rows）转换为纯文本和结构化数据。
/// 如果 JSON 解析失败，回退到原始字符串（兼容渐进式开发）。
///
/// 返回 (text, Option<StructuredContent>)
/// 解析 core 返回的结构化屏幕数据
///
/// JSON 格式:
/// ```json
/// {
///   "cells": [[{"ch":"H","attrs":{"fg":"Default","bg":"Default","bold":false,...},"is_wide_continuation":false}, ...], ...],
///   "cursor": {"row":0,"col":5,"visible":true},
///   "cols": 80, "rows": 24
/// }
/// ```
/// TermColor 序列化为: "Default" | {"Indexed":5} | {"Rgb":[255,0,0]}
fn parse_screen_content(input: &str) -> (String, Option<StructuredContent>) {
    use render::indexed_color_to_rgb;

    let parsed: serde_json::Value = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(_) => {
            let text = input
                .chars()
                .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
                .collect();
            return (text, None);
        }
    };

    let cols = parsed.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as usize;
    let rows = parsed.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as usize;
    let default_fg: (u8, u8, u8) = (204, 204, 204); // Campbell white

    /// 解析 TermColor JSON 为 RGB
    fn parse_term_color(v: &serde_json::Value, default: (u8, u8, u8)) -> (u8, u8, u8) {
        match v {
            serde_json::Value::String(s) if s == "Default" => default,
            serde_json::Value::Object(m) => {
                if let Some(idx) = m.get("Indexed").and_then(|v| v.as_u64()) {
                    indexed_color_to_rgb(idx as u8)
                } else if let Some(arr) = m.get("Rgb").and_then(|v| v.as_array()) {
                    if arr.len() == 3 {
                        (
                            arr[0].as_u64().unwrap_or(0) as u8,
                            arr[1].as_u64().unwrap_or(0) as u8,
                            arr[2].as_u64().unwrap_or(0) as u8,
                        )
                    } else {
                        default
                    }
                } else {
                    default
                }
            }
            _ => default,
        }
    }

    let default_bg: (u8, u8, u8) = (12, 12, 12); // Campbell black

    let mut cell_grid: Vec<Vec<RenderCell>> = Vec::with_capacity(rows);
    let mut text_lines: Vec<String> = Vec::with_capacity(rows);

    if let Some(row_array) = parsed.get("cells").and_then(|v| v.as_array()) {
        for (r, row_val) in row_array.iter().enumerate() {
            if r >= rows { break; }
            let mut row_cells = Vec::with_capacity(cols);
            let mut row_text = String::with_capacity(cols);

            if let Some(col_array) = row_val.as_array() {
                for (c, cell_val) in col_array.iter().enumerate() {
                    if c >= cols { break; }
                    let ch = cell_val.get("ch")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.chars().next())
                        .unwrap_or(' ');

                    let attrs = cell_val.get("attrs");
                    let fg = attrs
                        .and_then(|a| a.get("fg"))
                        .map(|v| parse_term_color(v, default_fg))
                        .unwrap_or(default_fg);
                    let bg_rgb = attrs
                        .and_then(|a| a.get("bg"))
                        .map(|v| parse_term_color(v, default_bg))
                        .unwrap_or(default_bg);
                    // 只有非默认背景才设置 bg
                    let bg = if bg_rgb == default_bg { None } else { Some(bg_rgb) };
                    let bold = attrs.and_then(|a| a.get("bold")).and_then(|v| v.as_bool()).unwrap_or(false);
                    let dim = attrs.and_then(|a| a.get("dim")).and_then(|v| v.as_bool()).unwrap_or(false);
                    let is_wide = cell_val.get("is_wide_continuation").and_then(|v| v.as_bool()).unwrap_or(false);
                    let inverse = attrs.and_then(|a| a.get("inverse")).and_then(|v| v.as_bool()).unwrap_or(false);

                    // 反色处理
                    let (fg, bg) = if inverse {
                        let actual_bg = bg.unwrap_or(default_bg);
                        (actual_bg, Some(fg))
                    } else {
                        (fg, bg)
                    };

                    row_text.push(ch);
                    row_cells.push(RenderCell { ch, fg, bg, bold, dim, is_wide_continuation: is_wide });
                }
            }
            // 补齐列数
            while row_cells.len() < cols {
                row_cells.push(RenderCell { ch: ' ', fg: default_fg, bg: None, bold: false, dim: false, is_wide_continuation: false });
                row_text.push(' ');
            }
            cell_grid.push(row_cells);
            text_lines.push(row_text.trim_end().to_string());
        }
    }
    // 补齐行数
    while cell_grid.len() < rows {
        cell_grid.push(vec![RenderCell { ch: ' ', fg: default_fg, bg: None, bold: false, dim: false, is_wide_continuation: false }; cols]);
        text_lines.push(String::new());
    }

    let cursor = parsed
        .get("cursor")
        .map(|c| RenderCursor {
            row: c.get("row").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
            col: c.get("col").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
            visible: c.get("visible").and_then(|v| v.as_bool()).unwrap_or(true),
        })
        .unwrap_or_default();

    // 去掉尾部空行
    while text_lines.last().is_some_and(|l| l.is_empty()) {
        text_lines.pop();
    }
    let text = text_lines.join("\n");

    // 解析 scrollback
    let scrollback_total = parsed.get("scrollback_len").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let mut scrollback: Vec<Vec<RenderCell>> = Vec::new();
    if let Some(sb_array) = parsed.get("scrollback").and_then(|v| v.as_array()) {
        for row_val in sb_array {
            let mut row_cells = Vec::new();
            if let Some(col_array) = row_val.as_array() {
                for cell_val in col_array {
                    let ch = cell_val.get("ch")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.chars().next())
                        .unwrap_or(' ');
                    let attrs = cell_val.get("attrs");
                    let fg = attrs
                        .and_then(|a| a.get("fg"))
                        .map(|v| parse_term_color(v, default_fg))
                        .unwrap_or(default_fg);
                    let bg_rgb = attrs
                        .and_then(|a| a.get("bg"))
                        .map(|v| parse_term_color(v, default_bg))
                        .unwrap_or(default_bg);
                    let bg = if bg_rgb == default_bg { None } else { Some(bg_rgb) };
                    let is_wide = cell_val.get("is_wide_continuation").and_then(|v| v.as_bool()).unwrap_or(false);
                    let inverse = attrs.and_then(|a| a.get("inverse")).and_then(|v| v.as_bool()).unwrap_or(false);
                    let (fg, bg) = if inverse {
                        let actual_bg = bg.unwrap_or(default_bg);
                        (actual_bg, Some(fg))
                    } else {
                        (fg, bg)
                    };
                    row_cells.push(RenderCell { ch, fg, bg, bold: false, dim: false, is_wide_continuation: is_wide });
                }
            }
            scrollback.push(row_cells);
        }
    }

    (text, Some(StructuredContent { cells: cell_grid, cursor, scrollback, scrollback_total }))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("unterm_ui=debug")
        .init();

    // 加载配置
    let config = AppConfig::load();
    info!("配置已加载: shell={}, cwd={:?}, core={}",
        config.default_shell, config.default_cwd, config.core_address);

    // 语言设置：配置文件 > 环境变量 > 默认英文
    let locale = config.locale.clone().unwrap_or_else(|| {
        std::env::var("UNTERM_LOCALE")
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default()
    });
    if locale.starts_with("zh") {
        rust_i18n::set_locale("zh-CN");
    }

    info!("unterm-ui 启动中...");

    let event_loop = EventLoop::new()?;
    let mut app = App::new(config);
    event_loop.run_app(&mut app)?;

    Ok(())
}
