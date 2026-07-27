use super::{process_tree, session_handles, state};
use crate::ShellSnapshot;
use anyhow::Result;

pub(super) fn shell_snapshot(pane_id: usize) -> Result<ShellSnapshot> {
    let handles = {
        let state = state().read();
        session_handles::shell(&state, pane_id)?
    };
    let mut shell = handles.shell;

    if let Some(cwd) = handles.screen.lock().current_dir() {
        shell.cwd = Some(cwd);
        return Ok(shell);
    }

    if shell.cwd.is_none() {
        if let Some(process) = process_tree::snapshot(handles.root_pid, &shell.process_name) {
            shell.cwd = process.foreground_cwd.or(process.root_cwd);
        }
    }
    Ok(shell)
}

pub(super) fn output(pane_id: usize) -> Result<String> {
    let output = {
        let state = state().read();
        session_handles::output(&state, pane_id)?
    };

    let text = output.lock().clone();
    Ok(text)
}

#[cfg(test)]
pub(super) fn bracketed_paste_enabled(pane_id: usize) -> Result<bool> {
    let screen = {
        let state = state().read();
        session_handles::screen(&state, pane_id)?
    };

    let enabled = screen.lock().bracketed_paste;
    Ok(enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_reports_missing_session() {
        let err = output(404).expect_err("missing session should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
    }

    #[test]
    fn bracketed_paste_reports_missing_session() {
        let err = bracketed_paste_enabled(404).expect_err("missing session should fail");

        assert!(err.to_string().contains("next-core session 404 not found"));
    }
}
