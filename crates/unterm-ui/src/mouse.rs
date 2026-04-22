//! 鼠标输入模块：命中测试、像素→字符坐标转换、文本选区管理、Tab 栏交互

use crate::layout::{PaneLayout, Rect};

/// Tab 栏点击结果
#[derive(Debug, Clone, PartialEq)]
pub enum TabBarHit {
    /// 点击了某个 Tab（切换）
    Tab(u64),
    /// 点击了某个 Tab 的关闭按钮
    CloseTab(u64),
    /// 点击了 + 新建按钮
    NewTab,
    /// 点击了 ˅ 下拉菜单按钮
    Dropdown,
    /// 没有命中任何可交互元素
    None,
}

/// Tab 栏各元素的像素区域（由渲染器计算后传回）
#[derive(Debug, Clone)]
pub struct TabBarRegion {
    pub tab_id: u64,
    pub tab_rect: Rect,
    pub close_rect: Rect,
}

/// Tab 栏布局信息
#[derive(Debug, Clone, Default)]
pub struct TabBarLayout {
    pub tabs: Vec<TabBarRegion>,
    pub new_btn_rect: Rect,
    pub dropdown_rect: Rect,
}

/// 鼠标状态
pub struct MouseState {
    /// 当前鼠标像素位置
    pub x: f32,
    pub y: f32,
    /// 鼠标左键是否按下
    pub left_pressed: bool,
    /// 选区起点 (col, row, pane_id)
    pub selection_start: Option<(u16, u16, u64)>,
    /// 选区终点 (col, row)
    pub selection_end: Option<(u16, u16)>,
    /// 是否正在拖拽分屏边框
    pub dragging_border: bool,
}

impl MouseState {
    /// 创建默认状态
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

    /// 根据当前鼠标像素坐标，找到鼠标所在的 pane
    pub fn hit_test_pane<'a>(&self, panes: &'a [PaneLayout]) -> Option<&'a PaneLayout> {
        panes.iter().find(|p| {
            let Rect { x, y, width, height } = p.rect;
            self.x >= x && self.x < x + width && self.y >= y && self.y < y + height
        })
    }

    /// 将当前鼠标像素坐标转换为 pane 内的字符坐标 (col, row)
    ///
    /// 减去 pane 左上角偏移和 4.0 内边距后，除以字体宽高得到字符坐标。
    pub fn pixel_to_cell(&self, pane: &PaneLayout, font_width: f32, font_height: f32) -> (u16, u16) {
        let local_x = (self.x - pane.rect.x - 4.0).max(0.0);
        let local_y = (self.y - pane.rect.y - 4.0).max(0.0);

        let col = (local_x / font_width).floor() as u16;
        let row = (local_y / font_height).floor() as u16;

        // 限制在 pane 范围内
        let col = col.min(pane.cols.saturating_sub(1));
        let row = row.min(pane.rows.saturating_sub(1));

        (col, row)
    }

    /// 是否有活跃的文本选区
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    /// 清除选区
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }

    /// Tab 栏命中测试
    pub fn hit_test_tab_bar(&self, layout: &TabBarLayout) -> TabBarHit {
        // 检查 ˅ 下拉按钮
        if layout.dropdown_rect.contains(self.x, self.y) {
            return TabBarHit::Dropdown;
        }
        // 检查 + 新建按钮
        if layout.new_btn_rect.contains(self.x, self.y) {
            return TabBarHit::NewTab;
        }
        // 检查各 Tab
        for region in &layout.tabs {
            if region.close_rect.contains(self.x, self.y) {
                return TabBarHit::CloseTab(region.tab_id);
            }
            if region.tab_rect.contains(self.x, self.y) {
                return TabBarHit::Tab(region.tab_id);
            }
        }
        TabBarHit::None
    }
}
