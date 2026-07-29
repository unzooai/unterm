//! From a styled screen to the quads a GPU draws.
//!
//! This is the layer where rendering bugs are both easiest to introduce and
//! hardest to read: everything compiles, the frame is submitted, and what
//! appears is subtly or completely wrong. It is deliberately pure -- styled
//! cells and atlas slots in, vertices out -- so it can be checked by assertion
//! rather than by looking at a window.
//!
//! One trap in particular is pinned by a test here: texture coordinates are
//! normalized against the atlas size, and the atlas grows. Coordinates
//! computed before a grow and reused after it point at the wrong pixels, which
//! shows up as glyphs turning into slices of their neighbours.

use crate::atlas::{GlyphAtlas, GlyphSlot};
use unterm_engine::{CellStyle, StyledCell, StyledColor};

/// A solid rectangle in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
}

/// A glyph, with where to sample it from the atlas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphQuad {
    pub quad: Quad,
    /// Texture coordinates in 0..1, normalized against the atlas *as it is
    /// now*. Recomputed per frame rather than cached, because the atlas grows.
    pub tex_left: f32,
    pub tex_top: f32,
    pub tex_right: f32,
    pub tex_bottom: f32,
}

/// How big a cell is and where its baseline sits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellMetrics {
    pub width: f32,
    pub height: f32,
    /// Distance from the top of the cell down to the baseline.
    pub baseline: f32,
}

/// The colours a frame falls back to when a cell does not name its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameColors {
    pub foreground: [f32; 4],
    pub background: [f32; 4],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameQuads {
    pub backgrounds: Vec<Quad>,
    pub glyphs: Vec<GlyphQuad>,
}

/// The foreground and background a cell is drawn in.
///
/// Public because a shaped row resolves the colour for a glyph that may cover
/// several cells, and has to reach the same answer this does for one.
pub fn resolve_style(style: Option<&CellStyle>, colors: FrameColors) -> ([f32; 4], [f32; 4]) {
    match style {
        Some(style) => resolve(style, colors),
        None => (colors.foreground, colors.background),
    }
}

/// Resolve a cell colour, honouring inverse.
///
/// Inverse swaps foreground and background rather than picking a third colour,
/// which is what every terminal does and what a selection highlight relies on.
fn resolve(style: &CellStyle, colors: FrameColors) -> ([f32; 4], [f32; 4]) {
    let foreground = style
        .fg
        .map(|color| to_rgba(color, colors.foreground))
        .unwrap_or(colors.foreground);
    let background = style
        .bg
        .map(|color| to_rgba(color, colors.background))
        .unwrap_or(colors.background);

    if style.inverse {
        (background, foreground)
    } else {
        (foreground, background)
    }
}

/// A palette index needs a palette to mean anything; without one, fall back to
/// the frame colour rather than inventing a shade.
fn to_rgba(color: StyledColor, fallback: [f32; 4]) -> [f32; 4] {
    match color {
        StyledColor::Rgb(red, green, blue) => [
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            1.0,
        ],
        StyledColor::Palette(_) => fallback,
    }
}

/// Build one row's quads.
///
/// `slot_for` is asked for a glyph's place in the atlas; returning None skips
/// the glyph and keeps the row's other cells, because a gap is easier to
/// diagnose than a blank line.
pub fn build_row(
    cells: &[StyledCell],
    left_origin: f32,
    top: f32,
    metrics: CellMetrics,
    colors: FrameColors,
    atlas: &GlyphAtlas,
    mut slot_for: impl FnMut(char) -> Option<GlyphSlot>,
    out: &mut FrameQuads,
    already_drawn: &std::collections::HashSet<usize>,
) {
    let mut column = 0usize;

    for cell in cells {
        // A wide character occupies two columns, and its background has to
        // cover both or the second shows through.
        let span = cell.width.max(1) as f32;
        let left = left_origin + column as f32 * metrics.width;
        let (foreground, background) = resolve(&cell.style, colors);

        if background != colors.background {
            // Only where it differs: filling every cell with the frame colour
            // is the whole screen drawn twice, every frame.
            out.backgrounds.push(Quad {
                left,
                top,
                width: metrics.width * span,
                height: metrics.height,
                color: background,
            });
        }

        // A column the shaper already drew: drawing it again would put a
        // second glyph underneath the ligature that replaced it.
        if !already_drawn.contains(&column)
            && !cell.style.hidden
            && cell.ch != ' '
            && cell.ch != '\0'
        {
            if let Some(slot) = slot_for(cell.ch) {
                if slot.width > 0 && slot.height > 0 {
                    out.glyphs.push(glyph_quad(
                        slot,
                        left,
                        top + metrics.baseline,
                        foreground,
                        atlas,
                    ));
                }
            }
        }

        // Underlines and the rest, over the background and under nothing --
        // a line the program asked for is information, and losing it silently
        // is the same as not parsing it.
        out.backgrounds.extend(crate::decorations::quads_for(
            &cell.style,
            left,
            top,
            metrics.width * span,
            metrics,
            foreground,
        ));

        column += cell.width.max(1);
    }
}

/// Place one glyph relative to a pen sitting on the baseline.
pub fn glyph_quad(
    slot: GlyphSlot,
    pen_x: f32,
    baseline_y: f32,
    color: [f32; 4],
    atlas: &GlyphAtlas,
) -> GlyphQuad {
    let atlas_width = atlas.width().max(1) as f32;
    let atlas_height = atlas.height().max(1) as f32;

    GlyphQuad {
        quad: Quad {
            left: pen_x + slot.bearing_x as f32,
            // Bearing is measured up from the baseline; the screen counts down.
            top: baseline_y - slot.bearing_y as f32,
            width: slot.width as f32,
            height: slot.height as f32,
            color,
        },
        tex_left: slot.x as f32 / atlas_width,
        tex_top: slot.y as f32 / atlas_height,
        tex_right: (slot.x + slot.width) as f32 / atlas_width,
        tex_bottom: (slot.y + slot.height) as f32 / atlas_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::GlyphKey;
    use unterm_engine::next_core::font_raster::RasterizedGlyph;

    fn metrics() -> CellMetrics {
        CellMetrics {
            width: 10.0,
            height: 20.0,
            baseline: 16.0,
        }
    }

    fn colors() -> FrameColors {
        FrameColors {
            foreground: [1.0, 1.0, 1.0, 1.0],
            background: [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn cell(ch: char, style: CellStyle) -> StyledCell {
        StyledCell {
            ch,
            style,
            width: 1,
        }
    }

    fn atlas_with_glyph() -> (GlyphAtlas, GlyphSlot) {
        let mut atlas = GlyphAtlas::new(64, 64);
        let slot = atlas.insert(
            GlyphKey {
                face: 0,
                glyph_index: 1,
                pixel_size: 16,
            },
            &RasterizedGlyph {
                coverage: vec![255; 6 * 8],
                width: 6,
                height: 8,
                bearing_x: 1,
                bearing_y: 7,
                advance_x: 10,
            },
        );
        (atlas, slot)
    }

    #[test]
    fn a_glyph_sits_on_the_baseline_by_its_bearings() {
        let (atlas, slot) = atlas_with_glyph();

        let quad = glyph_quad(slot, 100.0, 50.0, [1.0; 4], &atlas);

        assert_eq!(quad.quad.left, 101.0);
        // Seven pixels above the baseline, because that is what bearing means.
        assert_eq!(quad.quad.top, 43.0);
        assert_eq!((quad.quad.width, quad.quad.height), (6.0, 8.0));
    }

    #[test]
    fn texture_coordinates_follow_the_atlas_when_it_grows() {
        let (mut atlas, slot) = atlas_with_glyph();
        let before = glyph_quad(slot, 0.0, 0.0, [1.0; 4], &atlas);

        // Force a grow, which doubles the height and halves every normalized
        // vertical coordinate.
        while atlas.height() == 64 {
            atlas.insert(
                GlyphKey {
                    face: 0,
                    glyph_index: atlas.len() as u32 + 100,
                    pixel_size: 16,
                },
                &RasterizedGlyph {
                    coverage: vec![255; 60 * 60],
                    width: 60,
                    height: 60,
                    bearing_x: 0,
                    bearing_y: 0,
                    advance_x: 60,
                },
            );
        }
        let after = glyph_quad(slot, 0.0, 0.0, [1.0; 4], &atlas);

        // Reusing the old coordinates would sample the wrong pixels, and the
        // glyph would come out as a slice of its neighbour.
        assert_ne!(before.tex_bottom, after.tex_bottom);
        assert!(after.tex_bottom < before.tex_bottom);
    }

    #[test]
    fn a_cell_with_the_frame_background_draws_no_background_quad() {
        let (atlas, slot) = atlas_with_glyph();
        let mut out = FrameQuads::default();

        build_row(
            &[cell('a', CellStyle::default())],
            0.0,
            0.0,
            metrics(),
            colors(),
            &atlas,
            |_| Some(slot),
            &mut out,
            &Default::default(),
        );

        // Filling every cell with the frame colour is the whole screen drawn
        // twice, every frame.
        assert!(out.backgrounds.is_empty());
        assert_eq!(out.glyphs.len(), 1);
    }

    #[test]
    fn a_coloured_cell_draws_its_background_over_exactly_its_cell() {
        let (atlas, slot) = atlas_with_glyph();
        let mut out = FrameQuads::default();
        let style = CellStyle {
            bg: Some(StyledColor::Rgb(255, 0, 0)),
            ..CellStyle::default()
        };

        build_row(
            &[cell('a', style)],
            0.0,
            40.0,
            metrics(),
            colors(),
            &atlas,
            |_| Some(slot),
            &mut out,
            &Default::default(),
        );

        let quad = out.backgrounds[0];
        assert_eq!((quad.left, quad.top), (0.0, 40.0));
        assert_eq!((quad.width, quad.height), (10.0, 20.0));
        assert_eq!(quad.color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_wide_cell_covers_both_its_columns() {
        let (atlas, slot) = atlas_with_glyph();
        let mut out = FrameQuads::default();
        let style = CellStyle {
            bg: Some(StyledColor::Rgb(0, 0, 255)),
            ..CellStyle::default()
        };
        let wide = StyledCell {
            ch: '漢',
            style,
            width: 2,
        };

        build_row(&[wide], 0.0, 0.0, metrics(), colors(), &atlas, |_| Some(slot), &mut out, &Default::default());

        // Covering only one column lets the frame background show through the
        // right half of the character.
        assert_eq!(out.backgrounds[0].width, 20.0);
    }

    #[test]
    fn cells_advance_by_their_own_width() {
        let (atlas, slot) = atlas_with_glyph();
        let mut out = FrameQuads::default();
        let style = CellStyle {
            bg: Some(StyledColor::Rgb(1, 2, 3)),
            ..CellStyle::default()
        };
        let wide = StyledCell {
            ch: '漢',
            style: style.clone(),
            width: 2,
        };

        build_row(
            &[wide, cell('a', style)],
            0.0,
            0.0,
            metrics(),
            colors(),
            &atlas,
            |_| Some(slot),
            &mut out,
            &Default::default(),
        );

        // The narrow cell starts after both columns of the wide one.
        assert_eq!(out.backgrounds[1].left, 20.0);
    }

    #[test]
    fn inverse_swaps_the_two_colours() {
        let (atlas, slot) = atlas_with_glyph();
        let mut out = FrameQuads::default();
        let style = CellStyle {
            inverse: true,
            ..CellStyle::default()
        };

        build_row(&[cell('a', style)], 0.0, 0.0, metrics(), colors(), &atlas, |_| Some(slot), &mut out, &Default::default());

        // What a selection highlight relies on: the background becomes the
        // frame's foreground, not some third colour.
        assert_eq!(out.backgrounds[0].color, colors().foreground);
        assert_eq!(out.glyphs[0].quad.color, colors().background);
    }

    #[test]
    fn a_space_draws_no_glyph() {
        let (atlas, slot) = atlas_with_glyph();
        let mut out = FrameQuads::default();

        build_row(&[cell(' ', CellStyle::default())], 0.0, 0.0, metrics(), colors(), &atlas, |_| Some(slot), &mut out, &Default::default());

        assert!(out.glyphs.is_empty());
    }

    #[test]
    fn a_hidden_cell_keeps_its_background_and_loses_its_glyph() {
        let (atlas, slot) = atlas_with_glyph();
        let mut out = FrameQuads::default();
        let style = CellStyle {
            hidden: true,
            bg: Some(StyledColor::Rgb(9, 9, 9)),
            ..CellStyle::default()
        };

        build_row(&[cell('a', style)], 0.0, 0.0, metrics(), colors(), &atlas, |_| Some(slot), &mut out, &Default::default());

        // `hidden` conceals the character, not the cell -- a password prompt
        // still occupies its space.
        assert_eq!(out.backgrounds.len(), 1);
        assert!(out.glyphs.is_empty());
    }

    #[test]
    fn a_glyph_the_atlas_does_not_have_skips_only_that_cell() {
        let (atlas, slot) = atlas_with_glyph();
        let mut out = FrameQuads::default();

        build_row(
            &[cell('a', CellStyle::default()), cell('b', CellStyle::default())],
            0.0,
            0.0,
            metrics(),
            colors(),
            &atlas,
            |ch| if ch == 'a' { None } else { Some(slot) },
            &mut out,
            &Default::default(),
        );

        // A gap is easier to diagnose than a blank line.
        assert_eq!(out.glyphs.len(), 1);
    }
}
