use super::{command::RuntimeCommand, queue::RuntimeQueueRejection, with_current_mut};
use anyhow::{anyhow, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::next_core) enum RuntimeConsumerLane {
    Input,
    Render,
    Screen,
}

impl RuntimeConsumerLane {
    fn label(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Render => "render",
            Self::Screen => "screen",
        }
    }
}

pub(in crate::next_core) fn consume_sync(
    command: RuntimeCommand,
    lane: RuntimeConsumerLane,
) -> Result<RuntimeCommand> {
    enqueue(command).map_err(|err| rejected_error(lane, err))?;
    dequeue().ok_or_else(|| anyhow!("runtime {} queue lost enqueued command", lane.label()))
}

fn enqueue(command: RuntimeCommand) -> Result<(), RuntimeQueueRejection> {
    with_current_mut(|state| state.command_queue.enqueue(command))
}

fn dequeue() -> Option<RuntimeCommand> {
    with_current_mut(|state| state.command_queue.dequeue())
}

fn rejected_error(lane: RuntimeConsumerLane, err: RuntimeQueueRejection) -> anyhow::Error {
    anyhow!("runtime {} queue rejected command: {err:?}", lane.label())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_core::runtime::{
        command::RuntimeQueuePolicy, queue::RuntimeCommandQueue, test_facade, with_current,
    };

    #[test]
    fn consume_sync_drains_enqueued_command() {
        test_facade::reset();

        let command = consume_sync(
            RuntimeCommand::ReadScreen { pane_id: 7 },
            RuntimeConsumerLane::Screen,
        )
        .expect("command should enqueue and dequeue");

        assert_eq!(command.pane_id(), Some(7));
        assert_eq!(
            with_current(|state| state.command_queue.stats().pending_commands),
            0
        );
    }

    #[test]
    fn consume_sync_reports_lane_specific_backpressure() {
        test_facade::reset();
        with_current_mut(|state| {
            state.command_queue = RuntimeCommandQueue::new(RuntimeQueuePolicy {
                max_pending_commands: 0,
                max_pending_input_bytes: 1024,
                max_render_wakeups_per_second: 120,
            });
        });

        let err = consume_sync(
            RuntimeCommand::ReadRenderFrame {
                pane_id: 1,
                since_revision: None,
            },
            RuntimeConsumerLane::Render,
        )
        .expect_err("zero command budget should reject");

        assert!(err
            .to_string()
            .contains("runtime render queue rejected command"));
        assert_eq!(
            with_current(|state| state.command_queue.stats().rejected_commands),
            1
        );
    }
}
