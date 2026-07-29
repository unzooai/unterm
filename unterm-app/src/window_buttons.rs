//! Minimise, maximise and close, drawn into the top bar.
//!
//! Ported from the previous front end's `window_buttons.rs`. The paths are
//! its paths, in the same normalised coordinates: a close that is two
//! corner-to-corner diagonals, a minimise that is a rule at six tenths of the
//! height, a maximise whose corners are chamfered a tenth, and a restore that
//! is one square behind another. They were drawn as vector outlines there and
//! are stroked here, which at a title bar's size is the same picture.
//!
//! The colours are its colours too, and the one that matters is the close
//! button's hover: Windows turns it red, and a window whose close button does
//! not is a window people hesitate over.

use unterm_render::quads::Quad;
use unterm_render::strokes;

/// Which button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Minimise,
    Maximise,
    Restore,
    Close,
}

/// How big the icon is drawn inside its button, as a fraction of the button's
/// height. The previous front end drew a 10pt glyph in a title bar; this keeps
/// the same proportion whatever the cell size.
const ICON: f32 = 0.42;

/// Windows' own close-button red. Not derived from the theme: it means "this
/// closes the window" and it means it in every theme.
pub const CLOSE_HOVER: [f32; 4] = [0.88, 0.05, 0.04, 1.0];

/// The icon's colour on a bar of the given lightness.
///
/// Black on a light bar, white on a dark one. Following the theme's own
/// foreground instead would leave a low-contrast icon on the one surface that
/// must never be hard to find.
pub fn icon_color(bar_is_light: bool) -> [f32; 4] {
    if bar_is_light {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}

/// The fill behind a hovered button.
pub fn hover_fill(button: Button, bar_is_light: bool) -> [f32; 4] {
    if button == Button::Close {
        return CLOSE_HOVER;
    }
    let icon = icon_color(bar_is_light);
    [icon[0], icon[1], icon[2], if bar_is_light { 0.14 } else { 0.11 }]
}

/// The icon's colour when the button is hovered.
///
/// White on the close button's red, whatever the bar is: the red is dark in
/// both themes and a black cross on it is barely there.
pub fn hovered_icon_color(button: Button, bar_is_light: bool) -> [f32; 4] {
    if button == Button::Close {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        icon_color(bar_is_light)
    }
}

/// Draw `button` centred in the box at `left`, `top`.
pub fn quads(
    button: Button,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
) -> Vec<Quad> {
    let size = (height * ICON).max(4.0).floor();
    let origin = (left + (width - size) / 2.0, top + (height - size) / 2.0);
    let at = |x: f32, y: f32| (origin.0 + size * x, origin.1 + size * y);
    let weight = (size / 10.0).round().max(1.0);

    match button {
        // Corner to corner, both ways.
        Button::Close => {
            let mut quads = strokes::line(at(0.0, 0.0), at(1.0, 1.0), weight, color);
            quads.extend(strokes::line(at(1.0, 0.0), at(0.0, 1.0), weight, color));
            quads
        }
        // A rule low in the box rather than through its middle, so it reads as
        // "down to the taskbar" rather than as a minus sign.
        Button::Minimise => strokes::line(at(0.0, 0.6), at(1.0, 0.6), weight, color),
        // Corners chamfered a tenth, as before -- but squared up: the path
        // this came from ran 0.1 to 1.0 across and 0.1 to 1.0 down, which puts
        // the box a tenth right of centre. Visible once it is the only thing
        // in the corner of a window.
        Button::Maximise => strokes::polyline(
            &[
                at(0.2, 0.1),
                at(0.8, 0.1),
                at(0.9, 0.2),
                at(0.9, 0.8),
                at(0.8, 0.9),
                at(0.2, 0.9),
                at(0.1, 0.8),
                at(0.1, 0.2),
                at(0.2, 0.1),
            ],
            weight,
            color,
        ),
        // One square behind another: the front one is where the window will
        // go back to, the back one is where it is now.
        Button::Restore => {
            let mut quads = strokes::polyline(
                &[at(0.25, 0.1), at(0.9, 0.1), at(0.9, 0.75)],
                weight,
                color,
            );
            quads.extend(strokes::rectangle(
                origin.0 + size * 0.05,
                origin.1 + size * 0.3,
                size * 0.65,
                size * 0.65,
                weight,
                color,
            ));
            quads
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(button: Button) -> Vec<Quad> {
        quads(button, 0.0, 0.0, 30.0, 30.0, [1.0; 4])
    }

    fn bounds(quads: &[Quad]) -> (f32, f32, f32, f32) {
        let left = quads.iter().map(|q| q.left).fold(f32::MAX, f32::min);
        let top = quads.iter().map(|q| q.top).fold(f32::MAX, f32::min);
        let right = quads
            .iter()
            .map(|q| q.left + q.width)
            .fold(f32::MIN, f32::max);
        let bottom = quads
            .iter()
            .map(|q| q.top + q.height)
            .fold(f32::MIN, f32::max);
        (left, top, right, bottom)
    }

    #[test]
    fn every_button_draws_something() {
        for button in [
            Button::Minimise,
            Button::Maximise,
            Button::Restore,
            Button::Close,
        ] {
            assert!(!drawn(button).is_empty(), "{button:?} drew nothing");
        }
    }

    /// Centred across its box. An icon that hugs one edge is the first thing
    /// that looks wrong about a hand-drawn title bar.
    #[test]
    fn an_icon_is_centred_across_its_button() {
        for button in [
            Button::Minimise,
            Button::Maximise,
            Button::Restore,
            Button::Close,
        ] {
            // A stroke adds its own weight past the end of the path, so the
            // drawn bounds are never exactly symmetric.
            let (left, _, right, _) = bounds(&drawn(button));
            let off = (left - (30.0 - right)).abs();
            assert!(off <= 1.5, "{button:?} sits off-centre by {off}");
        }
    }

    /// Vertically too, for the symmetric ones. Minimise is deliberately low --
    /// see below -- so it is not one of them.
    #[test]
    fn a_symmetric_icon_is_centred_down_its_button() {
        for button in [Button::Maximise, Button::Close] {
            let (_, top, _, bottom) = bounds(&drawn(button));
            let off = (top - (30.0 - bottom)).abs();
            assert!(off <= 1.5, "{button:?} sits off-centre by {off}");
        }
    }

    /// And stays inside it, or it draws over the button beside it.
    #[test]
    fn an_icon_stays_inside_its_button() {
        for button in [
            Button::Minimise,
            Button::Maximise,
            Button::Restore,
            Button::Close,
        ] {
            for quad in drawn(button) {
                assert!(quad.left >= 0.0, "{button:?}: {quad:?}");
                assert!(quad.top >= 0.0, "{button:?}: {quad:?}");
                assert!(quad.left + quad.width <= 30.0, "{button:?}: {quad:?}");
                assert!(quad.top + quad.height <= 30.0, "{button:?}: {quad:?}");
            }
        }
    }

    /// The minimise rule sits low, not through the middle: high enough and it
    /// reads as a minus sign.
    #[test]
    fn the_minimise_rule_sits_below_the_middle() {
        let (_, top, _, bottom) = bounds(&drawn(Button::Minimise));
        let middle = (top + bottom) / 2.0;
        assert!(middle > 15.0, "the rule is at {middle}, not below centre");
    }

    /// Windows turns the close button red, and a window whose close button
    /// does not is a window people hesitate over.
    #[test]
    fn only_the_close_button_turns_red() {
        assert_eq!(hover_fill(Button::Close, false), CLOSE_HOVER);
        for button in [Button::Minimise, Button::Maximise, Button::Restore] {
            let fill = hover_fill(button, false);
            assert_ne!(fill, CLOSE_HOVER, "{button:?} should not go red");
            assert!(fill[3] < 0.2, "{button:?} should be a tint: {fill:?}");
        }
    }

    /// And its cross goes white, because the red is dark in both themes.
    #[test]
    fn the_close_cross_is_white_on_its_red() {
        for light in [true, false] {
            assert_eq!(hovered_icon_color(Button::Close, light), [1.0, 1.0, 1.0, 1.0]);
        }
    }

    #[test]
    fn the_icon_follows_the_bars_lightness_rather_than_the_theme() {
        assert_eq!(icon_color(true), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(icon_color(false), [1.0, 1.0, 1.0, 1.0]);
    }

    /// A tiny bar still gets a visible icon rather than nothing.
    #[test]
    fn a_small_button_still_draws() {
        let quads = quads(Button::Close, 0.0, 0.0, 8.0, 8.0, [1.0; 4]);
        assert!(!quads.is_empty());
        for quad in quads {
            assert!(quad.width >= 1.0 && quad.height >= 1.0);
        }
    }
}
