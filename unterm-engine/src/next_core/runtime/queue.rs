#![allow(dead_code)]

use super::command::{RuntimeCommand, RuntimeCommandLane, RuntimeQueuePolicy};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::next_core) struct RuntimeQueueLaneStats {
    pub lifecycle: usize,
    pub input: usize,
    pub render: usize,
    pub screen: usize,
    pub background: usize,
}

impl RuntimeQueueLaneStats {
    pub(in crate::next_core) fn count(self, lane: RuntimeCommandLane) -> usize {
        match lane {
            RuntimeCommandLane::Lifecycle => self.lifecycle,
            RuntimeCommandLane::Input => self.input,
            RuntimeCommandLane::Render => self.render,
            RuntimeCommandLane::Screen => self.screen,
            RuntimeCommandLane::Background => self.background,
        }
    }

    fn increment(&mut self, lane: RuntimeCommandLane) {
        *self.count_mut(lane) += 1;
    }

    fn decrement(&mut self, lane: RuntimeCommandLane) {
        let count = self.count_mut(lane);
        *count = count.saturating_sub(1);
    }

    fn count_mut(&mut self, lane: RuntimeCommandLane) -> &mut usize {
        match lane {
            RuntimeCommandLane::Lifecycle => &mut self.lifecycle,
            RuntimeCommandLane::Input => &mut self.input,
            RuntimeCommandLane::Render => &mut self.render,
            RuntimeCommandLane::Screen => &mut self.screen,
            RuntimeCommandLane::Background => &mut self.background,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::next_core) struct RuntimeQueueStats {
    pub pending_commands: usize,
    pub pending_input_bytes: usize,
    pub pending_lanes: RuntimeQueueLaneStats,
    pub rejected_commands: u64,
    pub rejected_input_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::next_core) enum RuntimeQueueRejection {
    CommandBackpressure {
        pending_commands: usize,
        max_pending_commands: usize,
    },
    InputBackpressure {
        pending_input_bytes: usize,
        command_input_bytes: usize,
        max_pending_input_bytes: usize,
    },
}

#[derive(Debug)]
pub(in crate::next_core) struct RuntimeQueuedCommand {
    pub(in crate::next_core) lane: RuntimeCommandLane,
    pub(in crate::next_core) command: RuntimeCommand,
}

impl RuntimeQueuedCommand {
    fn new(command: RuntimeCommand) -> Self {
        Self {
            lane: command.lane(),
            command,
        }
    }
}

pub(in crate::next_core) struct RuntimeCommandQueue {
    policy: RuntimeQueuePolicy,
    pending: VecDeque<RuntimeQueuedCommand>,
    pending_lanes: RuntimeQueueLaneStats,
    pending_input_bytes: usize,
    rejected_commands: u64,
    rejected_input_bytes: u64,
}

impl RuntimeCommandQueue {
    pub(in crate::next_core) fn new(policy: RuntimeQueuePolicy) -> Self {
        Self {
            policy,
            pending: VecDeque::new(),
            pending_lanes: RuntimeQueueLaneStats::default(),
            pending_input_bytes: 0,
            rejected_commands: 0,
            rejected_input_bytes: 0,
        }
    }

    pub(in crate::next_core) fn enqueue(
        &mut self,
        command: RuntimeCommand,
    ) -> Result<(), RuntimeQueueRejection> {
        if self.pending.len() >= self.policy.max_pending_commands {
            self.rejected_commands += 1;
            return Err(RuntimeQueueRejection::CommandBackpressure {
                pending_commands: self.pending.len(),
                max_pending_commands: self.policy.max_pending_commands,
            });
        }

        let command_input_bytes = command.input_bytes();
        let queued = RuntimeQueuedCommand::new(command);
        if self.pending_input_bytes.saturating_add(command_input_bytes)
            > self.policy.max_pending_input_bytes
        {
            self.rejected_input_bytes += command_input_bytes as u64;
            return Err(RuntimeQueueRejection::InputBackpressure {
                pending_input_bytes: self.pending_input_bytes,
                command_input_bytes,
                max_pending_input_bytes: self.policy.max_pending_input_bytes,
            });
        }

        self.pending_input_bytes += command_input_bytes;
        self.pending_lanes.increment(queued.lane);
        self.pending.push_back(queued);
        Ok(())
    }

    pub(in crate::next_core) fn dequeue(&mut self) -> Option<RuntimeQueuedCommand> {
        let queued = self.pending.pop_front()?;
        self.release(&queued);
        Some(queued)
    }

    pub(in crate::next_core) fn dequeue_lane(
        &mut self,
        lane: RuntimeCommandLane,
    ) -> Option<RuntimeQueuedCommand> {
        let index = self.pending.iter().position(|queued| queued.lane == lane)?;
        let queued = self.pending.remove(index)?;
        self.release(&queued);
        Some(queued)
    }

    fn release(&mut self, queued: &RuntimeQueuedCommand) {
        self.pending_input_bytes = self
            .pending_input_bytes
            .saturating_sub(queued.command.input_bytes());
        self.pending_lanes.decrement(queued.lane);
    }

    pub(in crate::next_core) fn stats(&self) -> RuntimeQueueStats {
        RuntimeQueueStats {
            pending_commands: self.pending.len(),
            pending_input_bytes: self.pending_input_bytes,
            pending_lanes: self.pending_lanes,
            rejected_commands: self.rejected_commands,
            rejected_input_bytes: self.rejected_input_bytes,
        }
    }
}

impl Default for RuntimeCommandQueue {
    fn default() -> Self {
        Self::new(RuntimeQueuePolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> RuntimeQueuePolicy {
        RuntimeQueuePolicy {
            max_pending_commands: 2,
            max_pending_input_bytes: 8,
            max_render_wakeups_per_second: 120,
        }
    }

    #[test]
    fn enqueue_tracks_pending_commands_and_input_bytes() {
        let mut queue = RuntimeCommandQueue::new(policy());

        queue
            .enqueue(RuntimeCommand::WriteInput {
                pane_id: 1,
                text: "abc".to_string(),
            })
            .unwrap();
        queue
            .enqueue(RuntimeCommand::ReadScreen { pane_id: 1 })
            .unwrap();

        assert_eq!(
            queue.stats(),
            RuntimeQueueStats {
                pending_commands: 2,
                pending_input_bytes: 3,
                pending_lanes: RuntimeQueueLaneStats {
                    input: 1,
                    screen: 1,
                    ..RuntimeQueueLaneStats::default()
                },
                rejected_commands: 0,
                rejected_input_bytes: 0,
            }
        );
    }

    #[test]
    fn rejects_when_command_budget_is_full() {
        let mut queue = RuntimeCommandQueue::new(policy());
        queue
            .enqueue(RuntimeCommand::ReadScreen { pane_id: 1 })
            .unwrap();
        queue
            .enqueue(RuntimeCommand::ReadScreen { pane_id: 2 })
            .unwrap();

        let err = queue
            .enqueue(RuntimeCommand::ReadScreen { pane_id: 3 })
            .expect_err("third command should exceed command budget");

        assert_eq!(
            err,
            RuntimeQueueRejection::CommandBackpressure {
                pending_commands: 2,
                max_pending_commands: 2,
            }
        );
        assert_eq!(queue.stats().rejected_commands, 1);
    }

    #[test]
    fn rejects_when_input_byte_budget_is_full() {
        let mut queue = RuntimeCommandQueue::new(policy());
        queue
            .enqueue(RuntimeCommand::PasteInput {
                pane_id: 1,
                text: "12345678".to_string(),
            })
            .unwrap();

        let err = queue
            .enqueue(RuntimeCommand::WriteInput {
                pane_id: 1,
                text: "x".to_string(),
            })
            .expect_err("extra input should exceed input byte budget");

        assert_eq!(
            err,
            RuntimeQueueRejection::InputBackpressure {
                pending_input_bytes: 8,
                command_input_bytes: 1,
                max_pending_input_bytes: 8,
            }
        );
        assert_eq!(queue.stats().rejected_input_bytes, 1);
    }

    #[test]
    fn dequeue_releases_input_byte_budget() {
        let mut queue = RuntimeCommandQueue::new(policy());
        queue
            .enqueue(RuntimeCommand::WriteInput {
                pane_id: 1,
                text: "1234".to_string(),
            })
            .unwrap();

        assert_eq!(queue.stats().pending_input_bytes, 4);
        let queued = queue.dequeue().expect("queued command");
        assert_eq!(queued.lane, RuntimeCommandLane::Input);
        assert_eq!(queue.stats().pending_input_bytes, 0);
        assert_eq!(queue.stats().pending_lanes.input, 0);
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn tracks_pending_commands_by_lane() {
        let mut queue = RuntimeCommandQueue::new(RuntimeQueuePolicy {
            max_pending_commands: 4,
            max_pending_input_bytes: 8,
            max_render_wakeups_per_second: 120,
        });

        queue
            .enqueue(RuntimeCommand::ReadRenderFrame {
                pane_id: 1,
                since_revision: None,
            })
            .unwrap();
        queue
            .enqueue(RuntimeCommand::ReadScreen { pane_id: 1 })
            .unwrap();

        let stats = queue.stats();
        assert_eq!(stats.pending_lanes.render, 1);
        assert_eq!(stats.pending_lanes.screen, 1);
        assert_eq!(stats.pending_lanes.input, 0);
    }

    #[test]
    fn dequeue_lane_preserves_other_lane_backlog() {
        let mut queue = RuntimeCommandQueue::new(RuntimeQueuePolicy {
            max_pending_commands: 4,
            max_pending_input_bytes: 8,
            max_render_wakeups_per_second: 120,
        });

        queue
            .enqueue(RuntimeCommand::ReadScreen { pane_id: 1 })
            .unwrap();
        queue
            .enqueue(RuntimeCommand::WriteInput {
                pane_id: 1,
                text: "x".to_string(),
            })
            .unwrap();

        let input = queue
            .dequeue_lane(RuntimeCommandLane::Input)
            .expect("input lane should be selectable");
        assert_eq!(input.lane, RuntimeCommandLane::Input);
        assert_eq!(queue.stats().pending_lanes.input, 0);
        assert_eq!(queue.stats().pending_lanes.screen, 1);
        assert_eq!(queue.stats().pending_commands, 1);

        let screen = queue.dequeue().expect("screen command remains queued");
        assert_eq!(screen.lane, RuntimeCommandLane::Screen);
    }
}
