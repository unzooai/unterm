use super::{
    health_snapshot, recording_lifecycle, session_activity, session_queries, session_registry,
    session_snapshots, NextCoreSession,
};
use crate::{
    EngineHealthSnapshot, RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot,
    RecordingStopResult, SessionActivitySnapshot, SessionSnapshot, ShellSnapshot,
};
use anyhow::Result;
use parking_lot::RwLock;
use std::sync::OnceLock;

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

pub(super) fn next_session_id() -> usize {
    with_current_mut(session_registry::next_session_id)
}

pub(super) fn focus(pane_id: usize) -> Result<()> {
    with_current_mut(|state| session_registry::focus(state, pane_id))
}

pub(super) fn insert_created(session: NextCoreSession) {
    with_current_mut(|state| session_registry::insert_created(state, session));
}

pub(super) fn destroy(pane_id: usize) -> Result<()> {
    with_current_mut(|state| session_registry::destroy(state, pane_id))
}

pub(super) fn with_session_mut<T>(
    pane_id: usize,
    visit: impl FnOnce(&mut NextCoreSession) -> Result<T>,
) -> Result<T> {
    with_current_mut(|state| visit(session_registry::session_mut(state, pane_id)?))
}

pub(super) fn with_session<T>(
    pane_id: usize,
    visit: impl FnOnce(&NextCoreSession) -> Result<T>,
) -> Result<T> {
    with_current(|state| visit(session_registry::session(state, pane_id)?))
}

pub(super) fn with_session_optional<T>(
    pane_id: usize,
    visit: impl FnOnce(&NextCoreSession) -> T,
) -> Option<T> {
    with_current(|state| session_registry::session(state, pane_id).ok().map(visit))
}

pub(super) fn list_sessions() -> Vec<SessionSnapshot> {
    with_current_mut(session_snapshots::list)
}

pub(super) fn get_session(pane_id: usize) -> Result<SessionSnapshot> {
    with_current_mut(|state| session_snapshots::get(state, pane_id))
}

pub(super) fn clone_session_base(pane_id: usize) -> Result<SessionSnapshot> {
    with_session(pane_id, |session| Ok(session.snapshot.clone()))
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
