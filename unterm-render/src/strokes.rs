//! Thin lines, from the only primitive this renderer has.
//!
//! The window buttons and the chrome's edges are outlines -- an X, a rule, a
//! square -- and the previous front end drew them as vector paths. There is no
//! path renderer here and there does not need to be: at a title bar's size a
//! diagonal is a dozen pixels, and a staircase of single-pixel rectangles is
//! indistinguishable from a stroked line while costing nothing new.
//!
//! The same trick already draws the powerline separators. What is here is the
//! general form of it, because a `×` that is two diagonals is exactly what the
//! close button was.

use crate::quads::Quad;

/// A straight line from `from` to `to`, `weight` pixels thick.
///
/// Axis-aligned lines come out as one rectangle. Diagonals are stepped along
/// their longer axis, which is what keeps a stroke even: stepping the shorter
/// one leaves gaps wherever the line advances faster than the steps do.
pub fn line(from: (f32, f32), to: (f32, f32), weight: f32, color: [f32; 4]) -> Vec<Quad> {
    let weight = weight.max(1.0);
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);

    if dy.abs() < 0.5 {
        let left = from.0.min(to.0);
        return vec![Quad {
            left,
            top: from.1.min(to.1),
            width: dx.abs().max(weight),
            height: weight,
            color,
        }];
    }
    if dx.abs() < 0.5 {
        return vec![Quad {
            left: from.0.min(to.0),
            top: from.1.min(to.1),
            width: weight,
            height: dy.abs().max(weight),
            color,
        }];
    }

    let steps = dx.abs().max(dy.abs()).ceil().max(1.0);
    (0..=steps as usize)
        .map(|step| {
            let along = step as f32 / steps;
            Quad {
                left: from.0 + dx * along,
                top: from.1 + dy * along,
                width: weight,
                height: weight,
                color,
            }
        })
        .collect()
}

/// A run of connected lines.
pub fn polyline(points: &[(f32, f32)], weight: f32, color: [f32; 4]) -> Vec<Quad> {
    points
        .windows(2)
        .flat_map(|pair| line(pair[0], pair[1], weight, color))
        .collect()
}

/// The four sides of a rectangle, as an outline.
pub fn rectangle(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    weight: f32,
    color: [f32; 4],
) -> Vec<Quad> {
    let (right, bottom) = (left + width, top + height);
    polyline(
        &[
            (left, top),
            (right, top),
            (right, bottom),
            (left, bottom),
            (left, top),
        ],
        weight,
        color,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    fn covers(quads: &[Quad], x: f32, y: f32) -> bool {
        quads.iter().any(|quad| {
            x >= quad.left - 0.5
                && x <= quad.left + quad.width + 0.5
                && y >= quad.top - 0.5
                && y <= quad.top + quad.height + 0.5
        })
    }

    /// A horizontal rule is one rectangle, not a hundred: the minimise button
    /// is exactly this, and drawing it as a staircase would cost a quad per
    /// pixel for a straight line.
    #[test]
    fn a_horizontal_line_is_a_single_quad() {
        let quads = line((0.0, 5.0), (20.0, 5.0), 1.0, WHITE);
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].width, 20.0);
        assert_eq!(quads[0].height, 1.0);
    }

    #[test]
    fn a_vertical_line_is_a_single_quad() {
        let quads = line((5.0, 0.0), (5.0, 20.0), 1.0, WHITE);
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].height, 20.0);
    }

    /// Drawn either way round. A path that only works left-to-right fails on
    /// the second stroke of an X.
    #[test]
    fn a_line_is_the_same_drawn_backwards() {
        let forward = line((0.0, 0.0), (10.0, 0.0), 1.0, WHITE);
        let backward = line((10.0, 0.0), (0.0, 0.0), 1.0, WHITE);
        assert_eq!(forward, backward);
    }

    /// A diagonal has to be continuous: a gap in a close button reads as a
    /// rendering fault, which is what it would be.
    #[test]
    fn a_diagonal_has_no_gaps_along_its_length() {
        let quads = line((0.0, 0.0), (10.0, 10.0), 1.0, WHITE);
        for step in 0..=10 {
            let at = step as f32;
            assert!(covers(&quads, at, at), "nothing at ({at}, {at})");
        }
    }

    /// Including a shallow one, where the line advances faster across than
    /// down -- stepping the shorter axis is what leaves those gaps.
    #[test]
    fn a_shallow_diagonal_has_no_gaps_either() {
        let quads = line((0.0, 0.0), (20.0, 4.0), 1.0, WHITE);
        for step in 0..=20 {
            let x = step as f32;
            let y = x * 0.2;
            assert!(covers(&quads, x, y), "nothing at ({x}, {y})");
        }
    }

    /// And it reaches both ends. A stroke that stops one step short leaves a
    /// close button with a visibly clipped corner.
    #[test]
    fn a_diagonal_reaches_both_of_its_ends() {
        let quads = line((2.0, 3.0), (12.0, 13.0), 1.0, WHITE);
        assert!(covers(&quads, 2.0, 3.0), "start");
        assert!(covers(&quads, 12.0, 13.0), "end");
    }

    #[test]
    fn a_rectangle_outlines_four_sides_and_no_middle() {
        let quads = rectangle(0.0, 0.0, 10.0, 10.0, 1.0, WHITE);
        assert!(covers(&quads, 5.0, 0.0), "top");
        assert!(covers(&quads, 5.0, 10.0), "bottom");
        assert!(covers(&quads, 0.0, 5.0), "left");
        assert!(covers(&quads, 10.0, 5.0), "right");
        assert!(!covers(&quads, 5.0, 5.0), "the middle should be empty");
    }

    /// Nothing may be thinner than a pixel: a weight that rounds to zero is a
    /// button that draws nothing at all.
    #[test]
    fn a_stroke_is_never_thinner_than_a_pixel() {
        for quad in line((0.0, 0.0), (10.0, 10.0), 0.0, WHITE) {
            assert!(quad.width >= 1.0 && quad.height >= 1.0, "{quad:?}");
        }
        let flat = line((0.0, 0.0), (10.0, 0.0), 0.2, WHITE);
        assert!(flat[0].height >= 1.0);
    }

    /// A zero-length line is a dot, not an empty list: a path that folds back
    /// on itself should still leave a mark where it was.
    #[test]
    fn a_line_going_nowhere_still_draws_a_point() {
        let quads = line((4.0, 4.0), (4.0, 4.0), 1.0, WHITE);
        assert_eq!(quads.len(), 1);
        assert!(covers(&quads, 4.0, 4.0));
    }

    #[test]
    fn a_polyline_joins_its_segments() {
        let quads = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], 1.0, WHITE);
        assert!(covers(&quads, 10.0, 0.0), "the corner is drawn");
        assert!(covers(&quads, 10.0, 10.0), "and the far end");
    }
}
