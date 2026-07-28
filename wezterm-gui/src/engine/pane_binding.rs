//! GUI pane to next-core session bindings.
//!
//! A GUI pane id and a next-core session id come from two independent
//! allocators: WezTerm hands out `PaneId`s, next-core hands out session ids
//! starting at 1. The numbers overlap, so a GUI caller must never index
//! next-core by raw pane id — pane 3 and session 3 are unrelated, and treating
//! them as the same session paints one session's content into another's pane.
//!
//! This registry keeps the mapping explicit and one-to-one in both directions,
//! so an unbound pane resolves to an error instead of to somebody else's
//! session.

use std::collections::HashMap;

/// Why a pane could not be resolved to a next-core session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextCorePaneBindingError {
    /// The pane has no next-core session bound to it.
    PaneNotBound { pane_id: usize },
}

impl std::fmt::Display for NextCorePaneBindingError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PaneNotBound { pane_id } => {
                write!(fmt, "pane {pane_id} has no next-core session bound")
            }
        }
    }
}

impl std::error::Error for NextCorePaneBindingError {}

/// A next-core session bound to a GUI pane, plus the geometry it was last
/// sized to. The size is tracked here so the render path can detect a stale
/// session without asking the engine on every frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BoundSession {
    session_id: usize,
    cols: usize,
    rows: usize,
}

/// One-to-one map between GUI pane ids and next-core session ids.
#[derive(Clone, Debug, Default)]
pub struct NextCorePaneBindings {
    session_by_pane: HashMap<usize, BoundSession>,
    pane_by_session: HashMap<usize, usize>,
}

#[allow(dead_code)]
impl NextCorePaneBindings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind `pane_id` to `session_id`, recording the size the session was
    /// created at.
    ///
    /// Both sides are kept one-to-one: a pane that was already bound gives up
    /// its previous session, and a session that was already bound to another
    /// pane is detached from it. Returns the session the pane was previously
    /// bound to, so the caller can tear it down.
    pub fn bind(
        &mut self,
        pane_id: usize,
        session_id: usize,
        cols: usize,
        rows: usize,
    ) -> Option<usize> {
        if let Some(previous_pane) = self.pane_by_session.insert(session_id, pane_id) {
            if previous_pane != pane_id {
                self.session_by_pane.remove(&previous_pane);
            }
        }
        let bound = BoundSession {
            session_id,
            cols,
            rows,
        };
        match self.session_by_pane.insert(pane_id, bound) {
            Some(previous) if previous.session_id != session_id => {
                self.pane_by_session.remove(&previous.session_id);
                Some(previous.session_id)
            }
            _ => None,
        }
    }

    pub fn session_for(&self, pane_id: usize) -> Option<usize> {
        self.session_by_pane
            .get(&pane_id)
            .map(|bound| bound.session_id)
    }

    /// The size this pane's session was last sized to.
    pub fn size_for(&self, pane_id: usize) -> Option<(usize, usize)> {
        self.session_by_pane
            .get(&pane_id)
            .map(|bound| (bound.cols, bound.rows))
    }

    /// Record a new pane size, reporting the session that needs resizing.
    ///
    /// Returns `Some(session_id)` only when the size actually changed, so the
    /// render path can call this every frame and only pay for a PTY resize
    /// when the geometry moved. Returns `None` for an unbound pane or a
    /// no-op resize.
    pub fn sync_size(&mut self, pane_id: usize, cols: usize, rows: usize) -> Option<usize> {
        let bound = self.session_by_pane.get_mut(&pane_id)?;
        if bound.cols == cols && bound.rows == rows {
            return None;
        }
        bound.cols = cols;
        bound.rows = rows;
        Some(bound.session_id)
    }

    pub fn pane_for(&self, session_id: usize) -> Option<usize> {
        self.pane_by_session.get(&session_id).copied()
    }

    /// Resolve a pane to its next-core session, failing loudly when unbound.
    ///
    /// Render and input callers must use this rather than passing a raw pane
    /// id to the engine.
    pub fn resolve_session(&self, pane_id: usize) -> Result<usize, NextCorePaneBindingError> {
        self.session_for(pane_id)
            .ok_or(NextCorePaneBindingError::PaneNotBound { pane_id })
    }

    /// Drop the binding for `pane_id`, returning the session it held.
    pub fn unbind_pane(&mut self, pane_id: usize) -> Option<usize> {
        let bound = self.session_by_pane.remove(&pane_id)?;
        self.pane_by_session.remove(&bound.session_id);
        Some(bound.session_id)
    }

    /// Drop the binding for `session_id`, returning the pane it held.
    pub fn unbind_session(&mut self, session_id: usize) -> Option<usize> {
        let pane_id = self.pane_by_session.remove(&session_id)?;
        self.session_by_pane.remove(&pane_id);
        Some(pane_id)
    }

    pub fn contains_pane(&self, pane_id: usize) -> bool {
        self.session_by_pane.contains_key(&pane_id)
    }

    pub fn contains_session(&self, session_id: usize) -> bool {
        self.pane_by_session.contains_key(&session_id)
    }

    pub fn len(&self) -> usize {
        self.session_by_pane.len()
    }

    pub fn is_empty(&self) -> bool {
        self.session_by_pane.is_empty()
    }

    pub fn clear(&mut self) {
        self.session_by_pane.clear();
        self.pane_by_session.clear();
    }

    /// Bound pane ids, ascending. Ordered so teardown and diagnostics are
    /// reproducible across runs.
    pub fn bound_panes(&self) -> Vec<usize> {
        let mut panes: Vec<usize> = self.session_by_pane.keys().copied().collect();
        panes.sort_unstable();
        panes
    }

    /// Bound `(pane_id, session_id)` pairs, ascending by pane id.
    pub fn bindings(&self) -> Vec<(usize, usize)> {
        let mut bindings: Vec<(usize, usize)> = self
            .session_by_pane
            .iter()
            .map(|(pane_id, bound)| (*pane_id, bound.session_id))
            .collect();
        bindings.sort_unstable();
        bindings
    }

    /// Retain only bindings whose pane is still alive, returning the sessions
    /// that were dropped so the caller can destroy them.
    pub fn retain_panes(&mut self, live_pane_ids: &[usize]) -> Vec<usize> {
        let live: std::collections::HashSet<usize> = live_pane_ids.iter().copied().collect();
        let stale: Vec<usize> = self
            .bound_panes()
            .into_iter()
            .filter(|pane_id| !live.contains(pane_id))
            .collect();
        stale
            .into_iter()
            .filter_map(|pane_id| self.unbind_pane(pane_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{NextCorePaneBindingError, NextCorePaneBindings};

    #[test]
    fn unbound_pane_resolves_to_error_not_a_numeric_alias() {
        let mut bindings = NextCorePaneBindings::new();
        // Session 3 exists, but it belongs to pane 7. Pane 3 must not
        // silently resolve to it just because the numbers match.
        bindings.bind(7, 3, 80, 24);

        assert_eq!(
            bindings.resolve_session(3),
            Err(NextCorePaneBindingError::PaneNotBound { pane_id: 3 })
        );
        assert_eq!(bindings.resolve_session(7), Ok(3));
        assert_eq!(bindings.session_for(3), None);
    }

    #[test]
    fn rebinding_a_pane_releases_its_previous_session() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(1, 10, 80, 24);

        assert_eq!(bindings.bind(1, 11, 80, 24), Some(10));
        assert_eq!(bindings.resolve_session(1), Ok(11));
        assert_eq!(bindings.pane_for(10), None);
        assert_eq!(bindings.pane_for(11), Some(1));
        assert_eq!(bindings.len(), 1);
    }

    #[test]
    fn rebinding_a_session_detaches_its_previous_pane() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(1, 10, 80, 24);
        bindings.bind(2, 10, 80, 24);

        assert_eq!(bindings.session_for(1), None);
        assert_eq!(bindings.resolve_session(2), Ok(10));
        assert_eq!(bindings.pane_for(10), Some(2));
        assert_eq!(bindings.len(), 1);
    }

    #[test]
    fn rebinding_the_same_pair_is_idempotent() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(4, 40, 80, 24);

        assert_eq!(bindings.bind(4, 40, 80, 24), None);
        assert_eq!(bindings.resolve_session(4), Ok(40));
        assert_eq!(bindings.pane_for(40), Some(4));
        assert_eq!(bindings.len(), 1);
    }

    #[test]
    fn unbinding_clears_both_directions() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(5, 50, 80, 24);
        bindings.bind(6, 60, 80, 24);

        assert_eq!(bindings.unbind_pane(5), Some(50));
        assert_eq!(bindings.pane_for(50), None);
        assert!(!bindings.contains_pane(5));

        assert_eq!(bindings.unbind_session(60), Some(6));
        assert_eq!(bindings.session_for(6), None);
        assert!(bindings.is_empty());

        assert_eq!(bindings.unbind_pane(5), None);
        assert_eq!(bindings.unbind_session(60), None);
    }

    #[test]
    fn retain_panes_returns_sessions_for_closed_panes() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(1, 10, 80, 24);
        bindings.bind(2, 20, 80, 24);
        bindings.bind(3, 30, 80, 24);

        let dropped = bindings.retain_panes(&[2]);

        assert_eq!(dropped, vec![10, 30]);
        assert_eq!(bindings.bindings(), vec![(2, 20)]);
        assert_eq!(bindings.pane_for(10), None);
        assert_eq!(bindings.pane_for(30), None);
    }

    #[test]
    fn sync_size_reports_only_real_geometry_changes() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(1, 10, 80, 24);

        // Same geometry every frame must not cost a PTY resize.
        assert_eq!(bindings.sync_size(1, 80, 24), None);
        assert_eq!(bindings.size_for(1), Some((80, 24)));

        assert_eq!(bindings.sync_size(1, 120, 30), Some(10));
        assert_eq!(bindings.size_for(1), Some((120, 30)));
        // ...and the new size is now the baseline.
        assert_eq!(bindings.sync_size(1, 120, 30), None);

        // A row-only change still counts.
        assert_eq!(bindings.sync_size(1, 120, 31), Some(10));
    }

    #[test]
    fn sync_size_ignores_unbound_panes() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(1, 10, 80, 24);

        assert_eq!(bindings.sync_size(2, 100, 40), None);
        assert_eq!(bindings.size_for(2), None);
    }

    #[test]
    fn rebinding_resets_the_tracked_size() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(1, 10, 80, 24);
        bindings.bind(1, 11, 100, 40);

        assert_eq!(bindings.size_for(1), Some((100, 40)));
        assert_eq!(bindings.sync_size(1, 100, 40), None);
    }

    #[test]
    fn bindings_and_bound_panes_are_sorted() {
        let mut bindings = NextCorePaneBindings::new();
        bindings.bind(9, 90, 80, 24);
        bindings.bind(2, 20, 80, 24);
        bindings.bind(5, 50, 80, 24);

        assert_eq!(bindings.bound_panes(), vec![2, 5, 9]);
        assert_eq!(bindings.bindings(), vec![(2, 20), (5, 50), (9, 90)]);
    }
}
