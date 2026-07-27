#![allow(dead_code)]

use super::{
    command::{RuntimeCommand, RuntimeCommandClass},
    input_executor, screen_executor, status_executor,
};
use crate::{
    CursorSnapshot, EngineHealthSnapshot, RenderFrameSnapshot, ScreenLine, ScreenSearchMatch,
    ScreenSnapshot, ScrollbackTextSnapshot, SessionActivitySnapshot, ShellSnapshot,
    StyledScreenSnapshot, StyledScrollbackSnapshot,
};
use anyhow::{bail, Result};

#[derive(Debug)]
pub(in crate::next_core) enum RuntimeDispatchResult {
    Unit,
    Screen(ScreenSnapshot),
    StyledScreen(StyledScreenSnapshot),
    RenderFrame(RenderFrameSnapshot),
    VisibleText(String),
    Lines(Vec<ScreenLine>),
    Scrollback(Vec<String>),
    ScrollbackText(ScrollbackTextSnapshot),
    StyledScrollback(StyledScrollbackSnapshot),
    Search(Vec<ScreenSearchMatch>),
    Cursor(CursorSnapshot),
    Output(String),
    ShellSnapshot(ShellSnapshot),
    SessionActivity(SessionActivitySnapshot),
    HealthSnapshot(EngineHealthSnapshot),
}

pub(in crate::next_core) fn execute(command: RuntimeCommand) -> Result<RuntimeDispatchResult> {
    match command.class() {
        RuntimeCommandClass::Input => {
            input_executor::execute(command)?;
            Ok(RuntimeDispatchResult::Unit)
        }
        RuntimeCommandClass::ScreenMutation => {
            screen_executor::execute_screen_mutation(command)?;
            Ok(RuntimeDispatchResult::Unit)
        }
        RuntimeCommandClass::ScreenRead => execute_screen_read(command),
        RuntimeCommandClass::Status => execute_status(command),
        RuntimeCommandClass::SessionLifecycle | RuntimeCommandClass::Recording => bail!(
            "runtime dispatch does not yet execute {:?} commands",
            command.class()
        ),
    }
}

fn execute_status(command: RuntimeCommand) -> Result<RuntimeDispatchResult> {
    match command {
        RuntimeCommand::RawOutput { .. } => Ok(RuntimeDispatchResult::Output(
            status_executor::execute_output(command)?,
        )),
        RuntimeCommand::ShellSnapshot { .. } => Ok(RuntimeDispatchResult::ShellSnapshot(
            status_executor::execute_shell_snapshot(command)?,
        )),
        RuntimeCommand::SessionActivity { .. } => Ok(RuntimeDispatchResult::SessionActivity(
            status_executor::execute_session_activity(command)?,
        )),
        RuntimeCommand::HealthSnapshot => Ok(RuntimeDispatchResult::HealthSnapshot(
            status_executor::execute_health_snapshot(command)?,
        )),
        _ => bail!("runtime dispatch expected status command"),
    }
}

fn execute_screen_read(command: RuntimeCommand) -> Result<RuntimeDispatchResult> {
    match command {
        RuntimeCommand::ReadScreen { .. } => Ok(RuntimeDispatchResult::Screen(
            screen_executor::execute_screen(command)?,
        )),
        RuntimeCommand::ReadStyledScreen { .. } => Ok(RuntimeDispatchResult::StyledScreen(
            screen_executor::execute_styled_screen(command)?,
        )),
        RuntimeCommand::ReadRenderFrame { .. } => Ok(RuntimeDispatchResult::RenderFrame(
            screen_executor::execute_render_frame(command)?,
        )),
        RuntimeCommand::ReadVisibleText { .. } => Ok(RuntimeDispatchResult::VisibleText(
            screen_executor::execute_visible_text(command)?,
        )),
        RuntimeCommand::ReadLines { .. } => Ok(RuntimeDispatchResult::Lines(
            screen_executor::execute_lines(command)?,
        )),
        RuntimeCommand::ReadScrollback { .. } => Ok(RuntimeDispatchResult::Scrollback(
            screen_executor::execute_scrollback(command)?,
        )),
        RuntimeCommand::ReadScrollbackText { .. } => Ok(RuntimeDispatchResult::ScrollbackText(
            screen_executor::execute_scrollback_text(command)?,
        )),
        RuntimeCommand::ReadStyledScrollback { .. } => Ok(RuntimeDispatchResult::StyledScrollback(
            screen_executor::execute_styled_scrollback(command)?,
        )),
        RuntimeCommand::SearchScreen { .. } => Ok(RuntimeDispatchResult::Search(
            screen_executor::execute_search(command)?,
        )),
        RuntimeCommand::Cursor { .. } => Ok(RuntimeDispatchResult::Cursor(
            screen_executor::execute_cursor(command)?,
        )),
        _ => bail!("runtime dispatch expected screen read command"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_core::runtime::test_facade;

    #[test]
    fn dispatch_rejects_unimplemented_lifecycle_commands() {
        let err = execute(RuntimeCommand::FocusSession { pane_id: 1 })
            .expect_err("lifecycle command should wait for response-channel plumbing");

        assert!(err
            .to_string()
            .contains("does not yet execute SessionLifecycle"));
    }

    #[test]
    fn dispatch_routes_status_commands_to_status_executor() {
        test_facade::reset();

        let result = execute(RuntimeCommand::HealthSnapshot).expect("health should dispatch");

        assert!(matches!(result, RuntimeDispatchResult::HealthSnapshot(_)));
    }

    #[test]
    fn dispatch_routes_screen_reads_to_screen_executor() {
        test_facade::reset();

        let err = execute(RuntimeCommand::ReadScreen { pane_id: 404 })
            .expect_err("missing pane should still come from screen executor");

        assert!(err.to_string().contains("next-core session 404 not found"));
    }
}
