use crate::grid::{CellAttrs, Grid, TermColor};
use vte::{Params, Parser, Perform};

/// 终端内部状态 — 实现 vte::Perform trait
///
/// 与 Parser 分离，因为 `Parser::advance(&mut self, &mut P, &[u8])`
/// 要求 performer 与 parser 是不同的可变引用。
struct TerminalInner {
    pub grid: Grid,
    /// 保存的光标位置 (DECSC/DECRC)
    saved_cursor: (u16, u16),
    saved_attrs: CellAttrs,
}

/// 终端状态机 — 封装 Grid + VTE Parser
pub struct Terminal {
    inner: TerminalInner,
    parser: Parser,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            inner: TerminalInner {
                grid: Grid::new(cols, rows),
                saved_cursor: (0, 0),
                saved_attrs: CellAttrs::default(),
            },
            parser: Parser::new(),
        }
    }

    /// 处理 PTY 输出字节流
    pub fn process(&mut self, bytes: &[u8]) {
        let inner = &mut self.inner;
        self.parser.advance(inner, bytes);
    }

    /// 调整终端大小
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.inner.grid.resize(cols, rows);
    }

    /// 获取 grid 的不可变引用
    pub fn grid(&self) -> &Grid {
        &self.inner.grid
    }

    /// 获取 grid 的可变引用
    pub fn grid_mut(&mut self) -> &mut Grid {
        &mut self.inner.grid
    }

    /// 取出待写回 PTY 的响应数据
    pub fn take_pending_responses(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.inner.grid.pending_responses)
    }
}

impl Perform for TerminalInner {
    /// 可打印字符
    fn print(&mut self, ch: char) {
        self.grid.put_char(ch);
    }

    /// 执行 C0/C1 控制字符
    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0B | 0x0C => self.grid.line_feed(),
            b'\r' => self.grid.carriage_return(),
            0x08 => {
                // Backspace
                if self.grid.cursor.col > 0 {
                    self.grid.cursor.col -= 1;
                }
            }
            0x07 => {} // Bell — 可以触发通知
            b'\t' => {
                // Tab — 跳到下一个 8 的倍数
                let next = (self.grid.cursor.col / 8 + 1) * 8;
                self.grid.cursor.col = next.min(self.grid.cols() - 1);
            }
            _ => {}
        }
    }

    /// CSI 序列
    fn csi_dispatch(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let p: Vec<u16> = params
            .iter()
            .map(|sub| sub.first().copied().unwrap_or(0))
            .collect();

        match action {
            // --- 光标移动 ---
            'A' => {
                // CUU — 上
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.cursor.row = self.grid.cursor.row.saturating_sub(n);
            }
            'B' => {
                // CUD — 下
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.cursor.row = (self.grid.cursor.row + n).min(self.grid.rows() - 1);
            }
            'C' => {
                // CUF — 右
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.cursor.col = (self.grid.cursor.col + n).min(self.grid.cols() - 1);
            }
            'D' => {
                // CUB — 左
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.cursor.col = self.grid.cursor.col.saturating_sub(n);
            }
            'H' | 'f' => {
                // CUP — 光标定位
                let row = p.first().copied().unwrap_or(1).max(1) - 1;
                let col = p.get(1).copied().unwrap_or(1).max(1) - 1;
                self.grid.cursor.row = row.min(self.grid.rows() - 1);
                self.grid.cursor.col = col.min(self.grid.cols() - 1);
            }

            // --- 擦除 ---
            'J' => {
                // ED — 擦除显示
                let mode = p.first().copied().unwrap_or(0);
                self.grid.erase_in_display(mode);
            }
            'K' => {
                // EL — 擦除行
                let mode = p.first().copied().unwrap_or(0);
                self.grid.erase_in_line(mode);
            }

            // --- SGR ---
            'm' => {
                self.handle_sgr(&p);
            }

            // --- 滚动区域 ---
            'r' => {
                // DECSTBM — 设置滚动区域
                let top = p.first().copied().unwrap_or(1).max(1) - 1;
                let bot = p.get(1).copied().unwrap_or(self.grid.rows()).max(1) - 1;
                self.grid.set_scroll_region(top, bot);
                self.grid.cursor.row = 0;
                self.grid.cursor.col = 0;
            }

            // --- 模式设置 ---
            'h' => {
                // SM — 设置模式 (?25h = 显示光标)
                if intermediates.contains(&b'?') {
                    for &param in &p {
                        if param == 25 {
                            self.grid.cursor.visible = true;
                        }
                    }
                }
            }
            'l' => {
                // RM — 重置模式 (?25l = 隐藏光标)
                if intermediates.contains(&b'?') {
                    for &param in &p {
                        if param == 25 {
                            self.grid.cursor.visible = false;
                        }
                    }
                }
            }

            // --- 滚动 ---
            'S' => {
                // SU — 向上滚动
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.scroll_up(n);
            }

            // --- 绝对定位 ---
            'd' => {
                // VPA — 行绝对定位
                let row = p.first().copied().unwrap_or(1).max(1) - 1;
                self.grid.cursor.row = row.min(self.grid.rows() - 1);
            }
            'G' => {
                // CHA — 列绝对定位
                let col = p.first().copied().unwrap_or(1).max(1) - 1;
                self.grid.cursor.col = col.min(self.grid.cols() - 1);
            }

            // --- DSR (Device Status Report) ---
            'n' => {
                let mode = p.first().copied().unwrap_or(0);
                match mode {
                    5 => {
                        // DSR — 设备状态：回复 "OK"
                        self.grid.pending_responses.extend_from_slice(b"\x1b[0n");
                    }
                    6 => {
                        // CPR — 光标位置报告
                        let resp = format!(
                            "\x1b[{};{}R",
                            self.grid.cursor.row + 1,
                            self.grid.cursor.col + 1
                        );
                        self.grid.pending_responses.extend_from_slice(resp.as_bytes());
                    }
                    _ => {}
                }
            }

            // --- 插入/删除行 ---
            'L' => {
                // IL — 插入行
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.insert_lines(n);
            }
            'M' => {
                // DL — 删除行
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.delete_lines(n);
            }

            // --- 插入/删除字符 ---
            '@' => {
                // ICH — 插入字符
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.insert_chars(n);
            }
            'P' => {
                // DCH — 删除字符
                let n = p.first().copied().unwrap_or(1).max(1);
                self.grid.delete_chars(n);
            }

            _ => {
                tracing::trace!(
                    "未处理 CSI: {:?} {:?} {}",
                    p,
                    intermediates,
                    action
                );
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => {
                // DECSC — 保存光标
                self.saved_cursor = (self.grid.cursor.row, self.grid.cursor.col);
                self.saved_attrs = self.grid.current_attrs();
            }
            b'8' => {
                // DECRC — 恢复光标
                self.grid.cursor.row = self.saved_cursor.0;
                self.grid.cursor.col = self.saved_cursor.1;
                self.grid.set_attrs(self.saved_attrs);
            }
            b'M' => {
                // RI — 反向换行
                if self.grid.cursor.row == 0 {
                    self.grid.scroll_down(1);
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
                    let _title = String::from_utf8_lossy(title);
                    // TODO: 通知 UI 更新标题
                }
            }
        }
    }

    fn hook(
        &mut self,
        _params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
    }

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}
}

impl TerminalInner {
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
                22 => {
                    self.grid.current_attrs_mut().bold = false;
                    self.grid.current_attrs_mut().dim = false;
                }
                23 => self.grid.current_attrs_mut().italic = false,
                24 => self.grid.current_attrs_mut().underline = false,
                27 => self.grid.current_attrs_mut().inverse = false,
                29 => self.grid.current_attrs_mut().strikethrough = false,
                // 标准前景色 30-37
                30..=37 => self.grid.set_fg(TermColor::Indexed((params[i] - 30) as u8)),
                // 高亮前景色 90-97
                90..=97 => self.grid.set_fg(TermColor::Indexed((params[i] - 90 + 8) as u8)),
                // 扩展前景色 38;5;N 或 38;2;R;G;B
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
                // 扩展背景色 48;5;N 或 48;2;R;G;B
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
