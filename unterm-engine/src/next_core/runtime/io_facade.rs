use super::super::{input_dispatch, screen_dispatch};
use super::command::{RuntimeCommand, RuntimeCommandClass};
use crate::{
    CursorSnapshot, RenderFrameSnapshot, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, StyledScreenSnapshot, StyledScrollbackSnapshot,
};
use anyhow::{bail, Result};

pub(in crate::next_core) fn scroll_viewport_to(pane_id: usize, target: isize) -> Result<()> {
    screen_dispatch::scroll_viewport_to(pane_id, target)
}

pub(in crate::next_core) fn read_screen(pane_id: usize) -> Result<ScreenSnapshot> {
    screen_dispatch::read_plain_viewport(pane_id)
}

pub(in crate::next_core) fn read_styled_screen(pane_id: usize) -> Result<StyledScreenSnapshot> {
    screen_dispatch::read_styled_viewport(pane_id)
}

pub(in crate::next_core) fn read_render_frame(
    pane_id: usize,
    since_revision: Option<u64>,
) -> Result<RenderFrameSnapshot> {
    screen_dispatch::read_render_frame(pane_id, since_revision)
}

pub(in crate::next_core) fn read_visible_text(pane_id: usize) -> Result<String> {
    screen_dispatch::read_visible_text(pane_id)
}

pub(in crate::next_core) fn read_lines(
    pane_id: usize,
    start: i64,
    count: usize,
) -> Result<Vec<ScreenLine>> {
    screen_dispatch::read_lines(pane_id, start, count)
}

pub(in crate::next_core) fn read_scrollback(pane_id: usize, limit: usize) -> Result<Vec<String>> {
    screen_dispatch::read_scrollback(pane_id, limit)
}

pub(in crate::next_core) fn read_scrollback_text(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<ScrollbackTextSnapshot> {
    screen_dispatch::read_scrollback_text(pane_id, request)
}

pub(in crate::next_core) fn read_styled_scrollback(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<StyledScrollbackSnapshot> {
    screen_dispatch::read_styled_scrollback(pane_id, request)
}

pub(in crate::next_core) fn search_screen(
    pane_id: usize,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<ScreenSearchMatch>> {
    screen_dispatch::search(pane_id, pattern, max_results)
}

pub(in crate::next_core) fn cursor(pane_id: usize) -> Result<CursorSnapshot> {
    screen_dispatch::cursor(pane_id)
}

pub(in crate::next_core) fn write_input(pane_id: usize, input: &str) -> Result<()> {
    execute_input_command(RuntimeCommand::WriteInput {
        pane_id,
        text: input.to_string(),
    })
}

pub(in crate::next_core) fn paste_input(pane_id: usize, text: &str) -> Result<()> {
    execute_input_command(RuntimeCommand::PasteInput {
        pane_id,
        text: text.to_string(),
    })
}

fn execute_input_command(command: RuntimeCommand) -> Result<()> {
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
}
