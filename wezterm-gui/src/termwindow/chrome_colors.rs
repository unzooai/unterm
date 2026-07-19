use window::color::LinearRgba;

#[derive(Debug, Clone, Copy)]
pub struct SidebarChromeColors {
    pub surface: LinearRgba,
    pub divider: LinearRgba,
    pub dim_text: LinearRgba,
    pub hover_bg: LinearRgba,
    pub selected_bg: LinearRgba,
    pub group_bg: LinearRgba,
    pub footer_bg: LinearRgba,
    pub focus_rail: LinearRgba,
    pub is_light: bool,
}

pub fn mix(a: LinearRgba, b: LinearRgba, t: f32) -> LinearRgba {
    LinearRgba::with_components(
        a.0 * (1. - t) + b.0 * t,
        a.1 * (1. - t) + b.1 * t,
        a.2 * (1. - t) + b.2 * t,
        1.,
    )
}

pub fn is_light_surface(bg: LinearRgba) -> bool {
    let luma = 0.2126 * bg.0 + 0.7152 * bg.1 + 0.0722 * bg.2;
    luma > 0.48
}

pub fn sidebar(bg: LinearRgba, fg: LinearRgba) -> SidebarChromeColors {
    let is_light = is_light_surface(bg);
    // Chrome should frame the terminal rather than become a large grey slab.
    // Linear-light mixing is visually stronger than the numeric amount
    // suggests, so dark schemes need only a very small lift.
    let surface = if is_light {
        mix(bg, fg, 0.035)
    } else {
        mix(bg, fg, 0.018)
    };
    let hover_bg = if is_light {
        mix(bg, fg, 0.075)
    } else {
        mix(surface, fg, 0.028)
    };
    let selected_bg = if is_light {
        mix(bg, fg, 0.115)
    } else {
        mix(surface, fg, 0.052)
    };

    SidebarChromeColors {
        surface,
        divider: fg.mul_alpha(if is_light { 0.18 } else { 0.12 }),
        dim_text: fg.mul_alpha(if is_light { 0.86 } else { 0.78 }),
        hover_bg,
        selected_bg,
        group_bg: mix(surface, fg, if is_light { 0.025 } else { 0.014 }),
        footer_bg: mix(surface, fg, if is_light { 0.032 } else { 0.020 }),
        // Relay Core's Signal Jade. Use a darker optical variant on light
        // surfaces so the 2px focus rail remains crisp without glowing.
        focus_rail: if is_light {
            LinearRgba::with_srgba(0x18, 0x7a, 0x68, 0xff)
        } else {
            LinearRgba::with_srgba(0x67, 0xcd, 0xb6, 0xff)
        },
        is_light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WCAG_AA_NORMAL_TEXT: f32 = 4.5;

    #[test]
    fn light_sidebar_states_step_down_from_surface() {
        let bg = LinearRgba::with_srgba(0xf6, 0xf7, 0xf4, 0xff);
        let fg = LinearRgba::with_srgba(0x16, 0x1a, 0x1d, 0xff);
        let chrome = sidebar(bg, fg);

        assert!(chrome.is_light);
        assert!(luma(chrome.hover_bg) < luma(chrome.surface));
        assert!(luma(chrome.selected_bg) < luma(chrome.hover_bg));
    }

    #[test]
    fn dark_sidebar_has_subtle_ordered_layers() {
        let bg = LinearRgba::with_srgba(0x0c, 0x12, 0x20, 0xff);
        let fg = LinearRgba::with_srgba(0xdf, 0xe7, 0xf1, 0xff);
        let chrome = sidebar(bg, fg);

        assert!(!chrome.is_light);
        assert!(luma(chrome.surface) < luma(chrome.hover_bg));
        assert!(luma(chrome.hover_bg) < luma(chrome.selected_bg));
        assert!(luma(chrome.group_bg) < luma(chrome.hover_bg));
        assert!(luma(chrome.footer_bg) < luma(chrome.hover_bg));
    }

    #[test]
    fn sidebar_text_meets_aa_in_light_and_dark_modes() {
        let schemes = [
            (
                LinearRgba::with_srgba(0xf6, 0xf7, 0xf4, 0xff),
                LinearRgba::with_srgba(0x16, 0x1a, 0x1d, 0xff),
            ),
            (
                LinearRgba::with_srgba(0x0c, 0x12, 0x20, 0xff),
                LinearRgba::with_srgba(0xdf, 0xe7, 0xf1, 0xff),
            ),
        ];

        for (bg, fg) in schemes {
            let chrome = sidebar(bg, fg);
            assert!(
                contrast_ratio(fg, chrome.selected_bg) >= WCAG_AA_NORMAL_TEXT,
                "selected sidebar label must meet WCAG AA"
            );
            assert!(
                contrast_ratio(composite(chrome.dim_text, chrome.surface), chrome.surface)
                    >= WCAG_AA_NORMAL_TEXT,
                "secondary sidebar label must meet WCAG AA"
            );
        }
    }

    fn composite(fg: LinearRgba, bg: LinearRgba) -> LinearRgba {
        LinearRgba::with_components(
            fg.0 * fg.3 + bg.0 * (1. - fg.3),
            fg.1 * fg.3 + bg.1 * (1. - fg.3),
            fg.2 * fg.3 + bg.2 * (1. - fg.3),
            1.,
        )
    }

    fn contrast_ratio(foreground: LinearRgba, background: LinearRgba) -> f32 {
        let lighter = luma(foreground).max(luma(background));
        let darker = luma(foreground).min(luma(background));
        (lighter + 0.05) / (darker + 0.05)
    }

    fn luma(c: LinearRgba) -> f32 {
        0.2126 * c.0 + 0.7152 * c.1 + 0.0722 * c.2
    }
}
