//! Box drawing, blocks and powerline separators, drawn rather than looked up.
//!
//! These characters have to *tile*. A table drawn from a font's glyphs shows
//! hairline gaps at every cell boundary, because the glyphs were designed to
//! sit inside a box with side bearings and the terminal is asking them to be
//! the box. Powerline separators are worse: a triangle from a patched font is
//! one pixel short of its neighbour and the prompt looks torn.
//!
//! So they are drawn as rectangles that reach exactly to the cell edge. It
//! also means a font with none of them -- which is most fonts -- still gets
//! them right, instead of a row of boxes where someone's prompt should be.

use crate::quads::{CellMetrics, Quad};

/// How thick a box-drawing line is at a given cell size.
///
/// Proportional, and never zero: a line that rounds away leaves a gap in the
/// middle of a table.
fn stroke(metrics: CellMetrics) -> f32 {
    (metrics.height / 14.0).round().max(1.0)
}

/// The branch mark, from the range patched fonts put their icons in.
pub const BRANCH: char = '\u{E0A0}';

/// A square with a line down the middle: two panes side by side.
///
/// A real code point rather than one of our own invention, so a program that
/// prints it gets the character it asked for and not an icon.
pub const SPLIT: char = '\u{25EB}';

/// Whether this is a character we draw ourselves.
///
/// Answered by asking for the drawing, so there is no second list to fall out
/// of step with the first. Callers use it to keep the shaper's hands off these
/// cells: a monospace font usually has its own box-drawing and powerline
/// glyphs, and if the shaper claims the column first, ours never runs. The
/// font's versions leave hairline gaps between cells -- that is the whole
/// reason to draw them.
pub fn draws(ch: char) -> bool {
    let unused = CellMetrics { width: 1.0, height: 1.0, baseline: 1.0 };
    quads_for(ch, 0.0, 0.0, unused, [0.0; 4]).is_some()
}

/// The rectangles that draw `ch` in the cell at `left`, `top`.
///
/// `None` when this is not a character we draw, so the caller falls through
/// to the font. Deliberately not paired with a separate "do we draw this?"
/// predicate: two lists drift, and a character claimed by one but missing
/// from the other is a blank cell where a table should be. That is not
/// hypothetical -- the first version had exactly that bug, and the
/// completeness test below is what found it.
pub fn quads_for(
    ch: char,
    left: f32,
    top: f32,
    metrics: CellMetrics,
    color: [f32; 4],
) -> Option<Vec<Quad>> {
    let width = metrics.width;
    let height = metrics.height;
    let thin = stroke(metrics);
    let thick = (thin * 2.0).min(height / 3.0);
    // The middle of the cell, snapped so a horizontal and a vertical line
    // meeting at a corner actually meet.
    let mid_x = left + ((width - thin) / 2.0).round();
    let mid_y = top + ((height - thin) / 2.0).round();

    let rect = |x: f32, y: f32, w: f32, h: f32| Quad {
        left: x,
        top: y,
        width: w,
        height: h,
        color,
    };
    // Lines run to the cell edge, which is what makes neighbours join.
    let left_arm = |weight: f32| rect(left, mid_y, mid_x - left + weight, weight);
    let right_arm = |weight: f32| rect(mid_x, mid_y, left + width - mid_x, weight);
    let up_arm = |weight: f32| rect(mid_x, top, weight, mid_y - top + weight);
    let down_arm = |weight: f32| rect(mid_x, mid_y, weight, top + height - mid_y);

    Some(match ch {
        // Horizontals and verticals.
        '\u{2500}' => vec![rect(left, mid_y, width, thin)],
        '\u{2501}' => vec![rect(left, mid_y, width, thick)],
        '\u{2502}' => vec![rect(mid_x, top, thin, height)],
        '\u{2503}' => vec![rect(mid_x, top, thick, height)],

        // Corners: down+right, down+left, up+right, up+left.
        '\u{250C}' => vec![right_arm(thin), down_arm(thin)],
        '\u{2510}' => vec![left_arm(thin), down_arm(thin)],
        '\u{2514}' => vec![right_arm(thin), up_arm(thin)],
        '\u{2518}' => vec![left_arm(thin), up_arm(thin)],

        // Tees.
        '\u{251C}' => vec![rect(mid_x, top, thin, height), right_arm(thin)],
        '\u{2524}' => vec![rect(mid_x, top, thin, height), left_arm(thin)],
        '\u{252C}' => vec![rect(left, mid_y, width, thin), down_arm(thin)],
        '\u{2534}' => vec![rect(left, mid_y, width, thin), up_arm(thin)],

        // Cross.
        '\u{253C}' => vec![
            rect(left, mid_y, width, thin),
            rect(mid_x, top, thin, height),
        ],

        // Every junction that involves a double line, from one model. See
        // `double_junction`: naming the four arms is enough to draw all 29 of
        // them, including the single/double mixes, without writing 29 shapes.
        ch if double_arms(ch).is_some() => {
            double_junction(double_arms(ch)?, left, top, metrics, color)
        }

        // Blocks. The eighths are what a progress bar is made of, and being
        // exact is the whole point of drawing them.
        '\u{2588}' => vec![rect(left, top, width, height)],
        '\u{2580}' => vec![rect(left, top, width, height / 2.0)],
        '\u{2584}' => vec![rect(left, top + height / 2.0, width, height / 2.0)],
        '\u{258C}' => vec![rect(left, top, width / 2.0, height)],
        '\u{2590}' => vec![rect(left + width / 2.0, top, width / 2.0, height)],
        '\u{2581}'..='\u{2587}' => {
            // U+2581 is one eighth from the bottom, U+2587 is seven.
            let eighths = (ch as u32 - 0x2580) as f32;
            let filled = height * eighths / 8.0;
            vec![rect(left, top + height - filled, width, filled)]
        }
        '\u{2589}'..='\u{258F}' => {
            // U+2589 is seven eighths from the left, U+258F is one.
            let eighths = 8.0 - (ch as u32 - 0x2588) as f32;
            vec![rect(left, top, width * eighths / 8.0, height)]
        }
        // Shades, as a colour rather than a stipple: a dithered pattern at
        // cell size is a moire, and the flat tone reads as the same weight.
        '\u{2591}' | '\u{2592}' | '\u{2593}' => {
            let level = match ch {
                '\u{2591}' => 0.25,
                '\u{2592}' => 0.5,
                _ => 0.75,
            };
            let mut shaded = color;
            shaded[3] *= level;
            vec![Quad {
                left,
                top,
                width,
                height,
                color: shaded,
            }]
        }

        // Powerline separators, as a staircase of rows. A triangle from
        // rectangles is not a triangle, but at a cell's height the steps are
        // a pixel each and it joins its neighbours exactly, which the font's
        // version does not.
        '\u{E0B0}' | '\u{E0B2}' => {
            let pointing_right = ch == '\u{E0B0}';
            let steps = height.ceil() as usize;
            (0..steps)
                .map(|step| {
                    let y = top + step as f32;
                    let along = (step as f32 / steps as f32 - 0.5).abs() * 2.0;
                    let run = width * (1.0 - along);
                    if pointing_right {
                        rect(left, y, run, 1.0)
                    } else {
                        rect(left + width - run, y, run, 1.0)
                    }
                })
                .collect()
        }
        // The hollow separators: the same shape as an outline.
        '\u{E0B1}' | '\u{E0B3}' => {
            let pointing_right = ch == '\u{E0B1}';
            let steps = height.ceil() as usize;
            (0..steps)
                .map(|step| {
                    let y = top + step as f32;
                    let along = (step as f32 / steps as f32 - 0.5).abs() * 2.0;
                    let run = width * (1.0 - along);
                    let x = if pointing_right {
                        left + run - thin
                    } else {
                        left + width - run
                    };
                    rect(x.max(left), y, thin, 1.0)
                })
                .collect()
        }

        // A square split down the middle -- what the button that splits a pane
        // is drawn as. Fonts that have this at all draw it at whatever weight
        // suits running text, which next to a hairline box-drawing table looks
        // like a different icon set; and the one this replaced was missing
        // from the font entirely and drew as an empty box.
        SPLIT => {
            // A hairline, not a box-drawing weight. This is an outline the
            // size of a letter, and at a table's weight the interior closes up
            // and it reads as a filled block with a slot in it.
            let weight = (thin - 1.0).max(1.0);
            let side = (width * 0.9).round();
            let box_left = left + ((width - side) / 2.0).round();
            let box_top = top + ((height - side) / 2.0).round();
            let mut quads =
                crate::strokes::rectangle(box_left, box_top, side, side, weight, color);
            quads.extend(crate::strokes::line(
                (box_left + ((side - weight) / 2.0).round(), box_top),
                (box_left + ((side - weight) / 2.0).round(), box_top + side),
                weight,
                color,
            ));
            quads
        }

        // The branch mark the status line puts before a branch name. It lives
        // in the private-use area, so only a patched font has it and every
        // other font draws an empty box -- which is why it is here rather than
        // looked up. Two stems and a join: the icon every git client uses, at
        // a size where a curve would be three pixels of staircase anyway.
        BRANCH => {
            let stem_x = left + (width * 0.30).round();
            let fork_x = left + (width * 0.70).round();
            let top_y = top + (height * 0.24).round();
            let bottom_y = top + (height * 0.78).round();
            let join_y = top + (height * 0.55).round();
            let node = (thin * 2.0).max(2.0);
            let dot = |x: f32, y: f32| {
                rect(
                    x - (node - thin) / 2.0,
                    y - (node - thin) / 2.0,
                    node,
                    node,
                )
            };
            let mut quads = crate::strokes::line((stem_x, top_y), (stem_x, bottom_y), thin, color);
            quads.extend(crate::strokes::line(
                (fork_x, top_y),
                (fork_x, join_y),
                thin,
                color,
            ));
            quads.extend(crate::strokes::line(
                (fork_x, join_y),
                (stem_x, join_y + (height * 0.12).round()),
                thin,
                color,
            ));
            quads.push(dot(stem_x, top_y));
            quads.push(dot(stem_x, bottom_y));
            quads.push(dot(fork_x, top_y));
            quads
        }

        // Dashed lines: the same run, broken. How many dashes and how
        // heavy is what separates these code points from each other.
        '\u{2504}' | '\u{2505}' | '\u{2508}' | '\u{2509}' => {
            let heavy = matches!(ch, '\u{2505}' | '\u{2509}');
            let dashes = if matches!(ch, '\u{2504}' | '\u{2505}') { 3 } else { 4 };
            dashed(left, mid_y, width, if heavy { thick } else { thin }, dashes, true, color)
        }
        '\u{2506}' | '\u{2507}' | '\u{250A}' | '\u{250B}' => {
            let heavy = matches!(ch, '\u{2507}' | '\u{250B}');
            let dashes = if matches!(ch, '\u{2506}' | '\u{2507}') { 3 } else { 4 };
            dashed(mid_x, top, height, if heavy { thick } else { thin }, dashes, false, color)
        }

        // Rounded corners, drawn square. At a cell's size the curve is a
        // couple of pixels of arc; square ones tile exactly and read the
        // same, which is the trade every terminal makes here.
        '\u{256D}' => vec![right_arm(thin), down_arm(thin)],
        '\u{256E}' => vec![left_arm(thin), down_arm(thin)],
        '\u{256F}' => vec![left_arm(thin), up_arm(thin)],
        '\u{2570}' => vec![right_arm(thin), up_arm(thin)],

        _ => return None,
    })
}

/// How a line leaves a cell.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Arm {
    None,
    Single,
    Double,
}

impl Arm {
    fn present(self) -> bool {
        self != Arm::None
    }
}

/// The four arms of a junction involving a double line, as (up, down, left,
/// right).
///
/// U+2550..=U+256C is nothing but junctions, so naming their arms is enough to
/// draw the whole block. The alternative -- a shape per character -- is 29
/// hand-drawn arms whose corners have to agree with each other, and the ones
/// this replaced did not: `\u{2554}` and friends were never written at all, so
/// a table mixed our `\u{2550}` with the font's corners and the two lines met
/// at different heights.
fn double_arms(ch: char) -> Option<[Arm; 4]> {
    use Arm::{Double as D, None as N, Single as S};
    Some(match ch {
        '\u{2550}' => [N, N, D, D],
        '\u{2551}' => [D, D, N, N],
        '\u{2552}' => [N, S, N, D],
        '\u{2553}' => [N, D, N, S],
        '\u{2554}' => [N, D, N, D],
        '\u{2555}' => [N, S, D, N],
        '\u{2556}' => [N, D, S, N],
        '\u{2557}' => [N, D, D, N],
        '\u{2558}' => [S, N, N, D],
        '\u{2559}' => [D, N, N, S],
        '\u{255A}' => [D, N, N, D],
        '\u{255B}' => [S, N, D, N],
        '\u{255C}' => [D, N, S, N],
        '\u{255D}' => [D, N, D, N],
        '\u{255E}' => [S, S, N, D],
        '\u{255F}' => [D, D, N, S],
        '\u{2560}' => [D, D, N, D],
        '\u{2561}' => [S, S, D, N],
        '\u{2562}' => [D, D, S, N],
        '\u{2563}' => [D, D, D, N],
        '\u{2564}' => [N, S, D, D],
        '\u{2565}' => [N, D, S, S],
        '\u{2566}' => [N, D, D, D],
        '\u{2567}' => [S, N, D, D],
        '\u{2568}' => [D, N, S, S],
        '\u{2569}' => [D, N, D, D],
        '\u{256A}' => [S, S, D, D],
        '\u{256B}' => [D, D, S, S],
        '\u{256C}' => [D, D, D, D],
        _ => return None,
    })
}

/// Draw a junction from its arms.
///
/// A double line is two rails; a single one is a single rail sitting where
/// both of a double's would be. Everything else is the same code, which is why
/// the single/double mixes need no cases of their own.
///
/// Where an arm is missing, the rail on that axis stops at a perpendicular
/// rail rather than at the cell edge -- at the far one when the corner is an
/// outer corner (nothing continues past it, as in `\u{2554}`) and at the near
/// one when something does (`\u{2566}`). Getting that backwards is what makes
/// a corner look like it overshoots.
fn double_junction(
    arms: [Arm; 4],
    left: f32,
    top: f32,
    metrics: CellMetrics,
    color: [f32; 4],
) -> Vec<Quad> {
    let [up, down, arm_left, arm_right] = arms;
    let thin = stroke(metrics);
    // Far enough apart to read as two lines at a cell's size, close enough
    // that the pair still reads as one border.
    let gap = thin * 2.0;
    let mid_x = left + ((metrics.width - thin) / 2.0).round();
    let mid_y = top + ((metrics.height - thin) / 2.0).round();
    let right_edge = left + metrics.width;
    let bottom_edge = top + metrics.height;

    let vertical_double = up == Arm::Double || down == Arm::Double;
    let horizontal_double = arm_left == Arm::Double || arm_right == Arm::Double;
    let (v_left_x, v_right_x) = if vertical_double {
        (mid_x - gap, mid_x + gap)
    } else {
        (mid_x, mid_x)
    };
    let (h_top_y, h_bot_y) = if horizontal_double {
        (mid_y - gap, mid_y + gap)
    } else {
        (mid_y, mid_y)
    };

    let mut quads = Vec::new();
    let mut rect = |x: f32, y: f32, width: f32, height: f32| {
        if width > 0.0 && height > 0.0 {
            quads.push(Quad { left: x, top: y, width, height, color });
        }
    };

    if up.present() || down.present() {
        // A rail's own side decides its corners. For a single rail there is
        // only one, so it takes the open side -- the one with no arm is where
        // the corner has to close.
        let rails: Vec<(f32, bool)> = if vertical_double {
            vec![(v_left_x, arm_left.present()), (v_right_x, arm_right.present())]
        } else {
            vec![(mid_x, arm_left.present() && arm_right.present())]
        };
        for (x, side) in rails {
            let starts = if up.present() {
                top
            } else if side {
                h_bot_y
            } else {
                h_top_y
            };
            let ends = if down.present() {
                bottom_edge
            } else if side {
                h_top_y + thin
            } else {
                h_bot_y + thin
            };
            if up.present() && down.present() && vertical_double && side {
                // The inside of the junction stays open: this is what makes
                // `\u{256C}` four corners rather than a filled cross.
                rect(x, top, thin, h_top_y - top);
                rect(x, h_bot_y + thin, thin, bottom_edge - h_bot_y - thin);
            } else {
                rect(x, starts, thin, ends - starts);
            }
        }
    }

    if arm_left.present() || arm_right.present() {
        let rails: Vec<(f32, bool)> = if horizontal_double {
            vec![(h_top_y, up.present()), (h_bot_y, down.present())]
        } else {
            vec![(mid_y, up.present() && down.present())]
        };
        for (y, side) in rails {
            let starts = if arm_left.present() {
                left
            } else if side {
                v_right_x
            } else {
                v_left_x
            };
            let ends = if arm_right.present() {
                right_edge
            } else if side {
                v_left_x + thin
            } else {
                v_right_x + thin
            };
            if arm_left.present() && arm_right.present() && horizontal_double && side {
                rect(left, y, v_left_x - left, thin);
                rect(v_right_x + thin, y, right_edge - v_right_x - thin, thin);
            } else {
                rect(starts, y, ends - starts, thin);
            }
        }
    }

    quads
}

/// A broken line along a cell, in `count` dashes with gaps between them.
#[allow(clippy::too_many_arguments)]
fn dashed(
    x: f32,
    y: f32,
    span: f32,
    weight: f32,
    count: usize,
    horizontal: bool,
    color: [f32; 4],
) -> Vec<Quad> {
    // A gap of a third, so the dashes still read as one line.
    let step = span / count as f32;
    let dash = step * 0.66;
    (0..count)
        .map(|index| {
            let along = index as f32 * step;
            if horizontal {
                Quad { left: x + along, top: y, width: dash, height: weight, color }
            } else {
                Quad { left: x, top: y + along, width: weight, height: dash, color }
            }
        })
        .collect()
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

    fn white() -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }

    fn draw(ch: char) -> Vec<Quad> {
        quads_for(ch, 0.0, 0.0, metrics(), white()).unwrap_or_default()
    }

    fn is_drawn(ch: char) -> bool {
        quads_for(ch, 0.0, 0.0, metrics(), white()).is_some()
    }

    #[test]
    fn ordinary_characters_are_left_to_the_font() {
        assert!(!is_drawn('a'));
        assert!(!is_drawn('中'));
        assert!(draw('a').is_empty());
    }

    #[test]
    fn box_drawing_and_powerline_are_ours() {
        assert!(is_drawn('─'));
        assert!(is_drawn('┼'));
        assert!(is_drawn('█'));
        assert!(is_drawn('\u{E0B0}'));
    }

    #[test]
    fn a_horizontal_line_reaches_both_cell_edges() {
        // The whole reason for drawing these: a font's glyph stops short and
        // a table shows a hairline gap at every boundary.
        let quads = draw('─');
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].left, 0.0);
        assert_eq!(quads[0].width, metrics().width);
    }

    #[test]
    fn a_vertical_line_reaches_top_and_bottom() {
        let quads = draw('│');
        assert_eq!(quads[0].top, 0.0);
        assert_eq!(quads[0].height, metrics().height);
    }

    #[test]
    fn a_corner_stops_at_the_middle_on_the_sides_it_has_no_arm() {
        // Down-and-right: nothing above, nothing to the left.
        let quads = draw('┌');
        let leftmost = quads.iter().map(|q| q.left).fold(f32::MAX, f32::min);
        let topmost = quads.iter().map(|q| q.top).fold(f32::MAX, f32::min);
        assert!(leftmost > 0.0, "a down-right corner has no left arm");
        assert!(topmost > 0.0, "and no upward arm");
        assert!(
            quads.iter().any(|q| q.left + q.width >= metrics().width),
            "its right arm reaches the edge so it joins the next cell"
        );
    }

    #[test]
    fn a_cross_spans_both_ways() {
        let quads = draw('┼');
        assert!(quads.iter().any(|q| q.width == metrics().width));
        assert!(quads.iter().any(|q| q.height == metrics().height));
    }

    #[test]
    fn a_line_and_a_corner_meet_at_the_same_place() {
        // A row of `──┐` looks broken if the corner's arm sits a pixel off
        // the line's.
        let line = draw('─');
        let corner = draw('┐');
        let arm = corner
            .iter()
            .find(|q| q.width > q.height)
            .expect("the corner has a horizontal arm");
        assert_eq!(arm.top, line[0].top);
        assert_eq!(arm.height, line[0].height);
    }

    #[test]
    fn a_full_block_fills_its_cell_exactly() {
        let quads = draw('█');
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].width, metrics().width);
        assert_eq!(quads[0].height, metrics().height);
    }

    #[test]
    fn the_eighth_blocks_climb_in_even_steps() {
        // What a progress bar is made of: each one has to be exactly an
        // eighth taller than the last or the bar stutters.
        let mut previous = 0.0;
        for (index, ch) in "▁▂▃▄▅▆▇".chars().enumerate() {
            let quads = draw(ch);
            let height = quads[0].height;
            assert!(
                (height - metrics().height * (index + 1) as f32 / 8.0).abs() < 0.01,
                "{ch} should be {} eighths tall",
                index + 1
            );
            assert!(height > previous);
            previous = height;
            assert_eq!(
                quads[0].top + height,
                metrics().height,
                "{ch} grows from the bottom"
            );
        }
    }

    #[test]
    fn the_left_blocks_grow_from_the_left_edge() {
        for ch in "▏▎▍▌▋▊▉".chars() {
            let quads = draw(ch);
            assert_eq!(quads[0].left, 0.0, "{ch} starts at the left edge");
            assert!(quads[0].width <= metrics().width);
        }
    }

    #[test]
    fn shades_differ_only_in_weight() {
        let light = draw('░')[0].color[3];
        let medium = draw('▒')[0].color[3];
        let dark = draw('▓')[0].color[3];
        assert!(light < medium && medium < dark);
        assert!(dark < 1.0, "the dark shade is not a solid block");
    }

    #[test]
    fn a_powerline_separator_covers_its_cell_from_edge_to_edge() {
        // Torn prompts come from a triangle that does not reach the boundary.
        let quads = draw('\u{E0B0}');
        assert!(!quads.is_empty());
        let widest = quads.iter().map(|q| q.width).fold(0.0, f32::max);
        assert!(
            (widest - metrics().width).abs() < 0.5,
            "at its widest the separator should span the cell"
        );
        for quad in &quads {
            assert!(quad.top >= 0.0 && quad.top < metrics().height);
        }
    }

    #[test]
    fn the_two_separators_point_opposite_ways() {
        let right = draw('\u{E0B0}');
        let left = draw('\u{E0B2}');
        // The right-pointing one is widest at the top-left; the left-pointing
        // one is pushed against the right edge.
        assert_eq!(right[0].left, 0.0);
        assert!(left.iter().any(|q| q.left + q.width >= metrics().width - 0.01));
    }

    #[test]
    fn every_drawn_character_produces_something() {
        // A character claimed and then not drawn is a blank cell where a
        // table should be -- worse than leaving it to the font.
        for code in (0x2500u32..=0x257F).chain(0x2580..=0x259F).chain(0xE0B0..=0xE0B3) {
            let ch = char::from_u32(code).unwrap();
            let Some(quads) = quads_for(ch, 0.0, 0.0, metrics(), white()) else {
                // Left to the font, which is a fine answer for the forms we
                // have not drawn yet.
                continue;
            };
            assert!(
                !quads.is_empty(),
                "{ch} (U+{code:04X}) is claimed but draws nothing"
            );
        }
    }
}

#[cfg(test)]
mod junction_tests {
    use super::*;

    fn metrics() -> CellMetrics {
        CellMetrics { width: 10.0, height: 20.0, baseline: 16.0 }
    }

    fn draw(ch: char) -> Vec<Quad> {
        quads_for(ch, 0.0, 0.0, metrics(), [1.0; 4]).unwrap_or_default()
    }

    /// Every junction in the block, not just the two lines that used to be
    /// here. A table drawn with our `═` and the font's `╔` meets at two
    /// different heights, which looks worse than using the font throughout.
    #[test]
    fn the_whole_double_block_is_ours() {
        for code in 0x2550u32..=0x256C {
            let ch = char::from_u32(code).unwrap();
            assert!(draws(ch), "{ch} (U+{code:04X}) fell through to the font");
            assert!(!draw(ch).is_empty(), "{ch} (U+{code:04X}) drew nothing");
        }
    }

    /// Two `═` side by side, and the pair has to be continuous both times.
    #[test]
    fn a_double_horizontal_tiles() {
        let here = quads_for('\u{2550}', 0.0, 0.0, metrics(), [1.0; 4]).unwrap();
        let next = quads_for('\u{2550}', metrics().width, 0.0, metrics(), [1.0; 4]).unwrap();
        assert_eq!(here.len(), 2, "a double is two lines");
        for line in &here {
            let joins = next
                .iter()
                .any(|other| other.top == line.top && other.left <= line.left + line.width);
            assert!(joins, "the line at {} does not reach its neighbour", line.top);
        }
    }

    /// The two rails of `═` must sit either side of where a single `─` runs,
    /// or a table that mixes them steps up or down at the join.
    #[test]
    fn a_double_straddles_the_single_it_replaces() {
        let single = draw('\u{2500}');
        let double = draw('\u{2550}');
        let middle = single[0].top;
        assert!(double[0].top < middle, "{:?}", double[0]);
        assert!(double[1].top > middle, "{:?}", double[1]);
    }

    /// `╔` has no arm going up or left, so nothing may run out of the cell
    /// that way. The rails themselves straddle the middle -- that is what
    /// makes a double a double -- so the check is against the cell's edges.
    #[test]
    fn a_double_corner_does_not_overshoot() {
        for quad in draw('\u{2554}') {
            assert!(quad.left > 0.0, "something runs out the left: {quad:?}");
            assert!(quad.top > 0.0, "something runs out the top: {quad:?}");
        }
    }

    /// And it does reach the two edges it has arms for, or the corner will not
    /// meet the lines either side of it.
    #[test]
    fn a_double_corner_reaches_the_edges_it_has_arms_for() {
        let quads = draw('\u{2554}');
        let rightward = quads
            .iter()
            .filter(|quad| quad.left + quad.width >= metrics().width)
            .count();
        let downward = quads
            .iter()
            .filter(|quad| quad.top + quad.height >= metrics().height)
            .count();
        assert_eq!(rightward, 2, "both rails must reach right: {quads:?}");
        assert_eq!(downward, 2, "both rails must reach down: {quads:?}");
    }

    /// `╔` over `╚` draws a closed left edge: the corners' downward and
    /// upward arms have to line up, or the border has a notch in it.
    #[test]
    fn the_corners_of_a_double_box_line_up() {
        let opening = draw('\u{2554}');
        let closing = draw('\u{255A}');
        let verticals = |quads: &[Quad]| -> Vec<f32> {
            let mut lefts: Vec<f32> = quads
                .iter()
                .filter(|quad| quad.width <= stroke(metrics()))
                .map(|quad| quad.left)
                .collect();
            lefts.sort_by(f32::total_cmp);
            lefts
        };
        assert_eq!(verticals(&opening), verticals(&closing));
        assert_eq!(verticals(&opening).len(), 2, "a double edge is two rails");
    }

    /// `╬` is four corners with an opening in the middle, not a filled cross.
    #[test]
    fn the_double_cross_stays_open_in_the_middle() {
        let quads = draw('\u{256C}');
        let centre_x = metrics().width / 2.0;
        let centre_y = metrics().height / 2.0;
        let covers_centre = quads.iter().any(|quad| {
            quad.left <= centre_x
                && quad.left + quad.width > centre_x
                && quad.top <= centre_y
                && quad.top + quad.height > centre_y
        });
        assert!(!covers_centre, "the middle was filled in: {quads:?}");
    }

    /// `╠` keeps its outer rail whole; only the rail the arms leave from opens.
    #[test]
    fn a_double_tee_keeps_its_outer_rail_whole() {
        let quads = draw('\u{2560}');
        let full_height = quads
            .iter()
            .filter(|quad| quad.height >= metrics().height)
            .count();
        assert_eq!(full_height, 1, "expected one unbroken rail: {quads:?}");
    }

    /// `╪` is a single line crossing a double one, and a single line has
    /// nothing to open up -- it runs the whole way.
    #[test]
    fn a_single_line_through_a_double_one_is_not_broken() {
        let quads = draw('\u{256A}');
        let spans_height = quads
            .iter()
            .any(|quad| quad.height >= metrics().height && quad.width <= stroke(metrics()));
        assert!(spans_height, "the single vertical was broken: {quads:?}");
    }

    /// A single/double mix joins a double neighbour on one side and a single
    /// neighbour on the other, so both rails have to reach their own edge.
    #[test]
    fn a_mixed_junction_reaches_the_edge_its_double_arm_points_at() {
        // U+255E: single vertical, double right.
        let quads = draw('\u{255E}');
        let reaching = quads
            .iter()
            .filter(|quad| quad.left + quad.width >= metrics().width)
            .count();
        assert_eq!(reaching, 2, "both rails must reach the right edge: {quads:?}");
    }
}

#[cfg(test)]
mod branch_tests {
    use super::*;

    /// The branch mark is in the private-use area: a font that has not been
    /// patched draws an empty box there, and an empty box in front of a branch
    /// name looks like a bug in the terminal.
    #[test]
    fn the_branch_mark_is_drawn_rather_than_looked_up() {
        assert!(draws(BRANCH));
    }

    /// It has to read as a branch, which means two stems at different heights
    /// joined in the middle -- not one line.
    #[test]
    fn the_branch_mark_has_two_stems_and_a_join() {
        let metrics = CellMetrics {
            width: 9.0,
            height: 20.0,
            baseline: 15.0,
        };
        let quads = quads_for(BRANCH, 0.0, 0.0, metrics, [1.0; 4]).expect("drawn");
        let columns: std::collections::BTreeSet<i32> = quads
            .iter()
            .map(|quad| quad.left.round() as i32)
            .collect();
        assert!(
            columns.len() >= 2,
            "the mark is one column wide: {columns:?}"
        );
        let tallest = quads
            .iter()
            .map(|quad| quad.height)
            .fold(0.0f32, f32::max);
        assert!(
            tallest > metrics.height * 0.4,
            "no stem: tallest piece is {tallest}"
        );
    }

    /// And it has to stay inside its cell, or it collides with the branch name
    /// that follows it.
    #[test]
    fn the_branch_mark_stays_in_its_cell() {
        let metrics = CellMetrics {
            width: 9.0,
            height: 20.0,
            baseline: 15.0,
        };
        for quad in quads_for(BRANCH, 100.0, 50.0, metrics, [1.0; 4]).expect("drawn") {
            assert!(quad.left >= 100.0, "{quad:?} starts left of the cell");
            assert!(
                quad.left + quad.width <= 100.0 + metrics.width + 1.0,
                "{quad:?} runs past the cell"
            );
            assert!(quad.top >= 50.0, "{quad:?} starts above the cell");
            assert!(
                quad.top + quad.height <= 50.0 + metrics.height + 1.0,
                "{quad:?} runs below the cell"
            );
        }
    }
}

#[cfg(test)]
mod split_tests {
    use super::*;

    /// The button that splits a pane was drawn with a code point the font did
    /// not have, so it appeared as an empty box -- a control with no icon,
    /// next to two that had one.
    #[test]
    fn the_split_mark_is_drawn_rather_than_looked_up() {
        assert!(draws(SPLIT));
    }

    /// A square with a line down the middle: two panes, side by side.
    #[test]
    fn the_split_mark_is_a_box_divided_in_two() {
        let metrics = CellMetrics {
            width: 18.0,
            height: 20.0,
            baseline: 15.0,
        };
        let quads = quads_for(SPLIT, 0.0, 0.0, metrics, [1.0; 4]).expect("drawn");
        let mut uprights: Vec<f32> = quads
            .iter()
            .filter(|quad| quad.height > metrics.height * 0.4)
            .map(|quad| quad.left)
            .collect();
        uprights.sort_by(f32::total_cmp);
        assert_eq!(
            uprights.len(),
            3,
            "a split mark has two sides and a divider: {uprights:?}"
        );
        let divider = uprights[1];
        assert!(
            divider > uprights[0] && divider < uprights[2],
            "the divider is not between the sides: {uprights:?}"
        );
    }

    #[test]
    fn the_split_mark_stays_in_its_cell() {
        let metrics = CellMetrics {
            width: 18.0,
            height: 20.0,
            baseline: 15.0,
        };
        for quad in quads_for(SPLIT, 40.0, 10.0, metrics, [1.0; 4]).expect("drawn") {
            assert!(quad.left >= 40.0, "{quad:?} starts left of the cell");
            assert!(
                quad.left + quad.width <= 40.0 + metrics.width + 1.0,
                "{quad:?} runs past the cell"
            );
            assert!(quad.top >= 10.0, "{quad:?} starts above the cell");
            assert!(
                quad.top + quad.height <= 10.0 + metrics.height + 1.0,
                "{quad:?} runs below the cell"
            );
        }
    }
}
