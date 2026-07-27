use crate::DirtyRows;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrameSelection {
    Unchanged,
    Changed {
        dirty_rows: Option<DirtyRows>,
        full: bool,
        clear_dirty: bool,
    },
}

pub(super) fn all_rows(rows: usize) -> Option<DirtyRows> {
    if rows == 0 {
        None
    } else {
        Some(DirtyRows {
            start: 0,
            end: rows - 1,
        })
    }
}

pub(super) fn select_frame(
    rows: usize,
    revision: u64,
    dirty_rows: Option<DirtyRows>,
    viewport_pinned: bool,
    since_revision: Option<u64>,
    can_render_delta_since: impl FnOnce(u64) -> bool,
) -> FrameSelection {
    if since_revision == Some(revision) {
        return FrameSelection::Unchanged;
    }

    let all_rows = all_rows(rows);
    let can_delta = since_revision
        .filter(|since| *since <= revision)
        .is_some_and(can_render_delta_since);
    let force_full = since_revision.is_none() || !can_delta || dirty_rows.is_none();
    let dirty_rows = if force_full {
        all_rows
    } else if viewport_pinned && dirty_rows != all_rows {
        None
    } else {
        dirty_rows
    };
    let full = dirty_rows.is_some() && dirty_rows == all_rows;

    FrameSelection::Changed {
        dirty_rows,
        full,
        clear_dirty: full || dirty_rows.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_revision_emits_empty_frame() {
        let selection = select_frame(
            3,
            7,
            Some(DirtyRows { start: 1, end: 1 }),
            false,
            Some(7),
            |_| true,
        );

        assert_eq!(selection, FrameSelection::Unchanged);
    }

    #[test]
    fn missing_baseline_forces_full_frame() {
        let selection = select_frame(
            3,
            7,
            Some(DirtyRows { start: 1, end: 1 }),
            false,
            None,
            |_| true,
        );

        assert_eq!(
            selection,
            FrameSelection::Changed {
                dirty_rows: Some(DirtyRows { start: 0, end: 2 }),
                full: true,
                clear_dirty: true,
            }
        );
    }

    #[test]
    fn reusable_baseline_selects_dirty_delta() {
        let selection = select_frame(
            3,
            7,
            Some(DirtyRows { start: 1, end: 1 }),
            false,
            Some(6),
            |_| true,
        );

        assert_eq!(
            selection,
            FrameSelection::Changed {
                dirty_rows: Some(DirtyRows { start: 1, end: 1 }),
                full: false,
                clear_dirty: true,
            }
        );
    }

    #[test]
    fn pinned_viewport_skips_partial_dirty_delta() {
        let selection = select_frame(
            3,
            7,
            Some(DirtyRows { start: 1, end: 1 }),
            true,
            Some(6),
            |_| true,
        );

        assert_eq!(
            selection,
            FrameSelection::Changed {
                dirty_rows: None,
                full: false,
                clear_dirty: false,
            }
        );
    }
}
