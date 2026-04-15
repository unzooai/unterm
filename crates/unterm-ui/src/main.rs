//! unterm-ui: Unterm GUI 渲染进程

mod bridge;
mod client;
mod config;
mod input;
mod layout;
mod render;

use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowId, WindowAttributes};

use crate::bridge::CoreBridge;
use crate::config::AppConfig;
use crate::layout::{LayoutManager, SplitDirection};
use crate::render::{PaneContent, PaneRect, TabInfo};

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

    /// 处理快捷键，返回 true 表示该按键已被快捷键消费
    fn handle_shortcut(&mut self, key_code: KeyCode, event_loop: &ActiveEventLoop) -> bool {
        let ctrl = self.modifiers.control_key();
        let shift = self.modifiers.shift_key();
        let alt = self.modifiers.alt_key();

        let layout = match self.layout.as_mut() {
            Some(l) => l,
            None => return false,
        };

        // Ctrl+Shift 组合键
        if ctrl && shift && !alt {
            match key_code {
                // Ctrl+Shift+T — 新建 Tab
                KeyCode::KeyT => {
                    let tab_count = layout.tab_infos().len();
                    let (_tab_id, pane_id) =
                        layout.add_tab(format!("Tab {}", tab_count + 1));
                    info!("新建 Tab, Pane ID: {}", pane_id);
                    // 自动创建 session
                    if self.config.auto_create_session {
                        self.create_session_for_pane(pane_id);
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+W — 关闭当前 Tab
                KeyCode::KeyW => {
                    // 销毁 Tab 内所有 pane 的 session
                    let pane_layouts = layout.compute_pane_layouts();
                    if let Some(bridge) = &mut self.bridge {
                        for pl in &pane_layouts {
                            bridge.destroy_pane_session(pl.pane_id);
                        }
                    }
                    let tab_id = layout.active_tab().id;
                    layout.close_tab(tab_id);
                    if layout.tab_infos().is_empty() {
                        info!("所有 Tab 已关闭，退出应用");
                        event_loop.exit();
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+D — 垂直分屏
                KeyCode::KeyD => {
                    if let Some(new_pane_id) = layout.split_pane(SplitDirection::Vertical) {
                        info!("垂直分屏，新 Pane ID: {}", new_pane_id);
                        if self.config.auto_create_session {
                            self.create_session_for_pane(new_pane_id);
                        }
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+R — 水平分屏
                KeyCode::KeyR => {
                    if let Some(new_pane_id) = layout.split_pane(SplitDirection::Horizontal) {
                        info!("水平分屏，新 Pane ID: {}", new_pane_id);
                        if self.config.auto_create_session {
                            self.create_session_for_pane(new_pane_id);
                        }
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+X — 关闭当前 Pane
                KeyCode::KeyX => {
                    // 销毁当前 pane 的 session
                    let active_pane_id = layout.active_tab().active_pane;
                    if let Some(bridge) = &mut self.bridge {
                        bridge.destroy_pane_session(active_pane_id);
                    }
                    let tab_closed = layout.close_pane();
                    if tab_closed && layout.tab_infos().is_empty() {
                        info!("所有 Tab 已关闭，退出应用");
                        event_loop.exit();
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

        // Ctrl+Tab — 下一个 Tab
        if ctrl && !shift && !alt && key_code == KeyCode::Tab {
            layout.next_tab();
            self.request_redraw();
            return true;
        }

        // Alt+ArrowLeft — 上一个 Pane
        if alt && !ctrl && !shift && key_code == KeyCode::ArrowLeft {
            layout.focus_prev_pane();
            self.request_redraw();
            return true;
        }

        // Alt+ArrowRight — 下一个 Pane
        if alt && !ctrl && !shift && key_code == KeyCode::ArrowRight {
            layout.focus_next_pane();
            self.request_redraw();
            return true;
        }

        false
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
            .with_title("Unterm")
            .with_inner_size(LogicalSize::new(1024, 768))
            .with_min_inner_size(LogicalSize::new(min_w, min_h));

        match event_loop.create_window(attrs) {
            Ok(window) => {
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

                // 初始化布局管理器
                let layout = LayoutManager::new(size.width as f32, size.height as f32);

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
                if let Some(layout) = &mut self.layout {
                    layout.resize(size.width as f32, size.height as f32);

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
                            // 优先从 bridge 获取实时屏幕内容
                            let text = self
                                .bridge
                                .as_ref()
                                .and_then(|b| b.get_pane_content(pl.pane_id))
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    if connected {
                                        format!("Pane {} ({}x{})\n正在加载...", pl.pane_id, pl.cols, pl.rows)
                                    } else {
                                        format!("Pane {} ({}x{})\n等待连接 unterm-core...", pl.pane_id, pl.cols, pl.rows)
                                    }
                                });
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

                    Some((tab_infos, pane_contents, status_text, status_rect, tab_bar_rect))
                } else {
                    None
                };

                if let (Some(renderer), Some((tabs, panes, status, sr, tr))) =
                    (&mut self.renderer, render_data)
                {
                    if let Err(e) = renderer.draw_frame(&tabs, &panes, &status, sr, tr) {
                        tracing::error!("渲染错误: {}", e);
                    }
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
