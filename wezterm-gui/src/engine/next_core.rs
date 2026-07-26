use super::{
    CreateSessionRequest, CursorSnapshot, InputEngine, ScreenEngine, ScreenLine,
    ScreenSearchMatch, ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot,
    SessionActivitySnapshot, SessionEngine, SessionSnapshot, ShellSnapshot, SplitSessionRequest,
};
use anyhow::{bail, Result};
use parking_lot::{Mutex, RwLock};
use portable_pty::{native_pty_system, Child, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, OnceLock};
use std::thread;

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Default)]
pub struct NextCoreEngine;

struct NextCoreSession {
    snapshot: SessionSnapshot,
    master: Mutex<Box<dyn MasterPty + Send>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output: Arc<Mutex<String>>,
}

impl Drop for NextCoreSession {
    fn drop(&mut self) {
        self.child.lock().kill().ok();
    }
}

#[derive(Default)]
struct NextCoreState {
    next_session_id: usize,
    sessions: Vec<NextCoreSession>,
}

fn state() -> &'static RwLock<NextCoreState> {
    static STATE: OnceLock<RwLock<NextCoreState>> = OnceLock::new();
    STATE.get_or_init(|| RwLock::new(NextCoreState::default()))
}

#[cfg(test)]
fn reset_state_for_test() {
    *state().write() = NextCoreState::default();
}

#[cfg(test)]
fn set_output_for_test(pane_id: usize, text: &str) -> Result<()> {
    let output = {
        let state = state().read();
        let Some(session) = state
            .sessions
            .iter()
            .find(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };
        Arc::clone(&session.output)
    };
    *output.lock() = text.to_string();
    Ok(())
}

impl NextCoreEngine {
    fn sessions(&self) -> Vec<SessionSnapshot> {
        state()
            .read()
            .sessions
            .iter()
            .map(|session| session.snapshot.clone())
            .collect()
    }

    fn session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        for session in self.sessions() {
            if session.id == pane_id {
                return Ok(session);
            }
        }

        bail!("next-core session {pane_id} not found")
    }

    fn output(&self, pane_id: usize) -> Result<String> {
        let output = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.output)
        };

        let text = output.lock().clone();
        Ok(text)
    }

    fn next_session_id(state: &mut NextCoreState) -> usize {
        state.next_session_id = state.next_session_id.max(1);
        let id = state.next_session_id;
        state.next_session_id += 1;
        id
    }

    fn set_active(state: &mut NextCoreState, pane_id: usize) {
        for session in &mut state.sessions {
            session.snapshot.is_active = session.snapshot.id == pane_id;
        }
    }

    fn default_cursor() -> CursorSnapshot {
        CursorSnapshot {
            x: 0,
            y: 0,
            visible: true,
            shape: "Default".to_string(),
        }
    }

    fn pty_size(cols: usize, rows: usize) -> PtySize {
        PtySize {
            rows: rows.clamp(1, u16::MAX as usize) as u16,
            cols: cols.clamp(1, u16::MAX as usize) as u16,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    fn command_label(command: &portable_pty::CommandBuilder) -> String {
        if command.is_default_prog() {
            command.get_shell()
        } else {
            command
                .get_argv()
                .first()
                .and_then(|arg| arg.to_str())
                .unwrap_or("command")
                .to_string()
        }
    }

    fn shell_type(label: &str) -> String {
        let label = label.to_lowercase();
        if label.contains("powershell") || label.contains("pwsh") {
            "powershell"
        } else if label.contains("cmd") {
            "cmd"
        } else if label.contains("bash") {
            "bash"
        } else if label.contains("zsh") {
            "zsh"
        } else if label.contains("fish") {
            "fish"
        } else {
            "unknown"
        }
        .to_string()
    }

    fn command_cwd(
        command: &portable_pty::CommandBuilder,
        fallback: Option<String>,
    ) -> Option<String> {
        command
            .get_cwd()
            .and_then(|cwd| cwd.to_str().map(|cwd| cwd.to_string()))
            .or(fallback)
    }

    fn prepare_command(
        command: Option<portable_pty::CommandBuilder>,
        command_dir: Option<String>,
    ) -> (portable_pty::CommandBuilder, Option<String>) {
        let mut command = command.unwrap_or_else(portable_pty::CommandBuilder::new_default_prog);
        if let Some(command_dir) = command_dir {
            if command.get_cwd().is_none() {
                command.cwd(&command_dir);
            }
        }
        let cwd = Self::command_cwd(&command, None);
        (command, cwd)
    }

    fn spawn_session(
        id: usize,
        title: String,
        cols: usize,
        rows: usize,
        command: portable_pty::CommandBuilder,
        cwd: Option<String>,
    ) -> Result<NextCoreSession> {
        let label = Self::command_label(&command);
        let pair = native_pty_system().openpty(Self::pty_size(cols, rows))?;
        let reader = pair.master.try_clone_reader()?;
        let child = pair.slave.spawn_command(command)?;
        let writer = Arc::new(Mutex::new(pair.master.take_writer()?));
        let output = Arc::new(Mutex::new(String::new()));
        Self::spawn_reader_thread(id, Arc::clone(&output), Arc::clone(&writer), reader);
        let shell = ShellSnapshot {
            shell_type: Self::shell_type(&label),
            process_name: label,
            cwd,
        };

        Ok(NextCoreSession {
            snapshot: SessionSnapshot {
                id,
                title,
                cols,
                rows,
                scrollback_rows: 0,
                cursor: Self::default_cursor(),
                is_dead: false,
                is_active: true,
                domain_id: 0,
                shell,
            },
            master: Mutex::new(pair.master),
            child: Mutex::new(child),
            writer,
            output,
        })
    }

    fn spawn_reader_thread(
        pane_id: usize,
        output: Arc<Mutex<String>>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        mut reader: Box<dyn Read + Send>,
    ) {
        thread::Builder::new()
            .name(format!("next-core-pty-reader-{pane_id}"))
            .spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let chunk = String::from_utf8_lossy(&buf[..n]);
                            Self::answer_terminal_queries(&chunk, &writer);
                            let mut output = output.lock();
                            output.push_str(&chunk);
                            if output.len() > MAX_OUTPUT_BYTES {
                                let keep_from = output.len() - MAX_OUTPUT_BYTES;
                                let keep_from = output
                                    .char_indices()
                                    .map(|(idx, _)| idx)
                                    .find(|idx| *idx >= keep_from)
                                    .unwrap_or(0);
                                output.drain(..keep_from);
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
            .ok();
    }

    fn answer_terminal_queries(chunk: &str, writer: &Arc<Mutex<Box<dyn Write + Send>>>) {
        let mut response = Vec::new();
        if chunk.contains("\x1b[6n") {
            response.extend_from_slice(b"\x1b[1;1R");
        }
        if chunk.contains("\x1b[c") {
            response.extend_from_slice(b"\x1b[?1;0c");
        }
        if chunk.contains("\x1b[1t") {
            response.extend_from_slice(b"\x1b[1t");
        }
        if !response.is_empty() {
            let mut writer = writer.lock();
            writer.write_all(&response).ok();
            writer.flush().ok();
        }
    }

    fn output_lines(output: &str) -> Vec<String> {
        output
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    fn tail_lines(lines: &[String], limit: usize) -> Vec<String> {
        let start = lines.len().saturating_sub(limit);
        lines[start..].to_vec()
    }

}

impl SessionEngine for NextCoreEngine {
    fn list_sessions(&self) -> Result<Vec<SessionSnapshot>> {
        Ok(self.sessions())
    }

    fn get_session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        self.session(pane_id)
    }

    fn create_session(&self, request: CreateSessionRequest) -> Result<SessionSnapshot> {
        let (command, cwd) = Self::prepare_command(request.command, request.command_dir);
        let mut state_guard = state().write();
        let id = Self::next_session_id(&mut state_guard);
        drop(state_guard);

        let session = Self::spawn_session(
            id,
            format!("next-core:{id}"),
            request.cols,
            request.rows,
            command,
            cwd,
        )?;

        let snapshot = session.snapshot.clone();
        let mut state_guard = state().write();
        Self::set_active(&mut state_guard, id);
        state_guard.sessions.push(session);
        Ok(snapshot)
    }

    fn split_session(&self, request: SplitSessionRequest) -> Result<SessionSnapshot> {
        let state_guard = state().read();
        let source = state_guard
            .sessions
            .iter()
            .find(|session| session.snapshot.id == request.source_pane_id)
            .map(|session| session.snapshot.clone());
        drop(state_guard);
        let Some(source) = source else {
            bail!("next-core session {} not found", request.source_pane_id);
        };

        let mut command = portable_pty::CommandBuilder::new_default_prog();
        if let Some(cwd) = request.command_dir.or(source.shell.cwd) {
            command.cwd(cwd);
        }
        let cwd = Self::command_cwd(&command, None);

        let mut state_guard = state().write();
        let id = Self::next_session_id(&mut state_guard);
        drop(state_guard);

        let session = Self::spawn_session(
            id,
            format!("next-core:{id}"),
            source.cols,
            source.rows,
            command,
            cwd,
        )?;

        let snapshot = session.snapshot.clone();
        let mut state_guard = state().write();
        Self::set_active(&mut state_guard, id);
        state_guard.sessions.push(session);
        Ok(snapshot)
    }

    fn focus_session(&self, pane_id: usize) -> Result<()> {
        let mut state = state().write();
        if !state
            .sessions
            .iter()
            .any(|session| session.snapshot.id == pane_id)
        {
            bail!("next-core session {pane_id} not found");
        }
        Self::set_active(&mut state, pane_id);
        Ok(())
    }

    fn shell(&self, pane_id: usize) -> Result<ShellSnapshot> {
        Ok(self.session(pane_id)?.shell)
    }

    fn activity(&self, pane_id: usize) -> Result<SessionActivitySnapshot> {
        let shell = self.shell(pane_id)?;
        let foreground_process = shell.process_name;
        Ok(SessionActivitySnapshot {
            idle: foreground_process.is_empty() || foreground_process == "unknown",
            foreground_process,
        })
    }

    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> Result<()> {
        let mut state = state().write();
        let Some(session) = state
            .sessions
            .iter_mut()
            .find(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };

        session.master.lock().resize(Self::pty_size(cols, rows))?;
        session.snapshot.cols = cols;
        session.snapshot.rows = rows;
        Ok(())
    }

    fn destroy_session(&self, pane_id: usize) -> Result<()> {
        let mut state = state().write();
        let Some(idx) = state
            .sessions
            .iter()
            .position(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };

        let was_active = state.sessions[idx].snapshot.is_active;
        let mut session = state.sessions.remove(idx);
        session.snapshot.is_dead = true;
        session.child.lock().kill().ok();

        if was_active {
            let next_active_id = state.sessions.last().map(|session| session.snapshot.id);
            if let Some(next_active_id) = next_active_id {
                Self::set_active(&mut state, next_active_id);
            }
        }

        Ok(())
    }
}

impl ScreenEngine for NextCoreEngine {
    fn read_screen(&self, pane_id: usize) -> Result<ScreenSnapshot> {
        let session = self.session(pane_id)?;
        let output = self.output(pane_id)?;
        let lines = Self::output_lines(&output);
        let visible = Self::tail_lines(&lines, session.rows);
        let first_row = lines.len().saturating_sub(visible.len()) as i64;
        let cells = visible
            .iter()
            .enumerate()
            .map(|(idx, text)| ScreenLine {
                row: first_row + idx as i64,
                text: text.clone(),
            })
            .collect();

        Ok(ScreenSnapshot {
            lines: visible,
            cells,
            cursor: session.cursor,
            cols: session.cols,
            rows: session.rows,
            scrollback_rows: lines.len().saturating_sub(session.rows),
        })
    }

    fn read_visible_text(&self, pane_id: usize) -> Result<String> {
        Ok(self.read_screen(pane_id)?.lines.join("\n"))
    }

    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
        let output = self.output(pane_id)?;
        let lines = Self::output_lines(&output);
        let start = start.max(0) as usize;
        Ok(lines
            .iter()
            .skip(start)
            .take(count)
            .enumerate()
            .map(|(idx, text)| ScreenLine {
                row: (start + idx) as i64,
                text: text.clone(),
            })
            .collect())
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>> {
        let output = self.output(pane_id)?;
        let lines = Self::output_lines(&output)
            .into_iter()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        Ok(Self::tail_lines(&lines, limit))
    }

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot> {
        let session = self.session(pane_id)?;
        let output = self.output(pane_id)?;
        let lines = Self::output_lines(&output);
        let end = request
            .end_line
            .map(|end| end.max(0) as usize)
            .unwrap_or(lines.len())
            .min(lines.len());
        let mut start = request
            .start_line
            .map(|start| start.max(0) as usize)
            .unwrap_or(0)
            .min(end);
        if let Some(tail) = request.tail_lines {
            if tail > 0 {
                start = start.max(end.saturating_sub(tail as usize));
            }
        }

        let selected = lines[start..end].to_vec();
        Ok(ScrollbackTextSnapshot {
            text: if request.escapes {
                String::new()
            } else {
                selected.join("\n")
            },
            lines: if request.escapes {
                Vec::new()
            } else {
                selected
            },
            first_row: start as i64,
            row_count: end.saturating_sub(start) as i64,
            cols: session.cols,
            escapes: request.escapes,
            scrollback_top: 0,
            physical_top: lines.len().saturating_sub(session.rows) as i64,
            viewport_rows: session.rows,
        })
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        let output = self.output(pane_id)?;
        let lines = Self::output_lines(&output);
        let mut matches = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            if let Some(col) = line.find(pattern) {
                matches.push(ScreenSearchMatch {
                    row: row as i64,
                    col,
                    text: line.clone(),
                });
                if matches.len() >= max_results {
                    break;
                }
            }
        }
        Ok(matches)
    }

    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        Ok(self.session(pane_id)?.cursor)
    }
}

impl InputEngine for NextCoreEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()> {
        let mut state = state().write();
        let Some(session) = state
            .sessions
            .iter_mut()
            .find(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };

        let mut writer = session.writer.lock();
        writer.write_all(input.as_bytes())?;
        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::MutexGuard;

    fn test_guard() -> MutexGuard<'static, ()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(())).lock()
    }

    #[test]
    fn manages_session_metadata_lifecycle() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let cwd = std::env::current_dir()
            .ok()
            .map(|path| path.display().to_string());

        let first = engine.create_session(CreateSessionRequest {
            cols: 120,
            rows: 30,
            command_dir: cwd,
            command: None,
        })?;
        assert_eq!(first.id, 1);
        assert!(first.is_active);

        let second = engine.split_session(SplitSessionRequest {
            source_pane_id: first.id,
            direction: super::super::SplitDirection::Right,
            size_percent: 50,
            command_dir: None,
        })?;
        assert_eq!(second.id, 2);
        assert!(engine.get_session(second.id)?.is_active);
        assert!(!engine.get_session(first.id)?.is_active);

        engine.focus_session(first.id)?;
        assert!(engine.get_session(first.id)?.is_active);
        assert!(!engine.get_session(second.id)?.is_active);

        engine.resize_session(first.id, 100, 25)?;
        let resized = engine.get_session(first.id)?;
        assert_eq!(resized.cols, 100);
        assert_eq!(resized.rows, 25);

        engine.destroy_session(first.id)?;
        assert!(engine.get_session(first.id).is_err());
        assert!(engine.get_session(second.id)?.is_active);
        assert_eq!(engine.list_sessions()?.len(), 1);
        engine.destroy_session(second.id)?;

        Ok(())
    }

    #[test]
    fn exposes_buffered_output_for_screen_reads() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
        })?;
        set_output_for_test(session.id, "line-one\r\nnext-core-output\r\nline-three\r\n")?;

        let text = engine.read_visible_text(session.id)?;
        assert!(text.contains("next-core-output"));
        assert!(!engine.search(session.id, "next-core-output", 1)?.is_empty());

        let scrollback = engine.read_scrollback_text(
            session.id,
            ScrollbackTextRequest {
                start_line: None,
                end_line: None,
                tail_lines: Some(10),
                escapes: false,
            },
        )?;
        assert!(scrollback.text.contains("next-core-output"));

        engine.destroy_session(session.id)?;
        Ok(())
    }
}
