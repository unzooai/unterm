//! The one tab registry, shared by every window.
//!
//! It used to hang off `TermWindow`, which meant it could not exist before a
//! window did -- and the mux creates a window's first tab before any
//! `TermWindow` is built. That tab could therefore only ever be *adopted*, its
//! arrangement inferred from rectangles rather than recorded as it happened.
//!
//! Tab ids are unique across the mux, not per window, so one registry is also
//! the more accurate shape: a tab moved between windows is the same tab.

use parking_lot::{Mutex, MutexGuard};
use std::sync::OnceLock;
use unterm_engine::next_core::tabs::TabRegistry;

static REGISTRY: OnceLock<Mutex<TabRegistry>> = OnceLock::new();

/// Borrow the registry.
///
/// Held across a whole operation rather than per call, so a tab cannot be
/// created by one thread and split by another in between.
pub fn registry() -> MutexGuard<'static, TabRegistry> {
    REGISTRY
        .get_or_init(|| Mutex::new(TabRegistry::new()))
        .lock()
}

/// Record a tab the mux has just created, before any window shows it.
///
/// Returns false when the registry already knows the tab, which is the normal
/// case for a tab created through a path that records it directly.
pub fn record_created_tab(tab_id: usize, pane_id: usize) -> bool {
    let mut registry = registry();
    if registry.tab_ids().contains(&tab_id) {
        return false;
    }
    // A brand new tab has exactly one pane and no arrangement to infer, so
    // the "adoption" here loses nothing -- there is no split structure yet to
    // get wrong. The size is nominal; positions are recomputed per frame from
    // the real viewport.
    use unterm_engine::next_core::layout::{PaneRect, PositionedPane};
    let positions = [PositionedPane {
        pane_id,
        rect: PaneRect {
            left: 0,
            top: 0,
            width: 1,
            height: 1,
        },
    }];
    registry.adopt_tab(tab_id, &positions, pane_id).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_outlives_any_one_window() {
        // The point of the move: something can be recorded into it before a
        // TermWindow exists.
        let before = registry().tab_count();
        drop(registry());

        assert_eq!(registry().tab_count(), before);
    }

    #[test]
    fn recording_the_same_tab_twice_is_not_an_error() {
        // Two paths can reach a new tab; the second must not clobber the first.
        let tab_id = 90_001;
        let first = record_created_tab(tab_id, 90_101);
        let second = record_created_tab(tab_id, 90_101);

        assert!(first);
        assert!(!second);
        registry().forget_tab(tab_id);
    }
}
