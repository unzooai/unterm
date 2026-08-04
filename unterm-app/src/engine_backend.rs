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
    CreateSessionRequest, CursorSnapshot, EngineHealthSnapshot, HealthEngine, InputEngine,
    PaneModesSnapshot, RecordingEngine, RecordingExportResult, RecordingStartResult,
    RecordingStatusSnapshot, RecordingStopResult, RenderFrameSnapshot, ScreenEngine, ScreenLine,
    ScreenSearchMatch, ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot, SearchMode,
    SessionActivitySnapshot, SessionEngine, SessionSnapshot, ShellSnapshot, SplitSessionRequest,
    StyledScreenSnapshot, StyledScrollbackSnapshot,
};

pub enum AppEngine {
    Local(NextCoreEngine),
    Core {
        client: CoreEngineClient,
        /// Serves every styled read from local memory. The benchmark
        /// that mandates this: a full styled screen over IPC costs
        /// ~5ms and this window reads 20+ per frame.
        cache: FrameCache,
    },
}

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
    /// The default is Local until the Core path has carried M1-04 to
    /// the gate. A requested-but-unreachable Core falls back to Local
    /// with a note instead of refusing to open a terminal: the flag
    /// is experimental, the user's shell is not.
    pub fn from_environment() -> Self {
        if std::env::var("UNTERM_CORE_CLIENT").is_ok_and(|value| value == "1") {
            match Self::connect_core() {
                Ok(engine) => {
                    eprintln!("unterm: UNTERM_CORE_CLIENT=1, sessions live in unterm-core");
                    return engine;
                }
                Err(err) => {
                    eprintln!(
                        "unterm: UNTERM_CORE_CLIENT=1 but the core is unavailable ({err:#}); \
                         using the in-process engine"
                    );
                }
            }
        }
        AppEngine::Local(NextCoreEngine)
    }

    fn connect_core() -> Result<Self> {
        let info = unterm_core::ensure_running()?;
        let client = CoreEngineClient::connect(&info.endpoint, info.token.clone())?;
        let cache = FrameCache::start(&info.endpoint, info.token)?;
        Ok(AppEngine::Core { client, cache })
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
