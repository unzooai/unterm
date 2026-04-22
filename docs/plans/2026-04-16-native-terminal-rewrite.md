# Unterm 原生终端重写实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 unterm-ui 从"原始文本显示"升级为完整的 GPU 加速终端模拟器，支持 ANSI 颜色/光标、图片剪贴板、鼠标交互。

**Architecture:** 在 unterm-core 中引入 `vte` crate 做 VT 解析，构建终端网格（grid）数据结构，通过 IPC 传输结构化的 cell 数据（而非原始文本）。unterm-ui 的渲染层从"纯文本 glyphon buffer"升级为"逐 cell 着色渲染"。剪贴板使用 `arboard` 支持文本+图片。

**Tech Stack:** Rust, wgpu, winit, glyphon, vte 0.15, arboard 3, image 0.25

---

## 并行任务拓扑

```
Task 1: 终端网格数据结构 (unterm-core)     ──┐
Task 2: VTE 解析器集成 (unterm-core)        ──┤──→ Task 5: 结构化屏幕传输
Task 3: 剪贴板模块 (unterm-ui)              ──┤──→ Task 6: 主循环集成
Task 4: 鼠标输入 (unterm-ui)                ──┘──→ Task 7: 渲染管线升级
```

**可并行的任务**: Task 1-4 完全独立，可同时开发
**依赖任务**: Task 5 依赖 1+2, Task 6 依赖 3+4+5, Task 7 依赖 5

---

## Task 1: 终端网格数据结构

**目标**: 创建 `Cell` + `Grid` 数据结构，表示终端的字符网格和属性。

**Files:**
- Create: `crates/unterm-core/src/grid.rs`
- Modify: `crates/unterm-core/src/lib.rs` (或 `main.rs` 添加 `mod grid;`)
- Modify: `crates/unterm-proto/src/screen.rs` (添加序列化类型)

**实现:**

```rust
// crates/unterm-core/src/grid.rs

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
    fn default() -> Self { Self::Default }
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
    /// 滚动回看缓冲区（已滚出屏幕的行）
    scrollback: Vec<Vec<Cell>>,
    /// 最大滚动回看行数
    max_scrollback: usize,
    /// 当前属性（新字符继承）
    current_attrs: CellAttrs,
    /// 滚动区域 (top, bottom)，默认 (0, rows-1)
    scroll_region: (u16, u16),
}

impl Grid {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cells = vec![vec![Cell::default(); cols as usize]; rows as usize];
        Self {
            cols,
            rows,
            cells,
            cursor: Cursor { row: 0, col: 0, visible: true },
            scrollback: Vec::new(),
            max_scrollback: 10000,
            current_attrs: CellAttrs::default(),
            scroll_region: (0, rows.saturating_sub(1)),
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
            if top < self.cells.len() {
                let row = self.cells.remove(top);
                if self.scrollback.len() >= self.max_scrollback {
                    self.scrollback.remove(0);
                }
                self.scrollback.push(row);
                self.cells.insert(bot, vec![Cell::default(); self.cols as usize]);
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
        self.cursor = Cursor { row: 0, col: 0, visible: true };
    }

    /// 擦除行内容
    pub fn erase_in_line(&mut self, mode: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row >= self.cells.len() { return; }
        match mode {
            0 => { // 光标到行尾
                for c in col..self.cols as usize {
                    self.cells[row][c] = Cell::default();
                }
            }
            1 => { // 行首到光标
                for c in 0..=col.min(self.cols as usize - 1) {
                    self.cells[row][c] = Cell::default();
                }
            }
            2 => { // 整行
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
            0 => { // 光标到屏幕末尾
                self.erase_in_line(0);
                for r in (self.cursor.row + 1) as usize..self.rows as usize {
                    for cell in &mut self.cells[r] { *cell = Cell::default(); }
                }
            }
            1 => { // 屏幕开头到光标
                for r in 0..self.cursor.row as usize {
                    for cell in &mut self.cells[r] { *cell = Cell::default(); }
                }
                self.erase_in_line(1);
            }
            2 | 3 => { // 整个屏幕
                self.clear();
            }
            _ => {}
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

    pub fn cols(&self) -> u16 { self.cols }
    pub fn rows(&self) -> u16 { self.rows }
}

/// 简易 Unicode 宽度判断（CJK 字符返回 2）
fn unicode_width(ch: char) -> u8 {
    let cp = ch as u32;
    if (0x1100..=0x115F).contains(&cp)    // Hangul Jamo
        || (0x2E80..=0x303E).contains(&cp)   // CJK
        || (0x3040..=0x33BF).contains(&cp)   // Hiragana/Katakana
        || (0x3400..=0x4DBF).contains(&cp)   // CJK Unified Ext A
        || (0x4E00..=0x9FFF).contains(&cp)   // CJK Unified
        || (0xA000..=0xA4CF).contains(&cp)   // Yi
        || (0xAC00..=0xD7AF).contains(&cp)   // Hangul Syllables
        || (0xF900..=0xFAFF).contains(&cp)   // CJK Compat
        || (0xFE30..=0xFE6F).contains(&cp)   // CJK Forms
        || (0xFF01..=0xFF60).contains(&cp)   // Fullwidth
        || (0xFFE0..=0xFFE6).contains(&cp)   // Fullwidth Signs
        || (0x20000..=0x2FFFF).contains(&cp) // CJK Ext B-F
        || (0x30000..=0x3FFFF).contains(&cp) // CJK Ext G-I
    {
        2
    } else {
        1
    }
}
```

---

## Task 2: VTE 解析器集成

**目标**: 用 `vte` crate 解析 PTY 输出的 ANSI 转义序列，驱动 Grid 状态变化。

**Files:**
- Modify: `crates/unterm-core/Cargo.toml` (添加 `vte = "0.15"`)
- Create: `crates/unterm-core/src/term.rs` (VTE handler + Terminal 封装)
- Modify: `crates/unterm-core/src/session/mod.rs` (用 Terminal 替代 output_buffer)

**实现:**

```rust
// crates/unterm-core/src/term.rs

use crate::grid::{Grid, TermColor, CellAttrs};
use vte::{Params, Parser, Perform};

/// 终端状态机 — 封装 Grid + VTE Parser
pub struct Terminal {
    pub grid: Grid,
    parser: Parser,
    /// 保存的光标位置 (DECSC/DECRC)
    saved_cursor: (u16, u16),
    saved_attrs: CellAttrs,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            grid: Grid::new(cols, rows),
            parser: Parser::new(),
            saved_cursor: (0, 0),
            saved_attrs: CellAttrs::default(),
        }
    }

    /// 处理 PTY 输出字节流
    pub fn process(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.parser.advance(self, byte);
        }
    }

    /// 调整终端大小
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.grid.resize(cols, rows);
    }
}

impl Perform for Terminal {
    /// 可打印字符
    fn print(&mut self, ch: char) {
        self.grid.put_char(ch);
    }

    /// 执行 C0/C1 控制字符
    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0B | 0x0C => self.grid.line_feed(),
            b'\r' => self.grid.carriage_return(),
            b'\x08' => { // Backspace
                if self.grid.cursor.col > 0 {
                    self.grid.cursor.col -= 1;
                }
            }
            b'\x07' => {} // Bell - 可以触发通知
            b'\t' => { // Tab - 跳到下一个 8 的倍数
                let next = (self.grid.cursor.col / 8 + 1) * 8;
                self.grid.cursor.col = next.min(self.grid.cols() - 1);
            }
            _ => {}
        }
    }

    /// CSI 序列
    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        let p: Vec<u16> = params.iter().map(|sub| sub.first().copied().unwrap_or(0)).collect();

        match action {
            // 光标移动
            'A' => { // CUU - 上
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.cursor.row = self.grid.cursor.row.saturating_sub(n);
            }
            'B' => { // CUD - 下
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.cursor.row = (self.grid.cursor.row + n).min(self.grid.rows() - 1);
            }
            'C' => { // CUF - 右
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.cursor.col = (self.grid.cursor.col + n).min(self.grid.cols() - 1);
            }
            'D' => { // CUB - 左
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.cursor.col = self.grid.cursor.col.saturating_sub(n);
            }
            'H' | 'f' => { // CUP - 光标定位
                let row = p.first().copied().unwrap_or(1).max(1) - 1;
                let col = p.get(1).copied().unwrap_or(1).max(1) - 1;
                self.grid.cursor.row = row.min(self.grid.rows() - 1);
                self.grid.cursor.col = col.min(self.grid.cols() - 1);
            }
            'J' => { // ED - 擦除显示
                let mode = p.first().copied().unwrap_or(0);
                self.grid.erase_in_display(mode);
            }
            'K' => { // EL - 擦除行
                let mode = p.first().copied().unwrap_or(0);
                self.grid.erase_in_line(mode);
            }
            'm' => { // SGR - 属性设置
                self.handle_sgr(&p);
            }
            'r' => { // DECSTBM - 设置滚动区域
                let top = p.first().copied().unwrap_or(1).max(1) - 1;
                let bot = p.get(1).copied().unwrap_or(self.grid.rows()).max(1) - 1;
                self.grid.set_scroll_region(top, bot);
                self.grid.cursor.row = 0;
                self.grid.cursor.col = 0;
            }
            'h' => { // SM - 模式设置（DECTCEM 等）
                // ?25h = 显示光标
                if _intermediates.contains(&b'?') {
                    for &param in &p {
                        if param == 25 { self.grid.cursor.visible = true; }
                    }
                }
            }
            'l' => { // RM - 模式重置
                // ?25l = 隐藏光标
                if _intermediates.contains(&b'?') {
                    for &param in &p {
                        if param == 25 { self.grid.cursor.visible = false; }
                    }
                }
            }
            'S' => { // SU - 向上滚动
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.scroll_up(n);
            }
            'd' => { // VPA - 行绝对定位
                let row = p.first().copied().unwrap_or(1).max(1) - 1;
                self.grid.cursor.row = row.min(self.grid.rows() - 1);
            }
            'G' => { // CHA - 列绝对定位
                let col = p.first().copied().unwrap_or(1).max(1) - 1;
                self.grid.cursor.col = col.min(self.grid.cols() - 1);
            }
            'L' => { // IL - 插入行
                let n = p.first().copied().unwrap_or(1).max(1);
                let _ = n; // TODO: 实现插入行
            }
            'M' => { // DL - 删除行
                let n = p.first().copied().unwrap_or(1).max(1);
                let _ = n; // TODO: 实现删除行
            }
            '@' => { // ICH - 插入字符
                let n = p.first().copied().unwrap_or(1).max(1);
                let _ = n; // TODO
            }
            'P' => { // DCH - 删除字符
                let n = p.first().copied().unwrap_or(1).max(1);
                let _ = n; // TODO
            }
            _ => {
                tracing::trace!("未处理 CSI: {:?} {:?} {}", p, _intermediates, action);
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => { // DECSC - 保存光标
                self.saved_cursor = (self.grid.cursor.row, self.grid.cursor.col);
                self.saved_attrs = self.grid.current_attrs();
            }
            b'8' => { // DECRC - 恢复光标
                self.grid.cursor.row = self.saved_cursor.0;
                self.grid.cursor.col = self.saved_cursor.1;
                self.grid.set_attrs(self.saved_attrs);
            }
            b'M' => { // RI - 反向换行
                if self.grid.cursor.row == 0 {
                    // TODO: scroll down
                } else {
                    self.grid.cursor.row -= 1;
                }
            }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // OSC 0/2: 设置窗口标题 — 可以传给 UI
        if let Some(code) = params.first() {
            if *code == b"0" || *code == b"2" {
                if let Some(title) = params.get(1) {
                    let _ = String::from_utf8_lossy(title);
                    // TODO: 通知 UI 更新标题
                }
            }
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
}

impl Terminal {
    /// 处理 SGR (Select Graphic Rendition) 序列
    fn handle_sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            self.grid.reset_attrs();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.grid.reset_attrs(),
                1 => self.grid.current_attrs_mut().bold = true,
                2 => self.grid.current_attrs_mut().dim = true,
                3 => self.grid.current_attrs_mut().italic = true,
                4 => self.grid.current_attrs_mut().underline = true,
                7 => self.grid.current_attrs_mut().inverse = true,
                9 => self.grid.current_attrs_mut().strikethrough = true,
                22 => { self.grid.current_attrs_mut().bold = false; self.grid.current_attrs_mut().dim = false; }
                23 => self.grid.current_attrs_mut().italic = false,
                24 => self.grid.current_attrs_mut().underline = false,
                27 => self.grid.current_attrs_mut().inverse = false,
                29 => self.grid.current_attrs_mut().strikethrough = false,
                // 标准前景色 30-37
                30..=37 => self.grid.set_fg(TermColor::Indexed((params[i] - 30) as u8)),
                // 高亮前景色 90-97
                90..=97 => self.grid.set_fg(TermColor::Indexed((params[i] - 90 + 8) as u8)),
                // 扩展前景色
                38 => {
                    if let Some(color) = parse_extended_color(params, &mut i) {
                        self.grid.set_fg(color);
                    }
                }
                39 => self.grid.set_fg(TermColor::Default),
                // 标准背景色 40-47
                40..=47 => self.grid.set_bg(TermColor::Indexed((params[i] - 40) as u8)),
                // 高亮背景色 100-107
                100..=107 => self.grid.set_bg(TermColor::Indexed((params[i] - 100 + 8) as u8)),
                // 扩展背景色
                48 => {
                    if let Some(color) = parse_extended_color(params, &mut i) {
                        self.grid.set_bg(color);
                    }
                }
                49 => self.grid.set_bg(TermColor::Default),
                _ => {}
            }
            i += 1;
        }
    }
}

/// 解析 256 色 / 真彩色 (38;5;N 或 38;2;R;G;B)
fn parse_extended_color(params: &[u16], i: &mut usize) -> Option<TermColor> {
    if *i + 1 < params.len() {
        match params[*i + 1] {
            5 if *i + 2 < params.len() => {
                *i += 2;
                Some(TermColor::Indexed(params[*i] as u8))
            }
            2 if *i + 4 < params.len() => {
                *i += 4;
                Some(TermColor::Rgb(
                    params[*i - 2] as u8,
                    params[*i - 1] as u8,
                    params[*i] as u8,
                ))
            }
            _ => None,
        }
    } else {
        None
    }
}
```

**Grid 需要额外暴露的方法** (添加到 Task 1 的 grid.rs):

```rust
impl Grid {
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        self.scroll_region = (top, bottom.min(self.rows.saturating_sub(1)));
    }
    pub fn current_attrs(&self) -> CellAttrs { self.current_attrs }
    pub fn current_attrs_mut(&mut self) -> &mut CellAttrs { &mut self.current_attrs }
}
```

---

## Task 3: 剪贴板模块

**目标**: 使用 `arboard` 支持文本和图片剪贴板，实现截图粘贴为文件路径。

**Files:**
- Modify: `crates/unterm-ui/Cargo.toml` (添加 `arboard = "3"`, `image = "0.25"`)
- Create: `crates/unterm-ui/src/clipboard.rs`

**实现:**

```rust
// crates/unterm-ui/src/clipboard.rs

use arboard::Clipboard;
use std::path::PathBuf;

/// 剪贴板内容类型
pub enum ClipboardContent {
    /// 纯文本
    Text(String),
    /// 图片（已保存为文件，返回路径）
    ImagePath(String),
    /// 空
    Empty,
}

/// 剪贴板管理器
pub struct ClipboardManager {
    clipboard: Option<Clipboard>,
    /// 截图保存目录
    screenshot_dir: PathBuf,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let clipboard = Clipboard::new().ok();
        let screenshot_dir = Self::default_screenshot_dir();
        // 确保目录存在
        let _ = std::fs::create_dir_all(&screenshot_dir);
        Self { clipboard, screenshot_dir }
    }

    fn default_screenshot_dir() -> PathBuf {
        let home = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".into())
        } else {
            std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
        };
        PathBuf::from(home).join(".unterm").join("screenshots")
    }

    /// 读取剪贴板内容 — 自动检测文本还是图片
    pub fn read(&mut self) -> ClipboardContent {
        let cb = match &mut self.clipboard {
            Some(c) => c,
            None => return ClipboardContent::Empty,
        };

        // 先尝试读图片
        if let Ok(img_data) = cb.get_image() {
            // 保存为 PNG 文件
            match self.save_image_to_file(&img_data) {
                Ok(path) => return ClipboardContent::ImagePath(path),
                Err(e) => {
                    tracing::warn!("剪贴板图片保存失败: {}", e);
                }
            }
        }

        // 回退到文本
        if let Ok(text) = cb.get_text() {
            if !text.is_empty() {
                return ClipboardContent::Text(text);
            }
        }

        ClipboardContent::Empty
    }

    /// 读取纯文本（忽略图片）
    pub fn read_text(&mut self) -> Option<String> {
        self.clipboard.as_mut()?.get_text().ok()
    }

    /// 写入文本到剪贴板
    pub fn write_text(&mut self, text: &str) {
        if let Some(cb) = &mut self.clipboard {
            let _ = cb.set_text(text);
        }
    }

    /// 将图片数据保存为 PNG 文件，返回文件路径
    fn save_image_to_file(&self, img_data: &arboard::ImageData) -> anyhow::Result<String> {
        use std::io::Cursor;

        let width = img_data.width as u32;
        let height = img_data.height as u32;

        // arboard ImageData 是 RGBA 格式
        let img = image::RgbaImage::from_raw(width, height, img_data.bytes.to_vec())
            .ok_or_else(|| anyhow::anyhow!("无法创建图片"))?;

        // 生成文件名
        let now = chrono::Local::now();
        let filename = format!("screenshot_{}.png", now.format("%Y%m%d_%H%M%S"));
        let filepath = self.screenshot_dir.join(&filename);

        // 保存
        img.save(&filepath)?;

        let path_str = filepath.to_string_lossy().to_string();
        tracing::info!("剪贴板图片已保存: {}", path_str);
        Ok(path_str)
    }
}
```

**Cargo.toml 添加:**
```toml
arboard = "3"
image = "0.25"
chrono = "0.4"
```

---

## Task 4: 鼠标输入

**目标**: 在 unterm-ui 中处理鼠标事件，支持文本选择、窗格焦点切换、分屏边框拖拽。

**Files:**
- Create: `crates/unterm-ui/src/mouse.rs`
- Modify: `crates/unterm-ui/src/main.rs` (添加 WindowEvent::CursorMoved/MouseInput 处理)

**实现:**

```rust
// crates/unterm-ui/src/mouse.rs

use crate::layout::{PaneLayout, Rect};

/// 鼠标状态
pub struct MouseState {
    /// 当前鼠标位置（像素）
    pub x: f32,
    pub y: f32,
    /// 左键是否按下
    pub left_pressed: bool,
    /// 选区起始位置（字符坐标）
    pub selection_start: Option<(u16, u16, u64)>,  // (col, row, pane_id)
    /// 选区结束位置
    pub selection_end: Option<(u16, u16)>,
    /// 是否正在拖拽分屏边框
    pub dragging_border: bool,
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            left_pressed: false,
            selection_start: None,
            selection_end: None,
            dragging_border: false,
        }
    }

    /// 根据像素坐标找到所在的 pane
    pub fn hit_test_pane<'a>(&self, panes: &'a [PaneLayout]) -> Option<&'a PaneLayout> {
        panes.iter().find(|p| {
            self.x >= p.rect.x && self.x < p.rect.x + p.rect.width
            && self.y >= p.rect.y && self.y < p.rect.y + p.rect.height
        })
    }

    /// 像素坐标转换为 pane 内的字符坐标
    pub fn pixel_to_cell(&self, pane: &PaneLayout, font_width: f32, font_height: f32) -> (u16, u16) {
        let rel_x = self.x - pane.rect.x - 4.0; // padding
        let rel_y = self.y - pane.rect.y - 4.0;
        let col = (rel_x / font_width).max(0.0) as u16;
        let row = (rel_y / font_height).max(0.0) as u16;
        (col.min(pane.cols.saturating_sub(1)), row.min(pane.rows.saturating_sub(1)))
    }

    /// 检查是否有有效选区
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    /// 清除选区
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }
}
```

**main.rs 中需要添加的事件处理:**

```rust
// 在 WindowEvent match 中添加:

WindowEvent::CursorMoved { position, .. } => {
    self.mouse.x = position.x as f32;
    self.mouse.y = position.y as f32;

    if self.mouse.left_pressed {
        if let Some(layout) = &self.layout {
            let panes = layout.compute_pane_layouts();
            if let Some((_, _, pane_id)) = self.mouse.selection_start {
                if let Some(pane) = panes.iter().find(|p| p.pane_id == pane_id) {
                    let (col, row) = self.mouse.pixel_to_cell(pane, FONT_WIDTH, FONT_HEIGHT);
                    self.mouse.selection_end = Some((col, row));
                    self.request_redraw();
                }
            }
        }
    }
}

WindowEvent::MouseInput { state, button, .. } => {
    use winit::event::MouseButton;
    match button {
        MouseButton::Left => {
            self.mouse.left_pressed = state == ElementState::Pressed;
            if state == ElementState::Pressed {
                // 点击切换 pane 焦点
                if let Some(layout) = &mut self.layout {
                    let panes = layout.compute_pane_layouts();
                    if let Some(pane) = self.mouse.hit_test_pane(&panes) {
                        let pane_id = pane.pane_id;
                        let (col, row) = self.mouse.pixel_to_cell(pane, FONT_WIDTH, FONT_HEIGHT);
                        layout.active_tab_mut().active_pane = pane_id;
                        self.mouse.selection_start = Some((col, row, pane_id));
                        self.mouse.selection_end = None;
                    }
                }
                self.request_redraw();
            } else {
                // 松开 — 如果有选区，复制到剪贴板
                if self.mouse.has_selection() {
                    // TODO: 从 grid 提取选区文本 → clipboard.write_text()
                }
            }
        }
        MouseButton::Right => {
            if state == ElementState::Pressed {
                // 右键粘贴
                if let Some(clip) = &mut self.clipboard {
                    let content = clip.read();
                    match content {
                        ClipboardContent::Text(text) => {
                            // 发送到当前 pane
                            self.send_to_active_pane(text.as_bytes());
                        }
                        ClipboardContent::ImagePath(path) => {
                            // 粘贴文件路径
                            self.send_to_active_pane(path.as_bytes());
                        }
                        ClipboardContent::Empty => {}
                    }
                }
            }
        }
        _ => {}
    }
}

WindowEvent::MouseWheel { delta, .. } => {
    // TODO: 滚动回看
}
```

---

## Task 5: 结构化屏幕传输 (依赖 Task 1+2)

**目标**: 修改 session 使用 Terminal，IPC 传输结构化 cell 数据而非原始文本。

**Files:**
- Modify: `crates/unterm-core/src/session/mod.rs`
- Modify: `crates/unterm-core/src/mcp/router.rs` (screen.read 返回格式)
- Modify: `crates/unterm-proto/src/screen.rs` (添加序列化类型)

**关键改动:**

Session 中 `output_buffer: Arc<RwLock<String>>` 替换为 `terminal: Arc<RwLock<Terminal>>`。

PTY 读取线程改为：
```rust
let terminal = terminal_clone.clone();
std::thread::spawn(move || {
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                terminal.write().process(&buf[..n]);
            }
            Err(_) => break,
        }
    }
});
```

screen.read 响应改为返回 JSON 格式的 cell 数据：
```json
{
  "rows": [[{"ch": "H", "fg": [255,255,255], "bg": null, "bold": true}, ...], ...],
  "cursor": {"row": 0, "col": 5, "visible": true},
  "cols": 80,
  "row_count": 24
}
```

---

## Task 6: 主循环集成 (依赖 Task 3+4+5)

**目标**: 将剪贴板、鼠标、结构化屏幕数据接入 main.rs 事件循环。

**Files:**
- Modify: `crates/unterm-ui/src/main.rs`

**关键改动:**
- `App` 结构体添加 `mouse: MouseState`, `clipboard: ClipboardManager`
- `poll_events` 解析结构化 cell 数据
- Ctrl+V 触发剪贴板读取
- 删除 `strip_ansi_escapes()` 函数（不再需要）

---

## Task 7: 渲染管线升级 (依赖 Task 5)

**目标**: 渲染器支持逐 cell 颜色、光标显示、选区高亮。

**Files:**
- Modify: `crates/unterm-ui/src/render/mod.rs`

**关键改动:**
- `PaneContent` 从 `text: String` 改为 `cells: Vec<Vec<Cell>>`, `cursor: Cursor`
- `draw_frame` 中为每个 cell 设置独立颜色属性
- 添加光标矩形渲染（独立 rect pipeline 或 custom glyph）
- 添加选区高亮渲染

使用 glyphon 的 `Attrs::new().color(Color::rgb(r,g,b))` 对每个字符单独着色，而非整个 buffer 统一颜色。

---

## 执行顺序

1. **Phase 1 (并行)**: Task 1 + Task 2 + Task 3 + Task 4
2. **Phase 2**: Task 5 (集成 Grid + VTE 到 session)
3. **Phase 3**: Task 6 + Task 7 (UI 集成)
4. **Phase 4**: 构建测试、端到端验证
