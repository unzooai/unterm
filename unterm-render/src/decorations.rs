//! Underlines, strikethrough, and the rest of the lines a cell can carry.
//!
//! `ls --color` marks some file types with an underline, man pages underline
//! their headings, compilers underline the span they are complaining about,
//! and a terminal that drops them silently loses information the program
//! deliberately sent.
//!
//! All of it is rectangles, which is why it lives here rather than in the font
//! layer: nothing below needs a glyph, so nothing below needs a font.

use crate::quads::{CellMetrics, Quad};
use unterm_engine::{CellStyle, StyledUnderline};

/// How thick a line is at a given cell height.
///
/// Proportional rather than fixed: a one-pixel underline disappears on a
/// high-DPI screen, and a three-pixel one swallows the descenders on a small
/// font.
pub fn thickness(metrics: CellMetrics) -> f32 {
    (metrics.height / 14.0).round().max(1.0)
}

/// Where the underline sits, in pixels from the top of the cell.
///
/// Below the baseline by a little, so it clears the descenders of `g` and `y`
/// rather than cutting through them.
pub fn underline_top(metrics: CellMetrics) -> f32 {
    let below = (metrics.height - metrics.baseline) * 0.4;
    (metrics.baseline + below).min(metrics.height - thickness(metrics))
}

/// The lines a cell's style asks for.
///
/// One quad per stroke: a double underline is two, a dotted one is a row of
/// short ones. The caller places them; this only decides the geometry.
pub fn quads_for(
    style: &CellStyle,
    left: f32,
    top: f32,
    width: f32,
    metrics: CellMetrics,
    color: [f32; 4],
) -> Vec<Quad> {
    let mut out = Vec::new();
    let stroke = thickness(metrics);

    if style.underline || style.underline_style.is_some() {
        let line_top = top + underline_top(metrics);
        match style.underline_style.unwrap_or(StyledUnderline::Single) {
            StyledUnderline::Single => out.push(Quad {
                left,
                top: line_top,
                width,
                height: stroke,
                color,
            }),
            StyledUnderline::Double => {
                // The second line goes above the first, so the pair still
                // clears the descenders the single one was placed to clear.
                out.push(Quad {
                    left,
                    top: line_top,
                    width,
                    height: stroke,
                    color,
                });
                out.push(Quad {
                    left,
                    top: line_top - stroke * 2.0,
                    width,
                    height: stroke,
                    color,
                });
            }
            StyledUnderline::Dotted => {
                out.extend(dashes(left, line_top, width, stroke, color, 1.0))
            }
            StyledUnderline::Dashed => {
                out.extend(dashes(left, line_top, width, stroke, color, 3.0))
            }
            // Drawn as a dense dash rather than a sine wave: a curl needs a
            // shader or a texture, and a distinct-looking line is worth more
            // than a missing one.
            StyledUnderline::Curly => out.extend(dashes(left, line_top, width, stroke, color, 2.0)),
        }
    }

    if style.strikethrough {
        // Through the middle of the x-height, which is where a reader expects
        // a line that means "deleted".
        out.push(Quad {
            left,
            top: top + metrics.baseline * 0.65,
            width,
            height: stroke,
            color,
        });
    }

    if style.overline {
        out.push(Quad {
            left,
            top,
            width,
            height: stroke,
            color,
        });
    }

    out
}

/// A broken line: `on` pixels of stroke, then the same again of gap.
fn dashes(left: f32, top: f32, width: f32, stroke: f32, color: [f32; 4], on: f32) -> Vec<Quad> {
    let dash = (stroke * on).max(1.0);
    let step = dash * 2.0;
    let mut out = Vec::new();
    let mut x = left;
    while x < left + width {
        out.push(Quad {
            left: x,
            top,
            width: dash.min(left + width - x),
            height: stroke,
            color,
        });
        x += step;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> CellMetrics {
        CellMetrics {
            width: 10.0,
            height: 20.0,
            baseline: 15.0,
        }
    }

    fn style() -> CellStyle {
        CellStyle::default()
    }

    fn white() -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }

    #[test]
    fn a_plain_cell_has_no_lines() {
        assert!(quads_for(&style(), 0.0, 0.0, 10.0, metrics(), white()).is_empty());
    }

    #[test]
    fn an_underline_sits_below_the_baseline() {
        let mut style = style();
        style.underline = true;
        let quads = quads_for(&style, 0.0, 0.0, 10.0, metrics(), white());
        assert_eq!(quads.len(), 1);
        assert!(
            quads[0].top > metrics().baseline,
            "an underline through the descenders is worse than none"
        );
        assert!(quads[0].top + quads[0].height <= metrics().height);
    }

    #[test]
    fn a_double_underline_is_two_lines_that_do_not_overlap() {
        let mut style = style();
        style.underline_style = Some(StyledUnderline::Double);
        let quads = quads_for(&style, 0.0, 0.0, 10.0, metrics(), white());
        assert_eq!(quads.len(), 2);
        let gap = (quads[0].top - quads[1].top).abs();
        assert!(gap >= quads[0].height, "two lines drawn as one is one line");
    }

    #[test]
    fn a_dotted_underline_is_a_row_of_gaps() {
        let mut style = style();
        style.underline_style = Some(StyledUnderline::Dotted);
        let quads = quads_for(&style, 0.0, 0.0, 40.0, metrics(), white());
        assert!(
            quads.len() > 1,
            "a dotted line with one dash is a solid one"
        );
        for quad in &quads {
            assert!(quad.left >= 0.0 && quad.left + quad.width <= 40.0 + 0.01);
        }
    }

    #[test]
    fn a_dashed_underline_has_longer_strokes_than_a_dotted_one() {
        let mut dotted = style();
        dotted.underline_style = Some(StyledUnderline::Dotted);
        let mut dashed = style();
        dashed.underline_style = Some(StyledUnderline::Dashed);
        let a = quads_for(&dotted, 0.0, 0.0, 60.0, metrics(), white());
        let b = quads_for(&dashed, 0.0, 0.0, 60.0, metrics(), white());
        assert!(b[0].width > a[0].width);
    }

    #[test]
    fn strikethrough_crosses_the_text_not_the_gap_below_it() {
        let mut style = style();
        style.strikethrough = true;
        let quads = quads_for(&style, 0.0, 0.0, 10.0, metrics(), white());
        assert_eq!(quads.len(), 1);
        assert!(quads[0].top > 0.0 && quads[0].top < metrics().baseline);
    }

    #[test]
    fn an_overline_sits_at_the_top_of_the_cell() {
        let mut style = style();
        style.overline = true;
        let quads = quads_for(&style, 0.0, 0.0, 10.0, metrics(), white());
        assert_eq!(quads[0].top, 0.0);
    }

    #[test]
    fn a_cell_can_carry_several_lines_at_once() {
        let mut style = style();
        style.underline = true;
        style.strikethrough = true;
        style.overline = true;
        assert_eq!(
            quads_for(&style, 0.0, 0.0, 10.0, metrics(), white()).len(),
            3
        );
    }

    #[test]
    fn lines_scale_with_the_cell_rather_than_staying_one_pixel() {
        let small = CellMetrics {
            width: 6.0,
            height: 12.0,
            baseline: 9.0,
        };
        let large = CellMetrics {
            width: 24.0,
            height: 48.0,
            baseline: 36.0,
        };
        assert!(
            thickness(large) > thickness(small),
            "a one-pixel line disappears on a high-DPI screen"
        );
        assert!(thickness(small) >= 1.0, "and never rounds away to nothing");
    }

    #[test]
    fn a_line_spans_the_whole_run_it_was_given() {
        let mut style = style();
        style.underline = true;
        let quads = quads_for(&style, 30.0, 0.0, 20.0, metrics(), white());
        assert_eq!(quads[0].left, 30.0);
        assert_eq!(quads[0].width, 20.0, "a wide cell underlines both columns");
    }
}
