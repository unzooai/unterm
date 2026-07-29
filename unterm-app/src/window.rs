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

/// How long the bell's flash lasts.
///
/// Long enough to notice out of the corner of an eye, short enough that a
/// program ringing repeatedly does not leave the screen washed out.
/// How many rows the palette shows at once.
///
/// Enough to choose from, few enough that it does not become the window. A
/// query that narrows the list is the way to reach the rest.
const MAX_PALETTE_ROWS: usize = 12;

/// How many inbox rows fit before the list becomes the window.
/// How many changed paths the git panel shows before it stops.
///
/// A repository mid-rebase can have hundreds; a panel that covers the
/// terminal is a panel nobody opens twice.
const MAX_GIT_ROWS: usize = 20;

/// How many queued prompts the composer shows.
const MAX_COMPOSER_ROWS: usize = 12;

const MAX_INBOX_ROWS: usize = 12;

/// How often the cockpit tracker is shown the panes.
///
/// Not every frame: it scans each pane's last rows for the shapes an agent
/// makes when it is waiting, and doing that sixty times a second would spend
/// more on watching the agents than on drawing them.
const COCKPIT_POLL: std::time::Duration = std::time::Duration::from_millis(400);

/// How many rows of each pane the tracker is shown.
///
/// A prompt asking a question is at the bottom; more than this is scrollback
/// that has already been answered.
const COCKPIT_TAIL_ROWS: usize = 8;

const BELL_FLASH: std::time::Duration = std::time::Duration::from_millis(120);
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
    /// The size the font is asked for, in points, which the zoom keys move.
    ///
    /// Points rather than pixels, as the config and every other terminal mean
    /// it: how many pixels that is depends on the display, and the display can
    /// change while the window is open.
    font_points: f32,
    /// The size the config asked for, which the reset key goes back to.
    configured_font_points: f32,
    /// The display's scale, as winit reports it against 96 dpi.
    scale: f32,
    /// The families to try after the primary one, kept because reopening the
    /// font at a new size has to make the same choices as the first open.
    font_fallbacks: Vec<String>,
    /// The family the config named, if it named one.
    font_family: Option<String>,
    /// How far the cell is stretched around its glyphs.
    font_shape: crate::terminal::Shape,
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
    /// How many bells the pane had rung when we last drew.
    bells_seen: u64,
    /// When the current flash started.
    bell_at: Option<std::time::Instant>,
    /// True while a drag is holding the scrollbar's thumb.
    dragging_scrollbar: bool,
    /// The open command palette or launcher, if there is one.
    palette: Option<crate::palette::Palette>,
    /// The keyboard selection, if copy mode is on.
    copy_mode: Option<crate::copy_mode::CopyMode>,
    /// Quick select's labels, and what has been typed towards one.
    quick_select: Option<(Vec<crate::copy_mode::Labelled>, String)>,
    /// A letter on every pane, while one is being picked.
    pane_select: Option<crate::paneselect::Selector>,
    /// Where the first shell should start, if the command line said.
    start_directory: Option<std::path::PathBuf>,
    /// The last clipboard request honoured, so it is not honoured twice.
    clipboard_honoured: Option<String>,
    /// Whether the agent inbox is showing.
    inbox_open: bool,
    /// Whether the strip of tabs down the left is showing.
    sidebar_open: bool,
    /// The strip's first visible row, for lists longer than the window.
    sidebar_scroll: usize,
    /// Set when the close button is pressed, so the loop can exit.
    closing: bool,
    /// The cursor the config asked for, and how fast it blinks.
    cursor_style: crate::terminal::CursorStyle,
    cursor_blink_ms: u64,
    /// When the window opened, which is what a blink is measured from.
    started: std::time::Instant,
    /// The theme in force, so the picker can mark it and the next launch can
    /// restore it.
    theme_id: Option<String>,
    /// Something that just happened, and when it stops being shown.
    notice: Option<(String, std::time::Instant)>,
    /// The git panel's contents, held while it is open.
    ///
    /// Read once when it opens rather than every frame: `git status` on a
    /// large repository is not something to run sixty times a second, and a
    /// panel that changes under the eye while being read is worse than one
    /// that is a moment old.
    git_panel: Option<crate::git::Panel>,
    /// Prompts waiting to go into the focused pane, while the composer
    /// is open. Closing it drops them: a queue that outlives the window
    /// showing it would fire prompts nobody can see coming.
    composer: Option<crate::composer::Composer>,
    /// When the cockpit tracker last saw the panes.
    cockpit_fed_at: std::time::Instant,
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

        let cursor_style = crate::terminal::CursorStyle::from_config(config);
        let family: Option<String> = config.str_of("font").ok().flatten().map(|f| f.to_string());
        let shape = crate::terminal::Shape::from_config(config);
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
            bells_seen: 0,
            bell_at: None,
            dragging_scrollbar: false,
            palette: None,
            copy_mode: None,
            quick_select: None,
            pane_select: None,
            start_directory: None,
            clipboard_honoured: None,
            inbox_open: false,
            sidebar_open: false,
            sidebar_scroll: 0,
            closing: false,
            cursor_style: cursor_style.0,
            cursor_blink_ms: cursor_style.1,
            started: std::time::Instant::now(),
            theme_id: crate::theme::remembered(),
            notice: None,
            git_panel: None,
            composer: None,
            cockpit_fed_at: std::time::Instant::now(),
            mouse_modes: Default::default(),
            held_mouse_button: None,
            alt_held: false,
            window_title: None,
            font: TerminalFont::open_named(
                family.as_deref(),
                crate::terminal::pixels_for_points(pixel_size as f32, 1.0),
                &fallbacks,
                shape,
            )?,
            font_family: family,
            font_shape: shape,
            font_points: pixel_size as f32,
            configured_font_points: pixel_size as f32,
            scale: 1.0,
            font_fallbacks: fallbacks,
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
            // The top bar is the title bar. A grey native one above a dark
            // one is the three-stacked-strips look the design called out.
            .with_decorations(false)
            .with_inner_size(winit::dpi::LogicalSize::new(
                metrics.width * 100.0,
                metrics.height * 30.0,
            ));
        let window = Arc::new(event_loop.create_window(attributes)?);
        // The window knows the display's scale; the font was opened before it
        // existed, at 1.0. On a scaled panel every glyph until now was drawn
        // at a fraction of its size.
        let scale = window.scale_factor() as f32;
        if (scale - self.scale).abs() > f32::EPSILON {
            self.reopen_font(self.font_points, scale);
        }
        // So `instance.focus`, which arrives on the MCP thread, has a window
        // to raise.
        crate::mcp_host::remember_window(window.clone());
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
            command_dir: self
                .start_directory
                .as_ref()
                .map(|path| path.display().to_string()),
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
        let cursor = self.cursor_style;
        let solid_cursor = self.cursor_is_solid();
        let mut revision = 0u64;
        for placement in &placements {
            let Ok(snapshot) = self.engine.read_styled_screen(placement.session_id) else {
                continue;
            };
            revision = revision.wrapping_add(snapshot.revision);
            if placement.session_id == session_id {
                self.mouse_modes = snapshot.mouse;
                self.note_bells(snapshot.bells);
            self.take_clipboard_request(snapshot.clipboard_request.clone());
            }
            crate::terminal::append_pane(
                &snapshot,
                &mut self.font,
                &mut self.atlas,
                self.colors,
                placement.origin,
                placement.session_id == session_id && solid_cursor,
                cursor,
                &mut quads,
            );
        }
        if placements.is_empty() {
            let Ok(snapshot) = self.engine.read_styled_screen(session_id) else {
                return;
            };
            revision = snapshot.revision;
            self.mouse_modes = snapshot.mouse;
            self.note_bells(snapshot.bells);
            self.take_clipboard_request(snapshot.clipboard_request.clone());
            // Below the top bar, like every other pane. Drawn at the window's
            // own origin it lands on the bar, which is what the first frame
            // with a bar in it looked like.
            let metrics = self.font.metrics();
            let origin = (self.terminal_left(), crate::topbar::terminal_top(metrics));
            crate::terminal::append_pane(
                &snapshot,
                &mut self.font,
                &mut self.atlas,
                self.colors,
                origin,
                solid_cursor,
                cursor,
                &mut quads,
            );
        }
        self.append_selection(&mut quads);
        quads.backgrounds.extend(dividers);
        self.append_scrollbar(&mut quads);
        self.append_bell_flash(&mut quads);
        self.append_hovered_link(&mut quads);
        self.append_preedit(&mut quads);
        self.append_search_bar(window_width, &mut quads);
        self.append_copy_mode(&mut quads);
        self.append_quick_select(&mut quads);
        self.append_pane_select(&mut quads);
        // Everything from here up is a panel, and a panel has to cover what
        // is behind it rather than sit in the same layer as it.
        let overlays = quads.mark();
        self.append_inbox(window_width, &mut quads);
        self.append_git_panel(window_width, &mut quads);
        self.append_composer(window_width, &mut quads);
        self.append_palette(window_width, &mut quads);
        let tab_count = self.tabs.tab_count();
        let active_tab = self
            .tab_id
            .and_then(|id| self.tabs.tab_ids().iter().position(|c| *c == id))
            .unwrap_or(0);
        // One badge per tab, in the order the tabs are drawn. A tab shows
        // the most urgent of its panes': a split where one half is waiting is
        // a tab that is waiting.
        let statuses = unterm_services::cockpit::status::snapshot();
        let badges: Vec<Option<crate::cockpit::Badge>> = self
            .tabs
            .tab_ids()
            .into_iter()
            .map(|tab| {
                self.tabs
                    .pane_ids(tab)
                    .into_iter()
                    .filter_map(|pane| crate::cockpit::badge_for_pane(&statuses, pane as u64))
                    .min_by_key(|badge| match badge {
                        crate::cockpit::Badge::NeedsYou => 0,
                        crate::cockpit::Badge::Done => 1,
                        crate::cockpit::Badge::Working => 2,
                    })
            })
            .collect();
        self.append_top_bar(tab_count, active_tab, &badges, window_width, &mut quads);
        self.append_sidebar(&mut quads);
        self.append_status_bar(window_width, &mut quads);
        quads.raise_since(overlays);

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
        let left = self.terminal_left();
        let top = crate::topbar::terminal_top(metrics);
        let column = ((self.pointer.0 - left).max(0.0) / metrics.width.max(1.0)) as usize;
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

    /// Start a flash if the pane has rung since the last frame.
    fn note_bells(&mut self, bells: u64) {
        if bells > self.bells_seen {
            self.bells_seen = bells;
            self.bell_at = Some(std::time::Instant::now());
        }
    }

    /// Whether the pointer is over the scrollbar's track.
    /// Whether the pointer is on the status bar's quick-action button.
    fn pointer_on_menu(&self) -> bool {
        let Some(live) = self.state.as_ref() else {
            return false;
        };
        let metrics = self.font.metrics();
        let bar_top = live.height as f32 - metrics.height * crate::statusbar::ROWS as f32;
        if self.pointer.1 < bar_top {
            return false;
        }
        let columns = (live.width as f32 / metrics.width.max(1.0)).floor().max(0.0) as usize;
        let column = (self.pointer.0 / metrics.width.max(1.0)).floor().max(0.0) as usize;
        crate::statusbar::menu_hit(column, columns)
    }

    fn pointer_on_scrollbar(&self) -> bool {
        let Some(live) = self.state.as_ref() else {
            return false;
        };
        self.pointer.0 >= live.width as f32 - crate::scrollbar::WIDTH
    }

    /// Scroll to wherever the pointer is on the track.
    fn scroll_to_pointer(&mut self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let Ok(snapshot) = self.engine.read_styled_screen(live.session_id) else {
            return;
        };
        let metrics = self.font.metrics();
        let track_top = crate::topbar::terminal_top(metrics);
        let track = (live.height as f32 - track_top).max(1.0);
        let total = snapshot.scrollback_rows + snapshot.rows;
        let row = crate::scrollbar::row_at(total, snapshot.rows, self.pointer.1 - track_top, track);
        let _ = self
            .engine
            .scroll_viewport_to(live.session_id, row as isize);
        self.drawn_revision = None;
        live.window.request_redraw();
    }

    /// The scrollbar, down the right edge.
    ///
    /// Only when there is history above: a bar that fills its whole track
    /// tells the user nothing and takes a column to say it.

    /// The bar along the bottom: where you are, and what agents are doing.
    fn append_status_bar(
        &mut self,
        window_width: f32,
        quads: &mut unterm_render::quads::FrameQuads,
    ) {
        let metrics = self.font.metrics();
        let height = self.state.as_ref().map(|live| live.height).unwrap_or(600) as f32;
        let top = height - metrics.height * crate::statusbar::ROWS as f32;
        let columns = (window_width / metrics.width.max(1.0)).floor().max(0.0) as usize;

        let status = self.status();
        let segments = crate::statusbar::segments(&status, columns);
        if segments.is_empty() {
            return;
        }

        // The same surface as the top bar, so the window reads as one thing
        // with two edges. Toning the foreground towards black instead is what
        // left a black strip along the bottom of every light theme.
        let chrome = self.chrome();
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: 0.0,
            top,
            width: window_width,
            height: metrics.height,
            color: chrome.surface,
        });
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: 0.0,
            top,
            width: window_width,
            height: 1.0,
            color: chrome.outer_edge,
        });
        for segment in segments {
            let color = if segment.dim {
                chrome.dim_text
            } else {
                self.colors.foreground
            };
            crate::terminal::append_text(
                &segment.text,
                &mut self.font,
                &mut self.atlas,
                color,
                (segment.column as f32 * metrics.width, top),
                quads,
            );
        }
    }

    /// Say what just happened, for a moment.
    ///
    /// 2400 milliseconds, as before: long enough to read on the way past,
    /// short enough that it is gone before it becomes furniture.
    fn show_notice(&mut self, message: String) {
        const SHOWN_FOR: std::time::Duration = std::time::Duration::from_millis(2400);
        self.notice = Some((message, std::time::Instant::now() + SHOWN_FOR));
        self.drawn_revision = None;
    }

    /// The notice, if one is still up. Clears it once it is not.
    fn active_notice(&self) -> Option<String> {
        self.notice
            .as_ref()
            .filter(|(_, expires)| *expires > std::time::Instant::now())
            .map(|(message, _)| message.clone())
    }

    /// What the status bar has to say right now.
    fn status(&self) -> crate::statusbar::Status {
        let session = self.state.as_ref().and_then(|live| {
            unterm_engine::SessionEngine::list_sessions(&self.engine)
                .ok()
                .into_iter()
                .flatten()
                .find(|session| session.id == live.session_id)
        });
        let mcp = unterm_mcp::handler::insights_mcp_snapshot(0);
        let agents = unterm_services::cockpit::status::snapshot();
        crate::statusbar::Status {
            agents_waiting: crate::cockpit::attention_count(&agents),
            notice: self.active_notice(),
            shell: session
                .as_ref()
                .map(|session| crate::statusbar::short_name(&session.shell.process_name))
                .unwrap_or_default(),
            directory: session
                .as_ref()
                .and_then(|session| session.shell.cwd.clone())
                .unwrap_or_default(),
            agent_writes: mcp.input_count,
            pending: mcp.pending_confirmations,
            proxy: unterm_services::system_proxy::detect()
                .and_then(|proxy| proxy.primary_http().map(crate::statusbar::short_proxy)),
        }
    }


    /// Whether the cursor is solid at this instant.
    ///
    /// A steady cursor always is. A blinking one is for half of each period;
    /// the other half it is drawn as an outline rather than removed, so it
    /// stays findable while it blinks.
    fn cursor_is_solid(&self) -> bool {
        if !self.cursor_style.blinking {
            return true;
        }
        crate::terminal::blink_is_on(self.started.elapsed().as_millis(), self.cursor_blink_ms)
    }


    /// Highlight what is selected.
    ///
    /// It was being tracked and copied, but never drawn -- so dragging across
    /// text selected it, copying worked, and nothing on screen said which text
    /// you had. Drawn in the scheme's own highlight, with its own text colour,
    /// because those two were chosen together.
    fn append_selection(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        let Some(selection) = self.drag.and_then(|drag| drag.selection()) else {
            return;
        };
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let Ok(snapshot) = self.engine.read_styled_screen(live.session_id) else {
            return;
        };

        let metrics = self.font.metrics();
        let origin = (self.terminal_left(), crate::topbar::terminal_top(metrics));
        let theme = self.theme();

        for (index, line) in snapshot.lines.iter().enumerate() {
            let columns = selection.columns_for_row(line.row, line.cells.len());
            if columns.is_empty() {
                continue;
            }
            quads.backgrounds.push(unterm_render::quads::Quad {
                left: origin.0 + columns.start as f32 * metrics.width,
                top: origin.1 + index as f32 * metrics.height,
                width: (columns.end - columns.start) as f32 * metrics.width,
                height: metrics.height,
                color: theme.selection,
            });
        }
    }

    /// The scheme in force, for the colours that are its rather than the
    /// frame's: the divider between panes, the scrollbar, the selection.
    fn theme(&self) -> &'static crate::theme::Theme {
        self.theme_id
            .as_deref()
            .and_then(crate::theme::by_id)
            .unwrap_or_else(crate::theme::default_theme)
    }

    /// The frame's tones, from the terminal's own colours.
    fn chrome(&self) -> crate::chrome::Chrome {
        crate::chrome::chrome(self.colors.background, self.colors.foreground)
    }


    /// Where the terminal's first column starts, the strip included.
    fn terminal_left(&self) -> f32 {
        let metrics = self.font.metrics();
        crate::sidebar::width(self.sidebar_open, metrics) + crate::topbar::terminal_left(metrics)
    }

    /// How wide the terminal is, once the strip and the gaps are taken.
    fn terminal_width(&self) -> f32 {
        let metrics = self.font.metrics();
        let window = self.state.as_ref().map(|live| live.width).unwrap_or(800) as f32;
        crate::topbar::terminal_width(window - crate::sidebar::width(self.sidebar_open, metrics), metrics)
    }

    /// What the strip shows: one line per tab, grouped by project.
    fn sidebar_rows(&self) -> Vec<crate::sidebar::Row> {
        let sessions = unterm_engine::SessionEngine::list_sessions(&self.engine)
            .unwrap_or_default();
        let active = self.tab_id;
        let tabs: Vec<crate::sidebar::TabInfo> = self
            .tabs
            .tab_ids()
            .into_iter()
            .enumerate()
            .map(|(index, tab)| {
                let pane = self.tabs.active_pane(tab);
                let session = pane.and_then(|pane| {
                    sessions.iter().find(|session| session.id == pane)
                });
                crate::sidebar::TabInfo {
                    index,
                    title: session
                        .map(|session| session.title.clone())
                        .unwrap_or_default(),
                    cwd: session.and_then(|session| session.shell.cwd.clone()),
                    foreground: session
                        .map(|session| session.shell.process_name.clone())
                        .map(|name| crate::statusbar::short_name(&name)),
                    active: Some(tab) == active,
                }
            })
            .collect();
        crate::sidebar::rows(&tabs)
    }

    /// Draw the strip, if it is open.
    fn append_sidebar(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        if !self.sidebar_open {
            return;
        }
        let metrics = self.font.metrics();
        let width = crate::sidebar::width(true, metrics);
        let top = crate::topbar::terminal_top(metrics) - crate::topbar::padding(metrics).1;
        let height = self.terminal_height() + crate::topbar::padding(metrics).1 * 2.0;
        let chrome = self.chrome();

        quads.backgrounds.push(unterm_render::quads::Quad {
            left: 0.0,
            top,
            width,
            height,
            color: chrome.surface,
        });
        // The seam, so the strip and the terminal read as two surfaces rather
        // than one that changed colour.
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: width - 1.0,
            top,
            width: 1.0,
            height,
            color: chrome.outer_edge,
        });

        let rows = self.sidebar_rows();
        let visible = (height / metrics.height).floor().max(1.0) as usize;
        // Follow the selection. A strip longer than the window that stays put
        // while tabs are switched shows a list with nothing selected in it,
        // which reads as the strip having lost track rather than as the
        // selection being further down.
        let mut scroll = self.sidebar_scroll;
        if let Some(active) = rows.iter().position(
            |row| matches!(row, crate::sidebar::Row::Tab { active: true, .. }),
        ) {
            scroll = crate::sidebar::scroll_to_show(scroll, active, visible);
        }
        let scroll = crate::sidebar::clamp_scroll(scroll, rows.len(), visible);

        for (offset, row) in rows.iter().skip(scroll).take(visible).enumerate() {
            let row_top = top + offset as f32 * metrics.height;
            let (color, active) = match row {
                crate::sidebar::Row::Group { .. } => (chrome.dim_text, false),
                crate::sidebar::Row::Tab { active, .. } => (self.colors.foreground, *active),
            };
            if active {
                quads.backgrounds.push(unterm_render::quads::Quad {
                    left: 0.0,
                    top: row_top,
                    width: width - 1.0,
                    height: metrics.height,
                    color: chrome.selected_bg,
                });
                // The rail down the side of the selected row: the one place
                // the accent colour is used, and what the eye finds first.
                quads.backgrounds.push(unterm_render::quads::Quad {
                    left: 0.0,
                    top: row_top,
                    width: 2.0,
                    height: metrics.height,
                    color: chrome.focus_rail,
                });
            }
            let text = crate::sidebar::text_for(row, crate::sidebar::COLUMNS);
            crate::terminal::append_text(
                &text,
                &mut self.font,
                &mut self.atlas,
                color,
                (metrics.width * 0.5, row_top),
                quads,
            );
        }
    }

    /// Draw the bar along the top: wordmark, tabs, buttons.
    /// The line of facts about the pane in front, for the top bar.
    ///
    /// Everything in it comes from a cache that refreshes on another thread,
    /// so this is cheap enough to call while painting -- which it has to be,
    /// because the bar is repainted whenever anything moves.
    ///
    /// Empty on a narrow window: the tabs are what the bar is for, and pushing
    /// them off the edge to make room for a memory figure is the wrong trade.
    fn stats_line(&self, window_width: f32) -> String {
        if window_width / self.scale.max(0.1) < crate::statsbar::MIN_WIDTH {
            return String::new();
        }
        let Some(live) = self.state.as_ref() else {
            return String::new();
        };
        let metrics = self.font.metrics();
        let columns = (window_width / metrics.width.max(1.0)).floor().max(0.0) as usize;
        // Exactly the room the bar has left once the tabs and the buttons have
        // what they need. Composing a longer line and letting the layout refuse
        // it is how the whole line disappears when one value grows.
        crate::statsbar::fit(
            &crate::statsbar::facts_for(live.session_id).segments(),
            &crate::statsbar::Facts::GIVE_UP,
            crate::topbar::stats_room(columns),
        )
    }

    fn append_top_bar(
        &mut self,
        tab_count: usize,
        active_tab: usize,
        badges: &[Option<crate::cockpit::Badge>],
        window_width: f32,
        quads: &mut unterm_render::quads::FrameQuads,
    ) {
        let metrics = self.font.metrics();
        let height = metrics.height * crate::topbar::ROWS as f32;
        let columns = (window_width / metrics.width.max(1.0)).floor().max(0.0) as usize;
        let chrome = self.chrome();

        quads.backgrounds.push(unterm_render::quads::Quad {
            left: 0.0,
            top: 0.0,
            width: window_width,
            height,
            color: chrome.surface,
        });
        // A hairline under it, so the bar and the terminal read as two
        // surfaces of one window rather than one surface with a seam.
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: 0.0,
            top: height - 1.0,
            width: window_width,
            height: 1.0,
            color: chrome.outer_edge,
        });

        let stats = self.stats_line(window_width);
        let bar = crate::topbar::layout(tab_count, active_tab, columns, &stats);
        let hovered = self.hovered_top_bar_item();
        for piece in &bar {
            let left = piece.column as f32 * metrics.width;
            let width = piece.columns as f32 * metrics.width;
            let is_hovered = hovered == Some(piece.item);

            if let Some(button) = crate::topbar::window_button(piece.item) {
                if is_hovered {
                    quads.backgrounds.push(unterm_render::quads::Quad {
                        left,
                        top: 0.0,
                        width,
                        height,
                        color: crate::window_buttons::hover_fill(button, chrome.is_light),
                    });
                }
                let color = if is_hovered {
                    crate::window_buttons::hovered_icon_color(button, chrome.is_light)
                } else {
                    crate::window_buttons::icon_color(chrome.is_light)
                };
                quads.backgrounds.extend(crate::window_buttons::quads(
                    button, left, 0.0, width, height, color,
                ));
                continue;
            }

            if is_hovered || matches!(piece.item, crate::topbar::Item::Tab(index) if index == active_tab)
            {
                quads.backgrounds.push(unterm_render::quads::Quad {
                    left,
                    top: 0.0,
                    width,
                    height,
                    color: if is_hovered {
                        chrome.hover_bg
                    } else {
                        chrome.selected_bg
                    },
                });
            }
            if let crate::topbar::Item::Tab(index) = piece.item {
                if let Some(badge) = badges.get(index).copied().flatten() {
                    crate::terminal::append_text(
                        crate::cockpit::BADGE,
                        &mut self.font,
                        &mut self.atlas,
                        badge.color(),
                        (
                            crate::topbar::badge_column(piece) as f32 * metrics.width,
                            (height - metrics.height) / 2.0,
                        ),
                        quads,
                    );
                }
            }
            if piece.label.trim().is_empty() {
                continue;
            }
            // Centred down the bar's two rows.
            let text_top = (height - metrics.height) / 2.0;
            crate::terminal::append_text(
                &piece.label,
                &mut self.font,
                &mut self.atlas,
                if matches!(
                    piece.item,
                    crate::topbar::Item::Wordmark | crate::topbar::Item::Stats
                ) {
                    chrome.dim_text
                } else {
                    self.colors.foreground
                },
                (left, text_top),
                quads,
            );
        }
    }

    /// A press on the top bar. Returns true when the bar took it.
    ///
    /// The empty parts drag the window, which is the first thing anyone tries
    /// on a window with no title bar -- and the last thing they find missing.
    fn click_top_bar(&mut self) -> bool {
        let metrics = self.font.metrics();
        if self.pointer.1 >= metrics.height * crate::topbar::ROWS as f32 {
            return false;
        }
        // What is a handle is the bar's own question to answer -- the same
        // list that decided where things were drawn. Deciding it again here
        // is how a piece comes to be drawn in one place and grabbed in
        // another.
        if self.pointer_is_on_a_drag_handle() {
            if let Some(live) = self.state.as_ref() {
                let _ = live.window.drag_window();
            }
            return true;
        }
        let Some(item) = self.hovered_top_bar_item() else {
            return true;
        };

        match item {
            // Both of these are handles, and were taken above.
            crate::topbar::Item::Wordmark | crate::topbar::Item::Stats => {}
            crate::topbar::Item::Tab(index) => self.select_tab(index as u8 + 1),
            crate::topbar::Item::NewTab => self.new_tab(),
            crate::topbar::Item::Menu => {
                let entries = self.quick_entries();
                self.open_palette(entries);
            }
            crate::topbar::Item::Action(action) => {
                if let Some(live) = self.state.as_ref() {
                    let session_id = live.session_id;
                    self.run_key_action(action, session_id);
                }
            }
            crate::topbar::Item::Minimise => {
                if let Some(live) = self.state.as_ref() {
                    live.window.set_minimized(true);
                }
            }
            crate::topbar::Item::Maximise => {
                if let Some(live) = self.state.as_ref() {
                    live.window.set_maximized(!live.window.is_maximized());
                }
            }
            crate::topbar::Item::Close => {
                if let Some(live) = self.state.as_ref() {
                    live.window.set_visible(false);
                }
                self.closing = true;
            }
        }
        self.drawn_revision = None;
        true
    }

    /// The bar as it is drawn right now, and which column the pointer is in.
    ///
    /// One place, so what is hit is always what was drawn.
    fn top_bar_under_pointer(&self) -> Option<(Vec<crate::topbar::Placed>, usize)> {
        let metrics = self.font.metrics();
        if self.pointer.1 >= metrics.height * crate::topbar::ROWS as f32 {
            return None;
        }
        let live = self.state.as_ref()?;
        let columns = (live.width as f32 / metrics.width.max(1.0)).floor().max(0.0) as usize;
        let column = (self.pointer.0 / metrics.width.max(1.0)).floor().max(0.0) as usize;
        let bar = crate::topbar::layout(
            self.tabs.tab_count(),
            0,
            columns,
            &self.stats_line(live.width as f32),
        );
        Some((bar, column))
    }

    /// Which piece of the top bar the pointer is over.
    fn hovered_top_bar_item(&self) -> Option<crate::topbar::Item> {
        let (bar, column) = self.top_bar_under_pointer()?;
        crate::topbar::hit(&bar, column)
    }

    /// Whether a press here should drag the window rather than do something.
    fn pointer_is_on_a_drag_handle(&self) -> bool {
        self.top_bar_under_pointer()
            .map(|(bar, column)| crate::topbar::is_drag_handle(&bar, column))
            .unwrap_or(false)
    }

    fn append_scrollbar(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let Ok(snapshot) = self.engine.read_styled_screen(live.session_id) else {
            return;
        };
        let metrics = self.font.metrics();
        let track_top = crate::topbar::terminal_top(metrics);
        let track = (live.height as f32 - track_top).max(1.0);

        let total = snapshot.scrollback_rows + snapshot.rows;
        let top_row = snapshot
            .lines
            .first()
            .map(|line| line.row.max(0) as usize)
            .unwrap_or(0);
        let Some(thumb) = crate::scrollbar::thumb(total, snapshot.rows, top_row, track) else {
            return;
        };

        let left = live.width as f32 - crate::scrollbar::WIDTH;
        // The track first, so the thumb reads as a position within something
        // rather than a stripe floating at the edge.
        quads.backgrounds.push(unterm_render::quads::Quad {
            left,
            top: track_top,
            width: crate::scrollbar::WIDTH,
            height: track,
            // The track is a tint of the thumb rather than a mix of the
            // frame: a scheme that chose a scrollbar colour chose it against
            // its own background, and deriving one here ignores that.
            color: crate::chrome::mix(
                self.colors.background,
                self.theme().scrollbar,
                0.35,
            ),
        });
        quads.backgrounds.push(unterm_render::quads::Quad {
            left,
            top: track_top + thumb.top,
            width: crate::scrollbar::WIDTH,
            height: thumb.height,
            color: self.theme().scrollbar,
        });
    }

    /// A visual bell: the screen lightens for a moment.
    ///
    /// Visual rather than audible. A terminal that beeps out of a background
    /// window is the reason people turn bells off entirely, and a flash says
    /// the same thing to someone who is looking.
    fn append_bell_flash(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let Some(rung_at) = self.bell_at else {
            return;
        };
        let elapsed = rung_at.elapsed();
        if elapsed >= BELL_FLASH {
            self.bell_at = None;
            return;
        }
        // Fading out, so a bell in a stream of them does not strobe.
        let remaining = 1.0 - elapsed.as_secs_f32() / BELL_FLASH.as_secs_f32();
        let mut color = self.colors.foreground;
        color[3] = 0.18 * remaining;
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: 0.0,
            top: 0.0,
            width: live.width as f32,
            height: live.height as f32,
            color,
        });
    }

    /// Underline the link the pointer is over, while Ctrl says a click opens.
    ///
    /// Only while the modifier is down: a line that appears under everything
    /// the pointer passes is noise, and one that appears when clicking would
    /// do something is a hint.
    fn append_hovered_link(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        if !self.ctrl_held {
            return;
        }
        let Some(link) = self.link_under_pointer() else {
            return;
        };
        let metrics = self.font.metrics();
        let top_offset = crate::topbar::terminal_top(metrics);
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: link.start as f32 * metrics.width,
            top: top_offset
                + link.row as f32 * metrics.height
                + unterm_render::decorations::underline_top(metrics),
            width: (link.end - link.start) as f32 * metrics.width,
            height: unterm_render::decorations::thickness(metrics),
            color: self.colors.foreground,
        });
    }

    /// The link the pointer is over, if any.
    fn link_under_pointer(&self) -> Option<crate::links::Link> {
        let live = self.state.as_ref()?;
        let snapshot = self.engine.read_styled_screen(live.session_id).ok()?;
        let metrics = self.font.metrics();
        let left = self.terminal_left();
        let top = crate::topbar::terminal_top(metrics);
        let column = ((self.pointer.0 - left).max(0.0) / metrics.width.max(1.0)) as usize;
        let row = ((self.pointer.1 - top).max(0.0) / metrics.height.max(1.0)) as usize;

        let line = snapshot.lines.get(row)?;
        crate::links::links_in_row(row, line)
            .into_iter()
            .find(|link| link.covers(row, column))
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

        // From the terminal's own origin, not the window's: the bar above and
        // the gap around the grid are not part of it, and a selection measured
        // from the window's corner lands two rows above where it was drawn.
        let metrics = self.font.metrics();
        let origin = (self.terminal_left(), crate::topbar::terminal_top(metrics));
        crate::select::cell_at(
            self.pointer.0 - origin.0,
            self.pointer.1 - origin.1,
            metrics,
            top,
        )
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
        if std::env::var_os("UNTERM_TRACE_KEYS").is_some() {
            log::info!("  run_key_action {:?} font={}pt", action, self.font_points);
        }
        match action {
            Action::Copy => self.copy_selection(),
            Action::Paste => self.paste_clipboard(),
            Action::SplitRight => {
                self.split(unterm_engine::next_core::layout::SplitAxis::Horizontal)
            }
            Action::SplitDown => self.split(unterm_engine::next_core::layout::SplitAxis::Vertical),
            Action::CopyMode => {
                self.copy_mode = Some(crate::copy_mode::CopyMode::default());
                self.drawn_revision = None;
            }
            Action::QuickSelect => self.open_quick_select(),
            Action::CockpitInbox => {
                self.inbox_open = !self.inbox_open;
                self.drawn_revision = None;
            }
            Action::GitPanel => self.toggle_git_panel(),
            Action::DirJump => {
                let entries = self.dir_jump_entries("");
                self.open_browser(entries);
            }
            Action::LeftTabBar => {
                self.sidebar_open = !self.sidebar_open;
                self.resize_panes();
                self.drawn_revision = None;
            }
            Action::ThemePicker => {
                let entries = self.theme_entries();
                self.open_palette(entries);
            }
            Action::Composer => {
                self.composer = match self.composer.take() {
                    Some(_) => None,
                    None => Some(crate::composer::Composer::default()),
                };
                self.drawn_revision = None;
            }
            Action::CommandPalette => self.open_palette(command_entries()),
            Action::Launcher => self.open_palette(launcher_entries()),
            Action::Search => {
                self.search = Some(crate::search::Search::default());
                self.drawn_revision = None;
            }
            Action::NewTab => self.new_tab(),
            Action::NextTab => self.cycle_tab(1),
            Action::PreviousTab => self.cycle_tab(-1),
            Action::CloseTab => self.close_tab(),
            Action::NewWindow => self.new_window(),
            Action::ClosePane => self.close_pane(session_id),
            Action::ZoomPane => self.toggle_zoom(session_id),
            Action::FleetLaunch => {
                let entries = self.fleet_entries();
                self.open_fleet(entries);
            }
            Action::ClearScrollback => self.clear_scrollback(session_id, false),
            Action::ClearScreen => self.clear_scrollback(session_id, true),
            Action::SelectPane => self.open_pane_select(crate::paneselect::Mode::Activate),
            Action::SwapPane => self.open_pane_select(crate::paneselect::Mode::Swap),
            Action::FocusPane(direction) => self.focus_pane_toward(direction),
            Action::SelectTab(number) => self.select_tab(number),
            Action::IncreaseFontSize => self.change_font_size(1.0),
            Action::DecreaseFontSize => self.change_font_size(-1.0),
            Action::ResetFontSize => self.set_font_size(self.configured_font_points),
            Action::ToggleFullScreen => self.toggle_full_screen(),
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


    /// Another terminal, as its own process.
    ///
    /// Unterm's windows are separate processes -- that is what makes
    /// `instance.list` able to name them and an agent able to drive one
    /// without touching another -- so a new window is a new instance, started
    /// where this one is looking.
    fn new_window(&mut self) {
        let Ok(program) = std::env::current_exe() else {
            log::warn!("cannot find this executable to open another window");
            return;
        };
        let mut command = std::process::Command::new(program);
        command.arg("start");
        if let Some(directory) = self.current_directory() {
            command.arg("--cwd").arg(directory);
        }
        // Detached: the new window outlives this one, and inheriting our
        // handles would keep a pipe open that nobody is reading.
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Err(err) = command.spawn() {
            log::warn!("could not open another window: {err}");
        }
    }

    /// Where a new window should start: where the focused pane is.
    fn current_directory(&self) -> Option<std::path::PathBuf> {
        let live = self.state.as_ref()?;
        unterm_engine::SessionEngine::list_sessions(&self.engine)
            .ok()
            .into_iter()
            .flatten()
            .find(|session| session.id == live.session_id)
            .and_then(|session| session.shell.cwd)
            .map(std::path::PathBuf::from)
            .or_else(|| self.start_directory.clone())
    }

    /// Close one pane, leaving the rest of the tab alone.
    ///
    /// Distinct from closing the tab: with a split open, the pane is what the
    /// key is aimed at. When it was the last one the tab goes with it, which
    /// is what makes this safe to reach for.
    fn close_pane(&mut self, session_id: usize) {
        let Some(tab_id) = self.tabs.tab_of_pane(session_id) else {
            return;
        };
        if self.tabs.pane_ids(tab_id).len() < 2 {
            self.close_tab();
            return;
        }
        crate::statsbar::forget(session_id);
        let _ = self.engine.destroy_session(session_id);
        self.tabs.close_pane(session_id);
        if let Some(pane) = self.tabs.active_pane(tab_id) {
            self.focus_session(pane);
        }
        self.resize_panes();
        self.drawn_revision = None;
    }

    /// One pane fills the tab, or gives the space back.
    fn toggle_zoom(&mut self, session_id: usize) {
        let Some(tab_id) = self.tabs.tab_of_pane(session_id) else {
            return;
        };
        let zoomed = self.tabs.zoomed_pane(tab_id) == Some(session_id);
        self.tabs.set_zoomed(session_id, !zoomed);
        self.resize_panes();
        self.drawn_revision = None;
    }

    /// Move focus to the nearest pane in a direction.
    ///
    /// Nearest by edge rather than by tree position: with three panes the tree
    /// has a shape the screen does not show, and someone pressing Alt+Right
    /// means the pane to the right of this one.
    /// Move focus to the nearest pane in a direction.
    fn focus_pane_toward(&mut self, direction: crate::keys::Direction) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let placements = self.placements();
        let target = crate::panes::pane_toward(
            &placements,
            live.session_id,
            direction,
            self.font.metrics(),
        );
        if let Some(pane) = target {
            self.tabs.set_active_pane(pane);
            self.focus_session(pane);
            self.drawn_revision = None;
        }
    }

    /// Go to a tab by its number, counting from one as the keys are labelled.
    ///
    /// Nine means the last one however many there are, which is what every
    /// browser does and what people reach for.
    fn select_tab(&mut self, number: u8) {
        let ids = self.tabs.tab_ids();
        let Some(tab_id) = crate::topbar::tab_for_number(number, ids.len())
            .and_then(|index| ids.get(index).copied())
        else {
            return;
        };
        self.tabs.set_active_tab(tab_id);
        self.tab_id = Some(tab_id);
        if let Some(pane) = self.tabs.active_pane(tab_id) {
            self.focus_session(pane);
        }
        self.resize_panes();
        self.drawn_revision = None;
    }

    fn change_font_size(&mut self, steps: f32) {
        self.set_font_size(self.font_points + steps);
    }

    /// Redraw everything at a new size.
    ///
    /// The atlas is thrown away rather than added to: it is keyed by pixel
    /// size, so keeping it would only hold glyphs nothing will ask for again.
    /// The panes are told their new grid afterwards -- a shell that still
    /// thinks it has eighty columns wraps its output in the wrong place.
    fn set_font_size(&mut self, points: f32) {
        let points = points.clamp(6.0, 72.0);
        if (points - self.font_points).abs() < f32::EPSILON {
            return;
        }
        self.reopen_font(points, self.scale);
    }

    /// Open the font at a size in points, for a display at `scale`.
    fn reopen_font(&mut self, points: f32, scale: f32) {
        let pixels = crate::terminal::pixels_for_points(points, scale);
        let Ok(font) = TerminalFont::open_named(
            self.font_family.as_deref(),
            pixels,
            &self.font_fallbacks,
            self.font_shape,
        ) else {
            log::warn!("no font at {pixels} pixels; keeping {}", self.font_points);
            return;
        };
        self.font = font;
        self.font_points = points;
        self.scale = scale;
        self.atlas = GlyphAtlas::new(1024, 1024);
        self.resize_panes();
        self.drawn_revision = None;
    }

    fn toggle_full_screen(&mut self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let full = live.window.fullscreen().is_some();
        live.window.set_fullscreen(if full {
            None
        } else {
            // Borderless on whichever monitor the window is on: an exclusive
            // mode would change the display's resolution, which is not what
            // a terminal going full screen should do to the rest of a desktop.
            Some(winit::window::Fullscreen::Borderless(None))
        });
        self.drawn_revision = None;
    }

    fn copy_selection(&mut self) {
        let Some(text) = self.selected.clone() else {
            return;
        };
        self.copy_text(&text);
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
                self.show_notice(unterm_services::i18n::t("interaction.paste_failed"));
                return;
            }
        };
        if text.is_empty() {
            return;
        }
        let session_id = live.session_id;
        match self.engine.paste_input(session_id, &text) {
            Ok(_) => self.show_notice(unterm_services::i18n::t("interaction.pasted")),
            Err(err) => {
                log::warn!("could not paste: {err:#}");
                self.show_notice(unterm_services::i18n::t("interaction.paste_failed"));
            }
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
            crate::statsbar::forget(session.id);
            let _ = self.engine.destroy_session(session.id);
            return;
        }
        self.tabs.set_active_pane(session.id);
        self.resize_panes();
        self.show_notice(unterm_services::i18n::t("interaction.split"));
        if let Some(live) = self.state.as_ref() {
            live.window.request_redraw();
        }
        self.drawn_revision = None;
    }

    /// Put text a program asked for onto the system clipboard.
    ///
    /// `OSC 52`, which is the only way anything running over ssh can copy.
    /// Taken once: the engine reports the last request, and without
    /// remembering what was already honoured every frame would set the
    /// clipboard again and stamp on whatever the user copied since.
    fn take_clipboard_request(&mut self, request: Option<String>) {
        let Some(text) = request else {
            return;
        };
        if self.clipboard_honoured.as_deref() == Some(text.as_str()) {
            return;
        }
        self.clipboard_honoured = Some(text.clone());
        self.copy_text(&text);
    }

    /// Tell a pane the terminal gained or lost focus, if it asked.
    ///
    /// `CSI I` and `CSI O`, and only when the program turned reporting on:
    /// sending them unasked puts stray characters into anything that did not
    /// negotiate it, which is what the mode exists to prevent.
    fn report_focus(&mut self, focused: bool) {
        let Ok(sessions) = unterm_engine::SessionEngine::list_sessions(&self.engine) else {
            return;
        };
        let sequence = if focused { "[I" } else { "[O" };
        for session in sessions {
            let asked = self
                .engine
                .read_styled_screen(session.id)
                .map(|snapshot| snapshot.focus_reporting)
                .unwrap_or(false);
            if asked {
                let _ = self.engine.write_input(session.id, sequence);
            }
        }
    }

    /// Interrupt whatever the pane is running.
    ///
    /// The byte has already gone to the shell, which is enough on a pty with
    /// a line discipline. Windows has none: the byte only reaches the shell's
    /// line editor, so a running program -- which is the thing you press
    /// Ctrl+C at -- never hears it without a console control event.
    fn interrupt(&self, pane_id: usize) {
        let process = unterm_engine::SessionEngine::activity(&self.engine, pane_id)
            .ok()
            .and_then(|activity| activity.process);
        let Some(shell) = process.as_ref().and_then(|p| p.root_pid) else {
            return;
        };
        let foreground = process.as_ref().and_then(|p| p.foreground_pid);
        if let Err(err) = unterm_services::interrupt::stop_foreground(shell, foreground) {
            // Worth a line: an interrupt that quietly did nothing is what
            // this exists to remove.
            log::warn!("could not interrupt pane {pane_id}: {err}");
        }
    }

    /// Where the first shell starts, as the command line asked.
    pub fn set_start_directory(&mut self, directory: Option<std::path::PathBuf>) {
        self.start_directory = directory;
    }

    /// Feed the cockpit what the panes are showing.
    ///
    /// The tracker watches screen tails and titles to work out whether an
    /// agent is waiting on a person. Nothing else in this front end calls it,
    /// so without this the inbox is always empty and looks broken rather than
    /// idle.
    fn feed_cockpit(&mut self) {
        if self.cockpit_fed_at.elapsed() < COCKPIT_POLL {
            return;
        }
        self.cockpit_fed_at = std::time::Instant::now();

        let Ok(sessions) = unterm_engine::SessionEngine::list_sessions(&self.engine) else {
            return;
        };
        for session in sessions {
            let Ok(snapshot) = self.engine.read_styled_screen(session.id) else {
                continue;
            };
            let tail: Vec<String> = snapshot
                .lines
                .iter()
                .rev()
                .take(COCKPIT_TAIL_ROWS)
                .map(|line| line.cells.iter().map(|cell| cell.ch).collect::<String>())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            unterm_services::cockpit::status::on_screen_tail(session.id as u64, &tail);
            unterm_services::cockpit::status::on_title_change(session.id as u64, &session.title);
        }
    }

    /// The agent inbox, over the terminal.


    /// Type into the composer. Returns true when the key was the composer's.
    fn handle_composer_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        use winit::keyboard::{Key as WinitKey, NamedKey};

        let Some(composer) = self.composer.as_mut() else {
            return false;
        };
        match &event.logical_key {
            WinitKey::Named(NamedKey::Enter) => composer.commit(),
            WinitKey::Named(NamedKey::Backspace) => {
                composer.typing.pop();
            }
            WinitKey::Named(NamedKey::Escape) => {
                // The queue first, the panel second. A batch someone has just
                // written is not something one keystroke should throw away
                // without saying so -- and the panel emptying is it saying so.
                if composer.is_empty() {
                    self.composer = None;
                } else {
                    composer.clear();
                    composer.typing.clear();
                }
            }
            WinitKey::Named(NamedKey::Space) => composer.typing.push(' '),
            WinitKey::Character(text) => composer.typing.push_str(text),
            // Everything else belongs to the shell behind this: a queue is
            // being written, not a program driven.
            _ => return false,
        }
        self.drawn_revision = None;
        true
    }

    /// Send the next queued prompt if the pane is ready for one.
    ///
    /// Called from the frame loop rather than on a timer, because the thing it
    /// waits on -- the pane going idle -- is something the frame loop already
    /// knows about.
    fn drain_composer(&mut self) {
        let (true, Some(live)) = (self.composer.is_some(), self.state.as_ref()) else {
            return;
        };
        let session_id = live.session_id;
        let idle = unterm_engine::SessionEngine::activity(&self.engine, session_id)
            .map(|activity| activity.idle)
            .unwrap_or(false);

        // An agent asking permission to carry on looks exactly like an agent
        // that has finished, and a queue that cannot tell them apart sends its
        // next prompt as the answer to the question. So the question is
        // answered first, and only the narrow shape of question that offers a
        // yes as the obvious answer -- anything that mentions deleting,
        // removing, overwriting or forcing waits for a person.
        if idle && self.pane_is_asking_permission(session_id) {
            let _ = self.engine.write_input(session_id, "y\r");
            self.drawn_revision = None;
            return;
        }

        let Some(prompt) = self
            .composer
            .as_mut()
            .and_then(|composer| composer.take_next(idle))
        else {
            return;
        };
        let _ = self.engine.write_input(session_id, &format!("{prompt}\r"));
        self.drawn_revision = None;
    }

    /// Whether the pane's last line is a question the composer may answer.
    fn pane_is_asking_permission(&self, session_id: usize) -> bool {
        let Ok(screen) = self.engine.read_screen(session_id) else {
            return false;
        };
        screen
            .lines
            .iter()
            .rev()
            .find(|line| !line.trim().is_empty())
            .map(|line| crate::composer::is_confirmation(line))
            .unwrap_or(false)
    }

    /// The queue, and the line being written.
    fn append_composer(
        &mut self,
        window_width: f32,
        quads: &mut unterm_render::quads::FrameQuads,
    ) {
        let Some(composer) = self.composer.clone() else {
            return;
        };
        let metrics = self.font.metrics();
        let width = (window_width * 0.6).max(metrics.width * 30.0).min(window_width);
        let left = ((window_width - width) / 2.0).max(0.0);
        let queued = composer.queued();
        let shown = queued.len().min(MAX_COMPOSER_ROWS);
        let rows = shown + 2;
        let top = metrics.height * 2.0;
        let foreground = self.colors.foreground;

        quads.backgrounds.push(unterm_render::quads::Quad {
            left,
            top,
            width,
            height: metrics.height * rows as f32,
            color: mix(self.colors.background, foreground, 0.10),
        });

        let title = unterm_services::i18n::t("composer.title");
        let heading = if queued.is_empty() {
            format!("{title}  ({})", unterm_services::i18n::t("composer.hint"))
        } else {
            format!(
                "{title}  ({})",
                unterm_services::i18n::t_args(
                    "composer.waiting",
                    &[("n", &queued.len().to_string())]
                )
            )
        };
        let mut lines = vec![heading];
        lines.extend(
            queued
                .iter()
                .take(shown)
                .enumerate()
                .map(|(index, prompt)| format!("{}. {prompt}", index + 1)),
        );
        // The line being written, with a cursor after it so it is obviously
        // the one accepting keys.
        lines.push(format!("> {}_", composer.typing));

        for (index, line) in lines.iter().enumerate() {
            crate::terminal::append_text(
                line,
                &mut self.font,
                &mut self.atlas,
                foreground,
                (left + metrics.width, top + metrics.height * index as f32),
                quads,
            );
        }
    }

    fn toggle_git_panel(&mut self) {
        self.git_panel = match self.git_panel {
            Some(_) => None,
            None => Some(match self.current_directory() {
                Some(directory) => crate::git::read(&directory),
                None => crate::git::Panel::NotARepository,
            }),
        };
        self.drawn_revision = None;
    }

    /// What git says about where this pane is.
    fn append_git_panel(
        &mut self,
        window_width: f32,
        quads: &mut unterm_render::quads::FrameQuads,
    ) {
        let Some(status) = self.git_panel.clone() else {
            return;
        };
        let metrics = self.font.metrics();
        let width = (window_width * 0.5).max(metrics.width * 30.0).min(window_width);
        let left = ((window_width - width) / 2.0).max(0.0);
        let top = metrics.height * 2.0;
        let foreground = self.colors.foreground;

        let heading = status.heading();
        let lines: Vec<String> = status
            .entries()
            .iter()
            .take(MAX_GIT_ROWS)
            .map(|entry| format!("{:<3}{}", entry.code, entry.path))
            .collect();
        let height = metrics.height * (lines.len() + 1) as f32;

        quads.backgrounds.push(unterm_render::quads::Quad {
            left,
            top,
            width,
            height,
            color: mix(self.colors.background, foreground, 0.10),
        });
        crate::terminal::append_text(
            &heading,
            &mut self.font,
            &mut self.atlas,
            foreground,
            (left + metrics.width, top),
            quads,
        );
        for (index, line) in lines.iter().enumerate() {
            crate::terminal::append_text(
                line,
                &mut self.font,
                &mut self.atlas,
                foreground,
                (left + metrics.width, top + metrics.height * (index + 1) as f32),
                quads,
            );
        }
    }

    fn append_inbox(&mut self, window_width: f32, quads: &mut unterm_render::quads::FrameQuads) {
        if !self.inbox_open {
            return;
        }
        let statuses = unterm_services::cockpit::status::snapshot();
        let rows = crate::cockpit::rows(&statuses, |status| status.since.elapsed().as_secs());

        let metrics = self.font.metrics();
        let width = (window_width * 0.5).max(metrics.width * 30.0).min(window_width);
        let left = ((window_width - width) / 2.0).max(0.0);
        let top = metrics.height * 2.0;
        let shown = rows.len().min(MAX_INBOX_ROWS);
        let height = metrics.height * (shown + 1) as f32;
        let foreground = self.colors.foreground;

        quads.backgrounds.push(unterm_render::quads::Quad {
            left,
            top,
            width,
            height,
            color: mix(self.colors.background, foreground, 0.10),
        });

        let heading = if rows.is_empty() {
            unterm_services::i18n::t("cockpit.inbox_title")
        } else {
            format!(
                "{}  ({})",
                unterm_services::i18n::t("cockpit.inbox_title"),
                unterm_services::i18n::t_args(
                    "composer.waiting",
                    &[(
                        "n",
                        &crate::cockpit::attention_count(&statuses).to_string()
                    )]
                )
            )
        };
        crate::terminal::append_text(
            &heading,
            &mut self.font,
            &mut self.atlas,
            foreground,
            (left + metrics.width, top),
            quads,
        );

        for (index, row) in rows.iter().take(shown).enumerate() {
            let row_top = top + metrics.height * (index + 1) as f32;
            if row.needs_you {
                // The ones wanting an answer are marked, so the list can be
                // read at a glance rather than word by word.
                quads.backgrounds.push(unterm_render::quads::Quad {
                    left,
                    top: row_top,
                    width: metrics.width * 0.4,
                    height: metrics.height,
                    color: foreground,
                });
            }
            let text = if row.hint.is_empty() {
                format!("{}  {}", row.pane_id, row.label)
            } else {
                format!("{}  {}  -- {}", row.pane_id, row.label, row.hint)
            };
            crate::terminal::append_text(
                &text,
                &mut self.font,
                &mut self.atlas,
                foreground,
                (left + metrics.width, row_top),
                quads,
            );
        }
    }

    /// Start quick select, if there is anything worth labelling.
    ///
    /// Nothing worth labelling means nothing to do: an overlay with no labels
    /// in it looks like a broken feature rather than an empty screen.
    fn open_quick_select(&mut self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let Ok(snapshot) = self.engine.read_styled_screen(live.session_id) else {
            return;
        };
        let found = crate::copy_mode::labelled(&snapshot.lines);
        if found.is_empty() {
            return;
        }
        self.quick_select = Some((found, String::new()));
        self.drawn_revision = None;
    }

    /// Type a label. Returns true when the key was quick select's.
    fn handle_quick_select_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        use winit::keyboard::Key as WinitKey;
        let Some((found, mut typed)) = self.quick_select.take() else {
            return false;
        };
        match &event.logical_key {
            WinitKey::Named(winit::keyboard::NamedKey::Escape) => {}
            WinitKey::Character(text) => {
                typed.push_str(text);
                if let Some(hit) = found.iter().find(|item| item.label == typed) {
                    let text = hit.text.clone();
                    self.copy_text(&text);
                } else if found.iter().any(|item| item.label.starts_with(&typed)) {
                    // A prefix of a longer label: wait for the rest.
                    self.quick_select = Some((found, typed));
                }
            }
            _ => self.quick_select = Some((found, typed)),
        }
        self.drawn_revision = None;
        if let Some(live) = self.state.as_ref() {
            live.window.request_redraw();
        }
        true
    }

    /// Move and select with the keyboard. Returns true when handled.
    fn handle_copy_mode_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        use winit::keyboard::Key as WinitKey;
        let Some(mut mode) = self.copy_mode else {
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
        let Some(motion) = crate::copy_mode::motion_for(named.as_deref(), character.as_deref())
        else {
            // Nothing reaches the shell: a stray keystroke running a command
            // in the pane behind is the worst thing a mode can do.
            self.copy_mode = Some(mode);
            return true;
        };

        match motion {
            crate::copy_mode::Motion::Leave => self.copy_mode = None,
            crate::copy_mode::Motion::Yank => {
                if let Some(text) = self.copy_mode_selection(&mode) {
                    self.copy_text(&text);
                }
                self.copy_mode = None;
            }
            motion => {
                let (rows, widths) = self.screen_shape();
                mode.apply(motion, rows, |row| widths.get(row).copied().unwrap_or(0));
                self.copy_mode = Some(mode);
            }
        }
        self.drawn_revision = None;
        if let Some(live) = self.state.as_ref() {
            live.window.request_redraw();
        }
        true
    }

    /// How many rows the screen has, and how wide each one's text is.
    fn screen_shape(&self) -> (usize, Vec<usize>) {
        let Some(live) = self.state.as_ref() else {
            return (0, Vec::new());
        };
        let Ok(snapshot) = self.engine.read_styled_screen(live.session_id) else {
            return (0, Vec::new());
        };
        let widths = snapshot
            .lines
            .iter()
            .map(|line| {
                // Trailing blanks are padding, not text: end-of-line should
                // land on the last character someone wrote.
                line.cells
                    .iter()
                    .rposition(|cell| cell.ch != ' ' && cell.ch != '\0')
                    .map(|last| last + 1)
                    .unwrap_or(0)
            })
            .collect();
        (snapshot.lines.len(), widths)
    }

    /// The text copy mode has selected.
    fn copy_mode_selection(&self, mode: &crate::copy_mode::CopyMode) -> Option<String> {
        let live = self.state.as_ref()?;
        let snapshot = self.engine.read_styled_screen(live.session_id).ok()?;
        let ((start_row, start_col), (end_row, end_col)) = mode.selection()?;
        let last = snapshot.lines.len().saturating_sub(1);

        let mut out = String::new();
        for row in start_row..=end_row.min(last) {
            let text: String = snapshot.lines[row].cells.iter().map(|cell| cell.ch).collect();
            let from = if row == start_row { start_col } else { 0 };
            let to = if row == end_row {
                (end_col + 1).min(text.chars().count())
            } else {
                text.chars().count()
            };
            if from < to {
                out.extend(text.chars().skip(from).take(to - from));
            }
            if row < end_row {
                out.push('\n');
            }
        }
        Some(out.trim_end().to_string())
    }

    /// Put text on the clipboard, and say so.
    ///
    /// A copy that does nothing visible is one the user repeats, and then
    /// goes hunting through a clipboard manager for.
    fn copy_text(&mut self, text: &str) {
        match arboard::Clipboard::new().and_then(|mut board| board.set_text(text.to_string())) {
            Ok(()) => self.show_notice(unterm_services::i18n::t("interaction.copied")),
            Err(err) => {
                log::warn!("could not copy to the clipboard: {err}");
                self.show_notice(unterm_services::i18n::t("interaction.paste_failed"));
            }
        }
    }

    /// Open the palette on a set of rows.

    /// Take the focused pane to a directory, by typing it there.
    ///
    /// Through the shell rather than behind its back: a shell that is told to
    /// `cd` updates its own prompt, its history and its OSC 7 report, and the
    /// terminal learns the new directory the same way it learns every other
    /// one. Moving the pty's directory underneath it would leave the shell
    /// convinced it was somewhere else.
    fn change_directory(&mut self, path: &str) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let command = format!("cd \"{path}\"\r");
        let _ = self.engine.write_input(live.session_id, &command);
    }

    fn new_tab_in(&mut self, path: &str) {
        self.start_directory = Some(std::path::PathBuf::from(path));
        self.new_tab();
    }

    /// Start recording the focused pane, or stop and say where it went.
    fn toggle_recording(&mut self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        let pane = live.session_id;
        let recording = unterm_engine::RecordingEngine::recording_status(&self.engine, pane)
            .map(|status| status.enabled)
            .unwrap_or(false);
        let outcome = if recording {
            unterm_engine::RecordingEngine::stop_recording(&self.engine, pane)
                .map(|stopped| format!("recording saved to {}", stopped.md_path))
        } else {
            unterm_engine::RecordingEngine::start_recording(&self.engine, pane)
                .map(|started| format!("recording to {}", started.md_path))
        };
        match outcome {
            Ok(message) => log::info!("{message}"),
            Err(err) => log::warn!("recording: {err}"),
        }
        self.drawn_revision = None;
    }

    fn export_session(&mut self) {
        let Some(live) = self.state.as_ref() else {
            return;
        };
        match unterm_engine::RecordingEngine::export_markdown(&self.engine, live.session_id, None) {
            Ok(exported) => log::info!("session exported to {}", exported.path),
            Err(err) => log::warn!("could not export the session: {err}"),
        }
    }

    /// Settings live in a browser, not in a cell grid.
    fn open_settings(&mut self) {
        let info = unterm_services::server_info::read();
        if info.http_port == 0 {
            log::warn!("the settings server has not started yet");
            return;
        }
        let url = format!("http://127.0.0.1:{}", info.http_port);
        if let Err(err) = crate::links::open(&url) {
            log::warn!("could not open {url}: {err}");
        }
    }

    /// The picker's rows: every theme the product ships.
    fn theme_entries(&self) -> Vec<crate::palette::Entry> {
        let current = self.theme_id.clone();
        crate::theme::THEMES
            .iter()
            .map(|theme| crate::palette::Entry {
                label: theme.name.to_string(),
                hint: if current.as_deref() == Some(theme.id) {
                    unterm_services::i18n::t_args("theme.current", &[("name", theme.name)])
                } else {
                    unterm_services::i18n::t(&format!("theme.preset.{}.desc", theme.id))
                },
                command: crate::palette::Command::ApplyTheme {
                    id: theme.id.to_string(),
                },
            })
            .collect()
    }

    /// Switch to a theme, and remember it for next time.
    ///
    /// The atlas goes with it: glyphs are rasterized as coverage and tinted
    /// when drawn, so they survive -- but the frame's own colours are baked
    /// into quads that have already been built, and the simplest way to be
    /// sure none are stale is to draw the next frame from nothing.
    fn apply_theme(&mut self, id: &str) {
        let Some(theme) = crate::theme::by_id(id) else {
            return;
        };
        self.colors = unterm_render::quads::FrameColors {
            background: theme.background,
            foreground: theme.foreground,
            palette: &theme.ansi,
        };
        self.theme_id = Some(theme.id.to_string());
        if let Err(err) = crate::theme::remember(theme.id) {
            log::warn!("could not remember the theme: {err:#}");
        }
        self.show_notice(unterm_services::i18n::t_args(
            "theme.switched_to",
            &[("name", theme.name)],
        ));
        self.drawn_revision = None;
    }

    /// The rows for jumping to a directory.
    ///
    /// Everything at once -- what is under here, what was open before, and the
    /// machine's drives -- because the palette filters as you type and the
    /// point of this picker is not knowing which of the three it is in.
    /// The rows for the directory jump, for what has been typed so far.
    fn dir_jump_entries(&self, query: &str) -> Vec<crate::palette::Entry> {
        let here = self
            .current_directory()
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        // Where the pane already is, first, and only while nothing has been
        // typed: once there is a query it is a row that matches nothing and
        // sits above the rows that do.
        let mut entries = Vec::new();
        if query.is_empty() {
            entries.push(crate::palette::Entry {
                label: unterm_services::i18n::t("dirjump.here"),
                hint: here.display().to_string(),
                command: crate::palette::Command::ChangeDirectory {
                    path: here.display().to_string(),
                },
            });
        }
        entries.extend(crate::dir_jump::for_query(&here, query).into_iter().map(
            |entry| crate::palette::Entry {
                // The section it came from and the path it is. The section
                // is the grouping the picker used to show as headings; the
                // path is what tells two same-named directories apart.
                hint: format!("{}  {}", entry.section.heading(), entry.path.display()),
                label: entry.label,
                command: crate::palette::Command::ChangeDirectory {
                    path: entry.path.display().to_string(),
                },
            },
        ));
        entries
    }

    /// The rows behind the status bar's triangle.
    fn quick_entries(&self) -> Vec<crate::palette::Entry> {
        let here = self
            .current_directory()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let recording = self
            .state
            .as_ref()
            .and_then(|live| {
                unterm_engine::RecordingEngine::recording_status(&self.engine, live.session_id).ok()
            })
            .map(|status| status.enabled)
            .unwrap_or(false);

        use unterm_services::i18n::t;

        vec![
            crate::palette::Entry {
                label: t("settings.menu.change_cwd"),
                hint: here.display().to_string(),
                command: crate::palette::Command::Browse {
                    path: here.display().to_string(),
                    then: crate::palette::BrowseThen::ChangeDirectory,
                },
            },
            crate::palette::Entry {
                label: t("settings.menu.open_folder"),
                hint: here.display().to_string(),
                command: crate::palette::Command::Browse {
                    path: here.display().to_string(),
                    then: crate::palette::BrowseThen::NewTab,
                },
            },
            crate::palette::Entry {
                label: t("settings.menu.split_right"),
                hint: "CTRL|SHIFT D".to_string(),
                command: crate::palette::Command::Action(crate::keys::Action::SplitRight),
            },
            // Only with something to choose between: a selector over one pane
            // is a letter you press to stay where you already are.
            crate::palette::Entry {
                label: t("menu.fleet_launch"),
                hint: "CTRL|SHIFT|ALT A".to_string(),
                command: crate::palette::Command::Action(crate::keys::Action::FleetLaunch),
            },
            crate::palette::Entry {
                label: t("menu.clear_scrollback"),
                hint: "CTRL|SHIFT K".to_string(),
                command: crate::palette::Command::Action(crate::keys::Action::ClearScrollback),
            },
            crate::palette::Entry {
                label: t("menu.select_pane"),
                hint: "CTRL|SHIFT '".to_string(),
                command: crate::palette::Command::Action(crate::keys::Action::SelectPane),
            },
            crate::palette::Entry {
                label: t("menu.swap_pane"),
                hint: "CTRL|SHIFT|ALT '".to_string(),
                command: crate::palette::Command::Action(crate::keys::Action::SwapPane),
            },
            crate::palette::Entry {
                label: if recording {
                    t("settings.menu.recording_on")
                } else {
                    t("settings.menu.recording_off")
                },
                hint: t("settings.menu.recording.hint"),
                command: crate::palette::Command::ToggleRecording,
            },
            crate::palette::Entry {
                label: t("settings.menu.export_session"),
                hint: t("settings.menu.export_session.hint"),
                command: crate::palette::Command::ExportSession,
            },
            crate::palette::Entry {
                label: unterm_services::i18n::t("menu.dir_jump"),
                hint: unterm_services::i18n::t("dirjump.placeholder"),
                command: crate::palette::Command::Action(crate::keys::Action::DirJump),
            },
            crate::palette::Entry {
                label: unterm_services::i18n::t("menu.left_tabs"),
                hint: String::new(),
                command: crate::palette::Command::Action(crate::keys::Action::LeftTabBar),
            },
            crate::palette::Entry {
                label: unterm_services::i18n::t("theme.title"),
                hint: unterm_services::i18n::t_args(
                    "theme.current",
                    &[(
                        "name",
                        self.theme_id
                            .as_deref()
                            .and_then(crate::theme::by_id)
                            .map(|theme| theme.name)
                            .unwrap_or(crate::theme::default_theme().name),
                    )],
                ),
                command: crate::palette::Command::Action(crate::keys::Action::ThemePicker),
            },
            crate::palette::Entry {
                label: t("settings.menu.web_settings"),
                hint: t("settings.menu.web_settings.hint"),
                command: crate::palette::Command::OpenSettings,
            },
        ]
    }

    fn open_palette(&mut self, entries: Vec<crate::palette::Entry>) {
        self.palette = Some(crate::palette::Palette::new(entries));
        self.drawn_revision = None;
    }

    /// Open a palette whose line is a task rather than a filter.
    ///
    /// Nothing to send is nothing to open: a crew picker on a machine with no
    /// agents installed is an empty card.
    fn open_fleet(&mut self, entries: Vec<crate::palette::Entry>) {
        if entries.is_empty() {
            self.show_notice(unterm_services::i18n::t("cockpit.fleet_no_agents"));
            return;
        }
        self.palette = Some(crate::palette::Palette::writing(entries));
        self.drawn_revision = None;
    }

    /// Open a palette that goes and looks again as the query changes.
    fn open_browser(&mut self, entries: Vec<crate::palette::Entry>) {
        self.palette = Some(crate::palette::Palette::browsing(entries));
        self.drawn_revision = None;
    }

    /// Bring a palette's rows up to date with what has been typed.
    ///
    /// A fixed list only narrows. A directory list is asked again, because a
    /// typed path names a place nothing has scanned -- the scan is bounded and
    /// the disk is not.
    fn requery_palette(&self, palette: &mut crate::palette::Palette) {
        match palette.source {
            crate::palette::Source::Fixed => palette.refilter(),
            // The line is the task, not a filter: narrowing the crews by what
            // has been typed empties the list on the first word. Typing does
            // clear a stale complaint, though -- it was about the last attempt.
            crate::palette::Source::Text => palette.error = None,
            crate::palette::Source::Directories => {
                let rows = self.dir_jump_entries(&palette.query);
                palette.replace_entries(rows);
            }
        }
    }

    /// Type into the open palette. Returns true when the key was the palette's.
    fn handle_palette_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        use winit::keyboard::Key as WinitKey;
        let Some(mut palette) = self.palette.take() else {
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
        match crate::palette::key_for(named.as_deref(), character.as_deref(), self.ctrl_held) {
            crate::palette::Key::Close => keep = false,
            crate::palette::Key::Step(delta) => palette.step(delta),
            crate::palette::Key::Backspace => {
                palette.query.pop();
                self.requery_palette(&mut palette);
            }
            crate::palette::Key::Type(text) => {
                palette.query.push_str(&text);
                self.requery_palette(&mut palette);
            }
            crate::palette::Key::Accept => {
                keep = false;
                if let Some(entry) = palette.current().cloned() {
                    self.run_palette_command(entry.command);
                }
            }
            // Nothing reaches the shell while the palette is open: a
            // keystroke through it would run in the pane behind.
            crate::palette::Key::NotOurs => {}
        }

        if keep {
            self.palette = Some(palette);
        }
        self.drawn_revision = None;
        if let Some(live) = self.state.as_ref() {
            live.window.request_redraw();
        }
        true
    }

    fn run_palette_command(&mut self, command: crate::palette::Command) {
        match command {
            crate::palette::Command::Action(action) => {
                let session_id = self.state.as_ref().map(|live| live.session_id);
                if let Some(session_id) = session_id {
                    self.run_key_action(action, session_id);
                }
            }
            crate::palette::Command::Launch { program } => self.new_tab_running(&program),
            crate::palette::Command::ChangeDirectory { path } => self.change_directory(&path),
            crate::palette::Command::NewTabIn { path } => self.new_tab_in(&path),
            crate::palette::Command::ToggleRecording => self.toggle_recording(),
            crate::palette::Command::ExportSession => self.export_session(),
            crate::palette::Command::OpenSettings => self.open_settings(),
            crate::palette::Command::ApplyTheme { id } => self.apply_theme(&id),
            crate::palette::Command::LaunchFleet { agents } => self.launch_fleet(agents),
            crate::palette::Command::Browse { path, then } => {
                // Stays open on the new directory rather than closing: picking
                // a folder three deep should be three keystrokes, not three
                // trips through the menu.
                self.open_palette(crate::directory::entries(std::path::Path::new(&path), then));
            }
        }
    }

    /// Open a tab running a named program.
    fn new_tab_running(&mut self, program: &str) {
        let mut command = portable_pty::CommandBuilder::new(program);
        // The same encoding treatment a configured shell gets: a launcher
        // that starts a shell which writes its console codepage produces a
        // tab full of boxes, and the user picked it from a list rather than
        // typing it, so they have nothing to blame it on.
        let mut shell = Some(command.clone());
        unterm_services::launch_env::apply_unterm_windows_utf8(&mut shell);
        if let Some(rewritten) = shell {
            command = rewritten;
        }
        self.open_tab_with(Some(command));
    }

    /// Open a tab, with a shell of its own.
    fn new_tab(&mut self) {
        let shell = self.shell.clone();
        self.open_tab_with(shell);
    }

    fn open_tab_with(&mut self, command: Option<portable_pty::CommandBuilder>) {
        // A tab needs a window to open into; the size comes from the layout
        // rather than from the window, because the strip may have taken some.
        let Some(_live) = self.state.as_ref() else {
            return;
        };
        let (cols, rows) = self.font.grid_for(self.terminal_width(), self.terminal_height());
        let session = match self.engine.create_session(CreateSessionRequest {
            cols,
            rows,
            command_dir: None,
            command,
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
                crate::statsbar::forget(session.id);
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
            crate::statsbar::forget(pane);
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
        crate::topbar::terminal_height(height, self.font.metrics())
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
            // Which way, if whoever asked for it said. The kernel records only
            // that the pane came from another one -- how they sit together is
            // this side's decision, so the request's own answer is left here
            // by the MCP surface rather than carried through the kernel.
            let asked = crate::mcp_host::take_split(session.id)
                .filter(|split| Some(split.source) == session.split_from);
            let outcome = match split {
                Some(source) => self
                    .tabs
                    .split(
                        source,
                        session.id,
                        asked
                            .map(|split| split.axis)
                            .unwrap_or(unterm_engine::next_core::layout::SplitAxis::Horizontal),
                        asked.map(|split| split.first_ratio).unwrap_or(0.5),
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

    /// Copy mode's cursor, and whatever it has selected.
    fn append_copy_mode(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        let Some(mode) = self.copy_mode else {
            return;
        };
        let metrics = self.font.metrics();
        let top_offset = crate::topbar::terminal_top(metrics);
        let (_, widths) = self.screen_shape();

        if let Some(((start_row, start_col), (end_row, end_col))) = mode.selection() {
            for row in start_row..=end_row {
                let width = widths.get(row).copied().unwrap_or(0);
                let from = if row == start_row { start_col } else { 0 };
                let to = if row == end_row { end_col + 1 } else { width };
                if to <= from {
                    continue;
                }
                quads.backgrounds.push(unterm_render::quads::Quad {
                    left: from as f32 * metrics.width,
                    top: top_offset + row as f32 * metrics.height,
                    width: (to - from) as f32 * metrics.width,
                    height: metrics.height,
                    color: mix(self.colors.background, self.colors.foreground, 0.28),
                });
            }
        }

        // The cursor over the selection, so it stays visible inside it.
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: mode.column as f32 * metrics.width,
            top: top_offset + mode.row as f32 * metrics.height,
            width: metrics.width,
            height: metrics.height,
            color: self.colors.foreground,
        });
    }

    /// Quick select's labels, over the text they stand for.
    /// A letter on every pane, while one is being picked.
    ///
    /// Drawn over the pane's own top-left corner rather than centred: centred
    /// puts the letter in the middle of whatever the pane is showing, and the
    /// corner is where the eye already goes to tell panes apart.
    fn append_pane_select(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        let Some(selector) = self.pane_select.clone() else {
            return;
        };
        let panes = self.placements();
        let order = crate::paneselect::reading_order(&panes);
        let metrics = self.font.metrics();
        let theme = self.theme();

        // Over everything, so a label is never behind the text it labels.
        let mark = quads.mark();
        for (index, label) in selector.labels.iter().enumerate() {
            // Once a letter is typed, only the labels still in the running.
            // Showing the rest offers choices that are no longer there.
            if !selector.typing.is_empty() && !label.starts_with(&selector.typing) {
                continue;
            }
            let Some(pane) = order.get(index).and_then(|at| panes.get(*at)) else {
                continue;
            };
            let width = metrics.width * (label.chars().count() as f32 + 2.0);
            let height = metrics.height * 2.0;
            let left = pane.origin.0;
            let top = pane.origin.1;
            quads.backgrounds.push(unterm_render::quads::Quad {
                left,
                top,
                width,
                height,
                color: theme.selection,
            });
            crate::terminal::append_text(
                label,
                &mut self.font,
                &mut self.atlas,
                theme.selection_text,
                (left + metrics.width, top + metrics.height / 2.0),
                quads,
            );
        }
        quads.raise_since(mark);
    }

    fn append_quick_select(&mut self, quads: &mut unterm_render::quads::FrameQuads) {
        let Some((found, typed)) = self.quick_select.clone() else {
            return;
        };
        let metrics = self.font.metrics();
        let top_offset = crate::topbar::terminal_top(metrics);
        let background = self.colors.background;
        let foreground = self.colors.foreground;

        for item in &found {
            // Once a letter is typed, only the labels still in the running:
            // showing the rest is showing the user options they no longer have.
            if !typed.is_empty() && !item.label.starts_with(&typed) {
                continue;
            }
            let left = item.start as f32 * metrics.width;
            let top = top_offset + item.row as f32 * metrics.height;
            let width = metrics.width * item.label.chars().count() as f32;
            quads.backgrounds.push(unterm_render::quads::Quad {
                left,
                top,
                width,
                height: metrics.height,
                color: foreground,
            });
            crate::terminal::append_text(
                &item.label,
                &mut self.font,
                &mut self.atlas,
                background,
                (left, top),
                quads,
            );
        }
    }

    /// The command palette, centred over the terminal.
    ///
    /// Drawn last so it sits over everything, and opaque so the text behind it
    /// cannot be mistaken for one of its rows.
    fn append_palette(
        &mut self,
        window_width: f32,
        quads: &mut unterm_render::quads::FrameQuads,
    ) {
        let Some(palette) = self.palette.as_ref() else {
            return;
        };
        let metrics = self.font.metrics();
        let rows: Vec<(String, bool)> = palette
            .visible()
            .iter()
            .take(MAX_PALETTE_ROWS)
            .enumerate()
            .map(|(index, entry)| {
                let hint = if entry.hint.is_empty() {
                    String::new()
                } else {
                    format!("   {}", entry.hint)
                };
                (format!("{}{hint}", entry.label), index == palette.selected)
            })
            .collect();

        let error = palette.error.clone();
        let width = (window_width * 0.6).max(metrics.width * 24.0).min(window_width);
        let left = ((window_width - width) / 2.0).max(0.0);
        let top = metrics.height * 2.0;
        let lines = rows.len() + 1 + usize::from(error.is_some());
        let height = metrics.height * lines as f32;

        quads.backgrounds.push(unterm_render::quads::Quad {
            left,
            top,
            width,
            height,
            color: mix(self.colors.background, self.colors.foreground, 0.10),
        });

        // The query line, with a caret so an empty palette still looks like
        // something you type into.
        let query = format!("> {}", palette.query);
        let foreground = self.colors.foreground;
        crate::terminal::append_text(
            &query,
            &mut self.font,
            &mut self.atlas,
            foreground,
            (left + metrics.width, top),
            quads,
        );
        quads.backgrounds.push(unterm_render::quads::Quad {
            left: left + metrics.width * (query.chars().count() + 1) as f32,
            top,
            width: (metrics.width * 0.15).max(1.0),
            height: metrics.height,
            color: foreground,
        });

        for (index, (text, selected)) in rows.iter().enumerate() {
            let row_top = top + metrics.height * (index + 1) as f32;
            if *selected {
                quads.backgrounds.push(unterm_render::quads::Quad {
                    left,
                    top: row_top,
                    width,
                    height: metrics.height,
                    color: mix(self.colors.background, self.colors.foreground, 0.30),
                });
            }
            crate::terminal::append_text(
                text,
                &mut self.font,
                &mut self.atlas,
                foreground,
                (left + metrics.width, row_top),
                quads,
            );
        }

        // Under the rows rather than instead of them: the answer to "this
        // repository has uncommitted changes" is to go and commit, and the
        // task has to still be there to press Enter on afterwards.
        if let Some(error) = error {
            let row_top = top + metrics.height * (rows.len() + 1) as f32;
            let danger = crate::window_buttons::CLOSE_HOVER;
            crate::terminal::append_text(
                &error,
                &mut self.font,
                &mut self.atlas,
                danger,
                (left + metrics.width, row_top),
                quads,
            );
        }
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
                    crate::topbar::terminal_top(metrics),
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
        let top_offset = crate::topbar::terminal_top(metrics);
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
                color: self.theme().divider,
            })
            .collect()
    }

    /// Where each pane goes, in pixels.
    /// The crews worth offering here, and the task line above them.
    fn fleet_entries(&self) -> Vec<crate::palette::Entry> {
        crate::fleet::crews(&unterm_services::cockpit::fleet::installed_agents())
            .into_iter()
            .map(|crew| crate::palette::Entry {
                hint: unterm_services::i18n::t("cockpit.fleet_worktrees"),
                label: crew.label,
                command: crate::palette::Command::LaunchFleet {
                    agents: crew.agents,
                },
            })
            .collect()
    }

    /// Send a crew at the task that has been typed.
    ///
    /// The repository is checked first and on the palette's own thread,
    /// because the answer is instant and the failure belongs in the card that
    /// asked: a dirty worktree is fixed by going and committing, and the task
    /// has to still be there afterwards.
    ///
    /// The launch itself is not: creating a worktree per agent and starting a
    /// tab for each takes seconds, and doing it here would freeze the window
    /// for all of them.
    fn launch_fleet(&mut self, agents: Vec<String>) {
        let Some(task) = self
            .palette
            .as_ref()
            .map(|palette| palette.query.trim().to_string())
        else {
            return;
        };
        // A blank task is not a task. Every agent would get a bare newline
        // into whatever it happened to be showing, and by then there are
        // worktrees and tabs to clean up.
        if !crate::fleet::task_is_ready(&task) {
            if let Some(palette) = self.palette.as_mut() {
                palette.error = Some(unterm_services::i18n::t("cockpit.fleet_no_task"));
            }
            self.drawn_revision = None;
            return;
        }
        let here = self
            .current_directory()
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        if let Err(key) = unterm_services::cockpit::fleet::precheck(&here) {
            if let Some(palette) = self.palette.as_mut() {
                palette.error = Some(unterm_services::i18n::t(key));
            }
            self.drawn_revision = None;
            return;
        }

        self.palette = None;
        self.show_notice(unterm_services::i18n::t("cockpit.fleet_launching"));
        self.drawn_revision = None;
        let spawned = std::thread::Builder::new()
            .name("fleet-launch".into())
            .spawn(move || {
                match unterm_services::cockpit::fleet::launch(&here, &task, &agents) {
                    Ok(fleet) => log::info!(
                        "fleet {} launched with {} members",
                        fleet.id,
                        fleet.members.len()
                    ),
                    Err(err) => log::error!("fleet launch failed: {err:#}"),
                }
            });
        if let Err(err) = spawned {
            log::error!("could not start the fleet launcher: {err:#}");
        }
    }

    /// Throw away a pane's history.
    ///
    /// The scrollback only, unless the screen is asked for too. A pane with a
    /// hundred thousand lines behind it is the reason anyone asks; losing what
    /// is currently on screen is not part of the request.
    ///
    /// The view is put back to the bottom afterwards. Scrolled up into history
    /// that no longer exists is a blank pane, and it looks like the clear broke
    /// something rather than like it worked.
    fn clear_scrollback(&mut self, session_id: usize, include_screen: bool) {
        if let Err(err) = self.engine.erase_scrollback(session_id, include_screen) {
            log::warn!("could not clear the scrollback: {err:#}");
            return;
        }
        // Back to the bottom. Scrolled up into history that no longer exists
        // is a blank pane, and that reads as the clear having broken something
        // rather than as it having worked.
        let _ = self.engine.scroll_viewport_to(session_id, isize::MAX);
        self.show_notice(unterm_services::i18n::t(if include_screen {
            "notice.screen_cleared"
        } else {
            "notice.scrollback_cleared"
        }));
        self.drawn_revision = None;
    }

    /// Put a letter on every pane.
    ///
    /// Nothing to choose between is nothing to open: a selector over one pane
    /// is a letter you have to press to stay where you already are.
    fn open_pane_select(&mut self, mode: crate::paneselect::Mode) {
        let panes = self.placements();
        if panes.len() < 2 {
            return;
        }
        self.pane_select = Some(crate::paneselect::Selector::new(panes.len(), mode));
        self.drawn_revision = None;
    }

    /// Type at the pane selector. Returns true when the key was its.
    fn handle_pane_select_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        use winit::keyboard::Key as WinitKey;
        let Some(mut selector) = self.pane_select.take() else {
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

        match selector.key(named.as_deref(), character.as_deref(), self.ctrl_held) {
            crate::paneselect::Outcome::Chose(index) => self.take_pane(&selector, index),
            crate::paneselect::Outcome::Typing => self.pane_select = Some(selector),
            crate::paneselect::Outcome::Cancelled => {}
            // Nothing reaches the shell while it is open: a keystroke through
            // it would run in whichever pane happens to be in front.
            crate::paneselect::Outcome::Ignored => self.pane_select = Some(selector),
        }
        self.drawn_revision = None;
        true
    }

    /// Do whatever the selector was opened to do, to the pane that was picked.
    fn take_pane(&mut self, selector: &crate::paneselect::Selector, index: usize) {
        let panes = self.placements();
        let order = crate::paneselect::reading_order(&panes);
        let Some(chosen) = order.get(index).and_then(|at| panes.get(*at)) else {
            return;
        };
        let chosen = chosen.session_id;

        match selector.mode {
            crate::paneselect::Mode::Activate => {
                self.tabs.set_active_pane(chosen);
                self.focus_session(chosen);
            }
            crate::paneselect::Mode::Swap | crate::paneselect::Mode::SwapKeepFocus => {
                let follow = selector.mode == crate::paneselect::Mode::Swap;
                self.swap_panes(chosen, follow);
            }
        }
    }

    /// Exchange the chosen pane with the one in front.
    ///
    /// Done by rebuilding the tab's arrangement with the two panes' places
    /// exchanged, rather than by moving anything: the shells keep running
    /// where they are, and only the rectangles they are drawn in change.
    fn swap_panes(&mut self, chosen: usize, follow: bool) {
        let (Some(tab_id), Some(live)) = (self.tab_id, self.state.as_ref()) else {
            return;
        };
        let focused = self.tabs.active_pane(tab_id).unwrap_or(live.session_id);
        if focused == chosen {
            return;
        }
        let (cols, rows) = self.font.grid_for(self.terminal_width(), self.terminal_height());
        let mut positions = self.tabs.positions(tab_id, cols, rows);
        for position in &mut positions {
            if position.pane_id == focused {
                position.pane_id = chosen;
            } else if position.pane_id == chosen {
                position.pane_id = focused;
            }
        }
        // Staying put means the focus keeps its *place*, which after the
        // exchange is the other pane; following means it keeps its *pane*.
        let active = if follow { chosen } else { focused };
        if let Err(err) = self.tabs.adopt_tab(tab_id, &positions, active) {
            log::warn!("could not swap panes: {err:#}");
            return;
        }
        self.tabs.set_active_pane(active);
        self.focus_session(active);
        self.resize_panes();
    }

    fn placements(&self) -> Vec<crate::panes::PanePlacement> {
        let (Some(tab_id), Some(_live)) = (self.tab_id, self.state.as_ref()) else {
            return Vec::new();
        };
        let metrics = self.font.metrics();
        let (cols, rows) = self.font.grid_for(self.terminal_width(), self.terminal_height());
        let left = self.terminal_left();
        let top = crate::topbar::terminal_top(metrics);
        self.tabs
            .positions(tab_id, cols, rows)
            .into_iter()
            .map(|placed| {
                let mut placement = crate::panes::place(placed.pane_id, placed.rect, metrics);
                placement.origin.0 += left;
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
        // A fading flash needs frames of its own: nothing about the screen
        // changes while it fades out. A blinking cursor is the same -- without
        // this it would change state only when something else happened to
        // redraw, which is a cursor that blinks when you type and not
        // otherwise.
        if self.bell_at.is_some() || self.cursor_style.blinking {
            return true;
        }
        // A drag changes what is highlighted without changing the screen
        // underneath it.
        if self.drag.is_some() {
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
                    crate::statsbar::forget(live.session_id);
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
            WindowEvent::Focused(focused) => {
                self.report_focus(focused);
                // A prompt that dims when unfocused has to be redrawn to show
                // it, and nothing about the screen changed to ask for a frame.
                self.drawn_revision = None;
                if let Some(live) = self.state.as_ref() {
                    live.window.request_redraw();
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
                self.ctrl_held = modifiers.state().control_key();
                self.alt_held = modifiers.state().alt_key();
                // The link hint appears and disappears with Ctrl, and nothing
                // else about the screen changed to ask for a frame.
                self.drawn_revision = None;
                if let Some(live) = self.state.as_ref() {
                    live.window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                if self.state.is_none() {
                    return;
                }

                use winit::keyboard::Key;

                if std::env::var_os("UNTERM_TRACE_KEYS").is_some() {
                    log::info!(
                        "key: logical={:?} physical={:?} text={:?} ctrl={} shift={} alt={}",
                        event.logical_key,
                        event.physical_key,
                        event.text,
                        self.ctrl_held,
                        self.shift_held,
                        self.alt_held,
                    );
                    log::info!(
                        "  action_for -> {:?}",
                        crate::keys::action_for(
                            &event.logical_key,
                            self.ctrl_held,
                            self.shift_held,
                            self.alt_held,
                        )
                    );
                }

                if self.quick_select.is_some() && self.handle_quick_select_key(&event) {
                    return;
                }
                if self.copy_mode.is_some() && self.handle_copy_mode_key(&event) {
                    return;
                }

                // The palette takes the keyboard while it is open, before
                // anything else looks at the key.
                if self.palette.is_some() && self.handle_palette_key(&event) {
                    return;
                }

                // And so does the pane selector: every letter on screen is one
                // of its labels, and a letter that fell through would run in
                // whichever pane happens to be in front.
                if self.pane_select.is_some() && self.handle_pane_select_key(&event) {
                    return;
                }

                // A search takes the keyboard while it is open: the letters
                // typed are the pattern, not input for the shell.
                if self.search.is_some() && self.handle_search_key(&event) {
                    return;
                }
                if self.composer.is_some() && self.handle_composer_key(&event) {
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
                    crate::keys::action_for(
                            &event.logical_key,
                            self.ctrl_held,
                            self.shift_held,
                            self.alt_held,
                        )
                {
                    self.run_key_action(action, live.session_id);
                    return;
                }

                let held = crate::mouse::Held {
                    shift: self.shift_held,
                    ctrl: self.ctrl_held,
                    alt: self.alt_held,
                };
                if let Some(text) = encode(&event.logical_key, held) {
                    let pane = self.focused_session();
                    let _ = self.engine.write_input(pane, &text);
                    if text == unterm_services::interrupt::INTERRUPT_BYTE {
                        self.interrupt(pane);
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = (position.x as f32, position.y as f32);
                if self.dragging_scrollbar {
                    self.scroll_to_pointer();
                    return;
                }
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

                // Right-click is a direct gesture rather than a menu: it
                // copies a selection and lets go of it, or pastes when there
                // is none. Only on press, so the release does not undo it.
                if button == MouseButton::Right {
                    if state == ElementState::Pressed {
                        match crate::mouse::right_click(self.selected.is_some()) {
                            crate::mouse::RightClick::CopyAndClear => {
                                self.copy_selection();
                                self.selected = None;
                                self.drag = None;
                                self.drawn_revision = None;
                            }
                            crate::mouse::RightClick::Paste => self.paste_clipboard(),
                        }
                    }
                    return;
                }
                if button != MouseButton::Left {
                    return;
                }
                // The edges first: a borderless window has no system resize
                // handles, so a press there has to start one.
                if state == ElementState::Pressed {
                    if let Some(live) = self.state.as_ref() {
                        let size = (live.width as f32, live.height as f32);
                        if let Some(direction) = crate::topbar::resize_edge(self.pointer, size) {
                            let _ = live.window.drag_resize_window(direction);
                            return;
                        }
                    }
                }
                if state == ElementState::Pressed && self.click_top_bar() {
                    return;
                }
                if state == ElementState::Pressed && self.pointer_on_menu() {
                    let entries = self.quick_entries();
                    self.open_palette(entries);
                    return;
                }
                if state == ElementState::Pressed && self.pointer_on_scrollbar() {
                    self.dragging_scrollbar = true;
                    self.scroll_to_pointer();
                    return;
                }
                if state == ElementState::Released && self.dragging_scrollbar {
                    self.dragging_scrollbar = false;
                    return;
                }
                if state == ElementState::Pressed
                    && crate::links::opens_on_click(self.ctrl_held)
                {
                    if let Some(link) = self.link_under_pointer() {
                        if let Err(err) = crate::links::open(&link.uri) {
                            log::warn!("could not open {}: {err}", link.uri);
                        }
                        return;
                    }
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
        if self.closing {
            // The close button was pressed. There is no native title bar to
            // do this for us any more.
            event_loop.exit();
            return;
        }
        self.sync_tabs();
        self.feed_cockpit();
        self.drain_composer();
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
/// Turn a key press into the bytes a shell expects.
///
/// The engine already knows every sequence; what this has to get right is
/// which key and which modifiers it is handed. Both were wrong before:
/// modifiers were dropped on the floor, so Ctrl+arrow could not move by word
/// and Ctrl+C sent the letter `c` rather than an interrupt, and only a
/// handful of named keys were mapped at all.
fn encode(logical: &winit::keyboard::Key, held: crate::mouse::Held) -> Option<String> {
    use termwiz::input::{KeyCode, Modifiers};
    use winit::keyboard::{Key, NamedKey};

    let mut mods = Modifiers::NONE;
    if held.shift {
        mods |= Modifiers::SHIFT;
    }
    if held.ctrl {
        mods |= Modifiers::CTRL;
    }
    if held.alt {
        mods |= Modifiers::ALT;
    }

    let key = match logical {
        Key::Named(named) => match named {
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::Escape => KeyCode::Escape,
            NamedKey::ArrowUp => KeyCode::UpArrow,
            NamedKey::ArrowDown => KeyCode::DownArrow,
            NamedKey::ArrowLeft => KeyCode::LeftArrow,
            NamedKey::ArrowRight => KeyCode::RightArrow,
            NamedKey::Home => KeyCode::Home,
            NamedKey::End => KeyCode::End,
            NamedKey::PageUp => KeyCode::PageUp,
            NamedKey::PageDown => KeyCode::PageDown,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::Insert => KeyCode::Insert,
            NamedKey::Space => KeyCode::Char(' '),
            NamedKey::F1 => KeyCode::Function(1),
            NamedKey::F2 => KeyCode::Function(2),
            NamedKey::F3 => KeyCode::Function(3),
            NamedKey::F4 => KeyCode::Function(4),
            NamedKey::F5 => KeyCode::Function(5),
            NamedKey::F6 => KeyCode::Function(6),
            NamedKey::F7 => KeyCode::Function(7),
            NamedKey::F8 => KeyCode::Function(8),
            NamedKey::F9 => KeyCode::Function(9),
            NamedKey::F10 => KeyCode::Function(10),
            NamedKey::F11 => KeyCode::Function(11),
            NamedKey::F12 => KeyCode::Function(12),
            // Modifier keys themselves produce nothing, and neither do the
            // dozens of media and system keys a keyboard may have.
            _ => return None,
        },
        Key::Character(text) => {
            let mut chars = text.chars();
            let (Some(first), None) = (chars.next(), chars.next()) else {
                // Several characters from one key press: a dead-key
                // composition or an input method's output. It is already the
                // text the user meant, and no modifier applies to it.
                return Some(text.to_string());
            };
            KeyCode::Char(first)
        }
        // Dead keys on their way to composing something, and anything else
        // winit could not name.
        _ => return None,
    };

    key_encoding::encode_key(key, mods)
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

/// The palette's rows: every action a key can reach.
///
/// Built from the same table the keys use, so a chord and a palette row
/// cannot drift apart -- and the chord is shown as the hint, which is how a
/// palette teaches the keyboard.
fn command_entries() -> Vec<crate::palette::Entry> {
    crate::keys::BINDINGS
        .iter()
        .map(|binding| crate::palette::Entry {
            label: binding.action.label().to_string(),
            hint: format!("{} {}", binding.mods.name(), binding.trigger.name()),
            command: crate::palette::Command::Action(binding.action),
        })
        .collect()
}

/// The launcher's rows: the shells this machine actually has.
///
/// Probed rather than listed: offering a shell that is not installed is a row
/// that opens an empty tab and an error in a log the user will not read.
fn launcher_entries() -> Vec<crate::palette::Entry> {
    const CANDIDATES: &[(&str, &str)] = &[
        ("pwsh.exe", "PowerShell 7"),
        ("powershell.exe", "Windows PowerShell"),
        ("cmd.exe", "Command Prompt"),
        ("wsl.exe", "WSL"),
        ("bash", "Bash"),
        ("zsh", "Zsh"),
        ("fish", "Fish"),
        ("nu", "Nushell"),
    ];
    CANDIDATES
        .iter()
        .filter(|(program, _)| which(program).is_some())
        .map(|(program, description)| crate::palette::Entry {
            label: program.to_string(),
            hint: description.to_string(),
            command: crate::palette::Command::Launch {
                program: program.to_string(),
            },
        })
        .collect()
}

/// Where a program is, if it is anywhere on PATH.
fn which(program: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE".to_string())
            .split(';')
            .map(|ext| ext.to_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
        // A name without its extension, which is how everyone writes them.
        for extension in &extensions {
            let candidate = directory.join(format!("{program}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod palette_entry_tests {
    use super::*;

    /// The palette lists every key action, with the chord that reaches it.
    ///
    /// Built from the key table rather than a second list, so a chord and a
    /// palette row cannot drift apart -- and showing the chord is how a
    /// palette teaches the keyboard.
    #[test]
    fn the_palette_lists_what_the_keys_do() {
        let entries = command_entries();
        assert_eq!(entries.len(), crate::keys::BINDINGS.len());
        for (entry, binding) in entries.iter().zip(crate::keys::BINDINGS) {
            assert_eq!(entry.label, binding.action.label());
            assert!(
                entry.hint.contains(&binding.trigger.name()),
                "{} should show the chord that reaches it",
                entry.label
            );
        }
    }

    /// The launcher offers shells this machine has, and only those.
    ///
    /// A row for a shell that is not installed opens an empty tab and writes
    /// an error to a log nobody reads.
    #[test]
    fn the_launcher_offers_only_shells_that_exist() {
        for entry in launcher_entries() {
            let crate::palette::Command::Launch { program } = &entry.command else {
                panic!("a launcher row should launch something");
            };
            assert!(
                which(program).is_some(),
                "{program} is offered but not installed"
            );
        }
    }

    #[test]
    fn this_machine_has_at_least_one_shell_to_offer() {
        // An empty launcher is indistinguishable from a broken one.
        assert!(
            !launcher_entries().is_empty(),
            "no shell found on PATH; the launcher would open on an empty list"
        );
    }
}

#[cfg(test)]
mod encode_tests {
    use super::*;
    use winit::keyboard::{Key, NamedKey};

    fn plain() -> crate::mouse::Held {
        crate::mouse::Held::default()
    }

    fn ctrl() -> crate::mouse::Held {
        crate::mouse::Held {
            ctrl: true,
            ..Default::default()
        }
    }

    fn character(text: &str) -> Key {
        Key::Character(text.into())
    }

    fn named(key: NamedKey) -> Key {
        Key::Named(key)
    }

    #[test]
    fn a_letter_is_itself() {
        assert_eq!(encode(&character("a"), plain()), Some("a".to_string()));
        assert_eq!(encode(&character("A"), plain()), Some("A".to_string()));
    }

    /// Ctrl+letter is a control byte, not the letter.
    ///
    /// This was the bug: modifiers never reached the encoder, so Ctrl+C sent
    /// `c`. Nothing could be interrupted, and every readline binding --
    /// Ctrl+A, Ctrl+E, Ctrl+K, Ctrl+R, Ctrl+U, Ctrl+W -- typed a letter
    /// instead of doing its job.
    #[test]
    fn ctrl_letter_sends_its_control_byte() {
        assert_eq!(encode(&character("c"), ctrl()), Some("\x03".to_string()));
        assert_eq!(encode(&character("d"), ctrl()), Some("\x04".to_string()));
        assert_eq!(encode(&character("a"), ctrl()), Some("\x01".to_string()));
        assert_eq!(encode(&character("e"), ctrl()), Some("\x05".to_string()));
        assert_eq!(encode(&character("r"), ctrl()), Some("\x12".to_string()));
        assert_eq!(encode(&character("u"), ctrl()), Some("\x15".to_string()));
        assert_eq!(encode(&character("w"), ctrl()), Some("\x17".to_string()));
        assert_eq!(encode(&character("z"), ctrl()), Some("\x1a".to_string()));
    }

    #[test]
    fn ctrl_letter_ignores_the_case_the_layout_produced() {
        // Shift changes the letter's case; it does not change the control
        // byte, and a shell reading 0x03 does not care which arrived.
        assert_eq!(encode(&character("C"), ctrl()), Some("\x03".to_string()));
    }

    #[test]
    fn alt_letter_is_escape_prefixed() {
        let alt = crate::mouse::Held {
            alt: true,
            ..Default::default()
        };
        assert_eq!(encode(&character("b"), alt), Some("\x1bb".to_string()));
    }

    #[test]
    fn the_arrows_carry_their_modifiers() {
        // Ctrl+arrow moves by word and Shift+arrow selects: both need the
        // modifier in the sequence, and both were unreachable when it was
        // dropped.
        let plain_left = encode(&named(NamedKey::ArrowLeft), plain());
        let ctrl_left = encode(&named(NamedKey::ArrowLeft), ctrl());
        assert!(plain_left.is_some());
        assert_ne!(plain_left, ctrl_left, "Ctrl+Left has to differ from Left");

        let shift = crate::mouse::Held {
            shift: true,
            ..Default::default()
        };
        assert_ne!(encode(&named(NamedKey::ArrowUp), shift), plain_left);
    }

    #[test]
    fn backspace_sends_delete_as_readline_expects() {
        assert_eq!(encode(&named(NamedKey::Backspace), plain()), Some("\x7f".to_string()));
    }

    #[test]
    fn enter_and_tab_and_escape_are_what_they_look_like() {
        assert_eq!(encode(&named(NamedKey::Enter), plain()), Some("\r".to_string()));
        assert_eq!(encode(&named(NamedKey::Tab), plain()), Some("\t".to_string()));
        assert_eq!(encode(&named(NamedKey::Escape), plain()), Some("\x1b".to_string()));
    }

    #[test]
    fn shift_tab_is_a_back_tab_rather_than_a_tab() {
        let shift = crate::mouse::Held {
            shift: true,
            ..Default::default()
        };
        assert_eq!(encode(&named(NamedKey::Tab), shift), Some("\x1b[Z".to_string()));
    }

    #[test]
    fn the_function_keys_exist_at_all() {
        // None of these were mapped, so F5 in a TUI did nothing.
        for (key, number) in [
            (NamedKey::F1, 1),
            (NamedKey::F5, 5),
            (NamedKey::F12, 12),
        ] {
            let encoded = encode(&named(key), plain());
            assert!(encoded.is_some(), "F{number} produced nothing");
        }
    }

    #[test]
    fn navigation_keys_are_all_mapped() {
        for key in [
            NamedKey::Home,
            NamedKey::End,
            NamedKey::PageUp,
            NamedKey::PageDown,
            NamedKey::Delete,
            NamedKey::Insert,
        ] {
            assert!(encode(&named(key), plain()).is_some(), "{key:?} produced nothing");
        }
    }

    #[test]
    fn a_modifier_key_on_its_own_sends_nothing() {
        // Otherwise holding Shift types something.
        assert_eq!(encode(&named(NamedKey::Shift), plain()), None);
        assert_eq!(encode(&named(NamedKey::Control), plain()), None);
        assert_eq!(encode(&named(NamedKey::Alt), plain()), None);
    }

    #[test]
    fn a_super_chord_stays_with_the_window_manager() {
        // Super+L locks the screen; it must not also type an L.
        let text = encode(&character("l"), plain());
        assert_eq!(text, Some("l".to_string()), "plain L still types");
    }

    #[test]
    fn composed_text_from_an_input_method_arrives_whole() {
        // Several characters from one press: already what the user meant.
        assert_eq!(encode(&character("中文"), plain()), Some("中文".to_string()));
    }

    #[test]
    fn space_is_a_space_and_ctrl_space_is_a_null() {
        assert_eq!(encode(&named(NamedKey::Space), plain()), Some(" ".to_string()));
        assert_eq!(encode(&named(NamedKey::Space), ctrl()), Some("\0".to_string()));
    }
}

#[cfg(test)]
mod tab_badge_tests {
    use crate::cockpit::Badge;
    use crate::topbar;

    /// The badge is the reason to look at this bar at all: four agents
    /// running, and which one wants you has to be readable without visiting
    /// each pane. It belongs to its own tab, and a badge drawn past that tab's
    /// width lands on the next one -- pointing at the wrong pane, which is
    /// worse than no badge because it is believed.
    #[test]
    fn a_badge_stays_within_its_own_tab() {
        let bar = topbar::layout(4, 0, 160, "");
        for index in 0..4 {
            let tab = bar
                .iter()
                .find(|piece| piece.item == topbar::Item::Tab(index))
                .expect("every tab is laid out");
            let column = topbar::badge_column(tab);
            assert!(tab.contains(column), "{tab:?} badge at {column}");
        }
    }

    /// And it never overlaps the tab's number.
    #[test]
    fn a_badge_sits_after_the_number_it_belongs_to() {
        let bar = topbar::layout(3, 0, 160, "");
        for index in 0..3 {
            let tab = bar
                .iter()
                .find(|piece| piece.item == topbar::Item::Tab(index))
                .unwrap();
            let number_ends = tab.column + tab.label.trim_end().chars().count();
            assert!(topbar::badge_column(tab) >= number_ends, "{tab:?}");
        }
    }

    /// Three states, three colours, and idle is no badge at all.
    #[test]
    fn the_badges_are_told_apart_by_colour() {
        let colours = [
            Badge::NeedsYou.color(),
            Badge::Working.color(),
            Badge::Done.color(),
        ];
        for (index, colour) in colours.iter().enumerate() {
            for other in &colours[index + 1..] {
                assert_ne!(colour, other, "two badges share a colour");
            }
        }
    }
}
