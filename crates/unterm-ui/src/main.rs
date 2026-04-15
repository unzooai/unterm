//! unterm-ui: Unterm GUI 渲染进程

mod render;
mod input;
mod layout;
mod client;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowId, WindowAttributes};

use crate::layout::{LayoutManager, SplitDirection};
use crate::render::{PaneContent, PaneRect, TabInfo};

rust_i18n::i18n!("locales", fallback = "en");

/// 应用状态
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<render::Renderer>,
    input_handler: input::InputHandler,
    layout: Option<LayoutManager>,

    /// 每个 pane 的终端内容，key 是 pane_id
    pane_contents: HashMap<u64, String>,

    /// 当前修饰键状态
    modifiers: ModifiersState,

    /// 是否已连接
    connected: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            input_handler: input::InputHandler::new(),
            layout: None,
            pane_contents: HashMap::new(),
            modifiers: ModifiersState::empty(),
            connected: false,
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
                    let (_tab_id, _pane_id) =
                        layout.add_tab(format!("Tab {}", tab_count + 1));
                    info!("新建 Tab");
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+W — 关闭当前 Tab
                KeyCode::KeyW => {
                    let tab_id = layout.active_tab().id;
                    layout.close_tab(tab_id);
                    // 当所有 Tab 都关闭时，退出应用
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
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+R — 水平分屏
                KeyCode::KeyR => {
                    if let Some(new_pane_id) = layout.split_pane(SplitDirection::Horizontal) {
                        info!("水平分屏，新 Pane ID: {}", new_pane_id);
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+X — 关闭当前 Pane
                KeyCode::KeyX => {
                    let tab_closed = layout.close_pane();
                    if tab_closed {
                        info!("Pane 关闭导致 Tab 关闭");
                        // 当所有 Tab 都关闭时，退出应用
                        if layout.tab_infos().is_empty() {
                            info!("所有 Tab 已关闭，退出应用");
                            event_loop.exit();
                        }
                    }
                    self.request_redraw();
                    return true;
                }
                // Ctrl+Shift+Tab — 上一个 Tab
                KeyCode::Tab => {
                    layout.prev_tab();
                    info!("切换到上一个 Tab");
                    self.request_redraw();
                    return true;
                }
                _ => {}
            }
        }

        // Ctrl+Tab — 下一个 Tab（无 Shift）
        if ctrl && !shift && !alt && key_code == KeyCode::Tab {
            layout.next_tab();
            info!("切换到下一个 Tab");
            self.request_redraw();
            return true;
        }

        // Alt+ArrowLeft — 切换到上一个 Pane
        if alt && !ctrl && !shift && key_code == KeyCode::ArrowLeft {
            layout.focus_prev_pane();
            info!("切换到上一个 Pane");
            self.request_redraw();
            return true;
        }

        // Alt+ArrowRight — 切换到下一个 Pane
        if alt && !ctrl && !shift && key_code == KeyCode::ArrowRight {
            layout.focus_next_pane();
            info!("切换到下一个 Pane");
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

                // 初始化布局管理器（需要窗口尺寸）
                self.layout = Some(LayoutManager::new(
                    size.width as f32,
                    size.height as f32,
                ));
                self.connected = false;
                self.window = Some(window);
                info!("unterm-ui 窗口已创建");
            }
            Err(e) => {
                tracing::error!("窗口创建失败: {}", e);
                event_loop.exit();
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
                }
                self.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                // 收集渲染数据（借用 layout 为不可变引用）
                let render_data = if let Some(layout) = &self.layout {
                    // Tab 栏数据
                    let tab_infos: Vec<TabInfo> = layout
                        .tab_infos()
                        .iter()
                        .map(|(id, title, active)| TabInfo {
                            id: *id,
                            title: title.to_string(),
                            is_active: *active,
                        })
                        .collect();

                    // Pane 布局和内容
                    let pane_layouts = layout.compute_pane_layouts();
                    let pane_contents: Vec<PaneContent> = pane_layouts
                        .iter()
                        .map(|pl| {
                            let text = self
                                .pane_contents
                                .get(&pl.pane_id)
                                .cloned()
                                .unwrap_or_else(|| {
                                    format!(
                                        "Pane {} ({}x{})\n等待连接...",
                                        pl.pane_id, pl.cols, pl.rows
                                    )
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

                    // 状态栏文本
                    let status_text = if self.connected {
                        rust_i18n::t!("ui.status_connected").to_string()
                    } else {
                        rust_i18n::t!("ui.status_disconnected").to_string()
                    };

                    // 布局区域（layout::Rect -> render::PaneRect 字段拷贝）
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

                // 调用渲染器绘制
                if let (Some(renderer), Some((tab_infos, pane_contents, status_text, status_rect, tab_bar_rect))) =
                    (&mut self.renderer, render_data)
                {
                    if let Err(e) = renderer.draw_frame(
                        &tab_infos,
                        &pane_contents,
                        &status_text,
                        status_rect,
                        tab_bar_rect,
                    ) {
                        tracing::error!("渲染错误: {}", e);
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // 只在按下时处理
                if event.state != ElementState::Pressed {
                    return;
                }

                // 先尝试快捷键
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    if self.handle_shortcut(key_code, event_loop) {
                        return;
                    }
                }

                // 非快捷键：通过 InputHandler 转换后路由到当前激活 pane 的 session
                let text = event.text.as_ref().map(|s| s.as_str());
                if let Some(_bytes) = self.input_handler.key_to_sequence(
                    &event.physical_key,
                    event.state,
                    text,
                ) {
                    if let Some(layout) = &self.layout {
                        if let Some(_session_id) = layout.active_session_id() {
                            // TODO: 通过 IPC 发送到 unterm-core
                            // client.call("exec.send", json!({
                            //     "session_id": session_id,
                            //     "input": String::from_utf8_lossy(&bytes)
                            // }))
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

    // 语言检测
    let locale = std::env::var("UNTERM_LOCALE")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    if locale.starts_with("zh") {
        rust_i18n::set_locale("zh-CN");
    }

    info!("unterm-ui 启动中...");

    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
