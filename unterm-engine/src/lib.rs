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
    pub launch_context: LaunchContextSnapshot,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LaunchContextSnapshot {
    pub profile: Option<String>,
    pub proxy_env_keys: Vec<String>,
    pub env_key_count: usize,
    pub policy: LaunchPolicySnapshot,
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
pub enum LaunchEnvSource {
    Proxy,
    Profile,
    Overlay,
    Explicit,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct LaunchEnvBinding {
    pub key: String,
    pub source: LaunchEnvSource,
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LaunchPolicyDecision {
    #[default]
    NotRequested,
    Applied,
    Deferred,
    Unsupported,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LaunchPolicyDecisionSnapshot {
    pub decision: LaunchPolicyDecision,
    pub supported: bool,
    pub reason: String,
}

impl Default for LaunchPolicyDecisionSnapshot {
    fn default() -> Self {
        Self {
            decision: LaunchPolicyDecision::NotRequested,
            supported: false,
            reason: "not requested".to_string(),
        }
    }
}

impl LaunchPolicyDecisionSnapshot {
    pub fn new(decision: LaunchPolicyDecision, supported: bool, reason: impl Into<String>) -> Self {
        Self {
            decision,
            supported,
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct LaunchPolicySnapshot {
    pub profile: Option<String>,
    pub env: Vec<LaunchEnvBinding>,
    pub proxy_env_keys: Vec<String>,
    pub domain: LaunchPolicyDecisionSnapshot,
    pub privilege: LaunchPolicyDecisionSnapshot,
    pub proxy_rotation: LaunchPolicyDecisionSnapshot,
    pub restart: LaunchPolicyDecisionSnapshot,
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
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum StyledBlink {
    Slow,
    Rapid,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum StyledUnderline {
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum StyledVerticalAlign {
    SuperScript,
    SubScript,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct CellStyle {
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub underline: bool,
    pub underline_style: Option<StyledUnderline>,
    pub underline_color: Option<StyledColor>,
    pub strikethrough: bool,
    pub hidden: bool,
    pub overline: bool,
    pub blink: Option<StyledBlink>,
    pub vertical_align: Option<StyledVerticalAlign>,
    pub inverse: bool,
    pub fg: Option<StyledColor>,
    pub bg: Option<StyledColor>,
    pub hyperlink: Option<String>,
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

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
pub struct RenderFrameSnapshot {
    pub lines: Vec<StyledScreenLine>,
    pub cursor: CursorSnapshot,
    pub cols: usize,
    pub rows: usize,
    pub scrollback_rows: usize,
    pub revision: u64,
    pub dirty_rows: Option<DirtyRows>,
    pub full: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderGlyphRun {
    pub row: usize,
    pub col: usize,
    pub cells: usize,
    pub text: String,
    pub style: CellStyle,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderCellRun {
    pub row: usize,
    pub col: usize,
    pub cells: usize,
    pub style: CellStyle,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderCursorDraw {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
    pub shape: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderDrawPlan {
    pub revision: u64,
    pub cols: usize,
    pub rows: usize,
    pub scrollback_rows: usize,
    pub dirty_rows: Option<DirtyRows>,
    pub full: bool,
    pub glyph_runs: Vec<RenderGlyphRun>,
    pub cell_runs: Vec<RenderCellRun>,
    pub cursor: Option<RenderCursorDraw>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct RenderCellMetrics {
    pub cell_width_px: usize,
    pub cell_height_px: usize,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct RenderRect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderGlyphRunGeometry {
    pub row: usize,
    pub col: usize,
    pub text: String,
    pub rect: RenderRect,
    pub style: CellStyle,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderCellRunGeometry {
    pub row: usize,
    pub col: usize,
    pub rect: RenderRect,
    pub style: CellStyle,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderCursorGeometry {
    pub row: usize,
    pub col: usize,
    pub rect: RenderRect,
    pub visible: bool,
    pub shape: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderGeometryPlan {
    pub revision: u64,
    pub cols: usize,
    pub rows: usize,
    pub scrollback_rows: usize,
    pub dirty_rows: Option<DirtyRows>,
    pub full: bool,
    pub viewport: RenderRect,
    pub glyph_runs: Vec<RenderGlyphRunGeometry>,
    pub cell_runs: Vec<RenderCellRunGeometry>,
    pub cursor: Option<RenderCursorGeometry>,
}

impl RenderDrawPlan {
    #[allow(dead_code)]
    pub fn to_geometry_plan(&self, metrics: RenderCellMetrics) -> RenderGeometryPlan {
        let glyph_runs = self
            .glyph_runs
            .iter()
            .map(|run| RenderGlyphRunGeometry {
                row: run.row,
                col: run.col,
                text: run.text.clone(),
                rect: grid_rect(run.row, run.col, run.cells, 1, metrics),
                style: run.style.clone(),
            })
            .collect();
        let cell_runs = self
            .cell_runs
            .iter()
            .map(|run| RenderCellRunGeometry {
                row: run.row,
                col: run.col,
                rect: grid_rect(run.row, run.col, run.cells, 1, metrics),
                style: run.style.clone(),
            })
            .collect();
        let cursor = self.cursor.as_ref().map(|cursor| RenderCursorGeometry {
            row: cursor.row,
            col: cursor.col,
            rect: grid_rect(cursor.row, cursor.col, 1, 1, metrics),
            visible: cursor.visible,
            shape: cursor.shape.clone(),
        });

        RenderGeometryPlan {
            revision: self.revision,
            cols: self.cols,
            rows: self.rows,
            scrollback_rows: self.scrollback_rows,
            dirty_rows: self.dirty_rows,
            full: self.full,
            viewport: grid_rect(0, 0, self.cols, self.rows, metrics),
            glyph_runs,
            cell_runs,
            cursor,
        }
    }
}

fn grid_rect(
    row: usize,
    col: usize,
    cells: usize,
    rows: usize,
    metrics: RenderCellMetrics,
) -> RenderRect {
    RenderRect {
        x: col.saturating_mul(metrics.cell_width_px),
        y: row.saturating_mul(metrics.cell_height_px),
        width: cells.saturating_mul(metrics.cell_width_px),
        height: rows.saturating_mul(metrics.cell_height_px),
    }
}

impl RenderFrameSnapshot {
    #[allow(dead_code)]
    pub fn to_draw_plan(&self) -> RenderDrawPlan {
        let mut glyph_runs = Vec::new();
        let mut cell_runs = Vec::new();

        for (line_idx, line) in self.lines.iter().enumerate().take(self.rows) {
            let row_idx = self.draw_plan_row_for_line(line_idx);
            let mut active_glyph: Option<RenderGlyphRun> = None;
            let mut active_cell: Option<RenderCellRun> = None;
            for (col_idx, cell) in line.cells.iter().enumerate().take(self.cols) {
                push_cell_run(
                    &mut cell_runs,
                    &mut active_cell,
                    row_idx,
                    col_idx,
                    &cell.style,
                );

                if cell.width == 0 || cell.style.hidden || cell.ch == ' ' {
                    flush_glyph_run(&mut glyph_runs, &mut active_glyph);
                    continue;
                }

                match active_glyph.as_mut() {
                    Some(run) if run.style == cell.style && run.col + run.cells == col_idx => {
                        run.text.push(cell.ch);
                        run.cells += cell.width.max(1);
                    }
                    _ => {
                        flush_glyph_run(&mut glyph_runs, &mut active_glyph);
                        active_glyph = Some(RenderGlyphRun {
                            row: row_idx,
                            col: col_idx,
                            cells: cell.width.max(1),
                            text: cell.ch.to_string(),
                            style: cell.style.clone(),
                        });
                    }
                }
            }
            flush_glyph_run(&mut glyph_runs, &mut active_glyph);
            flush_cell_run(&mut cell_runs, &mut active_cell);
        }

        RenderDrawPlan {
            revision: self.revision,
            cols: self.cols,
            rows: self.rows,
            scrollback_rows: self.scrollback_rows,
            dirty_rows: self.dirty_rows,
            full: self.full,
            glyph_runs,
            cell_runs,
            cursor: self.cursor_draw(),
        }
    }

    fn cursor_draw(&self) -> Option<RenderCursorDraw> {
        if self.cursor.y < 0 {
            return None;
        }
        let row = self.cursor.y as usize;
        (row < self.rows && self.cursor.x < self.cols).then(|| RenderCursorDraw {
            row,
            col: self.cursor.x,
            visible: self.cursor.visible,
            shape: self.cursor.shape.clone(),
        })
    }

    fn draw_plan_row_for_line(&self, line_idx: usize) -> usize {
        if self.full {
            return line_idx;
        }
        self.dirty_rows
            .map(|rows| rows.start.saturating_add(line_idx))
            .unwrap_or(line_idx)
    }
}

fn flush_glyph_run(runs: &mut Vec<RenderGlyphRun>, active: &mut Option<RenderGlyphRun>) {
    if let Some(run) = active.take() {
        runs.push(run);
    }
}

fn flush_cell_run(runs: &mut Vec<RenderCellRun>, active: &mut Option<RenderCellRun>) {
    if let Some(run) = active.take() {
        runs.push(run);
    }
}

fn push_cell_run(
    runs: &mut Vec<RenderCellRun>,
    active: &mut Option<RenderCellRun>,
    row: usize,
    col: usize,
    style: &CellStyle,
) {
    match active.as_mut() {
        Some(run) if run.style == *style && run.col + run.cells == col => {
            run.cells += 1;
        }
        _ => {
            flush_cell_run(runs, active);
            *active = Some(RenderCellRun {
                row,
                col,
                cells: 1,
                style: style.clone(),
            });
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
pub struct StyledScrollbackSnapshot {
    pub lines: Vec<StyledScreenLine>,
    pub first_row: i64,
    pub row_count: i64,
    pub cols: usize,
    pub scrollback_top: i64,
    pub physical_top: i64,
    pub viewport_rows: usize,
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
    pub root_cwd: Option<String>,
    pub foreground_pid: Option<u32>,
    pub foreground_process: String,
    pub foreground_cwd: Option<String>,
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
    pub launch_policy: LaunchPolicySnapshot,
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
    #[allow(dead_code)]
    fn read_render_frame(
        &self,
        pane_id: usize,
        since_revision: Option<u64>,
    ) -> Result<RenderFrameSnapshot> {
        let screen = self.read_styled_screen(pane_id)?;
        if since_revision == Some(screen.revision) {
            return Ok(RenderFrameSnapshot {
                lines: Vec::new(),
                cursor: screen.cursor,
                cols: screen.cols,
                rows: screen.rows,
                scrollback_rows: screen.scrollback_rows,
                revision: screen.revision,
                dirty_rows: None,
                full: false,
            });
        }

        let dirty_rows = if screen.rows == 0 {
            None
        } else {
            Some(DirtyRows {
                start: 0,
                end: screen.rows - 1,
            })
        };
        Ok(RenderFrameSnapshot {
            lines: screen.lines,
            cursor: screen.cursor,
            cols: screen.cols,
            rows: screen.rows,
            scrollback_rows: screen.scrollback_rows,
            revision: screen.revision,
            dirty_rows,
            full: true,
        })
    }
    #[allow(dead_code)]
    fn read_render_draw_plan(
        &self,
        pane_id: usize,
        since_revision: Option<u64>,
    ) -> Result<RenderDrawPlan> {
        Ok(self
            .read_render_frame(pane_id, since_revision)?
            .to_draw_plan())
    }
    fn read_visible_text(&self, pane_id: usize) -> Result<String>;
    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>>;
    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>>;
    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot>;
    #[allow(dead_code)]
    fn read_styled_scrollback(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<StyledScrollbackSnapshot> {
        let text = self.read_scrollback_text(pane_id, request)?;
        let lines = text
            .lines
            .iter()
            .enumerate()
            .map(|(idx, line)| StyledScreenLine {
                row: text.first_row + idx as i64,
                cells: line
                    .chars()
                    .map(|ch| {
                        let mut buf = [0u8; 4];
                        StyledCell {
                            ch,
                            style: CellStyle::default(),
                            width: termwiz::cell::unicode_column_width(
                                ch.encode_utf8(&mut buf),
                                None,
                            ),
                        }
                    })
                    .collect(),
            })
            .collect();
        Ok(StyledScrollbackSnapshot {
            lines,
            first_row: text.first_row,
            row_count: text.row_count,
            cols: text.cols,
            scrollback_top: text.scrollback_top,
            physical_top: text.physical_top,
            viewport_rows: text.viewport_rows,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeScreenEngine;

    impl ScreenEngine for FakeScreenEngine {
        fn read_screen(&self, _pane_id: usize) -> Result<ScreenSnapshot> {
            unimplemented!("not needed by draw-plan fallback test")
        }

        fn read_styled_screen(&self, _pane_id: usize) -> Result<StyledScreenSnapshot> {
            Ok(StyledScreenSnapshot {
                lines: vec![StyledScreenLine {
                    row: 0,
                    cells: vec![cell('x', CellStyle::default(), 1)],
                }],
                cursor: CursorSnapshot {
                    x: 0,
                    y: 0,
                    visible: true,
                    shape: "block".to_string(),
                },
                cols: 1,
                rows: 1,
                scrollback_rows: 0,
                revision: 11,
                dirty_rows: Some(DirtyRows { start: 0, end: 0 }),
            })
        }

        fn read_visible_text(&self, _pane_id: usize) -> Result<String> {
            Ok("x".to_string())
        }

        fn read_lines(
            &self,
            _pane_id: usize,
            _start: i64,
            _count: usize,
        ) -> Result<Vec<ScreenLine>> {
            Ok(Vec::new())
        }

        fn read_scrollback(&self, _pane_id: usize, _limit: usize) -> Result<Vec<String>> {
            Ok(Vec::new())
        }

        fn read_scrollback_text(
            &self,
            _pane_id: usize,
            _request: ScrollbackTextRequest,
        ) -> Result<ScrollbackTextSnapshot> {
            unimplemented!("not needed by draw-plan fallback test")
        }

        fn search(
            &self,
            _pane_id: usize,
            _pattern: &str,
            _max_results: usize,
        ) -> Result<Vec<ScreenSearchMatch>> {
            Ok(Vec::new())
        }

        fn cursor(&self, _pane_id: usize) -> Result<CursorSnapshot> {
            Ok(CursorSnapshot {
                x: 0,
                y: 0,
                visible: true,
                shape: "block".to_string(),
            })
        }
    }

    fn cell(ch: char, style: CellStyle, width: usize) -> StyledCell {
        StyledCell { ch, style, width }
    }

    fn frame(cells: Vec<StyledCell>) -> RenderFrameSnapshot {
        RenderFrameSnapshot {
            lines: vec![StyledScreenLine { row: 0, cells }],
            cursor: CursorSnapshot {
                x: 2,
                y: 0,
                visible: true,
                shape: "block".to_string(),
            },
            cols: 6,
            rows: 1,
            scrollback_rows: 0,
            revision: 7,
            dirty_rows: Some(DirtyRows { start: 0, end: 0 }),
            full: false,
        }
    }

    #[test]
    fn render_draw_plan_merges_glyph_and_cell_runs() {
        let red = CellStyle {
            fg: Some(StyledColor::Palette(1)),
            ..CellStyle::default()
        };
        let blue = CellStyle {
            fg: Some(StyledColor::Palette(4)),
            ..CellStyle::default()
        };
        let plan = frame(vec![
            cell('a', red.clone(), 1),
            cell('b', red.clone(), 1),
            cell(' ', red.clone(), 1),
            cell('c', blue.clone(), 1),
            cell('d', blue.clone(), 1),
            cell(' ', CellStyle::default(), 1),
        ])
        .to_draw_plan();

        assert_eq!(plan.revision, 7);
        assert_eq!(plan.dirty_rows, Some(DirtyRows { start: 0, end: 0 }));
        assert_eq!(plan.glyph_runs.len(), 2);
        assert_eq!(plan.glyph_runs[0].text, "ab");
        assert_eq!(plan.glyph_runs[0].col, 0);
        assert_eq!(plan.glyph_runs[0].cells, 2);
        assert_eq!(plan.glyph_runs[1].text, "cd");
        assert_eq!(plan.glyph_runs[1].col, 3);
        assert_eq!(plan.glyph_runs[1].style, blue);
        assert_eq!(plan.cell_runs.len(), 3);
        assert_eq!(plan.cell_runs[0].cells, 3);
        assert_eq!(plan.cell_runs[1].col, 3);
        assert_eq!(plan.cell_runs[1].cells, 2);
        assert_eq!(
            plan.cursor,
            Some(RenderCursorDraw {
                row: 0,
                col: 2,
                visible: true,
                shape: "block".to_string()
            })
        );
    }

    #[test]
    fn screen_engine_default_reads_render_draw_plan() {
        let engine = FakeScreenEngine;
        let plan = engine.read_render_draw_plan(1, None).unwrap();
        assert!(plan.full);
        assert_eq!(plan.revision, 11);
        assert_eq!(plan.glyph_runs.len(), 1);
        assert_eq!(plan.glyph_runs[0].text, "x");

        let unchanged = engine.read_render_draw_plan(1, Some(11)).unwrap();
        assert!(!unchanged.full);
        assert!(unchanged.glyph_runs.is_empty());
        assert_eq!(
            unchanged.cursor,
            Some(RenderCursorDraw {
                row: 0,
                col: 0,
                visible: true,
                shape: "block".to_string()
            })
        );
    }

    #[test]
    fn render_geometry_plan_maps_runs_to_pixel_rects() {
        let plan = frame(vec![
            cell('a', CellStyle::default(), 1),
            cell('b', CellStyle::default(), 1),
            cell(' ', CellStyle::default(), 1),
        ])
        .to_draw_plan()
        .to_geometry_plan(RenderCellMetrics {
            cell_width_px: 9,
            cell_height_px: 17,
        });

        assert_eq!(
            plan.viewport,
            RenderRect {
                x: 0,
                y: 0,
                width: 54,
                height: 17,
            }
        );
        assert_eq!(plan.glyph_runs.len(), 1);
        assert_eq!(
            plan.glyph_runs[0].rect,
            RenderRect {
                x: 0,
                y: 0,
                width: 18,
                height: 17,
            }
        );
        assert_eq!(plan.cell_runs.len(), 1);
        assert_eq!(
            plan.cell_runs[0].rect,
            RenderRect {
                x: 0,
                y: 0,
                width: 27,
                height: 17,
            }
        );
        assert_eq!(
            plan.cursor.unwrap().rect,
            RenderRect {
                x: 18,
                y: 0,
                width: 9,
                height: 17,
            }
        );
    }

    #[test]
    fn render_geometry_plan_preserves_dirty_row_pixels() {
        let dirty = RenderFrameSnapshot {
            lines: vec![StyledScreenLine {
                row: 42,
                cells: vec![cell('z', CellStyle::default(), 1)],
            }],
            cursor: CursorSnapshot {
                x: 1,
                y: 5,
                visible: true,
                shape: "bar".to_string(),
            },
            cols: 4,
            rows: 8,
            scrollback_rows: 12,
            revision: 9,
            dirty_rows: Some(DirtyRows { start: 5, end: 5 }),
            full: false,
        };

        let plan = dirty.to_draw_plan().to_geometry_plan(RenderCellMetrics {
            cell_width_px: 8,
            cell_height_px: 16,
        });
        assert_eq!(plan.glyph_runs[0].row, 5);
        assert_eq!(
            plan.glyph_runs[0].rect,
            RenderRect {
                x: 0,
                y: 80,
                width: 8,
                height: 16,
            }
        );
        assert_eq!(
            plan.cursor.unwrap().rect,
            RenderRect {
                x: 8,
                y: 80,
                width: 8,
                height: 16,
            }
        );
    }

    #[test]
    fn render_draw_plan_skips_hidden_and_wide_continuation_glyphs() {
        let hidden = CellStyle {
            hidden: true,
            ..CellStyle::default()
        };
        let plan = frame(vec![
            cell('你', CellStyle::default(), 2),
            cell(' ', CellStyle::default(), 0),
            cell('x', hidden, 1),
            cell('y', CellStyle::default(), 1),
        ])
        .to_draw_plan();

        assert_eq!(plan.glyph_runs.len(), 2);
        assert_eq!(plan.glyph_runs[0].text, "你");
        assert_eq!(plan.glyph_runs[0].cells, 2);
        assert_eq!(plan.glyph_runs[1].text, "y");
        assert_eq!(plan.glyph_runs[1].col, 3);
    }

    #[test]
    fn render_draw_plan_preserves_dirty_frame_viewport_rows() {
        let dirty = RenderFrameSnapshot {
            lines: vec![
                StyledScreenLine {
                    row: 10,
                    cells: vec![cell('a', CellStyle::default(), 1)],
                },
                StyledScreenLine {
                    row: 11,
                    cells: vec![cell('b', CellStyle::default(), 1)],
                },
            ],
            cursor: CursorSnapshot {
                x: 0,
                y: 4,
                visible: true,
                shape: "block".to_string(),
            },
            cols: 4,
            rows: 8,
            scrollback_rows: 20,
            revision: 8,
            dirty_rows: Some(DirtyRows { start: 3, end: 4 }),
            full: false,
        };

        let plan = dirty.to_draw_plan();
        assert_eq!(plan.glyph_runs.len(), 2);
        assert_eq!(plan.glyph_runs[0].row, 3);
        assert_eq!(plan.glyph_runs[1].row, 4);
        assert_eq!(plan.cell_runs.len(), 2);
        assert_eq!(plan.cell_runs[0].row, 3);
        assert_eq!(plan.cell_runs[1].row, 4);
        assert_eq!(
            plan.cursor,
            Some(RenderCursorDraw {
                row: 4,
                col: 0,
                visible: true,
                shape: "block".to_string()
            })
        );
    }

    #[test]
    fn render_draw_plan_drops_out_of_bounds_cursor() {
        let mut frame = frame(vec![cell('x', CellStyle::default(), 1)]);
        frame.cursor.y = -1;
        assert_eq!(frame.to_draw_plan().cursor, None);

        frame.cursor.y = 0;
        frame.cursor.x = 99;
        assert_eq!(frame.to_draw_plan().cursor, None);
    }
}
