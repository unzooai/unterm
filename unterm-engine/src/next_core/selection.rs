//! Selection: which cells are selected, and the text that comes out.
//!
//! The state behind copy mode, without any of its UI. A selection is two
//! points and a shape; turning that into text is where the decisions live,
//! and they are the ones users notice:
//!
//! - A selection is anchored, not ordered. Dragging upward or leftward is
//!   normal, so the two points are sorted when they are read, never when they
//!   are set — sorting on set would make the anchor jump.
//! - Trailing blanks are dropped per line. A terminal pads every row to the
//!   full width, so keeping them would paste a wall of spaces.
//! - A soft-wrapped row joins the next without a newline. The user selected
//!   one command; the window being narrow is not their punctuation.

/// Where a selection point sits, in cells from the top-left of the scrollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectionPoint {
    /// Row index in the scrollback, not the viewport, so scrolling does not
    /// move the selection.
    pub row: i64,
    pub column: usize,
}

impl SelectionPoint {
    pub fn new(column: usize, row: i64) -> Self {
        Self { row, column }
    }
}

/// How the two points define a region.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionShape {
    /// Follows the text: partial first and last rows, full rows between.
    #[default]
    Linear,
    /// A rectangle, taking the same columns from every row.
    Block,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: SelectionPoint,
    pub cursor: SelectionPoint,
    pub shape: SelectionShape,
}

impl Selection {
    /// Start a selection at one point; it covers a single cell until extended.
    pub fn new(at: SelectionPoint, shape: SelectionShape) -> Self {
        Self {
            anchor: at,
            cursor: at,
            shape,
        }
    }

    /// Move the free end. The anchor stays put, which is what makes dragging
    /// backwards work.
    pub fn extend_to(&mut self, to: SelectionPoint) {
        self.cursor = to;
    }

    /// The two points in reading order.
    pub fn normalized(&self) -> (SelectionPoint, SelectionPoint) {
        if self.anchor <= self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    /// Rows the selection touches, inclusive.
    pub fn rows(&self) -> std::ops::RangeInclusive<i64> {
        let (start, end) = self.normalized();
        start.row..=end.row
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.cursor
    }

    /// Columns selected on `row`, as a half-open range.
    ///
    /// `row_width` bounds the end so a caller can slice without checking.
    pub fn columns_for_row(&self, row: i64, row_width: usize) -> std::ops::Range<usize> {
        let (start, end) = self.normalized();
        if row < start.row || row > end.row {
            return 0..0;
        }
        match self.shape {
            SelectionShape::Block => {
                // A block takes the same columns from every row regardless of
                // which corner was dragged from.
                let left = start.column.min(end.column);
                let right = start.column.max(end.column);
                left.min(row_width)..(right + 1).min(row_width)
            }
            SelectionShape::Linear => {
                let from = if row == start.row { start.column } else { 0 };
                let to = if row == end.row {
                    end.column + 1
                } else {
                    row_width
                };
                from.min(row_width)..to.min(row_width)
            }
        }
    }
}

/// One row as the extractor needs to see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionRow {
    pub row: i64,
    pub text: String,
    /// True when this row soft-wrapped into the next one.
    pub wrapped: bool,
}

/// Extract the selected text.
///
/// Rows must be supplied in order and cover the selection; rows outside it
/// contribute nothing.
pub fn selected_text(selection: &Selection, rows: &[SelectionRow]) -> String {
    let mut out = String::new();
    let mut pending_newline = false;

    for row in rows {
        let chars: Vec<char> = row.text.chars().collect();
        let range = selection.columns_for_row(row.row, chars.len());
        if range.start >= range.end && !(range.start == 0 && chars.is_empty()) {
            // Nothing selected on this row, but it is inside the selection, so
            // it still contributes a line break.
            if selection.rows().contains(&row.row) && !out.is_empty() {
                pending_newline = true;
            }
            continue;
        }

        let mut piece: String = chars[range].iter().collect();
        // A terminal pads rows to full width; keeping that would paste a wall
        // of spaces. Only the end is trimmed -- leading spaces are indentation
        // the user selected on purpose.
        if !row.wrapped {
            piece = piece.trim_end().to_string();
        }

        if pending_newline {
            out.push('\n');
            pending_newline = false;
        }
        out.push_str(&piece);

        // A soft wrap continues the same logical line: the window being narrow
        // is not the user's punctuation.
        if !row.wrapped {
            pending_newline = true;
        }
    }

    // The last row contributes no trailing newline; a paste should not add a
    // line the user did not select.
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(row: i64, text: &str) -> SelectionRow {
        SelectionRow {
            row,
            text: text.to_string(),
            wrapped: false,
        }
    }

    fn wrapped_row(row: i64, text: &str) -> SelectionRow {
        SelectionRow {
            row,
            text: text.to_string(),
            wrapped: true,
        }
    }

    #[test]
    fn a_fresh_selection_covers_one_cell() {
        let sel = Selection::new(SelectionPoint::new(3, 5), SelectionShape::Linear);

        assert!(sel.is_empty());
        assert_eq!(sel.rows(), 5..=5);
        assert_eq!(sel.columns_for_row(5, 80), 3..4);
        assert_eq!(sel.columns_for_row(4, 80), 0..0);
    }

    #[test]
    fn dragging_backwards_keeps_the_anchor() {
        let mut sel = Selection::new(SelectionPoint::new(10, 5), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(2, 3));

        // Sorting on set would move the anchor and the selection would jump
        // as soon as the user dragged back the other way.
        assert_eq!(sel.anchor, SelectionPoint::new(10, 5));
        assert_eq!(sel.cursor, SelectionPoint::new(2, 3));
        let (start, end) = sel.normalized();
        assert_eq!(start, SelectionPoint::new(2, 3));
        assert_eq!(end, SelectionPoint::new(10, 5));
        assert_eq!(sel.rows(), 3..=5);
    }

    #[test]
    fn a_linear_selection_takes_partial_ends_and_full_middles() {
        let mut sel = Selection::new(SelectionPoint::new(4, 1), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(2, 3));

        assert_eq!(sel.columns_for_row(1, 80), 4..80);
        assert_eq!(sel.columns_for_row(2, 80), 0..80);
        assert_eq!(sel.columns_for_row(3, 80), 0..3);
    }

    #[test]
    fn a_block_selection_takes_the_same_columns_from_every_row() {
        let mut sel = Selection::new(SelectionPoint::new(8, 1), SelectionShape::Block);
        // Dragged left and down, so the corner order is reversed.
        sel.extend_to(SelectionPoint::new(3, 3));

        for row in 1..=3 {
            assert_eq!(sel.columns_for_row(row, 80), 3..9, "row {row}");
        }
    }

    #[test]
    fn columns_never_run_past_the_row() {
        let mut sel = Selection::new(SelectionPoint::new(2, 1), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(100, 1));

        // A caller slices with this; an end past the row would panic.
        let range = sel.columns_for_row(1, 10);
        assert!(range.end <= 10);
        assert!(range.start <= range.end);
    }

    #[test]
    fn extraction_trims_the_padding_a_terminal_adds() {
        let mut sel = Selection::new(SelectionPoint::new(0, 0), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(79, 0));

        let text = selected_text(&sel, &[row(0, "hello                    ")]);

        // Every row is padded to the full width; pasting that would be a wall
        // of spaces.
        assert_eq!(text, "hello");
    }

    #[test]
    fn extraction_keeps_leading_indentation() {
        let mut sel = Selection::new(SelectionPoint::new(0, 0), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(79, 0));

        // Leading spaces are indentation the user selected on purpose.
        assert_eq!(selected_text(&sel, &[row(0, "    indented   ")]), "    indented");
    }

    #[test]
    fn a_soft_wrapped_row_joins_the_next_without_a_newline() {
        let mut sel = Selection::new(SelectionPoint::new(0, 0), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(79, 1));

        let text = selected_text(
            &sel,
            &[wrapped_row(0, "git commit -m "), row(1, "'a long message'")],
        );

        // One command, split only because the window is narrow. A newline here
        // would be punctuation the user never typed.
        assert_eq!(text, "git commit -m 'a long message'");
    }

    #[test]
    fn a_hard_break_between_rows_becomes_a_newline() {
        let mut sel = Selection::new(SelectionPoint::new(0, 0), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(79, 1));

        let text = selected_text(&sel, &[row(0, "first"), row(1, "second")]);

        assert_eq!(text, "first\nsecond");
    }

    #[test]
    fn extraction_does_not_add_a_trailing_newline() {
        let mut sel = Selection::new(SelectionPoint::new(0, 0), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(79, 1));

        let text = selected_text(&sel, &[row(0, "one"), row(1, "two")]);

        // Pasting must not insert a line the user did not select -- in a shell
        // that would run the command.
        assert!(!text.ends_with('\n'));
    }

    #[test]
    fn a_partial_selection_takes_only_the_selected_columns() {
        let mut sel = Selection::new(SelectionPoint::new(6, 0), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(10, 0));

        assert_eq!(selected_text(&sel, &[row(0, "hello world here")]), "world");
    }

    #[test]
    fn a_block_selection_extracts_a_column_from_each_row() {
        let mut sel = Selection::new(SelectionPoint::new(0, 0), SelectionShape::Block);
        sel.extend_to(SelectionPoint::new(2, 2));

        let text = selected_text(&sel, &[row(0, "abcdef"), row(1, "ghijkl"), row(2, "mnopqr")]);

        assert_eq!(text, "abc\nghi\nmno");
    }

    #[test]
    fn rows_outside_the_selection_contribute_nothing() {
        let mut sel = Selection::new(SelectionPoint::new(0, 1), SelectionShape::Linear);
        sel.extend_to(SelectionPoint::new(5, 1));

        let text = selected_text(&sel, &[row(0, "before"), row(1, "middle"), row(2, "after")]);

        assert_eq!(text, "middle");
    }
}
