use super::DirtyRows;

#[derive(Default)]
pub(super) struct RenderState {
    revision: u64,
    dirty_rows: Option<DirtyRows>,
}

impl RenderState {
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }

    pub(super) fn set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }

    pub(super) fn dirty_rows(&self) -> Option<DirtyRows> {
        self.dirty_rows
    }

    pub(super) fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1).max(1);
    }

    pub(super) fn clear_dirty_rows(&mut self) {
        self.dirty_rows = None;
    }

    pub(super) fn mark_dirty_row(&mut self, row: usize, rows: usize) {
        if rows == 0 {
            return;
        }
        let row = row.min(rows.saturating_sub(1));
        self.dirty_rows = Some(match self.dirty_rows {
            Some(dirty) => DirtyRows {
                start: dirty.start.min(row),
                end: dirty.end.max(row),
            },
            None => DirtyRows {
                start: row,
                end: row,
            },
        });
    }

    pub(super) fn mark_dirty_range(&mut self, start: usize, end: usize, rows: usize) {
        if rows == 0 {
            return;
        }
        let start = start.min(rows - 1);
        let end = end.min(rows - 1);
        if start <= end {
            self.dirty_rows = Some(match self.dirty_rows {
                Some(dirty) => DirtyRows {
                    start: dirty.start.min(start),
                    end: dirty.end.max(end),
                },
                None => DirtyRows { start, end },
            });
        }
    }

    pub(super) fn mark_all_dirty(&mut self, rows: usize) {
        if rows > 0 {
            self.mark_dirty_range(0, rows - 1, rows);
        }
    }
}
