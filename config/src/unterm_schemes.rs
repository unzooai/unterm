//! Unterm 自定义内置 color scheme。
//!
//! 这些方案随产品一起内置(不依赖用户在 unterm.lua 手写 config.color_schemes),
//! 由 `build_default_schemes()` 注册进全局 `COLOR_SCHEMES`,可被 theme 系统按
//! 名字引用(见 `wezterm-gui/src/overlay/theme_selector.rs`)。
//!
//! 设计要点:**所有方案的背景都不是纯黑**,这样 `inactive_pane_hsb` 的
//! 亮度变暗(乘法)对非活动分屏才有可见效果——纯黑乘任何系数仍是纯黑。

/// (scheme_name, toml_string),格式与 `scheme_data.rs` 内置方案一致。
pub const UNTERM_SCHEMES: [(&str, &str); 6] = [
    ("Notion Light", NOTION_LIGHT),
    ("Notion Dark", NOTION_DARK),
    ("Classic Dark", CLASSIC_DARK),
    ("Unterm Dark", UNTERM_DARK),
    ("Unterm Midnight", UNTERM_MIDNIGHT),
    ("Unterm Daylight", UNTERM_DAYLIGHT),
];

// Notion 浅色:米白底 + 暖深灰文字,低饱和克制的强调色。
const NOTION_LIGHT: &str = r##"
[colors]
background = "#f8f7f4"
foreground = "#1f1e1a"
cursor_bg = "#1f1e1a"
cursor_fg = "#f8f7f4"
cursor_border = "#1f1e1a"
selection_bg = "#b8d4e6"
selection_fg = "#101315"
split = "#b8b4a8"
scrollbar_thumb = "#77736a"
ansi = [
    "#1f1e1a",
    "#b83232",
    "#2f6f4f",
    "#8b5e12",
    "#1f6f9f",
    "#734a9b",
    "#006f7f",
    "#5d5b55",
]
brights = [
    "#4f4d47",
    "#d54848",
    "#3a8a60",
    "#a87416",
    "#2b82ba",
    "#865bb0",
    "#00899a",
    "#0f0f0d",
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
background = "#181818"
foreground = "#eeeeec"
cursor_bg = "#eeeeec"
cursor_fg = "#181818"
cursor_border = "#eeeeec"
selection_bg = "#363632"
selection_fg = "#ffffff"
split = "#4b4a44"
scrollbar_thumb = "#77736b"
ansi = [
    "#252525",
    "#ff6f61",
    "#4fb286",
    "#e7b84f",
    "#5aa7d6",
    "#b083d9",
    "#5fc6bd",
    "#d8d8d4",
]
brights = [
    "#66635d",
    "#ff8b80",
    "#6ed0a2",
    "#f2cc6b",
    "#7bc0e6",
    "#c49be6",
    "#7de0d7",
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
background = "#121212"
foreground = "#eeeeee"
cursor_bg = "#eeeeee"
cursor_fg = "#121212"
cursor_border = "#eeeeee"
selection_bg = "#383838"
selection_fg = "#ffffff"
split = "#4a4a4a"
scrollbar_thumb = "#686868"
ansi = [
    "#1c1c1c",
    "#ef4444",
    "#22c55e",
    "#eab308",
    "#3b82f6",
    "#a855f7",
    "#06b6d4",
    "#d4d4d4",
]
brights = [
    "#737373",
    "#f87171",
    "#4ade80",
    "#facc15",
    "#60a5fa",
    "#c084fc",
    "#22d3ee",
    "#ffffff",
]

[colors.indexed]

[metadata]
aliases = []
author = "unterm"
name = "Classic Dark"
"##;

// Unterm Dark: neutral high-contrast (Warp-like). Near-black NEUTRAL
// background (#101010, no blue/warm tint), near-white foreground (~13:1
// contrast) for crisp chrome and tab text. Balanced saturated ANSI
// (Snazzy-derived) so the agent status dot (ANSI bright-cyan) pops while
// the row greyscale stays clean. Background is not pure black so
// inactive_pane_hsb dimming still registers.
const UNTERM_DARK: &str = r##"
[colors]
background = "#101010"
foreground = "#f2f2f2"
cursor_bg = "#f2f2f2"
cursor_fg = "#101010"
cursor_border = "#f2f2f2"
selection_bg = "#333333"
selection_fg = "#ffffff"
split = "#3a3a3a"
scrollbar_thumb = "#666666"
ansi = [
    "#1c1c1c",
    "#ff5f57",
    "#5fd17a",
    "#e5c463",
    "#5aa7ff",
    "#c678dd",
    "#4fd6d6",
    "#d6d6d6",
]
brights = [
    "#737373",
    "#ff7b72",
    "#7ee787",
    "#f2d16b",
    "#79b8ff",
    "#d2a8ff",
    "#77e4e4",
    "#ffffff",
]

[colors.indexed]

[metadata]
aliases = []
author = "unterm"
name = "Unterm Dark"
"##;

// Unterm Midnight: low-glare blue-black without washed-out inactive
// greys from older blue tab-bar palettes. Keeps enough cyan
// energy for agents/status while staying calmer than Standard.
const UNTERM_MIDNIGHT: &str = r##"
[colors]
background = "#0f1420"
foreground = "#e6edf7"
cursor_bg = "#e6edf7"
cursor_fg = "#0f1420"
cursor_border = "#e6edf7"
selection_bg = "#263755"
selection_fg = "#f8fbff"
split = "#34425c"
scrollbar_thumb = "#59677f"
ansi = [
    "#171d2b",
    "#ff6b7a",
    "#8bdc88",
    "#e6c46a",
    "#82aaff",
    "#c99cff",
    "#72d6e8",
    "#cbd5e1",
]
brights = [
    "#667085",
    "#ff8794",
    "#a7ec9f",
    "#f0d37a",
    "#9cc0ff",
    "#d8b4ff",
    "#92e5f2",
    "#ffffff",
]

[colors.indexed]

[metadata]
aliases = []
author = "unterm"
name = "Unterm Midnight"
"##;

// Unterm Daylight: neutral light mode with stronger text contrast than
// Solarized Light and less cream/yellow cast in chrome.
const UNTERM_DAYLIGHT: &str = r##"
[colors]
background = "#fbfbfa"
foreground = "#0b0f14"
cursor_bg = "#0b0f14"
cursor_fg = "#fbfbfa"
cursor_border = "#0b0f14"
selection_bg = "#b7cbe6"
selection_fg = "#07101c"
split = "#b5beb3"
scrollbar_thumb = "#737b72"
ansi = [
    "#0b0f14",
    "#b42335",
    "#17643b",
    "#7a5200",
    "#005ea8",
    "#6537a0",
    "#006f7f",
    "#3f4752",
]
brights = [
    "#606975",
    "#cf3347",
    "#25824d",
    "#936300",
    "#0a74c9",
    "#7b4cc2",
    "#00889a",
    "#020406",
]

[colors.indexed]

[metadata]
aliases = []
author = "unterm"
name = "Unterm Daylight"
"##;
