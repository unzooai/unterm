//! The Unterm mark, rasterized from the icon's own geometry.
//!
//! The source of truth is `assets/icon/unterm-icon-small.svg` -- the optical
//! master the Dock and installer icons are built from: a heavy hook, a dot at
//! its start, an arrowhead at its open end. The first cut of this file drew a
//! different, thinner squiggle in one colour, and the first thing anyone said
//! on seeing it was that the logo was wrong. The mark is three parts because
//! the icon is three colours: the hook takes the chrome's foreground, the dot
//! and the arrow keep the icon's own teals.

use unterm_engine::next_core::font_raster::RasterizedGlyph;

// The icon's geometry, in its own 64-unit box.
//
// Hook: `M20.5 14.5 V35 c0 10.5 5 15.5 13.5 15.5 c8.5 0 12.5-5 12.5-12.5`.
const HOOK_START: (f32, f32) = (20.5, 14.5);
const HOOK_CORNER: (f32, f32) = (20.5, 35.0);
const HOOK_CURVES: [[(f32, f32); 4]; 2] = [
    [(20.5, 35.0), (20.5, 45.5), (25.5, 50.5), (34.0, 50.5)],
    [(34.0, 50.5), (42.5, 50.5), (46.5, 45.5), (46.5, 38.0)],
];
const HOOK_HALF: f32 = 7.5 / 2.0;
const DOT_CENTER: (f32, f32) = (20.5, 14.5);
const DOT_RADIUS: f32 = 3.75;
// Arrow: `m41.5 32 l7 6 l-7 6`.
const ARROW: &[(f32, f32)] = &[(41.5, 32.0), (48.5, 38.0), (41.5, 44.0)];
const ARROW_HALF: f32 = 7.0 / 2.0;

// What the ink actually covers, stroke caps included: the raster box maps to
// this rectangle so the mark fills what it is given instead of floating in
// the icon's margins.
const CONTENT_LEFT: f32 = HOOK_START.0 - HOOK_HALF;
const CONTENT_TOP: f32 = HOOK_START.1 - HOOK_HALF;
const CONTENT_RIGHT: f32 = 48.5 + ARROW_HALF;
const CONTENT_BOTTOM: f32 = 50.5 + HOOK_HALF;
const CONTENT_WIDTH: f32 = CONTENT_RIGHT - CONTENT_LEFT;
const CONTENT_HEIGHT: f32 = CONTENT_BOTTOM - CONTENT_TOP;

/// Width over height of the drawn mark, for a box that keeps its shape.
pub const ASPECT: f32 = CONTENT_WIDTH / CONTENT_HEIGHT;

/// The icon's teals, as the SVG names them.
pub const DOT_COLOR: [f32; 4] = [
    0x77 as f32 / 255.0,
    0xD9 as f32 / 255.0,
    0xC1 as f32 / 255.0,
    1.0,
];
pub const ARROW_COLOR: [f32; 4] = [
    0x55 as f32 / 255.0,
    0xC6 as f32 / 255.0,
    0xAA as f32 / 255.0,
    1.0,
];

/// The mark's three parts, rasterized into one shared box so they overlay.
pub struct Mark {
    pub hook: RasterizedGlyph,
    pub dot: RasterizedGlyph,
    pub arrow: RasterizedGlyph,
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

/// The hook flattened to a polyline, in icon units.
fn hook_points() -> Vec<(f32, f32)> {
    let mut points = vec![HOOK_START, HOOK_CORNER];
    for [p0, p1, p2, p3] in HOOK_CURVES {
        // Sixteen steps per cubic: at chrome sizes the chords land well under
        // half a pixel apart, and the rasterizer's falloff hides the rest.
        for step in 1..=16 {
            let t = step as f32 / 16.0;
            let u = 1.0 - t;
            let x = u * u * u * p0.0
                + 3.0 * u * u * t * p1.0
                + 3.0 * u * t * t * p2.0
                + t * t * t * p3.0;
            let y = u * u * u * p0.1
                + 3.0 * u * u * t * p1.1
                + 3.0 * u * t * t * p2.1
                + t * t * t * p3.1;
            points.push((x, y));
        }
    }
    points
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

/// Rasterize the mark into its three coverage bitmaps, anti-aliased.
///
/// Coverage falls off over one device pixel around each edge, which is what
/// the font rasterizer would have done with the same shapes.
pub fn rasterize(width: usize, height: usize) -> Mark {
    let hook_line = hook_points();
    let mut hook = vec![0u8; width * height];
    let mut dot = vec![0u8; width * height];
    let mut arrow = vec![0u8; width * height];

    // Pixels per icon unit; the box arrives in the mark's own ASPECT, so one
    // scale serves both axes.
    let scale = (height as f32 / CONTENT_HEIGHT).max(f32::EPSILON);
    for y in 0..height {
        for x in 0..width {
            let u = CONTENT_LEFT + (x as f32 + 0.5) / scale;
            let v = CONTENT_TOP + (y as f32 + 0.5) / scale;
            let p = (u, v);
            let at = y * width + x;

            let hook_alpha = (HOOK_HALF - polyline_distance(p, &hook_line)) * scale + 0.5;
            hook[at] = (hook_alpha.clamp(0.0, 1.0) * 255.0) as u8;

            let from_center = ((u - DOT_CENTER.0).powi(2) + (v - DOT_CENTER.1).powi(2)).sqrt();
            let dot_alpha = (DOT_RADIUS - from_center) * scale + 0.5;
            dot[at] = (dot_alpha.clamp(0.0, 1.0) * 255.0) as u8;

            let arrow_alpha = (ARROW_HALF - polyline_distance(p, ARROW)) * scale + 0.5;
            arrow[at] = (arrow_alpha.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    Mark {
        hook: glyph(width, height, hook),
        dot: glyph(width, height, dot),
        arrow: glyph(width, height, arrow),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink(glyph: &RasterizedGlyph) -> usize {
        glyph.coverage.iter().filter(|&&c| c > 0).count()
    }

    /// Every part of the icon is drawn, and edges are soft rather than a
    /// staircase.
    #[test]
    fn the_mark_is_drawn_and_anti_aliased() {
        let mark = rasterize(33, 40);
        for (name, part) in [
            ("hook", &mark.hook),
            ("dot", &mark.dot),
            ("arrow", &mark.arrow),
        ] {
            assert!(ink(part) > 10, "{name} has no ink");
            let partial = part.coverage.iter().filter(|&&c| c > 0 && c < 255).count();
            assert!(partial > 10, "{name}'s edges are not anti-aliased");
        }
    }

    /// The parts sit where the icon puts them: dot at the top-left, arrow in
    /// the lower-right half, exactly the icon's arrangement.
    #[test]
    fn the_parts_keep_the_icons_arrangement() {
        let mark = rasterize(33, 40);
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
        let dot = centroid(&mark.dot);
        assert!(
            dot.0 < 0.35 && dot.1 < 0.25,
            "the dot is not at the start: {dot:?}"
        );
        let arrow = centroid(&mark.arrow);
        assert!(
            arrow.0 > 0.6 && arrow.1 > 0.45,
            "the arrow is not at the open end: {arrow:?}"
        );
    }

    /// Empty at the far corners: ink that bleeds to the box's edge smears
    /// into the atlas's neighbouring glyph.
    #[test]
    fn the_mark_keeps_out_of_its_own_corners() {
        let mark = rasterize(36, 44);
        for part in [&mark.hook, &mark.dot, &mark.arrow] {
            for (x, y) in [(0, 43), (35, 0)] {
                assert_eq!(part.coverage[y * 36 + x], 0, "ink in the corner {x},{y}");
            }
        }
    }

    /// Size follows the box it is asked for.
    #[test]
    fn the_mark_scales_with_its_box() {
        let small = rasterize(16, 20);
        let large = rasterize(33, 40);
        assert!(ink(&large.hook) > ink(&small.hook) * 2);
    }
}
