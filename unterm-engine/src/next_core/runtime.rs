use super::{
    health_snapshot, input_dispatch, recording_lifecycle, screen_dispatch, session_activity,
    session_queries, session_registry,
};
use crate::{
    CursorSnapshot, EngineHealthSnapshot, RecordingExportResult, RecordingStartResult,
    RecordingStatusSnapshot, RecordingStopResult, RenderFrameSnapshot, ScreenLine,
    ScreenSearchMatch, ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot,
    SessionActivitySnapshot, ShellSnapshot, StyledScreenSnapshot, StyledScrollbackSnapshot,
};
use anyhow::Result;
use parking_lot::RwLock;
use std::sync::OnceLock;

mod session_facade;
#[cfg(test)]
pub(super) mod test_facade;

pub(super) use session_facade::{
    clone_session_base, create_session, destroy, focus, get_session, insert_created, list_sessions,
    next_session_id, resize, split_session, with_session, with_session_optional,
};

#[derive(Default)]
pub(super) struct NextCoreRuntime {
    pub(super) registry: session_registry::SessionRegistry,
}

pub(super) fn current() -> &'static RwLock<NextCoreRuntime> {
    static RUNTIME: OnceLock<RwLock<NextCoreRuntime>> = OnceLock::new();
    RUNTIME.get_or_init(|| RwLock::new(NextCoreRuntime::default()))
}

pub(super) fn with_current<T>(visit: impl FnOnce(&NextCoreRuntime) -> T) -> T {
    let state = current().read();
    visit(&state)
}

pub(super) fn with_current_mut<T>(visit: impl FnOnce(&mut NextCoreRuntime) -> T) -> T {
    let mut state = current().write();
    visit(&mut state)
}

pub(super) fn shell_snapshot(pane_id: usize) -> Result<ShellSnapshot> {
    session_queries::shell_snapshot(pane_id)
}

pub(super) fn output(pane_id: usize) -> Result<String> {
    session_queries::output(pane_id)
}

pub(super) fn scroll_viewport_to(pane_id: usize, target: isize) -> Result<()> {
    screen_dispatch::scroll_viewport_to(pane_id, target)
}

pub(super) fn read_screen(pane_id: usize) -> Result<ScreenSnapshot> {
    screen_dispatch::read_plain_viewport(pane_id)
}

pub(super) fn read_styled_screen(pane_id: usize) -> Result<StyledScreenSnapshot> {
    screen_dispatch::read_styled_viewport(pane_id)
}

pub(super) fn read_render_frame(
    pane_id: usize,
    since_revision: Option<u64>,
) -> Result<RenderFrameSnapshot> {
    screen_dispatch::read_render_frame(pane_id, since_revision)
}

pub(super) fn read_visible_text(pane_id: usize) -> Result<String> {
    screen_dispatch::read_visible_text(pane_id)
}

pub(super) fn read_lines(pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
    screen_dispatch::read_lines(pane_id, start, count)
}

pub(super) fn read_scrollback(pane_id: usize, limit: usize) -> Result<Vec<String>> {
    screen_dispatch::read_scrollback(pane_id, limit)
}

pub(super) fn read_scrollback_text(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<ScrollbackTextSnapshot> {
    screen_dispatch::read_scrollback_text(pane_id, request)
}

pub(super) fn read_styled_scrollback(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<StyledScrollbackSnapshot> {
    screen_dispatch::read_styled_scrollback(pane_id, request)
}

pub(super) fn search_screen(
    pane_id: usize,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<ScreenSearchMatch>> {
    screen_dispatch::search(pane_id, pattern, max_results)
}

pub(super) fn cursor(pane_id: usize) -> Result<CursorSnapshot> {
    screen_dispatch::cursor(pane_id)
}

pub(super) fn write_input(pane_id: usize, input: &str) -> Result<()> {
    input_dispatch::write(pane_id, input)
}

pub(super) fn paste_input(pane_id: usize, text: &str) -> Result<()> {
    input_dispatch::paste(pane_id, text)
}

pub(super) fn session_activity(pane_id: usize) -> Result<SessionActivitySnapshot> {
    with_current_mut(|state| {
        session_activity::read_snapshot(state, pane_id, std::time::Instant::now())
    })
}

pub(super) fn health_snapshot() -> EngineHealthSnapshot {
    with_current_mut(health_snapshot::snapshot)
}

pub(super) fn start_recording(pane_id: usize) -> Result<RecordingStartResult> {
    recording_lifecycle::start(pane_id, recording_lifecycle::timestamp_string())
}

pub(super) fn stop_recording(pane_id: usize) -> Result<RecordingStopResult> {
    recording_lifecycle::stop(pane_id, recording_lifecycle::timestamp_string())
}

pub(super) fn recording_status(pane_id: usize) -> Result<RecordingStatusSnapshot> {
    recording_lifecycle::status(pane_id)
}

pub(super) fn attach_recording_trace(pane_id: usize, trace_id: String) -> Result<Vec<String>> {
    recording_lifecycle::attach_trace(pane_id, trace_id)
}

pub(super) fn export_recording_markdown(
    pane_id: usize,
    target_path: Option<String>,
) -> Result<RecordingExportResult> {
    recording_lifecycle::export_markdown(pane_id, target_path)
}
