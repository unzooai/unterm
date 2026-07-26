//! Engine-neutral terminal access layer.
//!
//! Unterm's product surface should not depend directly on WezTerm internals.
//! This crate is the migration boundary for next-core and any future adapter.

pub mod next_core;

use anyhow::Result;
use portable_pty::CommandBuilder;
use serde::Serialize;

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
    pub launch_env_keys: Vec<String>,
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
    pub dead_reason: Option<String>,
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
pub struct InputActivitySnapshot {
    pub total_writes: u64,
    pub total_bytes: u64,
    pub last_bytes: usize,
    pub last_duration_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct OutputActivitySnapshot {
    pub total_chunks: u64,
    pub total_bytes: u64,
    pub last_bytes: usize,
    pub last_duration_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PasteActivitySnapshot {
    pub total_pastes: u64,
    pub total_text_bytes: u64,
    pub total_chunks: u64,
    pub last_text_bytes: usize,
    pub last_wire_bytes: usize,
    pub last_chunk_count: usize,
    pub last_bracketed: bool,
    pub last_duration_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScreenActivitySnapshot {
    pub total_reads: u64,
    pub total_viewport_scrolls: u64,
    pub last_read_duration_ms: u64,
    pub last_scroll_duration_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProcessTreeSnapshot {
    pub root_pid: Option<u32>,
    pub root_process: String,
    pub foreground_pid: Option<u32>,
    pub foreground_process: String,
    pub foreground_argv: Vec<String>,
    pub child_count: usize,
    pub detected_agent: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionActivitySnapshot {
    pub idle: bool,
    pub foreground_process: String,
    pub process: Option<ProcessTreeSnapshot>,
    pub input: Option<InputActivitySnapshot>,
    pub output: Option<OutputActivitySnapshot>,
    pub paste: Option<PasteActivitySnapshot>,
    pub screen: Option<ScreenActivitySnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecordingStartResult {
    pub session_id: String,
    pub log_path: String,
    pub md_path: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecordingStopResult {
    pub session_id: String,
    pub ended_at: String,
    pub block_count: u64,
    pub exit_reason: String,
    pub md_path: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecordingExportResult {
    pub session_id: String,
    pub path: String,
    pub bytes: usize,
    pub block_count: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecordingStatusSnapshot {
    pub enabled: bool,
    pub session_id: Option<String>,
    pub started_at: Option<String>,
    pub block_count: Option<u64>,
    pub bytes: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EngineIoHealthSnapshot {
    pub input_writes: u64,
    pub input_bytes: u64,
    pub output_chunks: u64,
    pub output_bytes: u64,
    pub paste_count: u64,
    pub paste_text_bytes: u64,
    pub screen_reads: u64,
    pub viewport_scrolls: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct EngineLifecycleHealthSnapshot {
    pub live_sessions: u64,
    pub dead_sessions: u64,
    pub total_created: u64,
    pub total_destroyed: u64,
    pub total_marked_dead: u64,
    pub last_dead_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EngineHealthSnapshot {
    pub engine: String,
    pub ready: bool,
    pub status: String,
    pub detail: String,
    pub pane_count: Option<usize>,
    pub io: Option<EngineIoHealthSnapshot>,
    pub lifecycle: Option<EngineLifecycleHealthSnapshot>,
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
    pub env: Vec<(String, String)>,
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
    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>>;
    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot>;
}

pub trait InputEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()>;

    #[allow(dead_code)]
    fn paste_input(&self, pane_id: usize, text: &str) -> Result<()> {
        self.write_input(pane_id, text)
    }
}

pub trait RecordingEngine {
    fn start_recording(&self, pane_id: usize) -> Result<RecordingStartResult>;
    fn stop_recording(&self, pane_id: usize) -> Result<RecordingStopResult>;
    fn recording_status(&self, pane_id: usize) -> Result<RecordingStatusSnapshot>;
    fn attach_recording_trace(&self, pane_id: usize, trace_id: String) -> Result<Vec<String>>;
    fn export_markdown(
        &self,
        pane_id: usize,
        target_path: Option<String>,
    ) -> Result<RecordingExportResult>;
}

pub trait HealthEngine {
    fn health(&self) -> Result<EngineHealthSnapshot>;
}

#[allow(dead_code)]
pub trait TerminalEngine:
    SessionEngine + ScreenEngine + InputEngine + RecordingEngine + HealthEngine
{
}

impl<T> TerminalEngine for T where
    T: SessionEngine + ScreenEngine + InputEngine + RecordingEngine + HealthEngine
{
}
