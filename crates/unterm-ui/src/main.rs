//! unterm-ui: Unterm GUI 渲染进程

mod render;
mod input;
mod layout;
mod client;

use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId, WindowAttributes};

rust_i18n::i18n!("locales", fallback = "en");

/// 应用状态
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<render::Renderer>,
    input_handler: input::InputHandler,
    layout: Option<layout::Layout>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            input_handler: input::InputHandler::new(),
            layout: None,
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

        let (min_w, min_h) = layout::Layout::min_window_size();
        let attrs = WindowAttributes::default()
            .with_title("Unterm")
            .with_inner_size(LogicalSize::new(1024, 768))
            .with_min_inner_size(LogicalSize::new(min_w, min_h));

        match event_loop.create_window(attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                let size = window.inner_size();

                match render::Renderer::new(window.clone()) {
                    Ok(mut renderer) => {
                        let welcome = rust_i18n::t!("ui.welcome");
                        renderer.set_text(&welcome);
                        self.renderer = Some(renderer);
                    }
                    Err(e) => {
                        tracing::error!("渲染器初始化失败: {}", e);
                        event_loop.exit();
                        return;
                    }
                }

                self.layout = Some(layout::Layout::new(size.width as f32, size.height as f32));
                self.window = Some(window);
                info!("unterm-ui 窗口已创建");
            }
            Err(e) => {
                tracing::error!("窗口创建失败: {}", e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("unterm-ui 正在关闭...");
                event_loop.exit();
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(width, height);
                }
                if let Some(layout) = &mut self.layout {
                    layout.resize(width as f32, height as f32);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    if let Err(e) = renderer.draw_frame() {
                        tracing::error!("渲染错误: {}", e);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let text = event.text.as_ref().map(|s| s.as_str());
                if let Some(_sequence) = self.input_handler.key_to_sequence(
                    &event.physical_key,
                    event.state,
                    text,
                ) {
                    // TODO: 通过 IPC 发送到 unterm-core
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
