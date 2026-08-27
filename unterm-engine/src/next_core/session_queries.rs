use super::{process_tree, session_handles};
use crate::ShellSnapshot;
use anyhow::Result;

pub(super) fn shell_snapshot(pane_id: usize) -> Result<ShellSnapshot> {
    let handles = session_handles::shell_current(pane_id)?;
    let mut shell = handles.shell;

    let reported = handles.screen.lock().current_dir();
    let existing = shell.cwd.take();
    shell.cwd = process_tree::resolve_cwd(reported, existing, handles.root_pid);
    Ok(shell)
}

pub(super) fn output(pane_id: usize) -> Result<String> {
    let output = session_handles::output_current(pane_id)?;

    let text = output.lock().clone();
    Ok(text)
}

#[cfg(test)]
pub(super) fn bracketed_paste_enabled(pane_id: usize) -> Result<bool> {
    let screen = session_handles::screen_current(pane_id)?;

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
