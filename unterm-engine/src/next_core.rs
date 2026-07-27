use super::{
    CellStyle, CreateSessionRequest, CursorSnapshot, DirtyRows, EngineHealthSnapshot,
    EngineIoHealthSnapshot, EngineLifecycleHealthSnapshot, HealthEngine, InputEngine,
    LaunchContextSnapshot, LaunchEnvBinding, LaunchEnvSource, LaunchPolicyDecision,
    LaunchPolicyDecisionSnapshot, LaunchPolicySnapshot, ProcessTreeSnapshot, RecordingEngine,
    RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot, RecordingStopResult,
    RenderFrameSnapshot, ScreenEngine, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, SessionActivitySnapshot, SessionEngine,
    SessionSnapshot, ShellSnapshot, SplitSessionRequest, StyledBlink, StyledCell, StyledScreenLine,
    StyledScreenSnapshot, StyledScrollbackSnapshot, StyledUnderline, StyledVerticalAlign,
};
use anyhow::{bail, Result};
use base64::Engine as _;
use parking_lot::{Mutex, RwLock};
use portable_pty::{native_pty_system, Child, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod activity;
mod cell;
mod history;
mod input_pipeline;
mod osc133;
mod parser_state;
mod process_tree;
mod recording_text;
mod render_state;
mod screen_state;
mod terminal_queries;

use activity::SessionIoActivity;
use cell::{CellAttributes, ScreenCell, TerminalColor};
use history::HistoryBuffer;
use parser_state::ParserState;
use render_state::RenderState;
use screen_state::{MouseTrackingMode, ScreenState};

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_RECORDING_BLOCKS: usize = 256;
const MAX_SCROLLBACK_LINES: usize = 10_000;

#[derive(Clone, Copy, Debug, Default)]
pub struct NextCoreEngine;

struct NextCoreSession {
    snapshot: SessionSnapshot,
    root_pid: Option<u32>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output: Arc<Mutex<String>>,
    screen: Arc<Mutex<NextCoreScreen>>,
    recording: Arc<Mutex<Option<NextCoreRecording>>>,
    activity: Arc<Mutex<SessionIoActivity>>,
    dead: Arc<AtomicBool>,
    dead_reason: Arc<Mutex<Option<String>>>,
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
    total_sessions_created: u64,
    total_sessions_destroyed: u64,
    total_sessions_marked_dead: u64,
    last_dead_reason: Option<String>,
}

#[derive(Clone, Debug)]
struct NextCoreRecording {
    session_id: String,
    pane_id: usize,
    project_path: Option<String>,
    project_slug: String,
    started_at: String,
    log_path: PathBuf,
    md_path: PathBuf,
    bytes_raw: u64,
    block_count: u64,
    trace_ids: Vec<String>,
    text_preview: String,
    blocks: Vec<NextCoreRecordingBlock>,
    osc133_seen: bool,
    command_blocks: Vec<NextCoreCommandBlock>,
    active_command: Option<NextCoreActiveCommand>,
}

#[derive(Clone, Debug)]
struct NextCoreRecordingBlock {
    index: u64,
    timestamp_micros: u128,
    text: String,
}

#[derive(Clone, Debug)]
struct NextCoreCommandBlock {
    index: u64,
    started_micros: u128,
    ended_micros: Option<u128>,
    exit_code: Option<String>,
    text: String,
}

#[derive(Clone, Debug)]
struct NextCoreActiveCommand {
    index: u64,
    started_micros: u128,
    text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RecordingIndexEntry {
    unterm_session_id: String,
    tab_id: u64,
    project_path: Option<String>,
    project_slug: String,
    started_at: String,
    ended_at: Option<String>,
    block_count: u64,
    total_lines: u64,
    bytes_raw: u64,
    log_path: String,
    md_path: String,
    exit_reason: Option<String>,
    parent_session_id: Option<String>,
    osc133_active: bool,
    redaction_active: bool,
    redaction_count: u64,
    trace_ids: Vec<String>,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    agent_manifest_version: Option<String>,
    #[serde(default)]
    agent_profile: Option<String>,
}

#[derive(Default)]
struct NextCoreScreen {
    cols: usize,
    history: HistoryBuffer,
    lines: Vec<Vec<ScreenCell>>,
    cursor_x: usize,
    cursor_y: usize,
    cursor_visible: bool,
    cursor_blinking: bool,
    cursor_shape: String,
    column_132_mode: bool,
    auto_wrap: bool,
    reverse_video: bool,
    application_cursor_keys: bool,
    application_keypad: bool,
    focus_event_reporting: bool,
    mouse_tracking: MouseTrackingMode,
    utf8_mouse: bool,
    urxvt_mouse: bool,
    sgr_mouse: bool,
    alternate_scroll: bool,
    sgr_pixel_mouse: bool,
    meta_sends_escape: bool,
    synchronized_output: bool,
    alternate_screen_modes: BTreeSet<usize>,
    origin_mode: bool,
    insert_mode: bool,
    left_right_margin_mode: bool,
    tab_stops: BTreeSet<usize>,
    bracketed_paste: bool,
    current_attr: CellAttributes,
    hyperlinks: Vec<String>,
    title: Option<String>,
    title_stack: Vec<Option<String>>,
    current_dir: Option<String>,
    render_state: RenderState,
    rows: usize,
    scroll_top: usize,
    scroll_bottom: usize,
    left_margin: usize,
    right_margin: usize,
    saved_cursor_x: usize,
    saved_cursor_y: usize,
    saved_cursor_attr: CellAttributes,
    alternate: Option<ScreenState>,
    parser: TerminalParser,
}

impl NextCoreScreen {
    const SGR_UNDERLINE_STYLE_BASE: usize = 10_000;

    fn new(cols: usize, rows: usize) -> Self {
        let mut screen = Self {
            cols: cols.max(1),
            rows: rows.max(1),
            cursor_visible: true,
            cursor_blinking: true,
            cursor_shape: "Default".to_string(),
            auto_wrap: true,
            ..Self::default()
        };
        screen.tab_stops = Self::default_tab_stops(screen.cols);
        screen.scroll_bottom = screen.rows - 1;
        screen.right_margin = screen.cols - 1;
        screen.ensure_cursor_line();
        screen
    }

    fn feed(&mut self, chunk: &str) {
        if !chunk.is_empty() {
            self.bump_revision();
        }
        let mut parser = std::mem::take(&mut self.parser);
        parser.feed(self, chunk);
        self.parser = parser;
    }

    fn bump_revision(&mut self) {
        self.render_state.bump_revision();
    }

    fn clear_dirty_rows(&mut self) {
        self.render_state.clear_dirty_rows();
    }

    fn mark_dirty_row(&mut self, row: usize) {
        self.render_state.mark_dirty_row(row, self.rows);
    }

    fn mark_dirty_range(&mut self, start: usize, end: usize) {
        self.render_state.mark_dirty_range(start, end, self.rows);
    }

    fn mark_all_dirty(&mut self) {
        self.render_state.mark_all_dirty(self.rows);
    }

    fn snapshot_lines(&self) -> Vec<String> {
        self.history_lines()
            .into_iter()
            .map(Self::line_text)
            .collect()
    }

    fn snapshot_viewport_lines(&self) -> Vec<String> {
        self.history_text_range(self.viewport_start(), self.rows)
    }

    #[allow(dead_code)]
    fn styled_viewport_lines(&self, first_row: i64) -> Vec<StyledScreenLine> {
        let viewport_start = self.viewport_start();
        (0..self.rows)
            .map(|idx| StyledScreenLine {
                row: first_row + idx as i64,
                cells: self.styled_viewport_cells(self.history_line(viewport_start + idx)),
            })
            .collect()
    }

    fn styled_viewport_dirty_lines(
        &self,
        dirty_rows: DirtyRows,
        first_row: i64,
    ) -> Vec<StyledScreenLine> {
        let viewport_start = self.viewport_start();
        (dirty_rows.start..=dirty_rows.end)
            .map(|row| StyledScreenLine {
                row: first_row + row as i64,
                cells: self.styled_viewport_cells(self.history_line(viewport_start + row)),
            })
            .collect()
    }

    fn styled_viewport_cells(&self, line: Option<&Vec<ScreenCell>>) -> Vec<StyledCell> {
        let mut cells: Vec<StyledCell> = line
            .into_iter()
            .flat_map(|line| {
                line.iter().take(self.cols).map(|cell| {
                    cell.styled_with_reverse_video(self.reverse_video, &self.hyperlinks)
                })
            })
            .collect();
        while cells.len() < self.cols {
            cells.push(
                ScreenCell::blank(CellAttributes::default())
                    .styled_with_reverse_video(self.reverse_video, &self.hyperlinks),
            );
        }
        cells
    }

    fn styled_history_range(&self, start: usize, count: usize) -> Vec<StyledScreenLine> {
        self.history_range(start, count)
            .into_iter()
            .enumerate()
            .map(|(idx, line)| StyledScreenLine {
                row: start as i64 + idx as i64,
                cells: line
                    .iter()
                    .map(|cell| {
                        cell.styled_with_reverse_video(self.reverse_video, &self.hyperlinks)
                    })
                    .collect(),
            })
            .collect()
    }

    fn scrollback_rows(&self) -> usize {
        self.history.scrollback_rows()
    }

    fn revision(&self) -> u64 {
        self.render_state.revision()
    }

    fn dirty_rows(&self) -> Option<DirtyRows> {
        self.render_state.dirty_rows()
    }

    fn can_render_delta_since(&self, since_revision: u64) -> bool {
        self.render_state.can_render_delta_since(since_revision)
    }

    fn cursor_snapshot(&self) -> CursorSnapshot {
        CursorSnapshot {
            x: self.cursor_x,
            y: self.cursor_y as isize,
            visible: self.cursor_visible,
            shape: self.cursor_shape.clone(),
        }
    }

    fn title(&self) -> Option<String> {
        self.title.clone()
    }

    fn current_dir(&self) -> Option<String> {
        self.current_dir.clone()
    }

    fn history_lines(&self) -> Vec<&Vec<ScreenCell>> {
        self.history.history_lines(&self.lines)
    }

    fn history_len(&self) -> usize {
        self.history.history_len(self.lines.len())
    }

    fn viewport_start(&self) -> usize {
        self.history.viewport_start(self.rows, self.lines.len())
    }

    fn viewport_first_row(&self) -> i64 {
        self.viewport_start() as i64
    }

    fn set_viewport_top_near(&mut self, target: isize) {
        self.history
            .set_viewport_top_near(target, self.rows, self.lines.len());
        self.bump_revision();
        self.mark_all_dirty();
    }

    fn history_range(&self, start: usize, count: usize) -> Vec<&Vec<ScreenCell>> {
        self.history.history_range(&self.lines, start, count)
    }

    fn history_line(&self, index: usize) -> Option<&Vec<ScreenCell>> {
        self.history.history_line(&self.lines, index)
    }

    fn history_text_range(&self, start: usize, count: usize) -> Vec<String> {
        self.history_range(start, count)
            .into_iter()
            .map(Self::line_text)
            .collect()
    }

    fn line_text(line: &Vec<ScreenCell>) -> String {
        let mut text = String::new();
        for cell in line.iter().filter(|cell| cell.width > 0) {
            text.push(cell.ch);
            text.push_str(&cell.combining);
        }
        text.trim_end().to_string()
    }

    fn put_char(&mut self, c: char) {
        self.ensure_cursor_line();
        self.mark_dirty_row(self.cursor_y);
        let cell = ScreenCell::new(c, self.current_attr);
        if cell.width == 0 {
            let line = &mut self.lines[self.cursor_y];
            let end = self.cursor_x.min(line.len());
            if end > 0 {
                if let Some(previous) = line[..end].iter_mut().rev().find(|cell| cell.width > 0) {
                    previous.push_combining(c);
                }
            }
            return;
        }
        self.put_cell(cell);
    }

    fn put_cell(&mut self, cell: ScreenCell) {
        let width = cell.width;
        let attr = cell.attr;
        let left_margin = self.active_left_margin();
        let right_margin = self.active_right_margin();
        if self.cursor_x > right_margin || self.cursor_x + width - 1 > right_margin {
            if self.auto_wrap {
                self.newline();
                self.cursor_x = left_margin;
                self.ensure_cursor_line();
                self.mark_dirty_row(self.cursor_y);
            } else {
                self.cursor_x = right_margin.saturating_sub(width.saturating_sub(1));
            }
        }
        {
            let line = &mut self.lines[self.cursor_y];
            if self.cursor_x > line.len() {
                line.resize(
                    self.cursor_x.min(self.cols),
                    ScreenCell::blank(self.current_attr),
                );
            }
            if self.insert_mode && self.cursor_x < self.cols {
                for _ in 0..width {
                    line.insert(self.cursor_x, ScreenCell::blank(self.current_attr));
                }
                if line.len() > self.cols {
                    line.truncate(self.cols);
                }
            }
            if self.cursor_x == line.len() {
                line.push(cell);
            } else if self.cursor_x < self.cols {
                line[self.cursor_x] = cell;
            }
            if width > 1 {
                for offset in 1..width {
                    let idx = self.cursor_x + offset;
                    if idx >= self.cols {
                        break;
                    }
                    if idx == line.len() {
                        line.push(ScreenCell::continuation(attr));
                    } else if idx < line.len() {
                        line[idx] = ScreenCell::continuation(attr);
                    }
                }
            }
            if line.len() > self.cols {
                line.truncate(self.cols);
            }
        }
        self.cursor_x += width;
    }

    fn repeat_previous_char(&mut self, count: usize) {
        self.ensure_cursor_line();
        let end = self.cursor_x.min(self.lines[self.cursor_y].len());
        let Some(cell) = self.lines[self.cursor_y][..end]
            .iter()
            .rev()
            .find(|cell| cell.width > 0)
            .cloned()
        else {
            return;
        };
        for _ in 0..count.max(1) {
            self.mark_dirty_row(self.cursor_y);
            self.put_cell(cell.clone());
        }
    }

    fn newline(&mut self) {
        let old_y = self.cursor_y;
        self.cursor_x = 0;
        self.index();
        self.mark_dirty_row(old_y);
    }

    fn index(&mut self) {
        let old_y = self.cursor_y;
        if self.cursor_y == self.scroll_bottom {
            self.scroll_up_region(self.scroll_top, self.scroll_bottom, 1);
        } else if self.cursor_y + 1 < self.rows {
            self.cursor_y += 1;
            self.mark_dirty_row(old_y);
            self.mark_dirty_row(self.cursor_y);
        }
        self.ensure_cursor_line();
    }

    fn next_line(&mut self) {
        self.cursor_x = 0;
        self.index();
    }

    fn reverse_index(&mut self) {
        let old_y = self.cursor_y;
        if self.cursor_y == self.scroll_top {
            self.scroll_down_region(self.scroll_top, self.scroll_bottom, 1);
        } else if self.cursor_y > 0 {
            self.cursor_y = self.cursor_y.saturating_sub(1);
            self.mark_dirty_row(old_y);
            self.mark_dirty_row(self.cursor_y);
        }
        self.ensure_cursor_line();
    }

    fn carriage_return(&mut self) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_x = 0;
    }

    fn backspace(&mut self) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_x = self.cursor_x.saturating_sub(1);
    }

    fn horizontal_tab(&mut self) {
        self.cursor_forward_tab(1);
    }

    fn cursor_forward_tab(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        for _ in 0..count.max(1) {
            let next_tab = self
                .tab_stops
                .range((self.cursor_x + 1)..)
                .next()
                .copied()
                .unwrap_or_else(|| self.cols.saturating_sub(1));
            self.cursor_x = next_tab.min(self.cols.saturating_sub(1));
        }
    }

    fn reverse_horizontal_tab(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        for _ in 0..count.max(1) {
            let previous = self
                .tab_stops
                .range(..self.cursor_x)
                .next_back()
                .copied()
                .unwrap_or(0);
            self.cursor_x = previous;
        }
    }

    fn set_tab_stop(&mut self) {
        if self.cursor_x < self.cols {
            self.tab_stops.insert(self.cursor_x);
        }
    }

    fn clear_tab_stop(&mut self, mode: usize) {
        match mode {
            0 => {
                self.tab_stops.remove(&self.cursor_x);
            }
            3 => self.tab_stops.clear(),
            _ => {}
        }
    }

    fn save_cursor(&mut self) {
        self.saved_cursor_x = self.cursor_x;
        self.saved_cursor_y = self.cursor_y;
        self.saved_cursor_attr = self.current_attr;
    }

    fn restore_cursor(&mut self) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_x = self.saved_cursor_x;
        self.cursor_y = self.saved_cursor_y;
        self.current_attr = self.saved_cursor_attr;
        self.ensure_cursor_line();
        self.mark_dirty_row(self.cursor_y);
    }

    fn set_cursor_shape(&mut self, shape: usize) {
        self.cursor_shape = match shape {
            1 => "BlinkingBlock",
            2 => "SteadyBlock",
            3 => "BlinkingUnderline",
            4 => "SteadyUnderline",
            5 => "BlinkingBar",
            6 => "SteadyBar",
            _ => "Default",
        }
        .to_string();
        self.mark_dirty_row(self.cursor_y);
    }

    fn push_title(&mut self) {
        self.title_stack.push(self.title.clone());
    }

    fn pop_title(&mut self) {
        if let Some(title) = self.title_stack.pop() {
            self.title = title;
        }
    }

    fn set_bracketed_paste(&mut self, enabled: bool) {
        self.bracketed_paste = enabled;
    }

    fn active_left_margin(&self) -> usize {
        if self.left_right_margin_mode {
            self.left_margin.min(self.cols.saturating_sub(1))
        } else {
            0
        }
    }

    fn active_right_margin(&self) -> usize {
        if self.left_right_margin_mode {
            self.right_margin
                .min(self.cols.saturating_sub(1))
                .max(self.active_left_margin())
        } else {
            self.cols.saturating_sub(1)
        }
    }

    fn active_top_margin(&self) -> usize {
        if self.origin_mode {
            self.scroll_top.min(self.rows.saturating_sub(1))
        } else {
            0
        }
    }

    fn active_bottom_margin(&self) -> usize {
        if self.origin_mode {
            self.scroll_bottom
                .min(self.rows.saturating_sub(1))
                .max(self.active_top_margin())
        } else {
            self.rows.saturating_sub(1)
        }
    }

    fn set_origin_mode(&mut self, enabled: bool) {
        self.origin_mode = enabled;
        self.set_cursor_position(0, 0);
    }

    fn ensure_cursor_line(&mut self) {
        while self.lines.len() <= self.cursor_y {
            self.lines.push(Vec::new());
        }
    }

    fn ensure_rows_through(&mut self, row: usize) {
        while self.lines.len() <= row {
            self.lines.push(Vec::new());
        }
    }

    fn set_cursor(&mut self, row: usize, col: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_y = row.min(self.rows.saturating_sub(1));
        self.cursor_x = col
            .max(self.active_left_margin())
            .min(self.active_right_margin());
        self.ensure_cursor_line();
        self.mark_dirty_row(self.cursor_y);
    }

    fn set_cursor_position(&mut self, row: usize, col: usize) {
        let row = if self.origin_mode {
            self.scroll_top
                .saturating_add(row)
                .min(self.scroll_bottom)
                .min(self.rows.saturating_sub(1))
        } else {
            row.min(self.rows.saturating_sub(1))
        };
        let col = if self.origin_mode && self.left_right_margin_mode {
            self.active_left_margin().saturating_add(col)
        } else {
            col
        };
        self.set_cursor(row, col);
    }

    fn move_cursor_up(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_y = self
            .cursor_y
            .saturating_sub(count)
            .max(self.active_top_margin());
        self.mark_dirty_row(self.cursor_y);
    }

    fn move_cursor_down(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_y = (self.cursor_y + count).min(self.active_bottom_margin());
        self.ensure_cursor_line();
        self.mark_dirty_row(self.cursor_y);
    }

    fn move_cursor_right(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_x = (self.cursor_x + count).min(self.active_right_margin());
    }

    fn move_cursor_left(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_x = self
            .cursor_x
            .saturating_sub(count)
            .max(self.active_left_margin());
    }

    fn cursor_next_line(&mut self, count: usize) {
        self.move_cursor_down(count.max(1));
        self.cursor_x = self.active_left_margin();
    }

    fn cursor_previous_line(&mut self, count: usize) {
        self.move_cursor_up(count.max(1));
        self.cursor_x = self.active_left_margin();
    }

    fn set_horizontal_position(&mut self, col: usize) {
        let col = if self.origin_mode && self.left_right_margin_mode {
            self.active_left_margin().saturating_add(col)
        } else {
            col
        };
        self.set_cursor(self.cursor_y, col);
    }

    fn set_vertical_position(&mut self, row: usize) {
        self.set_cursor_position(row, self.cursor_x);
    }

    fn clear_screen(&mut self) {
        self.lines.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.ensure_cursor_line();
        self.mark_all_dirty();
    }

    fn clear_display(&mut self) {
        self.lines.clear();
        self.ensure_cursor_line();
        self.mark_all_dirty();
    }

    fn fill_alignment_test(&mut self) {
        self.lines = (0..self.rows)
            .map(|_| vec![ScreenCell::new('E', self.current_attr); self.cols])
            .collect();
        self.mark_all_dirty();
    }

    fn fill_rect(&mut self, ch: char, top: usize, left: usize, bottom: usize, right: usize) {
        if self.rows == 0 || self.cols == 0 {
            return;
        }
        let Some((top, left, bottom, right)) = self.clip_rect(top, left, bottom, right) else {
            return;
        };

        self.ensure_rows_through(bottom);
        let cell = ScreenCell::new(ch, self.current_attr);
        for row in top..=bottom {
            let line = &mut self.lines[row];
            if line.len() <= right {
                line.resize(right + 1, ScreenCell::blank(self.current_attr));
            }
            for col in left..=right {
                line[col] = cell.clone();
            }
        }
        self.mark_dirty_range(top, bottom);
    }

    fn erase_rect(&mut self, top: usize, left: usize, bottom: usize, right: usize) {
        if self.rows == 0 || self.cols == 0 {
            return;
        }
        let Some((top, left, bottom, right)) = self.clip_rect(top, left, bottom, right) else {
            return;
        };

        self.ensure_rows_through(bottom);
        let blank = ScreenCell::blank(self.current_attr);
        for row in top..=bottom {
            let line = &mut self.lines[row];
            if line.len() <= right {
                line.resize(right + 1, ScreenCell::blank(self.current_attr));
            }
            for col in left..=right {
                line[col] = blank.clone();
            }
        }
        self.mark_dirty_range(top, bottom);
    }

    fn selective_erase_rect(&mut self, top: usize, left: usize, bottom: usize, right: usize) {
        if self.rows == 0 || self.cols == 0 {
            return;
        }
        let Some((top, left, bottom, right)) = self.clip_rect(top, left, bottom, right) else {
            return;
        };

        self.ensure_rows_through(bottom);
        let blank = ScreenCell::blank(self.current_attr);
        for row in top..=bottom {
            let line = &mut self.lines[row];
            if line.len() <= right {
                line.resize(right + 1, ScreenCell::blank(self.current_attr));
            }
            for col in left..=right {
                if !line[col].attr.protected {
                    line[col] = blank.clone();
                }
            }
        }
        self.mark_dirty_range(top, bottom);
    }

    fn change_rect_attributes(
        &mut self,
        top: usize,
        left: usize,
        bottom: usize,
        right: usize,
        params: &[usize],
    ) {
        if self.rows == 0 || self.cols == 0 {
            return;
        }
        let Some((top, left, bottom, right)) = self.clip_rect(top, left, bottom, right) else {
            return;
        };

        self.ensure_rows_through(bottom);
        let params = if params.is_empty() { &[0][..] } else { params };
        for row in top..=bottom {
            let line = &mut self.lines[row];
            if line.len() <= right {
                line.resize(right + 1, ScreenCell::blank(self.current_attr));
            }
            for col in left..=right {
                Self::apply_deccara_attributes(&mut line[col].attr, params);
            }
        }
        self.mark_dirty_range(top, bottom);
    }

    fn reverse_rect_attributes(
        &mut self,
        top: usize,
        left: usize,
        bottom: usize,
        right: usize,
        params: &[usize],
    ) {
        if self.rows == 0 || self.cols == 0 {
            return;
        }
        let Some((top, left, bottom, right)) = self.clip_rect(top, left, bottom, right) else {
            return;
        };

        self.ensure_rows_through(bottom);
        let params = if params.is_empty() { &[0][..] } else { params };
        for row in top..=bottom {
            let line = &mut self.lines[row];
            if line.len() <= right {
                line.resize(right + 1, ScreenCell::blank(self.current_attr));
            }
            for col in left..=right {
                Self::reverse_decrara_attributes(&mut line[col].attr, params);
            }
        }
        self.mark_dirty_range(top, bottom);
    }

    fn apply_deccara_attributes(attr: &mut CellAttributes, params: &[usize]) {
        for param in params {
            match *param {
                0 => {
                    attr.bold = false;
                    attr.faint = false;
                    attr.clear_underline();
                    attr.blink = None;
                    attr.inverse = false;
                }
                1 => attr.bold = true,
                4 => attr.set_underline(StyledUnderline::Single),
                5 => attr.blink = Some(StyledBlink::Slow),
                7 => attr.inverse = true,
                8 => attr.hidden = true,
                22 => {
                    attr.bold = false;
                    attr.faint = false;
                }
                24 => attr.clear_underline(),
                25 => attr.blink = None,
                27 => attr.inverse = false,
                28 => attr.hidden = false,
                _ => {}
            }
        }
    }

    fn reverse_decrara_attributes(attr: &mut CellAttributes, params: &[usize]) {
        for param in params {
            match *param {
                0 => {
                    attr.bold = !attr.bold;
                    attr.underline = !attr.underline;
                    attr.underline_style = if attr.underline {
                        Some(attr.underline_style.unwrap_or(StyledUnderline::Single))
                    } else {
                        None
                    };
                    attr.blink = if attr.blink.is_some() {
                        None
                    } else {
                        Some(StyledBlink::Slow)
                    };
                    attr.inverse = !attr.inverse;
                }
                1 => attr.bold = !attr.bold,
                4 => {
                    attr.underline = !attr.underline;
                    attr.underline_style = if attr.underline {
                        Some(attr.underline_style.unwrap_or(StyledUnderline::Single))
                    } else {
                        None
                    };
                }
                5 => {
                    attr.blink = if attr.blink.is_some() {
                        None
                    } else {
                        Some(StyledBlink::Slow)
                    };
                }
                7 => attr.inverse = !attr.inverse,
                8 => attr.hidden = !attr.hidden,
                _ => {}
            }
        }
    }

    fn set_character_protection(&mut self, mode: usize) {
        self.current_attr.set_protected(mode == 1);
    }

    fn clip_rect(
        &self,
        top: usize,
        left: usize,
        bottom: usize,
        right: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let top = top.min(self.rows.saturating_sub(1));
        let bottom = bottom.min(self.rows.saturating_sub(1));
        let left = left.min(self.cols.saturating_sub(1));
        let right = right.min(self.cols.saturating_sub(1));
        if top > bottom || left > right {
            None
        } else {
            Some((top, left, bottom, right))
        }
    }

    fn reset_terminal(&mut self) {
        let cols = self.cols;
        let rows = self.rows;
        let revision = self.revision();
        let title = self.title.take();
        let current_dir = self.current_dir.take();

        *self = Self::new(cols, rows);
        self.render_state.set_revision(revision);
        self.title = title;
        self.current_dir = current_dir;
        self.mark_all_dirty();
    }

    fn soft_reset_terminal(&mut self) {
        self.current_attr = CellAttributes::default();
        self.insert_mode = false;
        self.left_right_margin_mode = false;
        self.origin_mode = false;
        self.auto_wrap = true;
        self.reverse_video = false;
        self.application_cursor_keys = false;
        self.application_keypad = false;
        self.focus_event_reporting = false;
        self.mouse_tracking = MouseTrackingMode::None;
        self.utf8_mouse = false;
        self.urxvt_mouse = false;
        self.sgr_mouse = false;
        self.alternate_scroll = false;
        self.sgr_pixel_mouse = false;
        self.meta_sends_escape = false;
        self.synchronized_output = false;
        self.cursor_visible = true;
        self.cursor_blinking = true;
        self.cursor_shape = "Default".to_string();
        self.tab_stops = Self::default_tab_stops(self.cols);
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.left_margin = 0;
        self.right_margin = self.cols.saturating_sub(1);
        self.saved_cursor_x = 0;
        self.saved_cursor_y = 0;
        self.saved_cursor_attr = CellAttributes::default();
        if let Some(alternate) = self.alternate.as_mut() {
            alternate.saved_cursor_x = 0;
            alternate.saved_cursor_y = 0;
            alternate.saved_cursor_attr = CellAttributes::default();
        }
        self.ensure_cursor_line();
        self.mark_all_dirty();
    }

    fn erase_line_range(&mut self, row: usize, start: usize, end: usize, selective: bool) {
        if start >= end || row >= self.rows {
            return;
        }
        self.ensure_rows_through(row);
        let line = &mut self.lines[row];
        let end = end.min(self.cols);
        if line.len() < end {
            line.resize(end, ScreenCell::blank(self.current_attr));
        }
        let blank = ScreenCell::blank(self.current_attr);
        for cell in line.iter_mut().take(end).skip(start) {
            if !selective || !cell.attr.protected {
                *cell = blank.clone();
            }
        }
        self.mark_dirty_row(row);
    }

    fn erase_in_display(&mut self, mode: usize) {
        self.erase_in_display_with_protection(mode, false);
    }

    fn selective_erase_in_display(&mut self, mode: usize) {
        self.erase_in_display_with_protection(mode, true);
    }

    fn erase_in_display_with_protection(&mut self, mode: usize, selective: bool) {
        match mode {
            0 => {
                self.erase_in_line_with_protection(0, selective);
                let start = self.cursor_y + 1;
                if start < self.rows {
                    if selective {
                        for row in start..self.rows.min(self.lines.len()) {
                            self.erase_line_range(row, 0, self.cols, true);
                        }
                    } else if start < self.lines.len() {
                        for line in self.lines.iter_mut().skip(start) {
                            line.clear();
                        }
                        self.mark_dirty_range(start, self.rows.saturating_sub(1));
                    }
                }
            }
            1 => {
                let end = self.cursor_y.min(self.lines.len().saturating_sub(1));
                if selective {
                    for row in 0..self.cursor_y.min(self.lines.len()) {
                        self.erase_line_range(row, 0, self.cols, true);
                    }
                } else {
                    for line in self.lines.iter_mut().take(end) {
                        line.clear();
                    }
                    self.mark_dirty_range(0, self.cursor_y);
                }
                self.erase_in_line_with_protection(1, selective);
            }
            2 => {
                if selective {
                    for row in 0..self.rows.min(self.lines.len()) {
                        self.erase_line_range(row, 0, self.cols, true);
                    }
                } else {
                    self.clear_display();
                }
            }
            3 => {
                self.history.clear();
                self.mark_all_dirty();
            }
            _ => {}
        }
    }

    fn erase_in_line(&mut self, mode: usize) {
        self.erase_in_line_with_protection(mode, false);
    }

    fn selective_erase_in_line(&mut self, mode: usize) {
        self.erase_in_line_with_protection(mode, true);
    }

    fn erase_in_line_with_protection(&mut self, mode: usize, selective: bool) {
        self.ensure_cursor_line();
        match mode {
            0 => {
                let start = self.cursor_x.min(self.cols);
                self.erase_line_range(self.cursor_y, start, self.cols, selective);
            }
            1 => {
                let end = self.cursor_x.saturating_add(1).min(self.cols);
                self.erase_line_range(self.cursor_y, 0, end, selective);
            }
            2 => {
                self.erase_line_range(self.cursor_y, 0, self.cols, selective);
            }
            _ => {}
        }
    }

    fn insert_chars(&mut self, count: usize) {
        self.ensure_cursor_line();
        let left = self.cursor_x.max(self.active_left_margin());
        let right = self.active_right_margin();
        if left > right {
            return;
        }
        let count = count.max(1).min(right + 1 - left);
        let line = &mut self.lines[self.cursor_y];
        if line.len() < right + 1 {
            line.resize(right + 1, ScreenCell::blank(self.current_attr));
        }
        for idx in (left..=right).rev() {
            line[idx] = if idx >= left + count {
                line[idx - count].clone()
            } else {
                ScreenCell::blank(self.current_attr)
            };
        }
        self.mark_dirty_row(self.cursor_y);
    }

    fn delete_chars(&mut self, count: usize) {
        self.ensure_cursor_line();
        let left = self.cursor_x.max(self.active_left_margin());
        let right = self.active_right_margin();
        if left > right {
            return;
        }
        let count = count.max(1).min(right + 1 - left);
        let line = &mut self.lines[self.cursor_y];
        if line.len() < right + 1 {
            line.resize(right + 1, ScreenCell::blank(self.current_attr));
        }
        for idx in left..=right {
            let source = idx + count;
            line[idx] = if source <= right {
                line[source].clone()
            } else {
                ScreenCell::blank(self.current_attr)
            };
        }
        self.mark_dirty_row(self.cursor_y);
    }

    fn erase_chars(&mut self, count: usize) {
        self.ensure_cursor_line();
        let end = self
            .cursor_x
            .saturating_add(count.max(1))
            .min(self.active_right_margin() + 1);
        let line = &mut self.lines[self.cursor_y];
        if line.len() < end {
            line.resize(end, ScreenCell::blank(self.current_attr));
        }
        for cell in line.iter_mut().take(end).skip(self.cursor_x) {
            *cell = ScreenCell::blank(self.current_attr);
        }
        self.mark_dirty_row(self.cursor_y);
    }

    fn scroll_left(&mut self, count: usize) {
        self.ensure_cursor_line();
        let left = self.cursor_x.max(self.active_left_margin());
        let right = self.active_right_margin();
        if left > right {
            return;
        }
        let count = count.max(1).min(right + 1 - left);
        let line = &mut self.lines[self.cursor_y];
        if line.len() < right + 1 {
            line.resize(right + 1, ScreenCell::blank(self.current_attr));
        }
        for idx in left..=right {
            let source = idx + count;
            line[idx] = if source <= right {
                line[source].clone()
            } else {
                ScreenCell::blank(self.current_attr)
            };
        }
        self.mark_dirty_row(self.cursor_y);
    }

    fn scroll_right(&mut self, count: usize) {
        self.ensure_cursor_line();
        let left = self.cursor_x.max(self.active_left_margin());
        let right = self.active_right_margin();
        if left > right {
            return;
        }
        let count = count.max(1).min(right + 1 - left);
        let line = &mut self.lines[self.cursor_y];
        if line.len() < right + 1 {
            line.resize(right + 1, ScreenCell::blank(self.current_attr));
        }
        for idx in (left..=right).rev() {
            line[idx] = if idx >= left + count {
                line[idx - count].clone()
            } else {
                ScreenCell::blank(self.current_attr)
            };
        }
        self.mark_dirty_row(self.cursor_y);
    }

    #[cfg(test)]
    fn attrs_for_viewport(&self) -> Vec<Vec<CellAttributes>> {
        self.lines
            .iter()
            .map(|line| line.iter().map(|cell| cell.attr).collect())
            .collect()
    }

    fn apply_sgr(&mut self, params: &[usize]) {
        let params = if params.is_empty() { &[0][..] } else { params };
        let mut idx = 0;
        while idx < params.len() {
            match params[idx] {
                0 => self.current_attr = CellAttributes::default(),
                1 => self.current_attr.bold = true,
                2 => self.current_attr.faint = true,
                3 => self.current_attr.italic = true,
                4 => self.current_attr.set_underline(StyledUnderline::Single),
                5 => self.current_attr.blink = Some(StyledBlink::Slow),
                6 => self.current_attr.blink = Some(StyledBlink::Rapid),
                7 => self.current_attr.inverse = true,
                8 => self.current_attr.hidden = true,
                9 => self.current_attr.strikethrough = true,
                22 => {
                    self.current_attr.bold = false;
                    self.current_attr.faint = false;
                }
                23 => self.current_attr.italic = false,
                21 => self.current_attr.set_underline(StyledUnderline::Double),
                24 => self.current_attr.clear_underline(),
                25 => self.current_attr.blink = None,
                27 => self.current_attr.inverse = false,
                28 => self.current_attr.hidden = false,
                29 => self.current_attr.strikethrough = false,
                53 => self.current_attr.overline = true,
                55 => self.current_attr.overline = false,
                73 => self.current_attr.vertical_align = Some(StyledVerticalAlign::SuperScript),
                74 => self.current_attr.vertical_align = Some(StyledVerticalAlign::SubScript),
                75 => self.current_attr.vertical_align = None,
                underline_style
                    if (Self::SGR_UNDERLINE_STYLE_BASE..=Self::SGR_UNDERLINE_STYLE_BASE + 5)
                        .contains(&underline_style) =>
                {
                    match underline_style - Self::SGR_UNDERLINE_STYLE_BASE {
                        0 => self.current_attr.clear_underline(),
                        1 => self.current_attr.set_underline(StyledUnderline::Single),
                        2 => self.current_attr.set_underline(StyledUnderline::Double),
                        3 => self.current_attr.set_underline(StyledUnderline::Curly),
                        4 => self.current_attr.set_underline(StyledUnderline::Dotted),
                        5 => self.current_attr.set_underline(StyledUnderline::Dashed),
                        _ => {}
                    }
                }
                30..=37 => {
                    self.current_attr.fg = Some(TerminalColor::Palette(params[idx] as u8 - 30))
                }
                39 => self.current_attr.fg = None,
                40..=47 => {
                    self.current_attr.bg = Some(TerminalColor::Palette(params[idx] as u8 - 40))
                }
                49 => self.current_attr.bg = None,
                90..=97 => {
                    self.current_attr.fg = Some(TerminalColor::Palette(params[idx] as u8 - 90 + 8))
                }
                100..=107 => {
                    self.current_attr.bg = Some(TerminalColor::Palette(params[idx] as u8 - 100 + 8))
                }
                38 | 48 | 58 => {
                    let color_target = params[idx];
                    if let Some((color, consumed)) = Self::parse_extended_color(&params[idx + 1..])
                    {
                        match color_target {
                            38 => self.current_attr.fg = Some(color),
                            48 => self.current_attr.bg = Some(color),
                            58 => self.current_attr.underline_color = Some(color),
                            _ => {}
                        }
                        idx += consumed;
                    }
                }
                59 => self.current_attr.underline_color = None,
                _ => {}
            }
            idx += 1;
        }
    }

    fn apply_osc(&mut self, sequence: &str) {
        let Some((kind, value)) = sequence.split_once(';') else {
            return;
        };
        if matches!(kind, "0" | "2") && !value.is_empty() {
            self.title = Some(value.to_string());
        } else if kind == "7" {
            if let Some(cwd) = Self::parse_osc7_cwd(value) {
                self.current_dir = Some(cwd);
            }
        } else if kind == "8" {
            self.apply_osc8_hyperlink(value);
        }
    }

    fn apply_osc8_hyperlink(&mut self, value: &str) {
        let Some((_params, uri)) = value.split_once(';') else {
            return;
        };
        if uri.is_empty() {
            self.current_attr.hyperlink = None;
            return;
        }
        let idx = self
            .hyperlinks
            .iter()
            .position(|known| known == uri)
            .unwrap_or_else(|| {
                self.hyperlinks.push(uri.to_string());
                self.hyperlinks.len() - 1
            });
        self.current_attr.hyperlink = Some(idx);
    }

    fn parse_osc7_cwd(value: &str) -> Option<String> {
        let uri = value.strip_prefix("file://")?;
        let path = if uri.starts_with('/') {
            uri
        } else {
            let slash = uri.find('/')?;
            &uri[slash..]
        };
        let decoded = Self::percent_decode(path)?;
        if decoded.is_empty() {
            return None;
        }
        #[cfg(windows)]
        {
            let mut path = decoded;
            let bytes = path.as_bytes();
            if path.starts_with('/')
                && bytes.len() >= 4
                && bytes[2] == b':'
                && bytes[1].is_ascii_alphabetic()
            {
                path.remove(0);
            }
            Some(path.replace('/', "\\"))
        }
        #[cfg(not(windows))]
        {
            Some(decoded)
        }
    }

    fn percent_decode(input: &str) -> Option<String> {
        let bytes = input.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut idx = 0;
        while idx < bytes.len() {
            if bytes[idx] == b'%' {
                let hi = *bytes.get(idx + 1)?;
                let lo = *bytes.get(idx + 2)?;
                out.push(Self::hex_value(hi)? << 4 | Self::hex_value(lo)?);
                idx += 3;
            } else {
                out.push(bytes[idx]);
                idx += 1;
            }
        }
        String::from_utf8(out).ok()
    }

    fn hex_value(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    fn parse_extended_color(params: &[usize]) -> Option<(TerminalColor, usize)> {
        match params {
            [5, color, ..] => Some((TerminalColor::Palette((*color).min(255) as u8), 2)),
            [2, r, g, b, ..] => Some((
                TerminalColor::Rgb(
                    (*r).min(255) as u8,
                    (*g).min(255) as u8,
                    (*b).min(255) as u8,
                ),
                4,
            )),
            _ => None,
        }
    }

    fn parse_sgr_params(raw_params: &str) -> Vec<usize> {
        let raw_params = raw_params.trim_start_matches('?');
        if raw_params.is_empty() {
            return vec![0];
        }

        let mut params = Vec::new();
        for part in raw_params.split(';') {
            let part = part.trim();
            if part.is_empty() {
                params.push(0);
            } else if part.starts_with("38:") || part.starts_with("48:") || part.starts_with("58:")
            {
                params.extend(Self::parse_colon_color_sgr_params(part));
            } else if let Some(underline) = Self::parse_colon_underline_sgr_param(part) {
                params.push(underline);
            } else if let Some((first, _)) = part.split_once(':') {
                params.push(first.trim().parse::<usize>().unwrap_or(0));
            } else {
                params.push(part.parse::<usize>().unwrap_or(0));
            }
        }

        if params.is_empty() {
            vec![0]
        } else {
            params
        }
    }

    fn parse_csi_numbers(raw_params: &str) -> Vec<usize> {
        let raw_params = raw_params
            .trim_start_matches('?')
            .trim_end_matches(|c: char| !c.is_ascii_digit() && c != ';' && c != ':');
        if raw_params.is_empty() {
            return Vec::new();
        }
        raw_params
            .split(';')
            .map(|part| part.trim().parse::<usize>().unwrap_or(0))
            .collect()
    }

    fn rect_from_numbers(&self, numbers: &[usize]) -> (usize, usize, usize, usize) {
        let top = numbers.first().copied().filter(|n| *n > 0).unwrap_or(1);
        let left = numbers.get(1).copied().filter(|n| *n > 0).unwrap_or(1);
        let bottom = numbers
            .get(2)
            .copied()
            .filter(|n| *n > 0)
            .unwrap_or(self.rows);
        let right = numbers
            .get(3)
            .copied()
            .filter(|n| *n > 0)
            .unwrap_or(self.cols);
        (
            top.saturating_sub(1),
            left.saturating_sub(1),
            bottom.saturating_sub(1),
            right.saturating_sub(1),
        )
    }

    fn parse_colon_color_sgr_params(part: &str) -> Vec<usize> {
        let mut pieces = part.split(':');
        let target = pieces
            .next()
            .and_then(|part| part.parse::<usize>().ok())
            .unwrap_or(0);
        let mode = pieces
            .next()
            .and_then(|part| part.parse::<usize>().ok())
            .unwrap_or(0);

        match mode {
            5 => pieces
                .find_map(|part| part.parse::<usize>().ok())
                .map(|color| vec![target, 5, color])
                .unwrap_or_else(|| vec![target]),
            2 => {
                let values = pieces
                    .filter_map(|part| part.parse::<usize>().ok())
                    .collect::<Vec<_>>();
                if values.len() >= 3 {
                    let start = values.len().saturating_sub(3);
                    vec![
                        target,
                        2,
                        values[start],
                        values[start + 1],
                        values[start + 2],
                    ]
                } else {
                    vec![target]
                }
            }
            _ => vec![target],
        }
    }

    fn parse_colon_underline_sgr_param(part: &str) -> Option<usize> {
        let (prefix, value) = part.split_once(':')?;
        if prefix.trim() != "4" {
            return None;
        }
        let value = value
            .split(':')
            .next()
            .unwrap_or_default()
            .trim()
            .parse::<usize>()
            .ok()?;
        Some(Self::SGR_UNDERLINE_STYLE_BASE + value.min(5))
    }

    fn insert_lines(&mut self, count: usize) {
        self.ensure_cursor_line();
        let bottom = if self.cursor_y >= self.scroll_top && self.cursor_y <= self.scroll_bottom {
            self.scroll_bottom
        } else {
            self.rows.saturating_sub(1)
        };
        self.ensure_rows_through(bottom);
        self.mark_dirty_range(self.cursor_y, bottom);
        for _ in 0..count.max(1) {
            self.lines.insert(self.cursor_y, Vec::new());
            if self.lines.len() > bottom + 1 {
                self.lines.remove(bottom + 1);
            }
        }
    }

    fn delete_lines(&mut self, count: usize) {
        self.ensure_cursor_line();
        let bottom = if self.cursor_y >= self.scroll_top && self.cursor_y <= self.scroll_bottom {
            self.scroll_bottom
        } else {
            self.rows.saturating_sub(1)
        };
        self.ensure_rows_through(bottom);
        self.mark_dirty_range(self.cursor_y, bottom);
        for _ in 0..count.max(1) {
            if self.cursor_y <= bottom && self.cursor_y < self.lines.len() {
                self.lines.remove(self.cursor_y);
            }
            self.lines.insert(bottom, Vec::new());
            if self.lines.len() > self.rows {
                self.lines.truncate(self.rows);
            }
        }
        self.ensure_cursor_line();
    }

    fn scroll_up(&mut self, count: usize) {
        self.scroll_up_region(self.scroll_top, self.scroll_bottom, count);
    }

    fn scroll_up_region(&mut self, top: usize, bottom: usize, count: usize) {
        if top > bottom {
            return;
        }
        self.ensure_rows_through(bottom);
        self.mark_dirty_range(top, bottom);
        for _ in 0..count.max(1) {
            let removed = self.lines.remove(top);
            if top == 0 && bottom + 1 >= self.rows && self.alternate.is_none() {
                self.history.push_scrollback(removed, MAX_SCROLLBACK_LINES);
            }
            self.lines.insert(bottom, Vec::new());
        }
        self.cursor_y = self.cursor_y.min(self.rows.saturating_sub(1));
    }

    fn scroll_down(&mut self, count: usize) {
        self.scroll_down_region(self.scroll_top, self.scroll_bottom, count);
    }

    fn scroll_down_region(&mut self, top: usize, bottom: usize, count: usize) {
        if top > bottom {
            return;
        }
        self.ensure_rows_through(bottom);
        self.mark_dirty_range(top, bottom);
        for _ in 0..count.max(1) {
            self.lines.remove(bottom);
            self.lines.insert(top, Vec::new());
        }
        self.cursor_y = self.cursor_y.min(self.rows.saturating_sub(1));
    }

    fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let top = top.min(self.rows.saturating_sub(1));
        let bottom = bottom.min(self.rows.saturating_sub(1));
        if top >= bottom {
            self.scroll_top = 0;
            self.scroll_bottom = self.rows.saturating_sub(1);
        } else {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
        self.set_cursor_position(0, 0);
        self.mark_all_dirty();
    }

    fn set_horizontal_margins(&mut self, left: usize, right: usize) {
        let left = left.min(self.cols.saturating_sub(1));
        let right = right.min(self.cols.saturating_sub(1));
        if left >= right {
            self.left_margin = 0;
            self.right_margin = self.cols.saturating_sub(1);
        } else {
            self.left_margin = left;
            self.right_margin = right;
        }
        self.set_cursor_position(0, 0);
        self.mark_all_dirty();
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
        self.bump_revision();
        self.clear_dirty_rows();
        self.mark_all_dirty();
        Self::truncate_lines_to_cols(&mut self.lines, self.cols);
        self.history.truncate_scrollback_to_cols(self.cols);
        let cols = self.cols;
        if let Some(alternate) = self.alternate.as_mut() {
            alternate.cols = cols;
            Self::truncate_lines_to_cols(&mut alternate.lines, cols);
            Self::truncate_lines_to_cols(&mut alternate.scrollback, cols);
            alternate.tab_stops.retain(|stop| *stop < cols);
            alternate.cursor_x = alternate.cursor_x.min(cols.saturating_sub(1));
            alternate.saved_cursor_x = alternate.saved_cursor_x.min(cols.saturating_sub(1));
        }
        self.tab_stops.retain(|stop| *stop < cols);
        if self.lines.len() > self.rows {
            let trim = self.lines.len() - self.rows;
            let drained = self.lines.drain(..trim).collect::<Vec<_>>();
            if self.alternate.is_none() {
                self.history
                    .extend_scrollback(drained, MAX_SCROLLBACK_LINES);
            }
            self.cursor_y = self.cursor_y.saturating_sub(trim);
            self.saved_cursor_y = self.saved_cursor_y.saturating_sub(trim);
        }
        self.cursor_x = self.cursor_x.min(self.cols.saturating_sub(1));
        self.saved_cursor_x = self.saved_cursor_x.min(self.cols.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(self.rows.saturating_sub(1));
        self.scroll_top = self.scroll_top.min(self.rows.saturating_sub(1));
        self.scroll_bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
        if self.scroll_top >= self.scroll_bottom {
            self.scroll_top = 0;
            self.scroll_bottom = self.rows.saturating_sub(1);
        }
        self.left_margin = self.left_margin.min(self.cols.saturating_sub(1));
        self.right_margin = self.right_margin.min(self.cols.saturating_sub(1));
        if self.left_margin >= self.right_margin {
            self.left_margin = 0;
            self.right_margin = self.cols.saturating_sub(1);
        }
        self.ensure_cursor_line();
    }

    fn set_column_mode(&mut self, wide: bool) {
        self.column_132_mode = wide;
        self.cols = if wide { 132 } else { 80 };
        self.lines.clear();
        self.history.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.saved_cursor_x = 0;
        self.saved_cursor_y = 0;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.left_right_margin_mode = false;
        self.left_margin = 0;
        self.right_margin = self.cols.saturating_sub(1);
        self.tab_stops = Self::default_tab_stops(self.cols);
        self.ensure_cursor_line();
        self.mark_all_dirty();
    }

    fn truncate_lines_to_cols(lines: &mut [Vec<ScreenCell>], cols: usize) {
        for line in lines {
            if line.len() > cols {
                line.truncate(cols);
            }
        }
    }

    fn default_tab_stops(cols: usize) -> BTreeSet<usize> {
        (8..cols).step_by(8).collect()
    }

    fn enter_alternate_screen(&mut self, mode: usize, clear: bool) {
        if self.alternate.is_some() {
            self.alternate_screen_modes.insert(mode);
            if clear {
                self.clear_screen();
            }
            return;
        }

        let main = ScreenState {
            cols: self.cols,
            scrollback: self.history.take_scrollback(),
            lines: std::mem::take(&mut self.lines),
            viewport_top: self.history.take_viewport_top(),
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
            cursor_visible: self.cursor_visible,
            cursor_blinking: self.cursor_blinking,
            cursor_shape: self.cursor_shape.clone(),
            column_132_mode: self.column_132_mode,
            auto_wrap: self.auto_wrap,
            reverse_video: self.reverse_video,
            application_cursor_keys: self.application_cursor_keys,
            application_keypad: self.application_keypad,
            focus_event_reporting: self.focus_event_reporting,
            mouse_tracking: self.mouse_tracking,
            utf8_mouse: self.utf8_mouse,
            urxvt_mouse: self.urxvt_mouse,
            sgr_mouse: self.sgr_mouse,
            alternate_scroll: self.alternate_scroll,
            sgr_pixel_mouse: self.sgr_pixel_mouse,
            meta_sends_escape: self.meta_sends_escape,
            synchronized_output: self.synchronized_output,
            alternate_screen_modes: std::mem::take(&mut self.alternate_screen_modes),
            origin_mode: self.origin_mode,
            insert_mode: self.insert_mode,
            left_right_margin_mode: self.left_right_margin_mode,
            tab_stops: std::mem::take(&mut self.tab_stops),
            bracketed_paste: self.bracketed_paste,
            current_attr: self.current_attr,
            scroll_top: self.scroll_top,
            scroll_bottom: self.scroll_bottom,
            left_margin: self.left_margin,
            right_margin: self.right_margin,
            saved_cursor_x: self.saved_cursor_x,
            saved_cursor_y: self.saved_cursor_y,
            saved_cursor_attr: self.saved_cursor_attr,
        };
        self.alternate = Some(main);
        self.alternate_screen_modes.insert(mode);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.saved_cursor_x = 0;
        self.saved_cursor_y = 0;
        self.saved_cursor_attr = CellAttributes::default();
        self.cursor_shape = "Default".to_string();
        self.cursor_blinking = true;
        self.auto_wrap = true;
        self.reverse_video = false;
        self.application_cursor_keys = false;
        self.application_keypad = false;
        self.focus_event_reporting = false;
        self.mouse_tracking = MouseTrackingMode::None;
        self.utf8_mouse = false;
        self.urxvt_mouse = false;
        self.sgr_mouse = false;
        self.alternate_scroll = false;
        self.sgr_pixel_mouse = false;
        self.meta_sends_escape = false;
        self.synchronized_output = false;
        self.origin_mode = false;
        self.insert_mode = false;
        self.left_right_margin_mode = false;
        self.tab_stops = Self::default_tab_stops(self.cols);
        self.bracketed_paste = false;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.left_margin = 0;
        self.right_margin = self.cols.saturating_sub(1);
        self.lines.clear();
        self.ensure_cursor_line();
        self.mark_all_dirty();
    }

    fn leave_alternate_screen(&mut self, mode: usize) {
        self.alternate_screen_modes.remove(&mode);
        if !self.alternate_screen_modes.is_empty() {
            return;
        }
        if let Some(main) = self.alternate.take() {
            self.restore_main_screen(main);
        }
    }

    fn restore_main_screen(&mut self, main: ScreenState) {
        self.cols = main.cols;
        self.history.replace_scrollback(main.scrollback);
        self.lines = main.lines;
        self.history.replace_viewport_top(main.viewport_top);
        self.cursor_x = main.cursor_x;
        self.cursor_y = main.cursor_y;
        self.cursor_visible = main.cursor_visible;
        self.cursor_blinking = main.cursor_blinking;
        self.cursor_shape = main.cursor_shape;
        self.column_132_mode = main.column_132_mode;
        self.auto_wrap = main.auto_wrap;
        self.reverse_video = main.reverse_video;
        self.application_cursor_keys = main.application_cursor_keys;
        self.application_keypad = main.application_keypad;
        self.focus_event_reporting = main.focus_event_reporting;
        self.mouse_tracking = main.mouse_tracking;
        self.utf8_mouse = main.utf8_mouse;
        self.urxvt_mouse = main.urxvt_mouse;
        self.sgr_mouse = main.sgr_mouse;
        self.alternate_scroll = main.alternate_scroll;
        self.sgr_pixel_mouse = main.sgr_pixel_mouse;
        self.meta_sends_escape = main.meta_sends_escape;
        self.synchronized_output = main.synchronized_output;
        self.alternate_screen_modes = main.alternate_screen_modes;
        self.origin_mode = main.origin_mode;
        self.insert_mode = main.insert_mode;
        self.left_right_margin_mode = main.left_right_margin_mode;
        self.tab_stops = main.tab_stops;
        self.bracketed_paste = main.bracketed_paste;
        self.current_attr = main.current_attr;
        self.scroll_top = main.scroll_top;
        self.scroll_bottom = main.scroll_bottom;
        self.left_margin = main.left_margin;
        self.right_margin = main.right_margin;
        self.saved_cursor_x = main.saved_cursor_x;
        self.saved_cursor_y = main.saved_cursor_y;
        self.saved_cursor_attr = main.saved_cursor_attr;
        if self.lines.len() > self.rows {
            let trim = self.lines.len() - self.rows;
            self.lines.drain(..trim);
            self.cursor_y = self.cursor_y.saturating_sub(trim);
            self.saved_cursor_y = self.saved_cursor_y.saturating_sub(trim);
        }
        self.ensure_cursor_line();
        self.mark_all_dirty();
    }
}

#[derive(Default)]
struct TerminalParser {
    state: ParserState,
}

impl TerminalParser {
    fn feed(&mut self, screen: &mut NextCoreScreen, chunk: &str) {
        for c in chunk.chars() {
            self.feed_char(screen, c);
        }
    }

    fn feed_char(&mut self, screen: &mut NextCoreScreen, c: char) {
        match self.state {
            ParserState::Ground => match c {
                '\x1b' => self.state = ParserState::Escape,
                '\u{0084}' => screen.index(),
                '\u{0085}' => screen.next_line(),
                '\u{008d}' => screen.reverse_index(),
                '\u{0090}' | '\u{0098}' | '\u{009e}' | '\u{009f}' => {
                    self.state = ParserState::IgnoredString;
                }
                '\u{009b}' => self.state = ParserState::Csi(String::new()),
                '\u{009d}' => self.state = ParserState::Osc(String::new()),
                '\r' => screen.carriage_return(),
                '\n' | '\x0b' | '\x0c' => screen.newline(),
                '\x08' => screen.backspace(),
                '\t' => screen.horizontal_tab(),
                c if !c.is_control() => screen.put_char(c),
                _ => {}
            },
            ParserState::Escape => match c {
                '[' => self.state = ParserState::Csi(String::new()),
                ']' => self.state = ParserState::Osc(String::new()),
                '=' => {
                    screen.application_keypad = true;
                    self.state = ParserState::Ground;
                }
                '>' => {
                    screen.application_keypad = false;
                    self.state = ParserState::Ground;
                }
                '(' | ')' | '*' | '+' | '-' | '.' | '/' | '%' => {
                    self.state = ParserState::EscapeIgnoreOne;
                }
                '#' => {
                    self.state = ParserState::EscapeHash;
                }
                'P' | 'X' | '^' | '_' => {
                    self.state = ParserState::IgnoredString;
                }
                '7' => {
                    screen.save_cursor();
                    self.state = ParserState::Ground;
                }
                '8' => {
                    screen.restore_cursor();
                    self.state = ParserState::Ground;
                }
                'D' => {
                    screen.index();
                    self.state = ParserState::Ground;
                }
                'E' => {
                    screen.next_line();
                    self.state = ParserState::Ground;
                }
                'H' => {
                    screen.set_tab_stop();
                    self.state = ParserState::Ground;
                }
                'M' => {
                    screen.reverse_index();
                    self.state = ParserState::Ground;
                }
                'c' => {
                    screen.reset_terminal();
                    self.state = ParserState::Ground;
                }
                _ => self.state = ParserState::Ground,
            },
            ParserState::EscapeIgnoreOne => {
                self.state = ParserState::Ground;
            }
            ParserState::EscapeHash => {
                if c == '8' {
                    screen.fill_alignment_test();
                }
                self.state = ParserState::Ground;
            }
            ParserState::IgnoredString => match c {
                '\x07' | '\u{009c}' => self.state = ParserState::Ground,
                '\x1b' => self.state = ParserState::IgnoredStringEscape,
                _ => {}
            },
            ParserState::IgnoredStringEscape => {
                if c == '\\' {
                    self.state = ParserState::Ground;
                } else {
                    self.state = ParserState::IgnoredString;
                }
            }
            ParserState::Csi(ref mut sequence) => {
                if ('@'..='~').contains(&c) {
                    sequence.push(c);
                    let sequence = std::mem::take(sequence);
                    Self::handle_csi(screen, &sequence);
                    self.state = ParserState::Ground;
                } else {
                    sequence.push(c);
                }
            }
            ParserState::Osc(ref mut sequence) => match c {
                '\x07' | '\u{009c}' => {
                    let sequence = std::mem::take(sequence);
                    screen.apply_osc(&sequence);
                    self.state = ParserState::Ground;
                }
                '\x1b' => {
                    let sequence = std::mem::take(sequence);
                    self.state = ParserState::OscEscape(sequence);
                }
                _ => sequence.push(c),
            },
            ParserState::OscEscape(ref mut sequence) => {
                if c == '\\' {
                    let sequence = std::mem::take(sequence);
                    screen.apply_osc(&sequence);
                }
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_csi(screen: &mut NextCoreScreen, sequence: &str) {
        let Some(final_byte) = sequence.chars().last() else {
            return;
        };
        let raw_params = &sequence[..sequence.len().saturating_sub(final_byte.len_utf8())];
        let private = raw_params.starts_with('?');
        let numeric_params = raw_params.trim_start_matches('?');
        let numbers = numeric_params
            .split(';')
            .map(|part| part.trim().parse::<usize>().unwrap_or(0))
            .collect::<Vec<_>>();
        let first = || numbers.first().copied().filter(|n| *n > 0).unwrap_or(1);

        match final_byte {
            '@' => {
                if raw_params.ends_with(' ') {
                    screen.scroll_left(first());
                } else {
                    screen.insert_chars(first());
                }
            }
            'A' => {
                if raw_params.ends_with(' ') {
                    screen.scroll_right(first());
                } else {
                    screen.move_cursor_up(first());
                }
            }
            'B' => screen.move_cursor_down(first()),
            'C' => screen.move_cursor_right(first()),
            'D' => screen.move_cursor_left(first()),
            'E' => screen.cursor_next_line(first()),
            'F' => screen.cursor_previous_line(first()),
            'X' => screen.erase_chars(first()),
            'L' => screen.insert_lines(first()),
            'M' => screen.delete_lines(first()),
            'P' => screen.delete_chars(first()),
            'S' => screen.scroll_up(first()),
            'T' => screen.scroll_down(first()),
            'Z' => screen.reverse_horizontal_tab(first()),
            '`' => screen.set_horizontal_position(first().saturating_sub(1)),
            'a' => screen.move_cursor_right(first()),
            'b' => screen.repeat_previous_char(first()),
            'd' => screen.set_vertical_position(first().saturating_sub(1)),
            'e' => screen.move_cursor_down(first()),
            'G' => {
                let row = screen.cursor_y;
                screen.set_cursor(row, first().saturating_sub(1));
            }
            'I' => screen.cursor_forward_tab(first()),
            'g' => screen.clear_tab_stop(numbers.first().copied().unwrap_or(0)),
            'H' | 'f' => {
                let row = numbers.first().copied().filter(|n| *n > 0).unwrap_or(1);
                let col = numbers.get(1).copied().filter(|n| *n > 0).unwrap_or(1);
                screen.set_cursor_position(row.saturating_sub(1), col.saturating_sub(1));
            }
            'J' => {
                let mode = numbers.first().copied().unwrap_or(0);
                if private {
                    screen.selective_erase_in_display(mode);
                } else {
                    screen.erase_in_display(mode);
                }
            }
            'K' => {
                let mode = numbers.first().copied().unwrap_or(0);
                if private {
                    screen.selective_erase_in_line(mode);
                } else {
                    screen.erase_in_line(mode);
                }
            }
            'm' => screen.apply_sgr(&NextCoreScreen::parse_sgr_params(raw_params)),
            'p' => {
                if raw_params == "!" {
                    screen.soft_reset_terminal();
                }
            }
            'q' => {
                if raw_params.ends_with(' ') {
                    screen.set_cursor_shape(numbers.first().copied().unwrap_or(0));
                } else if raw_params.ends_with('"') {
                    let numbers = NextCoreScreen::parse_csi_numbers(raw_params);
                    screen.set_character_protection(numbers.first().copied().unwrap_or(0));
                }
            }
            's' => {
                if !private && screen.left_right_margin_mode && numbers.len() >= 2 {
                    let left = numbers.first().copied().filter(|n| *n > 0).unwrap_or(1);
                    let right = numbers
                        .get(1)
                        .copied()
                        .filter(|n| *n > 0)
                        .unwrap_or(screen.cols);
                    screen.set_horizontal_margins(left.saturating_sub(1), right.saturating_sub(1));
                } else {
                    screen.save_cursor();
                }
            }
            'u' => screen.restore_cursor(),
            't' => {
                if raw_params.ends_with('$') {
                    let numbers = NextCoreScreen::parse_csi_numbers(raw_params);
                    let (top, left, bottom, right) = screen.rect_from_numbers(&numbers);
                    let params = numbers.get(4..).unwrap_or(&[]);
                    screen.reverse_rect_attributes(top, left, bottom, right, params);
                } else {
                    Self::handle_window_operation(screen, &numbers);
                }
            }
            'x' => {
                if raw_params.ends_with('$') {
                    let numbers = NextCoreScreen::parse_csi_numbers(raw_params);
                    let ch = numbers
                        .first()
                        .copied()
                        .and_then(|code| char::from_u32(code as u32))
                        .filter(|ch| ScreenCell::char_width(*ch) == 1)
                        .unwrap_or(' ');
                    let (top, left, bottom, right) = screen.rect_from_numbers(&numbers[1..]);
                    screen.fill_rect(ch, top, left, bottom, right);
                }
            }
            'z' => {
                if raw_params.ends_with('$') {
                    let numbers = NextCoreScreen::parse_csi_numbers(raw_params);
                    let (top, left, bottom, right) = screen.rect_from_numbers(&numbers);
                    screen.erase_rect(top, left, bottom, right);
                }
            }
            '{' => {
                if raw_params.ends_with('$') {
                    let numbers = NextCoreScreen::parse_csi_numbers(raw_params);
                    let (top, left, bottom, right) = screen.rect_from_numbers(&numbers);
                    screen.selective_erase_rect(top, left, bottom, right);
                }
            }
            'r' => {
                if raw_params.ends_with('$') {
                    let numbers = NextCoreScreen::parse_csi_numbers(raw_params);
                    let (top, left, bottom, right) = screen.rect_from_numbers(&numbers);
                    let params = numbers.get(4..).unwrap_or(&[]);
                    screen.change_rect_attributes(top, left, bottom, right, params);
                    return;
                }
                let top = numbers.first().copied().filter(|n| *n > 0).unwrap_or(1);
                let bottom = numbers
                    .get(1)
                    .copied()
                    .filter(|n| *n > 0)
                    .unwrap_or(screen.rows);
                screen.set_scroll_region(top.saturating_sub(1), bottom.saturating_sub(1));
            }
            'h' => {
                for mode in &numbers {
                    if private {
                        match *mode {
                            1049 => screen.enter_alternate_screen(1049, true),
                            1047 | 47 => screen.enter_alternate_screen(*mode, false),
                            1048 => screen.save_cursor(),
                            5 => {
                                screen.reverse_video = true;
                                screen.mark_all_dirty();
                            }
                            1 => screen.application_cursor_keys = true,
                            3 => screen.set_column_mode(true),
                            6 => screen.set_origin_mode(true),
                            7 => screen.auto_wrap = true,
                            12 => {
                                screen.cursor_blinking = true;
                                screen.mark_dirty_row(screen.cursor_y);
                            }
                            25 => {
                                screen.cursor_visible = true;
                                screen.mark_dirty_row(screen.cursor_y);
                            }
                            66 => screen.application_keypad = true,
                            69 => screen.left_right_margin_mode = true,
                            1000 => screen.mouse_tracking = MouseTrackingMode::X10,
                            1002 => screen.mouse_tracking = MouseTrackingMode::ButtonEvent,
                            1003 => screen.mouse_tracking = MouseTrackingMode::AnyEvent,
                            1004 => screen.focus_event_reporting = true,
                            1005 => screen.utf8_mouse = true,
                            1006 => screen.sgr_mouse = true,
                            1007 => screen.alternate_scroll = true,
                            1015 => screen.urxvt_mouse = true,
                            1016 => screen.sgr_pixel_mouse = true,
                            1034 => screen.meta_sends_escape = true,
                            2004 => screen.set_bracketed_paste(true),
                            2026 => screen.synchronized_output = true,
                            _ => {}
                        }
                    } else if *mode == 4 {
                        screen.insert_mode = true;
                    }
                }
            }
            'l' => {
                for mode in &numbers {
                    if private {
                        match *mode {
                            1049 | 1047 | 47 => screen.leave_alternate_screen(*mode),
                            1048 => screen.restore_cursor(),
                            5 => {
                                screen.reverse_video = false;
                                screen.mark_all_dirty();
                            }
                            1 => screen.application_cursor_keys = false,
                            3 => screen.set_column_mode(false),
                            6 => screen.set_origin_mode(false),
                            7 => screen.auto_wrap = false,
                            12 => {
                                screen.cursor_blinking = false;
                                screen.mark_dirty_row(screen.cursor_y);
                            }
                            25 => {
                                screen.cursor_visible = false;
                                screen.mark_dirty_row(screen.cursor_y);
                            }
                            66 => screen.application_keypad = false,
                            69 => {
                                screen.left_right_margin_mode = false;
                                screen.set_horizontal_margins(0, screen.cols.saturating_sub(1));
                            }
                            1000 => {
                                if screen.mouse_tracking == MouseTrackingMode::X10 {
                                    screen.mouse_tracking = MouseTrackingMode::None;
                                }
                            }
                            1002 => {
                                if screen.mouse_tracking == MouseTrackingMode::ButtonEvent {
                                    screen.mouse_tracking = MouseTrackingMode::None;
                                }
                            }
                            1003 => {
                                if screen.mouse_tracking == MouseTrackingMode::AnyEvent {
                                    screen.mouse_tracking = MouseTrackingMode::None;
                                }
                            }
                            1004 => screen.focus_event_reporting = false,
                            1005 => screen.utf8_mouse = false,
                            1006 => screen.sgr_mouse = false,
                            1007 => screen.alternate_scroll = false,
                            1015 => screen.urxvt_mouse = false,
                            1016 => screen.sgr_pixel_mouse = false,
                            1034 => screen.meta_sends_escape = false,
                            2004 => screen.set_bracketed_paste(false),
                            2026 => screen.synchronized_output = false,
                            _ => {}
                        }
                    } else if *mode == 4 {
                        screen.insert_mode = false;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_window_operation(screen: &mut NextCoreScreen, numbers: &[usize]) {
        let op = numbers.first().copied().unwrap_or(0);
        let target = numbers.get(1).copied().unwrap_or(0);
        match (op, target) {
            (22, 0 | 2) => screen.push_title(),
            (23, 0 | 2) => screen.pop_title(),
            _ => {}
        }
    }
}

fn state() -> &'static RwLock<NextCoreState> {
    static STATE: OnceLock<RwLock<NextCoreState>> = OnceLock::new();
    STATE.get_or_init(|| RwLock::new(NextCoreState::default()))
}

fn recording_index_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
fn reset_state_for_test() {
    *state().write() = NextCoreState::default();
}

#[cfg(test)]
fn set_output_for_test(pane_id: usize, text: &str) -> Result<()> {
    let (output, screen, recording, activity, cols, rows) = {
        let state = state().read();
        let Some(session) = state
            .sessions
            .iter()
            .find(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };
        (
            Arc::clone(&session.output),
            Arc::clone(&session.screen),
            Arc::clone(&session.recording),
            Arc::clone(&session.activity),
            session.snapshot.cols,
            session.snapshot.rows,
        )
    };
    let started_at = Instant::now();
    *output.lock() = text.to_string();
    let mut screen = screen.lock();
    let revision = screen.revision();
    *screen = NextCoreScreen::new(cols, rows);
    screen.render_state.set_revision(revision);
    screen.feed(text);
    if let Some(recording) = recording.lock().as_mut() {
        NextCoreEngine::append_recording_output(recording, text);
    }
    activity
        .lock()
        .mark_output(text.len(), started_at.elapsed());
    Ok(())
}

#[cfg(test)]
fn mark_dead_for_test(pane_id: usize) -> Result<()> {
    let state = state().read();
    let Some(session) = state
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
    else {
        bail!("next-core session {pane_id} not found");
    };

    *session.dead_reason.lock() = Some("test_dead_marker".to_string());
    session.dead.store(true, Ordering::Release);
    Ok(())
}

#[cfg(test)]
fn make_activity_stale_for_test(pane_id: usize) -> Result<()> {
    let state = state().read();
    let Some(session) = state
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
    else {
        bail!("next-core session {pane_id} not found");
    };

    session.activity.lock().mark_stale_for_test();
    Ok(())
}

#[cfg(test)]
fn reset_activity_for_test(pane_id: usize) -> Result<()> {
    let state = state().read();
    let Some(session) = state
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
    else {
        bail!("next-core session {pane_id} not found");
    };

    *session.activity.lock() = SessionIoActivity::new();
    make_activity_stale_for_test(pane_id)
}

#[cfg(test)]
fn viewport_attrs_for_test(pane_id: usize) -> Result<Vec<Vec<CellAttributes>>> {
    let screen = {
        let state = state().read();
        let Some(session) = state
            .sessions
            .iter()
            .find(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };
        Arc::clone(&session.screen)
    };

    let attrs = screen.lock().attrs_for_viewport();
    Ok(attrs)
}

impl NextCoreEngine {
    fn refresh_liveness(session: &mut NextCoreSession) -> Option<String> {
        if session.snapshot.is_dead {
            return None;
        }

        if session.dead.load(Ordering::Acquire) {
            session.snapshot.is_dead = true;
            if session.snapshot.dead_reason.is_none() {
                session.snapshot.dead_reason = session
                    .dead_reason
                    .lock()
                    .clone()
                    .or_else(|| Some("unknown".to_string()));
            }
            return session.snapshot.dead_reason.clone();
        }

        if let Ok(Some(status)) = session.child.lock().try_wait() {
            let reason = format!("process_exited:{status}");
            session.snapshot.is_dead = true;
            session.snapshot.dead_reason = Some(reason.clone());
            *session.dead_reason.lock() = Some(reason);
            session.dead.store(true, Ordering::Release);
            return session.snapshot.dead_reason.clone();
        }

        None
    }

    fn record_dead_reason(state: &mut NextCoreState, reason: String) {
        state.total_sessions_marked_dead = state.total_sessions_marked_dead.saturating_add(1);
        state.last_dead_reason = Some(reason);
    }

    fn sessions(&self) -> Vec<SessionSnapshot> {
        let mut state = state().write();
        let mut snapshots = Vec::with_capacity(state.sessions.len());
        let mut dead_reasons = Vec::new();
        for session in &mut state.sessions {
            if let Some(reason) = Self::refresh_liveness(session) {
                dead_reasons.push(reason);
            }
            let mut snapshot = session.snapshot.clone();
            let screen = session.screen.lock();
            snapshot.cursor = screen.cursor_snapshot();
            snapshot.scrollback_rows = screen.scrollback_rows();
            if let Some(title) = screen.title() {
                snapshot.title = title;
            }
            if let Some(cwd) = screen.current_dir() {
                snapshot.shell.cwd = Some(cwd);
            } else if snapshot.shell.cwd.is_none() {
                if let Some(process) =
                    process_tree::snapshot(session.root_pid, &snapshot.shell.process_name)
                {
                    snapshot.shell.cwd = process.foreground_cwd.or(process.root_cwd);
                }
            }
            snapshots.push(snapshot);
        }
        for reason in dead_reasons {
            Self::record_dead_reason(&mut state, reason);
        }
        snapshots
    }

    fn session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        for session in self.sessions() {
            if session.id == pane_id {
                return Ok(session);
            }
        }

        bail!("next-core session {pane_id} not found")
    }

    fn shell_snapshot(&self, pane_id: usize) -> Result<ShellSnapshot> {
        let (mut shell, screen, root_pid) = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            (
                session.snapshot.shell.clone(),
                Arc::clone(&session.screen),
                session.root_pid,
            )
        };

        if let Some(cwd) = screen.lock().current_dir() {
            shell.cwd = Some(cwd);
            return Ok(shell);
        }

        if shell.cwd.is_none() {
            if let Some(process) = process_tree::snapshot(root_pid, &shell.process_name) {
                shell.cwd = process.foreground_cwd.or(process.root_cwd);
            }
        }
        Ok(shell)
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

    #[doc(hidden)]
    pub fn debug_output(&self, pane_id: usize) -> Result<String> {
        self.output(pane_id)
    }

    fn unix_micros() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_micros())
            .unwrap_or_default()
    }

    fn timestamp_string() -> String {
        Self::unix_micros().to_string()
    }

    fn sanitize_slug(value: &str) -> String {
        value
            .chars()
            .map(|ch| {
                if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                    ch
                } else {
                    '-'
                }
            })
            .collect()
    }

    fn sessions_root() -> PathBuf {
        if let Ok(root) = std::env::var("UNTERM_SESSIONS_ROOT") {
            if !root.trim().is_empty() {
                return PathBuf::from(root);
            }
        }
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        home.join(".unterm").join("sessions")
    }

    fn project_slug(project_path: Option<&str>) -> String {
        project_path
            .and_then(|path| {
                Path::new(path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.is_empty())
                    .map(Self::sanitize_slug)
            })
            .unwrap_or_else(|| "_orphan".to_string())
    }

    fn recording_paths(
        pane_id: usize,
        project_path: Option<&str>,
        project_slug: &str,
    ) -> (PathBuf, PathBuf) {
        let date = Self::timestamp_string();
        let dir = project_path
            .map(PathBuf::from)
            .map(|path| path.join(".unterm").join("sessions").join(&date))
            .unwrap_or_else(|| Self::sessions_root().join(project_slug).join(&date));
        let _ = std::fs::create_dir_all(&dir);
        let stem = format!("tab-{pane_id}-{date}");
        (
            dir.join(format!("{stem}.log")),
            dir.join(format!("{stem}.md")),
        )
    }

    fn index_path() -> PathBuf {
        Self::sessions_root().join("index.json")
    }

    fn index_entry(recording: &NextCoreRecording, ended_at: Option<String>) -> RecordingIndexEntry {
        RecordingIndexEntry {
            unterm_session_id: recording.session_id.clone(),
            tab_id: recording.pane_id as u64,
            project_path: recording.project_path.clone(),
            project_slug: recording.project_slug.clone(),
            started_at: recording.started_at.clone(),
            ended_at,
            block_count: recording.block_count,
            total_lines: recording.text_preview.lines().count() as u64,
            bytes_raw: recording.bytes_raw,
            log_path: recording.log_path.display().to_string(),
            md_path: recording.md_path.display().to_string(),
            exit_reason: None,
            parent_session_id: None,
            osc133_active: recording.osc133_seen,
            redaction_active: true,
            redaction_count: 0,
            trace_ids: recording.trace_ids.clone(),
            agent_id: std::env::var("UNTERM_AGENT_ID").ok(),
            agent_manifest_version: std::env::var("UNTERM_AGENT_MANIFEST_VERSION").ok(),
            agent_profile: std::env::var("UNTERM_PROFILE").ok(),
        }
    }

    fn upsert_recording_index(
        recording: &NextCoreRecording,
        ended_at: Option<String>,
    ) -> Result<()> {
        let _guard = recording_index_lock().lock();
        let path = Self::index_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut entries: Vec<RecordingIndexEntry> = if path.exists() {
            let raw = std::fs::read_to_string(&path).unwrap_or_default();
            if raw.trim().is_empty() {
                Vec::new()
            } else {
                serde_json::from_str(&raw).unwrap_or_default()
            }
        } else {
            Vec::new()
        };
        let entry = Self::index_entry(recording, ended_at);
        if let Some(existing) = entries
            .iter_mut()
            .find(|existing| existing.unterm_session_id == entry.unterm_session_id)
        {
            *existing = entry;
        } else {
            entries.push(entry);
        }
        std::fs::write(path, serde_json::to_string_pretty(&entries)?)?;
        Ok(())
    }

    fn append_recording_output(recording: &mut NextCoreRecording, text: &str) {
        if text.is_empty() {
            return;
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
        let timestamp_micros = Self::unix_micros();
        let line = format!("{}\tout\t{}\n", timestamp_micros, encoded);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&recording.log_path)
        {
            let _ = file.write_all(line.as_bytes());
        }
        recording.bytes_raw = recording.bytes_raw.saturating_add(text.len() as u64);
        recording.block_count = recording.block_count.saturating_add(1);
        recording.blocks.push(NextCoreRecordingBlock {
            index: recording.block_count,
            timestamp_micros,
            text: text.to_string(),
        });
        Self::record_osc133_command_blocks(recording, text, timestamp_micros);
        if recording.blocks.len() > MAX_RECORDING_BLOCKS {
            recording
                .blocks
                .drain(..recording.blocks.len() - MAX_RECORDING_BLOCKS);
        }
        if recording.command_blocks.len() > MAX_RECORDING_BLOCKS {
            recording
                .command_blocks
                .drain(..recording.command_blocks.len() - MAX_RECORDING_BLOCKS);
        }
        recording.text_preview.push_str(text);
        if recording.text_preview.len() > MAX_OUTPUT_BYTES {
            let keep_from = recording.text_preview.len() - MAX_OUTPUT_BYTES;
            let keep_from = recording
                .text_preview
                .char_indices()
                .map(|(idx, _)| idx)
                .find(|idx| *idx >= keep_from)
                .unwrap_or(0);
            recording.text_preview.drain(..keep_from);
        }
    }

    fn record_osc133_command_blocks(
        recording: &mut NextCoreRecording,
        text: &str,
        timestamp_micros: u128,
    ) {
        for item in osc133::split_stream(text) {
            match item {
                osc133::StreamItem::Text(text) => {
                    if let Some(active) = recording.active_command.as_mut() {
                        active.text.push_str(text);
                    }
                }
                osc133::StreamItem::Marker(marker) => {
                    recording.osc133_seen = true;
                    match marker.kind {
                        'C' => {
                            if let Some(active) = recording.active_command.take() {
                                recording.command_blocks.push(NextCoreCommandBlock {
                                    index: active.index,
                                    started_micros: active.started_micros,
                                    ended_micros: None,
                                    exit_code: None,
                                    text: active.text,
                                });
                            }
                            let index = recording
                                .command_blocks
                                .last()
                                .map(|block| block.index.saturating_add(1))
                                .unwrap_or(1);
                            recording.active_command = Some(NextCoreActiveCommand {
                                index,
                                started_micros: timestamp_micros,
                                text: String::new(),
                            });
                        }
                        'D' => {
                            if let Some(active) = recording.active_command.take() {
                                recording.command_blocks.push(NextCoreCommandBlock {
                                    index: active.index,
                                    started_micros: active.started_micros,
                                    ended_micros: Some(timestamp_micros),
                                    exit_code: marker.exit_code,
                                    text: active.text,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn write_recording_markdown(
        recording: &NextCoreRecording,
        ended_at: Option<&str>,
        exit_reason: &str,
    ) -> Result<usize> {
        if let Some(parent) = recording.md_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let markdown = Self::render_recording_markdown(recording, ended_at, exit_reason);
        std::fs::write(&recording.md_path, markdown.as_bytes())?;
        Ok(markdown.len())
    }

    fn render_recording_markdown(
        recording: &NextCoreRecording,
        ended_at: Option<&str>,
        exit_reason: &str,
    ) -> String {
        let stripped = recording_text::strip_ansi(&recording.text_preview);
        let (redacted, redaction_count) = recording_text::redact_text(&stripped);
        let total_lines = redacted.lines().count() as u64;
        let mut md = String::new();

        writeln!(&mut md, "---").ok();
        writeln!(&mut md, "unterm_session_id: {}", recording.session_id).ok();
        writeln!(&mut md, "tab_id: {}", recording.pane_id).ok();
        match &recording.project_path {
            Some(path) => writeln!(&mut md, "project_path: {}", path).ok(),
            None => writeln!(&mut md, "project_path: null").ok(),
        };
        writeln!(&mut md, "project_slug: {}", recording.project_slug).ok();
        writeln!(&mut md, "shell: {}", Self::env_var_or("SHELL", "next-core")).ok();
        writeln!(&mut md, "hostname: {}", Self::hostname()).ok();
        writeln!(&mut md, "unterm_version: next-core").ok();
        writeln!(&mut md, "started_at: {}", recording.started_at).ok();
        match ended_at {
            Some(value) => writeln!(&mut md, "ended_at: {}", value).ok(),
            None => writeln!(&mut md, "ended_at: null").ok(),
        };
        writeln!(&mut md, "exit_reason: {}", exit_reason).ok();
        let command_blocks = Self::recording_command_blocks(recording);
        writeln!(&mut md, "osc133_active: {}", recording.osc133_seen).ok();
        writeln!(
            &mut md,
            "block_render: {}",
            if recording.osc133_seen {
                "osc133_command_blocks"
            } else {
                "chunked_output"
            }
        )
        .ok();
        writeln!(&mut md, "block_count: {}", recording.block_count).ok();
        writeln!(&mut md, "command_block_count: {}", command_blocks.len()).ok();
        writeln!(&mut md, "total_lines: {}", total_lines).ok();
        writeln!(&mut md, "bytes_raw: {}", recording.bytes_raw).ok();
        writeln!(
            &mut md,
            "trace_ids: {}",
            recording_text::yaml_string_array(&recording.trace_ids)
        )
        .ok();
        writeln!(&mut md, "redaction_active: true").ok();
        writeln!(&mut md, "redaction_count: {}", redaction_count).ok();
        writeln!(&mut md, "parent_session_id: null").ok();
        writeln!(&mut md, "---\n").ok();

        let title_ts = recording
            .started_at
            .split('+')
            .next()
            .unwrap_or(&recording.started_at)
            .replace('T', " ");
        writeln!(&mut md, "# Unterm session - {}\n", title_ts).ok();
        if recording.osc133_seen {
            writeln!(
                &mut md,
                "> next-core recording with OSC133 shell command markers.\n"
            )
            .ok();
        } else {
            writeln!(
                &mut md,
                "> next-core fallback recording; shell command markers were not captured.\n"
            )
            .ok();
        }
        if !command_blocks.is_empty() {
            writeln!(&mut md, "## Command Blocks\n").ok();
            for block in &command_blocks {
                let stripped = recording_text::strip_ansi(&block.text);
                let (redacted_block, _) = recording_text::redact_text(&stripped);
                writeln!(
                    &mut md,
                    "### Command {} `{}`\n\n- started: `{}`\n- ended: `{}`\n- exit_code: `{}`\n\n```\n{}\n```\n",
                    block.index,
                    block.started_micros,
                    block.started_micros,
                    block
                        .ended_micros
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "null".to_string()),
                    block.exit_code.as_deref().unwrap_or("null"),
                    redacted_block.trim_end()
                )
                .ok();
            }
        }
        if !recording.blocks.is_empty() {
            writeln!(
                &mut md,
                "## Output Blocks\n\nThese blocks are raw output chunks captured by next-core.\n"
            )
            .ok();
            for block in &recording.blocks {
                let stripped = recording_text::strip_ansi(&block.text);
                let (redacted_block, _) = recording_text::redact_text(&stripped);
                writeln!(
                    &mut md,
                    "### Block {} `{}`\n\n```\n{}\n```\n",
                    block.index,
                    block.timestamp_micros,
                    redacted_block.trim_end()
                )
                .ok();
            }
            writeln!(&mut md, "## Aggregated Preview\n").ok();
        }
        writeln!(&mut md, "```\n{}\n```", redacted.trim_end()).ok();

        md
    }

    fn recording_command_blocks(recording: &NextCoreRecording) -> Vec<NextCoreCommandBlock> {
        let mut blocks = recording.command_blocks.clone();
        if let Some(active) = recording.active_command.as_ref() {
            blocks.push(NextCoreCommandBlock {
                index: active.index,
                started_micros: active.started_micros,
                ended_micros: None,
                exit_code: None,
                text: active.text.clone(),
            });
        }
        blocks
    }

    fn env_var_or(name: &str, default: &str) -> String {
        std::env::var(name).unwrap_or_else(|_| default.to_string())
    }

    fn hostname() -> String {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_default()
    }

    fn screen_lines(&self, pane_id: usize) -> Result<Vec<String>> {
        let screen = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.screen)
        };

        let lines = screen.lock().snapshot_lines();
        Ok(lines)
    }

    fn screen_line_count(&self, pane_id: usize) -> Result<usize> {
        let screen = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.screen)
        };

        let count = screen.lock().history_len();
        Ok(count)
    }

    fn screen_line_text_range(
        &self,
        pane_id: usize,
        start: usize,
        count: usize,
    ) -> Result<Vec<String>> {
        let screen = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.screen)
        };

        let lines = screen.lock().history_text_range(start, count);
        Ok(lines)
    }

    fn scrollback_lines(&self, pane_id: usize) -> Result<Vec<String>> {
        let screen = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.screen)
        };

        let lines = screen
            .lock()
            .history
            .scrollback()
            .iter()
            .map(NextCoreScreen::line_text)
            .collect();
        Ok(lines)
    }

    fn mark_screen_read_for_pane(&self, pane_id: usize, duration: Duration) -> Result<()> {
        let activity = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.activity)
        };
        activity.lock().mark_screen_read(duration);
        Ok(())
    }

    pub fn scroll_viewport_to(&self, pane_id: usize, target: isize) -> Result<()> {
        let started_at = Instant::now();
        let (screen, activity) = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            (Arc::clone(&session.screen), Arc::clone(&session.activity))
        };

        screen.lock().set_viewport_top_near(target);
        activity.lock().mark_viewport_scroll(started_at.elapsed());
        Ok(())
    }

    fn screen_cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        let screen = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.screen)
        };

        let cursor = screen.lock().cursor_snapshot();
        Ok(cursor)
    }

    #[allow(dead_code)]
    fn bracketed_paste_enabled(&self, pane_id: usize) -> Result<bool> {
        let screen = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.screen)
        };

        let enabled = screen.lock().bracketed_paste;
        Ok(enabled)
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

    fn launch_context(
        env: &[(String, String)],
        launch_policy: &LaunchPolicySnapshot,
    ) -> LaunchContextSnapshot {
        let mut proxy_env_keys = env
            .iter()
            .filter_map(|(key, _)| {
                let upper = key.to_ascii_uppercase();
                matches!(
                    upper.as_str(),
                    "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY" | "NO_PROXY"
                )
                .then(|| key.clone())
            })
            .collect::<Vec<_>>();
        proxy_env_keys.sort();
        proxy_env_keys.dedup();

        let mut policy = if launch_policy.env.is_empty()
            && launch_policy.profile.is_none()
            && launch_policy.proxy_env_keys.is_empty()
        {
            Self::infer_launch_policy(env)
        } else {
            launch_policy.clone()
        };
        if policy.profile.is_none() {
            policy.profile = env
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("UNTERM_PROFILE"))
                .map(|(_, value)| value.clone())
                .filter(|value| !value.trim().is_empty());
        }
        if policy.proxy_env_keys.is_empty() {
            policy.proxy_env_keys = proxy_env_keys.clone();
        }
        Self::complete_launch_policy_decisions(&mut policy);

        LaunchContextSnapshot {
            profile: policy.profile.clone().or_else(|| {
                env.iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case("UNTERM_PROFILE"))
                    .map(|(_, value)| value.clone())
                    .filter(|value| !value.trim().is_empty())
            }),
            proxy_env_keys,
            env_key_count: env.len(),
            policy,
        }
    }

    fn complete_launch_policy_decisions(policy: &mut LaunchPolicySnapshot) {
        let default_decision = LaunchPolicyDecisionSnapshot::default();
        if policy.domain == default_decision {
            policy.domain = LaunchPolicyDecisionSnapshot::new(
                LaunchPolicyDecision::NotRequested,
                false,
                "next-core currently launches local-domain sessions only",
            );
        }
        if policy.privilege == default_decision {
            policy.privilege = LaunchPolicyDecisionSnapshot::new(
                LaunchPolicyDecision::NotRequested,
                false,
                "elevation is host-owned and not applied by next-core launch",
            );
        }
        if policy.proxy_rotation == default_decision {
            policy.proxy_rotation = if policy.proxy_env_keys.is_empty() {
                LaunchPolicyDecisionSnapshot::new(
                    LaunchPolicyDecision::NotRequested,
                    false,
                    "no proxy env keys were provided",
                )
            } else {
                LaunchPolicyDecisionSnapshot::new(
                    LaunchPolicyDecision::Deferred,
                    false,
                    "proxy env is applied; proxy rotation remains product-managed",
                )
            };
        }
        if policy.restart == default_decision {
            policy.restart = LaunchPolicyDecisionSnapshot::new(
                LaunchPolicyDecision::NotRequested,
                false,
                "restart policy is not applied during next-core session launch",
            );
        }
    }

    fn infer_launch_policy(env: &[(String, String)]) -> LaunchPolicySnapshot {
        let mut proxy_env_keys = Vec::new();
        let bindings = env
            .iter()
            .map(|(key, _)| {
                let upper = key.to_ascii_uppercase();
                let source = if key.eq_ignore_ascii_case("UNTERM_PROFILE") {
                    LaunchEnvSource::Profile
                } else if matches!(
                    upper.as_str(),
                    "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY" | "NO_PROXY"
                ) {
                    proxy_env_keys.push(key.clone());
                    LaunchEnvSource::Proxy
                } else {
                    LaunchEnvSource::Explicit
                };
                LaunchEnvBinding {
                    key: key.clone(),
                    source,
                }
            })
            .collect();
        proxy_env_keys.sort();
        proxy_env_keys.dedup();
        LaunchPolicySnapshot {
            profile: env
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("UNTERM_PROFILE"))
                .map(|(_, value)| value.clone())
                .filter(|value| !value.trim().is_empty()),
            proxy_env_keys,
            env: bindings,
            ..Default::default()
        }
    }

    fn prepare_command(
        command: Option<portable_pty::CommandBuilder>,
        command_dir: Option<String>,
        env: Vec<(String, String)>,
    ) -> (portable_pty::CommandBuilder, Option<String>) {
        let mut command = command.unwrap_or_else(portable_pty::CommandBuilder::new_default_prog);
        if let Some(command_dir) = command_dir {
            if command.get_cwd().is_none() {
                command.cwd(&command_dir);
            }
        }
        for (key, value) in env {
            command.env(key, value);
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
        launch_env_keys: Vec<String>,
    ) -> Result<NextCoreSession> {
        let label = Self::command_label(&command);
        let pair = native_pty_system().openpty(Self::pty_size(cols, rows))?;
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
        Self::spawn_reader_thread(
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
            shell_type: Self::shell_type(&label),
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
                cursor: Self::default_cursor(),
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
                            let output_started_at = Instant::now();
                            let Some(chunk) = Self::decode_pty_chunk(&mut pending_utf8, &buf[..n])
                            else {
                                continue;
                            };
                            let mut output = output.lock();
                            output.push_str(chunk.as_str());
                            if output.len() > MAX_OUTPUT_BYTES {
                                let keep_from = output.len() - MAX_OUTPUT_BYTES;
                                let keep_from = output
                                    .char_indices()
                                    .map(|(idx, _)| idx)
                                    .find(|idx| *idx >= keep_from)
                                    .unwrap_or(0);
                                output.drain(..keep_from);
                            }
                            let mut screen = screen.lock();
                            screen.feed(chunk.as_str());
                            Self::answer_terminal_queries_with_pending(
                                chunk.as_str(),
                                &screen,
                                &writer,
                                &mut pending_terminal_query,
                            );
                            activity
                                .lock()
                                .mark_output(chunk.len(), output_started_at.elapsed());
                            if let Some(recording) = recording.lock().as_mut() {
                                Self::append_recording_output(recording, chunk.as_str());
                            }
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

    fn decode_pty_chunk(pending: &mut Vec<u8>, bytes: &[u8]) -> Option<String> {
        pending.extend_from_slice(bytes);
        match std::str::from_utf8(pending.as_slice()) {
            Ok(text) => {
                let text = text.to_string();
                pending.clear();
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
            Err(err) if err.error_len().is_none() => {
                let valid_up_to = err.valid_up_to();
                if valid_up_to == 0 {
                    return None;
                }
                let text = String::from_utf8(pending[..valid_up_to].to_vec()).ok()?;
                pending.drain(..valid_up_to);
                Some(text)
            }
            Err(_) => {
                let text = String::from_utf8_lossy(pending.as_slice()).to_string();
                pending.clear();
                Some(text)
            }
        }
    }

    #[cfg(test)]
    fn answer_terminal_queries(
        chunk: &str,
        screen: &NextCoreScreen,
        writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    ) {
        let mut pending = String::new();
        Self::answer_terminal_queries_with_pending(chunk, screen, writer, &mut pending);
    }

    fn answer_terminal_queries_with_pending(
        chunk: &str,
        screen: &NextCoreScreen,
        writer: &Arc<Mutex<Box<dyn Write + Send>>>,
        pending: &mut String,
    ) {
        terminal_queries::answer_with_pending(chunk, screen, writer, pending);
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

    #[allow(dead_code)]
    fn paste_payload(text: &str, bracketed: bool) -> String {
        input_pipeline::paste_payload(text, bracketed)
    }

    fn application_cursor_input(input: &str, enabled: bool) -> String {
        input_pipeline::application_cursor_input(input, enabled)
    }

    fn paste_chunks(text: &str, bracketed: bool) -> Vec<String> {
        input_pipeline::paste_chunks(text, bracketed)
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
        let launch_env_keys = request.env.iter().map(|(key, _)| key.clone()).collect();
        let launch_context = Self::launch_context(&request.env, &request.launch_policy);
        let (command, cwd) =
            Self::prepare_command(request.command, request.command_dir, request.env);
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
            launch_env_keys,
        )?;

        let mut snapshot = session.snapshot.clone();
        snapshot.shell.launch_context = launch_context.clone();
        let mut session = session;
        session.snapshot.shell.launch_context = launch_context;
        let mut state_guard = state().write();
        Self::set_active(&mut state_guard, id);
        state_guard.sessions.push(session);
        state_guard.total_sessions_created = state_guard.total_sessions_created.saturating_add(1);
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
        let launch_env_keys = Vec::new();

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
            launch_env_keys,
        )?;

        let snapshot = session.snapshot.clone();
        let mut state_guard = state().write();
        Self::set_active(&mut state_guard, id);
        state_guard.sessions.push(session);
        state_guard.total_sessions_created = state_guard.total_sessions_created.saturating_add(1);
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
        self.shell_snapshot(pane_id)
    }

    fn activity(&self, pane_id: usize) -> Result<SessionActivitySnapshot> {
        let mut state = state().write();
        let Some(session) = state
            .sessions
            .iter_mut()
            .find(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };
        let dead_reason = Self::refresh_liveness(session);
        let is_dead = session.snapshot.is_dead;
        let process =
            process_tree::snapshot(session.root_pid, &session.snapshot.shell.process_name);
        let foreground_process = process
            .as_ref()
            .map(|process| process.foreground_process.clone())
            .unwrap_or_else(|| session.snapshot.shell.process_name.clone());
        let activity = session.activity.lock();
        let idle = is_dead || activity.is_idle(Instant::now());
        let input = activity.input.clone();
        let output = activity.output.clone();
        let paste = activity.paste.clone();
        let screen = activity.screen.clone();
        drop(activity);
        if let Some(reason) = dead_reason {
            Self::record_dead_reason(&mut state, reason);
        }
        Ok(SessionActivitySnapshot {
            idle,
            foreground_process,
            process,
            input,
            output,
            paste,
            screen,
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
        session.screen.lock().resize(cols, rows);
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
        let previous_dead = session.snapshot.is_dead;
        session.snapshot.is_dead = true;
        let reason = session
            .snapshot
            .dead_reason
            .clone()
            .or_else(|| session.dead_reason.lock().clone())
            .unwrap_or_else(|| "destroyed".to_string());
        session.snapshot.dead_reason = Some(reason.clone());
        *session.dead_reason.lock() = Some(reason.clone());
        session.dead.store(true, Ordering::Release);
        session.child.lock().kill().ok();
        state.total_sessions_destroyed = state.total_sessions_destroyed.saturating_add(1);
        if !previous_dead {
            Self::record_dead_reason(&mut state, reason);
        } else {
            state.last_dead_reason = Some(reason);
        }

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
        let started_at = Instant::now();
        let (screen_handle, activity_handle) = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            (Arc::clone(&session.screen), Arc::clone(&session.activity))
        };

        let snapshot = {
            let screen = screen_handle.lock();
            let visible = screen.snapshot_viewport_lines();
            let first_row = screen.viewport_first_row();
            let cells = visible
                .iter()
                .enumerate()
                .map(|(idx, text)| ScreenLine {
                    row: first_row + idx as i64,
                    text: text.clone(),
                })
                .collect();

            ScreenSnapshot {
                lines: visible,
                cells,
                cursor: screen.cursor_snapshot(),
                cols: screen.cols,
                rows: screen.rows,
                scrollback_rows: screen.scrollback_rows(),
                revision: screen.revision(),
                dirty_rows: screen.dirty_rows(),
            }
        };
        activity_handle
            .lock()
            .mark_screen_read(started_at.elapsed());
        Ok(snapshot)
    }

    fn read_styled_screen(&self, pane_id: usize) -> Result<StyledScreenSnapshot> {
        let started_at = Instant::now();
        let (screen_handle, activity_handle) = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            (Arc::clone(&session.screen), Arc::clone(&session.activity))
        };

        let snapshot = {
            let screen = screen_handle.lock();
            let first_row = screen.viewport_first_row();
            StyledScreenSnapshot {
                lines: screen.styled_viewport_lines(first_row),
                cursor: screen.cursor_snapshot(),
                cols: screen.cols,
                rows: screen.rows,
                scrollback_rows: screen.scrollback_rows(),
                revision: screen.revision(),
                dirty_rows: screen.dirty_rows(),
            }
        };
        activity_handle
            .lock()
            .mark_screen_read(started_at.elapsed());
        Ok(snapshot)
    }

    fn read_render_frame(
        &self,
        pane_id: usize,
        since_revision: Option<u64>,
    ) -> Result<RenderFrameSnapshot> {
        let started_at = Instant::now();
        let (screen_handle, activity_handle) = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            (Arc::clone(&session.screen), Arc::clone(&session.activity))
        };

        let snapshot = {
            let mut screen = screen_handle.lock();
            let first_row = screen.viewport_first_row();
            let revision = screen.revision();
            let all_rows = if screen.rows == 0 {
                None
            } else {
                Some(DirtyRows {
                    start: 0,
                    end: screen.rows - 1,
                })
            };

            if since_revision == Some(revision) {
                RenderFrameSnapshot {
                    lines: Vec::new(),
                    cursor: screen.cursor_snapshot(),
                    cols: screen.cols,
                    rows: screen.rows,
                    scrollback_rows: screen.scrollback_rows(),
                    revision,
                    dirty_rows: None,
                    full: false,
                }
            } else {
                let can_delta = since_revision
                    .filter(|since| *since <= revision)
                    .is_some_and(|since| screen.can_render_delta_since(since));
                let force_full =
                    since_revision.is_none() || !can_delta || screen.dirty_rows().is_none();
                let dirty_rows = if force_full {
                    all_rows
                } else if screen.history.viewport_is_pinned() && screen.dirty_rows() != all_rows {
                    None
                } else {
                    screen.dirty_rows()
                };
                let full = dirty_rows.is_some() && dirty_rows == all_rows;
                let lines = match dirty_rows {
                    Some(rows) if full => screen.styled_viewport_lines(first_row),
                    Some(rows) => screen.styled_viewport_dirty_lines(rows, first_row),
                    None => Vec::new(),
                };

                let snapshot = RenderFrameSnapshot {
                    lines,
                    cursor: screen.cursor_snapshot(),
                    cols: screen.cols,
                    rows: screen.rows,
                    scrollback_rows: screen.scrollback_rows(),
                    revision,
                    dirty_rows,
                    full,
                };
                if snapshot.full || snapshot.dirty_rows.is_some() {
                    screen.clear_dirty_rows();
                }
                snapshot
            }
        };
        activity_handle
            .lock()
            .mark_screen_read(started_at.elapsed());
        Ok(snapshot)
    }

    fn read_visible_text(&self, pane_id: usize) -> Result<String> {
        Ok(self.read_screen(pane_id)?.lines.join("\n"))
    }

    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
        let started_at = Instant::now();
        let start = start.max(0) as usize;
        let lines = self
            .screen_line_text_range(pane_id, start, count)?
            .into_iter()
            .enumerate()
            .map(|(idx, text)| ScreenLine {
                row: (start + idx) as i64,
                text,
            })
            .collect();
        self.mark_screen_read_for_pane(pane_id, started_at.elapsed())?;
        Ok(lines)
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>> {
        let started_at = Instant::now();
        let lines = self
            .scrollback_lines(pane_id)?
            .into_iter()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        let lines = Self::tail_lines(&lines, limit);
        self.mark_screen_read_for_pane(pane_id, started_at.elapsed())?;
        Ok(lines)
    }

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot> {
        let started_at = Instant::now();
        let session = self.session(pane_id)?;
        let raw_lines;
        let line_count;
        if request.escapes {
            raw_lines = Some(Self::output_lines(&self.output(pane_id)?));
            line_count = raw_lines.as_ref().map_or(0, Vec::len);
        } else {
            raw_lines = None;
            line_count = self.screen_line_count(pane_id)?;
        }
        let end = request
            .end_line
            .map(|end| end.max(0) as usize)
            .unwrap_or(line_count)
            .min(line_count);
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

        let selected = if let Some(lines) = raw_lines {
            lines[start..end].to_vec()
        } else {
            self.screen_line_text_range(pane_id, start, end.saturating_sub(start))?
        };
        let snapshot = ScrollbackTextSnapshot {
            text: selected.join("\n"),
            lines: selected,
            first_row: start as i64,
            row_count: end.saturating_sub(start) as i64,
            cols: session.cols,
            escapes: request.escapes,
            scrollback_top: 0,
            physical_top: line_count.saturating_sub(session.rows) as i64,
            viewport_rows: session.rows,
        };
        self.mark_screen_read_for_pane(pane_id, started_at.elapsed())?;
        Ok(snapshot)
    }

    fn read_styled_scrollback(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<StyledScrollbackSnapshot> {
        if request.escapes {
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
            return Ok(StyledScrollbackSnapshot {
                lines,
                first_row: text.first_row,
                row_count: text.row_count,
                cols: text.cols,
                scrollback_top: text.scrollback_top,
                physical_top: text.physical_top,
                viewport_rows: text.viewport_rows,
            });
        }

        let started_at = Instant::now();
        let session = self.session(pane_id)?;
        let screen = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.screen)
        };

        let screen = screen.lock();
        let line_count = screen.history_len();
        let end = request
            .end_line
            .map(|end| end.max(0) as usize)
            .unwrap_or(line_count)
            .min(line_count);
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
        let count = end.saturating_sub(start);
        let snapshot = StyledScrollbackSnapshot {
            lines: screen.styled_history_range(start, count),
            first_row: start as i64,
            row_count: count as i64,
            cols: session.cols,
            scrollback_top: 0,
            physical_top: line_count.saturating_sub(session.rows) as i64,
            viewport_rows: session.rows,
        };
        drop(screen);
        self.mark_screen_read_for_pane(pane_id, started_at.elapsed())?;
        Ok(snapshot)
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        let started_at = Instant::now();
        if pattern.is_empty() || max_results == 0 {
            return Ok(Vec::new());
        }
        let lines = self.screen_lines(pane_id)?;
        let mut matches = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            for (byte_col, _) in line.match_indices(pattern) {
                matches.push(ScreenSearchMatch {
                    row: row as i64,
                    col: line[..byte_col].chars().count(),
                    text: line.clone(),
                });
                if matches.len() >= max_results {
                    self.mark_screen_read_for_pane(pane_id, started_at.elapsed())?;
                    return Ok(matches);
                }
            }
        }
        self.mark_screen_read_for_pane(pane_id, started_at.elapsed())?;
        Ok(matches)
    }

    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        self.screen_cursor(pane_id)
    }
}

impl InputEngine for NextCoreEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()> {
        let (writer, activity, application_cursor_keys) = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            let application_cursor_keys = session.screen.lock().application_cursor_keys;
            (
                Arc::clone(&session.writer),
                Arc::clone(&session.activity),
                application_cursor_keys,
            )
        };

        let started_at = Instant::now();
        let input = Self::application_cursor_input(input, application_cursor_keys);
        let bytes = input.len();
        let mut writer = writer.lock();
        writer.write_all(input.as_bytes())?;
        writer.flush()?;
        if !input.is_empty() {
            activity.lock().mark_input(bytes, started_at.elapsed());
        }
        Ok(())
    }

    fn paste_input(&self, pane_id: usize, text: &str) -> Result<()> {
        let bracketed = self.bracketed_paste_enabled(pane_id)?;
        let chunks = Self::paste_chunks(text, bracketed);
        let wire_bytes = chunks.iter().map(|chunk| chunk.len()).sum::<usize>();
        let chunk_count = chunks.len();
        let started_at = Instant::now();
        let (writer, activity) = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            (Arc::clone(&session.writer), Arc::clone(&session.activity))
        };

        {
            let mut writer = writer.lock();
            for chunk in &chunks {
                writer.write_all(chunk.as_bytes())?;
            }
            writer.flush()?;
        }

        if !text.is_empty() || bracketed {
            let mut activity = activity.lock();
            activity.mark_input(wire_bytes, started_at.elapsed());
            activity.mark_paste(
                text.len(),
                wire_bytes,
                chunk_count,
                bracketed,
                started_at.elapsed(),
            );
        }
        Ok(())
    }
}

impl RecordingEngine for NextCoreEngine {
    fn start_recording(&self, pane_id: usize) -> Result<RecordingStartResult> {
        let recording_handle;
        let project_path;
        {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            recording_handle = Arc::clone(&session.recording);
            project_path = session.snapshot.shell.cwd.clone();
        }

        let mut slot = recording_handle.lock();
        if slot.is_some() {
            bail!("Recording already active for pane {pane_id}");
        }

        let project_slug = Self::project_slug(project_path.as_deref());
        let (log_path, md_path) =
            Self::recording_paths(pane_id, project_path.as_deref(), &project_slug);
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        File::create(&log_path)?;

        let started_at = Self::timestamp_string();
        let session_id = format!("next-core-{pane_id}-{started_at}");
        let recording = NextCoreRecording {
            session_id: session_id.clone(),
            pane_id,
            project_path,
            project_slug,
            started_at,
            log_path: log_path.clone(),
            md_path: md_path.clone(),
            bytes_raw: 0,
            block_count: 0,
            trace_ids: Vec::new(),
            text_preview: String::new(),
            blocks: Vec::new(),
            osc133_seen: false,
            command_blocks: Vec::new(),
            active_command: None,
        };
        Self::upsert_recording_index(&recording, None)?;
        *slot = Some(recording);

        Ok(RecordingStartResult {
            session_id,
            log_path: log_path.display().to_string(),
            md_path: md_path.display().to_string(),
        })
    }

    fn stop_recording(&self, pane_id: usize) -> Result<RecordingStopResult> {
        let recording_handle = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.recording)
        };
        let mut slot = recording_handle.lock();
        let Some(recording) = slot.take() else {
            bail!("No active recording for pane {pane_id}");
        };
        drop(slot);

        let ended_at = Self::timestamp_string();
        Self::write_recording_markdown(&recording, Some(&ended_at), "recording_stopped")?;
        Self::upsert_recording_index(&recording, Some(ended_at.clone()))?;

        Ok(RecordingStopResult {
            session_id: recording.session_id,
            ended_at,
            block_count: recording.block_count,
            exit_reason: "recording_stopped".to_string(),
            md_path: recording.md_path.display().to_string(),
        })
    }

    fn recording_status(&self, pane_id: usize) -> Result<RecordingStatusSnapshot> {
        let recording_handle = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                return Ok(RecordingStatusSnapshot {
                    enabled: false,
                    session_id: None,
                    started_at: None,
                    block_count: None,
                    bytes: None,
                });
            };
            Arc::clone(&session.recording)
        };
        let slot = recording_handle.lock();
        if let Some(recording) = slot.as_ref() {
            Ok(RecordingStatusSnapshot {
                enabled: true,
                session_id: Some(recording.session_id.clone()),
                started_at: Some(recording.started_at.clone()),
                block_count: Some(recording.block_count),
                bytes: Some(recording.bytes_raw),
            })
        } else {
            Ok(RecordingStatusSnapshot {
                enabled: false,
                session_id: None,
                started_at: None,
                block_count: None,
                bytes: None,
            })
        }
    }

    fn attach_recording_trace(&self, pane_id: usize, trace_id: String) -> Result<Vec<String>> {
        let recording_handle = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.recording)
        };
        let mut slot = recording_handle.lock();
        let Some(recording) = slot.as_mut() else {
            bail!("No active recording for pane {pane_id}");
        };
        if !recording
            .trace_ids
            .iter()
            .any(|existing| existing == &trace_id)
        {
            recording.trace_ids.push(trace_id);
        }
        Self::upsert_recording_index(recording, None)?;
        Ok(recording.trace_ids.clone())
    }

    fn export_markdown(
        &self,
        pane_id: usize,
        target_path: Option<String>,
    ) -> Result<RecordingExportResult> {
        let recording_handle = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.recording)
        };
        let slot = recording_handle.lock();
        let Some(recording) = slot.as_ref() else {
            bail!("No active recording for pane {pane_id}");
        };
        let mut export = recording.clone();
        drop(slot);

        if let Some(target_path) = target_path {
            export.md_path = PathBuf::from(target_path);
        }
        let bytes = Self::write_recording_markdown(&export, None, "recording_exported")?;

        Ok(RecordingExportResult {
            session_id: export.session_id,
            path: export.md_path.display().to_string(),
            bytes,
            block_count: export.block_count,
        })
    }
}

impl HealthEngine for NextCoreEngine {
    fn health(&self) -> Result<EngineHealthSnapshot> {
        let mut state = state().write();
        let pane_count = state.sessions.len();
        let mut io = EngineIoHealthSnapshot {
            input_writes: 0,
            input_bytes: 0,
            output_chunks: 0,
            output_bytes: 0,
            paste_count: 0,
            paste_text_bytes: 0,
            screen_reads: 0,
            viewport_scrolls: 0,
        };
        let mut dead_reasons = Vec::new();
        let mut dead_sessions = 0u64;
        for session in &mut state.sessions {
            if let Some(reason) = Self::refresh_liveness(session) {
                dead_reasons.push(reason);
            }
            if session.snapshot.is_dead {
                dead_sessions = dead_sessions.saturating_add(1);
            }
            let activity = session.activity.lock();
            if let Some(input) = &activity.input {
                io.input_writes = io.input_writes.saturating_add(input.total_writes);
                io.input_bytes = io.input_bytes.saturating_add(input.total_bytes);
            }
            if let Some(output) = &activity.output {
                io.output_chunks = io.output_chunks.saturating_add(output.total_chunks);
                io.output_bytes = io.output_bytes.saturating_add(output.total_bytes);
            }
            if let Some(paste) = &activity.paste {
                io.paste_count = io.paste_count.saturating_add(paste.total_pastes);
                io.paste_text_bytes = io.paste_text_bytes.saturating_add(paste.total_text_bytes);
            }
            if let Some(screen) = &activity.screen {
                io.screen_reads = io.screen_reads.saturating_add(screen.total_reads);
                io.viewport_scrolls = io
                    .viewport_scrolls
                    .saturating_add(screen.total_viewport_scrolls);
            }
        }
        for reason in dead_reasons {
            Self::record_dead_reason(&mut state, reason);
        }
        let lifecycle = EngineLifecycleHealthSnapshot {
            live_sessions: pane_count.saturating_sub(dead_sessions as usize) as u64,
            dead_sessions,
            total_created: state.total_sessions_created,
            total_destroyed: state.total_sessions_destroyed,
            total_marked_dead: state.total_sessions_marked_dead,
            last_dead_reason: state.last_dead_reason.clone(),
        };
        Ok(EngineHealthSnapshot {
            engine: "next-core".to_string(),
            ready: true,
            status: "ok".to_string(),
            detail: "next-core session registry is available".to_string(),
            pane_count: Some(pane_count),
            io: Some(io),
            lifecycle: Some(lifecycle),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StyledColor;
    use parking_lot::MutexGuard;

    struct SharedWriter {
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.bytes.lock().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn test_guard() -> MutexGuard<'static, ()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(())).lock()
    }

    fn quiet_wait_command_for_test() -> portable_pty::CommandBuilder {
        #[cfg(windows)]
        {
            let mut command = portable_pty::CommandBuilder::new("cmd.exe");
            command.args(["/c", "ping -n 5 127.0.0.1 >nul"]);
            command
        }
        #[cfg(not(windows))]
        {
            let mut command = portable_pty::CommandBuilder::new("sh");
            command.args(["-c", "sleep 5"]);
            command
        }
    }

    #[test]
    fn answers_cursor_position_queries_from_screen_state() {
        let _guard = test_guard();
        let mut screen = NextCoreScreen::new(80, 10);
        screen.set_cursor(2, 4);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[6n", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[3;5R");
    }

    #[test]
    fn answers_dec_private_cursor_position_queries_from_screen_state() {
        let _guard = test_guard();
        let mut screen = NextCoreScreen::new(80, 10);
        screen.set_cursor(2, 4);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[?6n", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[?3;5R");
    }

    #[test]
    fn answers_terminal_status_queries() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(80, 10);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[5n", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[0n");
    }

    #[test]
    fn answers_text_area_size_queries_from_screen_dimensions() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(132, 43);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[18t", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[8;43;132t");
    }

    #[test]
    fn answers_window_pixel_size_queries_from_headless_cell_dimensions() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(132, 43);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[14t", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[4;688;1056t");
    }

    #[test]
    fn answers_mode_report_queries_from_screen_state() {
        let _guard = test_guard();
        let mut screen = NextCoreScreen::new(80, 10);
        screen.feed(
            "\x1b[?1047h\x1b[?3h\x1b[?1h\x1b[?5h\x1b[?6h\x1b[?12l\x1b[?25l\x1b[?66h\x1b[?69h\x1b[?1002h\x1b[?1004h\x1b[?1005h\x1b[?1006h\x1b[?1007h\x1b[?1015h\x1b[?1016h\x1b[?1034h\x1b[?2004h\x1b[?2026h\x1b[4h",
        );
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries(
            "\x1b[?1$p\x1b[?3$p\x1b[?5$p\x1b[?6$p\x1b[?7$p\x1b[?12$p\x1b[?25$p\x1b[?66$p\x1b[?69$p\x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1004$p\x1b[?1005$p\x1b[?1006$p\x1b[?1007$p\x1b[?1015$p\x1b[?1016$p\x1b[?1034$p\x1b[?47$p\x1b[?1047$p\x1b[?1049$p\x1b[?2004$p\x1b[?2026$p\x1b[4$p",
            &screen,
            &writer,
        );

        assert_eq!(
            bytes.lock().as_slice(),
            b"\x1b[?1;1$y\x1b[?3;1$y\x1b[?5;1$y\x1b[?6;1$y\x1b[?7;1$y\x1b[?12;2$y\x1b[?25;2$y\x1b[?66;1$y\x1b[?69;1$y\x1b[?1000;2$y\x1b[?1002;1$y\x1b[?1003;2$y\x1b[?1004;1$y\x1b[?1005;1$y\x1b[?1006;1$y\x1b[?1007;1$y\x1b[?1015;1$y\x1b[?1016;1$y\x1b[?1034;1$y\x1b[?47;2$y\x1b[?1047;1$y\x1b[?1049;2$y\x1b[?2004;1$y\x1b[?2026;1$y\x1b[4;1$y"
        );
    }

    #[test]
    fn answers_reset_mode_report_queries_from_default_screen_state() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(80, 10);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries(
            "\x1b[?1$p\x1b[?3$p\x1b[?5$p\x1b[?6$p\x1b[?12$p\x1b[?66$p\x1b[?69$p\x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1004$p\x1b[?1005$p\x1b[?1006$p\x1b[?1007$p\x1b[?1015$p\x1b[?1016$p\x1b[?1034$p\x1b[?47$p\x1b[?1047$p\x1b[?1049$p\x1b[?2004$p\x1b[?2026$p\x1b[4$p",
            &screen,
            &writer,
        );

        assert_eq!(
            bytes.lock().as_slice(),
            b"\x1b[?1;2$y\x1b[?3;2$y\x1b[?5;2$y\x1b[?6;2$y\x1b[?12;1$y\x1b[?66;2$y\x1b[?69;2$y\x1b[?1000;2$y\x1b[?1002;2$y\x1b[?1003;2$y\x1b[?1004;2$y\x1b[?1005;2$y\x1b[?1006;2$y\x1b[?1007;2$y\x1b[?1015;2$y\x1b[?1016;2$y\x1b[?1034;2$y\x1b[?47;2$y\x1b[?1047;2$y\x1b[?1049;2$y\x1b[?2004;2$y\x1b[?2026;2$y\x1b[4;2$y"
        );
    }

    #[test]
    fn answers_alternate_screen_mode_reports_independently() {
        let _guard = test_guard();
        let mut screen = NextCoreScreen::new(80, 10);
        screen.feed("\x1b[?47h\x1b[?1049h");
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries(
            "\x1b[?47$p\x1b[?1047$p\x1b[?1049$p",
            &screen,
            &writer,
        );

        assert_eq!(
            bytes.lock().as_slice(),
            b"\x1b[?47;1$y\x1b[?1047;2$y\x1b[?1049;1$y"
        );
    }

    #[test]
    fn answers_multiple_terminal_queries_in_chunk() {
        let _guard = test_guard();
        let mut screen = NextCoreScreen::new(80, 10);
        screen.feed("\x1b[?1h");
        screen.set_cursor(2, 4);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries(
            "\x1b[5n\x1b[?1$p\x1b[?6n\x1b[14t\x1b[18t\x1b[6n\x1b[>c\x1b[c",
            &screen,
            &writer,
        );

        assert_eq!(
            bytes.lock().as_slice(),
            b"\x1b[0n\x1b[?1;1$y\x1b[?3;5R\x1b[4;160;640t\x1b[8;10;80t\x1b[3;5R\x1b[>0;0;0c\x1b[?64;1;2;6;9;15;18;21;22c"
        );
    }

    #[test]
    fn answers_terminal_queries_in_input_order() {
        let _guard = test_guard();
        let mut screen = NextCoreScreen::new(80, 10);
        screen.set_cursor(2, 4);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("x\x1b[c\x1b[6ny\x1b[5n\x1b[>0c", &screen, &writer);

        assert_eq!(
            bytes.lock().as_slice(),
            b"\x1b[?64;1;2;6;9;15;18;21;22c\x1b[3;5R\x1b[0n\x1b[>0;0;0c"
        );
    }

    #[test]
    fn answers_terminal_queries_across_split_chunks() {
        let _guard = test_guard();
        let mut screen = NextCoreScreen::new(80, 10);
        screen.feed("\x1b[?1h\x1b[4h");
        screen.set_cursor(2, 4);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));
        let mut pending = String::new();

        NextCoreEngine::answer_terminal_queries_with_pending(
            "x\x1b[?",
            &screen,
            &writer,
            &mut pending,
        );
        assert_eq!(bytes.lock().as_slice(), b"");
        assert_eq!(pending, "\x1b[?");

        NextCoreEngine::answer_terminal_queries_with_pending(
            "1$p y\x1b[",
            &screen,
            &writer,
            &mut pending,
        );
        NextCoreEngine::answer_terminal_queries_with_pending("6", &screen, &writer, &mut pending);
        NextCoreEngine::answer_terminal_queries_with_pending(
            "n\x1b[>0",
            &screen,
            &writer,
            &mut pending,
        );
        NextCoreEngine::answer_terminal_queries_with_pending(
            "c\x1b[4",
            &screen,
            &writer,
            &mut pending,
        );
        NextCoreEngine::answer_terminal_queries_with_pending("$p", &screen, &writer, &mut pending);

        assert_eq!(
            bytes.lock().as_slice(),
            b"\x1b[?1;1$y\x1b[3;5R\x1b[>0;0;0c\x1b[4;1$y"
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn terminal_query_pending_buffer_drops_overlong_incomplete_sequences() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(80, 10);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));
        let mut pending = String::new();

        NextCoreEngine::answer_terminal_queries_with_pending(
            &format!(
                "\x1b[{}",
                "1".repeat(terminal_queries::MAX_PENDING_TERMINAL_QUERY_BYTES + 1)
            ),
            &screen,
            &writer,
            &mut pending,
        );

        assert!(pending.is_empty());
        assert!(bytes.lock().is_empty());
    }

    #[test]
    fn answers_primary_device_attributes_with_xterm_capabilities() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(80, 10);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[c", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[?64;1;2;6;9;15;18;21;22c");
    }

    #[test]
    fn answers_parameterized_primary_device_attributes() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(80, 10);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[0c", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[?64;1;2;6;9;15;18;21;22c");
    }

    #[test]
    fn answers_secondary_device_attributes() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(80, 10);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[>c", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[>0;0;0c");
    }

    #[test]
    fn answers_parameterized_secondary_device_attributes_without_primary() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(80, 10);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[>0c", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[>0;0;0c");
    }

    #[test]
    fn decodes_pty_utf8_across_chunk_boundaries() {
        let _guard = test_guard();
        let mut pending = Vec::new();
        let bytes = "你A".as_bytes();

        assert_eq!(
            NextCoreEngine::decode_pty_chunk(&mut pending, &bytes[..1]),
            None
        );
        assert_eq!(
            NextCoreEngine::decode_pty_chunk(&mut pending, &bytes[1..3]),
            Some("你".to_string())
        );
        assert_eq!(
            NextCoreEngine::decode_pty_chunk(&mut pending, &bytes[3..]),
            Some("A".to_string())
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn wraps_paste_payload_when_bracketed_paste_is_enabled() {
        assert_eq!(NextCoreEngine::paste_payload("plain", false), "plain");
        assert_eq!(
            NextCoreEngine::paste_payload("line1\nline2", true),
            "\x1b[200~line1\nline2\x1b[201~"
        );
    }

    #[test]
    fn chunks_paste_payload_without_splitting_utf8() {
        let text = format!(
            "{}{}",
            "a".repeat(input_pipeline::PASTE_CHUNK_BYTES),
            "你".repeat(3)
        );
        let chunks = NextCoreEngine::paste_chunks(&text, false);

        assert!(chunks.len() > 1);
        assert_eq!(chunks.concat(), text);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.is_char_boundary(chunk.len())));
    }

    #[test]
    fn chunks_bracketed_paste_with_intact_markers() {
        let text = "x".repeat(input_pipeline::PASTE_CHUNK_BYTES + 10);
        let chunks = NextCoreEngine::paste_chunks(&text, true);

        assert_eq!(chunks.first().map(String::as_str), Some("\x1b[200~"));
        assert_eq!(chunks.last().map(String::as_str), Some("\x1b[201~"));
        assert_eq!(chunks[1..chunks.len() - 1].concat(), text);
    }

    #[test]
    fn translates_arrow_keys_in_application_cursor_mode() {
        let _guard = test_guard();

        assert_eq!(
            NextCoreEngine::application_cursor_input("\x1b[A\x1b[B\x1b[C\x1b[D", true),
            "\x1bOA\x1bOB\x1bOC\x1bOD"
        );
        assert_eq!(
            NextCoreEngine::application_cursor_input("x\x1b[C你", true),
            "x\x1bOC你"
        );
        assert_eq!(
            NextCoreEngine::application_cursor_input("\x1b[C", false),
            "\x1b[C"
        );
    }

    #[test]
    fn translates_home_end_keys_in_application_cursor_mode() {
        let _guard = test_guard();

        assert_eq!(
            NextCoreEngine::application_cursor_input("\x1b[H\x1b[F\x1b[1~\x1b[4~", true),
            "\x1bOH\x1bOF\x1bOH\x1bOF"
        );
        assert_eq!(
            NextCoreEngine::application_cursor_input("x\x1b[H你\x1b[F", true),
            "x\x1bOH你\x1bOF"
        );
        assert_eq!(
            NextCoreEngine::application_cursor_input("\x1b[H\x1b[F", false),
            "\x1b[H\x1b[F"
        );
    }

    #[test]
    fn paste_activity_reports_last_write_metrics() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: Some(quiet_wait_command_for_test()),
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        let text = format!(
            "{}{}",
            "a".repeat(input_pipeline::PASTE_CHUNK_BYTES + 1),
            "你"
        );
        engine.paste_input(session.id, &text)?;
        let paste = engine
            .activity(session.id)?
            .paste
            .expect("paste metrics after paste");

        assert_eq!(paste.total_pastes, 1);
        assert_eq!(paste.total_text_bytes, text.len() as u64);
        assert!(paste.total_chunks > 1);
        assert_eq!(paste.last_text_bytes, text.len());
        assert_eq!(paste.last_wire_bytes, text.len());
        assert_eq!(paste.last_chunk_count as u64, paste.total_chunks);
        assert!(!paste.last_bracketed);

        set_output_for_test(session.id, "\x1b[?2004h")?;
        engine.paste_input(session.id, "token")?;
        let paste = engine
            .activity(session.id)?
            .paste
            .expect("paste metrics after bracketed paste");

        assert_eq!(paste.total_pastes, 2);
        assert_eq!(paste.last_text_bytes, 5);
        assert_eq!(paste.last_wire_bytes, "\x1b[200~token\x1b[201~".len());
        assert_eq!(paste.last_chunk_count, 3);
        assert!(paste.last_bracketed);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn input_activity_reports_regular_writes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        engine.write_input(session.id, "abc")?;
        engine.write_input(session.id, "你")?;
        let activity = engine.activity(session.id)?;
        let input = activity.input.expect("input metrics after writes");

        assert_eq!(input.total_writes, 2);
        assert_eq!(input.total_bytes, "abc你".len() as u64);
        assert_eq!(input.last_bytes, "你".len());
        assert!(activity.paste.is_none());

        engine.paste_input(session.id, "token")?;
        let activity = engine.activity(session.id)?;
        let input = activity.input.expect("input metrics after paste");
        let paste = activity.paste.expect("paste metrics after paste");

        assert_eq!(input.total_writes, 3);
        assert_eq!(input.last_bytes, paste.last_wire_bytes);
        assert_eq!(paste.total_pastes, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn shell_uses_process_cwd_fallback_until_osc7_updates() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: Some(quiet_wait_command_for_test()),
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        let process_cwd = engine
            .activity(session.id)?
            .process
            .and_then(|process| process.foreground_cwd.or(process.root_cwd));
        let shell_cwd = engine.shell(session.id)?.cwd;
        assert_eq!(shell_cwd, process_cwd);
        assert_eq!(engine.get_session(session.id)?.shell.cwd, shell_cwd);

        set_output_for_test(
            session.id,
            "\x1b]7;file://localhost/C:/Users/alex/osc-project\x07",
        )?;
        assert_eq!(
            engine.shell(session.id)?.cwd.as_deref(),
            Some("C:\\Users\\alex\\osc-project")
        );
        assert_eq!(
            engine.get_session(session.id)?.shell.cwd.as_deref(),
            Some("C:\\Users\\alex\\osc-project")
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn output_activity_reports_screen_updates() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: Some(quiet_wait_command_for_test()),
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        reset_activity_for_test(session.id)?;
        let before = engine.activity(session.id)?.output;
        let before_chunks = before.as_ref().map_or(0, |output| output.total_chunks);
        let before_bytes = before.as_ref().map_or(0, |output| output.total_bytes);

        set_output_for_test(session.id, "first")?;
        let activity = engine.activity(session.id)?;
        assert!(!activity.idle);
        let output = activity.output.expect("output metrics after screen update");
        assert!(output.total_chunks >= before_chunks + 1);
        assert!(output.total_bytes >= before_bytes + 5);
        assert_eq!(output.last_bytes, 5);

        set_output_for_test(session.id, "second line")?;
        let output = engine
            .activity(session.id)?
            .output
            .expect("output metrics after second screen update");
        assert!(output.total_chunks >= before_chunks + 2);
        assert!(output.total_bytes >= before_bytes + 16);
        assert!(output.last_bytes > 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn health_reports_aggregate_io_metrics() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        engine.write_input(session.id, "abc")?;
        engine.paste_input(session.id, "token")?;
        set_output_for_test(session.id, "one\ntwo\nthree\nfour")?;
        let _ = engine.read_screen(session.id)?;
        engine.scroll_viewport_to(session.id, 1)?;

        let health = engine.health()?;
        let io = health.io.expect("next-core io health");
        assert_eq!(health.pane_count, Some(1));
        assert_eq!(io.input_writes, 2);
        assert_eq!(io.input_bytes, 8);
        assert_eq!(io.paste_count, 1);
        assert_eq!(io.paste_text_bytes, 5);
        assert_eq!(io.output_chunks, 1);
        assert_eq!(io.output_bytes, 18);
        assert_eq!(io.screen_reads, 1);
        assert_eq!(io.viewport_scrolls, 1);
        let lifecycle = health.lifecycle.expect("next-core lifecycle health");
        assert_eq!(lifecycle.live_sessions, 1);
        assert_eq!(lifecycle.dead_sessions, 0);
        assert_eq!(lifecycle.total_created, 1);
        assert_eq!(lifecycle.total_destroyed, 0);
        assert_eq!(lifecycle.total_marked_dead, 0);

        engine.destroy_session(session.id)?;
        let lifecycle = engine
            .health()?
            .lifecycle
            .expect("next-core lifecycle health");
        assert_eq!(lifecycle.live_sessions, 0);
        assert_eq!(lifecycle.dead_sessions, 0);
        assert_eq!(lifecycle.total_created, 1);
        assert_eq!(lifecycle.total_destroyed, 1);
        assert_eq!(lifecycle.total_marked_dead, 1);
        assert_eq!(lifecycle.last_dead_reason.as_deref(), Some("destroyed"));
        Ok(())
    }

    #[test]
    fn prepare_command_applies_launch_env_overlay() {
        let expected_cwd = std::env::current_dir()
            .expect("current dir")
            .display()
            .to_string();
        let (command, cwd) = NextCoreEngine::prepare_command(
            None,
            Some(expected_cwd.clone()),
            vec![
                ("UNTERM_PROFILE".to_string(), "work-acme".to_string()),
                (
                    "HTTPS_PROXY".to_string(),
                    "http://127.0.0.1:7890".to_string(),
                ),
            ],
        );

        assert!(command.is_default_prog());
        assert_eq!(cwd.as_deref(), Some(expected_cwd.as_str()));
        assert_eq!(
            command
                .get_env("UNTERM_PROFILE")
                .and_then(|value| value.to_str()),
            Some("work-acme")
        );
        assert_eq!(
            command
                .get_env("HTTPS_PROXY")
                .and_then(|value| value.to_str()),
            Some("http://127.0.0.1:7890")
        );
    }

    #[test]
    fn launch_context_summarizes_profile_and_proxy_env_without_values() {
        let env = [
            ("GITHUB_TOKEN".to_string(), "secret-token".to_string()),
            ("UNTERM_PROFILE".to_string(), "work-acme".to_string()),
            (
                "HTTPS_PROXY".to_string(),
                "http://127.0.0.1:7890".to_string(),
            ),
            ("NO_PROXY".to_string(), "localhost".to_string()),
        ];
        let context = NextCoreEngine::launch_context(&env, &Default::default());

        assert_eq!(context.profile.as_deref(), Some("work-acme"));
        assert_eq!(context.proxy_env_keys, vec!["HTTPS_PROXY", "NO_PROXY"]);
        assert_eq!(context.env_key_count, 4);
        assert_eq!(context.policy.profile.as_deref(), Some("work-acme"));
        assert_eq!(
            context.policy.domain.decision,
            LaunchPolicyDecision::NotRequested
        );
        assert_eq!(context.policy.domain.supported, false);
        assert_eq!(
            context.policy.privilege.decision,
            LaunchPolicyDecision::NotRequested
        );
        assert_eq!(
            context.policy.proxy_rotation.decision,
            LaunchPolicyDecision::Deferred
        );
        assert_eq!(context.policy.proxy_rotation.supported, false);
        assert_eq!(
            context.policy.restart.decision,
            LaunchPolicyDecision::NotRequested
        );
        assert_eq!(
            context
                .policy
                .env
                .iter()
                .map(|binding| (binding.key.as_str(), binding.source))
                .collect::<Vec<_>>(),
            vec![
                ("GITHUB_TOKEN", LaunchEnvSource::Explicit),
                ("UNTERM_PROFILE", LaunchEnvSource::Profile),
                ("HTTPS_PROXY", LaunchEnvSource::Proxy),
                ("NO_PROXY", LaunchEnvSource::Proxy)
            ]
        );
    }

    #[test]
    fn session_snapshot_records_launch_env_keys_without_values() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: vec![
                ("UNTERM_PROFILE".to_string(), "work-acme".to_string()),
                ("GITHUB_TOKEN".to_string(), "secret-token".to_string()),
                (
                    "HTTPS_PROXY".to_string(),
                    "http://127.0.0.1:7890".to_string(),
                ),
            ],
            launch_policy: Default::default(),
        })?;

        let mut keys = engine.shell(session.id)?.launch_env_keys;
        keys.sort();
        assert_eq!(keys, vec!["GITHUB_TOKEN", "HTTPS_PROXY", "UNTERM_PROFILE"]);
        let launch_context = engine.shell(session.id)?.launch_context;
        assert_eq!(launch_context.profile.as_deref(), Some("work-acme"));
        assert_eq!(launch_context.proxy_env_keys, vec!["HTTPS_PROXY"]);
        assert_eq!(launch_context.env_key_count, 3);

        engine.destroy_session(session.id)?;
        Ok(())
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
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        assert_eq!(first.id, 1);
        assert!(first.is_active);

        let second = engine.split_session(SplitSessionRequest {
            source_pane_id: first.id,
            direction: crate::SplitDirection::Right,
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
    fn propagates_reader_dead_marker_to_session_snapshots() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: Some(quiet_wait_command_for_test()),
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        mark_dead_for_test(session.id)?;
        let snapshot = engine.get_session(session.id)?;
        assert!(snapshot.is_dead);
        assert_eq!(snapshot.dead_reason.as_deref(), Some("test_dead_marker"));
        let lifecycle = engine
            .health()?
            .lifecycle
            .expect("next-core lifecycle health");
        assert_eq!(lifecycle.live_sessions, 0);
        assert_eq!(lifecycle.dead_sessions, 1);
        assert_eq!(lifecycle.total_marked_dead, 1);
        assert_eq!(
            lifecycle.last_dead_reason.as_deref(),
            Some("test_dead_marker")
        );
        assert!(engine.activity(session.id)?.idle);
        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn session_activity_tracks_recent_next_core_io() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: Some(quiet_wait_command_for_test()),
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        reset_activity_for_test(session.id)?;

        set_output_for_test(session.id, "recent output")?;
        let _ = engine.read_visible_text(session.id)?;
        engine.scroll_viewport_to(session.id, 0)?;
        let activity = engine.activity(session.id)?;
        assert!(!activity.idle);
        assert!(!activity.foreground_process.is_empty());
        assert!(activity.process.is_some());
        let screen = activity.screen.expect("screen activity");
        assert_eq!(screen.total_reads, 1);
        assert_eq!(screen.total_viewport_scrolls, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_range_reads_update_activity_metrics() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "one\ntwo\nthree\nfour")?;
        reset_activity_for_test(session.id)?;

        let _ = engine.read_lines(session.id, 0, 2)?;
        let _ = engine.read_scrollback(session.id, 2)?;
        let _ = engine.read_scrollback_text(
            session.id,
            ScrollbackTextRequest {
                start_line: Some(0),
                end_line: Some(2),
                tail_lines: None,
                escapes: false,
            },
        )?;
        let _ = engine.read_styled_scrollback(
            session.id,
            ScrollbackTextRequest {
                start_line: Some(0),
                end_line: Some(2),
                tail_lines: None,
                escapes: false,
            },
        )?;
        let _ = engine.search(session.id, "two", 10)?;

        let activity = engine.activity(session.id)?;
        let screen = activity.screen.expect("screen activity");
        assert_eq!(screen.total_reads, 5);

        let health = engine.health()?;
        assert_eq!(health.io.expect("next-core io health").screen_reads, 5);

        engine.destroy_session(session.id)?;
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
            env: Vec::new(),
            launch_policy: Default::default(),
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

    #[test]
    fn screen_snapshots_report_revision_changes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        let initial = engine.read_screen(session.id)?;
        assert_eq!(initial.revision, 0);
        assert_eq!(initial.dirty_rows, None);

        set_output_for_test(session.id, "first")?;
        let first = engine.read_screen(session.id)?;
        let styled = engine.read_styled_screen(session.id)?;
        assert!(first.revision > 0);
        assert_eq!(first.dirty_rows, Some(DirtyRows { start: 0, end: 0 }));
        assert_eq!(styled.revision, first.revision);
        assert_eq!(styled.dirty_rows, first.dirty_rows);

        set_output_for_test(session.id, "second")?;
        let second = engine.read_screen(session.id)?;
        assert!(second.revision > first.revision);
        assert_eq!(second.dirty_rows, Some(DirtyRows { start: 0, end: 0 }));

        engine.resize_session(session.id, 80, 3)?;
        let resized = engine.read_screen(session.id)?;
        assert!(resized.revision > second.revision);
        assert_eq!(resized.dirty_rows, Some(DirtyRows { start: 0, end: 2 }));

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn render_frames_report_full_then_dirty_delta() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        let screen_handle = {
            let state = state().read();
            Arc::clone(
                &state
                    .sessions
                    .iter()
                    .find(|candidate| candidate.snapshot.id == session.id)
                    .expect("session exists")
                    .screen,
            )
        };

        screen_handle.lock().feed("alpha");
        let full = engine.read_render_frame(session.id, None)?;
        assert!(full.full);
        assert_eq!(full.dirty_rows, Some(DirtyRows { start: 0, end: 3 }));
        assert_eq!(full.lines.len(), 4);
        assert!(full.lines.iter().all(|line| line.cells.len() == 12));
        assert_eq!(full.lines[0].row, 0);
        assert_eq!(
            full.lines[0]
                .cells
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "alpha       "
        );
        assert_eq!(
            full.lines[3]
                .cells
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "            "
        );

        let unchanged = engine.read_render_frame(session.id, Some(full.revision))?;
        assert!(!unchanged.full);
        assert_eq!(unchanged.dirty_rows, None);
        assert!(unchanged.lines.is_empty());

        screen_handle.lock().feed("!");
        let delta = engine.read_render_frame(session.id, Some(full.revision))?;
        assert!(!delta.full);
        assert_eq!(delta.dirty_rows, Some(DirtyRows { start: 0, end: 0 }));
        assert_eq!(delta.lines.len(), 1);
        assert_eq!(delta.lines[0].cells.len(), 12);
        assert_eq!(delta.lines[0].row, 0);
        assert_eq!(
            delta.lines[0]
                .cells
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "alpha!      "
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn render_frames_accumulate_dirty_rows_across_chunks() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        let screen_handle = {
            let state = state().read();
            Arc::clone(
                &state
                    .sessions
                    .iter()
                    .find(|candidate| candidate.snapshot.id == session.id)
                    .expect("session exists")
                    .screen,
            )
        };

        screen_handle.lock().feed("seed");
        let baseline = engine.read_render_frame(session.id, None)?;
        assert!(baseline.full);

        screen_handle.lock().feed("\x1b[1;1HA");
        screen_handle.lock().feed("\x1b[2;1HB");
        let delta = engine.read_render_frame(session.id, Some(baseline.revision))?;
        assert!(!delta.full);
        assert_eq!(delta.dirty_rows, Some(DirtyRows { start: 0, end: 1 }));
        assert_eq!(delta.lines.len(), 2);
        assert!(delta.lines.iter().all(|line| line.cells.len() == 12));
        assert_eq!(delta.lines[0].row, 0);
        assert_eq!(delta.lines[0].cells[0].ch, 'A');
        assert_eq!(delta.lines[1].row, 1);
        assert_eq!(delta.lines[1].cells[0].ch, 'B');

        let stale = engine.read_render_frame(session.id, Some(baseline.revision))?;
        assert!(stale.full);
        assert_eq!(stale.dirty_rows, Some(DirtyRows { start: 0, end: 3 }));
        assert_eq!(stale.lines.len(), 4);
        assert!(stale.lines.iter().all(|line| line.cells.len() == 12));

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn render_frames_mark_cursor_only_moves_dirty() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        let screen_handle = {
            let state = state().read();
            Arc::clone(
                &state
                    .sessions
                    .iter()
                    .find(|candidate| candidate.snapshot.id == session.id)
                    .expect("session exists")
                    .screen,
            )
        };

        screen_handle.lock().feed("cursor");
        let baseline = engine.read_render_frame(session.id, None)?;
        assert_eq!(baseline.cursor.x, 6);

        screen_handle.lock().feed("\x1b[2D");
        let delta = engine.read_render_frame(session.id, Some(baseline.revision))?;
        assert!(!delta.full);
        assert_eq!(delta.dirty_rows, Some(DirtyRows { start: 0, end: 0 }));
        assert_eq!(delta.cursor.x, 4);
        assert_eq!(delta.lines.len(), 1);
        assert_eq!(delta.lines[0].row, 0);
        assert_eq!(delta.lines[0].cells.len(), 12);
        assert_eq!(
            delta.lines[0]
                .cells
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "cursor      "
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_wraps_text_at_configured_columns() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 5,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "abcdefgh")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.cols, 5);
        assert_eq!(screen.lines, vec!["abcde", "fgh"]);
        assert_eq!(screen.cursor.x, 3);
        assert_eq!(screen.cursor.y, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_honors_decawm_auto_wrap_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 5,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[?7labcdef")?;
        let disabled = engine.read_screen(session.id)?;
        assert_eq!(disabled.lines, vec!["abcdf"]);
        assert_eq!(disabled.cursor.x, 5);
        assert_eq!(disabled.cursor.y, 0);

        set_output_for_test(session.id, "\x1b[?7labcde\x1b[?7hf")?;
        let reenabled = engine.read_screen(session.id)?;
        assert_eq!(reenabled.lines, vec!["abcde", "f"]);
        assert_eq!(reenabled.cursor.x, 1);
        assert_eq!(reenabled.cursor.y, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_treats_tab_as_cursor_movement() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "a\tb")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["a       b"]);
        assert_eq!(screen.cursor.x, 9);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_treats_vertical_tab_and_form_feed_as_newline() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "one\x0btwo\x0cthree\x0bfour")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["two", "three", "four"]);
        assert_eq!(screen.scrollback_rows, 1);
        assert_eq!(engine.read_scrollback(session.id, 10)?, vec!["one"]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_clamps_tab_at_right_edge_without_wrapping() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 5,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "abcd\tZ")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["abcdZ"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_supports_custom_tab_stops() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[5G\x1bH\rA\tB")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["A   B"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_forward_tab_csi() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 20,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "A\x1b[2IB")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["A               B"]);
        assert_eq!(screen.cursor.x, 17);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_forward_tab_csi_uses_custom_tab_stops() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[5G\x1bH\x1b[9G\x1bH\rA\x1b[2IB")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["A       B"]);
        assert_eq!(screen.cursor.x, 9);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_forward_tab_csi_clamps_at_right_edge() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "A\x1b[5IZ")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["A        Z"]);
        assert_eq!(screen.cursor.x, 10);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_clears_current_tab_stop() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[5G\x1bH\x1b[0g\rA\tB")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["A       B"]);
        assert_eq!(screen.cursor.x, 9);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_clears_all_tab_stops() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 6,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[3gA\tB")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["A    B"]);
        assert_eq!(screen.cursor.x, 6);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_wraps_wide_cells_before_right_edge() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 5,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "abcd你")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["abcd", "你"]);
        assert_eq!(screen.cursor.x, 2);
        assert_eq!(screen.cursor.y, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_truncates_existing_lines_on_column_resize() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "abcdef")?;
        engine.resize_session(session.id, 4, 3)?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.cols, 4);
        assert_eq!(screen.lines, vec!["abcd"]);
        assert_eq!(screen.cursor.x, 3);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_strips_terminal_control_sequences() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            "\x1b[1t\x1b[6n\x1b[c\x1b[31mred\x1b[0m\rOK\nplain\x1b]0;title\x07\n",
        )?;

        let text = engine.read_visible_text(session.id)?;
        assert!(text.contains("OKd"));
        assert!(text.contains("plain"));
        assert!(!text.contains("\x1b["));
        assert!(!text.contains("title"));

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_osc_title_updates() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            "before\x1b]0;Claude Session\x07after\x1b]2;Codex Pane\x1b\\",
        )?;

        assert_eq!(engine.read_visible_text(session.id)?, "beforeafter");
        assert_eq!(engine.get_session(session.id)?.title, "Codex Pane");
        assert_eq!(engine.list_sessions()?[0].title, "Codex Pane");

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_title_stack_window_operations() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "pre\x1b]2;Original\x07\x1b[22;0t\x1b]2;Temporary\x1b\\mid\x1b[22;2t\x1b]2;Nested\x07\x1b[23;2t\x1b[23;0tpost",
        )?;

        assert_eq!(engine.read_visible_text(session.id)?, "premidpost");
        assert_eq!(engine.get_session(session.id)?.title, "Original");

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_osc8_hyperlinks() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 2,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "pre \x1b]8;id=one;https://example.test/item\x07link\x1b]8;;\x07 post\nnext",
        )?;

        assert_eq!(engine.read_visible_text(session.id)?, "pre link post\nnext");
        let styled = engine.read_styled_screen(session.id)?;
        let first = &styled.lines[0].cells;
        assert_eq!(first[4].ch, 'l');
        assert_eq!(
            first[4].style.hyperlink.as_deref(),
            Some("https://example.test/item")
        );
        assert_eq!(
            first[7].style.hyperlink.as_deref(),
            Some("https://example.test/item")
        );
        assert_eq!(first[8].style.hyperlink, None);

        let scrollback = engine.read_styled_scrollback(
            session.id,
            ScrollbackTextRequest {
                start_line: Some(0),
                end_line: Some(1),
                tail_lines: None,
                escapes: false,
            },
        )?;
        assert_eq!(
            scrollback.lines[0].cells[4].style.hyperlink.as_deref(),
            Some("https://example.test/item")
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_osc7_cwd_updates() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let launch_cwd = std::env::current_dir()?.display().to_string();
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: Some(launch_cwd),
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "before\x1b]7;file://localhost/D:/code/unterm%20next\x07after",
        )?;

        #[cfg(windows)]
        let expected = "D:\\code\\unterm next";
        #[cfg(not(windows))]
        let expected = "/D:/code/unterm next";

        assert_eq!(engine.read_visible_text(session.id)?, "beforeafter");
        assert_eq!(engine.shell(session.id)?.cwd.as_deref(), Some(expected));
        assert_eq!(
            engine.get_session(session.id)?.shell.cwd.as_deref(),
            Some(expected)
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_ignores_invalid_osc7_cwd_updates() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let launch_cwd = std::env::current_dir()?.display().to_string();
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: Some(launch_cwd),
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        let initial = engine.shell(session.id)?.cwd;

        set_output_for_test(session.id, "\x1b]7;https://example.test/D:/bad\x1b\\")?;

        assert_eq!(engine.shell(session.id)?.cwd, initial);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_sgr_cell_attributes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[1;31mR",
                "\x1b[0mN",
                "\x1b[2mF",
                "\x1b[1mB",
                "\x1b[22mI",
                "\x1b[9mS",
                "\x1b[29mT",
                "\x1b[8mH",
                "\x1b[28mV",
                "\x1b[5mL",
                "\x1b[6mR",
                "\x1b[25mQ",
                "\x1b[53mO",
                "\x1b[55mP",
                "\x1b[73mA",
                "\x1b[74mB",
                "\x1b[75mC",
                "\x1b[3;4;7;38;5;202;48;2;1;2;3mX",
                "\x1b[22;23;24;25;27;28;29;55;75;39;49mY"
            ),
        )?;

        assert_eq!(engine.read_visible_text(session.id)?, "RNFBISTHVLRQOPABCXY");
        let attrs = viewport_attrs_for_test(session.id)?;
        let line = &attrs[0];

        assert!(line[0].bold);
        assert!(!line[0].faint);
        assert_eq!(line[0].fg, Some(TerminalColor::Palette(1)));
        assert_eq!(line[0].bg, None);

        assert_eq!(line[1], CellAttributes::default());

        assert!(line[2].faint);
        assert!(!line[2].bold);
        assert!(line[3].bold);
        assert!(line[3].faint);
        assert!(!line[4].bold);
        assert!(!line[4].faint);

        assert!(line[5].strikethrough);
        assert!(!line[6].strikethrough);
        assert!(line[7].hidden);
        assert!(!line[8].hidden);

        assert_eq!(line[9].blink, Some(StyledBlink::Slow));
        assert_eq!(line[10].blink, Some(StyledBlink::Rapid));
        assert_eq!(line[11].blink, None);
        assert!(line[12].overline);
        assert!(!line[13].overline);
        assert_eq!(
            line[14].vertical_align,
            Some(StyledVerticalAlign::SuperScript)
        );
        assert_eq!(
            line[15].vertical_align,
            Some(StyledVerticalAlign::SubScript)
        );
        assert_eq!(line[16].vertical_align, None);

        assert!(line[17].italic);
        assert!(line[17].underline);
        assert_eq!(line[17].underline_style, Some(StyledUnderline::Single));
        assert!(line[17].inverse);
        assert_eq!(line[17].fg, Some(TerminalColor::Palette(202)));
        assert_eq!(line[17].bg, Some(TerminalColor::Rgb(1, 2, 3)));

        assert_eq!(line[18], CellAttributes::default());

        let styled = engine.read_styled_screen(session.id)?;
        assert_eq!(styled.lines[0].row, 0);
        assert_eq!(styled.lines[0].cells[0].ch, 'R');
        assert!(styled.lines[0].cells[0].style.bold);
        assert!(styled.lines[0].cells[2].style.faint);
        assert!(!styled.lines[0].cells[4].style.bold);
        assert!(!styled.lines[0].cells[4].style.faint);
        assert!(styled.lines[0].cells[5].style.strikethrough);
        assert!(!styled.lines[0].cells[6].style.strikethrough);
        assert!(styled.lines[0].cells[7].style.hidden);
        assert!(!styled.lines[0].cells[8].style.hidden);
        assert_eq!(
            styled.lines[0].cells[9].style.blink,
            Some(StyledBlink::Slow)
        );
        assert_eq!(
            styled.lines[0].cells[10].style.blink,
            Some(StyledBlink::Rapid)
        );
        assert_eq!(styled.lines[0].cells[11].style.blink, None);
        assert!(styled.lines[0].cells[12].style.overline);
        assert!(!styled.lines[0].cells[13].style.overline);
        assert_eq!(
            styled.lines[0].cells[14].style.vertical_align,
            Some(StyledVerticalAlign::SuperScript)
        );
        assert_eq!(
            styled.lines[0].cells[15].style.vertical_align,
            Some(StyledVerticalAlign::SubScript)
        );
        assert_eq!(styled.lines[0].cells[16].style.vertical_align, None);
        assert_eq!(
            styled.lines[0].cells[0].style.fg,
            Some(StyledColor::Palette(1))
        );
        assert_eq!(
            styled.lines[0].cells[17].style.bg,
            Some(StyledColor::Rgb(1, 2, 3))
        );
        assert_eq!(styled.lines[0].cells[18].style, CellStyle::default());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_reverse_video_mode_to_styled_cells() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 24);
        screen.feed("\x1b[?5hA\x1b[7mB\x1b[27mC");

        assert_eq!(screen.snapshot_viewport_lines()[0], "ABC");
        let styled = screen.styled_viewport_lines(0);
        let cells = &styled[0].cells;
        assert!(cells[0].style.inverse);
        assert!(!cells[1].style.inverse);
        assert!(cells[2].style.inverse);

        screen.feed("\x1b[?5lD");
        assert_eq!(screen.snapshot_viewport_lines()[0], "ABCD");
        let styled = screen.styled_viewport_lines(0);
        let cells = &styled[0].cells;
        assert!(!cells[0].style.inverse);
        assert!(cells[1].style.inverse);
        assert!(!cells[2].style.inverse);
        assert!(!cells[3].style.inverse);

        Ok(())
    }

    #[test]
    fn screen_buffer_keeps_reverse_video_isolated_across_alternate_screen() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        screen.feed("\x1b[?5hM\x1b[?1049hA");
        assert_eq!(screen.snapshot_viewport_lines(), vec!["A"]);
        let styled = screen.styled_viewport_lines(0);
        assert!(!styled[0].cells[0].style.inverse);

        screen.feed("\x1b[?5hB\x1b[?1049lC");
        assert_eq!(screen.snapshot_viewport_lines(), vec!["MC"]);
        let styled = screen.styled_viewport_lines(0);
        assert!(styled[0].cells[0].style.inverse);
        assert!(styled[0].cells[1].style.inverse);

        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_colon_sgr_extended_colors() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[38:5:196mR",
                "\x1b[48:5:25mB",
                "\x1b[38:2::1:2:3mF",
                "\x1b[48:2:0:4:5:6mG",
                "\x1b[0mN"
            ),
        )?;

        assert_eq!(engine.read_visible_text(session.id)?, "RBFGN");
        let attrs = viewport_attrs_for_test(session.id)?;
        let line = &attrs[0];

        assert_eq!(line[0].fg, Some(TerminalColor::Palette(196)));
        assert_eq!(line[1].fg, Some(TerminalColor::Palette(196)));
        assert_eq!(line[1].bg, Some(TerminalColor::Palette(25)));
        assert_eq!(line[2].fg, Some(TerminalColor::Rgb(1, 2, 3)));
        assert_eq!(line[2].bg, Some(TerminalColor::Palette(25)));
        assert_eq!(line[3].fg, Some(TerminalColor::Rgb(1, 2, 3)));
        assert_eq!(line[3].bg, Some(TerminalColor::Rgb(4, 5, 6)));
        assert_eq!(line[4], CellAttributes::default());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_sgr_underline_colors() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[4;58;5;220mP",
                "\x1b[58;2;1;2;3mR",
                "\x1b[58:5:45mC",
                "\x1b[58:2::4:5:6mT",
                "\x1b[59mD",
                "\x1b[24mN"
            ),
        )?;

        let attrs = viewport_attrs_for_test(session.id)?;
        let line = &attrs[0];
        assert!(line[0].underline);
        assert_eq!(line[0].underline_color, Some(TerminalColor::Palette(220)));
        assert_eq!(line[1].underline_color, Some(TerminalColor::Rgb(1, 2, 3)));
        assert_eq!(line[2].underline_color, Some(TerminalColor::Palette(45)));
        assert_eq!(line[3].underline_color, Some(TerminalColor::Rgb(4, 5, 6)));
        assert!(line[4].underline);
        assert_eq!(line[4].underline_color, None);
        assert_eq!(line[5], CellAttributes::default());

        let styled = engine.read_styled_screen(session.id)?;
        assert_eq!(
            styled.lines[0].cells[0].style.underline_color,
            Some(StyledColor::Palette(220))
        );
        assert_eq!(
            styled.lines[0].cells[1].style.underline_color,
            Some(StyledColor::Rgb(1, 2, 3))
        );
        assert_eq!(
            styled.lines[0].cells[2].style.underline_color,
            Some(StyledColor::Palette(45))
        );
        assert_eq!(
            styled.lines[0].cells[3].style.underline_color,
            Some(StyledColor::Rgb(4, 5, 6))
        );
        assert_eq!(styled.lines[0].cells[4].style.underline_color, None);
        assert_eq!(styled.lines[0].cells[5].style, CellStyle::default());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_extended_underline_styles() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[4:3mC",
                "\x1b[4:4mD",
                "\x1b[4:5mS",
                "\x1b[4:0mN",
                "\x1b[21mB",
                "\x1b[24mR"
            ),
        )?;

        let attrs = viewport_attrs_for_test(session.id)?;
        let line = &attrs[0];
        assert!(line[0].underline);
        assert_eq!(line[0].underline_style, Some(StyledUnderline::Curly));
        assert!(!line[0].italic);
        assert!(line[1].underline);
        assert_eq!(line[1].underline_style, Some(StyledUnderline::Dotted));
        assert!(line[2].underline);
        assert_eq!(line[2].underline_style, Some(StyledUnderline::Dashed));
        assert!(!line[3].underline);
        assert_eq!(line[3].underline_style, None);
        assert!(line[4].underline);
        assert_eq!(line[4].underline_style, Some(StyledUnderline::Double));
        assert!(!line[5].underline);
        assert_eq!(line[5].underline_style, None);

        let styled = engine.read_styled_screen(session.id)?;
        assert_eq!(
            styled.lines[0].cells[0].style.underline_style,
            Some(StyledUnderline::Curly)
        );
        assert_eq!(
            styled.lines[0].cells[1].style.underline_style,
            Some(StyledUnderline::Dotted)
        );
        assert_eq!(
            styled.lines[0].cells[2].style.underline_style,
            Some(StyledUnderline::Dashed)
        );
        assert_eq!(
            styled.lines[0].cells[4].style.underline_style,
            Some(StyledUnderline::Double)
        );
        assert_eq!(styled.lines[0].cells[5].style, CellStyle::default());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_wide_character_cells() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "你A")?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines[0], "你A");
        assert_eq!(screen.cursor.x, 3);

        let styled = engine.read_styled_screen(session.id)?;
        let cells = &styled.lines[0].cells;
        assert_eq!(cells[0].ch, '你');
        assert_eq!(cells[0].width, 2);
        assert_eq!(cells[1].width, 0);
        assert_eq!(cells[2].ch, 'A');
        assert_eq!(cells[2].width, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_preserves_combining_marks_on_base_cells() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "e\u{0301}X")?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["e\u{0301}X"]);
        assert_eq!(screen.cursor.x, 2);

        let styled = engine.read_styled_screen(session.id)?;
        let cells = &styled.lines[0].cells;
        assert_eq!(cells[0].ch, 'e');
        assert_eq!(cells[0].width, 1);
        assert_eq!(cells[1].ch, 'X');
        assert_eq!(cells[1].width, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_attaches_combining_marks_to_previous_wide_cell() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "你\u{0301}A")?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["你\u{0301}A"]);
        assert_eq!(screen.cursor.x, 3);

        let styled = engine.read_styled_screen(session.id)?;
        let cells = &styled.lines[0].cells;
        assert_eq!(cells[0].ch, '你');
        assert_eq!(cells[0].width, 2);
        assert_eq!(cells[1].width, 0);
        assert_eq!(cells[2].ch, 'A');
        assert_eq!(cells[2].width, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_repeats_previous_character_with_rep() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "\x1b[31mA\x1b[3b")?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["AAAA"]);
        assert_eq!(screen.cursor.x, 4);

        let styled = engine.read_styled_screen(session.id)?;
        assert_eq!(
            styled.lines[0].cells[0].style.fg,
            Some(StyledColor::Palette(1))
        );
        assert_eq!(
            styled.lines[0].cells[3].style.fg,
            Some(StyledColor::Palette(1))
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_repeats_wide_character_with_rep() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "你\x1b[2b")?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["你你你"]);
        assert_eq!(screen.cursor.x, 6);

        let styled = engine.read_styled_screen(session.id)?;
        assert_eq!(styled.lines[0].cells[0].width, 2);
        assert_eq!(styled.lines[0].cells[1].width, 0);
        assert_eq!(styled.lines[0].cells[4].ch, '你');
        assert_eq!(styled.lines[0].cells[5].width, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn read_styled_scrollback_preserves_history_cell_attributes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 2,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "one \x1b[31mred\x1b[0m\nplain\n")?;

        let styled = engine.read_styled_scrollback(
            session.id,
            ScrollbackTextRequest {
                start_line: Some(0),
                end_line: Some(1),
                tail_lines: None,
                escapes: false,
            },
        )?;

        assert_eq!(styled.first_row, 0);
        assert_eq!(styled.row_count, 1);
        assert_eq!(styled.lines[0].row, 0);
        assert_eq!(styled.lines[0].cells[0].ch, 'o');
        assert_eq!(
            styled.lines[0].cells[4].style.fg,
            Some(StyledColor::Palette(1))
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_basic_csi_screen_operations() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            "old line\n\x1b[2J\x1b[Hhello\nworld\x1b[1A\x1b[3G!\x1b[K",
        )?;

        let lines = engine.read_screen(session.id)?.lines;
        assert!(!lines.iter().any(|line| line.contains("old line")));
        assert!(lines.iter().any(|line| line == "he!"));
        assert!(lines.iter().any(|line| line == "world"));

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_character_edit_and_erase_modes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 6,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "abcde",
                "\x1b[1;3H",
                "\x1b[2@",
                "XY",
                "\x1b[1;5H",
                "\x1b[2P",
                "\x1b[2;1Hkeep",
                "\x1b[3;1Hprefix-tail",
                "\x1b[3;7H",
                "\x1b[K",
                "\x1b[4;1Herase-left",
                "\x1b[4;6H",
                "\x1b[1K",
                "\x1b[5;1Herase-all",
                "\x1b[2K",
                "\x1b[6;1Herase-chars",
                "\x1b[6;6H",
                "\x1b[3X"
            ),
        )?;

        let lines = engine.read_screen(session.id)?.lines;
        assert_eq!(
            lines,
            vec!["abXYe", "keep", "prefix", "      left", "", "erase   ars"]
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_delete_chars_backfills_cells_to_right_margin() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 6,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "abcdef\x1b[31m\x1b[1;3H\x1b[2P")?;

        assert_eq!(engine.read_screen(session.id)?.lines, vec!["abef"]);
        let styled = engine.read_styled_screen(session.id)?;
        let cells = &styled.lines[0].cells;
        assert_eq!(cells.len(), 6);
        assert_eq!(
            cells.iter().map(|cell| cell.ch).collect::<String>(),
            "abef  "
        );
        assert_eq!(cells[4].style.fg, Some(StyledColor::Palette(1)));
        assert_eq!(cells[5].style.fg, Some(StyledColor::Palette(1)));

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_insert_chars_preserves_cells_outside_right_margin() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "0123456789\x1b[?69h\x1b[3;8s\x1b[1;4H\x1b[2@")?;

        assert_eq!(engine.read_screen(session.id)?.lines, vec!["012  34589"]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_delete_chars_preserves_cells_outside_right_margin() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "0123456789\x1b[?69h\x1b[3;8s\x1b[1;4H\x1b[2P")?;

        assert_eq!(engine.read_screen(session.id)?.lines, vec!["012567  89"]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_horizontal_scroll() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "abcdefghij\x1b[1;3H\x1b[2 @\x1b[1;5H\x1b[3 A")?;
        let screen = engine.read_screen(session.id)?;

        assert_eq!(screen.lines, vec!["abef   ghi"]);
        assert_eq!(screen.cursor.x, 4);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_limits_horizontal_scroll_to_margins() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "0123456789\x1b[?69h\x1b[3;8s\x1b[1;4H\x1b[2 @\x1b[1;5H\x1b[1 A",
        )?;
        let screen = engine.read_screen(session.id)?;

        assert_eq!(screen.lines, vec!["0125 67 89"]);
        assert_eq!(screen.cursor.x, 4);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_erase_line_modes_backfill_styled_cells() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 6,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "abcdef",
                "\x1b[31m\x1b[1;4H\x1b[K",
                "\x1b[2;1Hghijkl",
                "\x1b[32m\x1b[2;4H\x1b[1K",
                "\x1b[3;1Hmnopqr",
                "\x1b[34m\x1b[3;3H\x1b[2K"
            ),
        )?;

        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["abc", "    kl", ""]
        );
        let styled = engine.read_styled_screen(session.id)?;

        let first = &styled.lines[0].cells;
        assert_eq!(
            first.iter().map(|cell| cell.ch).collect::<String>(),
            "abc   "
        );
        assert_eq!(first[3].style.fg, Some(StyledColor::Palette(1)));
        assert_eq!(first[5].style.fg, Some(StyledColor::Palette(1)));

        let second = &styled.lines[1].cells;
        assert_eq!(
            second.iter().map(|cell| cell.ch).collect::<String>(),
            "    kl"
        );
        assert_eq!(second[0].style.fg, Some(StyledColor::Palette(2)));
        assert_eq!(second[3].style.fg, Some(StyledColor::Palette(2)));

        let third = &styled.lines[2].cells;
        assert_eq!(
            third.iter().map(|cell| cell.ch).collect::<String>(),
            "      "
        );
        assert!(third
            .iter()
            .all(|cell| cell.style.fg == Some(StyledColor::Palette(4))));

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_escape_index_sequences() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "A\x1bDB\x1bEC")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["A", " B", "C"]);
        assert_eq!(screen.cursor.x, 1);
        assert_eq!(screen.cursor.y, 2);

        set_output_for_test(
            session.id,
            "\x1b[2J\x1b[Hr0\nr1\nr2\nr3\x1b[2;4r\x1b[2;1H\x1bM",
        )?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["r0", "", "r1", "r2"]);
        assert_eq!(screen.cursor.x, 0);
        assert_eq!(screen.cursor.y, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_index_sequences_do_not_scroll_when_cursor_outside_region() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 5,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "top\r\none\r\ntwo\r\nthree\r\nbottom\x1b[2;4r\x1b[5;1H\x1bD!",
        )?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["top", "one", "two", "three", "!ottom"]);
        assert_eq!(screen.cursor.x, 1);
        assert_eq!(screen.cursor.y, 4);

        set_output_for_test(
            session.id,
            "\x1b[2J\x1b[Htop\r\none\r\ntwo\r\nthree\r\nbottom\x1b[2;4r\x1b[1;1H\x1bM!",
        )?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["!op", "one", "two", "three", "bottom"]);
        assert_eq!(screen.cursor.x, 1);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_ignores_charset_and_utf8_designators() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 16,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "ab\x1b(Bcd\x1b)0ef\x1b%Ggh")?;
        assert_eq!(engine.read_screen(session.id)?.lines, vec!["abcdefgh"]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_ignores_non_osc_string_controls() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 24,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "ab\x1bP1;payload\x1b\\cd\x1b_should-not-print\x07ef\x1b^hidden\x1b\\gh\x1bXmore\x07ij",
        )?;
        assert_eq!(engine.read_screen(session.id)?.lines, vec!["abcdefghij"]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn terminal_parser_preserves_state_across_split_chunks() {
        let mut screen = NextCoreScreen::new(24, 3);

        screen.feed("ab\x1b[31");
        screen.feed("mR\x1b[0");
        screen.feed("m\x1b]2;Split");
        screen.feed(" title\x07Z");

        assert_eq!(screen.snapshot_viewport_lines(), vec!["abRZ"]);
        assert_eq!(screen.title.as_deref(), Some("Split title"));
        assert_eq!(screen.lines[0][2].attr.fg, Some(TerminalColor::Palette(1)));
        assert_eq!(screen.lines[0][3].attr, CellAttributes::default());
    }

    #[test]
    fn terminal_parser_ignores_split_string_controls() {
        let mut screen = NextCoreScreen::new(24, 3);

        screen.feed("A\x1bPpayload");
        screen.feed("\x1b\\B\x1b_more");
        screen.feed("\x07C");

        assert_eq!(screen.snapshot_viewport_lines(), vec!["ABC"]);
    }

    #[test]
    fn screen_buffer_handles_c1_control_forms() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 16,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "ab\u{009b}2GZ\u{009d}0;C1 Title\u{009c}\u{0090}hidden\u{009c}\u{0085}next\u{0084}ind\u{008d}ri",
        )?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["aZ", "next   ri", "    ind"]);
        assert_eq!(engine.get_session(session.id)?.title, "C1 Title");

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_decaln_alignment_test() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 5,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "old\x1b#8")?;
        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["EEEEE", "EEEEE", "EEEEE"]
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_decfra_rectangular_fill() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "0123456789\r\nabcdefghij\r\nABCDEFGHIJ\x1b[1;6H\x1b[31m\x1b[88;2;3;3;8$x",
        )?;
        let screen = engine.read_screen(session.id)?;
        let styled = engine.read_styled_screen(session.id)?;

        assert_eq!(screen.lines, vec!["0123456789", "abXXXXXXij", "ABXXXXXXIJ"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);
        assert_eq!(
            styled.lines[1].cells[2].style.fg,
            Some(StyledColor::Palette(1))
        );
        assert_eq!(
            styled.lines[2].cells[7].style.fg,
            Some(StyledColor::Palette(1))
        );
        assert_eq!(styled.lines[1].cells[1].style.fg, None);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_decfra_clips_to_viewport_and_defaults_to_space() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 6,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "abcdef\r\nABCDEF\x1b[20320;2;5;9;9$x")?;
        let screen = engine.read_screen(session.id)?;
        let styled = engine.read_styled_screen(session.id)?;

        assert_eq!(screen.lines, vec!["abcdef", "ABCD", ""]);
        assert_eq!(
            styled.lines[1]
                .cells
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "ABCD  "
        );
        assert_eq!(
            styled.lines[2]
                .cells
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "      "
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_decera_rectangular_erase() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "0123456789\r\nabcdefghij\r\nABCDEFGHIJ\x1b[1;6H\x1b[32m\x1b[2;3;3;8$z",
        )?;
        let screen = engine.read_screen(session.id)?;
        let styled = engine.read_styled_screen(session.id)?;

        assert_eq!(screen.lines, vec!["0123456789", "ab      ij", "AB      IJ"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);
        assert_eq!(
            styled.lines[1].cells[2].style.fg,
            Some(StyledColor::Palette(2))
        );
        assert_eq!(
            styled.lines[2].cells[7].style.fg,
            Some(StyledColor::Palette(2))
        );
        assert_eq!(styled.lines[1].cells[1].style.fg, None);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_decera_clips_and_defaults_to_full_viewport() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 6,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "abcdef\r\nABCDEF\x1b[2;5;9;9$z")?;
        let screen = engine.read_screen(session.id)?;
        let styled = engine.read_styled_screen(session.id)?;

        assert_eq!(screen.lines, vec!["abcdef", "ABCD", ""]);
        assert_eq!(
            styled.lines[1]
                .cells
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "ABCD  "
        );
        assert_eq!(
            styled.lines[2]
                .cells
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>(),
            "      "
        );

        set_output_for_test(session.id, "abcdef\r\nABCDEF\x1b[$z")?;
        let styled = engine.read_styled_screen(session.id)?;
        assert!(styled
            .lines
            .iter()
            .all(|line| line.cells.iter().all(|cell| cell.ch == ' ')));

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_decsera_selective_rectangular_erase() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            concat!(
                "0123456789\r\n",
                "ab",
                "\x1b[1\"q",
                "PROT",
                "\x1b[0\"q",
                "ghij\r\n",
                "ABCDEFGHIJ",
                "\x1b[1;6H",
                "\x1b[2;1;3;10${"
            ),
        )?;
        let screen = engine.read_screen(session.id)?;

        assert_eq!(screen.lines, vec!["0123456789", "  PROT", ""]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);

        set_output_for_test(
            session.id,
            concat!(
                "0123456789\r\n",
                "ab",
                "\x1b[1\"q",
                "PROT",
                "\x1b[0\"q",
                "ghij",
                "\x1b[2;1;2;10$z"
            ),
        )?;
        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["0123456789", ""]
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_decsel_selective_line_erase() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            concat!(
                "ab",
                "\x1b[1\"q",
                "PROT",
                "\x1b[0\"q",
                "ghij",
                "\x1b[1;1H",
                "\x1b[?0K"
            ),
        )?;
        assert_eq!(engine.read_screen(session.id)?.lines, vec!["  PROT"]);

        set_output_for_test(
            session.id,
            concat!(
                "ab",
                "\x1b[1\"q",
                "PROT",
                "\x1b[0\"q",
                "ghij",
                "\x1b[1;1H",
                "\x1b[0K"
            ),
        )?;
        assert_eq!(engine.read_screen(session.id)?.lines, vec![""]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_decsed_selective_display_erase() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            concat!(
                "0123456789\r\n",
                "ab",
                "\x1b[1\"q",
                "PROT",
                "\x1b[0\"q",
                "ghij\r\n",
                "ABCDEFGHIJ",
                "\x1b[2;1H",
                "\x1b[?0J"
            ),
        )?;
        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["0123456789", "  PROT", ""]
        );

        set_output_for_test(
            session.id,
            concat!(
                "0123456789\r\n",
                "ab",
                "\x1b[1\"q",
                "PROT",
                "\x1b[0\"q",
                "ghij\r\n",
                "ABCDEFGHIJ",
                "\x1b[2;1H",
                "\x1b[0J"
            ),
        )?;
        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["0123456789", "", ""]
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_decsca_mode_two_returns_to_erasable_cells() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[1\"qAB\x1b[2\"qCD\x1b[1;1;1;4${")?;

        assert_eq!(engine.read_screen(session.id)?.lines, vec!["AB"]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_deccara_rectangular_attributes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "0123456789\r\nabcdefghij\r\nABCDEFGHIJ\x1b[1;6H\x1b[2;3;3;8;1;4;7;8$r",
        )?;
        let screen = engine.read_screen(session.id)?;
        let styled = engine.read_styled_screen(session.id)?;

        assert_eq!(screen.lines, vec!["0123456789", "abcdefghij", "ABCDEFGHIJ"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);
        assert!(styled.lines[1].cells[2].style.bold);
        assert!(styled.lines[1].cells[2].style.underline);
        assert!(styled.lines[1].cells[2].style.inverse);
        assert!(styled.lines[1].cells[2].style.hidden);
        assert!(styled.lines[2].cells[7].style.bold);
        assert!(styled.lines[2].cells[7].style.hidden);
        assert!(!styled.lines[1].cells[1].style.bold);
        assert!(!styled.lines[2].cells[8].style.inverse);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_deccara_resets_rectangular_attributes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "\x1b[1;4;5;7;8mabcdefgh\r\nABCDEFGH\x1b[2;3;2;6;0;28$r",
        )?;
        let styled = engine.read_styled_screen(session.id)?;

        let preserved = &styled.lines[0].cells[2].style;
        assert!(preserved.bold);
        assert!(preserved.underline);
        assert!(preserved.inverse);
        assert!(preserved.hidden);

        let reset = &styled.lines[1].cells[2].style;
        assert!(!reset.bold);
        assert!(!reset.underline);
        assert!(!reset.inverse);
        assert!(!reset.hidden);

        let outside = &styled.lines[1].cells[6].style;
        assert!(outside.bold);
        assert!(outside.underline);
        assert!(outside.inverse);
        assert!(outside.hidden);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_decrara_rectangular_attribute_reverse() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "0123456789\r\nabcdefghij\r\nABCDEFGHIJ\x1b[1;6H\x1b[2;3;3;8;1;4;7;8$t",
        )?;
        let screen = engine.read_screen(session.id)?;
        let styled = engine.read_styled_screen(session.id)?;

        assert_eq!(screen.lines, vec!["0123456789", "abcdefghij", "ABCDEFGHIJ"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);
        assert!(styled.lines[1].cells[2].style.bold);
        assert!(styled.lines[1].cells[2].style.underline);
        assert!(styled.lines[1].cells[2].style.inverse);
        assert!(styled.lines[1].cells[2].style.hidden);
        assert!(styled.lines[2].cells[7].style.bold);
        assert!(styled.lines[2].cells[7].style.hidden);
        assert!(!styled.lines[1].cells[1].style.bold);
        assert!(!styled.lines[2].cells[8].style.inverse);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_decrara_toggles_existing_rectangular_attributes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "\x1b[1;4;5;7;8mabcdefgh\r\nABCDEFGH\x1b[2;3;2;6;0;8$t",
        )?;
        let styled = engine.read_styled_screen(session.id)?;

        let preserved = &styled.lines[0].cells[2].style;
        assert!(preserved.bold);
        assert!(preserved.underline);
        assert!(preserved.inverse);
        assert!(preserved.hidden);

        let toggled = &styled.lines[1].cells[2].style;
        assert!(!toggled.bold);
        assert!(!toggled.underline);
        assert!(!toggled.inverse);
        assert!(toggled.blink.is_none());
        assert!(!toggled.hidden);

        let outside = &styled.lines[1].cells[6].style;
        assert!(outside.bold);
        assert!(outside.underline);
        assert!(outside.inverse);
        assert!(outside.hidden);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_csi_save_and_restore_cursor() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "one\x1b[sXX\x1b[uY")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["oneYX"]);
        assert_eq!(screen.cursor.x, 4);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_save_and_restore_cursor_preserves_sgr_attributes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[31mA\x1b7\x1b[0mB\x1b8C")?;
        let screen = engine.read_screen(session.id)?;
        let styled = engine.read_styled_screen(session.id)?;

        assert_eq!(screen.lines, vec!["AC"]);
        assert_eq!(
            styled.lines[0].cells[0].style.fg,
            Some(StyledColor::Palette(1))
        );
        assert_eq!(
            styled.lines[0].cells[1].style.fg,
            Some(StyledColor::Palette(1))
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_save_and_restore_cursor_preserves_hyperlinks() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "\x1b]8;;https://example.test\x07A\x1b7\x1b]8;;\x07B\x1b8C",
        )?;
        let styled = engine.read_styled_screen(session.id)?;

        assert_eq!(engine.read_screen(session.id)?.lines, vec!["AC"]);
        assert_eq!(
            styled.lines[0].cells[0].style.hyperlink.as_deref(),
            Some("https://example.test")
        );
        assert_eq!(
            styled.lines[0].cells[1].style.hyperlink.as_deref(),
            Some("https://example.test")
        );

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_dec_private_cursor_save_and_restore() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "ab\x1b[?1048h\x1b[3;5HZ\x1b[?1048lY")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["abY", "", "    Z"]);
        assert_eq!(screen.cursor.x, 3);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_extended_cursor_positioning() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 5,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            concat!("A", "\x1b[2EB", "\x1b[1FC", "\x1b[6`D", "\x1b[2aE", "\x1b[4dF", "\x1b[1eG"),
        )?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(
            screen.lines,
            vec!["A", "C    D  E", "B", "         F", "          G"]
        );
        assert_eq!(screen.cursor.x, 11);
        assert_eq!(screen.cursor.y, 4);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_reverse_tab_positioning() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 16,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[5G\x1bH\x1b[13GX\x1b[ZY\x1b[2ZZ")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["    Z   Y   X"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_display_erase_modes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;

        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "one\nprefix-tail\nthree\x1b[2;7H\x1b[J")?;
        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["one", "prefix", ""]
        );
        engine.destroy_session(session.id)?;

        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "one\ntwo-three\nthree\x1b[2;4H\x1b[1J")?;
        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["", "    three", "three"]
        );
        engine.destroy_session(session.id)?;

        Ok(())
    }

    #[test]
    fn screen_buffer_display_erase_mode_2_preserves_cursor_position() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "one\ntwo\nthree\x1b[3;5H\x1b[2JZ")?;
        let screen = engine.read_screen(session.id)?;

        assert_eq!(screen.lines, vec!["", "", "    Z"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 2);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_clears_scrollback_with_display_erase_mode_3() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "one\ntwo\nthree\nfour\nfive")?;
        assert_eq!(engine.read_screen(session.id)?.scrollback_rows, 2);
        assert_eq!(engine.read_scrollback(session.id, 10)?, vec!["one", "two"]);
        engine.destroy_session(session.id)?;

        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "one\ntwo\nthree\nfour\nfive\x1b[3J")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["three", "four", "five"]);
        assert_eq!(screen.scrollback_rows, 0);
        assert!(engine.read_scrollback(session.id, 10)?.is_empty());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_reports_cursor_state() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            "abc\nxx\x1b[1A\x1b[3G!\x1b7\x1b[3;4Hq\x1b8z\x1b[?25l",
        )?;

        let screen = engine.read_screen(session.id)?;
        assert!(screen.lines.iter().any(|line| line == "ab!z"));
        assert_eq!(screen.cursor.x, 4);
        assert_eq!(screen.cursor.y, 0);
        assert!(!screen.cursor.visible);

        let cursor = engine.cursor(session.id)?;
        assert_eq!(cursor.x, screen.cursor.x);
        assert_eq!(cursor.y, screen.cursor.y);
        assert_eq!(cursor.visible, screen.cursor.visible);

        let listed = engine.list_sessions()?.remove(0);
        assert_eq!(listed.cursor.x, screen.cursor.x);
        assert_eq!(listed.cursor.y, screen.cursor.y);
        assert_eq!(listed.cursor.visible, screen.cursor.visible);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_cursor_shape() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "abc\x1b[5 q")?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.cursor.shape, "BlinkingBar");
        assert_eq!(engine.cursor(session.id)?.shape, "BlinkingBar");
        assert_eq!(
            engine.list_sessions()?.remove(0).cursor.shape,
            "BlinkingBar"
        );

        set_output_for_test(
            session.id,
            concat!("main\x1b[4 q", "\x1b[?1049h", "\x1b[2 q", "\x1b[?1049l"),
        )?;

        assert_eq!(engine.cursor(session.id)?.shape, "SteadyUnderline");

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_bracketed_paste_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        assert!(!engine.bracketed_paste_enabled(session.id)?);
        set_output_for_test(session.id, "\x1b[?2004h")?;
        assert!(engine.bracketed_paste_enabled(session.id)?);
        set_output_for_test(session.id, "\x1b[?2004h\x1b[?2004l")?;
        assert!(!engine.bracketed_paste_enabled(session.id)?);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_cursor_blink_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        assert!(screen.cursor_blinking);
        screen.feed("\x1b[?12l");
        assert!(!screen.cursor_blinking);
        screen.feed("\x1b[?12h");
        assert!(screen.cursor_blinking);
        screen.feed("\x1b[?12l\x1b[!p");
        assert!(screen.cursor_blinking);

        Ok(())
    }

    #[test]
    fn screen_buffer_applies_column_mode_switching() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "main\x1b[2;3r\x1b[3;5Hbefore\x1b[?3hwide")?;
        let wide = engine.read_screen(session.id)?;
        assert_eq!(wide.cols, 132);
        assert_eq!(wide.cursor.x, 4);
        assert_eq!(wide.cursor.y, 0);
        assert_eq!(wide.lines, vec!["wide"]);

        set_output_for_test(session.id, "\x1b[?3lnarrow")?;
        let narrow = engine.read_screen(session.id)?;
        assert_eq!(narrow.cols, 80);
        assert_eq!(narrow.cursor.x, 6);
        assert_eq!(narrow.cursor.y, 0);
        assert_eq!(narrow.lines, vec!["narrow"]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_honors_left_right_margins() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "\x1b[?69h\x1b[3;6s\x1b[1;6HAB\x1b[?69l\x1b[1;1HC",
        )?;
        let screen = engine.read_screen(session.id)?;

        assert_eq!(screen.lines, vec!["C    A", "  B"]);
        assert_eq!(screen.cursor.x, 1);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_application_cursor_key_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        assert!(!screen.application_cursor_keys);
        screen.feed("\x1b[?1h");
        assert!(screen.application_cursor_keys);
        screen.feed("\x1b[?1l");
        assert!(!screen.application_cursor_keys);
        screen.feed("\x1b[?1h\x1b[!p");
        assert!(!screen.application_cursor_keys);

        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_application_keypad_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        assert!(!screen.application_keypad);

        screen.feed("\x1b=");
        assert!(screen.application_keypad);
        screen.feed("\x1b>");
        assert!(!screen.application_keypad);

        screen.feed("\x1b[?66h");
        assert!(screen.application_keypad);
        screen.feed("\x1b[?66l");
        assert!(!screen.application_keypad);

        screen.feed("\x1b=\x1b[!p");
        assert!(!screen.application_keypad);

        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_focus_event_reporting_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        assert!(!screen.focus_event_reporting);
        screen.feed("\x1b[?1004h");
        assert!(screen.focus_event_reporting);
        screen.feed("\x1b[?1004l");
        assert!(!screen.focus_event_reporting);
        screen.feed("\x1b[?1004h\x1b[!p");
        assert!(!screen.focus_event_reporting);

        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_synchronized_output_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        assert!(!screen.synchronized_output);
        screen.feed("\x1b[?2026h");
        assert!(screen.synchronized_output);
        screen.feed("\x1b[?2026l");
        assert!(!screen.synchronized_output);
        screen.feed("\x1b[?2026h\x1b[!p");
        assert!(!screen.synchronized_output);

        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_meta_sends_escape_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        assert!(!screen.meta_sends_escape);
        screen.feed("\x1b[?1034h");
        assert!(screen.meta_sends_escape);
        screen.feed("\x1b[?1034l");
        assert!(!screen.meta_sends_escape);
        screen.feed("\x1b[?1034h\x1b[!p");
        assert!(!screen.meta_sends_escape);

        Ok(())
    }

    #[test]
    fn screen_buffer_tracks_mouse_reporting_modes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        assert_eq!(screen.mouse_tracking, MouseTrackingMode::None);
        assert!(!screen.utf8_mouse);
        assert!(!screen.urxvt_mouse);
        assert!(!screen.sgr_mouse);
        assert!(!screen.alternate_scroll);
        assert!(!screen.sgr_pixel_mouse);

        screen.feed("\x1b[?1000h");
        assert_eq!(screen.mouse_tracking, MouseTrackingMode::X10);
        screen.feed("\x1b[?1002h");
        assert_eq!(screen.mouse_tracking, MouseTrackingMode::ButtonEvent);
        screen.feed("\x1b[?1003h\x1b[?1005h\x1b[?1006h\x1b[?1007h\x1b[?1015h\x1b[?1016h");
        assert_eq!(screen.mouse_tracking, MouseTrackingMode::AnyEvent);
        assert!(screen.utf8_mouse);
        assert!(screen.sgr_mouse);
        assert!(screen.alternate_scroll);
        assert!(screen.urxvt_mouse);
        assert!(screen.sgr_pixel_mouse);

        screen.feed("\x1b[?1002l");
        assert_eq!(screen.mouse_tracking, MouseTrackingMode::AnyEvent);
        screen.feed("\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1007l\x1b[?1015l\x1b[?1016l");
        assert_eq!(screen.mouse_tracking, MouseTrackingMode::None);
        assert!(!screen.utf8_mouse);
        assert!(!screen.sgr_mouse);
        assert!(!screen.alternate_scroll);
        assert!(!screen.urxvt_mouse);
        assert!(!screen.sgr_pixel_mouse);

        screen.feed("\x1b[?1000h\x1b[?1005h\x1b[?1006h\x1b[?1007h\x1b[?1015h\x1b[?1016h\x1b[!p");
        assert_eq!(screen.mouse_tracking, MouseTrackingMode::None);
        assert!(!screen.utf8_mouse);
        assert!(!screen.sgr_mouse);
        assert!(!screen.alternate_scroll);
        assert!(!screen.urxvt_mouse);
        assert!(!screen.sgr_pixel_mouse);

        Ok(())
    }

    #[test]
    fn screen_buffer_applies_insert_mode() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 8,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "abcd\x1b[1;3H\x1b[4hXY\x1b[4lZ")?;
        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["abXYZd"]);
        assert_eq!(screen.cursor.x, 5);
        assert_eq!(screen.cursor.y, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_combined_modes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[?1049;2004halt")?;
        assert_eq!(engine.read_screen(session.id)?.lines, vec!["alt"]);
        assert!(engine.bracketed_paste_enabled(session.id)?);

        set_output_for_test(session.id, "\x1b[?25;2004lmain")?;
        let screen = engine.read_screen(session.id)?;
        assert!(!screen.cursor.visible);
        assert!(!engine.bracketed_paste_enabled(session.id)?);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_ris_terminal_reset() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        let (cwd_osc, expected_cwd) = if cfg!(windows) {
            (
                "\x1b]7;file://localhost/D:/code/unterm\x07",
                "D:\\code\\unterm",
            )
        } else {
            ("\x1b]7;file://localhost/tmp/unterm\x07", "/tmp/unterm")
        };
        set_output_for_test(
            session.id,
            &format!(
                "{cwd_osc}\x1b]0;Reset Test\x07main\x1b[?1049halt\x1b[31m\x1b[?7l\x1b[?6h\x1b[?2004h\x1b[3g\x1b[2;3r\x1b[?25l\x1bcZ\tT"
            ),
        )?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["Z       T"]);
        assert_eq!(screen.cursor.x, 9);
        assert_eq!(screen.cursor.y, 0);
        assert!(screen.cursor.visible);
        assert_eq!(screen.cursor.shape, "Default");
        assert_eq!(screen.scrollback_rows, 0);
        assert!(!engine.bracketed_paste_enabled(session.id)?);
        assert_eq!(engine.get_session(session.id)?.title, "Reset Test");
        assert_eq!(engine.shell(session.id)?.cwd.as_deref(), Some(expected_cwd));

        let styled = engine.read_styled_screen(session.id)?;
        assert_eq!(styled.lines[0].cells[0].style, CellStyle::default());
        assert_eq!(styled.lines[0].cells[8].style, CellStyle::default());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_decstr_soft_reset() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b]0;Soft Reset\x07",
                "\x1b[31mA",
                "\x1b[?7l",
                "\x1b[4h",
                "\x1b[3g",
                "\x1b[?25l",
                "\x1b[!p",
                "B\tC"
            ),
        )?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["AB      C"]);
        assert!(screen.cursor.visible);
        assert_eq!(screen.cursor.shape, "Default");
        assert_eq!(engine.get_session(session.id)?.title, "Soft Reset");

        let styled = engine.read_styled_screen(session.id)?;
        assert_eq!(
            styled.lines[0].cells[0].style.fg,
            Some(StyledColor::Palette(1))
        );
        assert_eq!(styled.lines[0].cells[1].style, CellStyle::default());
        assert_eq!(styled.lines[0].cells[8].style, CellStyle::default());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_decstr_keeps_current_alternate_screen_active() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(20, 3);

        screen.feed(concat!("main", "\x1b[?1049h", "alt", "\x1b[!p", "stay"));
        assert_eq!(screen.snapshot_viewport_lines(), vec!["altstay"]);
        assert!(screen.alternate_screen_modes.contains(&1049));

        screen.feed("\x1b[?1049lback");
        assert_eq!(screen.snapshot_viewport_lines(), vec!["mainback"]);
        assert!(screen.alternate_screen_modes.is_empty());

        Ok(())
    }

    #[test]
    fn screen_buffer_handles_alternate_screen_and_line_mutations() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "main-one\nmain-two",
                "\x1b[?1049h",
                "alt-only\n",
                "\x1b[?1049l",
                "\x1b[H",
                "\x1b[1L",
                "inserted",
                "\x1b[3;1H",
                "\x1b[1M",
                "\x1b7",
                "\x1b[2;6H!",
                "\x1b8",
                "."
            ),
        )?;

        let lines = engine.read_screen(session.id)?.lines;
        assert!(lines.iter().any(|line| line == "inserted"));
        assert!(lines.iter().any(|line| line == "main-!ne"));
        assert!(lines.iter().any(|line| line == "."));
        assert!(!lines.iter().any(|line| line.contains("alt-only")));
        assert!(!lines.iter().any(|line| line.contains("main-two")));

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_scrolls_when_output_exceeds_viewport() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "one\ntwo\nthree\nfour\nfive")?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.rows, 3);
        assert_eq!(screen.scrollback_rows, 2);
        assert_eq!(screen.lines, vec!["three", "four", "five"]);
        assert!(!screen.lines.iter().any(|line| line == "one"));
        assert!(!screen.lines.iter().any(|line| line == "two"));

        assert_eq!(engine.read_scrollback(session.id, 10)?, vec!["one", "two"]);
        let lines = engine.read_lines(session.id, 0, 5)?;
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(text, vec!["one", "two", "three", "four", "five"]);
        let middle = engine.read_lines(session.id, 1, 3)?;
        assert_eq!(
            middle
                .iter()
                .map(|line| (line.row, line.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "two"), (2, "three"), (3, "four")]
        );
        let scrollback_text = engine.read_scrollback_text(
            session.id,
            ScrollbackTextRequest {
                start_line: Some(1),
                end_line: Some(4),
                tail_lines: None,
                escapes: false,
            },
        )?;
        assert_eq!(scrollback_text.lines, vec!["two", "three", "four"]);
        assert_eq!(scrollback_text.first_row, 1);
        assert_eq!(scrollback_text.row_count, 3);
        assert_eq!(engine.search(session.id, "one", 1)?[0].row, 0);

        engine.scroll_viewport_to(session.id, 1)?;
        let scrolled = engine.read_screen(session.id)?;
        assert_eq!(scrolled.lines, vec!["two", "three", "four"]);
        assert_eq!(
            scrolled
                .cells
                .iter()
                .map(|line| (line.row, line.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "two"), (2, "three"), (3, "four")]
        );
        assert!(scrolled.revision > screen.revision);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_keeps_viewport_stable_when_scrollback_is_trimmed() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        let initial = (0..10_010)
            .map(|idx| format!("line-{idx:05}"))
            .collect::<Vec<_>>()
            .join("\n");
        screen.feed(&initial);
        assert_eq!(screen.scrollback_rows(), MAX_SCROLLBACK_LINES);

        screen.set_viewport_top_near(107);
        let before = screen.snapshot_viewport_lines();
        assert_ne!(
            before,
            vec![
                "line-10007".to_string(),
                "line-10008".to_string(),
                "line-10009".to_string()
            ]
        );

        let more = (10_010..10_060)
            .map(|idx| format!("line-{idx:05}"))
            .collect::<Vec<_>>()
            .join("\n");
        screen.feed(&format!("\n{more}"));

        assert_eq!(screen.scrollback_rows(), MAX_SCROLLBACK_LINES);
        assert_eq!(screen.snapshot_viewport_lines(), before);

        Ok(())
    }

    #[test]
    fn screen_buffer_scroll_to_bottom_follows_new_output() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let mut screen = NextCoreScreen::new(80, 3);

        screen.feed("one\ntwo\nthree\nfour\nfive");
        screen.set_viewport_top_near(1);
        assert_eq!(
            screen.snapshot_viewport_lines(),
            vec!["two".to_string(), "three".to_string(), "four".to_string()]
        );

        screen.set_viewport_top_near(2);
        assert!(!screen.history.viewport_is_pinned());
        assert_eq!(
            screen.snapshot_viewport_lines(),
            vec!["three".to_string(), "four".to_string(), "five".to_string()]
        );

        screen.feed("\nsix");
        assert_eq!(
            screen.snapshot_viewport_lines(),
            vec!["four".to_string(), "five".to_string(), "six".to_string()]
        );

        Ok(())
    }

    #[test]
    fn search_reports_multiple_matches_and_character_columns() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "你abc abc\nabc\n")?;

        let matches = engine.search(session.id, "abc", 10)?;
        assert_eq!(
            matches
                .iter()
                .map(|m| (m.row, m.col, m.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(0, 1, "你abc abc"), (0, 5, "你abc abc"), (1, 0, "abc")]
        );
        assert_eq!(engine.search(session.id, "abc", 2)?.len(), 2);
        assert!(engine.search(session.id, "", 10)?.is_empty());
        assert!(engine.search(session.id, "abc", 0)?.is_empty());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_explicit_scroll_commands() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(session.id, "a\nb\nc\x1b[S")?;

        let lines = engine.read_screen(session.id)?.lines;
        assert_eq!(lines, vec!["b", "c", ""]);

        set_output_for_test(session.id, "a\nb\nc\x1b[T")?;
        let lines = engine.read_screen(session.id)?.lines;
        assert_eq!(lines, vec!["", "a", "b"]);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_applies_scroll_regions() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 5,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[1;1Htop",
                "\x1b[2;1Hone",
                "\x1b[3;1Htwo",
                "\x1b[4;1Hthree",
                "\x1b[5;1Hbottom",
                "\x1b[2;4r",
                "\x1b[4;1H\n"
            ),
        )?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["top", "two", "three", "", "bottom"]);
        assert_eq!(screen.scrollback_rows, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_limits_line_insert_delete_to_scroll_region() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;

        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 5,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[1;1Htop",
                "\x1b[2;1Hone",
                "\x1b[3;1Htwo",
                "\x1b[4;1Hthree",
                "\x1b[5;1Hbottom",
                "\x1b[2;4r",
                "\x1b[3;1H",
                "\x1b[L"
            ),
        )?;
        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["top", "one", "", "two", "bottom"]
        );
        engine.destroy_session(session.id)?;

        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 5,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[1;1Htop",
                "\x1b[2;1Hone",
                "\x1b[3;1Htwo",
                "\x1b[4;1Hthree",
                "\x1b[5;1Hbottom",
                "\x1b[2;4r",
                "\x1b[2;1H",
                "\x1b[M"
            ),
        )?;
        assert_eq!(
            engine.read_screen(session.id)?.lines,
            vec!["top", "two", "three", "", "bottom"]
        );
        engine.destroy_session(session.id)?;

        Ok(())
    }

    #[test]
    fn screen_buffer_honors_origin_mode_with_scroll_region() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 5,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[1;1Htop",
                "\x1b[2;1Hone",
                "\x1b[3;1Htwo",
                "\x1b[4;1Hthree",
                "\x1b[5;1Hbottom",
                "\x1b[2;4r",
                "\x1b[?6h",
                "\x1b[1;1HX",
                "\x1b[9;1HY",
                "\x1b[?6l",
                "Z"
            ),
        )?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["Zop", "Xne", "two", "Yhree", "bottom"]);
        assert_eq!(screen.cursor.x, 1);
        assert_eq!(screen.cursor.y, 0);
        assert_eq!(screen.scrollback_rows, 0);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_origin_mode_limits_vertical_cursor_motion_to_scroll_region() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 12,
            rows: 5,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(session.id, "\x1b[2;4r\x1b[?6h\x1b[9Bbottom\r\x1b[9Atop")?;
        let screen = engine.read_screen(session.id)?;

        assert_eq!(screen.lines, vec!["", "top", "", "bottom"]);
        assert_eq!(screen.cursor.x, 3);
        assert_eq!(screen.cursor.y, 1);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_line_cursor_motions_return_to_active_left_margin() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 10,
            rows: 5,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            "\x1b[?69h\x1b[3;8s\x1b[2;4r\x1b[?6h\x1b[1;4H\x1b[2EX\x1b[1FY",
        )?;
        let screen = engine.read_screen(session.id)?;

        assert_eq!(screen.lines, vec!["", "", "  Y", "  X"]);
        assert_eq!(screen.cursor.x, 3);
        assert_eq!(screen.cursor.y, 2);

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_keeps_alternate_screen_out_of_main_scrollback() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 2,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "main-one\nmain-two",
                "\x1b[?1049h",
                "alt-one\nalt-two\nalt-three",
                "\x1b[?1049l"
            ),
        )?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["main-one", "main-two"]);
        assert!(engine.search(session.id, "alt-three", 1)?.is_empty());
        assert!(engine.read_scrollback(session.id, 10)?.is_empty());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn screen_buffer_keeps_alternate_screen_until_all_active_modes_leave() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 20,
            rows: 3,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        set_output_for_test(
            session.id,
            concat!(
                "main",
                "\x1b[?1047h",
                "alt",
                "\x1b[?1049h",
                "clear",
                "\x1b[?1047l",
                "still-alt",
                "\x1b[?1049l",
                "back"
            ),
        )?;

        let screen = engine.read_screen(session.id)?;
        assert_eq!(screen.lines, vec!["mainback"]);
        assert!(engine.search(session.id, "still-alt", 1)?.is_empty());

        engine.destroy_session(session.id)?;
        Ok(())
    }

    #[test]
    fn recording_status_is_inactive_without_session() {
        let engine = NextCoreEngine;
        let status = engine.recording_status(123).unwrap();

        assert!(!status.enabled);
        assert_eq!(status.session_id, None);
        assert_eq!(status.started_at, None);
        assert_eq!(status.block_count, None);
        assert_eq!(status.bytes, None);
    }

    #[test]
    fn recording_lifecycle_taps_next_core_output() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let sessions_root = std::env::temp_dir().join(format!(
            "unterm-next-core-recording-{}-{suffix}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&sessions_root);
        std::fs::create_dir_all(&sessions_root)?;
        let project_dir = sessions_root.join("project");
        std::fs::create_dir_all(&project_dir)?;
        let previous_root = std::env::var("UNTERM_SESSIONS_ROOT").ok();
        std::env::set_var("UNTERM_SESSIONS_ROOT", &sessions_root);

        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 4,
            command_dir: Some(project_dir.display().to_string()),
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        let started = engine.start_recording(session.id)?;
        set_output_for_test(
            session.id,
            "\x1b[31mhello from next-core\x1b[0m token=super-secret-value\n",
        )?;
        let traces = engine.attach_recording_trace(session.id, "trace-1".to_string())?;
        let status = engine.recording_status(session.id)?;
        let stopped = engine.stop_recording(session.id)?;

        match previous_root {
            Some(value) => std::env::set_var("UNTERM_SESSIONS_ROOT", value),
            None => std::env::remove_var("UNTERM_SESSIONS_ROOT"),
        }

        assert_eq!(started.session_id, stopped.session_id);
        assert_eq!(traces, vec!["trace-1".to_string()]);
        assert!(status.enabled);
        assert!(status.block_count.unwrap_or_default() >= 1);
        assert!(std::fs::read_to_string(&started.log_path)?.contains("\tout\t"));
        let markdown = std::fs::read_to_string(&stopped.md_path)?;
        assert!(markdown.starts_with("---\n"));
        assert!(markdown.contains("unterm_session_id: "));
        assert!(markdown.contains("exit_reason: recording_stopped"));
        assert!(markdown.contains("ended_at: "));
        assert!(markdown.contains("osc133_active: false"));
        assert!(markdown.contains("block_render: chunked_output"));
        assert!(markdown.contains("## Output Blocks"));
        assert!(markdown.contains("### Block 1 `"));
        assert!(markdown.contains("## Aggregated Preview"));
        assert!(markdown.contains("trace_ids: [\"trace-1\"]"));
        assert!(markdown.contains("redaction_count: 1"));
        assert!(markdown.contains("hello from next-core [REDACTED]"));
        assert!(!markdown.contains("\x1b[31m"));
        assert!(!markdown.contains("super-secret-value"));
        assert!(std::fs::read_to_string(sessions_root.join("index.json"))?.contains("trace-1"));

        engine.destroy_session(session.id)?;
        let _ = std::fs::remove_dir_all(&sessions_root);
        Ok(())
    }

    #[test]
    fn recording_markdown_renders_osc133_command_blocks() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let sessions_root = std::env::temp_dir().join(format!(
            "unterm-next-core-osc133-recording-{}-{suffix}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&sessions_root);
        std::fs::create_dir_all(&sessions_root)?;
        let previous_root = std::env::var("UNTERM_SESSIONS_ROOT").ok();
        std::env::set_var("UNTERM_SESSIONS_ROOT", &sessions_root);

        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 4,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: Default::default(),
        })?;

        let started = engine.start_recording(session.id)?;
        set_output_for_test(
            session.id,
            concat!(
                "prompt> echo hi\r\n",
                "\x1b]133;C\x07",
                "command output token=super-secret-value\r\n",
                "\x1b]133;D;0\x07",
                "prompt> "
            ),
        )?;
        let stopped = engine.stop_recording(session.id)?;

        match previous_root {
            Some(value) => std::env::set_var("UNTERM_SESSIONS_ROOT", value),
            None => std::env::remove_var("UNTERM_SESSIONS_ROOT"),
        }

        assert_eq!(started.session_id, stopped.session_id);
        let markdown = std::fs::read_to_string(&stopped.md_path)?;
        assert!(markdown.contains("osc133_active: true"));
        assert!(markdown.contains("block_render: osc133_command_blocks"));
        assert!(markdown.contains("command_block_count: 1"));
        assert!(markdown.contains("## Command Blocks"));
        assert!(markdown.contains("### Command 1 `"));
        assert!(markdown.contains("exit_code: `0`"));
        assert!(markdown.contains("command output [REDACTED]"));
        assert!(!markdown.contains("OSC133 command markers are not available yet"));
        assert!(!markdown.contains("super-secret-value"));
        let index = std::fs::read_to_string(sessions_root.join("index.json"))?;
        assert!(index.contains("\"osc133_active\": true"));

        engine.destroy_session(session.id)?;
        let _ = std::fs::remove_dir_all(&sessions_root);
        Ok(())
    }
}
