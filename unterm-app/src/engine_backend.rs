//! Which engine this window drives.
//!
//! `Local` is the pre-M1 arrangement: sessions live in this process,
//! inside `unterm-engine`'s global runtime. `Core` is where issue #12
//! is taking the product: sessions live in the per-user `unterm-core`
//! process and survive this window closing.
//!
//! `Core` is opt-in via `UNTERM_CORE_CLIENT=1` while M1-04 lands
//! piecewise. Known gaps behind the flag, tracked in the development
//! plan: the in-GUI MCP server and the statsbar/cockpit refresh
//! threads still address the process-local engine, and the scrollback
//! size global set at startup does not reach the Core process.

use anyhow::Result;
use unterm_core::{CoreEngineClient, FrameCache};
use unterm_engine::next_core::mouse_encoding::MouseEvent;
use unterm_engine::next_core::NextCoreEngine;
use unterm_engine::{
    CaptureEngine, CreateSessionRequest, CursorSnapshot, EngineHealthSnapshot, HealthEngine,
    HostEngine, InputEngine, PaneLocation, PaneModesSnapshot, RecordingEngine,
    RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot, RecordingStopResult,
    RenderFrameSnapshot, ScreenEngine, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, SearchMode, SessionActivitySnapshot,
    SessionEngine, SessionSnapshot, ShellSnapshot, SplitSessionRequest, StyledScreenSnapshot,
    StyledScrollbackSnapshot, ViewportScrollResult, WindowEngine, WindowFocusResult,
};

pub enum AppEngine {
    Local(NextCoreEngine),
    Core {
        client: std::sync::Arc<CoreEngineClient>,
        /// Serves every styled read from local memory. The benchmark
        /// that mandates this: a full styled screen over IPC costs
        /// ~5ms and this window reads 20+ per frame.
        cache: FrameCache,
    },
}

/// The one Core connection this process shares between the window, the
/// MCP surface and background threads. Set once by
/// `init_from_environment` before the MCP server starts; never set in
/// Local mode.
struct CoreShared {
    client: std::sync::Arc<CoreEngineClient>,
    endpoint: String,
    token: String,
}

static CORE_SHARED: std::sync::OnceLock<CoreShared> = std::sync::OnceLock::new();

/// Decide the engine backend for this whole process, and install the
/// matching MCP engine provider. Must run before the MCP server
/// starts: the provider slot is set-once, and an MCP surface that
/// came up on the local engine while the window talks to a Core
/// would split the world in two.
pub fn init_from_environment() {
    if std::env::var("UNTERM_CORE_CLIENT").is_ok_and(|value| value == "1") {
        match connect_core_shared() {
            Ok(()) => {
                unterm_engine::set_engine_provider(|| Box::new(CoreHostEngine));
                eprintln!("unterm: UNTERM_CORE_CLIENT=1, sessions live in unterm-core");
                return;
            }
            Err(err) => {
                eprintln!(
                    "unterm: UNTERM_CORE_CLIENT=1 but the core is unavailable ({err:#}); \
                     using the in-process engine"
                );
            }
        }
    }
    unterm_engine::install_next_core_provider();
}

fn connect_core_shared() -> Result<()> {
    let info = unterm_core::ensure_running()?;
    let client = CoreEngineClient::connect(&info.endpoint, info.token.clone())?;
    // The config file is this process's to read; the Core just
    // applies whatever the connecting client was configured with.
    // main() stored the value in the local engine global before any
    // window opened, so it is current here.
    client.set_new_session_scrollback_lines(NextCoreEngine::new_session_scrollback_lines())?;
    CORE_SHARED
        .set(CoreShared {
            client: std::sync::Arc::new(client),
            endpoint: info.endpoint,
            token: info.token,
        })
        .map_err(|_| anyhow::anyhow!("core backend initialized twice"))
}

fn core_client() -> &'static std::sync::Arc<CoreEngineClient> {
    &CORE_SHARED
        .get()
        .expect("CoreHostEngine used before init_from_environment connected the core")
        .client
}

/// What the MCP surface drives in Core mode: sessions and screens from
/// the Core process, window and capture questions from this front end
/// (those never left this process -- the window lives here).
pub struct CoreHostEngine;

/// Route a call to whichever engine is live. Trait methods on both
/// sides share names and signatures, so the arms stay symmetrical.
macro_rules! route {
    ($self:ident, $engine:ident => $call:expr) => {
        match $self {
            AppEngine::Local($engine) => $call,
            AppEngine::Core { client: $engine, .. } => $call,
        }
    };
}

impl AppEngine {
    /// The backend `init_from_environment` decided on. Local is the
    /// default; a Core that connected then but cannot start a frame
    /// cache now falls back loudly rather than refusing to open a
    /// terminal -- the flag is experimental, the user's shell is not.
    pub fn from_environment() -> Self {
        if let Some(shared) = CORE_SHARED.get() {
            // The cache's update thread wakes the event loop, so a
            // screen change becomes a redraw now rather than at the
            // next timer tick -- the Core-mode replacement for the
            // engine sharing this process's memory.
            match FrameCache::start_with_notify(
                shared.endpoint.as_str(),
                shared.token.clone(),
                crate::mcp_host::request_repaint,
            ) {
                Ok(cache) => {
                    return AppEngine::Core {
                        client: shared.client.clone(),
                        cache,
                    }
                }
                Err(err) => {
                    eprintln!(
                        "unterm: core frame cache unavailable ({err:#}); \
                         window falls back to the in-process engine"
                    );
                }
            }
        }
        AppEngine::Local(NextCoreEngine)
    }

    pub fn pane_modes(&self, pane_id: usize) -> Result<PaneModesSnapshot> {
        route!(self, engine => engine.pane_modes(pane_id))
    }

    pub fn screen_revision(&self, pane_id: usize) -> Result<u64> {
        route!(self, engine => engine.screen_revision(pane_id))
    }

    pub fn scroll_viewport_to(&self, pane_id: usize, target: isize) -> Result<()> {
        route!(self, engine => engine.scroll_viewport_to(pane_id, target))
    }

    pub fn scroll_viewport_by(&self, pane_id: usize, delta: isize) -> Result<()> {
        route!(self, engine => engine.scroll_viewport_by(pane_id, delta))
    }

    pub fn scroll_viewport_to_prompt(&self, pane_id: usize, amount: isize) -> Result<()> {
        route!(self, engine => engine.scroll_viewport_to_prompt(pane_id, amount))
    }

    pub fn report_mouse(&self, pane_id: usize, event: MouseEvent) -> Result<()> {
        route!(self, engine => engine.report_mouse(pane_id, event))
    }
}

impl SessionEngine for AppEngine {
    fn list_sessions(&self) -> Result<Vec<SessionSnapshot>> {
        route!(self, engine => engine.list_sessions())
    }

    fn get_session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        route!(self, engine => engine.get_session(pane_id))
    }

    fn create_session(&self, request: CreateSessionRequest) -> Result<SessionSnapshot> {
        route!(self, engine => engine.create_session(request))
    }

    fn split_session(&self, request: SplitSessionRequest) -> Result<SessionSnapshot> {
        route!(self, engine => engine.split_session(request))
    }

    fn focus_session(&self, pane_id: usize) -> Result<()> {
        route!(self, engine => engine.focus_session(pane_id))
    }

    fn shell(&self, pane_id: usize) -> Result<ShellSnapshot> {
        route!(self, engine => engine.shell(pane_id))
    }

    fn activity(&self, pane_id: usize) -> Result<SessionActivitySnapshot> {
        route!(self, engine => engine.activity(pane_id))
    }

    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> Result<()> {
        route!(self, engine => engine.resize_session(pane_id, cols, rows))
    }

    fn destroy_session(&self, pane_id: usize) -> Result<()> {
        route!(self, engine => engine.destroy_session(pane_id))
    }
}

impl ScreenEngine for AppEngine {
    fn read_screen(&self, pane_id: usize) -> Result<ScreenSnapshot> {
        route!(self, engine => engine.read_screen(pane_id))
    }

    fn erase_scrollback(&self, pane_id: usize, include_viewport: bool) -> Result<()> {
        route!(self, engine => engine.erase_scrollback(pane_id, include_viewport))
    }

    /// The hottest call in the window. In Core mode it must stay off
    /// the wire: cache first, one direct fetch only on a genuine miss
    /// (a pane the cache has not seen yet).
    fn read_styled_screen(&self, pane_id: usize) -> Result<StyledScreenSnapshot> {
        match self {
            Self::Local(engine) => engine.read_styled_screen(pane_id),
            Self::Core { client, cache } => match cache.styled_screen(pane_id) {
                Some(screen) => Ok(screen),
                None => client.read_styled_screen(pane_id),
            },
        }
    }

    fn read_render_frame(
        &self,
        pane_id: usize,
        since_revision: Option<u64>,
    ) -> Result<RenderFrameSnapshot> {
        route!(self, engine => engine.read_render_frame(pane_id, since_revision))
    }

    fn read_visible_text(&self, pane_id: usize) -> Result<String> {
        route!(self, engine => engine.read_visible_text(pane_id))
    }

    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
        route!(self, engine => engine.read_lines(pane_id, start, count))
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>> {
        route!(self, engine => engine.read_scrollback(pane_id, limit))
    }

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot> {
        route!(self, engine => engine.read_scrollback_text(pane_id, request))
    }

    fn read_styled_scrollback(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<StyledScrollbackSnapshot> {
        route!(self, engine => engine.read_styled_scrollback(pane_id, request))
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        mode: SearchMode,
        max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        route!(self, engine => engine.search(pane_id, pattern, mode, max_results))
    }

    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        route!(self, engine => engine.cursor(pane_id))
    }
}

impl InputEngine for AppEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()> {
        route!(self, engine => engine.write_input(pane_id, input))
    }

    fn paste_input(&self, pane_id: usize, text: &str) -> Result<()> {
        route!(self, engine => engine.paste_input(pane_id, text))
    }
}

impl RecordingEngine for AppEngine {
    fn start_recording(&self, pane_id: usize) -> Result<RecordingStartResult> {
        route!(self, engine => engine.start_recording(pane_id))
    }

    fn stop_recording(&self, pane_id: usize) -> Result<RecordingStopResult> {
        route!(self, engine => engine.stop_recording(pane_id))
    }

    fn recording_status(&self, pane_id: usize) -> Result<RecordingStatusSnapshot> {
        route!(self, engine => engine.recording_status(pane_id))
    }

    fn attach_recording_trace(&self, pane_id: usize, trace_id: String) -> Result<Vec<String>> {
        route!(self, engine => engine.attach_recording_trace(pane_id, trace_id))
    }

    fn export_markdown(
        &self,
        pane_id: usize,
        target_path: Option<String>,
    ) -> Result<RecordingExportResult> {
        route!(self, engine => engine.export_markdown(pane_id, target_path))
    }
}

impl HealthEngine for AppEngine {
    fn health(&self) -> Result<EngineHealthSnapshot> {
        route!(self, engine => engine.health())
    }
}

impl SessionEngine for CoreHostEngine {
    fn list_sessions(&self) -> Result<Vec<SessionSnapshot>> {
        core_client().list_sessions()
    }

    fn get_session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        core_client().get_session(pane_id)
    }

    fn create_session(&self, request: CreateSessionRequest) -> Result<SessionSnapshot> {
        core_client().create_session(request)
    }

    fn split_session(&self, request: SplitSessionRequest) -> Result<SessionSnapshot> {
        core_client().split_session(request)
    }

    fn focus_session(&self, pane_id: usize) -> Result<()> {
        core_client().focus_session(pane_id)
    }

    fn shell(&self, pane_id: usize) -> Result<ShellSnapshot> {
        core_client().shell(pane_id)
    }

    fn activity(&self, pane_id: usize) -> Result<SessionActivitySnapshot> {
        core_client().activity(pane_id)
    }

    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> Result<()> {
        core_client().resize_session(pane_id, cols, rows)
    }

    fn destroy_session(&self, pane_id: usize) -> Result<()> {
        core_client().destroy_session(pane_id)
    }
}

impl ScreenEngine for CoreHostEngine {
    fn read_screen(&self, pane_id: usize) -> Result<ScreenSnapshot> {
        core_client().read_screen(pane_id)
    }

    fn erase_scrollback(&self, pane_id: usize, include_viewport: bool) -> Result<()> {
        core_client().erase_scrollback(pane_id, include_viewport)
    }

    fn read_styled_screen(&self, pane_id: usize) -> Result<StyledScreenSnapshot> {
        core_client().read_styled_screen(pane_id)
    }

    fn read_render_frame(
        &self,
        pane_id: usize,
        since_revision: Option<u64>,
    ) -> Result<RenderFrameSnapshot> {
        core_client().read_render_frame(pane_id, since_revision)
    }

    fn read_visible_text(&self, pane_id: usize) -> Result<String> {
        core_client().read_visible_text(pane_id)
    }

    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
        core_client().read_lines(pane_id, start, count)
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>> {
        core_client().read_scrollback(pane_id, limit)
    }

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot> {
        core_client().read_scrollback_text(pane_id, request)
    }

    fn read_styled_scrollback(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<StyledScrollbackSnapshot> {
        core_client().read_styled_scrollback(pane_id, request)
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        mode: SearchMode,
        max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        core_client().search(pane_id, pattern, mode, max_results)
    }

    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        core_client().cursor(pane_id)
    }
}

impl InputEngine for CoreHostEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()> {
        core_client().write_input(pane_id, input)
    }

    fn paste_input(&self, pane_id: usize, text: &str) -> Result<()> {
        core_client().paste_input(pane_id, text)
    }
}

impl RecordingEngine for CoreHostEngine {
    fn start_recording(&self, pane_id: usize) -> Result<RecordingStartResult> {
        core_client().start_recording(pane_id)
    }

    fn stop_recording(&self, pane_id: usize) -> Result<RecordingStopResult> {
        core_client().stop_recording(pane_id)
    }

    fn recording_status(&self, pane_id: usize) -> Result<RecordingStatusSnapshot> {
        core_client().recording_status(pane_id)
    }

    fn attach_recording_trace(&self, pane_id: usize, trace_id: String) -> Result<Vec<String>> {
        core_client().attach_recording_trace(pane_id, trace_id)
    }

    fn export_markdown(
        &self,
        pane_id: usize,
        target_path: Option<String>,
    ) -> Result<RecordingExportResult> {
        core_client().export_markdown(pane_id, target_path)
    }
}

impl HealthEngine for CoreHostEngine {
    fn health(&self) -> Result<EngineHealthSnapshot> {
        core_client().health()
    }
}

// Window and capture questions never left this process: the window
// lives here, and next-core's impls of these already route to the
// front end via `mcp_host()` without touching session state.
impl WindowEngine for CoreHostEngine {
    fn focus_current_instance_window(&self) -> Result<WindowFocusResult> {
        WindowEngine::focus_current_instance_window(&NextCoreEngine)
    }

    fn active_pane_id(&self) -> Result<Option<u64>> {
        WindowEngine::active_pane_id(&NextCoreEngine)
    }

    fn pane_locations(&self) -> Result<std::collections::HashMap<u64, PaneLocation>> {
        WindowEngine::pane_locations(&NextCoreEngine)
    }

    fn scroll_viewport_to(&self, pane_id: usize, target: isize) -> Result<ViewportScrollResult> {
        WindowEngine::scroll_viewport_to(&NextCoreEngine, pane_id, target)
    }
}

impl CaptureEngine for CoreHostEngine {
    fn capture_screen_image(&self, include_base64: bool) -> Result<serde_json::Value> {
        CaptureEngine::capture_screen_image(&NextCoreEngine, include_base64)
    }

    fn capture_window_image(
        &self,
        title_filter: Option<&str>,
        pid_filter: Option<u32>,
        include_base64: bool,
    ) -> Result<serde_json::Value> {
        CaptureEngine::capture_window_image(&NextCoreEngine, title_filter, pid_filter, include_base64)
    }

    fn capture_region_image(
        &self,
        left: i32,
        top: i32,
        width: usize,
        height: usize,
        include_base64: bool,
    ) -> Result<serde_json::Value> {
        CaptureEngine::capture_region_image(&NextCoreEngine, left, top, width, height, include_base64)
    }
}

impl HostEngine for CoreHostEngine {
    fn name(&self) -> &'static str {
        "unterm-core"
    }
}
