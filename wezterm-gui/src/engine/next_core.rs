use super::{
    CreateSessionRequest, CursorSnapshot, InputEngine, ScreenEngine, ScreenLine,
    ScreenSearchMatch, ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot,
    SessionActivitySnapshot, SessionEngine, SessionSnapshot, ShellSnapshot, SplitSessionRequest,
};
use anyhow::{bail, Result};

#[derive(Clone, Copy, Debug, Default)]
pub struct NextCoreEngine;

fn not_implemented<T>(operation: &str) -> Result<T> {
    bail!("next-core engine operation is not implemented yet: {operation}")
}

impl SessionEngine for NextCoreEngine {
    fn list_sessions(&self) -> Result<Vec<SessionSnapshot>> {
        not_implemented("list_sessions")
    }

    fn get_session(&self, _pane_id: usize) -> Result<SessionSnapshot> {
        not_implemented("get_session")
    }

    fn create_session(&self, _request: CreateSessionRequest) -> Result<SessionSnapshot> {
        not_implemented("create_session")
    }

    fn split_session(&self, _request: SplitSessionRequest) -> Result<SessionSnapshot> {
        not_implemented("split_session")
    }

    fn focus_session(&self, _pane_id: usize) -> Result<()> {
        not_implemented("focus_session")
    }

    fn shell(&self, _pane_id: usize) -> Result<ShellSnapshot> {
        not_implemented("shell")
    }

    fn activity(&self, _pane_id: usize) -> Result<SessionActivitySnapshot> {
        not_implemented("activity")
    }

    fn resize_session(&self, _pane_id: usize, _cols: usize, _rows: usize) -> Result<()> {
        not_implemented("resize_session")
    }

    fn destroy_session(&self, _pane_id: usize) -> Result<()> {
        not_implemented("destroy_session")
    }
}

impl ScreenEngine for NextCoreEngine {
    fn read_screen(&self, _pane_id: usize) -> Result<ScreenSnapshot> {
        not_implemented("read_screen")
    }

    fn read_visible_text(&self, _pane_id: usize) -> Result<String> {
        not_implemented("read_visible_text")
    }

    fn read_lines(&self, _pane_id: usize, _start: i64, _count: usize) -> Result<Vec<ScreenLine>> {
        not_implemented("read_lines")
    }

    fn read_scrollback(&self, _pane_id: usize, _limit: usize) -> Result<Vec<String>> {
        not_implemented("read_scrollback")
    }

    fn read_scrollback_text(
        &self,
        _pane_id: usize,
        _request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot> {
        not_implemented("read_scrollback_text")
    }

    fn search(
        &self,
        _pane_id: usize,
        _pattern: &str,
        _max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        not_implemented("search")
    }

    fn cursor(&self, _pane_id: usize) -> Result<CursorSnapshot> {
        not_implemented("cursor")
    }
}

impl InputEngine for NextCoreEngine {
    fn write_input(&self, _pane_id: usize, _input: &str) -> Result<()> {
        not_implemented("write_input")
    }
}
