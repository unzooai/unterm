use super::{
    health_snapshot, recording_lifecycle, session_activity, session_queries, session_registry,
};
use crate::{
    EngineHealthSnapshot, RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot,
    RecordingStopResult, SessionActivitySnapshot, ShellSnapshot,
};
use anyhow::Result;
use parking_lot::RwLock;
use std::sync::OnceLock;

mod io_facade;
mod session_facade;
#[cfg(test)]
pub(super) mod test_facade;

pub(super) use io_facade::{
    cursor, paste_input, read_lines, read_render_frame, read_screen, read_scrollback,
    read_scrollback_text, read_styled_screen, read_styled_scrollback, read_visible_text,
    scroll_viewport_to, search_screen, write_input,
};
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
