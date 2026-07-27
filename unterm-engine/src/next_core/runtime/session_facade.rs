use super::super::{session_creation, session_snapshots, NextCoreSession};
use super::{scheduler, session_registry, with_current, with_current_mut};
use crate::{CreateSessionRequest, SessionSnapshot, SplitSessionRequest};
use anyhow::Result;

pub(in crate::next_core) fn next_session_id() -> usize {
    with_current_mut(session_registry::next_session_id)
}

pub(in crate::next_core) fn focus(pane_id: usize) -> Result<()> {
    scheduler::focus_session(pane_id)
}

pub(in crate::next_core) fn insert_created(session: NextCoreSession) {
    with_current_mut(|state| session_registry::insert_created(state, session));
}

pub(in crate::next_core) fn destroy(pane_id: usize) -> Result<()> {
    scheduler::destroy_session(pane_id)
}

pub(in crate::next_core) fn resize(pane_id: usize, cols: usize, rows: usize) -> Result<()> {
    scheduler::resize_session(pane_id, cols, rows)
}

pub(in crate::next_core) fn with_session<T>(
    pane_id: usize,
    visit: impl FnOnce(&NextCoreSession) -> Result<T>,
) -> Result<T> {
    with_current(|state| visit(session_registry::session(state, pane_id)?))
}

pub(in crate::next_core) fn with_session_optional<T>(
    pane_id: usize,
    visit: impl FnOnce(&NextCoreSession) -> T,
) -> Option<T> {
    with_current(|state| session_registry::session(state, pane_id).ok().map(visit))
}

pub(in crate::next_core) fn list_sessions() -> Vec<SessionSnapshot> {
    with_current_mut(session_snapshots::list)
}

pub(in crate::next_core) fn get_session(pane_id: usize) -> Result<SessionSnapshot> {
    with_current_mut(|state| session_snapshots::get(state, pane_id))
}

pub(in crate::next_core) fn create_session(
    request: CreateSessionRequest,
) -> Result<SessionSnapshot> {
    session_creation::create(request)
}

pub(in crate::next_core) fn split_session(request: SplitSessionRequest) -> Result<SessionSnapshot> {
    session_creation::split(request)
}

pub(in crate::next_core) fn clone_session_base(pane_id: usize) -> Result<SessionSnapshot> {
    with_session(pane_id, |session| Ok(session.snapshot.clone()))
}
