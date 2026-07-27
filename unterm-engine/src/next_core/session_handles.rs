use super::{
    activity::SessionIoActivity, state, NextCoreRecording, NextCoreScreen, NextCoreSession,
    NextCoreState,
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

pub(super) struct ScrollbackHandles {
    pub(super) screen: Arc<Mutex<NextCoreScreen>>,
    pub(super) output: Arc<Mutex<String>>,
    pub(super) activity: Arc<Mutex<SessionIoActivity>>,
    pub(super) cols: usize,
    pub(super) rows: usize,
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

pub(super) fn output_current(pane_id: usize) -> Result<Arc<Mutex<String>>> {
    let state = state().read();
    output(&state, pane_id)
}

pub(super) fn screen(state: &NextCoreState, pane_id: usize) -> Result<Arc<Mutex<NextCoreScreen>>> {
    Ok(Arc::clone(&session(state, pane_id)?.screen))
}

pub(super) fn screen_current(pane_id: usize) -> Result<Arc<Mutex<NextCoreScreen>>> {
    let state = state().read();
    screen(&state, pane_id)
}

pub(super) fn activity(
    state: &NextCoreState,
    pane_id: usize,
) -> Result<Arc<Mutex<SessionIoActivity>>> {
    Ok(Arc::clone(&session(state, pane_id)?.activity))
}

pub(super) fn activity_current(pane_id: usize) -> Result<Arc<Mutex<SessionIoActivity>>> {
    let state = state().read();
    activity(&state, pane_id)
}

pub(super) fn screen_activity(
    state: &NextCoreState,
    pane_id: usize,
) -> Result<(Arc<Mutex<NextCoreScreen>>, Arc<Mutex<SessionIoActivity>>)> {
    let session = session(state, pane_id)?;
    Ok((Arc::clone(&session.screen), Arc::clone(&session.activity)))
}

pub(super) fn screen_activity_current(
    pane_id: usize,
) -> Result<(Arc<Mutex<NextCoreScreen>>, Arc<Mutex<SessionIoActivity>>)> {
    let state = state().read();
    screen_activity(&state, pane_id)
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

pub(super) fn input_current(pane_id: usize) -> Result<InputHandles> {
    let state = state().read();
    input(&state, pane_id)
}

pub(super) fn shell(state: &NextCoreState, pane_id: usize) -> Result<ShellHandles> {
    let session = session(state, pane_id)?;
    Ok(ShellHandles {
        shell: session.snapshot.shell.clone(),
        screen: Arc::clone(&session.screen),
        root_pid: session.root_pid,
    })
}

pub(super) fn shell_current(pane_id: usize) -> Result<ShellHandles> {
    let state = state().read();
    shell(&state, pane_id)
}

pub(super) fn scrollback(state: &NextCoreState, pane_id: usize) -> Result<ScrollbackHandles> {
    let session = session(state, pane_id)?;
    Ok(ScrollbackHandles {
        screen: Arc::clone(&session.screen),
        output: Arc::clone(&session.output),
        activity: Arc::clone(&session.activity),
        cols: session.snapshot.cols,
        rows: session.snapshot.rows,
    })
}

pub(super) fn scrollback_current(pane_id: usize) -> Result<ScrollbackHandles> {
    let state = state().read();
    scrollback(&state, pane_id)
}

pub(super) fn recording(state: &NextCoreState, pane_id: usize) -> Result<RecordingHandles> {
    let session = session(state, pane_id)?;
    Ok(RecordingHandles {
        recording: Arc::clone(&session.recording),
        project_path: session.snapshot.shell.cwd.clone(),
    })
}

pub(super) fn recording_current(pane_id: usize) -> Result<RecordingHandles> {
    let state = state().read();
    recording(&state, pane_id)
}

pub(super) fn recording_optional(
    state: &NextCoreState,
    pane_id: usize,
) -> Option<Arc<Mutex<Option<NextCoreRecording>>>> {
    session(state, pane_id)
        .ok()
        .map(|session| Arc::clone(&session.recording))
}

pub(super) fn recording_optional_current(
    pane_id: usize,
) -> Option<Arc<Mutex<Option<NextCoreRecording>>>> {
    let state = state().read();
    recording_optional(&state, pane_id)
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

    #[test]
    fn optional_recording_reports_none_for_missing_session() {
        assert!(recording_optional(&NextCoreState::default(), 42).is_none());
    }
}
