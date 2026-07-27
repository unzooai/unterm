use super::{
    activity::SessionIoActivity, session_registry, NextCoreRecording, NextCoreRuntime,
    NextCoreScreen,
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

pub(super) fn output(state: &NextCoreRuntime, pane_id: usize) -> Result<Arc<Mutex<String>>> {
    Ok(Arc::clone(
        &session_registry::session(state, pane_id)?.output,
    ))
}

pub(super) fn output_current(pane_id: usize) -> Result<Arc<Mutex<String>>> {
    session_registry::with_current_runtime(|state| output(state, pane_id))
}

pub(super) fn screen(
    state: &NextCoreRuntime,
    pane_id: usize,
) -> Result<Arc<Mutex<NextCoreScreen>>> {
    Ok(Arc::clone(
        &session_registry::session(state, pane_id)?.screen,
    ))
}

pub(super) fn screen_current(pane_id: usize) -> Result<Arc<Mutex<NextCoreScreen>>> {
    session_registry::with_current_runtime(|state| screen(state, pane_id))
}

pub(super) fn activity(
    state: &NextCoreRuntime,
    pane_id: usize,
) -> Result<Arc<Mutex<SessionIoActivity>>> {
    Ok(Arc::clone(
        &session_registry::session(state, pane_id)?.activity,
    ))
}

pub(super) fn activity_current(pane_id: usize) -> Result<Arc<Mutex<SessionIoActivity>>> {
    session_registry::with_current_runtime(|state| activity(state, pane_id))
}

pub(super) fn screen_activity(
    state: &NextCoreRuntime,
    pane_id: usize,
) -> Result<(Arc<Mutex<NextCoreScreen>>, Arc<Mutex<SessionIoActivity>>)> {
    let session = session_registry::session(state, pane_id)?;
    Ok((Arc::clone(&session.screen), Arc::clone(&session.activity)))
}

pub(super) fn screen_activity_current(
    pane_id: usize,
) -> Result<(Arc<Mutex<NextCoreScreen>>, Arc<Mutex<SessionIoActivity>>)> {
    session_registry::with_current_runtime(|state| screen_activity(state, pane_id))
}

pub(super) fn input(state: &NextCoreRuntime, pane_id: usize) -> Result<InputHandles> {
    let session = session_registry::session(state, pane_id)?;
    let screen = session.screen.lock();
    Ok(InputHandles {
        writer: Arc::clone(&session.writer),
        activity: Arc::clone(&session.activity),
        application_cursor_keys: screen.application_cursor_keys,
        bracketed_paste: screen.bracketed_paste,
    })
}

pub(super) fn input_current(pane_id: usize) -> Result<InputHandles> {
    session_registry::with_current_runtime(|state| input(state, pane_id))
}

pub(super) fn shell(state: &NextCoreRuntime, pane_id: usize) -> Result<ShellHandles> {
    let session = session_registry::session(state, pane_id)?;
    Ok(ShellHandles {
        shell: session.snapshot.shell.clone(),
        screen: Arc::clone(&session.screen),
        root_pid: session.root_pid,
    })
}

pub(super) fn shell_current(pane_id: usize) -> Result<ShellHandles> {
    session_registry::with_current_runtime(|state| shell(state, pane_id))
}

pub(super) fn scrollback(state: &NextCoreRuntime, pane_id: usize) -> Result<ScrollbackHandles> {
    let session = session_registry::session(state, pane_id)?;
    Ok(ScrollbackHandles {
        screen: Arc::clone(&session.screen),
        output: Arc::clone(&session.output),
        activity: Arc::clone(&session.activity),
        cols: session.snapshot.cols,
        rows: session.snapshot.rows,
    })
}

pub(super) fn scrollback_current(pane_id: usize) -> Result<ScrollbackHandles> {
    session_registry::with_current_runtime(|state| scrollback(state, pane_id))
}

pub(super) fn recording(state: &NextCoreRuntime, pane_id: usize) -> Result<RecordingHandles> {
    let session = session_registry::session(state, pane_id)?;
    Ok(RecordingHandles {
        recording: Arc::clone(&session.recording),
        project_path: session.snapshot.shell.cwd.clone(),
    })
}

pub(super) fn recording_current(pane_id: usize) -> Result<RecordingHandles> {
    session_registry::with_current_runtime(|state| recording(state, pane_id))
}

pub(super) fn recording_optional(
    state: &NextCoreRuntime,
    pane_id: usize,
) -> Option<Arc<Mutex<Option<NextCoreRecording>>>> {
    session_registry::session(state, pane_id)
        .ok()
        .map(|session| Arc::clone(&session.recording))
}

pub(super) fn recording_optional_current(
    pane_id: usize,
) -> Option<Arc<Mutex<Option<NextCoreRecording>>>> {
    session_registry::with_current_runtime(|state| recording_optional(state, pane_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_session_reports_pane_id() {
        let err = match screen(&NextCoreRuntime::default(), 42) {
            Ok(_) => panic!("expected missing session error"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("next-core session 42 not found"));
    }

    #[test]
    fn optional_recording_reports_none_for_missing_session() {
        assert!(recording_optional(&NextCoreRuntime::default(), 42).is_none());
    }
}
