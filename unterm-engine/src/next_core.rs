use super::{
    CellStyle, CreateSessionRequest, CursorSnapshot, DirtyRows, InputEngine, ScreenEngine,
    ScreenLine, ScreenSearchMatch, ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot,
    SessionActivitySnapshot, SessionEngine, SessionSnapshot, ShellSnapshot, SplitSessionRequest,
    StyledCell, StyledColor, StyledScreenLine, StyledScreenSnapshot,
};
use anyhow::{bail, Result};
use parking_lot::{Mutex, RwLock};
use portable_pty::{native_pty_system, Child, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;

const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_SCROLLBACK_LINES: usize = 10_000;

#[derive(Clone, Copy, Debug, Default)]
pub struct NextCoreEngine;

struct NextCoreSession {
    snapshot: SessionSnapshot,
    master: Mutex<Box<dyn MasterPty + Send>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output: Arc<Mutex<String>>,
    screen: Arc<Mutex<NextCoreScreen>>,
    dead: Arc<AtomicBool>,
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

#[derive(Default)]
struct NextCoreScreen {
    scrollback: Vec<Vec<ScreenCell>>,
    lines: Vec<Vec<ScreenCell>>,
    cursor_x: usize,
    cursor_y: usize,
    cursor_visible: bool,
    cursor_shape: String,
    bracketed_paste: bool,
    current_attr: CellAttributes,
    title: Option<String>,
    revision: u64,
    dirty_rows: Option<DirtyRows>,
    rows: usize,
    scroll_top: usize,
    scroll_bottom: usize,
    saved_cursor_x: usize,
    saved_cursor_y: usize,
    alternate: Option<ScreenState>,
}

#[derive(Default)]
struct ScreenState {
    scrollback: Vec<Vec<ScreenCell>>,
    lines: Vec<Vec<ScreenCell>>,
    cursor_x: usize,
    cursor_y: usize,
    cursor_visible: bool,
    cursor_shape: String,
    bracketed_paste: bool,
    current_attr: CellAttributes,
    scroll_top: usize,
    scroll_bottom: usize,
    saved_cursor_x: usize,
    saved_cursor_y: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ScreenCell {
    ch: char,
    attr: CellAttributes,
    width: usize,
}

impl ScreenCell {
    fn new(ch: char, attr: CellAttributes) -> Self {
        Self {
            ch,
            attr,
            width: Self::char_width(ch),
        }
    }

    fn blank(attr: CellAttributes) -> Self {
        Self {
            ch: ' ',
            attr,
            width: 1,
        }
    }

    fn continuation(attr: CellAttributes) -> Self {
        Self {
            ch: ' ',
            attr,
            width: 0,
        }
    }

    fn char_width(ch: char) -> usize {
        let mut buf = [0u8; 4];
        termwiz::cell::unicode_column_width(ch.encode_utf8(&mut buf), None)
    }

    #[allow(dead_code)]
    fn styled(&self) -> StyledCell {
        StyledCell {
            ch: self.ch,
            style: self.attr.into(),
            width: self.width,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CellAttributes {
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
    fg: Option<TerminalColor>,
    bg: Option<TerminalColor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalColor {
    Palette(u8),
    Rgb(u8, u8, u8),
}

impl From<CellAttributes> for CellStyle {
    fn from(attr: CellAttributes) -> Self {
        Self {
            bold: attr.bold,
            italic: attr.italic,
            underline: attr.underline,
            inverse: attr.inverse,
            fg: attr.fg.map(Into::into),
            bg: attr.bg.map(Into::into),
        }
    }
}

impl From<TerminalColor> for StyledColor {
    fn from(color: TerminalColor) -> Self {
        match color {
            TerminalColor::Palette(idx) => StyledColor::Palette(idx),
            TerminalColor::Rgb(r, g, b) => StyledColor::Rgb(r, g, b),
        }
    }
}

impl NextCoreScreen {
    fn new(rows: usize) -> Self {
        let mut screen = Self {
            rows: rows.max(1),
            cursor_visible: true,
            cursor_shape: "Default".to_string(),
            ..Self::default()
        };
        screen.scroll_bottom = screen.rows - 1;
        screen.ensure_cursor_line();
        screen
    }

    fn feed(&mut self, chunk: &str) {
        if !chunk.is_empty() {
            self.bump_revision();
            self.clear_dirty_rows();
        }
        let mut parser = ScreenParser::new(self);
        parser.feed(chunk);
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1).max(1);
    }

    fn clear_dirty_rows(&mut self) {
        self.dirty_rows = None;
    }

    fn mark_dirty_row(&mut self, row: usize) {
        let row = row.min(self.rows.saturating_sub(1));
        self.dirty_rows = Some(match self.dirty_rows {
            Some(dirty) => DirtyRows {
                start: dirty.start.min(row),
                end: dirty.end.max(row),
            },
            None => DirtyRows {
                start: row,
                end: row,
            },
        });
    }

    fn mark_dirty_range(&mut self, start: usize, end: usize) {
        if self.rows == 0 {
            return;
        }
        let start = start.min(self.rows - 1);
        let end = end.min(self.rows - 1);
        if start <= end {
            self.dirty_rows = Some(match self.dirty_rows {
                Some(dirty) => DirtyRows {
                    start: dirty.start.min(start),
                    end: dirty.end.max(end),
                },
                None => DirtyRows { start, end },
            });
        }
    }

    fn mark_all_dirty(&mut self) {
        if self.rows > 0 {
            self.mark_dirty_range(0, self.rows - 1);
        }
    }

    fn snapshot_lines(&self) -> Vec<String> {
        self.history_lines()
            .into_iter()
            .map(Self::line_text)
            .collect()
    }

    fn snapshot_viewport_lines(&self) -> Vec<String> {
        self.lines.iter().map(Self::line_text).collect()
    }

    #[allow(dead_code)]
    fn styled_viewport_lines(&self, first_row: i64) -> Vec<StyledScreenLine> {
        self.lines
            .iter()
            .enumerate()
            .map(|(idx, line)| StyledScreenLine {
                row: first_row + idx as i64,
                cells: line.iter().map(ScreenCell::styled).collect(),
            })
            .collect()
    }

    fn scrollback_rows(&self) -> usize {
        self.scrollback.len()
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn dirty_rows(&self) -> Option<DirtyRows> {
        self.dirty_rows
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

    fn history_lines(&self) -> Vec<&Vec<ScreenCell>> {
        self.scrollback.iter().chain(self.lines.iter()).collect()
    }

    fn history_len(&self) -> usize {
        self.scrollback.len() + self.lines.len()
    }

    fn history_text_range(&self, start: usize, count: usize) -> Vec<String> {
        let end = start.saturating_add(count).min(self.history_len());
        (start..end)
            .filter_map(|idx| {
                if idx < self.scrollback.len() {
                    self.scrollback.get(idx)
                } else {
                    self.lines.get(idx - self.scrollback.len())
                }
            })
            .map(Self::line_text)
            .collect()
    }

    fn line_text(line: &Vec<ScreenCell>) -> String {
        line.iter()
            .filter(|cell| cell.width > 0)
            .map(|cell| cell.ch)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn put_char(&mut self, c: char) {
        self.ensure_cursor_line();
        self.mark_dirty_row(self.cursor_y);
        let cell = ScreenCell::new(c, self.current_attr);
        let width = cell.width;
        if width == 0 {
            if self.cursor_x > 0 {
                let line = &mut self.lines[self.cursor_y];
                if let Some(previous) = line.get_mut(self.cursor_x - 1) {
                    previous.ch = c;
                }
            }
            return;
        }
        let line = &mut self.lines[self.cursor_y];
        if self.cursor_x > line.len() {
            line.resize(self.cursor_x, ScreenCell::blank(self.current_attr));
        }
        if self.cursor_x == line.len() {
            line.push(cell);
        } else {
            line[self.cursor_x] = cell;
        }
        if width > 1 {
            for offset in 1..width {
                let idx = self.cursor_x + offset;
                if idx == line.len() {
                    line.push(ScreenCell::continuation(self.current_attr));
                } else if idx < line.len() {
                    line[idx] = ScreenCell::continuation(self.current_attr);
                }
            }
        }
        self.cursor_x += width;
    }

    fn newline(&mut self) {
        let old_y = self.cursor_y;
        self.cursor_x = 0;
        if self.cursor_y >= self.scroll_bottom {
            self.scroll_up_region(self.scroll_top, self.scroll_bottom, 1);
        } else {
            self.cursor_y += 1;
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

    fn save_cursor(&mut self) {
        self.saved_cursor_x = self.cursor_x;
        self.saved_cursor_y = self.cursor_y;
    }

    fn restore_cursor(&mut self) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_x = self.saved_cursor_x;
        self.cursor_y = self.saved_cursor_y;
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

    fn set_bracketed_paste(&mut self, enabled: bool) {
        self.bracketed_paste = enabled;
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
        self.cursor_x = col;
        self.ensure_cursor_line();
        self.mark_dirty_row(self.cursor_y);
    }

    fn move_cursor_up(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_y = self.cursor_y.saturating_sub(count);
        self.mark_dirty_row(self.cursor_y);
    }

    fn move_cursor_down(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_y = (self.cursor_y + count).min(self.rows.saturating_sub(1));
        self.ensure_cursor_line();
        self.mark_dirty_row(self.cursor_y);
    }

    fn move_cursor_right(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_x += count;
    }

    fn move_cursor_left(&mut self, count: usize) {
        self.mark_dirty_row(self.cursor_y);
        self.cursor_x = self.cursor_x.saturating_sub(count);
    }

    fn clear_screen(&mut self) {
        self.lines.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.ensure_cursor_line();
        self.mark_all_dirty();
    }

    fn erase_in_display(&mut self, mode: usize) {
        match mode {
            0 => {
                self.erase_in_line(0);
                let start = self.cursor_y + 1;
                if start < self.lines.len() {
                    for line in self.lines.iter_mut().skip(start) {
                        line.clear();
                    }
                    self.mark_dirty_range(start, self.rows.saturating_sub(1));
                }
            }
            1 => {
                let end = self.cursor_y.min(self.lines.len().saturating_sub(1));
                for line in self.lines.iter_mut().take(end) {
                    line.clear();
                }
                self.erase_in_line(1);
                self.mark_dirty_range(0, self.cursor_y);
            }
            2 => self.clear_screen(),
            _ => {}
        }
    }

    fn erase_in_line(&mut self, mode: usize) {
        self.ensure_cursor_line();
        let line = &mut self.lines[self.cursor_y];
        match mode {
            0 => line.truncate(self.cursor_x),
            1 => {
                let end = self.cursor_x.saturating_add(1).min(line.len());
                for cell in line.iter_mut().take(end) {
                    *cell = ScreenCell::blank(self.current_attr);
                }
            }
            2 => line.clear(),
            _ => {}
        }
        self.mark_dirty_row(self.cursor_y);
    }

    fn insert_chars(&mut self, count: usize) {
        self.ensure_cursor_line();
        let line = &mut self.lines[self.cursor_y];
        if self.cursor_x > line.len() {
            line.resize(self.cursor_x, ScreenCell::blank(self.current_attr));
        }
        for _ in 0..count.max(1) {
            line.insert(self.cursor_x, ScreenCell::blank(self.current_attr));
        }
        self.mark_dirty_row(self.cursor_y);
    }

    fn delete_chars(&mut self, count: usize) {
        self.ensure_cursor_line();
        let line = &mut self.lines[self.cursor_y];
        for _ in 0..count.max(1) {
            if self.cursor_x < line.len() {
                line.remove(self.cursor_x);
            }
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
                3 => self.current_attr.italic = true,
                4 => self.current_attr.underline = true,
                7 => self.current_attr.inverse = true,
                22 => self.current_attr.bold = false,
                23 => self.current_attr.italic = false,
                24 => self.current_attr.underline = false,
                27 => self.current_attr.inverse = false,
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
                38 | 48 => {
                    let target_fg = params[idx] == 38;
                    if let Some((color, consumed)) = Self::parse_extended_color(&params[idx + 1..])
                    {
                        if target_fg {
                            self.current_attr.fg = Some(color);
                        } else {
                            self.current_attr.bg = Some(color);
                        }
                        idx += consumed;
                    }
                }
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

    fn insert_lines(&mut self, count: usize) {
        self.ensure_cursor_line();
        self.mark_dirty_range(self.cursor_y, self.rows.saturating_sub(1));
        for _ in 0..count.max(1) {
            self.lines.insert(self.cursor_y, Vec::new());
            if self.lines.len() > self.rows {
                self.lines.pop();
            }
        }
    }

    fn delete_lines(&mut self, count: usize) {
        self.ensure_cursor_line();
        self.mark_dirty_range(self.cursor_y, self.rows.saturating_sub(1));
        for _ in 0..count.max(1) {
            if self.cursor_y < self.lines.len() {
                self.lines.remove(self.cursor_y);
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
                self.scrollback.push(removed);
                if self.scrollback.len() > MAX_SCROLLBACK_LINES {
                    let overflow = self.scrollback.len() - MAX_SCROLLBACK_LINES;
                    self.scrollback.drain(..overflow);
                }
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
        self.set_cursor(0, 0);
        self.mark_all_dirty();
    }

    fn resize(&mut self, rows: usize) {
        self.rows = rows.max(1);
        self.bump_revision();
        self.clear_dirty_rows();
        self.mark_all_dirty();
        if self.lines.len() > self.rows {
            let trim = self.lines.len() - self.rows;
            let drained = self.lines.drain(..trim).collect::<Vec<_>>();
            if self.alternate.is_none() {
                self.scrollback.extend(drained);
                if self.scrollback.len() > MAX_SCROLLBACK_LINES {
                    let overflow = self.scrollback.len() - MAX_SCROLLBACK_LINES;
                    self.scrollback.drain(..overflow);
                }
            }
            self.cursor_y = self.cursor_y.saturating_sub(trim);
            self.saved_cursor_y = self.saved_cursor_y.saturating_sub(trim);
        }
        self.cursor_y = self.cursor_y.min(self.rows.saturating_sub(1));
        self.scroll_top = self.scroll_top.min(self.rows.saturating_sub(1));
        self.scroll_bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
        if self.scroll_top >= self.scroll_bottom {
            self.scroll_top = 0;
            self.scroll_bottom = self.rows.saturating_sub(1);
        }
        self.ensure_cursor_line();
    }

    fn enter_alternate_screen(&mut self, clear: bool) {
        if self.alternate.is_some() {
            if clear {
                self.clear_screen();
            }
            return;
        }

        let main = ScreenState {
            scrollback: std::mem::take(&mut self.scrollback),
            lines: std::mem::take(&mut self.lines),
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
            cursor_visible: self.cursor_visible,
            cursor_shape: self.cursor_shape.clone(),
            bracketed_paste: self.bracketed_paste,
            current_attr: self.current_attr,
            scroll_top: self.scroll_top,
            scroll_bottom: self.scroll_bottom,
            saved_cursor_x: self.saved_cursor_x,
            saved_cursor_y: self.saved_cursor_y,
        };
        self.alternate = Some(main);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.saved_cursor_x = 0;
        self.saved_cursor_y = 0;
        self.cursor_shape = "Default".to_string();
        self.bracketed_paste = false;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.lines.clear();
        self.ensure_cursor_line();
        self.mark_all_dirty();
    }

    fn leave_alternate_screen(&mut self) {
        if let Some(main) = self.alternate.take() {
            self.scrollback = main.scrollback;
            self.lines = main.lines;
            self.cursor_x = main.cursor_x;
            self.cursor_y = main.cursor_y;
            self.cursor_visible = main.cursor_visible;
            self.cursor_shape = main.cursor_shape;
            self.bracketed_paste = main.bracketed_paste;
            self.current_attr = main.current_attr;
            self.scroll_top = main.scroll_top;
            self.scroll_bottom = main.scroll_bottom;
            self.saved_cursor_x = main.saved_cursor_x;
            self.saved_cursor_y = main.saved_cursor_y;
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
}

struct ScreenParser<'a> {
    screen: &'a mut NextCoreScreen,
    state: ParserState,
}

enum ParserState {
    Ground,
    Escape,
    Csi(String),
    Osc(String),
    OscEscape(String),
}

impl<'a> ScreenParser<'a> {
    fn new(screen: &'a mut NextCoreScreen) -> Self {
        Self {
            screen,
            state: ParserState::Ground,
        }
    }

    fn feed(&mut self, chunk: &str) {
        for c in chunk.chars() {
            self.feed_char(c);
        }
    }

    fn feed_char(&mut self, c: char) {
        match self.state {
            ParserState::Ground => match c {
                '\x1b' => self.state = ParserState::Escape,
                '\r' => self.screen.carriage_return(),
                '\n' => self.screen.newline(),
                '\x08' => self.screen.backspace(),
                '\t' => {
                    let next_tab = ((self.screen.cursor_x / 8) + 1) * 8;
                    while self.screen.cursor_x < next_tab {
                        self.screen.put_char(' ');
                    }
                }
                c if !c.is_control() => self.screen.put_char(c),
                _ => {}
            },
            ParserState::Escape => match c {
                '[' => self.state = ParserState::Csi(String::new()),
                ']' => self.state = ParserState::Osc(String::new()),
                '7' => {
                    self.screen.save_cursor();
                    self.state = ParserState::Ground;
                }
                '8' => {
                    self.screen.restore_cursor();
                    self.state = ParserState::Ground;
                }
                _ => self.state = ParserState::Ground,
            },
            ParserState::Csi(ref mut sequence) => {
                if ('@'..='~').contains(&c) {
                    sequence.push(c);
                    let sequence = std::mem::take(sequence);
                    self.handle_csi(&sequence);
                    self.state = ParserState::Ground;
                } else {
                    sequence.push(c);
                }
            }
            ParserState::Osc(ref mut sequence) => match c {
                '\x07' => {
                    let sequence = std::mem::take(sequence);
                    self.screen.apply_osc(&sequence);
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
                    self.screen.apply_osc(&sequence);
                }
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_csi(&mut self, sequence: &str) {
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
            '@' => self.screen.insert_chars(first()),
            'A' => self.screen.move_cursor_up(first()),
            'B' => self.screen.move_cursor_down(first()),
            'C' => self.screen.move_cursor_right(first()),
            'D' => self.screen.move_cursor_left(first()),
            'L' => self.screen.insert_lines(first()),
            'M' => self.screen.delete_lines(first()),
            'P' => self.screen.delete_chars(first()),
            'S' => self.screen.scroll_up(first()),
            'T' => self.screen.scroll_down(first()),
            'G' => {
                self.screen.mark_dirty_row(self.screen.cursor_y);
                self.screen.cursor_x = first().saturating_sub(1);
            }
            'H' | 'f' => {
                let row = numbers.first().copied().filter(|n| *n > 0).unwrap_or(1);
                let col = numbers.get(1).copied().filter(|n| *n > 0).unwrap_or(1);
                self.screen
                    .set_cursor(row.saturating_sub(1), col.saturating_sub(1));
            }
            'J' => self
                .screen
                .erase_in_display(numbers.first().copied().unwrap_or(0)),
            'K' => self
                .screen
                .erase_in_line(numbers.first().copied().unwrap_or(0)),
            'm' => self.screen.apply_sgr(&numbers),
            'q' => {
                if raw_params.ends_with(' ') {
                    self.screen
                        .set_cursor_shape(numbers.first().copied().unwrap_or(0));
                }
            }
            'r' => {
                let top = numbers.first().copied().filter(|n| *n > 0).unwrap_or(1);
                let bottom = numbers
                    .get(1)
                    .copied()
                    .filter(|n| *n > 0)
                    .unwrap_or(self.screen.rows);
                self.screen
                    .set_scroll_region(top.saturating_sub(1), bottom.saturating_sub(1));
            }
            'h' => {
                if private && numbers.iter().any(|n| matches!(*n, 1049 | 1047 | 47)) {
                    self.screen.enter_alternate_screen(true);
                } else if private && numbers.iter().any(|n| *n == 25) {
                    self.screen.cursor_visible = true;
                    self.screen.mark_dirty_row(self.screen.cursor_y);
                } else if private && numbers.iter().any(|n| *n == 2004) {
                    self.screen.set_bracketed_paste(true);
                }
            }
            'l' => {
                if private && numbers.iter().any(|n| matches!(*n, 1049 | 1047 | 47)) {
                    self.screen.leave_alternate_screen();
                } else if private && numbers.iter().any(|n| *n == 25) {
                    self.screen.cursor_visible = false;
                    self.screen.mark_dirty_row(self.screen.cursor_y);
                } else if private && numbers.iter().any(|n| *n == 2004) {
                    self.screen.set_bracketed_paste(false);
                }
            }
            _ => {}
        }
    }
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
    let (output, screen, rows) = {
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
            session.snapshot.rows,
        )
    };
    *output.lock() = text.to_string();
    let mut screen = screen.lock();
    let revision = screen.revision();
    *screen = NextCoreScreen::new(rows);
    screen.revision = revision;
    screen.feed(text);
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

    session.dead.store(true, Ordering::Release);
    Ok(())
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
    fn refresh_liveness(session: &mut NextCoreSession) {
        if session.snapshot.is_dead {
            return;
        }

        if session.dead.load(Ordering::Acquire) {
            session.snapshot.is_dead = true;
            return;
        }

        if matches!(session.child.lock().try_wait(), Ok(Some(_))) {
            session.snapshot.is_dead = true;
            session.dead.store(true, Ordering::Release);
        }
    }

    fn sessions(&self) -> Vec<SessionSnapshot> {
        state()
            .write()
            .sessions
            .iter_mut()
            .map(|session| {
                Self::refresh_liveness(session);
                let mut snapshot = session.snapshot.clone();
                let screen = session.screen.lock();
                snapshot.cursor = screen.cursor_snapshot();
                snapshot.scrollback_rows = screen.scrollback_rows();
                if let Some(title) = screen.title() {
                    snapshot.title = title;
                }
                snapshot
            })
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

    #[doc(hidden)]
    pub fn debug_output(&self, pane_id: usize) -> Result<String> {
        self.output(pane_id)
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

    fn viewport_lines(&self, pane_id: usize) -> Result<Vec<String>> {
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

        let lines = screen.lock().snapshot_viewport_lines();
        Ok(lines)
    }

    #[allow(dead_code)]
    fn styled_viewport_lines(
        &self,
        pane_id: usize,
        first_row: i64,
    ) -> Result<Vec<StyledScreenLine>> {
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

        let lines = screen.lock().styled_viewport_lines(first_row);
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
            .scrollback
            .iter()
            .map(NextCoreScreen::line_text)
            .collect();
        Ok(lines)
    }

    fn scrollback_rows(&self, pane_id: usize) -> Result<usize> {
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

        let rows = screen.lock().scrollback_rows();
        Ok(rows)
    }

    fn screen_revision(&self, pane_id: usize) -> Result<u64> {
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

        let revision = screen.lock().revision();
        Ok(revision)
    }

    fn screen_dirty_rows(&self, pane_id: usize) -> Result<Option<DirtyRows>> {
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

        let dirty_rows = screen.lock().dirty_rows();
        Ok(dirty_rows)
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
        let child = pair.slave.spawn_command(command)?;
        let reader = pair.master.try_clone_reader()?;
        let writer = Arc::new(Mutex::new(pair.master.take_writer()?));
        let output = Arc::new(Mutex::new(String::new()));
        let screen = Arc::new(Mutex::new(NextCoreScreen::new(rows)));
        let dead = Arc::new(AtomicBool::new(false));
        Self::spawn_reader_thread(
            id,
            Arc::clone(&output),
            Arc::clone(&screen),
            Arc::clone(&writer),
            Arc::clone(&dead),
            reader,
        );
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
            screen,
            dead,
        })
    }

    fn spawn_reader_thread(
        pane_id: usize,
        output: Arc<Mutex<String>>,
        screen: Arc<Mutex<NextCoreScreen>>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        dead: Arc<AtomicBool>,
        mut reader: Box<dyn Read + Send>,
    ) {
        thread::Builder::new()
            .name(format!("next-core-pty-reader-{pane_id}"))
            .spawn(move || {
                let mut buf = [0u8; 8192];
                let mut pending_utf8 = Vec::new();
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
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
                            Self::answer_terminal_queries(chunk.as_str(), &screen, &writer);
                        }
                        Err(_) => break,
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

    fn answer_terminal_queries(
        chunk: &str,
        screen: &NextCoreScreen,
        writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    ) {
        let mut response = Vec::new();
        if chunk.contains("\x1b[6n") {
            response.extend_from_slice(
                format!("\x1b[{};{}R", screen.cursor_y + 1, screen.cursor_x + 1).as_bytes(),
            );
        }
        if chunk.contains("\x1b[c") {
            response.extend_from_slice(b"\x1b[?64;1;2;6;9;15;18;21;22c");
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

    #[allow(dead_code)]
    fn paste_payload(text: &str, bracketed: bool) -> String {
        if bracketed {
            format!("\x1b[200~{text}\x1b[201~")
        } else {
            text.to_string()
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
        let session = self.session(pane_id)?;
        let foreground_process = session.shell.process_name;
        Ok(SessionActivitySnapshot {
            idle: session.is_dead
                || foreground_process.is_empty()
                || foreground_process == "unknown",
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
        session.screen.lock().resize(rows);
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
        session.dead.store(true, Ordering::Release);
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
        let visible = self.viewport_lines(pane_id)?;
        let scrollback_rows = self.scrollback_rows(pane_id)?;
        let first_row = scrollback_rows as i64;
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
            cursor: self.screen_cursor(pane_id)?,
            cols: session.cols,
            rows: session.rows,
            scrollback_rows,
            revision: self.screen_revision(pane_id)?,
            dirty_rows: self.screen_dirty_rows(pane_id)?,
        })
    }

    fn read_styled_screen(&self, pane_id: usize) -> Result<StyledScreenSnapshot> {
        let session = self.session(pane_id)?;
        let scrollback_rows = self.scrollback_rows(pane_id)?;
        let first_row = scrollback_rows as i64;

        Ok(StyledScreenSnapshot {
            lines: self.styled_viewport_lines(pane_id, first_row)?,
            cursor: self.screen_cursor(pane_id)?,
            cols: session.cols,
            rows: session.rows,
            scrollback_rows,
            revision: self.screen_revision(pane_id)?,
            dirty_rows: self.screen_dirty_rows(pane_id)?,
        })
    }

    fn read_visible_text(&self, pane_id: usize) -> Result<String> {
        Ok(self.read_screen(pane_id)?.lines.join("\n"))
    }

    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
        let start = start.max(0) as usize;
        Ok(self
            .screen_line_text_range(pane_id, start, count)?
            .into_iter()
            .enumerate()
            .map(|(idx, text)| ScreenLine {
                row: (start + idx) as i64,
                text,
            })
            .collect())
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>> {
        let lines = self
            .scrollback_lines(pane_id)?
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
        Ok(ScrollbackTextSnapshot {
            text: selected.join("\n"),
            lines: selected,
            first_row: start as i64,
            row_count: end.saturating_sub(start) as i64,
            cols: session.cols,
            escapes: request.escapes,
            scrollback_top: 0,
            physical_top: line_count.saturating_sub(session.rows) as i64,
            viewport_rows: session.rows,
        })
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        let lines = self.screen_lines(pane_id)?;
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
        self.screen_cursor(pane_id)
    }
}

impl InputEngine for NextCoreEngine {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()> {
        let writer = {
            let state = state().read();
            let Some(session) = state
                .sessions
                .iter()
                .find(|session| session.snapshot.id == pane_id)
            else {
                bail!("next-core session {pane_id} not found");
            };
            Arc::clone(&session.writer)
        };

        let mut writer = writer.lock();
        writer.write_all(input.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    fn paste_input(&self, pane_id: usize, text: &str) -> Result<()> {
        let input = Self::paste_payload(text, self.bracketed_paste_enabled(pane_id)?);
        self.write_input(pane_id, &input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn answers_cursor_position_queries_from_screen_state() {
        let _guard = test_guard();
        let mut screen = NextCoreScreen::new(10);
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
    fn answers_primary_device_attributes_with_xterm_capabilities() {
        let _guard = test_guard();
        let screen = NextCoreScreen::new(10);
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(SharedWriter {
                bytes: Arc::clone(&bytes),
            })));

        NextCoreEngine::answer_terminal_queries("\x1b[c", &screen, &writer);

        assert_eq!(bytes.lock().as_slice(), b"\x1b[?64;1;2;6;9;15;18;21;22c");
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
            command: None,
        })?;

        mark_dead_for_test(session.id)?;
        assert!(engine.get_session(session.id)?.is_dead);
        assert!(engine.activity(session.id)?.idle);
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
    fn screen_buffer_strips_terminal_control_sequences() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
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
    fn screen_buffer_tracks_sgr_cell_attributes() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
        })?;
        set_output_for_test(
            session.id,
            concat!(
                "\x1b[1;31mR",
                "\x1b[0mN",
                "\x1b[3;4;7;38;5;202;48;2;1;2;3mX",
                "\x1b[22;23;24;27;39;49mY"
            ),
        )?;

        assert_eq!(engine.read_visible_text(session.id)?, "RNXY");
        let attrs = viewport_attrs_for_test(session.id)?;
        let line = &attrs[0];

        assert!(line[0].bold);
        assert_eq!(line[0].fg, Some(TerminalColor::Palette(1)));
        assert_eq!(line[0].bg, None);

        assert_eq!(line[1], CellAttributes::default());

        assert!(line[2].italic);
        assert!(line[2].underline);
        assert!(line[2].inverse);
        assert_eq!(line[2].fg, Some(TerminalColor::Palette(202)));
        assert_eq!(line[2].bg, Some(TerminalColor::Rgb(1, 2, 3)));

        assert_eq!(line[3], CellAttributes::default());

        let styled = engine.read_styled_screen(session.id)?;
        assert_eq!(styled.lines[0].row, 0);
        assert_eq!(styled.lines[0].cells[0].ch, 'R');
        assert!(styled.lines[0].cells[0].style.bold);
        assert_eq!(
            styled.lines[0].cells[0].style.fg,
            Some(StyledColor::Palette(1))
        );
        assert_eq!(
            styled.lines[0].cells[2].style.bg,
            Some(StyledColor::Rgb(1, 2, 3))
        );
        assert_eq!(styled.lines[0].cells[3].style, CellStyle::default());

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
    fn screen_buffer_applies_basic_csi_screen_operations() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
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
            rows: 5,
            command_dir: None,
            command: None,
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
                "\x1b[2K"
            ),
        )?;

        let lines = engine.read_screen(session.id)?.lines;
        assert_eq!(lines, vec!["abXYe", "keep", "prefix", "      left", ""]);

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
    fn screen_buffer_reports_cursor_state() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 3,
            command_dir: None,
            command: None,
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
    fn screen_buffer_handles_alternate_screen_and_line_mutations() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 24,
            command_dir: None,
            command: None,
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
    fn screen_buffer_keeps_alternate_screen_out_of_main_scrollback() -> Result<()> {
        let _guard = test_guard();
        reset_state_for_test();
        let engine = NextCoreEngine;
        let session = engine.create_session(CreateSessionRequest {
            cols: 80,
            rows: 2,
            command_dir: None,
            command: None,
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
}
