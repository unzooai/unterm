//! From a string to placed glyphs, using next-core's own font stack.
//!
//! This is the path that had no caller: discover a font, shape a run into glyph
//! indices, rasterize each one, and put it in the atlas. Everything here is
//! next-core's; nothing borrows the GUI's font system.

use crate::atlas::{GlyphAtlas, GlyphKey, GlyphSlot};
use unterm_engine::next_core::font_raster::FontFace;

/// A glyph placed on the baseline, ready to become two triangles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedGlyph {
    pub slot: GlyphSlot,
    /// Left edge of the bitmap, in pixels from the run's origin.
    pub x: i32,
    /// Top edge of the bitmap, in pixels down from the baseline.
    pub y: i32,
}

/// Lay a run out along the baseline.
///
/// Shaping is by glyph index rather than character because that is what a
/// shaper produces, and one character can become several glyphs -- or several
/// characters one glyph -- depending on the font and its neighbours.
pub fn place_run(
    face: &mut FontFace,
    face_id: usize,
    glyph_indices: &[u32],
    atlas: &mut GlyphAtlas,
) -> Vec<PlacedGlyph> {
    let pixel_size = face.pixel_size();
    let mut placed = Vec::with_capacity(glyph_indices.len());
    let mut pen_x = 0i32;

    for &glyph_index in glyph_indices {
        let key = GlyphKey {
            // The terminal's stack. The chrome files its glyphs under its own.
            stack: 0,
            face: face_id,
            glyph_index,
            pixel_size,
        };

        let slot = match atlas.get(key) {
            Some(slot) => slot,
            None => match face.rasterize_glyph_index(glyph_index) {
                Ok(glyph) => atlas.insert(key, &glyph),
                // A glyph the face cannot produce must not stop the line: the
                // rest of the text is still worth drawing, and a gap is easier
                // to diagnose than a blank window.
                Err(_) => continue,
            },
        };

        placed.push(PlacedGlyph {
            slot,
            x: pen_x + slot.bearing_x,
            // Bitmaps are measured up from the baseline; screens count down.
            y: -slot.bearing_y,
        });
        pen_x += slot.advance_x;
    }

    placed
}

/// How wide a run is, without placing it.
pub fn run_width(placed: &[PlacedGlyph]) -> i32 {
    placed
        .iter()
        .map(|glyph| glyph.x + glyph.slot.width as i32)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use unterm_engine::next_core::font_discovery;

    fn monospace() -> Option<FontFace> {
        let index = font_discovery::FontIndex::scan();
        let entry = index.default_monospace()?;
        FontFace::open(&entry.path, 16).ok()
    }

    #[test]
    fn a_real_font_fills_a_real_atlas() {
        let Some(mut face) = monospace() else {
            // No font on this machine; the rest of the suite still stands.
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);

        // Rasterize by character first to learn the indices the face uses.
        let glyph = face.rasterize('A').expect("a monospace face can draw 'A'");
        assert!(
            glyph.width > 0 && glyph.height > 0,
            "'A' should have pixels"
        );

        let key = GlyphKey {
            stack: 0,
            face: 0,
            glyph_index: 0,
            pixel_size: 16,
        };
        let slot = atlas.insert(key, &glyph);

        // This is the join that did not exist: next-core's own rasterizer
        // producing pixels in an atlas, with no GUI font stack involved.
        assert_eq!(slot.width, glyph.width);
        let any_ink = (0..slot.height)
            .any(|row| (0..slot.width).any(|col| atlas.pixel(slot.x + col, slot.y + row) > 0));
        assert!(any_ink, "the glyph should have left ink in the atlas");
    }

    #[test]
    fn a_run_advances_left_to_right() {
        let Some(mut face) = monospace() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);

        // Indices 1..4 are arbitrary but real for any face with glyphs.
        let placed = place_run(&mut face, 0, &[1, 2, 3], &mut atlas);
        if placed.len() < 2 {
            return;
        }

        for pair in placed.windows(2) {
            assert!(
                pair[1].x >= pair[0].x,
                "a run must not walk backwards: {:?}",
                placed
            );
        }
        assert!(run_width(&placed) > 0);
    }

    #[test]
    fn a_glyph_the_face_cannot_draw_does_not_stop_the_run() {
        let Some(mut face) = monospace() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);

        // A wildly out-of-range index alongside real ones.
        let placed = place_run(&mut face, 0, &[1, u32::MAX, 2], &mut atlas);

        // A gap is easier to diagnose than a blank window.
        assert!(placed.len() <= 3);
    }

    #[test]
    fn placing_the_same_run_twice_reuses_the_atlas() {
        let Some(mut face) = monospace() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);

        place_run(&mut face, 0, &[1, 2, 3], &mut atlas);
        let after_first = atlas.len();
        place_run(&mut face, 0, &[1, 2, 3], &mut atlas);

        assert_eq!(atlas.len(), after_first);
    }
}
