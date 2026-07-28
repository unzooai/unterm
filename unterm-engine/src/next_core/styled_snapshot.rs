use super::cell::{CellAttributes, ScreenCell};
use crate::{StyledCell, StyledScreenLine};

pub(super) fn viewport_lines<'a>(
    rows: usize,
    first_row: i64,
    lines: impl IntoIterator<Item = Option<&'a Vec<ScreenCell>>>,
    cols: usize,
    reverse_video: bool,
    hyperlinks: &[String],
) -> Vec<StyledScreenLine> {
    lines
        .into_iter()
        .take(rows)
        .enumerate()
        .map(|(idx, line)| StyledScreenLine {
            row: first_row + idx as i64,
            wrapped: line_is_wrapped(line),
            cells: viewport_cells(line, cols, reverse_video, hyperlinks),
        })
        .collect()
}

pub(super) fn viewport_dirty_lines<'a>(
    first_row: i64,
    lines: impl IntoIterator<Item = (usize, Option<&'a Vec<ScreenCell>>)>,
    cols: usize,
    reverse_video: bool,
    hyperlinks: &[String],
) -> Vec<StyledScreenLine> {
    lines
        .into_iter()
        .map(|(row, line)| StyledScreenLine {
            row: first_row + row as i64,
            wrapped: line_is_wrapped(line),
            cells: viewport_cells(line, cols, reverse_video, hyperlinks),
        })
        .collect()
}

/// Whether a row soft-wrapped, read from the marker on its last cell.
pub(super) fn line_is_wrapped(line: Option<&Vec<ScreenCell>>) -> bool {
    line.and_then(|line| line.last())
        .is_some_and(|cell| cell.wrapped)
}

pub(super) fn viewport_cells(
    line: Option<&Vec<ScreenCell>>,
    cols: usize,
    reverse_video: bool,
    hyperlinks: &[String],
) -> Vec<StyledCell> {
    let mut cells: Vec<StyledCell> = line
        .into_iter()
        .flat_map(|line| {
            line.iter()
                .take(cols)
                .map(|cell| cell.styled_with_reverse_video(reverse_video, hyperlinks))
        })
        .collect();
    while cells.len() < cols {
        cells.push(
            ScreenCell::blank(CellAttributes::default())
                .styled_with_reverse_video(reverse_video, hyperlinks),
        );
    }
    cells
}

pub(super) fn history_range(
    start: usize,
    lines: Vec<&Vec<ScreenCell>>,
    reverse_video: bool,
    hyperlinks: &[String],
) -> Vec<StyledScreenLine> {
    lines
        .into_iter()
        .enumerate()
        .map(|(idx, line)| StyledScreenLine {
            row: start as i64 + idx as i64,
            wrapped: line_is_wrapped(Some(line)),
            cells: line
                .iter()
                .map(|cell| cell.styled_with_reverse_video(reverse_video, hyperlinks))
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_cells_pad_missing_columns() {
        let line = vec![ScreenCell::new('x', CellAttributes::default())];

        let cells = viewport_cells(Some(&line), 3, false, &[]);

        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].ch, 'x');
        assert_eq!(cells[1].ch, ' ');
        assert_eq!(cells[2].ch, ' ');
    }

    #[test]
    fn viewport_cells_apply_reverse_video_to_existing_and_padding_cells() {
        let line = vec![ScreenCell::new('x', CellAttributes::default())];

        let cells = viewport_cells(Some(&line), 2, true, &[]);

        assert!(cells[0].style.inverse);
        assert!(cells[1].style.inverse);
    }

    #[test]
    fn history_range_preserves_hyperlinks() {
        let hyperlinks = vec!["https://example.test".to_string()];
        let mut attr = CellAttributes::default();
        attr.hyperlink = Some(0);
        let line = vec![ScreenCell::new('x', attr)];

        let lines = history_range(5, vec![&line], false, &hyperlinks);

        assert_eq!(lines[0].row, 5);
        assert_eq!(
            lines[0].cells[0].style.hyperlink.as_deref(),
            Some("https://example.test")
        );
    }
}
