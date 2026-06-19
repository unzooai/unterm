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
    let surface = if is_light {
        mix(bg, fg, 0.045)
    } else {
        let lifted = mix(bg, fg, 0.028);
        LinearRgba::with_components(lifted.0 * 0.965, lifted.1 * 0.965, lifted.2 * 0.965, 1.)
    };

    SidebarChromeColors {
        surface,
        divider: fg.mul_alpha(if is_light { 0.18 } else { 0.10 }),
        dim_text: fg.mul_alpha(if is_light { 0.76 } else { 0.72 }),
        hover_bg: if is_light {
            mix(bg, fg, 0.11)
        } else {
            mix(surface, fg, 0.07)
        },
        selected_bg: if is_light {
            mix(bg, fg, 0.20)
        } else {
            mix(surface, fg, 0.155)
        },
        is_light,
    }
}
