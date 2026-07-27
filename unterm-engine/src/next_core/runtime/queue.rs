#![allow(dead_code)]

use super::command::{RuntimeCommand, RuntimeQueuePolicy};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::next_core) struct RuntimeQueueStats {
    pub pending_commands: usize,
    pub pending_input_bytes: usize,
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

pub(in crate::next_core) struct RuntimeCommandQueue {
    policy: RuntimeQueuePolicy,
    pending: VecDeque<RuntimeCommand>,
    pending_input_bytes: usize,
    rejected_commands: u64,
    rejected_input_bytes: u64,
}

impl RuntimeCommandQueue {
    pub(in crate::next_core) fn new(policy: RuntimeQueuePolicy) -> Self {
        Self {
            policy,
            pending: VecDeque::new(),
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
        self.pending.push_back(command);
        Ok(())
    }

    pub(in crate::next_core) fn dequeue(&mut self) -> Option<RuntimeCommand> {
        let command = self.pending.pop_front()?;
        self.pending_input_bytes = self
            .pending_input_bytes
            .saturating_sub(command.input_bytes());
        Some(command)
    }

    pub(in crate::next_core) fn stats(&self) -> RuntimeQueueStats {
        RuntimeQueueStats {
            pending_commands: self.pending.len(),
            pending_input_bytes: self.pending_input_bytes,
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
        assert!(queue.dequeue().is_some());
        assert_eq!(queue.stats().pending_input_bytes, 0);
        assert!(queue.dequeue().is_none());
    }
}
