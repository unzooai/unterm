use super::{
    activity::SessionIoActivity, NextCoreRecording, NextCoreScreen, NextCoreSession, NextCoreState,
};
use crate::ShellSnapshot;
use anyhow::Result;
use parking_lot::Mutex;
use std::io::Write;
use std::sync::Arc;

pub(super) struct InputHandles {
    pub(super) writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub(super) activity: Arc<Mutex<SessionIoActivity>>,
    pub(super) application_cursor_keys: bool,
    pub(super) bracketed_paste: bool,
}

pub(super) struct RecordingHandles {
    pub(super) recording: Arc<Mutex<Option<NextCoreRecording>>>,
    pub(super) project_path: Option<String>,
}

pub(super) struct ShellHandles {
    pub(super) shell: ShellSnapshot,
    pub(super) screen: Arc<Mutex<NextCoreScreen>>,
    pub(super) root_pid: Option<u32>,
}

fn session(state: &NextCoreState, pane_id: usize) -> Result<&NextCoreSession> {
    state
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
        .ok_or_else(|| anyhow::anyhow!("next-core session {pane_id} not found"))
}

pub(super) fn output(state: &NextCoreState, pane_id: usize) -> Result<Arc<Mutex<String>>> {
    Ok(Arc::clone(&session(state, pane_id)?.output))
}

pub(super) fn screen(state: &NextCoreState, pane_id: usize) -> Result<Arc<Mutex<NextCoreScreen>>> {
    Ok(Arc::clone(&session(state, pane_id)?.screen))
}

pub(super) fn activity(
    state: &NextCoreState,
    pane_id: usize,
) -> Result<Arc<Mutex<SessionIoActivity>>> {
    Ok(Arc::clone(&session(state, pane_id)?.activity))
}

pub(super) fn screen_activity(
    state: &NextCoreState,
    pane_id: usize,
) -> Result<(Arc<Mutex<NextCoreScreen>>, Arc<Mutex<SessionIoActivity>>)> {
    let session = session(state, pane_id)?;
    Ok((Arc::clone(&session.screen), Arc::clone(&session.activity)))
}

pub(super) fn input(state: &NextCoreState, pane_id: usize) -> Result<InputHandles> {
    let session = session(state, pane_id)?;
    let screen = session.screen.lock();
    Ok(InputHandles {
        writer: Arc::clone(&session.writer),
        activity: Arc::clone(&session.activity),
        application_cursor_keys: screen.application_cursor_keys,
        bracketed_paste: screen.bracketed_paste,
    })
}

pub(super) fn shell(state: &NextCoreState, pane_id: usize) -> Result<ShellHandles> {
    let session = session(state, pane_id)?;
    Ok(ShellHandles {
        shell: session.snapshot.shell.clone(),
        screen: Arc::clone(&session.screen),
        root_pid: session.root_pid,
    })
}

pub(super) fn recording(state: &NextCoreState, pane_id: usize) -> Result<RecordingHandles> {
    let session = session(state, pane_id)?;
    Ok(RecordingHandles {
        recording: Arc::clone(&session.recording),
        project_path: session.snapshot.shell.cwd.clone(),
    })
}

pub(super) fn recording_optional(
    state: &NextCoreState,
    pane_id: usize,
) -> Option<Arc<Mutex<Option<NextCoreRecording>>>> {
    state
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
        .map(|session| Arc::clone(&session.recording))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_session_reports_pane_id() {
        let err = match screen(&NextCoreState::default(), 42) {
            Ok(_) => panic!("expected missing session error"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("next-core session 42 not found"));
    }
}
