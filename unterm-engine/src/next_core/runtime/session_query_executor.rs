use super::super::session_snapshots;
use super::{command::RuntimeCommand, with_current_mut};
use crate::SessionSnapshot;
use anyhow::{bail, Result};

pub(in crate::next_core) fn execute_list(command: RuntimeCommand) -> Result<Vec<SessionSnapshot>> {
    match command {
        RuntimeCommand::ListSessions => Ok(with_current_mut(session_snapshots::list)),
        _ => bail!("runtime session query executor expected list-sessions command"),
    }
}

pub(in crate::next_core) fn execute_get(command: RuntimeCommand) -> Result<SessionSnapshot> {
    match command {
        RuntimeCommand::GetSession { pane_id } => {
            with_current_mut(|state| session_snapshots::get(state, pane_id))
        }
        _ => bail!("runtime session query executor expected get-session command"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_query_command_shape() {
        let err = execute_list(RuntimeCommand::HealthSnapshot)
            .expect_err("wrong query command should fail");

        assert!(err.to_string().contains("expected list-sessions command"));
    }

    #[test]
    fn get_reports_missing_session() {
        let err =
            execute_get(RuntimeCommand::GetSession { pane_id: 404 }).expect_err("missing session");

        assert!(err.to_string().contains("next-core session 404 not found"));
    }
}
