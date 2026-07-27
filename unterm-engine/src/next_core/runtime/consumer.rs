use super::{
    command::{RuntimeCommand, RuntimeCommandLane},
    dispatch::{self, RuntimeDispatchResult},
    queue::{RuntimeQueueRejection, RuntimeQueuedCommand},
    scheduling::RuntimeSchedulePolicy,
    with_current_mut,
};
use anyhow::{anyhow, Result};

pub(in crate::next_core) fn consume_sync(command: RuntimeCommand) -> Result<RuntimeCommand> {
    let lane = command.lane();
    enqueue(command).map_err(|err| rejected_error(lane, err))?;
    let queued = dequeue_lane(lane)
        .ok_or_else(|| anyhow!("runtime {} queue lost enqueued command", lane.label()))?;
    Ok(queued.command)
}

#[allow(dead_code)]
pub(in crate::next_core) fn dispatch_next_scheduled() -> Result<Option<RuntimeDispatchResult>> {
    let Some(queued) = dequeue_next_scheduled() else {
        return Ok(None);
    };
    dispatch::execute(queued.command).map(Some)
}

fn enqueue(command: RuntimeCommand) -> Result<(), RuntimeQueueRejection> {
    with_current_mut(|state| state.command_queue.enqueue(command))
}

fn dequeue_lane(lane: RuntimeCommandLane) -> Option<RuntimeQueuedCommand> {
    with_current_mut(|state| state.command_queue.dequeue_lane(lane))
}

#[allow(dead_code)]
fn dequeue_next_scheduled() -> Option<RuntimeQueuedCommand> {
    with_current_mut(|state| {
        RuntimeSchedulePolicy::default().dequeue_next(&mut state.command_queue)
    })
}

fn rejected_error(lane: RuntimeCommandLane, err: RuntimeQueueRejection) -> anyhow::Error {
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

        let command = consume_sync(RuntimeCommand::ReadScreen { pane_id: 7 })
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

        let err = consume_sync(RuntimeCommand::ReadRenderFrame {
            pane_id: 1,
            since_revision: None,
        })
        .expect_err("zero command budget should reject");

        assert!(err
            .to_string()
            .contains("runtime render queue rejected command"));
        assert_eq!(
            with_current(|state| state.command_queue.stats().rejected_commands),
            1
        );
    }

    #[test]
    fn consume_sync_selects_matching_lane_from_existing_backlog() {
        test_facade::reset();
        with_current_mut(|state| {
            state
                .command_queue
                .enqueue(RuntimeCommand::ReadScreen { pane_id: 1 })
                .unwrap();
        });

        let command = consume_sync(RuntimeCommand::WriteInput {
            pane_id: 1,
            text: "x".to_string(),
        })
        .expect("input should not wait behind screen backlog");

        assert_eq!(command.lane(), RuntimeCommandLane::Input);
        let stats = with_current(|state| state.command_queue.stats());
        assert_eq!(stats.pending_lanes.input, 0);
        assert_eq!(stats.pending_lanes.screen, 1);
    }

    #[test]
    fn dispatch_next_scheduled_uses_input_first_policy() {
        test_facade::reset();
        with_current_mut(|state| {
            state
                .command_queue
                .enqueue(RuntimeCommand::ReadScreen { pane_id: 404 })
                .unwrap();
            state
                .command_queue
                .enqueue(RuntimeCommand::WriteInput {
                    pane_id: 404,
                    text: "x".to_string(),
                })
                .unwrap();
        });

        let err = dispatch_next_scheduled()
            .expect_err("scheduled input should be dispatched before older screen read");

        assert!(err.to_string().contains("next-core session 404 not found"));
        let stats = with_current(|state| state.command_queue.stats());
        assert_eq!(stats.pending_lanes.input, 0);
        assert_eq!(stats.pending_lanes.screen, 1);
    }
}
