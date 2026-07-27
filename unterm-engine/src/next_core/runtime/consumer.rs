use super::{
    command::{RuntimeCommand, RuntimeCommandLane},
    dispatch::{self, RuntimeDispatchResult},
    queue::{RuntimeQueueRejection, RuntimeQueuedCommand},
    response::{self, RuntimeResponseReceiver},
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
pub(in crate::next_core) fn submit_with_response(
    command: RuntimeCommand,
) -> RuntimeResponseReceiver {
    let lane = command.lane();
    let (tx, rx) = response::channel();
    if let Err(err) = enqueue_with_response(command, Some(tx)) {
        let (tx, rx) = response::channel();
        tx.complete(Err(rejected_error(lane, err)));
        return rx;
    }
    rx
}

#[allow(dead_code)]
pub(in crate::next_core) fn submit_and_dispatch_response(
    command: RuntimeCommand,
) -> Result<RuntimeDispatchResult> {
    let rx = submit_with_response(command);
    loop {
        if let Some(result) = rx.try_recv()? {
            return Ok(result);
        }
        if dispatch_next_scheduled()?.is_none() {
            return rx.recv();
        }
    }
}

#[allow(dead_code)]
pub(in crate::next_core) fn dispatch_next_scheduled() -> Result<Option<RuntimeDispatchResult>> {
    let Some(queued) = dequeue_next_scheduled() else {
        return Ok(None);
    };
    dispatch_queued(queued)
}

fn dispatch_queued(queued: RuntimeQueuedCommand) -> Result<Option<RuntimeDispatchResult>> {
    let result = dispatch::execute(queued.command);
    if let Some(response) = queued.response {
        response.complete(result);
        return Ok(None);
    }
    result.map(Some)
}

fn enqueue(command: RuntimeCommand) -> Result<(), RuntimeQueueRejection> {
    with_current_mut(|state| state.command_queue.enqueue(command))
}

fn enqueue_with_response(
    command: RuntimeCommand,
    response: Option<response::RuntimeResponseSender>,
) -> Result<(), RuntimeQueueRejection> {
    with_current_mut(|state| state.command_queue.enqueue_with_response(command, response))
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
        command::RuntimeQueuePolicy, queue::RuntimeCommandQueue, response, test_facade,
        with_current,
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

    #[test]
    fn dispatch_next_scheduled_completes_attached_response() {
        test_facade::reset();
        let (tx, rx) = response::channel();
        with_current_mut(|state| {
            state
                .command_queue
                .enqueue_with_response(RuntimeCommand::ReadScreen { pane_id: 404 }, Some(tx))
                .unwrap();
        });

        let returned = dispatch_next_scheduled().expect("dispatch should run");

        assert!(returned.is_none());
        assert!(rx
            .recv()
            .unwrap_err()
            .to_string()
            .contains("next-core session 404 not found"));
    }

    #[test]
    fn submit_with_response_completes_rejected_commands() {
        test_facade::reset();
        with_current_mut(|state| {
            state.command_queue = RuntimeCommandQueue::new(RuntimeQueuePolicy {
                max_pending_commands: 0,
                max_pending_input_bytes: 1024,
                max_render_wakeups_per_second: 120,
            });
        });

        let rx = submit_with_response(RuntimeCommand::ReadRenderFrame {
            pane_id: 1,
            since_revision: None,
        });

        assert!(rx
            .recv()
            .unwrap_err()
            .to_string()
            .contains("runtime render queue rejected command"));
    }

    #[test]
    fn submit_and_dispatch_response_returns_dispatch_result() {
        test_facade::reset();

        let err = submit_and_dispatch_response(RuntimeCommand::ReadScreen { pane_id: 404 })
            .expect_err("missing pane should flow through response receiver");

        assert!(err.to_string().contains("next-core session 404 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
    }

    fn queue_stats() -> super::super::queue::RuntimeQueueStats {
        with_current(|state| state.command_queue.stats())
    }
}
