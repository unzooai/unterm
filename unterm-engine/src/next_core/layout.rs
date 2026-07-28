//! Pane layout: the split tree and the geometry it produces.
//!
//! A tab is a binary tree — leaves are panes, interior nodes are splits with a
//! ratio. Asking it for positions turns that tree into rectangles.
//!
//! Two things here are easy to get subtly wrong, so they are stated up front:
//!
//! - **A divider costs a cell.** Two panes side by side in 80 columns get 40
//!   and 39, not 40 and 40. Forgetting this makes every nested split drift one
//!   cell wider than the space it has.
//! - **Closing a pane promotes its sibling.** A split with one child left is
//!   not a split; leaving it in the tree would reserve a divider for a
//!   neighbour that no longer exists.

use anyhow::{anyhow, Result};

/// How a split divides its space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitAxis {
    /// Side by side, dividing the width.
    Horizontal,
    /// Stacked, dividing the height.
    Vertical,
}

/// A pane's cell rectangle within its tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneRect {
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionedPane {
    pub pane_id: usize,
    pub rect: PaneRect,
}

/// Cells taken by the divider between two panes.
const DIVIDER_CELLS: usize = 1;

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Leaf(usize),
    Split {
        axis: SplitAxis,
        /// Fraction of the usable space (after the divider) given to `first`.
        first_ratio: f64,
        first: Box<Node>,
        second: Box<Node>,
    },
}

impl Node {
    fn contains(&self, pane_id: usize) -> bool {
        match self {
            Node::Leaf(id) => *id == pane_id,
            Node::Split { first, second, .. } => {
                first.contains(pane_id) || second.contains(pane_id)
            }
        }
    }

    fn collect_leaves(&self, out: &mut Vec<usize>) {
        match self {
            Node::Leaf(id) => out.push(*id),
            Node::Split { first, second, .. } => {
                first.collect_leaves(out);
                second.collect_leaves(out);
            }
        }
    }
}

/// The split tree for one tab.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// `None` once the last pane closes. A tab with no panes has no geometry;
    /// modelling that as "some pane" would hand the renderer a pane that does
    /// not exist.
    root: Option<Node>,
}

impl Layout {
    /// A tab with one pane filling it.
    pub fn new(pane_id: usize) -> Self {
        Self {
            root: Some(Node::Leaf(pane_id)),
        }
    }

    /// True once every pane has closed; the caller should close the tab.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn contains(&self, pane_id: usize) -> bool {
        self.root
            .as_ref()
            .is_some_and(|root| root.contains(pane_id))
    }

    /// Pane ids, in tree order (left to right, top to bottom).
    pub fn pane_ids(&self) -> Vec<usize> {
        let mut ids = Vec::new();
        if let Some(root) = self.root.as_ref() {
            root.collect_leaves(&mut ids);
        }
        ids
    }

    pub fn pane_count(&self) -> usize {
        self.pane_ids().len()
    }

    /// Split `pane_id`, putting `new_pane_id` in the second half.
    ///
    /// `first_ratio` is clamped away from 0 and 1: a split where one side has
    /// no cells is not a split, and would render as an invisible pane the user
    /// cannot reach.
    pub fn split(
        &mut self,
        pane_id: usize,
        new_pane_id: usize,
        axis: SplitAxis,
        first_ratio: f64,
    ) -> Result<()> {
        if self.contains(new_pane_id) {
            return Err(anyhow!("pane {new_pane_id} is already in this layout"));
        }
        let ratio = clamp_ratio(first_ratio);
        let split = self
            .root
            .as_mut()
            .is_some_and(|root| Self::split_node(root, pane_id, new_pane_id, axis, ratio));
        if !split {
            return Err(anyhow!("pane {pane_id} is not in this layout"));
        }
        Ok(())
    }

    fn split_node(
        node: &mut Node,
        pane_id: usize,
        new_pane_id: usize,
        axis: SplitAxis,
        first_ratio: f64,
    ) -> bool {
        match node {
            Node::Leaf(id) if *id == pane_id => {
                *node = Node::Split {
                    axis,
                    first_ratio,
                    first: Box::new(Node::Leaf(pane_id)),
                    second: Box::new(Node::Leaf(new_pane_id)),
                };
                true
            }
            Node::Leaf(_) => false,
            Node::Split { first, second, .. } => {
                Self::split_node(first, pane_id, new_pane_id, axis, first_ratio)
                    || Self::split_node(second, pane_id, new_pane_id, axis, first_ratio)
            }
        }
    }

    /// Remove `pane_id`, giving its space to its sibling.
    ///
    /// Returns `false` when the pane is not here. Removing the last pane
    /// leaves the layout empty, which the caller should treat as "close the
    /// tab" rather than trying to render it.
    pub fn close(&mut self, pane_id: usize) -> bool {
        let Some(root) = self.root.as_mut() else {
            return false;
        };
        match Self::close_node(root, pane_id) {
            CloseOutcome::NotFound => false,
            CloseOutcome::Removed => true,
            // The root itself was the pane, so nothing is left. Putting a leaf
            // back here would resurrect the pane the caller just closed.
            CloseOutcome::RemoveSelf => {
                self.root = None;
                true
            }
        }
    }

    fn close_node(node: &mut Node, pane_id: usize) -> CloseOutcome {
        match node {
            Node::Leaf(id) if *id == pane_id => CloseOutcome::RemoveSelf,
            Node::Leaf(_) => CloseOutcome::NotFound,
            Node::Split { first, second, .. } => {
                match Self::close_node(first, pane_id) {
                    // The child was the pane: replace this split with the
                    // surviving sibling. A split with one child would still
                    // reserve a divider for a pane that is gone.
                    CloseOutcome::RemoveSelf => {
                        let survivor = (**second).clone();
                        *node = survivor;
                        return CloseOutcome::Removed;
                    }
                    CloseOutcome::Removed => return CloseOutcome::Removed,
                    CloseOutcome::NotFound => {}
                }
                match Self::close_node(second, pane_id) {
                    CloseOutcome::RemoveSelf => {
                        let survivor = (**first).clone();
                        *node = survivor;
                        CloseOutcome::Removed
                    }
                    other => other,
                }
            }
        }
    }

    /// Change the ratio of the split that directly contains `pane_id`.
    ///
    /// Returns `false` when the pane is not in a split, i.e. it is the only
    /// pane in the tab and has nothing to resize against.
    pub fn set_split_ratio(&mut self, pane_id: usize, first_ratio: f64) -> bool {
        self.root
            .as_mut()
            .is_some_and(|root| Self::set_ratio_node(root, pane_id, clamp_ratio(first_ratio)))
    }

    fn set_ratio_node(node: &mut Node, pane_id: usize, ratio: f64) -> bool {
        let Node::Split {
            first_ratio,
            first,
            second,
            ..
        } = node
        else {
            return false;
        };
        if matches!(**first, Node::Leaf(id) if id == pane_id)
            || matches!(**second, Node::Leaf(id) if id == pane_id)
        {
            *first_ratio = ratio;
            return true;
        }
        Self::set_ratio_node(first, pane_id, ratio) || Self::set_ratio_node(second, pane_id, ratio)
    }

    /// Lay the tree out in a `cols` x `rows` grid.
    ///
    /// Every pane gets at least one cell. When there is not enough room for
    /// the divider and both sides, the second pane is squeezed to a single
    /// cell rather than dropped: a pane that exists but cannot be seen is
    /// worse than a cramped one.
    pub fn positions(&self, cols: usize, rows: usize) -> Vec<PositionedPane> {
        let mut out = Vec::new();
        let Some(root) = self.root.as_ref() else {
            return out;
        };
        Self::position_node(
            root,
            PaneRect {
                left: 0,
                top: 0,
                width: cols.max(1),
                height: rows.max(1),
            },
            &mut out,
        );
        out
    }

    fn position_node(node: &Node, rect: PaneRect, out: &mut Vec<PositionedPane>) {
        match node {
            Node::Leaf(pane_id) => out.push(PositionedPane {
                pane_id: *pane_id,
                rect,
            }),
            Node::Split {
                axis,
                first_ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = split_rect(rect, *axis, *first_ratio);
                Self::position_node(first, first_rect, out);
                Self::position_node(second, second_rect, out);
            }
        }
    }

    /// The rectangle for one pane, or `None` if it is not in this layout.
    pub fn position_of(&self, pane_id: usize, cols: usize, rows: usize) -> Option<PaneRect> {
        self.positions(cols, rows)
            .into_iter()
            .find(|pos| pos.pane_id == pane_id)
            .map(|pos| pos.rect)
    }
}

enum CloseOutcome {
    NotFound,
    Removed,
    RemoveSelf,
}

/// Keep a ratio strictly inside (0, 1) so neither side vanishes.
fn clamp_ratio(ratio: f64) -> f64 {
    if !ratio.is_finite() {
        return 0.5;
    }
    ratio.clamp(0.05, 0.95)
}

fn split_rect(rect: PaneRect, axis: SplitAxis, first_ratio: f64) -> (PaneRect, PaneRect) {
    match axis {
        SplitAxis::Horizontal => {
            let (first_width, second_width) = divide(rect.width, first_ratio);
            (
                PaneRect {
                    width: first_width,
                    ..rect
                },
                PaneRect {
                    left: rect.left + first_width + DIVIDER_CELLS,
                    width: second_width,
                    ..rect
                },
            )
        }
        SplitAxis::Vertical => {
            let (first_height, second_height) = divide(rect.height, first_ratio);
            (
                PaneRect {
                    height: first_height,
                    ..rect
                },
                PaneRect {
                    top: rect.top + first_height + DIVIDER_CELLS,
                    height: second_height,
                    ..rect
                },
            )
        }
    }
}

/// Divide `total` cells between two panes, with the divider between them.
///
/// Both sides always get at least one cell; when even that does not fit, the
/// sizes overlap rather than going to zero, because a zero-sized pane is
/// unreachable while a cramped one is merely unpleasant.
fn divide(total: usize, first_ratio: f64) -> (usize, usize) {
    let usable = total.saturating_sub(DIVIDER_CELLS);
    if usable < 2 {
        return (1, 1);
    }
    let first = ((usable as f64) * first_ratio).round() as usize;
    let first = first.clamp(1, usable - 1);
    (first, usable - first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rects(layout: &Layout, cols: usize, rows: usize) -> Vec<(usize, PaneRect)> {
        layout
            .positions(cols, rows)
            .into_iter()
            .map(|pos| (pos.pane_id, pos.rect))
            .collect()
    }

    #[test]
    fn a_single_pane_fills_the_tab() {
        let layout = Layout::new(1);

        assert_eq!(layout.pane_count(), 1);
        assert_eq!(
            layout.positions(80, 24),
            vec![PositionedPane {
                pane_id: 1,
                rect: PaneRect {
                    left: 0,
                    top: 0,
                    width: 80,
                    height: 24
                }
            }]
        );
    }

    #[test]
    fn a_horizontal_split_leaves_a_cell_for_the_divider() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();

        let panes = rects(&layout, 80, 24);
        assert_eq!(panes.len(), 2);
        let (left, right) = (panes[0].1, panes[1].1);

        // 80 columns: 40 + divider + 39. Dropping the divider is the classic
        // off-by-one that makes nested splits overflow their space.
        assert_eq!(left.width + right.width + DIVIDER_CELLS, 80);
        assert_eq!(right.left, left.left + left.width + DIVIDER_CELLS);
        // Height is untouched by a horizontal split.
        assert_eq!(left.height, 24);
        assert_eq!(right.height, 24);
        assert_eq!(left.top, 0);
        assert_eq!(right.top, 0);
    }

    #[test]
    fn a_vertical_split_divides_height_instead() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Vertical, 0.5).unwrap();

        let panes = rects(&layout, 80, 24);
        let (top, bottom) = (panes[0].1, panes[1].1);

        assert_eq!(top.height + bottom.height + DIVIDER_CELLS, 24);
        assert_eq!(bottom.top, top.top + top.height + DIVIDER_CELLS);
        assert_eq!(top.width, 80);
        assert_eq!(bottom.width, 80);
    }

    #[test]
    fn nested_splits_stay_inside_their_parent() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();
        layout.split(2, 3, SplitAxis::Vertical, 0.5).unwrap();

        let panes = rects(&layout, 80, 24);
        assert_eq!(panes.len(), 3);

        let left = panes.iter().find(|(id, _)| *id == 1).unwrap().1;
        let upper_right = panes.iter().find(|(id, _)| *id == 2).unwrap().1;
        let lower_right = panes.iter().find(|(id, _)| *id == 3).unwrap().1;

        // The nested split divides the right column only.
        assert_eq!(upper_right.left, lower_right.left);
        assert_eq!(upper_right.width, lower_right.width);
        assert_eq!(
            upper_right.height + lower_right.height + DIVIDER_CELLS,
            left.height
        );
        // Nothing escapes the tab.
        for (_, rect) in &panes {
            assert!(rect.left + rect.width <= 80, "{rect:?} overflows 80 columns");
            assert!(rect.top + rect.height <= 24, "{rect:?} overflows 24 rows");
        }
    }

    #[test]
    fn closing_a_pane_gives_its_space_to_its_sibling() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();

        assert!(layout.close(2));

        assert_eq!(layout.pane_ids(), vec![1]);
        // The survivor takes the whole tab back, divider included. A split
        // left holding one child would keep reserving that cell.
        assert_eq!(
            layout.position_of(1, 80, 24),
            Some(PaneRect {
                left: 0,
                top: 0,
                width: 80,
                height: 24
            })
        );
    }

    #[test]
    fn closing_an_inner_pane_promotes_the_right_subtree() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();
        layout.split(2, 3, SplitAxis::Vertical, 0.5).unwrap();

        // Close the upper right; the lower right should take the whole right
        // column, not collapse into a stale split.
        assert!(layout.close(2));

        assert_eq!(layout.pane_ids(), vec![1, 3]);
        let left = layout.position_of(1, 80, 24).unwrap();
        let right = layout.position_of(3, 80, 24).unwrap();
        assert_eq!(right.height, 24, "the survivor should own the full column");
        assert_eq!(left.width + right.width + DIVIDER_CELLS, 80);
    }

    #[test]
    fn closing_the_last_pane_empties_the_layout() {
        let mut layout = Layout::new(1);

        assert!(!layout.is_empty());
        assert!(layout.close(1));

        // The pane must actually be gone. Putting a leaf back would resurrect
        // the pane that was just closed, and the tab would never close.
        assert!(layout.is_empty());
        assert!(layout.pane_ids().is_empty());
        assert_eq!(layout.pane_count(), 0);
        assert!(!layout.contains(1));
        assert!(layout.positions(80, 24).is_empty());
        assert!(layout.position_of(1, 80, 24).is_none());

        // Nothing works on an empty layout, and nothing panics either.
        assert!(!layout.close(1));
        assert!(!layout.set_split_ratio(1, 0.5));
        assert!(layout.split(1, 2, SplitAxis::Horizontal, 0.5).is_err());
    }

    #[test]
    fn closing_panes_one_by_one_ends_empty() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();
        layout.split(2, 3, SplitAxis::Vertical, 0.5).unwrap();

        assert!(layout.close(3));
        assert!(layout.close(1));
        assert_eq!(layout.pane_ids(), vec![2]);
        assert!(!layout.is_empty());

        assert!(layout.close(2));
        assert!(layout.is_empty());
    }

    #[test]
    fn closing_an_absent_pane_changes_nothing() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();
        let before = layout.clone();

        assert!(!layout.close(99));
        assert_eq!(layout, before);
    }

    #[test]
    fn splitting_an_absent_pane_is_an_error_and_leaves_the_tree_alone() {
        let mut layout = Layout::new(1);
        let before = layout.clone();

        assert!(layout.split(99, 2, SplitAxis::Horizontal, 0.5).is_err());
        assert_eq!(layout, before);
        // Reusing an id would make two leaves indistinguishable.
        assert!(layout.split(1, 1, SplitAxis::Horizontal, 0.5).is_err());
    }

    #[test]
    fn ratios_are_clamped_so_neither_side_disappears() {
        for ratio in [0.0, -5.0, 1.0, 2.0, f64::NAN, f64::INFINITY] {
            let mut layout = Layout::new(1);
            layout.split(1, 2, SplitAxis::Horizontal, ratio).unwrap();
            let panes = rects(&layout, 80, 24);
            assert!(
                panes.iter().all(|(_, rect)| rect.width >= 1),
                "ratio {ratio} produced a zero-width pane"
            );
        }
    }

    #[test]
    fn a_tab_too_small_to_split_still_gives_every_pane_a_cell() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();

        // Two columns cannot hold two panes and a divider. A pane with zero
        // width would be unreachable; a cramped one is merely unpleasant.
        for cols in [1, 2, 3] {
            let panes = rects(&layout, cols, 24);
            assert_eq!(panes.len(), 2);
            assert!(
                panes.iter().all(|(_, rect)| rect.width >= 1),
                "{cols} columns produced a zero-width pane"
            );
        }
    }

    #[test]
    fn resizing_a_split_moves_the_divider_and_survives_a_tab_resize() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();

        assert!(layout.set_split_ratio(1, 0.25));
        let narrow = layout.position_of(1, 80, 24).unwrap();
        assert!(narrow.width < 40, "the ratio should have moved the divider");

        // Ratios, not absolute cells: widening the tab keeps the proportion.
        let wide = layout.position_of(1, 160, 24).unwrap();
        assert!(
            wide.width > narrow.width * 3 / 2,
            "resizing the tab should scale the pane, got {} then {}",
            narrow.width,
            wide.width
        );

        // A lone pane has nothing to resize against.
        let mut single = Layout::new(7);
        assert!(!single.set_split_ratio(7, 0.25));
    }

    #[test]
    fn every_pane_is_positioned_exactly_once() {
        let mut layout = Layout::new(1);
        layout.split(1, 2, SplitAxis::Horizontal, 0.5).unwrap();
        layout.split(2, 3, SplitAxis::Vertical, 0.5).unwrap();
        layout.split(1, 4, SplitAxis::Vertical, 0.5).unwrap();

        let mut ids: Vec<usize> = layout.positions(120, 40).iter().map(|p| p.pane_id).collect();
        ids.sort_unstable();

        assert_eq!(ids, vec![1, 2, 3, 4]);
        assert_eq!(layout.pane_count(), 4);
    }
}
