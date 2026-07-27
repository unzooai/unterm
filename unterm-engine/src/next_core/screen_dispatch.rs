use super::{
    render_frame::{self, FrameSelection},
    screen_snapshot::{self, ScreenSnapshotMeta},
    session_handles, state,
};
use crate::{RenderFrameSnapshot, ScreenSnapshot, StyledScreenSnapshot};
use anyhow::Result;
use std::time::Instant;

pub(super) fn read_plain_viewport(pane_id: usize) -> Result<ScreenSnapshot> {
    let started_at = Instant::now();
    let (screen_handle, activity_handle) = {
        let state = state().read();
        session_handles::screen_activity(&state, pane_id)?
    };

    let snapshot = {
        let screen = screen_handle.lock();
        let visible = screen.snapshot_viewport_lines();
        let first_row = screen.viewport_first_row();
        screen_snapshot::plain_viewport(visible, first_row, meta(&screen))
    };
    activity_handle
        .lock()
        .mark_screen_read(started_at.elapsed());
    Ok(snapshot)
}

pub(super) fn read_styled_viewport(pane_id: usize) -> Result<StyledScreenSnapshot> {
    let started_at = Instant::now();
    let (screen_handle, activity_handle) = {
        let state = state().read();
        session_handles::screen_activity(&state, pane_id)?
    };

    let snapshot = {
        let screen = screen_handle.lock();
        let first_row = screen.viewport_first_row();
        screen_snapshot::styled_viewport(screen.styled_viewport_lines(first_row), meta(&screen))
    };
    activity_handle
        .lock()
        .mark_screen_read(started_at.elapsed());
    Ok(snapshot)
}

pub(super) fn read_render_frame(
    pane_id: usize,
    since_revision: Option<u64>,
) -> Result<RenderFrameSnapshot> {
    let started_at = Instant::now();
    let (screen_handle, activity_handle) = {
        let state = state().read();
        session_handles::screen_activity(&state, pane_id)?
    };

    let snapshot = {
        let mut screen = screen_handle.lock();
        let first_row = screen.viewport_first_row();
        let revision = screen.revision();

        match render_frame::select_frame(
            screen.rows,
            revision,
            screen.dirty_rows(),
            screen.history.viewport_is_pinned(),
            since_revision,
            |since| screen.can_render_delta_since(since),
        ) {
            FrameSelection::Unchanged => RenderFrameSnapshot {
                lines: Vec::new(),
                cursor: screen.cursor_snapshot(),
                cols: screen.cols,
                rows: screen.rows,
                scrollback_rows: screen.scrollback_rows(),
                revision,
                dirty_rows: None,
                full: false,
            },
            FrameSelection::Changed {
                dirty_rows,
                full,
                clear_dirty,
            } => {
                let lines = match dirty_rows {
                    Some(rows) if full => screen.styled_viewport_lines(first_row),
                    Some(rows) => screen.styled_viewport_dirty_lines(rows, first_row),
                    None => Vec::new(),
                };

                let snapshot = RenderFrameSnapshot {
                    lines,
                    cursor: screen.cursor_snapshot(),
                    cols: screen.cols,
                    rows: screen.rows,
                    scrollback_rows: screen.scrollback_rows(),
                    revision,
                    dirty_rows,
                    full,
                };
                if clear_dirty {
                    screen.clear_dirty_rows();
                }
                snapshot
            }
        }
    };
    activity_handle
        .lock()
        .mark_screen_read(started_at.elapsed());
    Ok(snapshot)
}

fn meta(screen: &super::NextCoreScreen) -> ScreenSnapshotMeta {
    ScreenSnapshotMeta {
        cursor: screen.cursor_snapshot(),
        cols: screen.cols,
        rows: screen.rows,
        scrollback_rows: screen.scrollback_rows(),
        revision: screen.revision(),
        dirty_rows: screen.dirty_rows(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_core::NextCoreScreen;

    #[test]
    fn meta_captures_screen_dimensions_and_revision() {
        let screen = NextCoreScreen::new(80, 24);
        let meta = meta(&screen);

        assert_eq!(meta.cols, 80);
        assert_eq!(meta.rows, 24);
        assert_eq!(meta.scrollback_rows, 0);
        assert_eq!(meta.revision, screen.revision());
    }
}
