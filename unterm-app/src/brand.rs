//! The Command Loop mark, rasterized.
//!
//! v0.57.4 fed these two polylines through its glyph pipeline, which
//! anti-aliased them like any other glyph. Drawing them as bare quads instead
//! reproduced the coordinates and lost the rasterizer: every diagonal came out
//! as a staircase, which on a mark this small is most of what the eye sees.
//! So: back through a rasterizer, and into the atlas like any other glyph.

use unterm_engine::next_core::font_raster::RasterizedGlyph;

/// The mark's two strokes, in fractions of its box. The compact U whose open
/// edge resolves into a terminal prompt, exactly as 0.57.4 plotted it.
const LOOP_STROKE: &[(f32, f32)] = &[
    (1.0 / 5.0, 1.0 / 10.0),
    (1.0 / 5.0, 3.0 / 5.0),
    (1.0 / 4.0, 7.0 / 10.0),
    (2.0 / 5.0, 4.0 / 5.0),
    (11.0 / 20.0, 4.0 / 5.0),
    (7.0 / 10.0, 7.0 / 10.0),
    (3.0 / 4.0, 3.0 / 5.0),
];
const PROMPT_STROKE: &[(f32, f32)] = &[
    (27.0 / 40.0, 17.0 / 40.0),
    (4.0 / 5.0, 11.0 / 20.0),
    (27.0 / 40.0, 27.0 / 40.0),
];

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

fn stroke_distance(p: (f32, f32), stroke: &[(f32, f32)], w: f32, h: f32) -> f32 {
    stroke
        .windows(2)
        .map(|pair| {
            segment_distance(
                p,
                (pair[0].0 * w, pair[0].1 * h),
                (pair[1].0 * w, pair[1].1 * h),
            )
        })
        .fold(f32::MAX, f32::min)
}

/// Rasterize the mark into coverage, anti-aliased.
///
/// Coverage falls off over one pixel around the stroke's edge, which is what
/// the font rasterizer would have done with the same shape.
pub fn rasterize(width: usize, height: usize, stroke_width: f32) -> RasterizedGlyph {
    let mut coverage = vec![0u8; width * height];
    let half = stroke_width.max(1.0) / 2.0;
    for y in 0..height {
        for x in 0..width {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let distance = stroke_distance(p, LOOP_STROKE, width as f32, height as f32).min(
                stroke_distance(p, PROMPT_STROKE, width as f32, height as f32),
            );
            let alpha = (half - distance + 0.5).clamp(0.0, 1.0);
            coverage[y * width + x] = (alpha * 255.0) as u8;
        }
    }
    RasterizedGlyph {
        coverage,
        width,
        height,
        bearing_x: 0,
        bearing_y: height as i32,
        advance_x: width as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mark has ink, and its edges are soft rather than a staircase:
    /// somewhere in the bitmap there are partial coverage values, which is
    /// the entire difference between this and the quads it replaces.
    #[test]
    fn the_mark_is_drawn_and_anti_aliased() {
        let glyph = rasterize(36, 40, 3.0);
        let full = glyph.coverage.iter().filter(|&&c| c == 255).count();
        let partial = glyph
            .coverage
            .iter()
            .filter(|&&c| c > 0 && c < 255)
            .count();
        assert!(full > 20, "the stroke has body: {full}");
        assert!(partial > 40, "the edges are anti-aliased: {partial}");
    }

    /// Empty at the corners: the strokes live inside the box, and a mark that
    /// bleeds to its own edge smears into the atlas's neighbouring glyph.
    #[test]
    fn the_mark_keeps_out_of_its_own_corners() {
        let glyph = rasterize(40, 40, 3.0);
        for (x, y) in [(0, 0), (39, 0), (0, 39), (39, 39)] {
            assert_eq!(glyph.coverage[y * 40 + x], 0, "ink in the corner {x},{y}");
        }
    }

    /// Size follows the box it is asked for.
    #[test]
    fn the_mark_scales_with_its_box() {
        let small = rasterize(20, 22, 2.0);
        let large = rasterize(40, 44, 4.0);
        let ink = |glyph: &RasterizedGlyph| glyph.coverage.iter().filter(|&&c| c > 0).count();
        assert!(ink(&large) > ink(&small) * 2);
    }
}
