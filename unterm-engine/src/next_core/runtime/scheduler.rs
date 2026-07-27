use super::{
    command::{RuntimeCommand, RuntimeCommandClass},
    input_executor,
    queue::RuntimeQueueRejection,
    screen_executor, with_current_mut,
};
use crate::{RenderFrameSnapshot, ScreenSnapshot};
use anyhow::{anyhow, bail, Result};

fn enqueue(command: RuntimeCommand) -> Result<(), RuntimeQueueRejection> {
    with_current_mut(|state| state.command_queue.enqueue(command))
}

fn dequeue() -> Option<RuntimeCommand> {
    with_current_mut(|state| state.command_queue.dequeue())
}

pub(in crate::next_core) fn submit_input(command: RuntimeCommand) -> Result<()> {
    if command.class() != RuntimeCommandClass::Input {
        bail!(
            "runtime scheduler expected input command, got {:?}",
            command.class()
        );
    }

    enqueue(command).map_err(|err| anyhow!("runtime input queue rejected command: {err:?}"))?;
    let command = dequeue().ok_or_else(|| anyhow!("runtime input queue lost enqueued command"))?;
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
    enqueue(command).map_err(|err| anyhow!("runtime render queue rejected command: {err:?}"))?;
    let command = dequeue().ok_or_else(|| anyhow!("runtime render queue lost enqueued command"))?;
    screen_executor::execute_render_frame(command)
}

pub(in crate::next_core) fn scroll_viewport_to(pane_id: usize, target: isize) -> Result<()> {
    let command = RuntimeCommand::ScrollViewport { pane_id, target };
    enqueue(command).map_err(|err| anyhow!("runtime screen queue rejected command: {err:?}"))?;
    let command = dequeue().ok_or_else(|| anyhow!("runtime screen queue lost enqueued command"))?;
    screen_executor::execute_screen_mutation(command)
}

pub(in crate::next_core) fn read_screen(pane_id: usize) -> Result<ScreenSnapshot> {
    let command = RuntimeCommand::ReadScreen { pane_id };
    enqueue(command).map_err(|err| anyhow!("runtime screen queue rejected command: {err:?}"))?;
    let command = dequeue().ok_or_else(|| anyhow!("runtime screen queue lost enqueued command"))?;
    screen_executor::execute_screen(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_core::runtime::{
        command::RuntimeQueuePolicy, queue::RuntimeQueueStats, test_facade, with_current,
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
    fn viewport_scrolls_enter_runtime_queue_before_dispatch() {
        test_facade::reset();

        let err = scroll_viewport_to(404, 5).expect_err("missing pane should fail");

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

        let err = enqueue(RuntimeCommand::PasteInput {
            pane_id: 1,
            text: "abc".to_string(),
        })
        .expect_err("input larger than budget should be rejected");

        assert_eq!(
            err,
            RuntimeQueueRejection::InputBackpressure {
                pending_input_bytes: 0,
                command_input_bytes: 3,
                max_pending_input_bytes: 2,
            }
        );
        assert_eq!(queue_stats().rejected_input_bytes, 3);
    }
}
