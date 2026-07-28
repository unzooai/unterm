//! Tabs: which panes exist, how they are arranged, and which one is active.
//!
//! `layout` knows how to divide a rectangle. This is the state around it —
//! several tabs, each with its own split tree and active pane, and the rules
//! for what happens when panes come and go.
//!
//! The rules that matter, because getting them wrong strands the user:
//!
//! - Closing the active pane moves focus to a surviving one. Leaving focus on
//!   a closed pane means keystrokes go nowhere.
//! - Closing the last pane in a tab closes the tab. A tab with no panes has
//!   nothing to render and no way to get a pane back.
//! - A pane belongs to exactly one tab, so a pane id can be resolved to its
//!   tab without searching.

use crate::next_core::layout::{Layout, PaneRect, PositionedPane, SplitAxis};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub type TabId = usize;
pub type PaneId = usize;

#[derive(Clone, Debug)]
struct Tab {
    layout: Layout,
    active_pane: PaneId,
}

/// What closing a pane did to its tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseResult {
    /// The pane went away; the tab remains, now focused on `active_pane`.
    PaneClosed { active_pane: PaneId },
    /// That was the last pane, so the tab is gone too.
    TabClosed,
}

/// Every tab and its panes.
#[derive(Debug, Default)]
pub struct TabRegistry {
    tabs: HashMap<TabId, Tab>,
    /// Reverse index so a pane resolves to its tab without scanning.
    tab_of_pane: HashMap<PaneId, TabId>,
    active_tab: Option<TabId>,
    next_tab_id: TabId,
}

impl TabRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Tab ids, ascending, so iteration order does not depend on hashing.
    pub fn tab_ids(&self) -> Vec<TabId> {
        let mut ids: Vec<TabId> = self.tabs.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    pub fn active_tab(&self) -> Option<TabId> {
        self.active_tab
    }

    /// Focus `tab_id`. Returns false if there is no such tab.
    pub fn set_active_tab(&mut self, tab_id: TabId) -> bool {
        if !self.tabs.contains_key(&tab_id) {
            return false;
        }
        self.active_tab = Some(tab_id);
        true
    }

    /// Create a tab holding `pane_id`, and focus it.
    pub fn create_tab(&mut self, pane_id: PaneId) -> Result<TabId> {
        if self.tab_of_pane.contains_key(&pane_id) {
            return Err(anyhow!("pane {pane_id} is already in a tab"));
        }
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.insert(
            tab_id,
            Tab {
                layout: Layout::new(pane_id),
                active_pane: pane_id,
            },
        );
        self.tab_of_pane.insert(pane_id, tab_id);
        self.active_tab = Some(tab_id);
        Ok(tab_id)
    }

    pub fn tab_of_pane(&self, pane_id: PaneId) -> Option<TabId> {
        self.tab_of_pane.get(&pane_id).copied()
    }

    pub fn active_pane(&self, tab_id: TabId) -> Option<PaneId> {
        self.tabs.get(&tab_id).map(|tab| tab.active_pane)
    }

    /// Focus `pane_id` within its own tab, and focus that tab.
    pub fn set_active_pane(&mut self, pane_id: PaneId) -> bool {
        let Some(tab_id) = self.tab_of_pane(pane_id) else {
            return false;
        };
        if let Some(tab) = self.tabs.get_mut(&tab_id) {
            tab.active_pane = pane_id;
            self.active_tab = Some(tab_id);
            return true;
        }
        false
    }

    pub fn pane_ids(&self, tab_id: TabId) -> Vec<PaneId> {
        self.tabs
            .get(&tab_id)
            .map(|tab| tab.layout.pane_ids())
            .unwrap_or_default()
    }

    /// Split `pane_id`, adding `new_pane_id` beside it, and focus the new pane.
    ///
    /// Focusing the new pane is what every terminal does: you split in order
    /// to type in the new half.
    pub fn split(
        &mut self,
        pane_id: PaneId,
        new_pane_id: PaneId,
        axis: SplitAxis,
        first_ratio: f64,
    ) -> Result<TabId> {
        if self.tab_of_pane.contains_key(&new_pane_id) {
            return Err(anyhow!("pane {new_pane_id} is already in a tab"));
        }
        let tab_id = self
            .tab_of_pane(pane_id)
            .ok_or_else(|| anyhow!("pane {pane_id} is not in any tab"))?;
        let tab = self
            .tabs
            .get_mut(&tab_id)
            .ok_or_else(|| anyhow!("tab {tab_id} vanished while splitting"))?;

        tab.layout.split(pane_id, new_pane_id, axis, first_ratio)?;
        tab.active_pane = new_pane_id;
        self.tab_of_pane.insert(new_pane_id, tab_id);
        self.active_tab = Some(tab_id);
        Ok(tab_id)
    }

    /// Close `pane_id`, reporting whether its tab survived.
    pub fn close_pane(&mut self, pane_id: PaneId) -> Option<CloseResult> {
        let tab_id = self.tab_of_pane(pane_id)?;
        let tab = self.tabs.get_mut(&tab_id)?;

        if !tab.layout.close(pane_id) {
            return None;
        }
        self.tab_of_pane.remove(&pane_id);

        if tab.layout.is_empty() {
            self.tabs.remove(&tab_id);
            if self.active_tab == Some(tab_id) {
                // Focus the lowest surviving tab rather than nothing: the
                // window is still open and needs somewhere to send input.
                self.active_tab = self.tab_ids().first().copied();
            }
            return Some(CloseResult::TabClosed);
        }

        // Focus must land on a live pane, or keystrokes go nowhere.
        if tab.active_pane == pane_id {
            tab.active_pane = tab
                .layout
                .pane_ids()
                .first()
                .copied()
                .expect("a non-empty layout has at least one pane");
        }
        Some(CloseResult::PaneClosed {
            active_pane: tab.active_pane,
        })
    }

    /// Lay out a tab in a `cols` x `rows` grid.
    pub fn positions(&self, tab_id: TabId, cols: usize, rows: usize) -> Vec<PositionedPane> {
        self.tabs
            .get(&tab_id)
            .map(|tab| tab.layout.positions(cols, rows))
            .unwrap_or_default()
    }

    /// Where one pane sits in its own tab.
    pub fn pane_rect(&self, pane_id: PaneId, cols: usize, rows: usize) -> Option<PaneRect> {
        let tab_id = self.tab_of_pane(pane_id)?;
        self.tabs
            .get(&tab_id)?
            .layout
            .position_of(pane_id, cols, rows)
    }

    /// Move the divider of the split holding `pane_id`.
    pub fn set_split_ratio(&mut self, pane_id: PaneId, first_ratio: f64) -> bool {
        let Some(tab_id) = self.tab_of_pane(pane_id) else {
            return false;
        };
        self.tabs
            .get_mut(&tab_id)
            .is_some_and(|tab| tab.layout.set_split_ratio(pane_id, first_ratio))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_tab_holds_one_pane_and_takes_focus() {
        let mut tabs = TabRegistry::new();
        let tab = tabs.create_tab(10).unwrap();

        assert_eq!(tabs.tab_count(), 1);
        assert_eq!(tabs.active_tab(), Some(tab));
        assert_eq!(tabs.active_pane(tab), Some(10));
        assert_eq!(tabs.pane_ids(tab), vec![10]);
        assert_eq!(tabs.tab_of_pane(10), Some(tab));
    }

    #[test]
    fn a_pane_belongs_to_exactly_one_tab() {
        let mut tabs = TabRegistry::new();
        tabs.create_tab(10).unwrap();

        // Reusing a pane id would make the reverse index lie about which tab
        // owns it, and the loser would render into the wrong rectangle.
        assert!(tabs.create_tab(10).is_err());
        assert!(tabs.split(10, 10, SplitAxis::Horizontal, 0.5).is_err());
    }

    #[test]
    fn splitting_focuses_the_new_pane() {
        let mut tabs = TabRegistry::new();
        let tab = tabs.create_tab(10).unwrap();

        tabs.split(10, 11, SplitAxis::Horizontal, 0.5).unwrap();

        // You split in order to type in the new half.
        assert_eq!(tabs.active_pane(tab), Some(11));
        assert_eq!(tabs.pane_ids(tab), vec![10, 11]);
        assert_eq!(tabs.tab_of_pane(11), Some(tab));
        assert_eq!(tabs.tab_count(), 1, "a split adds a pane, not a tab");
    }

    #[test]
    fn closing_the_active_pane_moves_focus_to_a_survivor() {
        let mut tabs = TabRegistry::new();
        let tab = tabs.create_tab(10).unwrap();
        tabs.split(10, 11, SplitAxis::Horizontal, 0.5).unwrap();
        assert_eq!(tabs.active_pane(tab), Some(11));

        let result = tabs.close_pane(11).expect("close the active pane");

        // Focus left on a closed pane would send keystrokes nowhere.
        assert_eq!(result, CloseResult::PaneClosed { active_pane: 10 });
        assert_eq!(tabs.active_pane(tab), Some(10));
        assert_eq!(tabs.tab_of_pane(11), None);
    }

    #[test]
    fn closing_an_inactive_pane_leaves_focus_alone() {
        let mut tabs = TabRegistry::new();
        let tab = tabs.create_tab(10).unwrap();
        tabs.split(10, 11, SplitAxis::Horizontal, 0.5).unwrap();

        let result = tabs.close_pane(10).expect("close the inactive pane");

        assert_eq!(result, CloseResult::PaneClosed { active_pane: 11 });
        assert_eq!(tabs.active_pane(tab), Some(11));
    }

    #[test]
    fn closing_the_last_pane_closes_the_tab() {
        let mut tabs = TabRegistry::new();
        let tab = tabs.create_tab(10).unwrap();

        assert_eq!(tabs.close_pane(10), Some(CloseResult::TabClosed));

        // A tab with no panes has nothing to render and no way back.
        assert!(tabs.is_empty());
        assert_eq!(tabs.active_tab(), None);
        assert_eq!(tabs.tab_of_pane(10), None);
        assert!(tabs.pane_ids(tab).is_empty());
        assert!(tabs.positions(tab, 80, 24).is_empty());
    }

    #[test]
    fn closing_the_active_tab_focuses_another_one() {
        let mut tabs = TabRegistry::new();
        let first = tabs.create_tab(10).unwrap();
        let second = tabs.create_tab(20).unwrap();
        assert_eq!(tabs.active_tab(), Some(second));

        assert_eq!(tabs.close_pane(20), Some(CloseResult::TabClosed));

        // The window is still open, so something has to hold focus.
        assert_eq!(tabs.active_tab(), Some(first));
        assert_eq!(tabs.tab_count(), 1);
    }

    #[test]
    fn closing_an_unknown_pane_reports_nothing() {
        let mut tabs = TabRegistry::new();
        tabs.create_tab(10).unwrap();

        assert_eq!(tabs.close_pane(99), None);
        assert_eq!(tabs.tab_count(), 1);
    }

    #[test]
    fn positions_come_from_the_tabs_own_layout() {
        let mut tabs = TabRegistry::new();
        let tab = tabs.create_tab(10).unwrap();
        tabs.split(10, 11, SplitAxis::Horizontal, 0.5).unwrap();

        let positions = tabs.positions(tab, 80, 24);
        assert_eq!(positions.len(), 2);
        // Same arithmetic as the layout tree, which is pinned to mux's.
        assert_eq!(positions[0].rect.width, 39);
        assert_eq!(positions[1].rect.width, 40);

        assert_eq!(
            tabs.pane_rect(11, 80, 24).map(|rect| rect.left),
            Some(40),
            "pane 11 starts after pane 10 and the divider"
        );
        // A pane in another tab must not resolve here.
        assert_eq!(tabs.pane_rect(99, 80, 24), None);
    }

    #[test]
    fn tabs_are_independent() {
        let mut tabs = TabRegistry::new();
        let first = tabs.create_tab(10).unwrap();
        let second = tabs.create_tab(20).unwrap();
        tabs.split(20, 21, SplitAxis::Vertical, 0.5).unwrap();

        // Splitting one tab must not touch the other.
        assert_eq!(tabs.pane_ids(first), vec![10]);
        assert_eq!(tabs.pane_ids(second), vec![20, 21]);
        assert_eq!(tabs.tab_ids(), vec![first, second]);
    }

    #[test]
    fn focus_follows_the_pane_across_tabs() {
        let mut tabs = TabRegistry::new();
        let first = tabs.create_tab(10).unwrap();
        let second = tabs.create_tab(20).unwrap();
        assert_eq!(tabs.active_tab(), Some(second));

        // Focusing a pane focuses its tab too, or the window would show one
        // tab while input went to another.
        assert!(tabs.set_active_pane(10));
        assert_eq!(tabs.active_tab(), Some(first));
        assert_eq!(tabs.active_pane(first), Some(10));

        assert!(!tabs.set_active_pane(99));
        assert!(!tabs.set_active_tab(999));
    }

    #[test]
    fn tab_ids_are_not_reused_after_a_tab_closes() {
        let mut tabs = TabRegistry::new();
        let first = tabs.create_tab(10).unwrap();
        tabs.close_pane(10).unwrap();
        let second = tabs.create_tab(20).unwrap();

        // A reused id would let a stale reference address the new tab.
        assert_ne!(first, second);
    }
}
