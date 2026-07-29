//! Turning a row of cells into glyphs, ligatures included.
//!
//! A terminal is a grid, and shaping is not: `!=` becomes one glyph in a font
//! that has the ligature, Arabic picks a different glyph per position, and a
//! combining mark rides on its base without advancing at all. So the two have
//! to be reconciled, and every terminal reconciles them the same way -- shape
//! the text, then place each glyph at the *cell its cluster came from* rather
//! than at the pen position shaping would have used.
//!
//! Placing by cluster is what keeps the grid honest. A ligature spans the
//! cells its characters occupied and the cursor still lands between them; a
//! font whose advances drift from the cell width cannot pull the row out of
//! alignment, because the row's positions never came from the advances.

/// A stretch of a row that can be shaped as one piece.
///
/// Runs stop at a style change, because a run is also what gets one colour,
/// and at a cell the primary face cannot draw, because shaping is per-face
/// and a run spanning two faces would be shaped by the wrong one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    /// Column this run starts at.
    pub column: usize,
    /// The text, for the shaper.
    pub text: String,
    /// Which column each byte offset in `text` belongs to.
    ///
    /// The shaper reports clusters as byte offsets; this is how a glyph gets
    /// back to the cell it came from.
    pub columns: Vec<(usize, usize)>,
}

impl Run {
    /// The column a cluster byte offset belongs to.
    ///
    /// Clusters land on character boundaries the shaper chose, which are not
    /// always ones we recorded -- a reordered script can report the start of
    /// the cluster it merged. The nearest recorded offset at or before it is
    /// the cell that character came from.
    pub fn column_of(&self, cluster: usize) -> usize {
        self.columns
            .iter()
            .rev()
            .find(|(offset, _)| *offset <= cluster)
            .map(|(_, column)| *column)
            .unwrap_or(self.column)
    }
}

/// Whether two cells can be shaped together.
///
/// Style, because a run is drawn in one colour. Face, because HarfBuzz shapes
/// with one font and a run crossing two would be shaped by whichever it
/// started with -- which is how a CJK character in the middle of a Latin word
/// comes out as a box.
pub fn same_run(
    a_style: &unterm_engine::CellStyle,
    a_face: usize,
    b_style: &unterm_engine::CellStyle,
    b_face: usize,
) -> bool {
    a_face == b_face && a_style == b_style
}

/// Split a line's cells into shapeable runs.
///
/// `face_of` says which face draws a character. Blank cells end a run: there
/// is nothing to shape across a space, and stopping there keeps runs short,
/// which is what makes shaping a whole screen affordable.
pub fn runs(
    cells: &[unterm_engine::StyledCell],
    mut face_of: impl FnMut(char) -> usize,
) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut current: Option<(Run, usize, unterm_engine::CellStyle)> = None;
    let mut column = 0usize;

    for cell in cells {
        let width = cell.width.max(1);
        if cell.ch == ' ' || cell.ch == '\0' || cell.style.hidden {
            if let Some((run, _, _)) = current.take() {
                runs.push(run);
            }
            column += width;
            continue;
        }
        let face = face_of(cell.ch);
        match current.as_mut() {
            Some((run, run_face, run_style))
                if same_run(run_style, *run_face, &cell.style, face) =>
            {
                run.columns.push((run.text.len(), column));
                run.text.push(cell.ch);
            }
            _ => {
                if let Some((run, _, _)) = current.take() {
                    runs.push(run);
                }
                let mut run = Run {
                    column,
                    text: String::new(),
                    columns: Vec::new(),
                };
                run.columns.push((0, column));
                run.text.push(cell.ch);
                current = Some((run, face, cell.style.clone()));
            }
        }
        column += width;
    }
    if let Some((run, _, _)) = current.take() {
        runs.push(run);
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use unterm_engine::{CellStyle, StyledCell, StyledColor};

    fn cells(text: &str, style: CellStyle) -> Vec<StyledCell> {
        text.chars()
            .map(|ch| StyledCell {
                ch,
                style: style.clone(),
                width: 1,
            })
            .collect()
    }

    fn latin(_: char) -> usize {
        0
    }

    #[test]
    fn a_plain_word_is_one_run() {
        let runs = runs(&cells("hello", CellStyle::default()), latin);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "hello");
        assert_eq!(runs[0].column, 0);
    }

    #[test]
    fn a_space_ends_a_run() {
        // Nothing to shape across a space, and short runs are what make
        // shaping a whole screen affordable.
        let runs = runs(&cells("ab cd", CellStyle::default()), latin);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "ab");
        assert_eq!(runs[1].text, "cd");
        assert_eq!(runs[1].column, 3, "the run starts after the space");
    }

    #[test]
    fn a_colour_change_ends_a_run() {
        // A run is also what gets one colour.
        let mut coloured = CellStyle::default();
        coloured.fg = Some(StyledColor::Palette(1));
        let mut line = cells("ab", CellStyle::default());
        line.extend(cells("cd", coloured));
        let runs = runs(&line, latin);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "ab");
        assert_eq!(runs[1].text, "cd");
    }

    #[test]
    fn a_font_change_ends_a_run() {
        // A run crossing two faces would be shaped by whichever it started
        // with, which is how a CJK character mid-word comes out as a box.
        let line = cells("ab中", CellStyle::default());
        let runs = runs(&line, |ch| if ch.is_ascii() { 0 } else { 1 });
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "ab");
        assert_eq!(runs[1].text, "中");
    }

    #[test]
    fn a_glyph_finds_the_column_its_cluster_came_from() {
        let runs = runs(&cells("a!=b", CellStyle::default()), latin);
        let run = &runs[0];
        assert_eq!(run.column_of(0), 0);
        assert_eq!(run.column_of(1), 1, "the '!' is in column 1");
        assert_eq!(run.column_of(2), 2);
        assert_eq!(run.column_of(3), 3);
    }

    #[test]
    fn a_cluster_between_recorded_offsets_takes_the_one_before_it() {
        // A reordered script reports the start of the cluster it merged,
        // which is not always an offset we recorded.
        let runs = runs(&cells("中文", CellStyle::default()), |_| 0);
        let run = &runs[0];
        assert_eq!(run.column_of(1), 0, "still inside the first character");
        assert_eq!(run.column_of(3), 1);
    }

    #[test]
    fn wide_characters_advance_the_column_by_their_width() {
        let line = vec![
            StyledCell {
                ch: '中',
                style: CellStyle::default(),
                width: 2,
            },
            StyledCell {
                ch: 'a',
                style: CellStyle::default(),
                width: 1,
            },
        ];
        let runs = runs(&line, |_| 0);
        // One run, and the 'a' sits at column 2 -- not column 1.
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].column_of(3), 2);
    }

    #[test]
    fn a_blank_line_produces_nothing_to_shape() {
        assert!(runs(&cells("   ", CellStyle::default()), latin).is_empty());
        assert!(runs(&[], latin).is_empty());
    }

    #[test]
    fn hidden_cells_are_not_shaped() {
        let mut hidden = CellStyle::default();
        hidden.hidden = true;
        let mut line = cells("ab", CellStyle::default());
        line.extend(cells("xy", hidden));
        let runs = runs(&line, latin);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "ab");
    }
}
