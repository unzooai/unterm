//! Engine-neutral terminal access layer.
//!
//! Unterm's product surface (MCP, CLI bridge, Agent Cockpit, recording,
//! profiles, review) should not depend directly on WezTerm internals. This
//! module is the migration boundary: the current build implements it with the
//! WezTerm mux, and next-core will implement the same shapes from its own pane
//! model.

pub mod wezterm;

use anyhow::Result;
use portable_pty::CommandBuilder;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CursorSnapshot {
    pub x: usize,
    pub y: isize,
    pub visible: bool,
    pub shape: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaneDimensions {
    pub cols: usize,
    pub viewport_rows: usize,
    pub scrollback_rows: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShellSnapshot {
    pub shell_type: String,
    pub process_name: String,
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionSnapshot {
    pub id: usize,
    pub title: String,
    pub cols: usize,
    pub rows: usize,
    pub scrollback_rows: usize,
    pub cursor: CursorSnapshot,
    pub is_dead: bool,
    pub is_active: bool,
    pub domain_id: usize,
    pub shell: ShellSnapshot,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScreenLine {
    pub row: i64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScreenSnapshot {
    pub lines: Vec<String>,
    pub cells: Vec<ScreenLine>,
    pub cursor: CursorSnapshot,
    pub cols: usize,
    pub rows: usize,
    pub scrollback_rows: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionActivitySnapshot {
    pub idle: bool,
    pub foreground_process: String,
}

#[derive(Clone, Copy, Debug)]
pub enum SplitDirection {
    Right,
    Left,
    Down,
    Up,
}

#[derive(Debug)]
pub struct CreateSessionRequest {
    pub cols: usize,
    pub rows: usize,
    pub command_dir: Option<String>,
    pub command: Option<CommandBuilder>,
}

#[derive(Clone, Debug)]
pub struct SplitSessionRequest {
    pub source_pane_id: usize,
    pub direction: SplitDirection,
    pub size_percent: u8,
    pub command_dir: Option<String>,
}

pub trait SessionEngine {
    fn list_sessions(&self) -> Result<Vec<SessionSnapshot>>;
    fn get_session(&self, pane_id: usize) -> Result<SessionSnapshot>;
    fn create_session(&self, request: CreateSessionRequest) -> Result<SessionSnapshot>;
    fn split_session(&self, request: SplitSessionRequest) -> Result<SessionSnapshot>;
    fn focus_session(&self, pane_id: usize) -> Result<()>;
    fn shell(&self, pane_id: usize) -> Result<ShellSnapshot>;
    fn activity(&self, pane_id: usize) -> Result<SessionActivitySnapshot>;
    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> Result<()>;
    fn destroy_session(&self, pane_id: usize) -> Result<()>;
}

pub trait ScreenEngine {
    fn read_screen(&self, pane_id: usize) -> Result<ScreenSnapshot>;
    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>>;
    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot>;
}

pub trait InputEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()>;
}

#[allow(dead_code)]
pub trait TerminalEngine: SessionEngine + ScreenEngine + InputEngine {}

impl<T> TerminalEngine for T where T: SessionEngine + ScreenEngine + InputEngine {}
