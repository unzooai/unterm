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

/// Terminal modes a GUI pane has to expose without reaching into the engine's
/// internals: whether the application grabbed the mouse, and whether the
/// alternate screen is up.
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
pub struct PaneModesSnapshot {
    pub mouse_grabbed: bool,
    pub alt_screen_active: bool,
    pub bracketed_paste: bool,
    pub application_cursor_keys: bool,
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
    /// True when this row soft-wrapped into the next one.
    ///
    /// A consumer that reassembles logical lines needs this: without it a
    /// wrapped command reads back as several lines and copying it inserts a
    /// newline the user never typed.
    pub wrapped: bool,
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
    pub cells: usize,
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

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderBackgroundQuad {
    pub rect: RenderRect,
    pub style: CellStyle,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderTextRun {
    pub row: usize,
    pub col: usize,
    pub cells: usize,
    pub text: String,
    pub rect: RenderRect,
    pub style: CellStyle,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderCursorQuad {
    pub rect: RenderRect,
    pub visible: bool,
    pub shape: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderSubmissionPlan {
    pub revision: u64,
    pub cols: usize,
    pub rows: usize,
    pub scrollback_rows: usize,
    pub viewport: RenderRect,
    pub full: bool,
    pub damage_rects: Vec<RenderRect>,
    pub background_quads: Vec<RenderBackgroundQuad>,
    pub text_runs: Vec<RenderTextRun>,
    pub cursor: Option<RenderCursorQuad>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct RenderConsumerState {
    submitted_revision: Option<u64>,
    viewport: Option<RenderRect>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RenderCommitPlan {
    pub submit: bool,
    pub previous_revision: Option<u64>,
    pub revision: u64,
    pub skipped_revisions: u64,
    pub requires_full_repaint: bool,
    pub submission: Option<RenderSubmissionPlan>,
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
                cells: run.cells,
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

impl RenderConsumerState {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn submitted_revision(&self) -> Option<u64> {
        self.submitted_revision
    }

    #[allow(dead_code)]
    pub fn prepare_commit(&mut self, mut submission: RenderSubmissionPlan) -> RenderCommitPlan {
        let previous_revision = self.submitted_revision;
        if previous_revision == Some(submission.revision)
            && self.viewport == Some(submission.viewport)
        {
            return RenderCommitPlan {
                submit: false,
                previous_revision,
                revision: submission.revision,
                skipped_revisions: 0,
                requires_full_repaint: false,
                submission: None,
            };
        }

        let viewport_changed = self
            .viewport
            .is_some_and(|viewport| viewport != submission.viewport);
        let requires_full_repaint =
            previous_revision.is_none() || viewport_changed || submission.full;
        let skipped_revisions = previous_revision
            .map(|revision| {
                submission
                    .revision
                    .saturating_sub(revision)
                    .saturating_sub(1)
            })
            .unwrap_or(0);

        if requires_full_repaint {
            submission.full = true;
            submission.damage_rects = vec![submission.viewport];
        }

        self.submitted_revision = Some(submission.revision);
        self.viewport = Some(submission.viewport);

        RenderCommitPlan {
            submit: true,
            previous_revision,
            revision: submission.revision,
            skipped_revisions,
            requires_full_repaint,
            submission: Some(submission),
        }
    }
}

impl RenderGeometryPlan {
    #[allow(dead_code)]
    pub fn to_submission_plan(&self) -> RenderSubmissionPlan {
        let damage_rects = self.damage_rects();
        let background_quads = self
            .cell_runs
            .iter()
            .map(|run| RenderBackgroundQuad {
                rect: run.rect,
                style: run.style.clone(),
            })
            .collect();
        let text_runs = self
            .glyph_runs
            .iter()
            .map(|run| RenderTextRun {
                row: run.row,
                col: run.col,
                cells: run.cells,
                text: run.text.clone(),
                rect: run.rect,
                style: run.style.clone(),
            })
            .collect();
        let cursor = self.cursor.as_ref().map(|cursor| RenderCursorQuad {
            rect: cursor.rect,
            visible: cursor.visible,
            shape: cursor.shape.clone(),
        });

        RenderSubmissionPlan {
            revision: self.revision,
            cols: self.cols,
            rows: self.rows,
            scrollback_rows: self.scrollback_rows,
            viewport: self.viewport,
            full: self.full,
            damage_rects,
            background_quads,
            text_runs,
            cursor,
        }
    }

    fn damage_rects(&self) -> Vec<RenderRect> {
        if self.full {
            return vec![self.viewport];
        }

        let Some(dirty_rows) = self.dirty_rows else {
            return Vec::new();
        };
        if self.rows == 0 || dirty_rows.start > dirty_rows.end {
            return Vec::new();
        }

        let cell_height = self.viewport.height / self.rows;
        let row_count = dirty_rows.end.saturating_sub(dirty_rows.start) + 1;
        vec![RenderRect {
            x: 0,
            y: dirty_rows.start.saturating_mul(cell_height),
            width: self.viewport.width,
            height: row_count.saturating_mul(cell_height),
        }]
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
    pub total_terminal_response_bytes: u64,
    pub recorded_chunks: u64,
    pub last_bytes: usize,
    pub last_terminal_response_bytes: usize,
    pub last_recorded: bool,
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
pub struct EngineRuntimeQueueHealthSnapshot {
    pub pending_commands: usize,
    pub pending_input_bytes: usize,
    pub pending_lifecycle_commands: usize,
    pub pending_input_commands: usize,
    pub pending_render_commands: usize,
    pub pending_screen_commands: usize,
    pub pending_background_commands: usize,
    pub rejected_commands: u64,
    pub rejected_input_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct EngineRuntimePumpHealthSnapshot {
    pub drain_calls: u64,
    pub dispatched_commands: u64,
    pub dispatched_lifecycle_commands: u64,
    pub dispatched_input_commands: u64,
    pub dispatched_render_commands: u64,
    pub dispatched_screen_commands: u64,
    pub dispatched_background_commands: u64,
    pub waited_for_response: u64,
    pub completed_without_wait: u64,
    pub total_dispatch_elapsed_micros: u64,
    pub max_dispatch_elapsed_micros: u64,
    pub total_drain_elapsed_micros: u64,
    pub max_drain_elapsed_micros: u64,
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
    pub runtime_queue: Option<EngineRuntimeQueueHealthSnapshot>,
    pub runtime_pump: Option<EngineRuntimePumpHealthSnapshot>,
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
    #[allow(dead_code)]
    fn read_render_commit_plan(
        &self,
        pane_id: usize,
        metrics: RenderCellMetrics,
        consumer: &mut RenderConsumerState,
    ) -> Result<RenderCommitPlan> {
        let draw_plan = self.read_render_draw_plan(pane_id, consumer.submitted_revision())?;
        Ok(consumer.prepare_commit(draw_plan.to_geometry_plan(metrics).to_submission_plan()))
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
                // Plain-text fallback: the source has no wrap state to carry.
                wrapped: false,
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
                    wrapped: false,
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
            lines: vec![StyledScreenLine { row: 0, wrapped: false, cells }],
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
    fn screen_engine_default_reads_render_commit_plan() {
        let engine = FakeScreenEngine;
        let mut consumer = RenderConsumerState::new();
        let metrics = RenderCellMetrics {
            cell_width_px: 8,
            cell_height_px: 16,
        };

        let first = engine
            .read_render_commit_plan(1, metrics, &mut consumer)
            .unwrap();
        assert!(first.submit);
        assert_eq!(first.revision, 11);
        assert!(first.requires_full_repaint);
        assert_eq!(first.submission.unwrap().damage_rects.len(), 1);

        let repeat = engine
            .read_render_commit_plan(1, metrics, &mut consumer)
            .unwrap();
        assert!(!repeat.submit);
        assert_eq!(repeat.previous_revision, Some(11));
        assert!(repeat.submission.is_none());
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
        assert_eq!(plan.glyph_runs[0].cells, 2);
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
                wrapped: false,
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
    fn render_submission_plan_maps_geometry_to_renderer_commands() {
        let fg = CellStyle {
            fg: Some(StyledColor::Palette(2)),
            ..CellStyle::default()
        };
        let bg = CellStyle {
            bg: Some(StyledColor::Palette(4)),
            ..CellStyle::default()
        };
        let geometry = frame(vec![
            cell('o', fg.clone(), 1),
            cell('k', fg.clone(), 1),
            cell(' ', bg.clone(), 1),
        ])
        .to_draw_plan()
        .to_geometry_plan(RenderCellMetrics {
            cell_width_px: 10,
            cell_height_px: 20,
        });

        let submission = geometry.to_submission_plan();
        assert_eq!(submission.revision, 7);
        assert!(!submission.full);
        assert_eq!(
            submission.damage_rects,
            vec![RenderRect {
                x: 0,
                y: 0,
                width: 60,
                height: 20,
            }]
        );
        assert_eq!(submission.text_runs.len(), 1);
        assert_eq!(submission.text_runs[0].text, "ok");
        assert_eq!(submission.text_runs[0].row, 0);
        assert_eq!(submission.text_runs[0].col, 0);
        assert_eq!(submission.text_runs[0].cells, 2);
        assert_eq!(submission.text_runs[0].style, fg);
        assert_eq!(submission.background_quads.len(), 2);
        assert_eq!(submission.background_quads[1].style, bg);
        assert_eq!(
            submission.cursor,
            Some(RenderCursorQuad {
                rect: RenderRect {
                    x: 20,
                    y: 0,
                    width: 10,
                    height: 20,
                },
                visible: true,
                shape: "block".to_string(),
            })
        );
    }

    #[test]
    fn render_submission_plan_uses_full_viewport_damage_for_full_frames() {
        let mut full = frame(vec![cell('x', CellStyle::default(), 1)]);
        full.full = true;
        full.dirty_rows = None;
        full.rows = 2;

        let submission = full
            .to_draw_plan()
            .to_geometry_plan(RenderCellMetrics {
                cell_width_px: 8,
                cell_height_px: 16,
            })
            .to_submission_plan();

        assert!(submission.full);
        assert_eq!(
            submission.damage_rects,
            vec![RenderRect {
                x: 0,
                y: 0,
                width: 48,
                height: 32,
            }]
        );
    }

    #[test]
    fn render_submission_plan_uses_dirty_row_damage_for_partial_frames() {
        let dirty = RenderGeometryPlan {
            revision: 10,
            cols: 5,
            rows: 4,
            scrollback_rows: 0,
            dirty_rows: Some(DirtyRows { start: 1, end: 2 }),
            full: false,
            viewport: RenderRect {
                x: 0,
                y: 0,
                width: 50,
                height: 80,
            },
            glyph_runs: Vec::new(),
            cell_runs: Vec::new(),
            cursor: None,
        };

        assert_eq!(
            dirty.to_submission_plan().damage_rects,
            vec![RenderRect {
                x: 0,
                y: 20,
                width: 50,
                height: 40,
            }]
        );
    }

    #[test]
    fn render_consumer_state_forces_first_commit_to_full_damage() {
        let submission = frame(vec![cell('x', CellStyle::default(), 1)])
            .to_draw_plan()
            .to_geometry_plan(RenderCellMetrics {
                cell_width_px: 8,
                cell_height_px: 16,
            })
            .to_submission_plan();
        let mut state = RenderConsumerState::new();

        let commit = state.prepare_commit(submission);

        assert!(commit.submit);
        assert_eq!(commit.previous_revision, None);
        assert!(commit.requires_full_repaint);
        assert_eq!(commit.skipped_revisions, 0);
        assert_eq!(
            commit.submission.unwrap().damage_rects,
            vec![RenderRect {
                x: 0,
                y: 0,
                width: 48,
                height: 16,
            }]
        );
        assert_eq!(state.submitted_revision(), Some(7));
    }

    #[test]
    fn render_consumer_state_skips_repeated_revision() {
        let submission = frame(vec![cell('x', CellStyle::default(), 1)])
            .to_draw_plan()
            .to_geometry_plan(RenderCellMetrics {
                cell_width_px: 8,
                cell_height_px: 16,
            })
            .to_submission_plan();
        let mut state = RenderConsumerState::new();
        assert!(state.prepare_commit(submission.clone()).submit);

        let repeat = state.prepare_commit(submission);

        assert!(!repeat.submit);
        assert_eq!(repeat.previous_revision, Some(7));
        assert_eq!(repeat.revision, 7);
        assert!(repeat.submission.is_none());
    }

    #[test]
    fn render_consumer_state_preserves_dirty_damage_for_incremental_commit() {
        let mut state = RenderConsumerState::new();
        let mut first = frame(vec![cell('x', CellStyle::default(), 1)]);
        first.full = true;
        first.rows = 3;
        state.prepare_commit(
            first
                .to_draw_plan()
                .to_geometry_plan(RenderCellMetrics {
                    cell_width_px: 8,
                    cell_height_px: 16,
                })
                .to_submission_plan(),
        );

        let dirty = RenderGeometryPlan {
            revision: 9,
            cols: 6,
            rows: 3,
            scrollback_rows: 0,
            dirty_rows: Some(DirtyRows { start: 2, end: 2 }),
            full: false,
            viewport: RenderRect {
                x: 0,
                y: 0,
                width: 48,
                height: 48,
            },
            glyph_runs: Vec::new(),
            cell_runs: Vec::new(),
            cursor: None,
        }
        .to_submission_plan();

        let commit = state.prepare_commit(dirty);

        assert!(commit.submit);
        assert!(!commit.requires_full_repaint);
        assert_eq!(commit.previous_revision, Some(7));
        assert_eq!(commit.skipped_revisions, 1);
        assert_eq!(
            commit.submission.unwrap().damage_rects,
            vec![RenderRect {
                x: 0,
                y: 32,
                width: 48,
                height: 16,
            }]
        );
    }

    #[test]
    fn render_consumer_state_forces_full_damage_when_viewport_changes() {
        let mut state = RenderConsumerState::new();
        state.prepare_commit(
            frame(vec![cell('x', CellStyle::default(), 1)])
                .to_draw_plan()
                .to_geometry_plan(RenderCellMetrics {
                    cell_width_px: 8,
                    cell_height_px: 16,
                })
                .to_submission_plan(),
        );
        let resized = RenderGeometryPlan {
            revision: 8,
            cols: 8,
            rows: 2,
            scrollback_rows: 0,
            dirty_rows: Some(DirtyRows { start: 1, end: 1 }),
            full: false,
            viewport: RenderRect {
                x: 0,
                y: 0,
                width: 64,
                height: 32,
            },
            glyph_runs: Vec::new(),
            cell_runs: Vec::new(),
            cursor: None,
        }
        .to_submission_plan();

        let commit = state.prepare_commit(resized);

        assert!(commit.requires_full_repaint);
        assert_eq!(
            commit.submission.unwrap().damage_rects,
            vec![RenderRect {
                x: 0,
                y: 0,
                width: 64,
                height: 32,
            }]
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
                    wrapped: false,
                    cells: vec![cell('a', CellStyle::default(), 1)],
                },
                StyledScreenLine {
                    row: 11,
                    wrapped: false,
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
