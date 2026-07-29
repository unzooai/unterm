
use super::{
    command::RuntimeCommandLane,
    dispatch::{self, RuntimeDispatchResult},
    queue::RuntimeQueuedCommand,
    response::RuntimeResponseReceiver,
    scheduling::RuntimeSchedulePolicy,
    with_current_mut,
};
use anyhow::Result;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::next_core) struct RuntimePumpStats {
    pub(in crate::next_core) drain_calls: u64,
    pub(in crate::next_core) dispatched_commands: u64,
    pub(in crate::next_core) dispatched_lifecycle_commands: u64,
    pub(in crate::next_core) dispatched_input_commands: u64,
    pub(in crate::next_core) dispatched_render_commands: u64,
    pub(in crate::next_core) dispatched_screen_commands: u64,
    pub(in crate::next_core) dispatched_background_commands: u64,
    pub(in crate::next_core) waited_for_response: u64,
    pub(in crate::next_core) completed_without_wait: u64,
    pub(in crate::next_core) total_dispatch_elapsed_micros: u64,
    pub(in crate::next_core) max_dispatch_elapsed_micros: u64,
    pub(in crate::next_core) total_drain_elapsed_micros: u64,
    pub(in crate::next_core) max_drain_elapsed_micros: u64,
}

impl RuntimePumpStats {
    fn record_dispatch(&mut self, lane: RuntimeCommandLane, elapsed: Duration) {
        self.dispatched_commands = self.dispatched_commands.saturating_add(1);
        match lane {
            RuntimeCommandLane::Lifecycle => {
                self.dispatched_lifecycle_commands =
                    self.dispatched_lifecycle_commands.saturating_add(1);
            }
            RuntimeCommandLane::Input => {
                self.dispatched_input_commands = self.dispatched_input_commands.saturating_add(1);
            }
            RuntimeCommandLane::Render => {
                self.dispatched_render_commands = self.dispatched_render_commands.saturating_add(1);
            }
            RuntimeCommandLane::Screen => {
                self.dispatched_screen_commands = self.dispatched_screen_commands.saturating_add(1);
            }
            RuntimeCommandLane::Background => {
                self.dispatched_background_commands =
                    self.dispatched_background_commands.saturating_add(1);
            }
        }
        let elapsed_micros = elapsed.as_micros().min(u128::from(u64::MAX)) as u64;
        self.total_dispatch_elapsed_micros = self
            .total_dispatch_elapsed_micros
            .saturating_add(elapsed_micros);
        self.max_dispatch_elapsed_micros = self.max_dispatch_elapsed_micros.max(elapsed_micros);
    }

    fn record_drain(&mut self, waited_for_response: bool, elapsed: Duration) {
        self.drain_calls = self.drain_calls.saturating_add(1);
        if waited_for_response {
            self.waited_for_response = self.waited_for_response.saturating_add(1);
        } else {
            self.completed_without_wait = self.completed_without_wait.saturating_add(1);
        }
        let elapsed_micros = elapsed.as_micros().min(u128::from(u64::MAX)) as u64;
        self.total_drain_elapsed_micros = self
            .total_drain_elapsed_micros
            .saturating_add(elapsed_micros);
        self.max_drain_elapsed_micros = self.max_drain_elapsed_micros.max(elapsed_micros);
    }
}

#[derive(Debug)]
pub(in crate::next_core) struct RuntimePumpDrain {
    pub(in crate::next_core) result: Result<RuntimeDispatchResult>,
}

#[derive(Debug)]
enum RuntimePumpStep {
    Empty,
    CompletedAttachedResponse,
    DirectResult(RuntimeDispatchResult),
}

pub(in crate::next_core) fn drain_until_response(
    rx: RuntimeResponseReceiver,
) -> Result<RuntimeDispatchResult> {
    drain_until_response_report(rx).result
}

pub(in crate::next_core) fn drain_until_response_report(
    rx: RuntimeResponseReceiver,
) -> RuntimePumpDrain {
    let started = Instant::now();
    loop {
        match rx.try_recv() {
            Ok(Some(result)) => return complete_drain(Ok(result), false, started.elapsed()),
            Ok(None) => {}
            Err(err) => return complete_drain(Err(err), false, started.elapsed()),
        }
        match dispatch_next_scheduled_step() {
            Ok(RuntimePumpStep::Empty) => {
                return complete_drain(rx.recv(), true, started.elapsed());
            }
            Ok(_) => {}
            Err(err) => return complete_drain(Err(err), false, started.elapsed()),
        }
    }
}

fn complete_drain(
    result: Result<RuntimeDispatchResult>,
    waited_for_response: bool,
    elapsed: Duration,
) -> RuntimePumpDrain {
    with_current_mut(|state| {
        state.pump_stats.record_drain(waited_for_response, elapsed);
    });
    RuntimePumpDrain { result }
}

pub(in crate::next_core) fn dispatch_next_scheduled() -> Result<Option<RuntimeDispatchResult>> {
    match dispatch_next_scheduled_step()? {
        RuntimePumpStep::Empty | RuntimePumpStep::CompletedAttachedResponse => Ok(None),
        RuntimePumpStep::DirectResult(result) => Ok(Some(result)),
    }
}

fn dispatch_next_scheduled_step() -> Result<RuntimePumpStep> {
    let Some(queued) = dequeue_next_scheduled() else {
        return Ok(RuntimePumpStep::Empty);
    };
    dispatch_queued(queued)
}

fn dispatch_queued(queued: RuntimeQueuedCommand) -> Result<RuntimePumpStep> {
    let lane = queued.lane;
    let started = Instant::now();
    let result = dispatch::execute(queued.command);
    with_current_mut(|state| state.pump_stats.record_dispatch(lane, started.elapsed()));
    if let Some(response) = queued.response {
        response.complete(result);
        return Ok(RuntimePumpStep::CompletedAttachedResponse);
    }
    result.map(RuntimePumpStep::DirectResult)
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
        let _runtime = test_facade::reset();
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
        let _runtime = test_facade::reset();
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
        let _runtime = test_facade::reset();
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
    fn drain_until_response_continues_after_unrelated_attached_response() {
        let _runtime = test_facade::reset();
        let (other_tx, other_rx) = response::channel();
        let (target_tx, target_rx) = response::channel();
        with_current_mut(|state| {
            state
                .command_queue
                .enqueue_with_response(RuntimeCommand::ReadScreen { pane_id: 111 }, Some(other_tx))
                .unwrap();
            state
                .command_queue
                .enqueue_with_response(RuntimeCommand::ReadScreen { pane_id: 222 }, Some(target_tx))
                .unwrap();
        });

        let report = drain_until_response_report(target_rx);

        assert!(report
            .result
            .unwrap_err()
            .to_string()
            .contains("next-core session 222 not found"));
        // Through the stats rather than the report: they are the same two
        // numbers, and one place to read them is one place to keep right.
        let stats = pump_stats();
        assert_eq!(stats.dispatched_commands, 2);
        assert_eq!(stats.waited_for_response, 0);
        assert!(other_rx
            .recv()
            .unwrap_err()
            .to_string()
            .contains("next-core session 111 not found"));
        assert_eq!(queue_stats().pending_commands, 0);
        let stats = pump_stats();
        assert_eq!(stats.drain_calls, 1);
        assert_eq!(stats.dispatched_commands, 2);
        assert_eq!(stats.dispatched_screen_commands, 2);
        assert_eq!(stats.waited_for_response, 0);
        assert_eq!(stats.completed_without_wait, 1);
        assert!(stats.max_dispatch_elapsed_micros <= stats.total_dispatch_elapsed_micros);
        assert!(stats.max_drain_elapsed_micros <= stats.total_drain_elapsed_micros);
    }

    #[test]
    fn drain_until_response_reads_immediate_rejected_response() {
        let _runtime = test_facade::reset();
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

    #[test]
    fn drain_until_response_report_counts_rejected_immediate_completion() {
        let _runtime = test_facade::reset();
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

        let report = drain_until_response_report(rx);

        assert!(report
            .result
            .unwrap_err()
            .to_string()
            .contains("runtime render queue rejected command"));
        let stats = pump_stats();
        assert_eq!(stats.drain_calls, 1);
        assert_eq!(stats.dispatched_commands, 0);
        assert_eq!(stats.waited_for_response, 0);
        assert_eq!(stats.completed_without_wait, 1);
        assert_eq!(stats.total_dispatch_elapsed_micros, 0);
    }

    fn queue_stats() -> super::super::queue::RuntimeQueueStats {
        with_current(|state| state.command_queue.stats())
    }

    fn pump_stats() -> RuntimePumpStats {
        with_current(|state| state.pump_stats)
    }
}
