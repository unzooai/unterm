use super::{
    CreateSessionRequest, CursorSnapshot, InputEngine, ScreenEngine, ScreenLine,
    ScreenSearchMatch, ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot,
    SessionActivitySnapshot, SessionEngine, SessionSnapshot, ShellSnapshot, SplitSessionRequest,
};
use anyhow::{bail, Result};
use parking_lot::RwLock;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default)]
pub struct NextCoreEngine;

#[derive(Default)]
struct NextCoreState {
    next_session_id: usize,
    sessions: Vec<SessionSnapshot>,
}

fn state() -> &'static RwLock<NextCoreState> {
    static STATE: OnceLock<RwLock<NextCoreState>> = OnceLock::new();
    STATE.get_or_init(|| RwLock::new(NextCoreState::default()))
}

#[cfg(test)]
fn reset_state_for_test() {
    *state().write() = NextCoreState::default();
}

fn not_implemented<T>(operation: &str) -> Result<T> {
    bail!("next-core engine operation is not implemented yet: {operation}")
}

impl NextCoreEngine {
    fn sessions(&self) -> Vec<SessionSnapshot> {
        state().read().sessions.clone()
    }

    fn session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        for session in self.sessions() {
            if session.id == pane_id {
                return Ok(session);
            }
        }

        bail!("next-core session {pane_id} not found")
    }

    fn ensure_session(&self, pane_id: usize) -> Result<()> {
        self.session(pane_id).map(|_| ())
    }

    fn next_session_id(state: &mut NextCoreState) -> usize {
        state.next_session_id = state.next_session_id.max(1);
        let id = state.next_session_id;
        state.next_session_id += 1;
        id
    }

    fn set_active(state: &mut NextCoreState, pane_id: usize) {
        for session in &mut state.sessions {
            session.is_active = session.id == pane_id;
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

    fn shell_snapshot(cwd: Option<String>, has_command: bool) -> ShellSnapshot {
        ShellSnapshot {
            shell_type: if has_command { "pending" } else { "unknown" }.to_string(),
            process_name: if has_command {
                "pending-command".to_string()
            } else {
                String::new()
            },
            cwd,
        }
    }

    fn new_session(
        id: usize,
        title: String,
        cols: usize,
        rows: usize,
        cwd: Option<String>,
        has_command: bool,
    ) -> SessionSnapshot {
        SessionSnapshot {
            id,
            title,
            cols,
            rows,
            scrollback_rows: 0,
            cursor: Self::default_cursor(),
            is_dead: false,
            is_active: true,
            domain_id: 0,
            shell: Self::shell_snapshot(cwd, has_command),
        }
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
        let has_command = request.command.is_some();
        let mut state = state().write();
        let id = Self::next_session_id(&mut state);
        let session = Self::new_session(
            id,
            format!("next-core:{id}"),
            request.cols,
            request.rows,
            request.command_dir,
            has_command,
        );
        Self::set_active(&mut state, id);
        state.sessions.push(session.clone());
        Ok(session)
    }

    fn split_session(&self, request: SplitSessionRequest) -> Result<SessionSnapshot> {
        let mut state = state().write();
        let source = state
            .sessions
            .iter()
            .find(|session| session.id == request.source_pane_id)
            .cloned();
        let Some(source) = source else {
            bail!("next-core session {} not found", request.source_pane_id);
        };

        let id = Self::next_session_id(&mut state);
        let session = Self::new_session(
            id,
            format!("next-core:{id}"),
            source.cols,
            source.rows,
            request.command_dir.or(source.shell.cwd),
            false,
        );
        Self::set_active(&mut state, id);
        state.sessions.push(session.clone());
        Ok(session)
    }

    fn focus_session(&self, pane_id: usize) -> Result<()> {
        let mut state = state().write();
        if !state.sessions.iter().any(|session| session.id == pane_id) {
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
            .find(|session| session.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };

        session.cols = cols;
        session.rows = rows;
        Ok(())
    }

    fn destroy_session(&self, pane_id: usize) -> Result<()> {
        let mut state = state().write();
        let Some(idx) = state
            .sessions
            .iter()
            .position(|session| session.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };

        let was_active = state.sessions[idx].is_active;
        state.sessions.remove(idx);

        if was_active {
            let next_active_id = state.sessions.last().map(|session| session.id);
            if let Some(next_active_id) = next_active_id {
                Self::set_active(&mut state, next_active_id);
            }
        }

        Ok(())
    }
}

impl ScreenEngine for NextCoreEngine {
    fn read_screen(&self, pane_id: usize) -> Result<ScreenSnapshot> {
        self.ensure_session(pane_id)?;
        not_implemented("read_screen")
    }

    fn read_visible_text(&self, pane_id: usize) -> Result<String> {
        self.ensure_session(pane_id)?;
        not_implemented("read_visible_text")
    }

    fn read_lines(&self, pane_id: usize, _start: i64, _count: usize) -> Result<Vec<ScreenLine>> {
        self.ensure_session(pane_id)?;
        not_implemented("read_lines")
    }

    fn read_scrollback(&self, pane_id: usize, _limit: usize) -> Result<Vec<String>> {
        self.ensure_session(pane_id)?;
        not_implemented("read_scrollback")
    }

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        _request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot> {
        self.ensure_session(pane_id)?;
        not_implemented("read_scrollback_text")
    }

    fn search(
        &self,
        pane_id: usize,
        _pattern: &str,
        _max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        self.ensure_session(pane_id)?;
        not_implemented("search")
    }

    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        Ok(self.session(pane_id)?.cursor)
    }
}

impl InputEngine for NextCoreEngine {
    fn write_input(&self, pane_id: usize, _input: &str) -> Result<()> {
        self.ensure_session(pane_id)?;
        not_implemented("write_input")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manages_session_metadata_lifecycle() -> Result<()> {
        reset_state_for_test();
        let engine = NextCoreEngine;

        let first = engine.create_session(CreateSessionRequest {
            cols: 120,
            rows: 30,
            command_dir: Some("D:\\code\\unterm".to_string()),
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

        Ok(())
    }
}
