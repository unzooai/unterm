use super::{
    render_frame::{self, FrameSelection},
    screen_snapshot::{self, ScreenSnapshotMeta},
    session_handles, state,
};
use crate::{CursorSnapshot, RenderFrameSnapshot, ScreenSnapshot, StyledScreenSnapshot};
use anyhow::Result;
use std::time::{Duration, Instant};

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

pub(super) fn snapshot_lines(pane_id: usize) -> Result<Vec<String>> {
    let screen = {
        let state = state().read();
        session_handles::screen(&state, pane_id)?
    };

    let lines = screen.lock().snapshot_lines();
    Ok(lines)
}

pub(super) fn line_count(pane_id: usize) -> Result<usize> {
    let screen = {
        let state = state().read();
        session_handles::screen(&state, pane_id)?
    };

    let count = screen.lock().history_len();
    Ok(count)
}

pub(super) fn line_text_range(pane_id: usize, start: usize, count: usize) -> Result<Vec<String>> {
    let screen = {
        let state = state().read();
        session_handles::screen(&state, pane_id)?
    };

    let lines = screen.lock().history_text_range(start, count);
    Ok(lines)
}

pub(super) fn scrollback_lines(pane_id: usize) -> Result<Vec<String>> {
    let screen = {
        let state = state().read();
        session_handles::screen(&state, pane_id)?
    };

    let lines = screen
        .lock()
        .history
        .scrollback()
        .iter()
        .map(super::NextCoreScreen::line_text)
        .collect();
    Ok(lines)
}

pub(super) fn mark_screen_read(pane_id: usize, duration: Duration) -> Result<()> {
    let activity = {
        let state = state().read();
        session_handles::activity(&state, pane_id)?
    };
    activity.lock().mark_screen_read(duration);
    Ok(())
}

pub(super) fn scroll_viewport_to(pane_id: usize, target: isize) -> Result<()> {
    let started_at = Instant::now();
    let (screen, activity) = {
        let state = state().read();
        session_handles::screen_activity(&state, pane_id)?
    };

    screen.lock().set_viewport_top_near(target);
    activity.lock().mark_viewport_scroll(started_at.elapsed());
    Ok(())
}

pub(super) fn cursor(pane_id: usize) -> Result<CursorSnapshot> {
    let screen = {
        let state = state().read();
        session_handles::screen(&state, pane_id)?
    };

    let cursor = screen.lock().cursor_snapshot();
    Ok(cursor)
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

    #[test]
    fn line_helpers_report_missing_session() {
        let err = line_count(404).expect_err("missing session should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
    }
}
