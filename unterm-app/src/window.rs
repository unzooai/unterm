//! The window, the surface, and the shell behind them.
//!
//! Everything here needs a display to exercise, which is why the frame-building
//! half lives in `terminal.rs` and is tested on its own. What is left is
//! plumbing: winit gives events, next-core gives a screen, unterm-render turns
//! one into the other.

use crate::terminal::{colors_from, TerminalFont};

/// How many matches a search collects.
///
/// Enough that the count means something, bounded so a pattern matching every
/// line of a long scrollback does not stall the keystroke that typed it.
const MAX_SEARCH_MATCHES: usize = 500;
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
    /// The split tree. next-core owns what an arrangement *is*; the window
    /// only says when to split and reads back where the panes go.
    tabs: unterm_engine::next_core::tabs::TabRegistry,
    tab_id: Option<usize>,
    /// The selection being dragged out, if one is.
    drag: Option<crate::select::Drag>,
    /// The text of the finished selection, kept so a copy key can find it.
    selected: Option<String>,
    /// The last screen we drew, so a frame is skipped when nothing changed.
    /// A terminal is idle most of the time; redrawing an unchanged screen at
    /// display rate is a fan that never stops.
    drawn_revision: Option<u64>,
    /// Which agent-write banner was on screen when we last drew, so one
    /// appearing or being answered is itself a reason to draw again.
    drawn_confirmation: Option<u64>,
    /// Text an input method is still composing, not yet the shell's.
    preedit: crate::ime::Preedit,
    /// The open search, if there is one.
    search: Option<crate::search::Search>,
    /// What the program wants from the mouse, as of the last frame drawn.
    ///
    /// Cached rather than read per event: a motion arrives a hundred times a
    /// second and building a screen snapshot for each would be most of a
    /// frame's work spent deciding who owns a pointer that has not moved a
    /// cell. Modes only change when the program writes an escape sequence,
    /// which changes the screen, which draws a frame -- so the cache is never
    /// more than one frame behind the thing that sets it.
    mouse_modes: unterm_engine::next_core::mouse_encoding::MouseModes,
    /// Which mouse button is down, so a drag reports the right one.
    held_mouse_button: Option<unterm_engine::next_core::mouse_encoding::MouseButton>,
    alt_held: bool,
    /// The title last set, so an unchanged one is not set again every frame.
    window_title: Option<String>,
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
            drawn_confirmation: None,
            preedit: crate::ime::Preedit::default(),
            search: None,
            mouse_modes: Default::default(),
            held_mouse_button: None,
            alt_held: false,
            window_title: None,
            font: TerminalFont::open_with_fallback(pixel_size.round() as u32, &fallbacks)?,
            atlas: GlyphAtlas::new(1024, 1024),
            colors: colors_from(config),
            shell: launch_shell(config),
            shift_held: false,
            ctrl_held: false,
            pointer: (0.0, 0.0),
            tabs: unterm_engine::next_core::tabs::TabRegistry::new(),
            tab_id: None,
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
        // Without this the system never starts an input method, and a Chinese
        // or Japanese keyboard can only produce Latin letters.
        window.set_ime_allowed(true);

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

        // The first pane is a tab of one. Recording it here means a later split
            // has an arrangement to grow rather than one to infer.
        self.tab_id = self.tabs.create_tab(session.id).ok();

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
        if self.state.is_none() {
            return;
        }
        let placements = self.placements();
        let dividers = self.divider_quads();
        let Some((window_width, session_id)) = self
            .state
            .as_ref()
            .map(|live| (live.width as f32, live.session_id))
        else {
            return;
        };

        let mut quads = unterm_render::quads::FrameQuads::default();
        let mut revision = 0u64;
        for placement in &placements {
            let Ok(snapshot) = self.engine.read_styled_screen(placement.session_id) else {
                continue;
            };
            revision = revision.wrapping_add(snapshot.revision);
            if placement.session_id == session_id {
                self.mouse_modes = snapshot.mouse;
            }
            crate::terminal::append_pane(
                &snapshot,
                &mut self.font,
                &mut self.atlas,
                self.colors,
                placement.origin,
                &mut quads,
            );
        }
        if placements.is_empty() {
            let Ok(snapshot) = self.engine.read_styled_screen(session_id) else {
                return;
            };
            revision = snapshot.revision;
            self.mouse_modes = snapshot.mouse;
            crate::terminal::append_pane(
                &snapshot,
                &mut self.font,
                &mut self.atlas,
                self.colors,
                (0.0, 0.0),
                &mut quads,
            );
        }
        quads.backgrounds.extend(dividers);
        self.append_preedit(&mut quads);
        self.append_search_bar(window_width, &mut quads);
        let tab_count = self.tabs.tab_count();
        let active_tab = self
            .tab_id
            .and_then(|id| self.tabs.tab_ids().iter().position(|c| *c == id))
            .unwrap_or(0);
        quads.backgrounds.extend(crate::tabbar::quads(
            tab_count,
            active_tab,
            window_width,
            self.font.metrics(),
            self.colors,
        ));
        append_tab_labels(
            tab_count,
            active_tab,
            window_width,
            &mut self.font,
            &mut self.atlas,
            self.colors,
            &mut quads,
        );

        append_confirmation_banner(
            window_width,
            &mut self.font,
            &mut self.atlas,
            self.colors,
            &mut quads,
        );

        let Some(live) = self.state.as_mut() else {
            return;
        };
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
        self.drawn_revision = Some(revision);
        self.drawn_confirmation = unterm_mcp::handler::pending_confirmation_view().map(|v| v.id);
    }

    /// Offer a mouse event to the program. True when it took it.
    ///
    /// The front end asks before acting on a click of its own: with reporting
    /// on, a drag that also selected text would give the user a selection they
    /// did not ask for on top of a click that did work.
    fn report_mouse(
        &mut self,
        kind: unterm_engine::next_core::mouse_encoding::MouseEventKind,
        button: Option<unterm_engine::next_core::mouse_encoding::MouseButton>,
    ) -> bool {
        let Some(live) = self.state.as_ref() else {
            return false;
        };
        let metrics = self.font.metrics();
        let top = crate::tabbar::terminal_top(metrics, self.tabs.tab_count());
        let column = (self.pointer.0 / metrics.width.max(1.0)) as usize;
        let row = ((self.pointer.1 - top).max(0.0) / metrics.height.max(1.0)) as usize;

        let held = crate::mouse::Held {
            shift: self.shift_held,
            ctrl: self.ctrl_held,
            alt: self.alt_held,
        };
        match crate::mouse::route(self.mouse_modes, kind, button, column, row, held) {
            crate::mouse::Route::ToProgram(event) => {
                let _ = self.engine.report_mouse(live.session_id, event);
                true
            }
            crate::mouse::Route::ToTerminal => false,
        }
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
    /// Carry out a key binding.
    fn run_key_action(&mut self, action: crate::keys::Action, session_id: usize) {
        use crate::keys::Action;
        match action {
            Action::Copy => self.copy_selection(),
            Action::Paste => self.paste_clipboard(),
            Action::SplitRight => {
                self.split(unterm_engine::next_core::layout::SplitAxis::Horizontal)
            }
            Action::SplitDown => self.split(unterm_engine::next_core::layout::SplitAxis::Vertical),
            Action::Search => {
                self.search = Some(crate::search::Search::default());
                self.drawn_revision = None;
            }
            Action::NewTab => self.new_tab(),
            Action::NextTab => self.cycle_tab(1),
            Action::PreviousTab => self.cycle_tab(-1),
            Action::CloseTab => self.close_tab(),
            Action::ScrollPageUp | Action::ScrollPageDown => {
                let rows = self
                    .engine
                    .read_styled_screen(session_id)
                    .map(|snapshot| snapshot.rows)
                    .unwrap_or(24);
                let page = crate::scroll::lines_for_page(rows);
                let delta = if action == Action::ScrollPageUp {
                    -page
                } else {
                    page
                };
                let _ = self.engine.scroll_viewport_by(session_id, delta);
            }
        }
    }

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

    /// Send the clipboard to the shell.
    ///
    /// Through the engine's paste rather than as typed input, because the
    /// engine knows whether the program asked for bracketed paste. Without the
    /// brackets a multi-line paste is executed a line at a time -- paste a
    /// script and you have run it.
    fn paste_clipboard(&mut self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let text = match arboard::Clipboard::new().and_then(|mut board| board.get_text()) {
            Ok(text) => text,
            Err(err) => {
                log::warn!("could not read the clipboard: {err}");
                return;
            }
        };
        if text.is_empty() {
            return;
        }
        if let Err(err) = self.engine.paste_input(live.session_id, &text) {
            log::warn!("could not paste: {err:#}");
        }
    }

    /// The pane keys and pastes go to.
    fn focused_session(&self) -> usize {
        self.tab_id
            .and_then(|tab_id| self.tabs.active_pane(tab_id))
            .or_else(|| self.state.as_ref().map(|live| live.session_id))
            .unwrap_or(0)
    }

    /// Split the focused pane.
    fn split(&mut self, axis: unterm_engine::next_core::layout::SplitAxis) {
        let (Some(tab_id), Some(live)) = (self.tab_id, self.state.as_ref()) else {
            return;
        };
        let focused = self.tabs.active_pane(tab_id).unwrap_or(live.session_id);

        // Size the new session to the rectangle it will actually get, so its
        // shell never sees a width it is not being drawn at.
        let (cols, rows) = self.font.grid_for(live.width as f32, live.height as f32);
        let session = match self.engine.create_session(CreateSessionRequest {
            cols: cols / 2,
            rows,
            command_dir: None,
            command: self.shell.clone(),
            env: Vec::new(),
            launch_policy: LaunchPolicySnapshot::default(),
        }) {
            Ok(session) => session,
            Err(err) => {
                log::warn!("could not open a pane: {err:#}");
                return;
            }
        };

        if let Err(err) = self.tabs.split(focused, session.id, axis, 0.5) {
            log::warn!("could not split: {err:#}");
            let _ = self.engine.destroy_session(session.id);
            return;
        }
        self.tabs.set_active_pane(session.id);
        self.resize_panes();
        if let Some(live) = self.state.as_ref() {
            live.window.request_redraw();
        }
        self.drawn_revision = None;
    }

    /// Open a tab, with a shell of its own.
    fn new_tab(&mut self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let (cols, rows) = self.font.grid_for(live.width as f32, self.terminal_height());
        let session = match self.engine.create_session(CreateSessionRequest {
            cols,
            rows,
            command_dir: None,
            command: self.shell.clone(),
            env: Vec::new(),
            launch_policy: LaunchPolicySnapshot::default(),
        }) {
            Ok(session) => session,
            Err(err) => {
                log::warn!("could not open a tab: {err:#}");
                return;
            }
        };
        match self.tabs.create_tab(session.id) {
            Ok(tab_id) => {
                self.tabs.set_active_tab(tab_id);
                self.tab_id = Some(tab_id);
                self.focus_session(session.id);
            }
            Err(err) => {
                log::warn!("could not record the tab: {err:#}");
                let _ = self.engine.destroy_session(session.id);
            }
        }
    }

    /// Move to the next tab along, wrapping at the ends.
    ///
    /// Wrapping rather than stopping: with three tabs, cycling forward twice
    /// from the last should land somewhere, and a key that does nothing at the
    /// edge reads as a key that is broken.
    fn cycle_tab(&mut self, step: isize) {
        let ids = self.tabs.tab_ids();
        if ids.len() < 2 {
            return;
        }
        let current = self.tab_id.or_else(|| self.tabs.active_tab());
        let next = ids[next_tab_index(&ids, current, step)];
        self.tabs.set_active_tab(next);
        self.tab_id = Some(next);
        if let Some(pane) = self.tabs.active_pane(next) {
            self.focus_session(pane);
        }
    }

    /// Close the active tab and everything in it.
    ///
    /// The last tab is not closable: a window with no tab has nothing to show
    /// and no way back, so closing the window is the user's own decision.
    fn close_tab(&mut self) {
        let ids = self.tabs.tab_ids();
        if ids.len() < 2 {
            return;
        }
        let Some(tab_id) = self.tab_id.or_else(|| self.tabs.active_tab()) else {
            return;
        };
        for pane in self.tabs.pane_ids(tab_id) {
            let _ = self.engine.destroy_session(pane);
        }
        self.tabs.forget_tab(tab_id);
        let remaining = self.tabs.tab_ids();
        let Some(next) = remaining.first().copied() else {
            return;
        };
        self.tabs.set_active_tab(next);
        self.tab_id = Some(next);
        if let Some(pane) = self.tabs.active_pane(next) {
            self.focus_session(pane);
        }
    }

    /// Point the window at a pane, and redraw.
    fn focus_session(&mut self, session_id: usize) {
        self.tabs.set_active_pane(session_id);
        if let Some(live) = self.state.as_mut() {
            live.session_id = session_id;
        }
        self.resize_panes();
        if let Some(live) = self.state.as_ref() {
            live.window.request_redraw();
        }
        self.drawn_revision = None;
    }

    /// How tall the terminal area is, once the tab bar has taken its share.
    fn terminal_height(&self) -> f32 {
        let height = self.state.as_ref().map(|live| live.height).unwrap_or(600) as f32;
        crate::tabbar::terminal_height(height, self.font.metrics(), self.tabs.tab_count())
    }

    /// Make the window's tabs match the engine's sessions.
    ///
    /// The engine is where sessions actually live, and it is not only this
    /// window that makes them: an agent calling `session.create` over MCP
    /// creates one too, and without this the window would never show it. The
    /// same pass drops tabs whose shells have exited, so a tab bar cannot
    /// outlive what it names.
    fn sync_tabs(&mut self) {
        let Ok(sessions) = unterm_engine::SessionEngine::list_sessions(&self.engine) else {
            return;
        };
        let live_ids: std::collections::HashSet<usize> =
            sessions.iter().map(|session| session.id).collect();
        let mut changed = false;

        for tab_id in self.tabs.tab_ids() {
            let panes = self.tabs.pane_ids(tab_id);
            if panes.iter().any(|pane| live_ids.contains(pane)) {
                continue;
            }
            // Every shell in this tab is gone; nothing left to show.
            self.tabs.forget_tab(tab_id);
            changed = true;
        }

        for session in &sessions {
            if self.tabs.tab_of_pane(session.id).is_some() {
                continue;
            }
            // A pane split off another belongs beside it, not in a tab of its
            // own: an agent asking for a split and getting a new tab got
            // something else than it asked for.
            let split = session
                .split_from
                .filter(|source| self.tabs.tab_of_pane(*source).is_some());
            let outcome = match split {
                Some(source) => self
                    .tabs
                    .split(
                        source,
                        session.id,
                        unterm_engine::next_core::layout::SplitAxis::Horizontal,
                        0.5,
                    )
                    .map(|_| ()),
                None => self.tabs.create_tab(session.id).map(|_| ()),
            };
            match outcome {
                Ok(()) => changed = true,
                Err(err) => log::warn!("could not adopt session {}: {err:#}", session.id),
            }
        }

        if !changed {
            return;
        }
        // The window may have been left pointing at a tab that no longer
        // exists, or at none at all.
        let ids = self.tabs.tab_ids();
        let still_there = self
            .tab_id
            .map(|id| ids.contains(&id))
            .unwrap_or(false);
        if !still_there {
            if let Some(first) = ids.first().copied() {
                self.tabs.set_active_tab(first);
                self.tab_id = Some(first);
                if let Some(pane) = self.tabs.active_pane(first) {
                    self.tabs.set_active_pane(pane);
                    if let Some(live) = self.state.as_mut() {
                        live.session_id = pane;
                    }
                }
            } else {
                self.tab_id = None;
            }
        }
        self.resize_panes();
        self.drawn_revision = None;
    }

    /// Type into the open search. Returns true when the key was the search's.
    ///
    /// Everything printable extends the pattern; Enter steps through the
    /// matches and Esc closes. Nothing else is taken, so a key the search has
    /// no use for still reaches the shell rather than vanishing.
    fn handle_search_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        use winit::keyboard::Key as WinitKey;
        let Some(mut search) = self.search.take() else {
            return false;
        };

        let named = match &event.logical_key {
            WinitKey::Named(named) => Some(format!("{named:?}")),
            _ => None,
        };
        let character = match &event.logical_key {
            WinitKey::Character(text) => Some(text.to_string()),
            _ => None,
        };

        let mut keep = true;
        let mut research = false;
        match crate::search::key_for(
            named.as_deref(),
            character.as_deref(),
            self.ctrl_held,
            self.shift_held,
        ) {
            crate::search::Key::Close => keep = false,
            crate::search::Key::Step(delta) => search.step(delta),
            crate::search::Key::Backspace => {
                search.pattern.pop();
                research = true;
            }
            crate::search::Key::Type(text) => {
                search.pattern.push_str(&text);
                research = true;
            }
            crate::search::Key::NotOurs => {
                self.search = Some(search);
                return false;
            }
        }

        if keep {
            if research {
                let matches = self
                    .state
                    .as_ref()
                    .filter(|_| !search.pattern.is_empty())
                    .and_then(|live| {
                        self.engine
                            .search(live.session_id, &search.pattern, MAX_SEARCH_MATCHES)
                            .ok()
                    })
                    .unwrap_or_default();
                search.adopt(matches);
            }
            // Follow the current match, so finding something shows it.
            if let (Some(found), Some(live)) = (search.current(), self.state.as_ref()) {
                let _ = self
                    .engine
                    .scroll_viewport_to(live.session_id, found.row as isize);
            }
            self.search = Some(search);
        }
        self.drawn_revision = None;
        if let Some(live) = self.state.as_ref() {
            live.window.request_redraw();
        }
        true
    }

    /// The search bar, along the bottom.
    ///
    /// The bottom rather than the top: the tab bar is up there, and a bar
    /// that moved the terminal's rows around every time it opened would
    /// reflow the shell mid-search.
    fn append_search_bar(
        &mut self,
        window_width: f32,
        quads: &mut unterm_render::quads::FrameQuads,
    ) {
        let Some(label) = self.search.as_ref().map(|search| search.label()) else {
            return;
        };
        let metrics = self.font.metrics();
        let height = self.state.as_ref().map(|live| live.height).unwrap_or(0) as f32;
        let top = (height - metrics.height).max(0.0);
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: 0.0,
            top,
            width: window_width,
            height: metrics.height,
            color: self.colors.foreground,
        });
        let background = self.colors.background;
        crate::terminal::append_text(
            &label,
            &mut self.font,
            &mut self.atlas,
            background,
            (metrics.width, top),
            quads,
        );
    }

    /// Name the window after what is running in it.
    ///
    /// A terminal called "Unterm" whatever is inside it tells the user
    /// nothing when they are looking at a taskbar full of them. The rules are
    /// the engine's, so both front ends name a shell the same way.
    fn update_window_title(&mut self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        use unterm_engine::next_core::tab_title::{render, TabContext, TabTitleRules};

        let shell = unterm_engine::SessionEngine::shell(&self.engine, live.session_id).ok();
        let process_path = shell
            .map(|shell| shell.process_name)
            .unwrap_or_default();
        let pane_title = unterm_engine::SessionEngine::get_session(&self.engine, live.session_id)
            .map(|session| session.title)
            .unwrap_or_default();

        let index = self
            .tab_id
            .and_then(|id| self.tabs.tab_ids().iter().position(|c| *c == id))
            .unwrap_or(0)
            + 1;
        let rendered = render(
            &TabTitleRules {
                // No padding: a window title is not a tab label.
                format: "{title}".to_string(),
                ..Default::default()
            },
            TabContext {
                pane_title: &pane_title,
                process_path: &process_path,
                index,
            },
        );
        let title = format!("{rendered} — Unterm");
        if self.window_title.as_deref() == Some(title.as_str()) {
            return;
        }
        live.window.set_title(&title);
        self.window_title = Some(title);
    }

    /// Tell the system where the candidate list belongs.
    ///
    /// At the caret, not in a corner: an input method that puts its
    /// candidates across the window from what is being typed makes the user
    /// look in two places to type one word.
    fn place_ime_candidates(&self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let Some((origin, metrics)) = self.preedit_origin() else {
            return;
        };
        live.window.set_ime_cursor_area(
            winit::dpi::PhysicalPosition::new(origin.0 as f64, origin.1 as f64),
            winit::dpi::PhysicalSize::new(
                (self.preedit.columns().max(1) as f32 * metrics.width) as u32,
                metrics.height as u32,
            ),
        );
    }

    /// Where composed text is drawn, and the cell size it is drawn on.
    fn preedit_origin(&self) -> Option<((f32, f32), unterm_render::quads::CellMetrics)> {
        let live = self.state.as_ref()?;
        let metrics = self.font.metrics();
        let snapshot = self.engine.read_styled_screen(live.session_id).ok()?;
        let placement = self
            .placements()
            .into_iter()
            .find(|placement| placement.session_id == live.session_id);
        let (pane_origin, pane_cols) = match placement {
            Some(placement) => (placement.origin, placement.cols),
            None => (
                (
                    0.0,
                    crate::tabbar::terminal_top(metrics, self.tabs.tab_count()),
                ),
                snapshot.cols,
            ),
        };
        let cursor = (
            snapshot.cursor.x,
            snapshot.cursor.y.max(0) as usize,
        );
        Some((
            crate::ime::origin(
                cursor,
                pane_origin,
                (metrics.width, metrics.height),
                pane_cols,
                self.preedit.columns(),
            ),
            metrics,
        ))
    }

    /// Draw what the input method is still composing.
    ///
    /// Inverted, the way every terminal marks text that is not committed yet:
    /// it is not in the shell, and drawing it like ordinary output would
    /// suggest it had already been typed.
    fn append_preedit(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        if self.preedit.is_empty() {
            return;
        }
        let Some((origin, metrics)) = self.preedit_origin() else {
            return;
        };
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: origin.0,
            top: origin.1,
            width: self.preedit.columns() as f32 * metrics.width,
            height: metrics.height,
            color: self.colors.foreground,
        });
        let text = self.preedit.text.clone();
        let background = self.colors.background;
        crate::terminal::append_text(
            &text,
            &mut self.font,
            &mut self.atlas,
            background,
            origin,
            quads,
        );
        // A caret inside the composition, so a user editing what they have
        // typed so far can see where they are.
        let caret = self.preedit.caret_column() as f32 * metrics.width;
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: origin.0 + caret,
            top: origin.1,
            width: (metrics.width * 0.15).max(1.0),
            height: metrics.height,
            color: background,
        });
    }

    /// The lines between split panes.
    ///
    /// Without them two shells sharing a background read as one shell with
    /// strange wrapping, which is exactly the confusion a split is supposed
    /// to remove.
    fn divider_quads(&self) -> Vec<unterm_render::quads::Quad> {
        let (Some(tab_id), Some(live)) = (self.tab_id, self.state.as_ref()) else {
            return Vec::new();
        };
        let metrics = self.font.metrics();
        let (cols, rows) = self
            .font
            .grid_for(live.width as f32, self.terminal_height());
        let top_offset = crate::tabbar::terminal_top(metrics, self.tabs.tab_count());
        let positions = self.tabs.positions(tab_id, cols, rows);
        if positions.len() < 2 {
            return Vec::new();
        }

        positions
            .iter()
            .filter_map(|placed| {
                let rect = &placed.rect;
                // Only where the pane does not reach the edge: an edge is
                // already a boundary, and a line drawn on it is a wasted row.
                let vertical = rect.left + rect.width < cols;
                if !vertical && rect.top + rect.height >= rows {
                    return None;
                }
                crate::panes::divider_after(rect.clone(), metrics, vertical)
            })
            .map(|(left, top, width, height)| unterm_render::quads::Quad {
                left,
                top: top + top_offset,
                width,
                height,
                // Between the two backgrounds: visible against both without
                // drawing attention to itself.
                color: mix(self.colors.background, self.colors.foreground, 0.25),
            })
            .collect()
    }

    /// Where each pane goes, in pixels.
    fn placements(&self) -> Vec<crate::panes::PanePlacement> {
        let (Some(tab_id), Some(live)) = (self.tab_id, self.state.as_ref()) else {
            return Vec::new();
        };
        let (cols, rows) = self
            .font
            .grid_for(live.width as f32, self.terminal_height());
        let top = crate::tabbar::terminal_top(self.font.metrics(), self.tabs.tab_count());
        self.tabs
            .positions(tab_id, cols, rows)
            .into_iter()
            .map(|placed| {
                let mut placement =
                    crate::panes::place(placed.pane_id, placed.rect, self.font.metrics());
                placement.origin.1 += top;
                placement
            })
            .collect()
    }

    /// Tell every pane the size it is being drawn at.
    ///
    /// A shell wrapping at a width it is not shown at is the most confusing
    /// possible symptom, so this runs on every split and every resize.
    fn resize_panes(&mut self) {
        for placement in self.placements() {
            let _ = self
                .engine
                .resize_session(placement.session_id, placement.cols, placement.rows);
        }
    }

    /// Redraw only when the screen actually moved.
    fn needs_redraw(&self) -> bool {
        let Some(live) = self.state.as_ref() else {
            return false;
        };
        // Summed across panes: any one of them moving is a reason to redraw,
        // and a sum changes whenever a term does.
        let placements = self.placements();
        let revision = if placements.is_empty() {
            self.engine.screen_revision(live.session_id).unwrap_or(0)
        } else {
            placements
                .iter()
                .filter_map(|placement| self.engine.screen_revision(placement.session_id).ok())
                .fold(0u64, |sum, value| sum.wrapping_add(value))
        };
        if Some(revision) != self.drawn_revision {
            return true;
        }
        // A banner arrives from the MCP thread, which changes no screen; if
        // only the screen could ask for a redraw, the question would never be
        // drawn and the agent would wait out its timeout looking at nothing.
        self.drawn_confirmation != unterm_mcp::handler::pending_confirmation_view().map(|v| v.id)
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
                    live.window.request_redraw();
                }
                // Every pane has to learn its new grid, or a shell keeps
                // wrapping at a width it is no longer drawn at.
                let _ = (cols, rows);
                self.resize_panes();
                self.drawn_revision = None;
            }

            WindowEvent::Ime(ime) => {
                use winit::event::Ime;
                match ime {
                    Ime::Preedit(text, caret) => {
                        self.preedit = crate::ime::Preedit {
                            text,
                            caret: caret.map(|(start, _end)| start),
                        };
                        self.place_ime_candidates();
                        if let Some(live) = self.state.as_ref() {
                            live.window.request_redraw();
                        }
                        self.drawn_revision = None;
                    }
                    Ime::Commit(text) => {
                        // Only now is it text the shell should see.
                        self.preedit = crate::ime::Preedit::default();
                        if let Some(live) = self.state.as_ref() {
                            let _ = self.engine.write_input(live.session_id, &text);
                            live.window.request_redraw();
                        }
                        self.drawn_revision = None;
                    }
                    Ime::Enabled => self.place_ime_candidates(),
                    Ime::Disabled => {
                        self.preedit = crate::ime::Preedit::default();
                        self.drawn_revision = None;
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
                self.ctrl_held = modifiers.state().control_key();
                self.alt_held = modifiers.state().alt_key();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                if self.state.is_none() {
                    return;
                }

                use winit::keyboard::Key;

                // A search takes the keyboard while it is open: the letters
                // typed are the pattern, not input for the shell.
                if self.search.is_some() && self.handle_search_key(&event) {
                    return;
                }

                let Some(live) = self.state.as_ref() else {
                    return;
                };

                // A parked agent write comes first: while the banner is up
                // these keys answer it rather than reaching the shell, and
                // everything else is ignored so a stray keystroke cannot be
                // mistaken for consent.
                if let Some(pending) = unterm_mcp::handler::pending_confirmation_view() {
                    let decision = match &event.logical_key {
                        Key::Named(winit::keyboard::NamedKey::Escape) => {
                            Some(unterm_mcp::handler::ConfirmationDecision::Block)
                        }
                        Key::Character(text) => crate::confirm::decision_for(text),
                        _ => None,
                    };
                    if let Some(decision) = decision {
                        unterm_mcp::handler::resolve_confirmation(pending.id, decision);
                        if let Some(live) = self.state.as_ref() {
                            live.window.request_redraw();
                        }
                    }
                    return;
                }

                // What the keys do lives in `keys`, so an agent asking the
                // MCP surface gets the same answer this acts on.
                if let Some(action) =
                    crate::keys::action_for(&event.logical_key, self.ctrl_held, self.shift_held)
                {
                    self.run_key_action(action, live.session_id);
                    return;
                }

                if let Some(text) = encode(&event) {
                    let _ = self.engine.write_input(self.focused_session(), &text);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = (position.x as f32, position.y as f32);
                if self.report_mouse(
                    unterm_engine::next_core::mouse_encoding::MouseEventKind::Motion,
                    self.held_mouse_button,
                ) {
                    return;
                }
                if self.drag.is_some() {
                    let point = self.cell_under_pointer();
                    if let Some(drag) = self.drag.as_mut() {
                        drag.extend(point);
                    }
                    self.update_selection();
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                use unterm_engine::next_core::mouse_encoding::MouseEventKind;
                use winit::event::MouseButton;

                // The program gets the click if it asked for one -- every
                // button, not only the left, since that is what a program
                // with a context menu is waiting for.
                let engine_button = match button {
                    MouseButton::Left => {
                        Some(unterm_engine::next_core::mouse_encoding::MouseButton::Left)
                    }
                    MouseButton::Middle => {
                        Some(unterm_engine::next_core::mouse_encoding::MouseButton::Middle)
                    }
                    MouseButton::Right => {
                        Some(unterm_engine::next_core::mouse_encoding::MouseButton::Right)
                    }
                    _ => None,
                };
                self.held_mouse_button = match state {
                    ElementState::Pressed => engine_button,
                    ElementState::Released => None,
                };
                let kind = match state {
                    ElementState::Pressed => MouseEventKind::Press,
                    ElementState::Released => MouseEventKind::Release,
                };
                if self.report_mouse(kind, engine_button) {
                    return;
                }

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
                if lines == 0 {
                    return;
                }
                // A program that asked for the mouse gets the wheel too --
                // that is how less and htop scroll themselves. One report per
                // notch, because that is what a wheel is.
                if let Some(button) = crate::mouse::wheel_button(lines as f32) {
                    let notches = lines.unsigned_abs().min(16);
                    let mut reported = false;
                    for _ in 0..notches {
                        reported = self.report_mouse(
                            unterm_engine::next_core::mouse_encoding::MouseEventKind::Press,
                            Some(button),
                        );
                        if !reported {
                            break;
                        }
                    }
                    if reported {
                        return;
                    }
                }
                if let Some(live) = self.state.as_ref() {
                    // Positive is toward older output, and the wheel rolls
                    // away from you to go back in time.
                    let _ = self.engine.scroll_viewport_by(live.session_id, -lines);
                }
            }

            WindowEvent::RedrawRequested => {
                self.draw();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.sync_tabs();
        self.update_window_title();
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
/// The shell to start, with the environment the rest of the product gives it.
///
/// A shell launched bare on a Chinese Windows writes its output in the console
/// codepage, not UTF-8, and a terminal that decodes it as UTF-8 shows a row of
/// boxes where the text should be. The same rewrite the other front end
/// applies is applied here, so both start shells that agree on encoding.
fn launch_shell(config: &config::Config) -> Option<portable_pty::CommandBuilder> {
    let mut shell = shell_from(config);
    unterm_services::launch_env::apply_unterm_windows_utf8(&mut shell);
    unterm_services::launch_env::apply_unterm_profile_env(&mut shell);
    unterm_services::launch_env::apply_unterm_proxy_env(&mut shell);
    shell
}

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

    #[test]
    fn cycling_forward_from_the_last_tab_wraps_to_the_first() {
        let ids = [10, 20, 30];
        assert_eq!(next_tab_index(&ids, Some(30), 1), 0);
        assert_eq!(next_tab_index(&ids, Some(10), 1), 1);
    }

    #[test]
    fn cycling_back_from_the_first_tab_wraps_to_the_last() {
        let ids = [10, 20, 30];
        assert_eq!(next_tab_index(&ids, Some(10), -1), 2);
        assert_eq!(next_tab_index(&ids, Some(30), -1), 1);
    }

    #[test]
    fn a_tab_that_is_no_longer_there_cycles_from_the_start() {
        // Closing a tab can leave the remembered id dangling; landing on the
        // first tab is a defined answer, and panicking is not.
        let ids = [10, 20, 30];
        assert_eq!(next_tab_index(&ids, Some(99), 1), 1);
        assert_eq!(next_tab_index(&ids, None, 1), 1);
    }

    #[test]
    fn cycling_with_no_tabs_answers_rather_than_dividing_by_zero() {
        let ids: [usize; 0] = [];
        assert_eq!(next_tab_index(&ids, None, 1), 0);
    }
}

/// Which tab a cycle lands on.
///
/// Wrapping rather than stopping at the ends: with three tabs, cycling
/// forward twice from the last has to land somewhere, and a key that does
/// nothing at the edge reads as a key that is broken.
fn next_tab_index<T: PartialEq>(ids: &[T], current: Option<T>, step: isize) -> usize {
    if ids.is_empty() {
        return 0;
    }
    let index = current
        .and_then(|id| ids.iter().position(|candidate| *candidate == id))
        .unwrap_or(0) as isize;
    (index + step).rem_euclid(ids.len() as isize) as usize
}

/// Blend two colours.
fn mix(from: [f32; 4], to: [f32; 4], amount: f32) -> [f32; 4] {
    let blend = |a: f32, b: f32| a + (b - a) * amount;
    [
        blend(from[0], to[0]),
        blend(from[1], to[1]),
        blend(from[2], to[2]),
        from[3],
    ]
}

/// The number on each tab.
///
/// Drawn separately from the bar's blocks so the active tab's number reads
/// against its highlight rather than disappearing into it.
fn append_tab_labels(
    tab_count: usize,
    active_index: usize,
    window_width: f32,
    font: &mut crate::terminal::TerminalFont,
    atlas: &mut unterm_render::atlas::GlyphAtlas,
    colors: unterm_render::quads::FrameColors,
    quads: &mut unterm_render::quads::FrameQuads,
) {
    if tab_count <= 1 {
        return;
    }
    let metrics = font.metrics();
    let width = (window_width / tab_count as f32).max(metrics.width);
    for index in 0..tab_count {
        let label = format!(" {} ", index + 1);
        let color = if index == active_index {
            colors.background
        } else {
            colors.foreground
        };
        crate::terminal::append_text(
            &label,
            font,
            atlas,
            color,
            (index as f32 * width + metrics.width, 0.0),
            quads,
        );
    }
}

/// Draw the pending agent-write banner, if one is waiting.
///
/// Over everything else and at the top, because a thread is parked on the
/// answer: a banner the user has to go looking for is a request that times out
/// into a refusal.
fn append_confirmation_banner(
    window_width: f32,
    font: &mut crate::terminal::TerminalFont,
    atlas: &mut unterm_render::atlas::GlyphAtlas,
    colors: unterm_render::quads::FrameColors,
    quads: &mut unterm_render::quads::FrameQuads,
) {
    let Some(view) = unterm_mcp::handler::pending_confirmation_view() else {
        return;
    };
    let metrics = font.metrics();
    let cols = (window_width / metrics.width.max(1.0)) as usize;
    let lines = crate::confirm::lines(&view.agent, &view.method, &view.input_preview, cols);

    // An opaque strip first, so the terminal text underneath cannot be
    // mistaken for part of the question.
    quads.backgrounds.push(unterm_render::quads::Quad {
        left: 0.0,
        top: 0.0,
        width: window_width,
        height: metrics.height * lines.len() as f32,
        color: colors.foreground,
    });
    for (row, line) in lines.iter().enumerate() {
        crate::terminal::append_text(
            line,
            font,
            atlas,
            colors.background,
            (metrics.width, row as f32 * metrics.height),
            quads,
        );
    }

}

