use super::{
    render_frame::{self, FrameSelection},
    screen_search,
    screen_snapshot::{self, ScreenSnapshotMeta},
    screen_text,
    session_handles::{self, ScrollbackHandles},
    state,
};
use crate::{
    CursorSnapshot, RenderFrameSnapshot, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, StyledScreenSnapshot, StyledScrollbackSnapshot,
};
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

pub(super) fn read_visible_text(pane_id: usize) -> Result<String> {
    Ok(read_plain_viewport(pane_id)?.lines.join("\n"))
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

pub(super) fn read_lines(pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
    let started_at = Instant::now();
    let start = start.max(0) as usize;
    let lines = screen_snapshot::plain_lines(line_text_range(pane_id, start, count)?, start);
    mark_screen_read(pane_id, started_at.elapsed())?;
    Ok(lines)
}

pub(super) fn read_scrollback(pane_id: usize, limit: usize) -> Result<Vec<String>> {
    let started_at = Instant::now();
    let lines = scrollback_lines(pane_id)?
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let lines = screen_text::tail_lines(&lines, limit);
    mark_screen_read(pane_id, started_at.elapsed())?;
    Ok(lines)
}

pub(super) fn search(
    pane_id: usize,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<ScreenSearchMatch>> {
    let started_at = Instant::now();
    let lines = snapshot_lines(pane_id)?;
    let matches = screen_search::find_matches(&lines, pattern, max_results);
    mark_screen_read(pane_id, started_at.elapsed())?;
    Ok(matches)
}

pub(super) fn snapshot_lines(pane_id: usize) -> Result<Vec<String>> {
    let screen = {
        let state = state().read();
        session_handles::screen(&state, pane_id)?
    };

    let lines = screen.lock().snapshot_lines();
    Ok(lines)
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

pub(super) fn read_scrollback_text(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<ScrollbackTextSnapshot> {
    let started_at = Instant::now();
    let handles = scrollback_handles(pane_id)?;

    let (selected, line_count, start, end) = if request.escapes {
        let output = handles.output.lock();
        let lines = screen_text::output_lines(&output);
        let line_count = lines.len();
        let (start, end) = screen_text::bounded_range(
            line_count,
            request.start_line,
            request.end_line,
            request.tail_lines,
        );
        (lines[start..end].to_vec(), line_count, start, end)
    } else {
        let screen = handles.screen.lock();
        let line_count = screen.history_len();
        let (start, end) = screen_text::bounded_range(
            line_count,
            request.start_line,
            request.end_line,
            request.tail_lines,
        );
        (
            screen.history_text_range(start, end.saturating_sub(start)),
            line_count,
            start,
            end,
        )
    };

    let snapshot = ScrollbackTextSnapshot {
        text: selected.join("\n"),
        lines: selected,
        first_row: start as i64,
        row_count: end.saturating_sub(start) as i64,
        cols: handles.cols,
        escapes: request.escapes,
        scrollback_top: 0,
        physical_top: line_count.saturating_sub(handles.rows) as i64,
        viewport_rows: handles.rows,
    };
    handles
        .activity
        .lock()
        .mark_screen_read(started_at.elapsed());
    Ok(snapshot)
}

pub(super) fn read_styled_scrollback(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<StyledScrollbackSnapshot> {
    if request.escapes {
        let text = read_scrollback_text(pane_id, request)?;
        return Ok(screen_snapshot::escaped_styled_scrollback(
            &text.lines,
            text.first_row,
            text.row_count,
            text.cols,
            text.scrollback_top,
            text.physical_top,
            text.viewport_rows,
        ));
    }

    let started_at = Instant::now();
    let handles = scrollback_handles(pane_id)?;
    let screen = handles.screen.lock();
    let line_count = screen.history_len();
    let (start, end) = screen_text::bounded_range(
        line_count,
        request.start_line,
        request.end_line,
        request.tail_lines,
    );
    let count = end.saturating_sub(start);
    let snapshot = StyledScrollbackSnapshot {
        lines: screen.styled_history_range(start, count),
        first_row: start as i64,
        row_count: count as i64,
        cols: handles.cols,
        scrollback_top: 0,
        physical_top: line_count.saturating_sub(handles.rows) as i64,
        viewport_rows: handles.rows,
    };
    drop(screen);
    handles
        .activity
        .lock()
        .mark_screen_read(started_at.elapsed());
    Ok(snapshot)
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

fn scrollback_handles(pane_id: usize) -> Result<ScrollbackHandles> {
    let state = state().read();
    session_handles::scrollback(&state, pane_id)
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
        let err = line_text_range(404, 0, 1).expect_err("missing session should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
    }

    #[test]
    fn scrollback_text_reports_missing_session() {
        let err = read_scrollback_text(
            404,
            ScrollbackTextRequest {
                start_line: None,
                end_line: None,
                tail_lines: None,
                escapes: false,
            },
        )
        .expect_err("missing session should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
    }
}
