use super::{
    command::{RuntimeCommand, RuntimeCommandClass},
    consumer,
    dispatch::RuntimeDispatchResult,
    input_executor, screen_executor,
};
use crate::{
    CursorSnapshot, RenderFrameSnapshot, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, StyledScreenSnapshot, StyledScrollbackSnapshot,
};
use anyhow::{bail, Result};

pub(in crate::next_core) fn submit_input(command: RuntimeCommand) -> Result<()> {
    if command.class() != RuntimeCommandClass::Input {
        bail!(
            "runtime scheduler expected input command, got {:?}",
            command.class()
        );
    }

    let command = consumer::consume_sync(command)?;
    input_executor::execute(command)
}

pub(in crate::next_core) fn read_render_frame(
    pane_id: usize,
    since_revision: Option<u64>,
) -> Result<RenderFrameSnapshot> {
    let command = RuntimeCommand::ReadRenderFrame {
        pane_id,
        since_revision,
    };
    match consumer::submit_and_dispatch_response(command)? {
        RuntimeDispatchResult::RenderFrame(frame) => Ok(frame),
        other => bail!(
            "runtime scheduler expected render-frame dispatch result, got {:?}",
            other
        ),
    }
}

pub(in crate::next_core) fn scroll_viewport_to(pane_id: usize, target: isize) -> Result<()> {
    let command = RuntimeCommand::ScrollViewport { pane_id, target };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_screen_mutation(command)
}

pub(in crate::next_core) fn read_screen(pane_id: usize) -> Result<ScreenSnapshot> {
    let command = RuntimeCommand::ReadScreen { pane_id };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_screen(command)
}

pub(in crate::next_core) fn read_styled_screen(pane_id: usize) -> Result<StyledScreenSnapshot> {
    let command = RuntimeCommand::ReadStyledScreen { pane_id };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_styled_screen(command)
}

pub(in crate::next_core) fn read_visible_text(pane_id: usize) -> Result<String> {
    let command = RuntimeCommand::ReadVisibleText { pane_id };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_visible_text(command)
}

pub(in crate::next_core) fn read_lines(
    pane_id: usize,
    start: i64,
    count: usize,
) -> Result<Vec<ScreenLine>> {
    let command = RuntimeCommand::ReadLines {
        pane_id,
        start,
        count,
    };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_lines(command)
}

pub(in crate::next_core) fn read_scrollback(pane_id: usize, limit: usize) -> Result<Vec<String>> {
    let command = RuntimeCommand::ReadScrollback { pane_id, limit };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_scrollback(command)
}

pub(in crate::next_core) fn read_scrollback_text(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<ScrollbackTextSnapshot> {
    let command = RuntimeCommand::ReadScrollbackText { pane_id, request };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_scrollback_text(command)
}

pub(in crate::next_core) fn read_styled_scrollback(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<StyledScrollbackSnapshot> {
    let command = RuntimeCommand::ReadStyledScrollback { pane_id, request };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_styled_scrollback(command)
}

pub(in crate::next_core) fn search_screen(
    pane_id: usize,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<ScreenSearchMatch>> {
    let command = RuntimeCommand::SearchScreen {
        pane_id,
        pattern: pattern.to_string(),
        max_results,
    };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_search(command)
}

pub(in crate::next_core) fn cursor(pane_id: usize) -> Result<CursorSnapshot> {
    let command = RuntimeCommand::Cursor { pane_id };
    let command = consumer::consume_sync(command)?;
    screen_executor::execute_cursor(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_core::runtime::{
        command::RuntimeQueuePolicy, consumer, queue::RuntimeQueueStats, test_facade, with_current,
        with_current_mut,
    };

    fn queue_stats() -> RuntimeQueueStats {
        with_current(|state| state.command_queue.stats())
    }

    #[test]
    fn runtime_owns_command_queue_stats() {
        test_facade::reset();

        assert_eq!(queue_stats(), RuntimeQueueStats::default());
    }

    #[test]
    fn submit_input_rejects_non_input_before_queueing() {
        test_facade::reset();

        let err = submit_input(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("non-input command should be rejected");

        assert!(err.to_string().contains("expected input command"));
        assert_eq!(queue_stats(), RuntimeQueueStats::default());
    }

    #[test]
    fn render_frame_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = read_render_frame(404, Some(7)).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn render_frame_reads_use_command_backpressure() {
        test_facade::reset();

        with_current_mut(|state| {
            state.command_queue =
                super::super::queue::RuntimeCommandQueue::new(RuntimeQueuePolicy {
                    max_pending_commands: 0,
                    max_pending_input_bytes: 1024,
                    max_render_wakeups_per_second: 120,
                });
        });

        let err = read_render_frame(1, None).expect_err("zero command budget should reject read");

        assert!(err
            .to_string()
            .contains("runtime render queue rejected command"));
        assert_eq!(queue_stats().rejected_commands, 1);
    }

    #[test]
    fn plain_screen_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = read_screen(404).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn styled_screen_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = read_styled_screen(404).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn visible_text_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = read_visible_text(404).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn line_range_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = read_lines(404, 0, 1).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn scrollback_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = read_scrollback(404, 10).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn viewport_scrolls_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = scroll_viewport_to(404, 5).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn scrollback_text_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = read_scrollback_text(
            404,
            ScrollbackTextRequest {
                start_line: None,
                end_line: None,
                tail_lines: Some(10),
                escapes: false,
            },
        )
        .expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn styled_scrollback_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = read_styled_scrollback(
            404,
            ScrollbackTextRequest {
                start_line: None,
                end_line: None,
                tail_lines: Some(10),
                escapes: false,
            },
        )
        .expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn screen_search_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = search_screen(404, "needle", 5).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn cursor_reads_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = cursor(404).expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        assert_eq!(queue_stats().rejected_commands, 0);
    }

    #[test]
    fn enqueue_dequeue_updates_runtime_queue_backpressure_state() {
        test_facade::reset();

        with_current_mut(|state| {
            state.command_queue =
                super::super::queue::RuntimeCommandQueue::new(RuntimeQueuePolicy {
                    max_pending_commands: 4,
                    max_pending_input_bytes: 2,
                    max_render_wakeups_per_second: 120,
                });
        });

        let err = consumer::consume_sync(RuntimeCommand::PasteInput {
            pane_id: 1,
            text: "abc".to_string(),
        })
        .expect_err("input larger than budget should be rejected");

        assert!(err
            .to_string()
            .contains("runtime input queue rejected command"));
        assert!(err.to_string().contains("InputBackpressure"));
        assert_eq!(queue_stats().rejected_input_bytes, 3);
    }
}
