use super::{
    activity::SessionIoActivity, launch, pty_io, session_defaults, session_output,
    session_registry, state, NextCoreRecording, NextCoreScreen, NextCoreSession, NextCoreState,
};
use crate::{SessionSnapshot, ShellSnapshot};
use anyhow::Result;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub(super) fn pty_size(cols: usize, rows: usize) -> PtySize {
    PtySize {
        rows: rows.clamp(1, u16::MAX as usize) as u16,
        cols: cols.clamp(1, u16::MAX as usize) as u16,
        pixel_width: 0,
        pixel_height: 0,
    }
}

pub(super) fn resize(
    state: &mut NextCoreState,
    pane_id: usize,
    cols: usize,
    rows: usize,
) -> Result<()> {
    let session = session_registry::session_mut(state, pane_id)?;
    resize_session(session, cols, rows)
}

pub(super) fn resize_current(pane_id: usize, cols: usize, rows: usize) -> Result<()> {
    let mut state = state().write();
    resize(&mut state, pane_id, cols, rows)
}

fn resize_session(session: &mut NextCoreSession, cols: usize, rows: usize) -> Result<()> {
    session.master.lock().resize(pty_size(cols, rows))?;
    session.snapshot.cols = cols;
    session.snapshot.rows = rows;
    session.screen.lock().resize(cols, rows);
    Ok(())
}

pub(super) fn spawn(
    id: usize,
    title: String,
    cols: usize,
    rows: usize,
    command: portable_pty::CommandBuilder,
    cwd: Option<String>,
    launch_env_keys: Vec<String>,
) -> Result<NextCoreSession> {
    let label = launch::command_label(&command);
    let pair = native_pty_system().openpty(pty_size(cols, rows))?;
    let child = pair.slave.spawn_command(command)?;
    let root_pid = child.process_id();
    let reader = pair.master.try_clone_reader()?;
    let writer = Arc::new(Mutex::new(pair.master.take_writer()?));
    let output = Arc::new(Mutex::new(String::new()));
    let screen = Arc::new(Mutex::new(NextCoreScreen::new(cols, rows)));
    let recording = Arc::new(Mutex::new(None));
    let activity = Arc::new(Mutex::new(SessionIoActivity::new()));
    let dead = Arc::new(AtomicBool::new(false));
    let dead_reason = Arc::new(Mutex::new(None));
    spawn_reader_thread(
        id,
        Arc::clone(&output),
        Arc::clone(&screen),
        Arc::clone(&recording),
        Arc::clone(&activity),
        Arc::clone(&writer),
        Arc::clone(&dead),
        Arc::clone(&dead_reason),
        reader,
    );
    let shell = ShellSnapshot {
        shell_type: launch::shell_type(&label),
        process_name: label,
        cwd,
        launch_env_keys,
        launch_context: Default::default(),
    };

    Ok(NextCoreSession {
        snapshot: SessionSnapshot {
            id,
            title,
            cols,
            rows,
            scrollback_rows: 0,
            cursor: session_defaults::default_cursor(),
            is_dead: false,
            dead_reason: None,
            is_active: true,
            domain_id: 0,
            shell,
        },
        root_pid,
        master: Mutex::new(pair.master),
        child: Mutex::new(child),
        writer,
        output,
        screen,
        recording,
        activity,
        dead,
        dead_reason,
    })
}

fn spawn_reader_thread(
    pane_id: usize,
    output: Arc<Mutex<String>>,
    screen: Arc<Mutex<NextCoreScreen>>,
    recording: Arc<Mutex<Option<NextCoreRecording>>>,
    activity: Arc<Mutex<SessionIoActivity>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    dead: Arc<AtomicBool>,
    dead_reason: Arc<Mutex<Option<String>>>,
    mut reader: Box<dyn Read + Send>,
) {
    thread::Builder::new()
        .name(format!("next-core-pty-reader-{pane_id}"))
        .spawn(move || {
            let mut buf = [0u8; 8192];
            let mut pending_utf8 = Vec::new();
            let mut pending_terminal_query = String::new();
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        *dead_reason.lock() = Some("pty_reader_eof".to_string());
                        break;
                    }
                    Ok(n) => {
                        let Some(chunk) = pty_io::decode_pty_chunk(&mut pending_utf8, &buf[..n])
                        else {
                            continue;
                        };
                        session_output::apply_chunk(
                            session_output::OutputHandles {
                                output: &output,
                                screen: &screen,
                                recording: &recording,
                                activity: &activity,
                                writer: &writer,
                            },
                            chunk.as_str(),
                            &mut pending_terminal_query,
                        );
                    }
                    Err(err) => {
                        *dead_reason.lock() = Some(format!("pty_reader_error:{err}"));
                        break;
                    }
                }
            }
            dead.store(true, Ordering::Release);
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_size_clamps_to_conpty_safe_range() {
        let size = pty_size(0, usize::MAX);

        assert_eq!(size.cols, 1);
        assert_eq!(size.rows, u16::MAX);
        assert_eq!(size.pixel_width, 0);
        assert_eq!(size.pixel_height, 0);
    }

    #[test]
    fn resize_reports_missing_session() {
        let mut state = NextCoreState::default();
        let err = resize(&mut state, 42, 80, 24).expect_err("missing session should fail");

        assert!(err.to_string().contains("next-core session 42 not found"));
    }
}
