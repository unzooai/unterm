//! The face the window's chrome is drawn in.
//!
//! Not the terminal's. The previous front end drew every tab, sidebar row and
//! status segment in a UI face at 13pt with a looser line height, sized in
//! points against the display rather than in terminal cells -- and that is most
//! of why this window read as a wall of output surrounded by more of the same.
//!
//! Two things follow from it being separate. It does not grow when somebody
//! zooms the terminal, because zooming the terminal means zooming the terminal.
//! And it files its glyphs in the atlas under its own stack, because a face
//! index means nothing without saying whose stack it indexes.

use crate::terminal::{Shape, TerminalFont};

/// Which stack the chrome's glyphs are filed under.
pub const STACK: u8 = 1;

/// The families to try for chrome text, in order.
///
/// A proportional UI face first, because chrome is prose rather than a grid:
/// the old front end used the platform's title font. A monospace face is the
/// fallback rather than the choice -- but it is a fallback that always exists,
/// so the chrome never fails to open.
const FAMILIES: &[&str] = &[
    // Windows
    "Segoe UI Variable Text",
    "Segoe UI",
    // macOS
    "SF Pro Text",
    "Helvetica Neue",
    // Linux
    "Inter",
    "Cantarell",
    "DejaVu Sans",
    "Noto Sans",
];

/// Open the chrome face at the display's scale.
pub fn open(fallbacks: &[String], scale: f32) -> anyhow::Result<TerminalFont> {
    let pixels = crate::terminal::pixels_for_points(crate::ui_tokens::UI_FONT_SIZE as f32, scale);
    let index = unterm_engine::next_core::font_discovery::FontIndex::scan();
    let family = FAMILIES
        .iter()
        .find(|family| !index.family(family).is_empty())
        .copied();

    // The chrome's own line height, which is looser than a terminal's: rows
    // that breathe are the difference between a sidebar and a list of output.
    let shape = Shape {
        line_height: crate::ui_tokens::UI_LINE_HEIGHT as f32,
        cell_width: 1.0,
    };
    Ok(TerminalFont::open_named(family, pixels, fallbacks, shape)?.as_stack(STACK))
}

/// How many pixels one point is at this scale, for the chrome's own geometry.
///
/// Every token in `ui_tokens` is in points; this is what turns one into a
/// distance on this display.
pub fn point(scale: f32) -> f32 {
    (scale.max(0.1) * 96.0 / 72.0).max(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chrome opens on whatever this machine has. A window that will not
    /// draw its own tabs is not a window.
    #[test]
    fn the_chrome_face_opens() {
        let font = open(&[], 1.0).expect("the chrome has to have a face");
        assert!(font.metrics().height > 0.0);
        assert_eq!(font.stack_id(), STACK);
    }

    /// It is filed under its own stack, or its glyphs and the terminal's
    /// overwrite each other in the atlas whenever the two land on one size.
    #[test]
    fn the_chrome_face_is_a_different_stack_from_the_terminal() {
        let chrome = open(&[], 1.0).expect("a chrome face");
        assert_ne!(chrome.stack_id(), 0, "the chrome shares the terminal's stack");
    }

    /// Its rows are looser than a terminal's. Chrome at the terminal's rhythm
    /// reads as output rather than as chrome.
    #[test]
    fn chrome_rows_are_looser_than_terminal_rows() {
        let chrome = open(&[], 1.0).expect("a chrome face");
        let ratio = chrome.metrics().height / chrome.metrics().baseline.max(1.0);
        assert!(ratio > 1.0, "the chrome's rows are not loose at all: {ratio}");
    }

    /// A point is a point at 1x and grows with the display, because the tokens
    /// are in points and a token that meant device pixels would be a third
    /// smaller on this machine than it was written to be.
    #[test]
    fn a_point_grows_with_the_display() {
        let one = point(1.0);
        let one_and_a_half = point(1.5);
        assert!(one > 1.0, "a point is at least a pixel: {one}");
        assert!(
            (one_and_a_half / one - 1.5).abs() < 0.01,
            "{one} then {one_and_a_half}"
        );
    }

    #[test]
    fn a_nonsense_scale_still_gives_a_usable_point() {
        for scale in [0.0, -1.0, f32::NAN] {
            assert!(point(scale) >= 0.5, "scale {scale} gave {}", point(scale));
        }
    }
}
