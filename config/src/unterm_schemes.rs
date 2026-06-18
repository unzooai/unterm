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

[colors.tab_bar]
background = "#eeece7"
inactive_tab_edge = "#d8d4ca"

[colors.tab_bar.active_tab]
bg_color = "#d9d6ce"
fg_color = "#171615"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab]
bg_color = "#eeece7"
fg_color = "#625f58"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab_hover]
bg_color = "#e1ded6"
fg_color = "#1f1e1a"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab]
bg_color = "#eeece7"
fg_color = "#625f58"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab_hover]
bg_color = "#e1ded6"
fg_color = "#1f1e1a"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

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

[colors.tab_bar]
background = "#151515"
inactive_tab_edge = "#242424"

[colors.tab_bar.active_tab]
bg_color = "#2d2d2a"
fg_color = "#ffffff"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab]
bg_color = "#191919"
fg_color = "#aaa7a0"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab_hover]
bg_color = "#252522"
fg_color = "#eeeeec"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab]
bg_color = "#151515"
fg_color = "#aaa7a0"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab_hover]
bg_color = "#252522"
fg_color = "#eeeeec"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

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

[colors.tab_bar]
background = "#101010"
inactive_tab_edge = "#242424"

[colors.tab_bar.active_tab]
bg_color = "#303030"
fg_color = "#ffffff"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab]
bg_color = "#141414"
fg_color = "#b8b8b8"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab_hover]
bg_color = "#242424"
fg_color = "#eeeeee"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab]
bg_color = "#101010"
fg_color = "#a8a8a8"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab_hover]
bg_color = "#242424"
fg_color = "#eeeeee"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

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

[colors.tab_bar]
background = "#0d0d0d"
inactive_tab_edge = "#242424"

[colors.tab_bar.active_tab]
bg_color = "#2f2f2f"
fg_color = "#ffffff"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab]
bg_color = "#101010"
fg_color = "#bdbdbd"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab_hover]
bg_color = "#202020"
fg_color = "#f2f2f2"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab]
bg_color = "#0d0d0d"
fg_color = "#a8a8a8"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab_hover]
bg_color = "#202020"
fg_color = "#f2f2f2"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

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

[colors.tab_bar]
background = "#0b1019"
inactive_tab_edge = "#20283a"

[colors.tab_bar.active_tab]
bg_color = "#253149"
fg_color = "#f8fbff"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab]
bg_color = "#101724"
fg_color = "#b8c2d3"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab_hover]
bg_color = "#1c2638"
fg_color = "#e6edf7"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab]
bg_color = "#0b1019"
fg_color = "#aab6c8"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab_hover]
bg_color = "#1c2638"
fg_color = "#e6edf7"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

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

[colors.tab_bar]
background = "#eceeec"
inactive_tab_edge = "#d3d8d3"

[colors.tab_bar.active_tab]
bg_color = "#d9dfda"
fg_color = "#07101c"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab]
bg_color = "#f3f4f2"
fg_color = "#58616c"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.inactive_tab_hover]
bg_color = "#e2e7e2"
fg_color = "#0b0f14"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab]
bg_color = "#eceeec"
fg_color = "#58616c"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[colors.tab_bar.new_tab_hover]
bg_color = "#e2e7e2"
fg_color = "#0b0f14"
intensity = "Normal"
italic = false
strikethrough = false
underline = "None"

[metadata]
aliases = []
author = "unterm"
name = "Unterm Daylight"
"##;

#[cfg(test)]
mod tests {
    use super::UNTERM_SCHEMES;
    use crate::{ColorSchemeFile, RgbaColor};

    #[test]
    fn unterm_schemes_parse_and_define_chrome_colors() {
        for (name, toml) in UNTERM_SCHEMES {
            let scheme = ColorSchemeFile::from_toml_str(toml)
                .unwrap_or_else(|err| panic!("{} should parse: {}", name, err));
            assert_eq!(scheme.metadata.name.as_deref(), Some(name));
            assert!(
                scheme.colors.tab_bar.is_some(),
                "{} should define tab_bar colors",
                name
            );
            assert!(
                scheme.colors.scrollbar_thumb.is_some(),
                "{} should define scrollbar_thumb",
                name
            );
        }
    }

    #[test]
    fn unterm_schemes_keep_core_surfaces_readable() {
        for (name, toml) in UNTERM_SCHEMES {
            let scheme = ColorSchemeFile::from_toml_str(toml)
                .unwrap_or_else(|err| panic!("{} should parse: {}", name, err));
            let colors = scheme.colors;

            assert_contrast(
                name,
                "terminal foreground/background",
                colors
                    .foreground
                    .unwrap_or_else(|| panic!("{} should define foreground", name)),
                colors
                    .background
                    .unwrap_or_else(|| panic!("{} should define background", name)),
                7.0,
            );
            assert_contrast(
                name,
                "selection foreground/background",
                colors
                    .selection_fg
                    .unwrap_or_else(|| panic!("{} should define selection_fg", name)),
                colors
                    .selection_bg
                    .unwrap_or_else(|| panic!("{} should define selection_bg", name)),
                4.5,
            );

            let tab_bar = colors
                .tab_bar
                .unwrap_or_else(|| panic!("{} should define tab_bar colors", name));
            assert_contrast(
                name,
                "active tab foreground/background",
                tab_bar.active_tab().fg_color,
                tab_bar.active_tab().bg_color,
                4.5,
            );
            assert_contrast(
                name,
                "inactive tab foreground/background",
                tab_bar.inactive_tab().fg_color,
                tab_bar.inactive_tab().bg_color,
                4.5,
            );
            assert_contrast(
                name,
                "new tab foreground/background",
                tab_bar.new_tab().fg_color,
                tab_bar.new_tab().bg_color,
                4.5,
            );
        }
    }

    fn assert_contrast(
        scheme_name: &str,
        surface_name: &str,
        foreground: RgbaColor,
        background: RgbaColor,
        minimum: f64,
    ) {
        let ratio = contrast_ratio(foreground, background);
        assert!(
            ratio >= minimum,
            "{} {} contrast {:.2} is below {:.1}",
            scheme_name,
            surface_name,
            ratio,
            minimum
        );
    }

    fn contrast_ratio(foreground: RgbaColor, background: RgbaColor) -> f64 {
        let fg = relative_luminance(foreground);
        let bg = relative_luminance(background);
        let lighter = fg.max(bg);
        let darker = fg.min(bg);
        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(color: RgbaColor) -> f64 {
        let (red, green, blue) = rgb_channels(color);
        0.2126 * srgb_to_linear(red)
            + 0.7152 * srgb_to_linear(green)
            + 0.0722 * srgb_to_linear(blue)
    }

    fn srgb_to_linear(channel: u8) -> f64 {
        let channel = channel as f64 / 255.0;
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    fn rgb_channels(color: RgbaColor) -> (u8, u8, u8) {
        let hex: String = color.into();
        let hex = hex
            .strip_prefix('#')
            .unwrap_or_else(|| panic!("{} should be a hex color", hex));
        let red = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let green = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let blue = u8::from_str_radix(&hex[4..6], 16).unwrap();
        (red, green, blue)
    }
}
