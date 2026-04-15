//! Tab/分屏布局管理模块

/// Tab 栏高度（像素）
const TAB_BAR_HEIGHT: f32 = 32.0;
/// 状态栏高度（像素）
const STATUS_BAR_HEIGHT: f32 = 24.0;
/// 分屏边框宽度（像素）
const SPLIT_BORDER_WIDTH: f32 = 2.0;
/// 默认字体大小
const DEFAULT_FONT_SIZE: f32 = 16.0;
/// 默认行高
const DEFAULT_LINE_HEIGHT: f32 = 20.0;
/// 等宽字体宽高比
const FONT_WIDTH_RATIO: f32 = 0.6;
/// 内边距
const PADDING: f32 = 4.0;

/// 唯一标识一个 Pane
pub type PaneId = u64;
/// 唯一标识一个 Tab
pub type TabId = u64;

/// 分屏方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// 左右分
    Horizontal,
    /// 上下分
    Vertical,
}

/// 分屏树节点
#[derive(Debug, Clone)]
pub enum PaneNode {
    /// 叶子节点 — 一个终端 pane
    Leaf {
        pane_id: PaneId,
        /// 关联的 session
        session_id: Option<String>,
    },
    /// 分支节点 — 两个子节点
    Split {
        direction: SplitDirection,
        /// 0.0..1.0，左/上占比
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    /// 按先序遍历收集所有叶子节点的 pane_id
    fn collect_leaves(&self, out: &mut Vec<PaneId>) {
        match self {
            PaneNode::Leaf { pane_id, .. } => out.push(*pane_id),
            PaneNode::Split { first, second, .. } => {
                first.collect_leaves(out);
                second.collect_leaves(out);
            }
        }
    }

    /// 查找指定 pane_id 对应的叶子节点的 session_id
    fn find_session(&self, target: PaneId) -> Option<Option<&str>> {
        match self {
            PaneNode::Leaf { pane_id, session_id } => {
                if *pane_id == target {
                    Some(session_id.as_deref())
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.find_session(target).or_else(|| second.find_session(target))
            }
        }
    }

    /// 设置指定 pane_id 的 session_id，返回是否成功
    fn set_session(&mut self, target: PaneId, new_session: String) -> bool {
        match self {
            PaneNode::Leaf { pane_id, session_id } => {
                if *pane_id == target {
                    *session_id = Some(new_session);
                    true
                } else {
                    false
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.set_session(target, new_session.clone())
                    || second.set_session(target, new_session)
            }
        }
    }

    /// 在树中找到目标 pane 并将其分裂为 Split 节点，返回新 pane_id
    fn split_leaf(&mut self, target: PaneId, direction: SplitDirection, new_pane_id: PaneId) -> bool {
        match self {
            PaneNode::Leaf { pane_id, session_id } => {
                if *pane_id == target {
                    // 将当前叶子替换为 Split
                    let original = PaneNode::Leaf {
                        pane_id: *pane_id,
                        session_id: session_id.take(),
                    };
                    let new_leaf = PaneNode::Leaf {
                        pane_id: new_pane_id,
                        session_id: None,
                    };
                    *self = PaneNode::Split {
                        direction,
                        ratio: 0.5,
                        first: Box::new(original),
                        second: Box::new(new_leaf),
                    };
                    true
                } else {
                    false
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.split_leaf(target, direction, new_pane_id)
                    || second.split_leaf(target, direction, new_pane_id)
            }
        }
    }

    /// 关闭指定 pane，返回 Some(兄弟节点) 如果成功移除，None 如果未找到
    /// 如果当前节点就是目标叶子，由调用者处理（root 情况）
    fn remove_leaf(&mut self, target: PaneId) -> Option<PaneNode> {
        match self {
            PaneNode::Leaf { .. } => None,
            PaneNode::Split { first, second, .. } => {
                // 检查 first 是否是目标叶子
                if let PaneNode::Leaf { pane_id, .. } = first.as_ref() {
                    if *pane_id == target {
                        // 用 second 替换整个节点
                        return Some(*second.clone());
                    }
                }
                // 检查 second 是否是目标叶子
                if let PaneNode::Leaf { pane_id, .. } = second.as_ref() {
                    if *pane_id == target {
                        // 用 first 替换整个节点
                        return Some(*first.clone());
                    }
                }
                // 递归到子树
                if let Some(replacement) = first.remove_leaf(target) {
                    *first = Box::new(replacement);
                    return Some(self.clone());
                }
                if let Some(replacement) = second.remove_leaf(target) {
                    *second = Box::new(replacement);
                    return Some(self.clone());
                }
                None
            }
        }
    }

    /// 获取第一个叶子节点的 pane_id
    fn first_leaf(&self) -> PaneId {
        match self {
            PaneNode::Leaf { pane_id, .. } => *pane_id,
            PaneNode::Split { first, .. } => first.first_leaf(),
        }
    }

    /// 递归计算布局
    fn compute_layouts(
        &self,
        rect: Rect,
        active_pane: PaneId,
        font_width: f32,
        font_height: f32,
        out: &mut Vec<PaneLayout>,
    ) {
        match self {
            PaneNode::Leaf { pane_id, session_id } => {
                let cols = ((rect.width - 2.0 * PADDING) / font_width).floor() as u16;
                let rows = ((rect.height - 2.0 * PADDING) / font_height).floor() as u16;
                out.push(PaneLayout {
                    pane_id: *pane_id,
                    session_id: session_id.clone(),
                    rect,
                    cols: cols.max(1),
                    rows: rows.max(1),
                    is_active: *pane_id == active_pane,
                });
            }
            PaneNode::Split { direction, ratio, first, second } => {
                let (first_rect, second_rect) = match direction {
                    SplitDirection::Horizontal => {
                        // 左右分
                        let first_width = (rect.width - SPLIT_BORDER_WIDTH) * ratio;
                        let second_width = rect.width - SPLIT_BORDER_WIDTH - first_width;
                        (
                            Rect {
                                x: rect.x,
                                y: rect.y,
                                width: first_width,
                                height: rect.height,
                            },
                            Rect {
                                x: rect.x + first_width + SPLIT_BORDER_WIDTH,
                                y: rect.y,
                                width: second_width,
                                height: rect.height,
                            },
                        )
                    }
                    SplitDirection::Vertical => {
                        // 上下分
                        let first_height = (rect.height - SPLIT_BORDER_WIDTH) * ratio;
                        let second_height = rect.height - SPLIT_BORDER_WIDTH - first_height;
                        (
                            Rect {
                                x: rect.x,
                                y: rect.y,
                                width: rect.width,
                                height: first_height,
                            },
                            Rect {
                                x: rect.x,
                                y: rect.y + first_height + SPLIT_BORDER_WIDTH,
                                width: rect.width,
                                height: second_height,
                            },
                        )
                    }
                };
                first.compute_layouts(first_rect, active_pane, font_width, font_height, out);
                second.compute_layouts(second_rect, active_pane, font_width, font_height, out);
            }
        }
    }
}

/// 一个 Tab
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub root: PaneNode,
    pub active_pane: PaneId,
}

/// 矩形区域（像素坐标）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Pane 的渲染信息
#[derive(Debug, Clone)]
pub struct PaneLayout {
    pub pane_id: PaneId,
    pub session_id: Option<String>,
    /// 像素区域
    pub rect: Rect,
    /// 字符列数
    pub cols: u16,
    /// 字符行数
    pub rows: u16,
    /// 是否为当前激活 pane
    pub is_active: bool,
}

/// 整体布局管理器
#[derive(Debug, Clone)]
pub struct LayoutManager {
    tabs: Vec<Tab>,
    active_tab: usize,
    next_pane_id: PaneId,
    next_tab_id: TabId,
    window_width: f32,
    window_height: f32,
}

impl LayoutManager {
    /// 创建，默认包含一个 Tab 一个 Pane
    pub fn new(width: f32, height: f32) -> Self {
        let pane_id = 1;
        let tab_id = 1;
        let tab = Tab {
            id: tab_id,
            title: "Tab 1".to_string(),
            root: PaneNode::Leaf {
                pane_id,
                session_id: None,
            },
            active_pane: pane_id,
        };
        Self {
            tabs: vec![tab],
            active_tab: 0,
            next_pane_id: pane_id + 1,
            next_tab_id: tab_id + 1,
            window_width: width,
            window_height: height,
        }
    }

    /// 新建 Tab，返回 (TabId, PaneId)
    pub fn add_tab(&mut self, title: String) -> (TabId, PaneId) {
        let tab_id = self.next_tab_id();
        let pane_id = self.next_pane_id();
        let tab = Tab {
            id: tab_id,
            title,
            root: PaneNode::Leaf {
                pane_id,
                session_id: None,
            },
            active_pane: pane_id,
        };
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        (tab_id, pane_id)
    }

    /// 关闭 Tab
    pub fn close_tab(&mut self, tab_id: TabId) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.tabs.remove(idx);
            if self.tabs.is_empty() {
                // 至少保留一个 Tab
                let pane_id = self.next_pane_id();
                let new_tab_id = self.next_tab_id();
                self.tabs.push(Tab {
                    id: new_tab_id,
                    title: "Tab 1".to_string(),
                    root: PaneNode::Leaf {
                        pane_id,
                        session_id: None,
                    },
                    active_pane: pane_id,
                });
                self.active_tab = 0;
            } else if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// 切换到指定 Tab
    pub fn switch_tab(&mut self, tab_id: TabId) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.active_tab = idx;
        }
    }

    /// 切换到下一个 Tab
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    /// 切换到上一个 Tab
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_tab == 0 {
                self.active_tab = self.tabs.len() - 1;
            } else {
                self.active_tab -= 1;
            }
        }
    }

    /// 获取当前 Tab
    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    /// 获取当前 Tab（可变引用）
    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    /// 获取所有 Tab 信息（用于渲染 Tab 栏）：(id, title, is_active)
    pub fn tab_infos(&self) -> Vec<(TabId, &str, bool)> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| (tab.id, tab.title.as_str(), i == self.active_tab))
            .collect()
    }

    /// 分屏：将当前激活 pane 按指定方向分成两个，返回新 pane_id
    pub fn split_pane(&mut self, direction: SplitDirection) -> Option<PaneId> {
        let new_pane_id = self.next_pane_id();
        let tab = &mut self.tabs[self.active_tab];
        let target = tab.active_pane;
        if tab.root.split_leaf(target, direction, new_pane_id) {
            tab.active_pane = new_pane_id;
            Some(new_pane_id)
        } else {
            // 回退 id 计数器
            self.next_pane_id -= 1;
            None
        }
    }

    /// 关闭当前激活 pane（如果是最后一个 pane 则关闭整个 tab）
    /// 返回 true 表示 tab 被关闭
    pub fn close_pane(&mut self) -> bool {
        let tab = &self.tabs[self.active_tab];
        let target = tab.active_pane;

        // 如果 root 就是目标叶子，关闭整个 tab
        if let PaneNode::Leaf { pane_id, .. } = &tab.root {
            if *pane_id == target {
                let tab_id = tab.id;
                self.close_tab(tab_id);
                return true;
            }
        }

        // 否则从树中移除
        let tab = &mut self.tabs[self.active_tab];
        if let Some(replacement) = tab.root.remove_leaf(target) {
            tab.root = replacement;
            // 激活兄弟节点中的第一个叶子
            tab.active_pane = tab.root.first_leaf();
        }
        false
    }

    /// 切换到下一个 pane
    pub fn focus_next_pane(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        let mut leaves = Vec::new();
        tab.root.collect_leaves(&mut leaves);
        if leaves.is_empty() {
            return;
        }
        if let Some(idx) = leaves.iter().position(|&id| id == tab.active_pane) {
            let next = (idx + 1) % leaves.len();
            tab.active_pane = leaves[next];
        }
    }

    /// 切换到上一个 pane
    pub fn focus_prev_pane(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        let mut leaves = Vec::new();
        tab.root.collect_leaves(&mut leaves);
        if leaves.is_empty() {
            return;
        }
        if let Some(idx) = leaves.iter().position(|&id| id == tab.active_pane) {
            let prev = if idx == 0 { leaves.len() - 1 } else { idx - 1 };
            tab.active_pane = leaves[prev];
        }
    }

    /// 计算所有 pane 的布局（遍历分屏树）
    pub fn compute_pane_layouts(&self) -> Vec<PaneLayout> {
        let content = self.content_rect();
        let font_width = DEFAULT_FONT_SIZE * FONT_WIDTH_RATIO;
        let font_height = DEFAULT_LINE_HEIGHT;
        let tab = &self.tabs[self.active_tab];
        let mut layouts = Vec::new();
        tab.root.compute_layouts(content, tab.active_pane, font_width, font_height, &mut layouts);
        layouts
    }

    /// 设置 pane 关联的 session_id
    pub fn set_pane_session(&mut self, pane_id: PaneId, session_id: String) {
        let tab = &mut self.tabs[self.active_tab];
        tab.root.set_session(pane_id, session_id);
    }

    /// 获取当前激活 pane 的 session_id
    pub fn active_session_id(&self) -> Option<&str> {
        let tab = &self.tabs[self.active_tab];
        tab.root.find_session(tab.active_pane).flatten()
    }

    /// 窗口大小变化
    pub fn resize(&mut self, width: f32, height: f32) {
        self.window_width = width;
        self.window_height = height;
    }

    /// Tab 栏区域
    pub fn tab_bar_rect(&self) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: self.window_width,
            height: TAB_BAR_HEIGHT,
        }
    }

    /// 状态栏区域
    pub fn status_bar_rect(&self) -> Rect {
        Rect {
            x: 0.0,
            y: self.window_height - STATUS_BAR_HEIGHT,
            width: self.window_width,
            height: STATUS_BAR_HEIGHT,
        }
    }

    /// 内容区域（Tab 栏和状态栏之间）
    pub fn content_rect(&self) -> Rect {
        Rect {
            x: 0.0,
            y: TAB_BAR_HEIGHT,
            width: self.window_width,
            height: self.window_height - TAB_BAR_HEIGHT - STATUS_BAR_HEIGHT,
        }
    }

    /// 下一个 pane id
    fn next_pane_id(&mut self) -> PaneId {
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
    }

    /// 下一个 tab id
    fn next_tab_id(&mut self) -> TabId {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        id
    }

    /// 计算满足 80x24 字符所需的最小窗口像素尺寸
    pub fn min_window_size() -> (u32, u32) {
        let font_width = DEFAULT_FONT_SIZE * FONT_WIDTH_RATIO;
        let width = (80.0 * font_width + PADDING * 2.0) as u32;
        let height =
            (24.0 * DEFAULT_LINE_HEIGHT + TAB_BAR_HEIGHT + STATUS_BAR_HEIGHT + PADDING * 2.0)
                as u32;
        (width, height)
    }
}
