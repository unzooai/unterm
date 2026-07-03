use window::color::LinearRgba;

#[derive(Debug, Clone, Copy)]
pub struct SidebarChromeColors {
    pub surface: LinearRgba,
    pub divider: LinearRgba,
    pub dim_text: LinearRgba,
    pub hover_bg: LinearRgba,
    pub selected_bg: LinearRgba,
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
    // Scheme A "layered neutral": the chrome (top bar + sidebar + bottom bar)
    // shares ONE clearly-lifted tone so it reads as a continuous frame around
    // the darker content, and the sidebar↔bar corners meet seamlessly. The
    // previous near-zero lift (0.028) left the sidebar a different tone than
    // the content-coloured bars, which made those corners read as a clash.
    let surface = if is_light {
        mix(bg, fg, 0.07)
    } else {
        mix(bg, fg, 0.055)
    };

    SidebarChromeColors {
        surface,
        // Divider between the chrome (sidebar / top / bottom bars) and the
        // terminal content. Kept deliberately visible: the earlier
        // near-invisible hairline (0.10 on dark) let the unified-tone chrome
        // bleed into the content so the boundaries read as mush. A clearly
        // present line defines the frame without shouting.
        divider: fg.mul_alpha(if is_light { 0.32 } else { 0.20 }),
        dim_text: fg.mul_alpha(if is_light { 0.82 } else { 0.72 }),
        hover_bg: if is_light {
            mix(bg, fg, 0.145)
        } else {
            mix(surface, fg, 0.07)
        },
        selected_bg: if is_light {
            mix(bg, fg, 0.255)
        } else {
            mix(surface, fg, 0.155)
        },
        is_light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_sidebar_states_step_down_from_surface() {
        let bg = LinearRgba::with_srgba(0xfb, 0xfb, 0xfa, 0xff);
        let fg = LinearRgba::with_srgba(0x0b, 0x0f, 0x14, 0xff);
        let chrome = sidebar(bg, fg);

        assert!(chrome.is_light);
        assert!(luma(chrome.hover_bg) < luma(chrome.surface));
        assert!(luma(chrome.selected_bg) < luma(chrome.hover_bg));
    }

    fn luma(c: LinearRgba) -> f32 {
        0.2126 * c.0 + 0.7152 * c.1 + 0.0722 * c.2
    }
}
