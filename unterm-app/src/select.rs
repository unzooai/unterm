//! Turning mouse positions into a selection.
//!
//! The engine already knows what a selection *is* and what text comes out of
//! one. What is missing between a window and that is arithmetic: which cell a
//! pixel is in, and when a drag has actually begun.
//!
//! Kept out of the event loop so both can be checked without a window, because
//! both are the kind of thing that is off by one cell until someone drags
//! across a line and looks.

use unterm_engine::next_core::selection::{Selection, SelectionPoint, SelectionShape};
use unterm_render::quads::CellMetrics;

/// Which cell a pixel position falls in.
///
/// The column is *not* clamped to the row's text: selecting past the end of a
/// short line and on into the next is how every terminal behaves, and clamping
/// here would stop the selection dead at the last character.
pub fn cell_at(x: f32, y: f32, metrics: CellMetrics, viewport_top: i64) -> SelectionPoint {
    let column = (x.max(0.0) / metrics.width.max(1.0)).floor() as usize;
    let row = (y.max(0.0) / metrics.height.max(1.0)).floor() as i64;
    SelectionPoint::new(column, viewport_top + row)
}

/// A selection being dragged out.
#[derive(Clone, Copy, Debug)]
pub struct Drag {
    anchor: SelectionPoint,
    current: SelectionPoint,
    shape: SelectionShape,
    /// Whether the pointer has left the cell it started in.
    ///
    /// A click that never moves is a click, not an empty selection. Treating
    /// it as a selection clears the clipboard on every stray click.
    moved: bool,
}

impl Drag {
    pub fn start(at: SelectionPoint, shape: SelectionShape) -> Self {
        Self {
            anchor: at,
            current: at,
            shape,
            moved: false,
        }
    }

    pub fn extend(&mut self, to: SelectionPoint) {
        if to != self.anchor {
            self.moved = true;
        }
        self.current = to;
    }

    /// The selection, or None while this is still just a click.
    pub fn selection(&self) -> Option<Selection> {
        if !self.moved {
            return None;
        }
        let mut selection = Selection::new(self.anchor, self.shape);
        selection.extend_to(self.current);
        Some(selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> CellMetrics {
        CellMetrics {
            width: 10.0,
            height: 20.0,
            baseline: 16.0,
        }
    }

    #[test]
    fn a_pixel_lands_in_the_cell_that_contains_it() {
        assert_eq!(cell_at(0.0, 0.0, metrics(), 0), SelectionPoint::new(0, 0));
        assert_eq!(cell_at(9.9, 19.9, metrics(), 0), SelectionPoint::new(0, 0));
        assert_eq!(cell_at(10.0, 20.0, metrics(), 0), SelectionPoint::new(1, 1));
    }

    #[test]
    fn the_row_is_measured_from_the_top_of_the_scrollback() {
        // The viewport moves; a selection made while scrolled back has to stay
        // on the text it was made on.
        assert_eq!(cell_at(0.0, 40.0, metrics(), 100), SelectionPoint::new(0, 102));
    }

    #[test]
    fn a_position_outside_the_window_does_not_go_negative() {
        // A drag that leaves the window reports negative coordinates, and a
        // negative row would select from the wrong end of the scrollback.
        assert_eq!(cell_at(-50.0, -50.0, metrics(), 5), SelectionPoint::new(0, 5));
    }

    #[test]
    fn a_click_that_never_moves_is_not_a_selection() {
        let drag = Drag::start(SelectionPoint::new(3, 2), SelectionShape::Linear);

        // Treating it as an empty selection clears the clipboard on every
        // stray click.
        assert!(drag.selection().is_none());
    }

    #[test]
    fn moving_within_the_same_cell_is_still_a_click() {
        let mut drag = Drag::start(SelectionPoint::new(3, 2), SelectionShape::Linear);

        drag.extend(SelectionPoint::new(3, 2));

        assert!(drag.selection().is_none());
    }

    #[test]
    fn a_drag_to_another_cell_is_a_selection() {
        let mut drag = Drag::start(SelectionPoint::new(3, 2), SelectionShape::Linear);

        drag.extend(SelectionPoint::new(7, 4));
        let selection = drag.selection().expect("a moved drag selects");

        assert_eq!(selection.rows(), 2..=4);
    }

    #[test]
    fn dragging_backwards_keeps_the_anchor_where_it_started() {
        let mut drag = Drag::start(SelectionPoint::new(7, 4), SelectionShape::Linear);

        drag.extend(SelectionPoint::new(3, 2));
        let selection = drag.selection().expect("a moved drag selects");

        // Dragging up and left is normal, and the anchor must not follow the
        // pointer or the selection jumps as soon as it is reversed.
        assert_eq!(selection.anchor, SelectionPoint::new(7, 4));
        assert_eq!(selection.rows(), 2..=4);
    }

    #[test]
    fn a_block_drag_keeps_its_shape() {
        let mut drag = Drag::start(SelectionPoint::new(1, 1), SelectionShape::Block);

        drag.extend(SelectionPoint::new(5, 5));
        let selection = drag.selection().expect("a moved drag selects");

        assert_eq!(selection.shape, SelectionShape::Block);
    }
}
