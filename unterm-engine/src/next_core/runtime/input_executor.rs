use super::super::input_dispatch;
use super::command::{RuntimeCommand, RuntimeCommandClass};
use anyhow::{bail, Result};

pub(in crate::next_core) fn execute(command: RuntimeCommand) -> Result<()> {
    if command.class() != RuntimeCommandClass::Input {
        bail!(
            "runtime command is not an input command: {:?}",
            command.class()
        );
    }

    match command {
        RuntimeCommand::WriteInput { pane_id, text } => input_dispatch::write(pane_id, &text),
        RuntimeCommand::PasteInput { pane_id, text } => input_dispatch::paste(pane_id, &text),
        _ => unreachable!("input command class must be write or paste"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_core::runtime::{
        command::RuntimeQueuePolicy,
        queue::{RuntimeCommandQueue, RuntimeQueueRejection},
    };

    #[test]
    fn input_commands_keep_runtime_classification() {
        let write = RuntimeCommand::WriteInput {
            pane_id: 1,
            text: "abc".to_string(),
        };
        let paste = RuntimeCommand::PasteInput {
            pane_id: 2,
            text: "abcdef".to_string(),
        };

        assert_eq!(write.class(), RuntimeCommandClass::Input);
        assert_eq!(write.pane_id(), Some(1));
        assert_eq!(write.input_bytes(), 3);
        assert_eq!(paste.class(), RuntimeCommandClass::Input);
        assert_eq!(paste.pane_id(), Some(2));
        assert_eq!(paste.input_bytes(), 6);
    }

    #[test]
    fn input_commands_can_be_backpressured_before_dispatch() {
        let mut queue = RuntimeCommandQueue::new(RuntimeQueuePolicy {
            max_pending_commands: 4,
            max_pending_input_bytes: 4,
            max_render_wakeups_per_second: 120,
        });

        queue
            .enqueue(RuntimeCommand::WriteInput {
                pane_id: 1,
                text: "1234".to_string(),
            })
            .unwrap();
        let err = queue
            .enqueue(RuntimeCommand::PasteInput {
                pane_id: 1,
                text: "5".to_string(),
            })
            .expect_err("input byte budget should reject extra input");

        assert_eq!(
            err,
            RuntimeQueueRejection::InputBackpressure {
                pending_input_bytes: 4,
                command_input_bytes: 1,
                max_pending_input_bytes: 4,
            }
        );
    }

    #[test]
    fn rejects_non_input_commands_before_dispatch() {
        let err = execute(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("non-input command should be rejected");

        assert!(err.to_string().contains("not an input command"));
    }
}
