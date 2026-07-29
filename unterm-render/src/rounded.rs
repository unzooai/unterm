//! Panels with rounded corners, built from rows of rectangles.
//!
//! Every overlay in the window -- the palette, the git panel, the notice, the
//! fleet card -- is a panel floating over the terminal. Square corners make
//! them read as another region of the grid; rounded ones make them read as
//! something on top of it, which is what they are.
//!
//! Built from one rectangle per pixel row in the corner bands, and a single
//! one for everything between. A curve from rectangles is not a curve, but at
//! a radius of a few pixels the steps are a pixel each -- the same trade the
//! powerline separators make, for the same reason: this renderer draws
//! rectangles, and adding a shader for a corner is a lot of machinery for
//! eight pixels.

use crate::quads::Quad;

/// How round a panel's corners are, in pixels at 1× scale.
///
/// Small. A panel is a surface with its edges taken off, not a lozenge: past
/// about a sixth of the panel's height the curve starts reading as the shape
/// of the thing rather than as its finish.
pub const RADIUS: f32 = 6.0;

/// A filled panel with rounded corners.
///
/// The radius is clamped to what the rectangle can carry, so a panel one row
/// high is a panel rather than a circle, and a radius of zero is an ordinary
/// rectangle rather than an empty list.
pub fn panel(left: f32, top: f32, width: f32, height: f32, radius: f32, color: [f32; 4]) -> Vec<Quad> {
    if width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }
    let radius = radius.min(width / 2.0).min(height / 2.0).max(0.0).floor();
    if radius < 1.0 {
        return vec![Quad {
            left,
            top,
            width,
            height,
            color,
        }];
    }

    let mut quads = Vec::new();
    // The middle: everything between the two corner bands, full width.
    quads.push(Quad {
        left,
        top: top + radius,
        width,
        height: height - radius * 2.0,
        color,
    });

    // The bands, a row at a time. `inset` is how far in the row starts, from
    // the circle the corner is a quarter of.
    for step in 0..radius as usize {
        let from_edge = radius - step as f32 - 0.5;
        let inset = radius - (radius * radius - from_edge * from_edge).max(0.0).sqrt();
        let inset = inset.min(radius);
        let row = width - inset * 2.0;
        if row <= 0.0 {
            continue;
        }
        quads.push(Quad {
            left: left + inset,
            top: top + step as f32,
            width: row,
            height: 1.0,
            color,
        });
        quads.push(Quad {
            left: left + inset,
            top: top + height - step as f32 - 1.0,
            width: row,
            height: 1.0,
            color,
        });
    }
    quads
}

/// A panel at the usual radius.
pub fn default_panel(left: f32, top: f32, width: f32, height: f32, color: [f32; 4]) -> Vec<Quad> {
    panel(left, top, width, height, RADIUS, color)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLOR: [f32; 4] = [0.2, 0.3, 0.4, 1.0];

    /// Whether a pixel's centre is covered by any of the rectangles.
    fn covered(quads: &[Quad], x: f32, y: f32) -> bool {
        quads.iter().any(|quad| {
            x >= quad.left
                && x < quad.left + quad.width
                && y >= quad.top
                && y < quad.top + quad.height
        })
    }

    #[test]
    fn a_panel_never_reaches_outside_its_rectangle() {
        let quads = panel(10.0, 20.0, 100.0, 40.0, 6.0, COLOR);
        for quad in &quads {
            assert!(quad.left >= 10.0, "{quad:?}");
            assert!(quad.top >= 20.0, "{quad:?}");
            assert!(quad.left + quad.width <= 110.0 + 0.001, "{quad:?}");
            assert!(quad.top + quad.height <= 60.0 + 0.001, "{quad:?}");
        }
    }

    /// The middle is filled solid. A panel with a gap in it is a panel you can
    /// see the terminal through.
    #[test]
    fn the_middle_is_filled() {
        let quads = panel(0.0, 0.0, 60.0, 30.0, 6.0, COLOR);
        for y in 0..30 {
            for x in 8..52 {
                assert!(
                    covered(&quads, x as f32 + 0.5, y as f32 + 0.5),
                    "a hole at {x},{y}"
                );
            }
        }
    }

    /// And the corners are not. This is the whole point: a square corner reads
    /// as another region of the grid rather than as something on top of it.
    #[test]
    fn the_corners_are_taken_off() {
        let quads = panel(0.0, 0.0, 60.0, 30.0, 6.0, COLOR);
        for (x, y) in [(0.5, 0.5), (59.5, 0.5), (0.5, 29.5), (59.5, 29.5)] {
            assert!(!covered(&quads, x, y), "the corner at {x},{y} is square");
        }
    }

    /// Each edge is reached in the middle of its run, or the panel looks
    /// inset rather than rounded.
    #[test]
    fn every_edge_is_reached_between_its_corners() {
        let quads = panel(0.0, 0.0, 60.0, 30.0, 6.0, COLOR);
        assert!(covered(&quads, 30.0, 0.5), "the top edge is missing");
        assert!(covered(&quads, 30.0, 29.5), "the bottom edge is missing");
        assert!(covered(&quads, 0.5, 15.0), "the left edge is missing");
        assert!(covered(&quads, 59.5, 15.0), "the right edge is missing");
    }

    /// The same shape whichever corner it is, or one corner looks wrong and
    /// nobody can say which.
    #[test]
    fn the_four_corners_match() {
        let quads = panel(0.0, 0.0, 60.0, 30.0, 6.0, COLOR);
        for offset in 0..6 {
            let near = offset as f32 + 0.5;
            let far_x = 60.0 - near;
            let far_y = 30.0 - near;
            let corners = [
                covered(&quads, near, near),
                covered(&quads, far_x, near),
                covered(&quads, near, far_y),
                covered(&quads, far_x, far_y),
            ];
            assert!(
                corners.iter().all(|hit| *hit == corners[0]),
                "the corners differ {offset} in: {corners:?}"
            );
        }
    }

    /// A panel too small for its radius is still a panel. Clamping matters:
    /// a notice one row high asked for a radius bigger than itself.
    #[test]
    fn a_small_panel_keeps_its_shape() {
        for (width, height) in [(4.0, 4.0), (20.0, 3.0), (3.0, 20.0), (1.0, 1.0)] {
            let quads = panel(0.0, 0.0, width, height, 6.0, COLOR);
            assert!(!quads.is_empty(), "{width}x{height} drew nothing");
            for quad in &quads {
                assert!(quad.left >= 0.0 && quad.top >= 0.0, "{quad:?}");
                assert!(quad.left + quad.width <= width + 0.001, "{quad:?}");
                assert!(quad.top + quad.height <= height + 0.001, "{quad:?}");
            }
            // The centre is still filled, whatever the size.
            assert!(covered(&quads, width / 2.0, height / 2.0));
        }
    }

    /// No radius is an ordinary rectangle, in one piece rather than in rows.
    #[test]
    fn no_radius_is_one_rectangle() {
        let quads = panel(5.0, 5.0, 50.0, 20.0, 0.0, COLOR);
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].width, 50.0);
        assert_eq!(quads[0].height, 20.0);
    }

    /// Nothing to draw draws nothing, rather than a rectangle of negative
    /// width that the renderer would turn inside out.
    #[test]
    fn an_empty_rectangle_draws_nothing() {
        assert!(panel(0.0, 0.0, 0.0, 10.0, 6.0, COLOR).is_empty());
        assert!(panel(0.0, 0.0, 10.0, 0.0, 6.0, COLOR).is_empty());
        assert!(panel(0.0, 0.0, -5.0, 10.0, 6.0, COLOR).is_empty());
    }

    /// Every piece carries the colour it was given: a panel that is two
    /// slightly different colours is a panel with a seam across it.
    #[test]
    fn every_piece_is_the_same_colour() {
        for quad in panel(0.0, 0.0, 40.0, 20.0, 6.0, COLOR) {
            assert_eq!(quad.color, COLOR);
        }
    }
}
