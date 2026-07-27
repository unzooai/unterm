use super::{
    lifecycle, process_tree,
    runtime::{self, NextCoreRuntime},
    session_registry, NextCoreSession,
};
use crate::SessionSnapshot;
use anyhow::Result;

pub(super) fn list_current() -> Vec<SessionSnapshot> {
    runtime::with_current_mut(list)
}

pub(super) fn get_current(pane_id: usize) -> Result<SessionSnapshot> {
    runtime::with_current_mut(|state| get(state, pane_id))
}

pub(super) fn list(state: &mut NextCoreRuntime) -> Vec<SessionSnapshot> {
    let mut snapshots = Vec::with_capacity(session_registry::pane_count(state));
    let mut dead_reasons = Vec::new();
    session_registry::for_each_session_mut(state, |session| {
        let (snapshot, dead_reason) = snapshot(session);
        if let Some(reason) = dead_reason {
            dead_reasons.push(reason);
        }
        snapshots.push(snapshot);
    });
    for reason in dead_reasons {
        lifecycle::record_dead_reason(state, reason);
    }
    snapshots
}

pub(super) fn get(state: &mut NextCoreRuntime, pane_id: usize) -> Result<SessionSnapshot> {
    list(state)
        .into_iter()
        .find(|session| session.id == pane_id)
        .ok_or_else(|| anyhow::anyhow!("next-core session {pane_id} not found"))
}

#[cfg(test)]
pub(super) fn clone_base(state: &NextCoreRuntime, pane_id: usize) -> Result<SessionSnapshot> {
    Ok(session_registry::session(state, pane_id)?.snapshot.clone())
}

pub(super) fn clone_base_current(pane_id: usize) -> Result<SessionSnapshot> {
    runtime::with_session(pane_id, |session| Ok(session.snapshot.clone()))
}

fn snapshot(session: &mut NextCoreSession) -> (SessionSnapshot, Option<String>) {
    let dead_reason = lifecycle::refresh_liveness(session);
    let mut snapshot = session.snapshot.clone();
    let screen = session.screen.lock();
    snapshot.cursor = screen.cursor_snapshot();
    snapshot.scrollback_rows = screen.scrollback_rows();
    if let Some(title) = screen.title() {
        snapshot.title = title;
    }
    if let Some(cwd) = screen.current_dir() {
        snapshot.shell.cwd = Some(cwd);
    } else if snapshot.shell.cwd.is_none() {
        if let Some(process) =
            process_tree::snapshot(session.root_pid, &snapshot.shell.process_name)
        {
            snapshot.shell.cwd = process.foreground_cwd.or(process.root_cwd);
        }
    }
    (snapshot, dead_reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_empty_state_has_no_snapshots() {
        let mut state = NextCoreRuntime::default();

        assert!(list(&mut state).is_empty());
    }

    #[test]
    fn get_reports_missing_session() {
        let mut state = NextCoreRuntime::default();
        let err = get(&mut state, 88).expect_err("missing session should fail");

        assert!(err.to_string().contains("next-core session 88 not found"));
    }

    #[test]
    fn clone_base_reports_missing_session() {
        let state = NextCoreRuntime::default();
        let err = clone_base(&state, 77).expect_err("missing source session should fail");

        assert!(err.to_string().contains("next-core session 77 not found"));
    }
}
