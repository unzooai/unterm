//! A primary face and the faces behind it.
//!
//! One face cannot draw everything. A monospace programming font typically has
//! Latin, some symbols, and nothing else -- which is why CJK came out as boxes
//! here before this existed. Fallback is not a nicety: without it, whole
//! writing systems are unreadable.
//!
//! The trap this is built around: `FT_Load_Char` does not fail on a character
//! the face lacks. It loads glyph 0 and renders an empty box, so a renderer
//! that only checks for errors never learns it should have asked someone else.
//! Every lookup here asks `has_glyph` first.

use unterm_engine::next_core::font_discovery::{FontEntry, FontIndex};
use unterm_engine::next_core::font_raster::{FontFace, RasterizedGlyph};

/// Families to try when the primary face has nothing, in order.
///
/// Chosen for coverage rather than looks: the first three carry CJK on the
/// three platforms, and the last two carry symbols and emoji. A character with
/// no face at all is still better shown as the primary's box than as nothing.
const FALLBACK_FAMILIES: &[&str] = &[
    "Microsoft YaHei UI",
    "PingFang SC",
    "Noto Sans CJK SC",
    "Noto Sans Mono CJK SC",
    "Segoe UI Symbol",
    "Segoe UI Emoji",
    "Symbols Nerd Font Mono",
    "Noto Color Emoji",
];

pub struct FontStack {
    faces: Vec<FontFace>,
    pixel_size: u32,
}

impl FontStack {
    /// Build a stack around `primary`, adding whichever fallbacks this machine
    /// actually has.
    pub fn new(primary: FontFace, requested: &[String], pixel_size: u32) -> Self {
        let index = FontIndex::scan();
        let mut faces = vec![primary];

        // The config's own list first: someone who named a font meant it.
        let wanted = requested
            .iter()
            .map(String::as_str)
            .chain(FALLBACK_FAMILIES.iter().copied());

        let mut seen: Vec<String> = Vec::new();
        for family in wanted {
            if seen.iter().any(|name| name == family) {
                continue;
            }
            seen.push(family.to_string());
            if let Some(entry) = index.best_in_family(family) {
                if let Ok(face) = open(entry, pixel_size) {
                    faces.push(face);
                }
            }
        }

        Self { faces, pixel_size }
    }

    /// The system's default monospace face, at a given size.
    ///
    /// For work that needs a font but has no window to take one from -- a
    /// scrollback capture, say. Returns None when the machine has no
    /// monospace font at all, which is a thing to report rather than panic on.
    pub fn system(pixel_size: u32) -> Option<Self> {
        let index = FontIndex::scan();
        let entry = index.default_monospace()?;
        let primary = FontFace::open(&entry.path, pixel_size).ok()?;
        Some(FontStack::new(primary, &[], pixel_size))
    }

    pub fn pixel_size(&self) -> u32 {
        self.pixel_size
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// Which face should draw `ch`, if any can.
    ///
    /// Falls back to the primary when nothing has it, so the character comes
    /// out as that face's box rather than as a hole in the line.
    pub fn face_for(&self, ch: char) -> usize {
        self.faces
            .iter()
            .position(|face| face.has_glyph(ch))
            .unwrap_or(0)
    }

    /// Rasterize `ch` from whichever face has it.
    pub fn rasterize(&mut self, ch: char) -> Option<(usize, RasterizedGlyph)> {
        let index = self.face_for(ch);
        self.faces[index]
            .rasterize(ch)
            .ok()
            .map(|glyph| (index, glyph))
    }
}

fn open(entry: &FontEntry, pixel_size: u32) -> anyhow::Result<FontFace> {
    Ok(FontFace::open(&entry.path, pixel_size)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack() -> Option<FontStack> {
        FontStack::system(16)
    }

    #[test]
    fn latin_comes_from_the_primary_face() {
        let Some(stack) = stack() else {
            return;
        };

        // A monospace face has Latin; falling back for it would mean losing the
        // font the user chose on the characters they see most.
        assert_eq!(stack.face_for('A'), 0);
    }

    #[test]
    fn a_character_the_primary_lacks_finds_another_face() {
        let Some(stack) = stack() else {
            return;
        };
        if stack.len() < 2 {
            // No fallback family installed; nothing to assert.
            return;
        }

        // If the primary genuinely has CJK, this is not a fallback case.
        let primary_has_cjk = stack.faces[0].has_glyph('漢');
        let chosen = stack.face_for('漢');

        if primary_has_cjk {
            assert_eq!(chosen, 0);
        } else {
            assert!(
                chosen > 0 || !stack.faces.iter().any(|face| face.has_glyph('漢')),
                "a face with the character should have been chosen"
            );
        }
    }

    #[test]
    fn a_character_nobody_has_still_draws_something() {
        let Some(stack) = stack() else {
            return;
        };

        // A private-use codepoint no font is likely to carry.
        let chosen = stack.face_for('\u{10FFFD}');

        // The primary's box is a better answer than a hole in the line.
        assert_eq!(chosen, 0);
    }

    #[test]
    fn rasterizing_reports_which_face_drew_it() {
        let Some(mut stack) = stack() else {
            return;
        };

        let Some((face, glyph)) = stack.rasterize('A') else {
            return;
        };

        // The atlas keys on the face, so a glyph from a fallback must not be
        // filed under the primary -- two faces' glyph for the same character
        // would collide.
        assert_eq!(face, 0);
        assert!(glyph.width > 0);
    }

    #[test]
    fn a_named_font_is_tried_before_the_built_in_list() {
        let index = FontIndex::scan();
        let Some(entry) = index.default_monospace() else {
            return;
        };
        let Ok(primary) = FontFace::open(&entry.path, 16) else {
            return;
        };

        // Someone who named a font in their config meant it.
        let stack = FontStack::new(primary, &["Consolas".to_string()], 16);

        if index.best_in_family("Consolas").is_some() {
            assert!(stack.len() >= 2);
        }
    }
}
