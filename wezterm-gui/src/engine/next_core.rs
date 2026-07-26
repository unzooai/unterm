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
    sessions: Vec<SessionSnapshot>,
}

fn state() -> &'static RwLock<NextCoreState> {
    static STATE: OnceLock<RwLock<NextCoreState>> = OnceLock::new();
    STATE.get_or_init(|| RwLock::new(NextCoreState::default()))
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
}

impl SessionEngine for NextCoreEngine {
    fn list_sessions(&self) -> Result<Vec<SessionSnapshot>> {
        Ok(self.sessions())
    }

    fn get_session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        self.session(pane_id)
    }

    fn create_session(&self, _request: CreateSessionRequest) -> Result<SessionSnapshot> {
        not_implemented("create_session")
    }

    fn split_session(&self, _request: SplitSessionRequest) -> Result<SessionSnapshot> {
        not_implemented("split_session")
    }

    fn focus_session(&self, pane_id: usize) -> Result<()> {
        self.ensure_session(pane_id)?;
        not_implemented("focus_session")
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

    fn resize_session(&self, pane_id: usize, _cols: usize, _rows: usize) -> Result<()> {
        self.ensure_session(pane_id)?;
        not_implemented("resize_session")
    }

    fn destroy_session(&self, pane_id: usize) -> Result<()> {
        self.ensure_session(pane_id)?;
        not_implemented("destroy_session")
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
