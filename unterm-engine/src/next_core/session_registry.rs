use super::{lifecycle, state, NextCoreSession, NextCoreState};
use anyhow::{bail, Result};

#[derive(Default)]
pub(super) struct SessionRegistry {
    next_session_id: usize,
    sessions: Vec<NextCoreSession>,
    total_created: u64,
    total_destroyed: u64,
    total_marked_dead: u64,
    last_dead_reason: Option<String>,
}

pub(super) struct SessionRegistryStats {
    pub(super) total_created: u64,
    pub(super) total_destroyed: u64,
    pub(super) total_marked_dead: u64,
    pub(super) last_dead_reason: Option<String>,
}

impl SessionRegistry {
    fn next_session_id(&mut self) -> usize {
        self.next_session_id = self.next_session_id.max(1);
        let id = self.next_session_id;
        self.next_session_id += 1;
        id
    }

    fn len(&self) -> usize {
        self.sessions.len()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut NextCoreSession> {
        self.sessions.iter_mut()
    }

    fn contains(&self, pane_id: usize) -> bool {
        self.sessions
            .iter()
            .any(|session| session.snapshot.id == pane_id)
    }

    fn session(&self, pane_id: usize) -> Option<&NextCoreSession> {
        self.sessions
            .iter()
            .find(|session| session.snapshot.id == pane_id)
    }

    fn session_mut(&mut self, pane_id: usize) -> Option<&mut NextCoreSession> {
        self.sessions
            .iter_mut()
            .find(|session| session.snapshot.id == pane_id)
    }

    fn push(&mut self, session: NextCoreSession) {
        self.sessions.push(session);
        self.total_created = self.total_created.saturating_add(1);
    }

    fn remove(&mut self, pane_id: usize) -> Option<NextCoreSession> {
        let idx = self
            .sessions
            .iter()
            .position(|session| session.snapshot.id == pane_id)?;
        Some(self.sessions.remove(idx))
    }

    fn last_id(&self) -> Option<usize> {
        self.sessions.last().map(|session| session.snapshot.id)
    }

    fn record_destroyed(&mut self) {
        self.total_destroyed = self.total_destroyed.saturating_add(1);
    }

    fn record_marked_dead(&mut self, reason: String) {
        self.total_marked_dead = self.total_marked_dead.saturating_add(1);
        self.last_dead_reason = Some(reason);
    }

    fn set_last_dead_reason(&mut self, reason: String) {
        self.last_dead_reason = Some(reason);
    }

    fn stats(&self) -> SessionRegistryStats {
        SessionRegistryStats {
            total_created: self.total_created,
            total_destroyed: self.total_destroyed,
            total_marked_dead: self.total_marked_dead,
            last_dead_reason: self.last_dead_reason.clone(),
        }
    }
}

pub(super) fn next_session_id(state: &mut NextCoreState) -> usize {
    state.registry.next_session_id()
}

pub(super) fn next_session_id_current() -> usize {
    let mut state = state().write();
    next_session_id(&mut state)
}

pub(super) fn pane_count(state: &NextCoreState) -> usize {
    state.registry.len()
}

pub(super) fn for_each_session_mut(
    state: &mut NextCoreState,
    mut visit: impl FnMut(&mut NextCoreSession),
) {
    for session in state.registry.iter_mut() {
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
    for session in state.registry.iter_mut() {
        session.snapshot.is_active = session.snapshot.id == pane_id;
    }
}

pub(super) fn session_mut(
    state: &mut NextCoreState,
    pane_id: usize,
) -> Result<&mut NextCoreSession> {
    state
        .registry
        .session_mut(pane_id)
        .ok_or_else(|| anyhow::anyhow!("next-core session {pane_id} not found"))
}

pub(super) fn session(state: &NextCoreState, pane_id: usize) -> Result<&NextCoreSession> {
    state
        .registry
        .session(pane_id)
        .ok_or_else(|| anyhow::anyhow!("next-core session {pane_id} not found"))
}

pub(super) fn focus(state: &mut NextCoreState, pane_id: usize) -> Result<()> {
    if !state.registry.contains(pane_id) {
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
    state.registry.push(session);
}

pub(super) fn insert_created_current(session: NextCoreSession) {
    let mut state = state().write();
    insert_created(&mut state, session);
}

pub(super) fn destroy(state: &mut NextCoreState, pane_id: usize) -> Result<()> {
    let Some(mut session) = state.registry.remove(pane_id) else {
        bail!("next-core session {pane_id} not found");
    };

    let was_active = session.snapshot.is_active;
    let (previous_dead, reason) = lifecycle::mark_destroyed(&mut session);
    state.registry.record_destroyed();
    if !previous_dead {
        lifecycle::record_dead_reason(state, reason);
    } else {
        state.registry.set_last_dead_reason(reason);
    }

    if was_active {
        if let Some(next_active_id) = state.registry.last_id() {
            set_active(state, next_active_id);
        }
    }

    Ok(())
}

pub(super) fn destroy_current(pane_id: usize) -> Result<()> {
    let mut state = state().write();
    destroy(&mut state, pane_id)
}

pub(super) fn record_dead_reason(state: &mut NextCoreState, reason: String) {
    state.registry.record_marked_dead(reason);
}

pub(super) fn stats(state: &NextCoreState) -> SessionRegistryStats {
    state.registry.stats()
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
    fn registry_store_push_remove_and_last_id_round_trip() {
        let mut registry = SessionRegistry::default();
        let command = portable_pty::CommandBuilder::new_default_prog();
        let session = crate::next_core::session_runtime::spawn(
            999,
            "sample".to_string(),
            80,
            24,
            command,
            None,
            Vec::new(),
        )
        .expect("spawn session");
        registry.push(session);

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.last_id(), Some(999));
        assert!(registry.contains(999));
        assert!(registry.session(999).is_some());
        assert!(registry.session_mut(999).is_some());

        let removed = registry.remove(999).expect("session should exist");

        assert_eq!(removed.snapshot.id, 999);
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.last_id(), None);
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
