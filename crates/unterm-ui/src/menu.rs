/// Menu system for Unterm terminal application.
///
/// Provides dropdown and context menu support with hit-testing,
/// hover tracking, and predefined menu layouts.

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const MENU_ITEM_HEIGHT: f32 = 28.0;
pub const MENU_PADDING: f32 = 8.0;
pub const MENU_WIDTH: f32 = 260.0;
pub const SEPARATOR_HEIGHT: f32 = 9.0;

// ---------------------------------------------------------------------------
// MenuAction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    NewTab,
    CloseTab,
    HorizontalSplit,
    VerticalSplit,
    ClosePane,
    Copy,
    Paste,
    Settings,
    About,
    None,
}

// ---------------------------------------------------------------------------
// MenuItem
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub action: MenuAction,
    pub is_separator: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>, action: MenuAction, shortcut: Option<&str>) -> Self {
        Self {
            label: label.into(),
            shortcut: shortcut.map(String::from),
            action,
            is_separator: false,
        }
    }

    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: None,
            action: MenuAction::None,
            is_separator: true,
        }
    }
}

// ---------------------------------------------------------------------------
// MenuKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    DropdownMenu,
    ContextMenu,
}

// ---------------------------------------------------------------------------
// MenuState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MenuState {
    pub kind: Option<MenuKind>,
    pub x: f32,
    pub y: f32,
    pub hovered: Option<usize>,
    pub items: Vec<MenuItem>,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            kind: None,
            x: 0.0,
            y: 0.0,
            hovered: None,
            items: Vec::new(),
        }
    }
}

impl MenuState {
    pub fn new() -> Self {
        Self::default()
    }

    // -- open helpers -------------------------------------------------------

    /// Opens the dropdown menu (triggered by a menu-bar button, for example).
    pub fn open_dropdown(&mut self, x: f32, y: f32) {
        self.kind = Some(MenuKind::DropdownMenu);
        self.x = x;
        self.y = y;
        self.hovered = None;
        self.items = vec![
            MenuItem::new("新建标签页", MenuAction::NewTab, Some("Ctrl+Shift+T")),
            MenuItem::separator(),
            MenuItem::new("水平拆分", MenuAction::HorizontalSplit, Some("Alt+Shift+D")),
            MenuItem::new("垂直拆分", MenuAction::VerticalSplit, Some("Alt+Shift+-")),
            MenuItem::separator(),
            MenuItem::new("设置", MenuAction::Settings, Some("Ctrl+,")),
            MenuItem::new("关于 Unterm", MenuAction::About, None),
        ];
    }

    /// Opens the context (right-click) menu.
    pub fn open_context(&mut self, x: f32, y: f32) {
        self.kind = Some(MenuKind::ContextMenu);
        self.x = x;
        self.y = y;
        self.hovered = None;
        self.items = vec![
            MenuItem::new("复制", MenuAction::Copy, Some("Ctrl+C")),
            MenuItem::new("粘贴", MenuAction::Paste, Some("Ctrl+V")),
            MenuItem::separator(),
            MenuItem::new("水平拆分", MenuAction::HorizontalSplit, Some("Alt+Shift+D")),
            MenuItem::new("垂直拆分", MenuAction::VerticalSplit, Some("Alt+Shift+-")),
            MenuItem::new("关闭窗格", MenuAction::ClosePane, Some("Ctrl+Shift+W")),
        ];
    }

    /// Closes the menu and resets state.
    pub fn close(&mut self) {
        self.kind = None;
        self.hovered = None;
        self.items.clear();
    }

    // -- queries ------------------------------------------------------------

    /// Returns `true` if any menu is currently open.
    pub fn is_open(&self) -> bool {
        self.kind.is_some()
    }

    /// Returns the bounding rectangle of the menu as `(x, y, width, height)`.
    pub fn menu_rect(&self) -> (f32, f32, f32, f32) {
        let height = self.total_height();
        (self.x, self.y, MENU_WIDTH, height)
    }

    /// Determines which menu item (by index) is at the given pixel position.
    ///
    /// Returns `None` if the position is outside the menu or lands on a
    /// separator.
    pub fn hit_test(&mut self, px: f32, py: f32) -> Option<usize> {
        if !self.is_open() {
            return None;
        }

        let (mx, my, mw, mh) = self.menu_rect();

        // Outside the menu rect entirely.
        if px < mx || px > mx + mw || py < my || py > my + mh {
            self.hovered = None;
            return None;
        }

        let mut offset_y = my + MENU_PADDING;

        for (i, item) in self.items.iter().enumerate() {
            let item_h = if item.is_separator {
                SEPARATOR_HEIGHT
            } else {
                MENU_ITEM_HEIGHT
            };

            if py >= offset_y && py < offset_y + item_h {
                if item.is_separator {
                    self.hovered = None;
                    return None;
                }
                self.hovered = Some(i);
                return Some(i);
            }

            offset_y += item_h;
        }

        self.hovered = None;
        None
    }

    /// Returns the `MenuAction` of the currently hovered item, if any.
    pub fn hovered_action(&self) -> Option<MenuAction> {
        let idx = self.hovered?;
        let item = self.items.get(idx)?;
        if item.is_separator {
            return None;
        }
        Some(item.action)
    }

    /// 预估右键菜单的高度（用于打开前的边界钳位）
    pub fn estimate_context_height(&self) -> f32 {
        // 右键菜单: 4 normal + 1 separator + 1 closePane = 5*28 + 1*9 + 2*8
        5.0 * MENU_ITEM_HEIGHT + 1.0 * SEPARATOR_HEIGHT + 2.0 * MENU_PADDING
    }

    // -- internal -----------------------------------------------------------

    /// Computes the total height of the menu including padding.
    fn total_height(&self) -> f32 {
        let content: f32 = self
            .items
            .iter()
            .map(|item| {
                if item.is_separator {
                    SEPARATOR_HEIGHT
                } else {
                    MENU_ITEM_HEIGHT
                }
            })
            .sum();

        content + MENU_PADDING * 2.0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_closed() {
        let state = MenuState::new();
        assert!(!state.is_open());
        assert!(state.hovered_action().is_none());
    }

    #[test]
    fn open_dropdown_populates_items() {
        let mut state = MenuState::new();
        state.open_dropdown(100.0, 50.0);

        assert!(state.is_open());
        assert_eq!(state.kind, Some(MenuKind::DropdownMenu));
        assert_eq!(state.items.len(), 7);
        assert_eq!(state.items[0].label, "新建标签页");
        assert!(state.items[1].is_separator);
    }

    #[test]
    fn open_context_populates_items() {
        let mut state = MenuState::new();
        state.open_context(200.0, 300.0);

        assert!(state.is_open());
        assert_eq!(state.kind, Some(MenuKind::ContextMenu));
        assert_eq!(state.items.len(), 6);
        assert_eq!(state.items[0].action, MenuAction::Copy);
    }

    #[test]
    fn close_resets_state() {
        let mut state = MenuState::new();
        state.open_dropdown(0.0, 0.0);
        state.close();

        assert!(!state.is_open());
        assert!(state.items.is_empty());
    }

    #[test]
    fn hit_test_returns_none_when_closed() {
        let mut state = MenuState::new();
        assert!(state.hit_test(10.0, 10.0).is_none());
    }

    #[test]
    fn hit_test_identifies_first_item() {
        let mut state = MenuState::new();
        state.open_dropdown(0.0, 0.0);

        // First item starts at y = MENU_PADDING, mid-point of first item:
        let mid_y = MENU_PADDING + MENU_ITEM_HEIGHT / 2.0;
        let mid_x = MENU_WIDTH / 2.0;

        let result = state.hit_test(mid_x, mid_y);
        assert_eq!(result, Some(0));
        assert_eq!(state.hovered_action(), Some(MenuAction::NewTab));
    }

    #[test]
    fn hit_test_skips_separator() {
        let mut state = MenuState::new();
        state.open_dropdown(0.0, 0.0);

        // Separator is item[1], starts right after first item.
        let sep_y = MENU_PADDING + MENU_ITEM_HEIGHT + SEPARATOR_HEIGHT / 2.0;
        let result = state.hit_test(MENU_WIDTH / 2.0, sep_y);
        assert!(result.is_none());
    }

    #[test]
    fn hit_test_outside_menu() {
        let mut state = MenuState::new();
        state.open_dropdown(100.0, 100.0);

        assert!(state.hit_test(0.0, 0.0).is_none());
    }

    #[test]
    fn menu_rect_dimensions() {
        let mut state = MenuState::new();
        state.open_context(10.0, 20.0);

        let (x, y, w, h) = state.menu_rect();
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
        assert_eq!(w, MENU_WIDTH);

        // 4 normal items + 1 separator + 1 close-pane item = 5*28 + 1*9 + padding*2
        let expected = 5.0 * MENU_ITEM_HEIGHT + 1.0 * SEPARATOR_HEIGHT + 2.0 * MENU_PADDING;
        assert!((h - expected).abs() < f32::EPSILON);
    }
}
