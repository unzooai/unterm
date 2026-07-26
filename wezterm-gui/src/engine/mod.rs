//! Engine-neutral terminal access layer.
//!
//! Unterm's product surface (MCP, CLI bridge, Agent Cockpit, recording,
//! profiles, review) should not depend directly on WezTerm internals. This
//! module is the migration boundary: the current build implements it with the
//! WezTerm mux, and next-core will implement the same shapes from its own pane
//! model.

pub mod next_core;
pub mod wezterm;

use anyhow::Result;
use portable_pty::CommandBuilder;
use serde::Serialize;

pub type CurrentTerminalEngine = wezterm::WezTermEngine;

pub fn current() -> CurrentTerminalEngine {
    wezterm::WezTermEngine
}

#[allow(dead_code)]
pub fn next_core() -> next_core::NextCoreEngine {
    next_core::NextCoreEngine
}

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

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum StyledColor {
    Palette(u8),
    Rgb(u8, u8, u8),
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
pub struct CellStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub fg: Option<StyledColor>,
    pub bg: Option<StyledColor>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StyledCell {
    pub ch: char,
    pub style: CellStyle,
    pub width: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StyledScreenLine {
    pub row: i64,
    pub cells: Vec<StyledCell>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct DirtyRows {
    pub start: usize,
    pub end: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
pub struct StyledScreenSnapshot {
    pub lines: Vec<StyledScreenLine>,
    pub cursor: CursorSnapshot,
    pub cols: usize,
    pub rows: usize,
    pub scrollback_rows: usize,
    pub revision: u64,
    pub dirty_rows: Option<DirtyRows>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScreenSearchMatch {
    pub row: i64,
    pub col: usize,
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
    pub revision: u64,
    pub dirty_rows: Option<DirtyRows>,
}

#[derive(Clone, Debug)]
pub struct ScrollbackTextRequest {
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub tail_lines: Option<i64>,
    pub escapes: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScrollbackTextSnapshot {
    pub text: String,
    pub lines: Vec<String>,
    pub first_row: i64,
    pub row_count: i64,
    pub cols: usize,
    pub escapes: bool,
    pub scrollback_top: i64,
    pub physical_top: i64,
    pub viewport_rows: usize,
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
    #[allow(dead_code)]
    fn read_styled_screen(&self, pane_id: usize) -> Result<StyledScreenSnapshot>;
    fn read_visible_text(&self, pane_id: usize) -> Result<String>;
    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>>;
    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>>;
    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot>;
    fn search(&self, pane_id: usize, pattern: &str, max_results: usize)
        -> Result<Vec<ScreenSearchMatch>>;
    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot>;
}

pub trait InputEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()>;
}

#[allow(dead_code)]
pub trait TerminalEngine: SessionEngine + ScreenEngine + InputEngine {}

impl<T> TerminalEngine for T where T: SessionEngine + ScreenEngine + InputEngine {}
