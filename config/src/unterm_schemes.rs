//! Unterm 自定义内置 color scheme。
//!
//! 这些方案随产品一起内置(不依赖用户在 unterm.lua 手写 config.color_schemes),
//! 由 `build_default_schemes()` 注册进全局 `COLOR_SCHEMES`,可被 theme 系统按
//! 名字引用(见 `wezterm-gui/src/overlay/theme_selector.rs`)。
//!
//! 设计要点:**所有方案的背景都不是纯黑**,这样 `inactive_pane_hsb` 的
//! 亮度变暗(乘法)对非活动分屏才有可见效果——纯黑乘任何系数仍是纯黑。

/// (scheme_name, toml_string),格式与 `scheme_data.rs` 内置方案一致。
pub const UNTERM_SCHEMES: [(&str, &str); 3] = [
    ("Notion Light", NOTION_LIGHT),
    ("Notion Dark", NOTION_DARK),
    ("Classic Dark", CLASSIC_DARK),
];

// Notion 浅色:米白底 + 暖深灰文字,低饱和克制的强调色。
const NOTION_LIGHT: &str = r##"
[colors]
background = "#f7f6f3"
foreground = "#37352f"
cursor_bg = "#37352f"
cursor_fg = "#f7f6f3"
cursor_border = "#37352f"
selection_bg = "#d3e5ef"
selection_fg = "#37352f"
split = "#e9e9e7"
scrollbar_thumb = "#cdccc6"
ansi = [
    "#37352f",
    "#e03e3e",
    "#448361",
    "#cb912f",
    "#337ea9",
    "#9065b0",
    "#0b6e99",
    "#9b9a97",
]
brights = [
    "#787774",
    "#eb5757",
    "#0f7b6c",
    "#dfab01",
    "#2383e2",
    "#6940a5",
    "#0b6e99",
    "#37352f",
]

[colors.indexed]

[metadata]
aliases = []
author = "unterm"
name = "Notion Light"
"##;

// Notion 深色:暖黑底(#191919) + 柔和浅灰文字,强调色取 Notion 暗色模式版本。
const NOTION_DARK: &str = r##"
[colors]
background = "#191919"
foreground = "#d4d4d4"
cursor_bg = "#d4d4d4"
cursor_fg = "#191919"
cursor_border = "#d4d4d4"
selection_bg = "#2c2c2c"
selection_fg = "#d4d4d4"
split = "#373737"
scrollbar_thumb = "#4d4c48"
ansi = [
    "#2e2e2e",
    "#ff7369",
    "#4dab9a",
    "#ffd666",
    "#529cca",
    "#9a6dd7",
    "#5bbfb5",
    "#d4d4d4",
]
brights = [
    "#5a5a5a",
    "#ff9b94",
    "#6fccb8",
    "#ffe08c",
    "#7bb4dd",
    "#b794e0",
    "#7fd1c8",
    "#ffffff",
]

[colors.indexed]

[metadata]
aliases = []
author = "unterm"
name = "Notion Dark"
"##;

// Classic:沿用 Tango Dark 的高对比 ANSI,但背景由纯黑改为 #14141a,
// 让 inactive_pane_hsb 变暗在 classic 主题下也能生效。
const CLASSIC_DARK: &str = r##"
[colors]
background = "#14141a"
foreground = "#d3d7cf"
cursor_bg = "#d3d7cf"
cursor_fg = "#14141a"
cursor_border = "#d3d7cf"
selection_bg = "#444444"
selection_fg = "#d3d7cf"
split = "#5b5b66"
scrollbar_thumb = "#4a4a55"
ansi = [
    "#2e2e2e",
    "#cc0000",
    "#4e9a06",
    "#c4a000",
    "#3465a4",
    "#75507b",
    "#06989a",
    "#d3d7cf",
]
brights = [
    "#555753",
    "#ef2929",
    "#8ae234",
    "#fce94f",
    "#729fcf",
    "#ad7fa8",
    "#34e2e2",
    "#eeeeec",
]

[colors.indexed]

[metadata]
aliases = []
author = "unterm"
name = "Classic Dark"
"##;
