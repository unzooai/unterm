use super::{
    command::{RuntimeCommand, RuntimeCommandClass},
    input_executor,
    queue::RuntimeQueueRejection,
    with_current_mut,
};
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
