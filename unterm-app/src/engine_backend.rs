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
pub fn init_from_environment() -> Backend {
    // Core by default: the agent surface lives there now, so sessions
    // survive closing the window and an agent's connection survives
    // with them. `UNTERM_CORE_CLIENT=0` opts back out for anyone who
    // needs the old single-process arrangement.
    //
    // Known wart, cosmetic: reopening onto a Core that kept its
    // sessions puts the adopted-active session in the first tab, so tab
    // *order* is not preserved across a restart even though every
    // session, its scrollback and its splits are. The focused pane and
    // its contents are correct either way.
    let wants_core = !std::env::var("UNTERM_CORE_CLIENT").is_ok_and(|value| value == "0");
    if wants_core {
        match connect_core_shared() {
            Ok(()) => {
                unterm_engine::set_engine_provider(|| Box::new(CoreHostEngine));
                return Backend::Core;
            }
            Err(err) => {
                // Falling back rather than refusing to start: a user
                // whose Core will not come up still gets a terminal,
                // and the one thing they lose is said out loud.
                eprintln!(
                    "unterm: unterm-core is unavailable ({err:#}); this window keeps its \
                     sessions in-process, and they will not outlive it"
                );
            }
        }
    }
    unterm_engine::install_next_core_provider();
    Backend::Local
}

/// Which arrangement `init_from_environment` settled on.
///
/// `main` needs it to decide whether to start an MCP server of its own:
/// in Core mode the Core is already serving one, and a second would give
/// agents two different answers about the same sessions.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Local,
    Core,
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

/// Held for the process's lifetime once attached; dropping it would tell
/// the Core this window is gone.
static HOST_CHANNEL: std::sync::OnceLock<unterm_core::HostChannelClient> =
    std::sync::OnceLock::new();

/// Offer this window to the Core as the front end it can call back into.
///
/// Separate from `init_from_environment` and called later on purpose:
/// the channel is only useful once there is a window to answer with, and
/// `AppMcpHost` reads a window handle that `main` has not filled in yet
/// when the backend is chosen.
///
/// Failure is not fatal. A window that cannot attach simply leaves the
/// Core headless, which is a state the whole surface already handles --
/// worth a log line, not worth refusing to start.
pub fn attach_host_channel() {
    let Some(shared) = CORE_SHARED.get() else {
        return;
    };
    match unterm_core::HostChannelClient::attach(
        &shared.endpoint,
        shared.token.clone(),
        std::sync::Arc::new(crate::mcp_host::AppHostResponder),
    ) {
        Ok(channel) => {
            let _ = HOST_CHANNEL.set(channel);
            eprintln!("unterm: attached to unterm-core as its front end");
        }
        Err(err) => {
            eprintln!("unterm: could not attach to unterm-core ({err:#}); it stays headless");
        }
    }
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

    /// Whether the process holding the sessions is still there.
    ///
    /// Always true in Local mode -- the engine is this process, and if
    /// it were gone there would be nobody to ask. In Core mode it goes
    /// false when the Core dies, which is the difference between a
    /// terminal that has nothing new to show and one that will never
    /// show anything again.
    pub fn sessions_reachable(&self) -> bool {
        match self {
            Self::Local(_) => true,
            Self::Core { cache, .. } => cache.is_live(),
        }
    }

    /// How many times the sessions behind this window have been replaced
    /// wholesale.
    ///
    /// Only a Core-backed window can see this change: it counts the
    /// Cores this process has outlived. A holder of pane ids that sees a
    /// new value is holding ids from a process that no longer exists.
    /// In Local mode the engine dies with the window, so it is always 0.
    pub fn session_epoch(&self) -> u64 {
        match self {
            Self::Local(_) => 0,
            Self::Core { cache, .. } => cache.epoch(),
        }
    }

    /// Whether the sessions outlive this window.
    ///
    /// What the close prompt hangs on: only a Core-backed window can
    /// honestly offer to keep running in the background, because only
    /// there do the shells belong to another process.
    pub fn sessions_outlive_this_window(&self) -> bool {
        matches!(self, Self::Core { .. })
    }

    /// Ask the Core to refuse new sessions, and optionally to stop
    /// once the running ones have ended. A no-op in Local mode, where
    /// there is no Core to outlive anything.
    pub fn drain_core(&self, exit_when_idle: bool) -> Result<()> {
        match self {
            Self::Local(_) => Ok(()),
            Self::Core { client, .. } => client.drain(exit_when_idle),
        }
    }

    /// Stop the Core now, ending every session it holds.
    pub fn shutdown_core(&self) -> Result<()> {
        match self {
            Self::Local(_) => Ok(()),
            Self::Core { client, .. } => client.shutdown(),
        }
    }

    pub fn pane_modes(&self, pane_id: usize) -> Result<PaneModesSnapshot> {
        route!(self, engine => engine.pane_modes(pane_id))
    }

    /// One number that changes whenever anything drawn has.
    ///
    /// The idle loop asks this on every tick. In Local mode that is a
    /// revision read per pane against process memory -- cheap. Over
    /// IPC it would be a round trip per pane per tick, twenty of them
    /// on a twenty-pane window, purely to ask whether anything moved.
    /// The cache already knows, because the Core told it: answering
    /// from its counter is one atomic load and no wire at all, and it
    /// is what makes the event-driven path actually pay.
    pub fn render_generation(&self, panes: &[usize]) -> u64 {
        match self {
            Self::Local(engine) => panes
                .iter()
                .filter_map(|pane| engine.screen_revision(*pane).ok())
                .fold(0u64, |sum, value| sum.wrapping_add(value)),
            Self::Core { cache, .. } => cache.generation(),
        }
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

/// The agent-facing state this window draws but does not own.
///
/// Suggestions, the Insights numbers and the audit trail all live
/// wherever the MCP surface does. That used to be this process always;
/// with the surface in a Core it is over there, and a window reading its
/// own empty copy would draw an empty Inbox next to a Core full of work.
///
/// Named exactly like the `unterm_mcp::handler` functions they stand in
/// for, so the call sites read the same and the only thing that changed
/// is which process answers.
pub mod mcp_state {
    use unterm_mcp::handler::{InsightsMcpSnapshot, Suggestion};

    pub fn pending_suggestions_for_pane(pane_id: u64) -> Vec<Suggestion> {
        match super::CORE_SHARED.get() {
            // A Core that cannot answer is reported as "nothing
            // pending" rather than as an error: this is drawn every
            // frame, and a failed fetch must not become a dialog.
            Some(shared) => shared.client.pending_suggestions(pane_id).unwrap_or_default(),
            None => unterm_mcp::handler::pending_suggestions_for_pane(pane_id),
        }
    }

    pub fn accept_suggestion(id: &str, run_immediately: bool) -> Result<String, String> {
        match super::CORE_SHARED.get() {
            Some(shared) => shared
                .client
                .accept_suggestion(id, run_immediately)
                .map_err(|err| format!("{err:#}")),
            None => unterm_mcp::handler::accept_suggestion(id, run_immediately),
        }
    }

    pub fn dismiss_suggestion(id: &str) -> Result<(), String> {
        match super::CORE_SHARED.get() {
            Some(shared) => shared
                .client
                .dismiss_suggestion(id)
                .map_err(|err| format!("{err:#}")),
            None => unterm_mcp::handler::dismiss_suggestion(id),
        }
    }

    pub fn insights_mcp_snapshot(recent_audit_limit: usize) -> InsightsMcpSnapshot {
        match super::CORE_SHARED.get() {
            Some(shared) => shared.client.insights(recent_audit_limit).unwrap_or_default(),
            None => unterm_mcp::handler::insights_mcp_snapshot(recent_audit_limit),
        }
    }

    pub fn audit_gui_write(method: &str, pane_id: u64, detail: &str) {
        match super::CORE_SHARED.get() {
            // Best-effort, as it always was: an audit line that cannot
            // be written must not stop the action it describes.
            Some(shared) => {
                let _ = shared.client.audit_gui_write(method, pane_id, detail);
            }
            None => unterm_mcp::handler::audit_gui_write(method, pane_id, detail),
        }
    }
}
