//! The Unterm mark, rasterized from the icon's own geometry.
//!
//! The source of truth is the Logo A geometry the icon assets are
//! generated from (`assets/windows/terminal.ico` and friends): a
//! round-capped prompt chevron, and the amber status dot at its upper
//! right — the terminal that tells you its state. The mark is two
//! parts because the icon is two colours: the chevron takes the
//! chrome's foreground, the dot keeps the icon's amber.

use unterm_engine::next_core::font_raster::RasterizedGlyph;

// The icon's geometry, in its own 96-unit box.
const CHEVRON: &[(f32, f32)] = &[(30.0, 32.0), (50.0, 48.0), (30.0, 64.0)];
const CHEVRON_HALF: f32 = 9.0 / 2.0;
const DOT_CENTER: (f32, f32) = (66.0, 34.0);
const DOT_RADIUS: f32 = 9.0;

// What the ink actually covers, stroke caps included: the raster box maps to
// this rectangle so the mark fills what it is given instead of floating in
// the icon's margins.
const CONTENT_LEFT: f32 = CHEVRON[0].0 - CHEVRON_HALF;
const CONTENT_TOP: f32 = DOT_CENTER.1 - DOT_RADIUS;
const CONTENT_RIGHT: f32 = DOT_CENTER.0 + DOT_RADIUS;
const CONTENT_BOTTOM: f32 = 64.0 + CHEVRON_HALF;
const CONTENT_WIDTH: f32 = CONTENT_RIGHT - CONTENT_LEFT;
const CONTENT_HEIGHT: f32 = CONTENT_BOTTOM - CONTENT_TOP;

/// Width over height of the drawn mark, for a box that keeps its shape.
pub const ASPECT: f32 = CONTENT_WIDTH / CONTENT_HEIGHT;

/// The icon's amber — the same "needs you" colour the status language
/// uses everywhere else.
pub const DOT_COLOR: [f32; 4] = [
    0xE8 as f32 / 255.0,
    0xB3 as f32 / 255.0,
    0x4B as f32 / 255.0,
    1.0,
];

/// The mark's two parts, rasterized into one shared box so they overlay.
pub struct Mark {
    pub chevron: RasterizedGlyph,
    pub dot: RasterizedGlyph,
}

/// Distance from a point to a segment.
fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (px, py) = (p.0 - a.0, p.1 - a.1);
    let (bx, by) = (b.0 - a.0, b.1 - a.1);
    let len_sq = bx * bx + by * by;
    let t = if len_sq <= f32::EPSILON {
        0.0
    } else {
        ((px * bx + py * by) / len_sq).clamp(0.0, 1.0)
    };
    let (dx, dy) = (px - t * bx, py - t * by);
    (dx * dx + dy * dy).sqrt()
}

fn polyline_distance(p: (f32, f32), points: &[(f32, f32)]) -> f32 {
    points
        .windows(2)
        .map(|pair| segment_distance(p, pair[0], pair[1]))
        .fold(f32::MAX, f32::min)
}

fn glyph(width: usize, height: usize, coverage: Vec<u8>) -> RasterizedGlyph {
    RasterizedGlyph {
        coverage,
        width,
        height,
        bearing_x: 0,
        bearing_y: height as i32,
        advance_x: width as i32,
    }
}

/// Rasterize the mark into its two coverage bitmaps, anti-aliased.
///
/// Coverage falls off over one device pixel around each edge, which is what
/// the font rasterizer would have done with the same shapes. Distance to a
/// round-capped polyline gives the chevron its round caps for free.
pub fn rasterize(width: usize, height: usize) -> Mark {
    let mut chevron = vec![0u8; width * height];
    let mut dot = vec![0u8; width * height];

    // Pixels per icon unit; the box arrives in the mark's own ASPECT, so one
    // scale serves both axes.
    let scale = (height as f32 / CONTENT_HEIGHT).max(f32::EPSILON);
    for y in 0..height {
        for x in 0..width {
            let u = CONTENT_LEFT + (x as f32 + 0.5) / scale;
            let v = CONTENT_TOP + (y as f32 + 0.5) / scale;
            let p = (u, v);
            let at = y * width + x;

            let chevron_alpha =
                (CHEVRON_HALF - polyline_distance(p, CHEVRON)) * scale + 0.5;
            chevron[at] = (chevron_alpha.clamp(0.0, 1.0) * 255.0) as u8;

            let from_center =
                ((u - DOT_CENTER.0).powi(2) + (v - DOT_CENTER.1).powi(2)).sqrt();
            let dot_alpha = (DOT_RADIUS - from_center) * scale + 0.5;
            dot[at] = (dot_alpha.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    Mark {
        chevron: glyph(width, height, chevron),
        dot: glyph(width, height, dot),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink(glyph: &RasterizedGlyph) -> usize {
        glyph.coverage.iter().filter(|&&c| c > 0).count()
    }

    #[test]
    fn the_mark_is_drawn_and_anti_aliased() {
        let mark = rasterize(37, 40);
        for (name, part) in [("chevron", &mark.chevron), ("dot", &mark.dot)] {
            assert!(ink(part) > 10, "{name} has no ink");
            let partial = part.coverage.iter().filter(|&&c| c > 0 && c < 255).count();
            assert!(partial > 10, "{name}'s edges are not anti-aliased");
        }
    }

    /// The parts sit where the icon puts them: the chevron on the left,
    /// the dot at the upper right — exactly the icon's arrangement.
    #[test]
    fn the_parts_keep_the_icons_arrangement() {
        let mark = rasterize(37, 40);
        let centroid = |part: &RasterizedGlyph| {
            let (mut sx, mut sy, mut n) = (0.0f32, 0.0f32, 0.0f32);
            for y in 0..part.height {
                for x in 0..part.width {
                    let c = part.coverage[y * part.width + x] as f32;
                    sx += x as f32 * c;
                    sy += y as f32 * c;
                    n += c;
                }
            }
            (sx / n / part.width as f32, sy / n / part.height as f32)
        };
        let chevron = centroid(&mark.chevron);
        assert!(
            chevron.0 < 0.45,
            "the chevron is not on the left: {chevron:?}"
        );
        let dot = centroid(&mark.dot);
        assert!(
            dot.0 > 0.65 && dot.1 < 0.35,
            "the dot is not at the upper right: {dot:?}"
        );
    }

    /// Empty at the far corners: ink that bleeds to the box's edge smears
    /// into the atlas's neighbouring glyph.
    #[test]
    fn the_mark_keeps_out_of_its_own_corners() {
        let mark = rasterize(37, 44);
        for part in [&mark.chevron, &mark.dot] {
            for (x, y) in [(0, 43), (36, 43)] {
                assert_eq!(part.coverage[y * 37 + x], 0, "ink in the corner {x},{y}");
            }
        }
    }

    /// Size follows the box it is asked for.
    #[test]
    fn the_mark_scales_with_its_box() {
        let small = rasterize(18, 20);
        let large = rasterize(37, 40);
        assert!(ink(&large.chevron) > ink(&small.chevron) * 2);
    }
}
