#![allow(dead_code)]

use super::{
    dispatch::{self, RuntimeDispatchResult},
    queue::RuntimeQueuedCommand,
    response::RuntimeResponseReceiver,
    scheduling::RuntimeSchedulePolicy,
    with_current_mut,
};
use anyhow::Result;

pub(in crate::next_core) fn drain_until_response(
    rx: RuntimeResponseReceiver,
) -> Result<RuntimeDispatchResult> {
    loop {
        if let Some(result) = rx.try_recv()? {
            return Ok(result);
        }
        if dispatch_next_scheduled()?.is_none() {
            return rx.recv();
        }
    }
}

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

fn dequeue_next_scheduled() -> Option<RuntimeQueuedCommand> {
    with_current_mut(|state| {
        RuntimeSchedulePolicy::default().dequeue_next(&mut state.command_queue)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_core::runtime::{
        command::{RuntimeCommand, RuntimeQueuePolicy},
        queue::RuntimeCommandQueue,
        response, test_facade, with_current, with_current_mut,
    };

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
        let stats = queue_stats();
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
    fn drain_until_response_pumps_until_attached_response_completes() {
        test_facade::reset();
        let (tx, rx) = response::channel();
        with_current_mut(|state| {
            state
                .command_queue
                .enqueue(RuntimeCommand::HealthSnapshot)
                .unwrap();
            state
                .command_queue
                .enqueue_with_response(RuntimeCommand::ReadScreen { pane_id: 404 }, Some(tx))
                .unwrap();
        });

        let err = drain_until_response(rx).expect_err("missing screen pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
        let stats = queue_stats();
        assert_eq!(stats.pending_lanes.screen, 0);
        assert_eq!(stats.pending_lanes.background, 1);
    }

    #[test]
    fn drain_until_response_waits_when_queue_has_no_work() {
        test_facade::reset();
        with_current_mut(|state| {
            state.command_queue = RuntimeCommandQueue::new(RuntimeQueuePolicy {
                max_pending_commands: 0,
                max_pending_input_bytes: 1024,
                max_render_wakeups_per_second: 120,
            });
        });

        let rx = super::super::consumer::submit_with_response(RuntimeCommand::ReadRenderFrame {
            pane_id: 1,
            since_revision: None,
        });

        let err = drain_until_response(rx).expect_err("rejected command should complete response");

        assert!(err
            .to_string()
            .contains("runtime render queue rejected command"));
    }

    fn queue_stats() -> super::super::queue::RuntimeQueueStats {
        with_current(|state| state.command_queue.stats())
    }
}
