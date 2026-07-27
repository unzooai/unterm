use super::super::screen_dispatch;
use super::command::{RuntimeCommand, RuntimeCommandClass};
use crate::{RenderFrameSnapshot, ScreenSnapshot};
use anyhow::{bail, Result};

fn ensure_screen_read(command: &RuntimeCommand) -> Result<()> {
    if command.class() != RuntimeCommandClass::ScreenRead {
        bail!(
            "runtime command is not a screen read command: {:?}",
            command.class()
        );
    }
    Ok(())
}

pub(in crate::next_core) fn execute_screen_mutation(command: RuntimeCommand) -> Result<()> {
    if command.class() != RuntimeCommandClass::ScreenMutation {
        bail!(
            "runtime command is not a screen mutation command: {:?}",
            command.class()
        );
    }

    match command {
        RuntimeCommand::ScrollViewport { pane_id, target } => {
            screen_dispatch::scroll_viewport_to(pane_id, target)
        }
        _ => bail!("runtime screen executor expected screen mutation command"),
    }
}

pub(in crate::next_core) fn execute_screen(command: RuntimeCommand) -> Result<ScreenSnapshot> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ReadScreen { pane_id } => screen_dispatch::read_plain_viewport(pane_id),
        _ => bail!("runtime screen executor expected plain screen read command"),
    }
}

pub(in crate::next_core) fn execute_render_frame(
    command: RuntimeCommand,
) -> Result<RenderFrameSnapshot> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ReadRenderFrame {
            pane_id,
            since_revision,
        } => screen_dispatch::read_render_frame(pane_id, since_revision),
        _ => bail!("runtime screen executor expected render-frame read command"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_screen_read_before_dispatch() {
        let err = execute_render_frame(RuntimeCommand::WriteInput {
            pane_id: 1,
            text: "x".to_string(),
        })
        .expect_err("non-screen read should be rejected");

        assert!(err.to_string().contains("not a screen read command"));
    }

    #[test]
    fn rejects_non_render_screen_reads_before_dispatch() {
        let err = execute_render_frame(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err
            .to_string()
            .contains("expected render-frame read command"));
    }

    #[test]
    fn rejects_non_plain_screen_reads_before_dispatch() {
        let err = execute_screen(RuntimeCommand::ReadRenderFrame {
            pane_id: 1,
            since_revision: None,
        })
        .expect_err("render frame read should be rejected");

        assert!(err
            .to_string()
            .contains("expected plain screen read command"));
    }

    #[test]
    fn rejects_non_screen_mutation_before_dispatch() {
        let err = execute_screen_mutation(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("screen read should be rejected");

        assert!(err.to_string().contains("not a screen mutation command"));
    }
}
