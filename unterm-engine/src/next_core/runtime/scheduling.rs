#![allow(dead_code)]

use super::{
    command::RuntimeCommandLane,
    queue::{RuntimeCommandQueue, RuntimeQueueStats, RuntimeQueuedCommand},
};

const INPUT_FIRST_LANES: [RuntimeCommandLane; 5] = [
    RuntimeCommandLane::Input,
    RuntimeCommandLane::Lifecycle,
    RuntimeCommandLane::Render,
    RuntimeCommandLane::Screen,
    RuntimeCommandLane::Background,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::next_core) struct RuntimeSchedulePolicy {
    lane_order: &'static [RuntimeCommandLane],
}

impl RuntimeSchedulePolicy {
    pub(in crate::next_core) fn input_first() -> Self {
        Self {
            lane_order: &INPUT_FIRST_LANES,
        }
    }

    pub(in crate::next_core) fn select_lane(
        self,
        stats: RuntimeQueueStats,
    ) -> Option<RuntimeCommandLane> {
        self.lane_order
            .iter()
            .copied()
            .find(|lane| stats.pending_lanes.count(*lane) > 0)
    }

    pub(in crate::next_core) fn dequeue_next(
        self,
        queue: &mut RuntimeCommandQueue,
    ) -> Option<RuntimeQueuedCommand> {
        for lane in self.lane_order {
            if let Some(queued) = queue.dequeue_lane(*lane) {
                return Some(queued);
            }
        }
        None
    }
}

impl Default for RuntimeSchedulePolicy {
    fn default() -> Self {
        Self::input_first()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_core::runtime::{
        command::{RuntimeCommand, RuntimeQueuePolicy},
        queue::RuntimeCommandQueue,
    };

    fn queue() -> RuntimeCommandQueue {
        RuntimeCommandQueue::new(RuntimeQueuePolicy {
            max_pending_commands: 4,
            max_pending_input_bytes: 8,
            max_render_wakeups_per_second: 120,
        })
    }

    #[test]
    fn input_first_policy_selects_input_over_older_screen_read() {
        let mut queue = queue();
        queue
            .enqueue(RuntimeCommand::ReadScreen { pane_id: 1 })
            .unwrap();
        queue
            .enqueue(RuntimeCommand::WriteInput {
                pane_id: 1,
                text: "x".to_string(),
            })
            .unwrap();

        assert_eq!(
            RuntimeSchedulePolicy::input_first().select_lane(queue.stats()),
            Some(RuntimeCommandLane::Input)
        );

        let queued = RuntimeSchedulePolicy::input_first()
            .dequeue_next(&mut queue)
            .expect("scheduled command");
        assert_eq!(queued.lane, RuntimeCommandLane::Input);
        assert_eq!(queue.stats().pending_lanes.screen, 1);
    }

    #[test]
    fn input_first_policy_falls_back_to_screen_when_input_is_empty() {
        let mut queue = queue();
        queue
            .enqueue(RuntimeCommand::ReadScreen { pane_id: 1 })
            .unwrap();

        assert_eq!(
            RuntimeSchedulePolicy::input_first().select_lane(queue.stats()),
            Some(RuntimeCommandLane::Screen)
        );
    }
}
