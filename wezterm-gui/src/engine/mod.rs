//! GUI adapter for the engine-neutral terminal layer.
//!
//! The neutral traits and next-core implementation live in `unterm-engine`.
//! This module keeps the current WezTerm adapter available to GUI callers while
//! letting product services migrate away from WezTerm internals.

pub mod wezterm;

use std::path::Path;
use window::WindowOps;

#[allow(unused_imports)]
pub use unterm_engine::{
    next_core, CellStyle, CreateSessionRequest, CursorSnapshot, DirtyRows, EngineHealthSnapshot,
    HealthEngine, InputEngine, PaneDimensions, RecordingEngine, RecordingExportResult,
    RecordingStartResult, RecordingStatusSnapshot, RecordingStopResult, ScreenEngine, ScreenLine,
    ScreenSearchMatch, ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot,
    SessionActivitySnapshot, SessionEngine, SessionSnapshot, ShellSnapshot, SplitDirection,
    SplitSessionRequest, StyledCell, StyledColor, StyledScreenLine, StyledScreenSnapshot,
    TerminalEngine,
};

#[derive(Clone, Debug)]
pub struct WindowFocusResult {
    pub mux_window_id: usize,
}

pub trait WindowEngine {
    fn focus_current_instance_window(&self) -> anyhow::Result<WindowFocusResult>;
}

pub struct RenderedScrollbackPng {
    pub image: crate::scrollshot::ScrollbackPng,
    pub session_id: usize,
}

pub trait CaptureEngine {
    fn render_scrollback_png(
        &self,
        pane_id: Option<usize>,
        path: &Path,
        opts: &crate::scrollshot::ScrollbackPngOptions,
    ) -> anyhow::Result<RenderedScrollbackPng>;
}

#[derive(Clone, Copy, Debug)]
pub enum CurrentTerminalEngine {
    WezTerm(wezterm::WezTermEngine),
    NextCore(unterm_engine::next_core::NextCoreEngine),
}

fn selected_engine_name_from_env(value: Option<&str>) -> &'static str {
    match value.map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("next-core") => "next-core",
        _ => "wezterm",
    }
}

pub fn selected_engine_name() -> &'static str {
    selected_engine_name_from_env(std::env::var("UNTERM_ENGINE").ok().as_deref())
}

pub fn current() -> CurrentTerminalEngine {
    match selected_engine_name() {
        "next-core" => CurrentTerminalEngine::NextCore(next_core()),
        _ => CurrentTerminalEngine::WezTerm(wezterm::WezTermEngine),
    }
}

impl CurrentTerminalEngine {
    pub fn name(&self) -> &'static str {
        match self {
            Self::WezTerm(_) => "wezterm",
            Self::NextCore(_) => "next-core",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::selected_engine_name_from_env;

    #[test]
    fn selects_wezterm_by_default() {
        assert_eq!(selected_engine_name_from_env(None), "wezterm");
        assert_eq!(selected_engine_name_from_env(Some("")), "wezterm");
        assert_eq!(selected_engine_name_from_env(Some("wezterm")), "wezterm");
    }

    #[test]
    fn selects_next_core_from_env() {
        assert_eq!(
            selected_engine_name_from_env(Some("next-core")),
            "next-core"
        );
        assert_eq!(
            selected_engine_name_from_env(Some("NEXT-CORE")),
            "next-core"
        );
        assert_eq!(
            selected_engine_name_from_env(Some(" next-core ")),
            "next-core"
        );
    }
}

impl SessionEngine for CurrentTerminalEngine {
    fn list_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>> {
        match self {
            Self::WezTerm(engine) => engine.list_sessions(),
            Self::NextCore(engine) => engine.list_sessions(),
        }
    }

    fn get_session(&self, pane_id: usize) -> anyhow::Result<SessionSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.get_session(pane_id),
            Self::NextCore(engine) => engine.get_session(pane_id),
        }
    }

    fn create_session(&self, request: CreateSessionRequest) -> anyhow::Result<SessionSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.create_session(request),
            Self::NextCore(engine) => engine.create_session(request),
        }
    }

    fn split_session(&self, request: SplitSessionRequest) -> anyhow::Result<SessionSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.split_session(request),
            Self::NextCore(engine) => engine.split_session(request),
        }
    }

    fn focus_session(&self, pane_id: usize) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.focus_session(pane_id),
            Self::NextCore(engine) => engine.focus_session(pane_id),
        }
    }

    fn shell(&self, pane_id: usize) -> anyhow::Result<ShellSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.shell(pane_id),
            Self::NextCore(engine) => engine.shell(pane_id),
        }
    }

    fn activity(&self, pane_id: usize) -> anyhow::Result<SessionActivitySnapshot> {
        match self {
            Self::WezTerm(engine) => engine.activity(pane_id),
            Self::NextCore(engine) => engine.activity(pane_id),
        }
    }

    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.resize_session(pane_id, cols, rows),
            Self::NextCore(engine) => engine.resize_session(pane_id, cols, rows),
        }
    }

    fn destroy_session(&self, pane_id: usize) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.destroy_session(pane_id),
            Self::NextCore(engine) => engine.destroy_session(pane_id),
        }
    }
}

impl ScreenEngine for CurrentTerminalEngine {
    fn read_screen(&self, pane_id: usize) -> anyhow::Result<ScreenSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.read_screen(pane_id),
            Self::NextCore(engine) => engine.read_screen(pane_id),
        }
    }

    fn read_styled_screen(&self, pane_id: usize) -> anyhow::Result<StyledScreenSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.read_styled_screen(pane_id),
            Self::NextCore(engine) => engine.read_styled_screen(pane_id),
        }
    }

    fn read_visible_text(&self, pane_id: usize) -> anyhow::Result<String> {
        match self {
            Self::WezTerm(engine) => engine.read_visible_text(pane_id),
            Self::NextCore(engine) => engine.read_visible_text(pane_id),
        }
    }

    fn read_lines(
        &self,
        pane_id: usize,
        start: i64,
        count: usize,
    ) -> anyhow::Result<Vec<ScreenLine>> {
        match self {
            Self::WezTerm(engine) => engine.read_lines(pane_id, start, count),
            Self::NextCore(engine) => engine.read_lines(pane_id, start, count),
        }
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> anyhow::Result<Vec<String>> {
        match self {
            Self::WezTerm(engine) => engine.read_scrollback(pane_id, limit),
            Self::NextCore(engine) => engine.read_scrollback(pane_id, limit),
        }
    }

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> anyhow::Result<ScrollbackTextSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.read_scrollback_text(pane_id, request),
            Self::NextCore(engine) => engine.read_scrollback_text(pane_id, request),
        }
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<ScreenSearchMatch>> {
        match self {
            Self::WezTerm(engine) => engine.search(pane_id, pattern, max_results),
            Self::NextCore(engine) => engine.search(pane_id, pattern, max_results),
        }
    }

    fn cursor(&self, pane_id: usize) -> anyhow::Result<CursorSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.cursor(pane_id),
            Self::NextCore(engine) => engine.cursor(pane_id),
        }
    }
}

impl InputEngine for CurrentTerminalEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.write_input(pane_id, input),
            Self::NextCore(engine) => engine.write_input(pane_id, input),
        }
    }

    fn paste_input(&self, pane_id: usize, text: &str) -> anyhow::Result<()> {
        match self {
            Self::WezTerm(engine) => engine.paste_input(pane_id, text),
            Self::NextCore(engine) => engine.paste_input(pane_id, text),
        }
    }
}

impl RecordingEngine for CurrentTerminalEngine {
    fn start_recording(&self, pane_id: usize) -> anyhow::Result<RecordingStartResult> {
        match self {
            Self::WezTerm(engine) => engine.start_recording(pane_id),
            Self::NextCore(engine) => engine.start_recording(pane_id),
        }
    }

    fn stop_recording(&self, pane_id: usize) -> anyhow::Result<RecordingStopResult> {
        match self {
            Self::WezTerm(engine) => engine.stop_recording(pane_id),
            Self::NextCore(engine) => engine.stop_recording(pane_id),
        }
    }

    fn recording_status(&self, pane_id: usize) -> anyhow::Result<RecordingStatusSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.recording_status(pane_id),
            Self::NextCore(engine) => engine.recording_status(pane_id),
        }
    }

    fn attach_recording_trace(
        &self,
        pane_id: usize,
        trace_id: String,
    ) -> anyhow::Result<Vec<String>> {
        match self {
            Self::WezTerm(engine) => engine.attach_recording_trace(pane_id, trace_id),
            Self::NextCore(engine) => engine.attach_recording_trace(pane_id, trace_id),
        }
    }

    fn export_markdown(
        &self,
        pane_id: usize,
        target_path: Option<String>,
    ) -> anyhow::Result<RecordingExportResult> {
        match self {
            Self::WezTerm(engine) => engine.export_markdown(pane_id, target_path),
            Self::NextCore(engine) => engine.export_markdown(pane_id, target_path),
        }
    }
}

impl HealthEngine for CurrentTerminalEngine {
    fn health(&self) -> anyhow::Result<EngineHealthSnapshot> {
        match self {
            Self::WezTerm(engine) => engine.health(),
            Self::NextCore(engine) => engine.health(),
        }
    }
}

impl WindowEngine for CurrentTerminalEngine {
    fn focus_current_instance_window(&self) -> anyhow::Result<WindowFocusResult> {
        focus_current_instance_window()
    }
}

impl CaptureEngine for CurrentTerminalEngine {
    fn render_scrollback_png(
        &self,
        pane_id: Option<usize>,
        path: &Path,
        opts: &crate::scrollshot::ScrollbackPngOptions,
    ) -> anyhow::Result<RenderedScrollbackPng> {
        match self {
            Self::WezTerm(engine) => engine.render_scrollback_png(pane_id, path, opts),
            Self::NextCore(_) => {
                anyhow::bail!("capture.scrollback PNG rendering is not implemented for next-core")
            }
        }
    }
}

fn focus_current_instance_window() -> anyhow::Result<WindowFocusResult> {
    let window = crate::frontend::try_front_end()
        .and_then(|fe| fe.gui_windows().into_iter().next())
        .ok_or_else(|| anyhow::anyhow!("no GUI window is registered for this instance"))?;
    window.window.focus();
    Ok(WindowFocusResult {
        mux_window_id: window.mux_window_id,
    })
}
