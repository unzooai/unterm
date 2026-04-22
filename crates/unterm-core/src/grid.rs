use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

/// ANSI 颜色
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TermColor {
    /// 默认前景/背景
    Default,
    /// 标准 16 色 (0-15)
    Indexed(u8),
    /// 24-bit 真彩色
    Rgb(u8, u8, u8),
}

impl Default for TermColor {
    fn default() -> Self {
        Self::Default
    }
}

/// 字符属性
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CellAttrs {
    pub fg: TermColor,
    pub bg: TermColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub inverse: bool,
}

/// 单个终端 cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    /// UTF-8 字符（可能为空表示空格）
    pub ch: char,
    /// 显示属性
    pub attrs: CellAttrs,
    /// 宽字符占位标记（CJK 字符的第二列）
    pub is_wide_continuation: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            attrs: CellAttrs::default(),
            is_wide_continuation: false,
        }
    }
}

/// 光标状态
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Cursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

/// 终端网格
pub struct Grid {
    /// 列数
    cols: u16,
    /// 行数
    rows: u16,
    /// 行优先存储 [row][col]
    cells: Vec<Vec<Cell>>,
    /// 光标
    pub cursor: Cursor,
    /// 滚动回看缓冲区（已滚出屏幕的行，VecDeque 保证头部删除 O(1)）
    scrollback: VecDeque<Vec<Cell>>,
    /// 最大滚动回看行数
    max_scrollback: usize,
    /// 当前属性（新字符继承）
    current_attrs: CellAttrs,
    /// 滚动区域 (top, bottom)，默认 (0, rows-1)
    scroll_region: (u16, u16),
    /// 需要写回 PTY 的响应数据（如 DSR 响应）
    pub pending_responses: Vec<u8>,
}

impl Grid {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cells = vec![vec![Cell::default(); cols as usize]; rows as usize];
        Self {
            cols,
            rows,
            cells,
            cursor: Cursor {
                row: 0,
                col: 0,
                visible: true,
            },
            scrollback: VecDeque::new(),
            max_scrollback: 100000,
            current_attrs: CellAttrs::default(),
            scroll_region: (0, rows.saturating_sub(1)),
            pending_responses: Vec::new(),
        }
    }

    /// 调整网格大小
    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        let mut new_cells = vec![vec![Cell::default(); new_cols as usize]; new_rows as usize];
        let copy_rows = self.rows.min(new_rows) as usize;
        let copy_cols = self.cols.min(new_cols) as usize;
        for r in 0..copy_rows {
            for c in 0..copy_cols {
                new_cells[r][c] = self.cells[r][c].clone();
            }
        }
        self.cells = new_cells;
        self.cols = new_cols;
        self.rows = new_rows;
        self.scroll_region = (0, new_rows.saturating_sub(1));
        self.cursor.row = self.cursor.row.min(new_rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(new_cols.saturating_sub(1));
    }

    /// 在光标位置写入字符
    pub fn put_char(&mut self, ch: char) {
        let col = self.cursor.col as usize;
        let row = self.cursor.row as usize;
        if row < self.rows as usize && col < self.cols as usize {
            self.cells[row][col] = Cell {
                ch,
                attrs: self.current_attrs,
                is_wide_continuation: false,
            };
            // 宽字符处理
            let width = unicode_width(ch);
            if width == 2 && col + 1 < self.cols as usize {
                self.cells[row][col + 1] = Cell {
                    ch: ' ',
                    attrs: self.current_attrs,
                    is_wide_continuation: true,
                };
                self.cursor.col += 2;
            } else {
                self.cursor.col += 1;
            }
            // 自动换行
            if self.cursor.col >= self.cols {
                self.cursor.col = 0;
                self.line_feed();
            }
        }
    }

    /// 换行（光标下移，超出时滚动）
    pub fn line_feed(&mut self) {
        if self.cursor.row >= self.scroll_region.1 {
            self.scroll_up(1);
        } else {
            self.cursor.row += 1;
        }
    }

    /// 回车
    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    /// 向上滚动 n 行
    pub fn scroll_up(&mut self, n: u16) {
        let top = self.scroll_region.0 as usize;
        let bot = self.scroll_region.1 as usize;
        let n = n as usize;
        for _ in 0..n {
            if top < self.cells.len() && top <= bot {
                // 保存顶行到 scrollback
                let row = std::mem::replace(
                    &mut self.cells[top],
                    vec![Cell::default(); self.cols as usize],
                );
                if self.scrollback.len() >= self.max_scrollback {
                    self.scrollback.pop_front();
                }
                self.scrollback.push_back(row);
                // 将 top..bot 范围内的行上移一行（用 rotate_left 代替 remove+insert）
                self.cells[top..=bot].rotate_left(1);
            }
        }
    }

    /// 清屏
    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
        self.cursor = Cursor {
            row: 0,
            col: 0,
            visible: true,
        };
    }

    /// 擦除行内容
    pub fn erase_in_line(&mut self, mode: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row >= self.cells.len() {
            return;
        }
        match mode {
            0 => {
                // 光标到行尾
                for c in col..self.cols as usize {
                    self.cells[row][c] = Cell::default();
                }
            }
            1 => {
                // 行首到光标
                for c in 0..=col.min(self.cols as usize - 1) {
                    self.cells[row][c] = Cell::default();
                }
            }
            2 => {
                // 整行
                for cell in &mut self.cells[row] {
                    *cell = Cell::default();
                }
            }
            _ => {}
        }
    }

    /// 擦除显示区域
    pub fn erase_in_display(&mut self, mode: u16) {
        match mode {
            0 => {
                // 光标到屏幕末尾
                self.erase_in_line(0);
                for r in (self.cursor.row + 1) as usize..self.rows as usize {
                    for cell in &mut self.cells[r] {
                        *cell = Cell::default();
                    }
                }
            }
            1 => {
                // 屏幕开头到光标
                for r in 0..self.cursor.row as usize {
                    for cell in &mut self.cells[r] {
                        *cell = Cell::default();
                    }
                }
                self.erase_in_line(1);
            }
            2 | 3 => {
                // 整个屏幕
                self.clear();
            }
            _ => {}
        }
    }

    /// 向下滚动 n 行（在滚动区域内，底部插入空行，顶部行溢出）
    pub fn scroll_down(&mut self, n: u16) {
        let top = self.scroll_region.0 as usize;
        let bot = self.scroll_region.1 as usize;
        let n = (n as usize).min(bot - top + 1);
        for _ in 0..n {
            if top <= bot && bot < self.cells.len() {
                // 底行丢弃，插入空行到顶部
                self.cells[top..=bot].rotate_right(1);
                self.cells[top] = vec![Cell::default(); self.cols as usize];
            }
        }
    }

    /// IL — 在光标所在行插入 n 个空行（光标行及以下下移）
    pub fn insert_lines(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let bot = self.scroll_region.1 as usize;
        if row > bot || row < self.scroll_region.0 as usize {
            return;
        }
        let n = (n as usize).min(bot - row + 1);
        for _ in 0..n {
            self.cells[row..=bot].rotate_right(1);
            self.cells[row] = vec![Cell::default(); self.cols as usize];
        }
    }

    /// DL — 删除光标所在行起的 n 行（下方行上移，底部补空行）
    pub fn delete_lines(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let bot = self.scroll_region.1 as usize;
        if row > bot || row < self.scroll_region.0 as usize {
            return;
        }
        let n = (n as usize).min(bot - row + 1);
        for _ in 0..n {
            self.cells[row..=bot].rotate_left(1);
            self.cells[bot] = vec![Cell::default(); self.cols as usize];
        }
    }

    /// ICH — 在光标位置插入 n 个空字符（右侧字符右移，溢出丢弃）
    pub fn insert_chars(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        let cols = self.cols as usize;
        if row >= self.cells.len() || col >= cols {
            return;
        }
        let n = (n as usize).min(cols - col);
        // 从右往左移动
        for c in (col + n..cols).rev() {
            self.cells[row][c] = self.cells[row][c - n].clone();
        }
        // 插入空字符
        for c in col..col + n {
            self.cells[row][c] = Cell::default();
        }
    }

    /// DCH — 删除光标位置起 n 个字符（右侧字符左移，右端补空）
    pub fn delete_chars(&mut self, n: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        let cols = self.cols as usize;
        if row >= self.cells.len() || col >= cols {
            return;
        }
        let n = (n as usize).min(cols - col);
        // 从左往右移动
        for c in col..cols - n {
            self.cells[row][c] = self.cells[row][c + n].clone();
        }
        // 右端补空
        for c in cols - n..cols {
            self.cells[row][c] = Cell::default();
        }
    }

    /// 设置当前属性
    pub fn set_attrs(&mut self, attrs: CellAttrs) {
        self.current_attrs = attrs;
    }

    /// 重置属性
    pub fn reset_attrs(&mut self) {
        self.current_attrs = CellAttrs::default();
    }

    /// 设置前景色
    pub fn set_fg(&mut self, color: TermColor) {
        self.current_attrs.fg = color;
    }

    /// 设置背景色
    pub fn set_bg(&mut self, color: TermColor) {
        self.current_attrs.bg = color;
    }

    /// 获取可见行数据（用于序列化传输到 UI）
    pub fn visible_rows(&self) -> &[Vec<Cell>] {
        &self.cells
    }

    /// 获取 scrollback 缓冲区
    pub fn scrollback(&self) -> &VecDeque<Vec<Cell>> {
        &self.scrollback
    }

    /// scrollback 行数
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// 获取列数
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// 获取行数
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// 设置滚动区域
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        self.scroll_region = (top, bottom.min(self.rows.saturating_sub(1)));
    }

    /// 获取当前属性（只读）
    pub fn current_attrs(&self) -> CellAttrs {
        self.current_attrs
    }

    /// 获取当前属性（可变引用）
    pub fn current_attrs_mut(&mut self) -> &mut CellAttrs {
        &mut self.current_attrs
    }
}

/// 简易 Unicode 宽度判断（CJK 字符返回 2）
fn unicode_width(ch: char) -> u8 {
    let cp = ch as u32;
    if (0x1100..=0x115F).contains(&cp) // Hangul Jamo
        || (0x2E80..=0x303E).contains(&cp) // CJK
        || (0x3040..=0x33BF).contains(&cp) // Hiragana/Katakana
        || (0x3400..=0x4DBF).contains(&cp) // CJK Unified Ext A
        || (0x4E00..=0x9FFF).contains(&cp) // CJK Unified
        || (0xA000..=0xA4CF).contains(&cp) // Yi
        || (0xAC00..=0xD7AF).contains(&cp) // Hangul Syllables
        || (0xF900..=0xFAFF).contains(&cp) // CJK Compat
        || (0xFE30..=0xFE6F).contains(&cp) // CJK Forms
        || (0xFF01..=0xFF60).contains(&cp) // Fullwidth
        || (0xFFE0..=0xFFE6).contains(&cp) // Fullwidth Signs
        || (0x20000..=0x2FFFF).contains(&cp) // CJK Ext B-F
        || (0x30000..=0x3FFFF).contains(&cp) // CJK Ext G-I
    {
        2
    } else {
        1
    }
}
