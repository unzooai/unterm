use super::{session_registry, session_snapshots, NextCoreSession};
use crate::SessionSnapshot;
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
