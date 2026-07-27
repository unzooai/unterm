use super::super::{health_snapshot as health_snapshot_engine, session_activity, session_queries};
use super::with_current_mut;
use crate::{EngineHealthSnapshot, SessionActivitySnapshot, ShellSnapshot};
use anyhow::Result;

pub(in crate::next_core) fn shell_snapshot(pane_id: usize) -> Result<ShellSnapshot> {
    session_queries::shell_snapshot(pane_id)
}

pub(in crate::next_core) fn output(pane_id: usize) -> Result<String> {
    session_queries::output(pane_id)
}

pub(in crate::next_core) fn session_activity(pane_id: usize) -> Result<SessionActivitySnapshot> {
    with_current_mut(|state| {
        session_activity::read_snapshot(state, pane_id, std::time::Instant::now())
    })
}

pub(in crate::next_core) fn health_snapshot() -> EngineHealthSnapshot {
    with_current_mut(health_snapshot_engine::snapshot)
}
