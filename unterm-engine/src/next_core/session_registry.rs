use super::{lifecycle, state, NextCoreSession, NextCoreState};
use anyhow::{bail, Result};

#[derive(Default)]
pub(super) struct SessionRegistry {
    sessions: Vec<NextCoreSession>,
}

pub(super) fn next_session_id(state: &mut NextCoreState) -> usize {
    state.next_session_id = state.next_session_id.max(1);
    let id = state.next_session_id;
    state.next_session_id += 1;
    id
}

pub(super) fn next_session_id_current() -> usize {
    let mut state = state().write();
    next_session_id(&mut state)
}

pub(super) fn pane_count(state: &NextCoreState) -> usize {
    state.registry.sessions.len()
}

pub(super) fn for_each_session_mut(
    state: &mut NextCoreState,
    mut visit: impl FnMut(&mut NextCoreSession),
) {
    for session in &mut state.registry.sessions {
        visit(session);
    }
}

pub(super) fn with_current_state<T>(visit: impl FnOnce(&NextCoreState) -> T) -> T {
    let state = state().read();
    visit(&state)
}

pub(super) fn with_current_state_mut<T>(visit: impl FnOnce(&mut NextCoreState) -> T) -> T {
    let mut state = state().write();
    visit(&mut state)
}

pub(super) fn with_session_mut_current<T>(
    pane_id: usize,
    visit: impl FnOnce(&mut NextCoreSession) -> Result<T>,
) -> Result<T> {
    with_current_state_mut(|state| visit(session_mut(state, pane_id)?))
}

pub(super) fn set_active(state: &mut NextCoreState, pane_id: usize) {
    for session in &mut state.registry.sessions {
        session.snapshot.is_active = session.snapshot.id == pane_id;
    }
}

pub(super) fn session_mut(
    state: &mut NextCoreState,
    pane_id: usize,
) -> Result<&mut NextCoreSession> {
    state
        .registry
        .sessions
        .iter_mut()
        .find(|session| session.snapshot.id == pane_id)
        .ok_or_else(|| anyhow::anyhow!("next-core session {pane_id} not found"))
}

pub(super) fn session(state: &NextCoreState, pane_id: usize) -> Result<&NextCoreSession> {
    state
        .registry
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
        .ok_or_else(|| anyhow::anyhow!("next-core session {pane_id} not found"))
}

pub(super) fn focus(state: &mut NextCoreState, pane_id: usize) -> Result<()> {
    if !state
        .registry
        .sessions
        .iter()
        .any(|session| session.snapshot.id == pane_id)
    {
        bail!("next-core session {pane_id} not found");
    }
    set_active(state, pane_id);
    Ok(())
}

pub(super) fn focus_current(pane_id: usize) -> Result<()> {
    let mut state = state().write();
    focus(&mut state, pane_id)
}

pub(super) fn insert_created(state: &mut NextCoreState, session: NextCoreSession) {
    let id = session.snapshot.id;
    set_active(state, id);
    state.registry.sessions.push(session);
    state.total_sessions_created = state.total_sessions_created.saturating_add(1);
}

pub(super) fn insert_created_current(session: NextCoreSession) {
    let mut state = state().write();
    insert_created(&mut state, session);
}

pub(super) fn destroy(state: &mut NextCoreState, pane_id: usize) -> Result<()> {
    let Some(idx) = state
        .registry
        .sessions
        .iter()
        .position(|session| session.snapshot.id == pane_id)
    else {
        bail!("next-core session {pane_id} not found");
    };

    let was_active = state.registry.sessions[idx].snapshot.is_active;
    let mut session = state.registry.sessions.remove(idx);
    let (previous_dead, reason) = lifecycle::mark_destroyed(&mut session);
    state.total_sessions_destroyed = state.total_sessions_destroyed.saturating_add(1);
    if !previous_dead {
        lifecycle::record_dead_reason(state, reason);
    } else {
        state.last_dead_reason = Some(reason);
    }

    if was_active {
        if let Some(next_active_id) = state
            .registry
            .sessions
            .last()
            .map(|session| session.snapshot.id)
        {
            set_active(state, next_active_id);
        }
    }

    Ok(())
}

pub(super) fn destroy_current(pane_id: usize) -> Result<()> {
    let mut state = state().write();
    destroy(&mut state, pane_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_session_ids_from_one() {
        let mut state = NextCoreState::default();

        assert_eq!(next_session_id(&mut state), 1);
        assert_eq!(next_session_id(&mut state), 2);
    }

    #[test]
    fn pane_count_reports_empty_registry() {
        let state = NextCoreState::default();

        assert_eq!(pane_count(&state), 0);
    }

    #[test]
    fn with_current_state_visits_registry_state() {
        crate::next_core::test_support::reset_state_for_test();

        let pane_count = with_current_state(pane_count);

        assert_eq!(pane_count, 0);
    }

    #[test]
    fn with_current_state_mut_visits_registry_state() {
        crate::next_core::test_support::reset_state_for_test();

        let id = with_current_state_mut(next_session_id);

        assert_eq!(id, 1);
    }

    #[test]
    fn focus_reports_missing_session() {
        let mut state = NextCoreState::default();

        let err = focus(&mut state, 7).unwrap_err();

        assert!(err.to_string().contains("next-core session 7 not found"));
    }

    #[test]
    fn session_mut_reports_missing_session() {
        let mut state = NextCoreState::default();

        let err = match session_mut(&mut state, 9) {
            Ok(_) => panic!("expected missing session error"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("next-core session 9 not found"));
    }

    #[test]
    fn session_reports_missing_session() {
        let state = NextCoreState::default();

        let err = match session(&state, 10) {
            Ok(_) => panic!("expected missing session error"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("next-core session 10 not found"));
    }
}
