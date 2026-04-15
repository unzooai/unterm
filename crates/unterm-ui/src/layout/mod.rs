//! Tab/分屏布局管理模块

/// 布局配置常量
const TAB_BAR_HEIGHT: f32 = 32.0;
const STATUS_BAR_HEIGHT: f32 = 24.0;
const DEFAULT_FONT_SIZE: f32 = 16.0;
const DEFAULT_LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 8.0;

/// 布局管理器
pub struct Layout {
    pub window_width: f32,
    pub window_height: f32,
    pub font_width: f32,
    pub font_height: f32,
}

impl Layout {
    pub fn new(window_width: f32, window_height: f32) -> Self {
        Self {
            window_width,
            window_height,
            // 等宽字体：宽度约为字号的 0.6 倍
            font_width: DEFAULT_FONT_SIZE * 0.6,
            font_height: DEFAULT_LINE_HEIGHT,
        }
    }

    /// 计算终端可用区域的字符网格大小
    pub fn grid_size(&self) -> (u16, u16) {
        let content_width = self.window_width - PADDING * 2.0;
        let content_height = self.window_height - TAB_BAR_HEIGHT - STATUS_BAR_HEIGHT - PADDING * 2.0;
        let cols = (content_width / self.font_width).floor() as u16;
        let rows = (content_height / self.font_height).floor() as u16;
        (cols.max(1), rows.max(1))
    }

    /// 更新窗口尺寸
    pub fn resize(&mut self, width: f32, height: f32) {
        self.window_width = width;
        self.window_height = height;
    }

    /// 计算满足 80x24 字符所需的最小窗口像素尺寸
    pub fn min_window_size() -> (u32, u32) {
        let width = (80.0 * DEFAULT_FONT_SIZE * 0.6 + PADDING * 2.0) as u32;
        let height = (24.0 * DEFAULT_LINE_HEIGHT + TAB_BAR_HEIGHT + STATUS_BAR_HEIGHT + PADDING * 2.0) as u32;
        (width, height)
    }
}
