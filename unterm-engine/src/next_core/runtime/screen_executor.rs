use super::super::screen_dispatch;
use super::command::{RuntimeCommand, RuntimeCommandClass};
use crate::{
    CursorSnapshot, RenderFrameSnapshot, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextSnapshot, StyledScreenSnapshot, StyledScrollbackSnapshot,
};
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
        RuntimeCommand::ScrollViewportBy { pane_id, delta } => {
            screen_dispatch::scroll_viewport_by(pane_id, delta)
        }
        RuntimeCommand::ScrollViewportToPrompt { pane_id, amount } => {
            screen_dispatch::scroll_viewport_to_prompt(pane_id, amount)
        }
        RuntimeCommand::EraseScrollback {
            pane_id,
            include_viewport,
        } => screen_dispatch::erase_scrollback(pane_id, include_viewport),
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

pub(in crate::next_core) fn execute_styled_screen(
    command: RuntimeCommand,
) -> Result<StyledScreenSnapshot> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ReadStyledScreen { pane_id } => {
            screen_dispatch::read_styled_viewport(pane_id)
        }
        _ => bail!("runtime screen executor expected styled screen read command"),
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

pub(in crate::next_core) fn execute_visible_text(command: RuntimeCommand) -> Result<String> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ReadVisibleText { pane_id } => screen_dispatch::read_visible_text(pane_id),
        _ => bail!("runtime screen executor expected visible text read command"),
    }
}

pub(in crate::next_core) fn execute_lines(command: RuntimeCommand) -> Result<Vec<ScreenLine>> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ReadLines {
            pane_id,
            start,
            count,
        } => screen_dispatch::read_lines(pane_id, start, count),
        _ => bail!("runtime screen executor expected line range read command"),
    }
}

pub(in crate::next_core) fn execute_scrollback(command: RuntimeCommand) -> Result<Vec<String>> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ReadScrollback { pane_id, limit } => {
            screen_dispatch::read_scrollback(pane_id, limit)
        }
        _ => bail!("runtime screen executor expected scrollback read command"),
    }
}

pub(in crate::next_core) fn execute_scrollback_text(
    command: RuntimeCommand,
) -> Result<ScrollbackTextSnapshot> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ReadScrollbackText { pane_id, request } => {
            screen_dispatch::read_scrollback_text(pane_id, request)
        }
        _ => bail!("runtime screen executor expected scrollback text read command"),
    }
}

pub(in crate::next_core) fn execute_styled_scrollback(
    command: RuntimeCommand,
) -> Result<StyledScrollbackSnapshot> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ReadStyledScrollback { pane_id, request } => {
            screen_dispatch::read_styled_scrollback(pane_id, request)
        }
        _ => bail!("runtime screen executor expected styled scrollback read command"),
    }
}

pub(in crate::next_core) fn execute_search(
    command: RuntimeCommand,
) -> Result<Vec<ScreenSearchMatch>> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::SearchScreen {
            pane_id,
            pattern,
            max_results,
        } => screen_dispatch::search(pane_id, &pattern, max_results),
        _ => bail!("runtime screen executor expected screen search command"),
    }
}

pub(in crate::next_core) fn execute_cursor(command: RuntimeCommand) -> Result<CursorSnapshot> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::Cursor { pane_id } => screen_dispatch::cursor(pane_id),
        _ => bail!("runtime screen executor expected cursor read command"),
    }
}

pub(in crate::next_core) fn execute_pane_modes(
    command: RuntimeCommand,
) -> Result<crate::PaneModesSnapshot> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::PaneModes { pane_id } => screen_dispatch::pane_modes(pane_id),
        _ => bail!("runtime screen executor expected pane-modes read command"),
    }
}

pub(in crate::next_core) fn execute_screen_revision(command: RuntimeCommand) -> Result<u64> {
    ensure_screen_read(&command)?;

    match command {
        RuntimeCommand::ScreenRevision { pane_id } => screen_dispatch::screen_revision(pane_id),
        _ => bail!("runtime screen executor expected screen-revision read command"),
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
    fn rejects_non_styled_screen_reads_before_dispatch() {
        let err = execute_styled_screen(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err
            .to_string()
            .contains("expected styled screen read command"));
    }

    #[test]
    fn rejects_non_visible_text_reads_before_dispatch() {
        let err = execute_visible_text(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err
            .to_string()
            .contains("expected visible text read command"));
    }

    #[test]
    fn rejects_non_line_range_reads_before_dispatch() {
        let err = execute_lines(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err.to_string().contains("expected line range read command"));
    }

    #[test]
    fn rejects_non_scrollback_reads_before_dispatch() {
        let err = execute_scrollback(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err.to_string().contains("expected scrollback read command"));
    }

    #[test]
    fn rejects_non_screen_mutation_before_dispatch() {
        let err = execute_screen_mutation(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("screen read should be rejected");

        assert!(err.to_string().contains("not a screen mutation command"));
    }

    #[test]
    fn rejects_non_scrollback_text_reads_before_dispatch() {
        let err = execute_scrollback_text(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err
            .to_string()
            .contains("expected scrollback text read command"));
    }

    #[test]
    fn rejects_non_styled_scrollback_reads_before_dispatch() {
        let err = execute_styled_scrollback(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err
            .to_string()
            .contains("expected styled scrollback read command"));
    }

    #[test]
    fn rejects_non_search_reads_before_dispatch() {
        let err = execute_search(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err.to_string().contains("expected screen search command"));
    }

    #[test]
    fn rejects_non_cursor_reads_before_dispatch() {
        let err = execute_cursor(RuntimeCommand::ReadScreen { pane_id: 1 })
            .expect_err("plain screen read should be rejected");

        assert!(err.to_string().contains("expected cursor read command"));
    }
}
