use super::super::{session_registry, session_runtime};
use super::{command::RuntimeCommand, with_current_mut};
use anyhow::{bail, Result};

pub(in crate::next_core) fn execute(command: RuntimeCommand) -> Result<()> {
    match command {
        RuntimeCommand::FocusSession { pane_id } => {
            with_current_mut(|state| session_registry::focus(state, pane_id))
        }
        RuntimeCommand::ResizeSession {
            pane_id,
            cols,
            rows,
        } => with_current_mut(|state| {
            let session = session_registry::session_mut(state, pane_id)?;
            session_runtime::resize_session(session, cols, rows)
        }),
        RuntimeCommand::DestroySession { pane_id } => {
            with_current_mut(|state| session_registry::destroy(state, pane_id))
        }
        _ => bail!("runtime session executor expected focus/resize/destroy command"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_lifecycle_command_shape() {
        let err =
            execute(RuntimeCommand::HealthSnapshot).expect_err("wrong command shape should fail");

        assert!(err.to_string().contains("expected focus/resize/destroy"));
    }

    #[test]
    fn reports_missing_session_from_registry() {
        let err = execute(RuntimeCommand::FocusSession { pane_id: 404 })
            .expect_err("missing pane should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
    }
}
