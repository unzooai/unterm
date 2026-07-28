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
    /// The shell the config asked for. Without this the session falls back to
    /// `%COMSPEC%`, which on Windows is `cmd.exe` -- not what a config naming
    /// `pwsh` meant, and it emits its output in the console codepage rather
    /// than UTF-8.
    shell: Option<portable_pty::CommandBuilder>,
    /// Whether shift is down, which is what separates a scroll from a key the
    /// program should receive.
    shift_held: bool,
    /// Whether control is down.
    ctrl_held: bool,
    /// Where the pointer is, in pixels. Winit reports movement and buttons
    /// separately, so a click has to be told where it happened.
    pointer: (f32, f32),
    /// The selection being dragged out, if one is.
    drag: Option<crate::select::Drag>,
    /// The text of the finished selection, kept so a copy key can find it.
    selected: Option<String>,
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
        // The config's own fallback list comes first: someone who named a font
        // meant it, and the built-in list is only what to try after.
        let fallbacks: Vec<String> = config
            .list_of("font_fallback")
            .ok()
            .flatten()
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| match value {
                        unterm_engine::next_core::config::Value::Str(name) => Some(name.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let pixel_size = config
            .float_of("font_size")
            .ok()
            .flatten()
            .unwrap_or(13.0)
            .max(6.0);

        Ok(Self {
            engine: NextCoreEngine,
            font: TerminalFont::open_with_fallback(pixel_size.round() as u32, &fallbacks)?,
            atlas: GlyphAtlas::new(1024, 1024),
            colors: colors_from(config),
            shell: shell_from(config),
            shift_held: false,
            ctrl_held: false,
            pointer: (0.0, 0.0),
            drag: None,
            selected: None,
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
            command: self.shell.clone(),
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

    /// Which cell the pointer is over, in scrollback coordinates.
    fn cell_under_pointer(&self) -> unterm_engine::next_core::selection::SelectionPoint {
        // The viewport's top row, so a selection made while scrolled back stays
        // on the text it was made on rather than on whatever is there later.
        let top = self
            .state
            .as_ref()
            .and_then(|live| self.engine.read_styled_screen(live.session_id).ok())
            .map(|snapshot| snapshot.lines.first().map(|line| line.row).unwrap_or(0))
            .unwrap_or(0);

        crate::select::cell_at(self.pointer.0, self.pointer.1, self.font.metrics(), top)
    }

    /// Extract what the current drag covers.
    fn update_selection(&mut self) {
        use unterm_engine::next_core::selection::{selected_text, SelectionRow};

        let Some(drag) = self.drag else {
            return;
        };
        let Some(selection) = drag.selection() else {
            // Still a click, not a selection.
            return;
        };
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let Ok(snapshot) = self.engine.read_styled_screen(live.session_id) else {
            return;
        };

        let rows: Vec<SelectionRow> = snapshot
            .lines
            .iter()
            .map(|line| SelectionRow {
                row: line.row,
                text: line.cells.iter().map(|cell| cell.ch).collect(),
                wrapped: line.wrapped,
            })
            .collect();

        let text = selected_text(&selection, &rows);
        if !text.is_empty() {
            // What was selected, so a copy that comes out wrong can be traced
            // to the selection rather than to the clipboard.
            log::debug!("selected {} char(s): {:?}", text.chars().count(), text);
        }
        self.selected = (!text.is_empty()).then_some(text);
    }

    /// Put the selection on the clipboard.
    ///
    /// A selection nobody can copy is decoration, and this is the one action
    /// every terminal user reaches for within a minute of selecting anything.
    fn copy_selection(&mut self) {
        let Some(text) = self.selected.clone() else {
            return;
        };
        match arboard::Clipboard::new().and_then(|mut board| board.set_text(text)) {
            Ok(()) => {}
            // Worth saying rather than swallowing: a copy that silently does
            // nothing sends the user hunting through their clipboard manager.
            Err(err) => log::warn!("could not copy to the clipboard: {err}"),
        }
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

            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
                self.ctrl_held = modifiers.state().control_key();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let Some(live) = self.state.as_ref() else {
                    return;
                };

                use winit::keyboard::{Key, NamedKey};

                // Ctrl+Shift+C copies. Plain Ctrl+C has to stay interrupt, or
                // a running program can never be stopped.
                if self.shift_held && self.ctrl_held {
                    if matches!(&event.logical_key, Key::Character(text) if text.eq_ignore_ascii_case("c"))
                    {
                        self.copy_selection();
                        return;
                    }
                }

                // Shift+Page scrolls the viewport; unshifted pages belong to
                // the program, which is how a pager gets its own keys.
                if self.shift_held {
                    let rows = self
                        .engine
                        .read_styled_screen(live.session_id)
                        .map(|snapshot| snapshot.rows)
                        .unwrap_or(24);
                    let page = crate::scroll::lines_for_page(rows);
                    let delta = match &event.logical_key {
                        Key::Named(NamedKey::PageUp) => Some(-page),
                        Key::Named(NamedKey::PageDown) => Some(page),
                        _ => None,
                    };
                    if let Some(delta) = delta {
                        let _ = self.engine.scroll_viewport_by(live.session_id, delta);
                        return;
                    }
                }

                if let Some(text) = encode(&event) {
                    let _ = self.engine.write_input(live.session_id, &text);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = (position.x as f32, position.y as f32);
                if self.drag.is_some() {
                    let point = self.cell_under_pointer();
                    if let Some(drag) = self.drag.as_mut() {
                        drag.extend(point);
                    }
                    self.update_selection();
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                use winit::event::MouseButton;
                if button != MouseButton::Left {
                    return;
                }
                match state {
                    ElementState::Pressed => {
                        let shape = if self.shift_held {
                            unterm_engine::next_core::selection::SelectionShape::Block
                        } else {
                            unterm_engine::next_core::selection::SelectionShape::Linear
                        };
                        self.drag =
                            Some(crate::select::Drag::start(self.cell_under_pointer(), shape));
                        self.selected = None;
                    }
                    ElementState::Released => {
                        self.update_selection();
                        self.drag = None;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                use winit::event::MouseScrollDelta;
                let cell_height = self.font.metrics().height;
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        crate::scroll::lines_for_wheel(crate::scroll::WheelDelta::Lines(y), cell_height)
                    }
                    MouseScrollDelta::PixelDelta(position) => crate::scroll::lines_for_wheel(
                        crate::scroll::WheelDelta::Pixels(position.y as f32),
                        cell_height,
                    ),
                };
                if lines != 0 {
                    if let Some(live) = self.state.as_ref() {
                        // Positive is toward older output, and the wheel rolls
                        // away from you to go back in time.
                        let _ = self.engine.scroll_viewport_by(live.session_id, -lines);
                    }
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

/// The command the config names, if it names one.
///
/// A string is the program; a list is the program and its arguments. Real
/// shells need arguments -- `pwsh -NoLogo`, `bash --login` -- and a setting
/// that cannot express them makes the user pick between their flags and the
/// config.
fn shell_from(config: &config::Config) -> Option<portable_pty::CommandBuilder> {
    match config.get("shell")? {
        config::Value::Str(program) => Some(portable_pty::CommandBuilder::new(program)),
        config::Value::List(parts) => {
            let mut words = parts.iter().filter_map(|part| match part {
                config::Value::Str(word) => Some(word.as_str()),
                _ => None,
            });
            let mut command = portable_pty::CommandBuilder::new(words.next()?);
            for word in words {
                command.arg(word);
            }
            Some(command)
        }
        _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_configured_shell_is_used() {
        let config = config::parse("shell = \"pwsh.exe\"").expect("config should parse");

        let shell = shell_from(&config).expect("a named shell should be used");

        // Without this the session falls back to %COMSPEC% -- cmd.exe on
        // Windows, which is not what a config naming pwsh meant.
        assert_eq!(shell.get_argv()[0], "pwsh.exe");
    }

    #[test]
    fn a_shell_can_carry_its_arguments() {
        let config = config::parse(r#"shell = ["pwsh.exe", "-NoLogo"]"#)
            .expect("config should parse");

        let shell = shell_from(&config).expect("a named shell should be used");

        // A setting that cannot express arguments makes the user choose
        // between their flags and the config.
        let argv = shell.get_argv();
        assert_eq!(argv[0], "pwsh.exe");
        assert_eq!(argv[1], "-NoLogo");
    }

    #[test]
    fn a_config_naming_no_shell_leaves_the_choice_to_the_engine() {
        let config = config::parse("font_size = 13").expect("config should parse");

        assert!(shell_from(&config).is_none());
    }
}
