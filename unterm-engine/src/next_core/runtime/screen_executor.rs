use super::super::screen_dispatch;
use super::command::{RuntimeCommand, RuntimeCommandClass};
use crate::RenderFrameSnapshot;
use anyhow::{bail, Result};

pub(in crate::next_core) fn execute_render_frame(
    command: RuntimeCommand,
) -> Result<RenderFrameSnapshot> {
    if command.class() != RuntimeCommandClass::ScreenRead {
        bail!(
            "runtime command is not a screen read command: {:?}",
            command.class()
        );
    }

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
}
