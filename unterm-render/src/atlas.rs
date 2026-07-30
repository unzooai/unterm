//! A glyph atlas built from next-core's own rasterizer.
//!
//! next-core can already find a font, shape a run and rasterize a glyph, but
//! nothing joined those to a renderer -- the pixels were drawn by the GUI's
//! font stack instead, and next-core's three font modules had no caller at all.
//! This is the join: rasterized coverage in, one texture and a slot per glyph
//! out, ready to upload.
//!
//! What an atlas has to get right, and why:
//!
//! - The same glyph must land in one slot however often it is asked for. A
//!   terminal draws the same handful of characters thousands of times a second;
//!   re-rasterizing each one is the difference between a smooth window and a
//!   hot laptop.
//! - A glyph that does not fit must grow the atlas rather than be dropped or
//!   wrapped over its neighbour. A missing or corrupted glyph is visible
//!   immediately and looks like a font bug.
//! - Zero-size glyphs are real. A space rasterizes to nothing and still
//!   advances, so it needs a slot with no pixels rather than an error.

use std::collections::HashMap;
use unterm_engine::next_core::font_raster::RasterizedGlyph;

/// Where a glyph sits in the atlas texture, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphSlot {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    /// Offset from the pen position to the bitmap's left edge.
    pub bearing_x: i32,
    /// Offset from the baseline up to the bitmap's top edge.
    pub bearing_y: i32,
    pub advance_x: i32,
}

/// What identifies a glyph: a face and an index within it.
///
/// Keyed by glyph index rather than character because shaping produces
/// indices, and one character can map to several depending on its neighbours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    /// Which font stack the face index belongs to.
    ///
    /// Two stacks are in play -- the terminal's and the chrome's -- and a face
    /// index means nothing without saying whose. Without this, face 0 of the
    /// chrome's UI font and face 0 of the terminal's monospace face are the
    /// same key whenever the two happen to be the same pixel size, and each
    /// overwrites the other's glyphs.
    pub stack: u8,
    pub face: usize,
    pub glyph_index: u32,
    pub pixel_size: u32,
}

/// Padding between glyphs.
///
/// Without it, linear sampling at a glyph's edge picks up its neighbour and
/// leaves a faint smear along one side -- the classic atlas bleed.
const PADDING: usize = 1;

pub struct GlyphAtlas {
    width: usize,
    height: usize,
    /// One byte of coverage per pixel; the renderer tints with the cell colour.
    coverage: Vec<u8>,
    slots: HashMap<GlyphKey, GlyphSlot>,
    /// Left edge of the next slot on the current shelf.
    pen_x: usize,
    /// Top edge of the current shelf.
    shelf_y: usize,
    /// Tallest glyph on the current shelf, which is how far the next one drops.
    shelf_height: usize,
}

impl GlyphAtlas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            coverage: vec![0; width * height],
            slots: HashMap::new(),
            pen_x: 0,
            shelf_y: 0,
            shelf_height: 0,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// The whole texture, one byte of coverage per pixel.
    pub fn coverage(&self) -> &[u8] {
        &self.coverage
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Where a glyph already is, if it is.
    pub fn get(&self, key: GlyphKey) -> Option<GlyphSlot> {
        self.slots.get(&key).copied()
    }

    /// Place a glyph, or return where it already is.
    ///
    /// Growing doubles the height rather than reallocating per glyph: a
    /// terminal's glyph set is small and stabilises within moments of opening.
    pub fn insert(&mut self, key: GlyphKey, glyph: &RasterizedGlyph) -> GlyphSlot {
        if let Some(slot) = self.slots.get(&key) {
            return *slot;
        }

        let width = glyph.width;
        let height = glyph.height;

        // A space rasterizes to nothing and still advances.
        if width == 0 || height == 0 {
            let slot = GlyphSlot {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                bearing_x: glyph.bearing_x,
                bearing_y: glyph.bearing_y,
                advance_x: glyph.advance_x,
            };
            self.slots.insert(key, slot);
            return slot;
        }

        if self.pen_x + width > self.width {
            // Next shelf down.
            self.pen_x = 0;
            self.shelf_y += self.shelf_height + PADDING;
            self.shelf_height = 0;
        }
        while self.shelf_y + height > self.height {
            self.grow();
        }

        let slot = GlyphSlot {
            x: self.pen_x,
            y: self.shelf_y,
            width,
            height,
            bearing_x: glyph.bearing_x,
            bearing_y: glyph.bearing_y,
            advance_x: glyph.advance_x,
        };

        for row in 0..height {
            let src = row * width;
            let dst = (self.shelf_y + row) * self.width + self.pen_x;
            self.coverage[dst..dst + width].copy_from_slice(&glyph.coverage[src..src + width]);
        }

        self.pen_x += width + PADDING;
        self.shelf_height = self.shelf_height.max(height);
        self.slots.insert(key, slot);
        slot
    }

    /// Double the height, keeping every existing slot where it is.
    ///
    /// Slots stay valid because rows are only appended; a caller holding a
    /// `GlyphSlot` does not have to re-look-up after a grow.
    fn grow(&mut self) {
        self.height *= 2;
        self.coverage.resize(self.width * self.height, 0);
    }

    /// Coverage of one pixel, for tests and for looking at a glyph in a
    /// debugger without a GPU.
    pub fn pixel(&self, x: usize, y: usize) -> u8 {
        self.coverage
            .get(y * self.width + x)
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(width: usize, height: usize, fill: u8) -> RasterizedGlyph {
        RasterizedGlyph {
            coverage: vec![fill; width * height],
            width,
            height,
            bearing_x: 1,
            bearing_y: 2,
            advance_x: 3,
        }
    }

    fn key(index: u32) -> GlyphKey {
        GlyphKey {
            stack: 0,
            face: 0,
            glyph_index: index,
            pixel_size: 16,
        }
    }

    #[test]
    fn a_glyph_lands_in_the_atlas_with_its_metrics() {
        let mut atlas = GlyphAtlas::new(64, 64);

        let slot = atlas.insert(key(1), &glyph(4, 5, 200));

        assert_eq!((slot.width, slot.height), (4, 5));
        assert_eq!((slot.bearing_x, slot.bearing_y, slot.advance_x), (1, 2, 3));
        assert_eq!(atlas.pixel(slot.x, slot.y), 200);
    }

    #[test]
    fn the_same_glyph_is_placed_once() {
        let mut atlas = GlyphAtlas::new(64, 64);

        let first = atlas.insert(key(1), &glyph(4, 5, 200));
        let second = atlas.insert(key(1), &glyph(4, 5, 200));

        // A terminal draws the same characters thousands of times a second;
        // re-rasterizing each one is a hot laptop.
        assert_eq!(first, second);
        assert_eq!(atlas.len(), 1);
    }

    #[test]
    fn glyphs_of_different_faces_or_sizes_are_different_glyphs() {
        let mut atlas = GlyphAtlas::new(64, 64);
        let base = key(1);

        atlas.insert(base, &glyph(4, 5, 200));
        atlas.insert(GlyphKey { face: 1, ..base }, &glyph(4, 5, 200));
        atlas.insert(
            GlyphKey {
                pixel_size: 32,
                ..base
            },
            &glyph(4, 5, 200),
        );

        assert_eq!(atlas.len(), 3);
    }

    #[test]
    fn a_row_that_is_full_starts_a_new_shelf() {
        let mut atlas = GlyphAtlas::new(16, 64);

        let first = atlas.insert(key(1), &glyph(10, 4, 100));
        let second = atlas.insert(key(2), &glyph(10, 4, 100));

        // Wrapping over the neighbour would corrupt the first glyph.
        assert!(second.y > first.y, "{second:?} should be below {first:?}");
        assert_eq!(second.x, 0);
    }

    #[test]
    fn an_atlas_that_runs_out_of_height_grows_instead_of_dropping_a_glyph() {
        let mut atlas = GlyphAtlas::new(16, 8);

        atlas.insert(key(1), &glyph(10, 6, 100));
        let tall = atlas.insert(key(2), &glyph(10, 6, 100));

        // A missing glyph is visible immediately and reads as a font bug.
        assert!(atlas.height() > 8);
        assert_eq!(atlas.pixel(tall.x, tall.y), 100);
    }

    #[test]
    fn growing_keeps_earlier_glyphs_where_they_were() {
        let mut atlas = GlyphAtlas::new(16, 8);
        let first = atlas.insert(key(1), &glyph(10, 6, 111));

        atlas.insert(key(2), &glyph(10, 6, 222));

        // Rows are only appended, so a caller holding a slot need not re-look
        // it up after a grow.
        assert_eq!(atlas.get(key(1)), Some(first));
        assert_eq!(atlas.pixel(first.x, first.y), 111);
    }

    #[test]
    fn a_space_gets_a_slot_with_no_pixels() {
        let mut atlas = GlyphAtlas::new(64, 64);

        let slot = atlas.insert(key(1), &glyph(0, 0, 0));

        // It rasterizes to nothing and still advances.
        assert_eq!((slot.width, slot.height), (0, 0));
        assert_eq!(slot.advance_x, 3);
        assert_eq!(atlas.len(), 1);
    }

    #[test]
    fn neighbours_do_not_touch() {
        let mut atlas = GlyphAtlas::new(64, 64);

        let first = atlas.insert(key(1), &glyph(4, 4, 255));
        let second = atlas.insert(key(2), &glyph(4, 4, 255));

        // Linear sampling at an edge would otherwise pick up the neighbour and
        // leave a faint smear down one side.
        assert!(second.x >= first.x + first.width + PADDING);
    }
}
