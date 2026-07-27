use super::{
    cell::{CellAttributes, TerminalColor},
    csi_params,
};
use crate::{StyledBlink, StyledUnderline, StyledVerticalAlign};

pub(super) fn apply(params: &[usize], attr: &mut CellAttributes) {
    let params = if params.is_empty() { &[0][..] } else { params };
    let mut idx = 0;
    while idx < params.len() {
        match params[idx] {
            0 => *attr = CellAttributes::default(),
            1 => attr.bold = true,
            2 => attr.faint = true,
            3 => attr.italic = true,
            4 => attr.set_underline(StyledUnderline::Single),
            5 => attr.blink = Some(StyledBlink::Slow),
            6 => attr.blink = Some(StyledBlink::Rapid),
            7 => attr.inverse = true,
            8 => attr.hidden = true,
            9 => attr.strikethrough = true,
            22 => {
                attr.bold = false;
                attr.faint = false;
            }
            23 => attr.italic = false,
            21 => attr.set_underline(StyledUnderline::Double),
            24 => attr.clear_underline(),
            25 => attr.blink = None,
            27 => attr.inverse = false,
            28 => attr.hidden = false,
            29 => attr.strikethrough = false,
            53 => attr.overline = true,
            55 => attr.overline = false,
            73 => attr.vertical_align = Some(StyledVerticalAlign::SuperScript),
            74 => attr.vertical_align = Some(StyledVerticalAlign::SubScript),
            75 => attr.vertical_align = None,
            underline_style
                if (csi_params::SGR_UNDERLINE_STYLE_BASE
                    ..=csi_params::SGR_UNDERLINE_STYLE_BASE + 5)
                    .contains(&underline_style) =>
            {
                match underline_style - csi_params::SGR_UNDERLINE_STYLE_BASE {
                    0 => attr.clear_underline(),
                    1 => attr.set_underline(StyledUnderline::Single),
                    2 => attr.set_underline(StyledUnderline::Double),
                    3 => attr.set_underline(StyledUnderline::Curly),
                    4 => attr.set_underline(StyledUnderline::Dotted),
                    5 => attr.set_underline(StyledUnderline::Dashed),
                    _ => {}
                }
            }
            30..=37 => attr.fg = Some(TerminalColor::Palette(params[idx] as u8 - 30)),
            39 => attr.fg = None,
            40..=47 => attr.bg = Some(TerminalColor::Palette(params[idx] as u8 - 40)),
            49 => attr.bg = None,
            90..=97 => attr.fg = Some(TerminalColor::Palette(params[idx] as u8 - 90 + 8)),
            100..=107 => attr.bg = Some(TerminalColor::Palette(params[idx] as u8 - 100 + 8)),
            38 | 48 | 58 => {
                let color_target = params[idx];
                if let Some((color, consumed)) = parse_extended_color(&params[idx + 1..]) {
                    match color_target {
                        38 => attr.fg = Some(color),
                        48 => attr.bg = Some(color),
                        58 => attr.underline_color = Some(color),
                        _ => {}
                    }
                    idx += consumed;
                }
            }
            59 => attr.underline_color = None,
            _ => {}
        }
        idx += 1;
    }
}

fn parse_extended_color(params: &[usize]) -> Option<(TerminalColor, usize)> {
    match params {
        [5, color, ..] => Some((TerminalColor::Palette((*color).min(255) as u8), 2)),
        [2, r, g, b, ..] => Some((
            TerminalColor::Rgb(
                (*r).min(255) as u8,
                (*g).min(255) as u8,
                (*b).min(255) as u8,
            ),
            4,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_basic_style_and_resets_intensity() {
        let mut attr = CellAttributes::default();
        apply(&[1, 2, 3, 4, 7], &mut attr);
        assert!(attr.bold);
        assert!(attr.faint);
        assert!(attr.italic);
        assert!(attr.underline);
        assert!(attr.inverse);

        apply(&[22, 23, 24, 27], &mut attr);
        assert!(!attr.bold);
        assert!(!attr.faint);
        assert!(!attr.italic);
        assert!(!attr.underline);
        assert!(!attr.inverse);
    }

    #[test]
    fn applies_extended_colors_and_resets_them() {
        let mut attr = CellAttributes::default();
        apply(&[38, 5, 202, 48, 2, 1, 2, 3, 58, 5, 45], &mut attr);
        assert_eq!(attr.fg, Some(TerminalColor::Palette(202)));
        assert_eq!(attr.bg, Some(TerminalColor::Rgb(1, 2, 3)));
        assert_eq!(attr.underline_color, Some(TerminalColor::Palette(45)));

        apply(&[39, 49, 59], &mut attr);
        assert_eq!(attr.fg, None);
        assert_eq!(attr.bg, None);
        assert_eq!(attr.underline_color, None);
    }

    #[test]
    fn applies_extended_underline_styles() {
        let mut attr = CellAttributes::default();
        apply(&[csi_params::SGR_UNDERLINE_STYLE_BASE + 3], &mut attr);
        assert_eq!(attr.underline_style, Some(StyledUnderline::Curly));
        apply(&[csi_params::SGR_UNDERLINE_STYLE_BASE], &mut attr);
        assert_eq!(attr.underline_style, None);
    }
}
