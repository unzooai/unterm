//! The window, the surface, and the shell behind them.
//!
//! Everything here needs a display to exercise, which is why the frame-building
//! half lives in `terminal.rs` and is tested on its own. What is left is
//! plumbing: winit gives events, next-core gives a screen, unterm-render turns
//! one into the other.

use crate::terminal::{colors_from, frame_quads, TerminalFont};
use anyhow::Context;
use std::sync::Arc;
use unterm_engine::next_core::{config, key_encoding, NextCoreEngine};
use unterm_engine::{
    CreateSessionRequest, InputEngine, LaunchPolicySnapshot, ScreenEngine, SessionEngine,
};
use unterm_render::atlas::GlyphAtlas;
use unterm_render::gpu::Renderer;
use unterm_render::quads::FrameColors;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

pub struct App {
    engine: NextCoreEngine,
    font: TerminalFont,
    atlas: GlyphAtlas,
    colors: FrameColors,
    state: Option<Live>,
    /// The last screen we drew, so a frame is skipped when nothing changed.
    /// A terminal is idle most of the time; redrawing an unchanged screen at
    /// display rate is a fan that never stops.
    drawn_revision: Option<u64>,
}

struct Live {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    atlas_texture: wgpu::Texture,
    session_id: usize,
    width: u32,
    height: u32,
}

impl App {
    pub fn new(config: &config::Config) -> anyhow::Result<Self> {
        let pixel_size = config
            .float_of("font_size")
            .ok()
            .flatten()
            .unwrap_or(13.0)
            .max(6.0);

        Ok(Self {
            engine: NextCoreEngine,
            font: TerminalFont::open(pixel_size.round() as u32)?,
            atlas: GlyphAtlas::new(1024, 1024),
            colors: colors_from(config),
            state: None,
            drawn_revision: None,
        })
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<Live> {
        let metrics = self.font.metrics();
        let attributes = Window::default_attributes()
            .with_title("Unterm")
            .with_inner_size(winit::dpi::LogicalSize::new(
                metrics.width * 100.0,
                metrics.height * 30.0,
            ));
        let window = Arc::new(event_loop.create_window(attributes)?);

        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            },
        ))
        .context("no GPU adapter available")?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        let capabilities = surface.get_capabilities(&adapter);
        // Prefer a non-sRGB format: the colours here are already the values the
        // config asked for, and an sRGB target would convert them a second time.
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .unwrap_or(capabilities.formats[0]);

        let renderer = Renderer::new(device, queue, format);
        let atlas_texture = renderer.upload_atlas(&self.atlas);

        let (cols, rows) = self.font.grid_for(size.width as f32, size.height as f32);
        let session = self.engine.create_session(CreateSessionRequest {
            cols,
            rows,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: LaunchPolicySnapshot::default(),
        })?;

        let live = Live {
            window,
            surface,
            renderer,
            atlas_texture,
            session_id: session.id,
            width: size.width.max(1),
            height: size.height.max(1),
        };
        live.configure(format);
        Ok(live)
    }

    fn draw(&mut self) {
        let Some(live) = self.state.as_mut() else {
            return;
        };
        let Ok(snapshot) = self.engine.read_styled_screen(live.session_id) else {
            return;
        };

        let quads = frame_quads(&snapshot, &mut self.font, &mut self.atlas, self.colors);
        // The atlas may have grown while building this frame's glyphs, so the
        // texture is uploaded after them rather than before.
        live.atlas_texture = live.renderer.upload_atlas(&self.atlas);

        let Ok(frame) = live.surface.get_current_texture() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        live.renderer.draw(
            &view,
            live.width,
            live.height,
            &quads,
            &live.atlas_texture,
            self.colors.background,
        );
        frame.present();
        self.drawn_revision = Some(snapshot.revision);
    }

    /// Redraw only when the screen actually moved.
    fn needs_redraw(&self) -> bool {
        let Some(live) = self.state.as_ref() else {
            return false;
        };
        match self.engine.screen_revision(live.session_id) {
            Ok(revision) => Some(revision) != self.drawn_revision,
            Err(_) => false,
        }
    }
}

impl Live {
    fn configure(&self, format: wgpu::TextureFormat) {
        self.surface.configure(
            self.renderer.device(),
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: self.width,
                height: self.height,
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        match self.start(event_loop) {
            Ok(live) => {
                live.window.request_redraw();
                self.state = Some(live);
            }
            Err(err) => {
                log::error!("could not start: {err:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(live) = self.state.take() {
                    // Destroy the session rather than leaving the shell running
                    // with nothing attached to it.
                    let _ = self.engine.destroy_session(live.session_id);
                }
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                let (width, height) = (size.width.max(1), size.height.max(1));
                let (cols, rows) = self.font.grid_for(width as f32, height as f32);
                if let Some(live) = self.state.as_mut() {
                    live.width = width;
                    live.height = height;
                    live.configure(live.renderer.format());
                    // The shell has to learn the new grid, or it keeps wrapping
                    // at the old width.
                    let _ = self.engine.resize_session(live.session_id, cols, rows);
                    live.window.request_redraw();
                }
                self.drawn_revision = None;
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let Some(live) = self.state.as_ref() else {
                    return;
                };
                if let Some(text) = encode(&event) {
                    let _ = self.engine.write_input(live.session_id, &text);
                }
            }

            WindowEvent::RedrawRequested => {
                self.draw();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.needs_redraw() {
            if let Some(live) = self.state.as_ref() {
                live.window.request_redraw();
            }
        }
        // Polling rather than waiting: the shell produces output on its own
        // schedule, and nothing wakes the loop when it does.
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    }
}

/// Turn a key press into the bytes a PTY expects.
///
/// Named keys go through next-core's encoder, which knows the escape sequences;
/// printable text is sent as typed, which is what the shell reads for anything
/// the encoder has no opinion about.
fn encode(event: &winit::event::KeyEvent) -> Option<String> {
    use termwiz::input::{KeyCode, Modifiers};
    use winit::keyboard::{Key, NamedKey};

    let key = match &event.logical_key {
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Backspace) => KeyCode::Backspace,
        Key::Named(NamedKey::Tab) => KeyCode::Tab,
        Key::Named(NamedKey::Escape) => KeyCode::Escape,
        Key::Named(NamedKey::ArrowUp) => KeyCode::UpArrow,
        Key::Named(NamedKey::ArrowDown) => KeyCode::DownArrow,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::LeftArrow,
        Key::Named(NamedKey::ArrowRight) => KeyCode::RightArrow,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        Key::Named(NamedKey::PageUp) => KeyCode::PageUp,
        Key::Named(NamedKey::PageDown) => KeyCode::PageDown,
        Key::Named(NamedKey::Delete) => KeyCode::Delete,
        Key::Character(text) => {
            return Some(text.to_string());
        }
        Key::Named(NamedKey::Space) => return Some(" ".to_string()),
        _ => return None,
    };

    key_encoding::encode_key(key, Modifiers::NONE)
}
