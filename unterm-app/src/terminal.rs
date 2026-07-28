//! A terminal on a surface: next-core's screen, unterm-render's pixels.
//!
//! Deliberately holds no window and no event loop, so the part that decides
//! what a frame looks like can be tested without opening anything.

use unterm_engine::next_core::font_raster::FontFace;
use unterm_engine::next_core::{config::Config, font_discovery};
use unterm_render::atlas::{GlyphAtlas, GlyphKey};
use unterm_render::quads::{build_row, CellMetrics, FrameColors, FrameQuads};
use unterm_engine::StyledScreenSnapshot;

/// The font and the cell it dictates.
///
/// Cell size comes from the font rather than the other way round: a terminal
/// grid that does not match its glyphs either clips them or leaves gaps, and
/// both are visible on every character.
pub struct TerminalFont {
    face: FontFace,
    metrics: CellMetrics,
}

impl TerminalFont {
    /// Open the machine's default monospace face at `pixel_size`.
    pub fn open(pixel_size: u32) -> anyhow::Result<Self> {
        let index = font_discovery::FontIndex::scan();
        let entry = index
            .default_monospace()
            .ok_or_else(|| anyhow::anyhow!("no monospace font found on this machine"))?;
        let face = FontFace::open(&entry.path, pixel_size)?;
        Ok(Self::from_face(face))
    }

    pub fn from_face(mut face: FontFace) -> Self {
        // Measure from a character every monospace face has, rather than
        // trusting a nominal size: hinting and rounding mean the advance for
        // `M` is what the grid actually has to be.
        let (advance, height, baseline) = match face.rasterize('M') {
            Ok(glyph) => {
                let advance = if glyph.advance_x > 0 {
                    glyph.advance_x as f32
                } else {
                    face.pixel_size() as f32 * 0.6
                };
                let ascent = glyph.bearing_y.max(1) as f32;
                (advance, ascent * 1.4, ascent * 1.15)
            }
            Err(_) => {
                let size = face.pixel_size() as f32;
                (size * 0.6, size * 1.2, size)
            }
        };

        Self {
            face,
            metrics: CellMetrics {
                width: advance,
                height,
                baseline,
            },
        }
    }

    pub fn metrics(&self) -> CellMetrics {
        self.metrics
    }

    pub fn face_mut(&mut self) -> &mut FontFace {
        &mut self.face
    }

    /// How many cells fit in a window of this size.
    ///
    /// At least one of each: a zero-sized grid makes the PTY reject the resize
    /// and the shell draw nothing.
    pub fn grid_for(&self, width: f32, height: f32) -> (usize, usize) {
        // A window sized to exactly N cells divides to 23.999... rather than
        // 24, and flooring that loses a row: an empty strip along the bottom,
        // and a shell that believes it is shorter than the window shows. The
        // slack is far below a pixel, so it cannot claim a cell that is not
        // there.
        const SLACK: f32 = 1e-3;
        let cols = (width / self.metrics.width + SLACK).floor().max(1.0) as usize;
        let rows = (height / self.metrics.height + SLACK).floor().max(1.0) as usize;
        (cols, rows)
    }
}

/// Turn a screen snapshot into the quads for one frame.
pub fn frame_quads(
    snapshot: &StyledScreenSnapshot,
    font: &mut TerminalFont,
    atlas: &mut GlyphAtlas,
    colors: FrameColors,
) -> FrameQuads {
    let metrics = font.metrics();
    let mut quads = FrameQuads::default();

    // Rasterize every character this frame needs before building quads, so the
    // atlas cannot grow midway and leave earlier texture coordinates pointing
    // at the wrong pixels.
    for line in &snapshot.lines {
        for cell in &line.cells {
            ensure_glyph(font, atlas, cell.ch);
        }
    }

    let pixel_size = font.face_mut().pixel_size();
    for (row, line) in snapshot.lines.iter().enumerate() {
        build_row(
            &line.cells,
            row as f32 * metrics.height,
            metrics,
            colors,
            atlas,
            |ch| {
                atlas.get(GlyphKey {
                    face: 0,
                    glyph_index: ch as u32,
                    pixel_size,
                })
            },
            &mut quads,
        );
    }

    quads
}

/// Put a character in the atlas if it is not already there.
///
/// Keyed by character rather than by shaped glyph index: shaping comes later,
/// and a terminal's grid means most cells are one character to one glyph.
fn ensure_glyph(font: &mut TerminalFont, atlas: &mut GlyphAtlas, ch: char) {
    if ch == ' ' || ch == '\0' {
        return;
    }
    let pixel_size = font.face_mut().pixel_size();
    let key = GlyphKey {
        face: 0,
        glyph_index: ch as u32,
        pixel_size,
    };
    if atlas.get(key).is_some() {
        return;
    }
    if let Ok(glyph) = font.face_mut().rasterize(ch) {
        atlas.insert(key, &glyph);
    }
}

/// Frame colours from a declarative config, falling back to a readable pair.
pub fn colors_from(config: &Config) -> FrameColors {
    use unterm_engine::next_core::color::parse_hex;

    let background = config
        .str_of("colors.background")
        .ok()
        .flatten()
        .and_then(parse_hex);
    let foreground = config
        .str_of("colors.foreground")
        .ok()
        .flatten()
        .and_then(parse_hex);

    FrameColors {
        background: background
            .map(|color| {
                [
                    color.red as f32 / 255.0,
                    color.green as f32 / 255.0,
                    color.blue as f32 / 255.0,
                    1.0,
                ]
            })
            .unwrap_or([0.07, 0.07, 0.08, 1.0]),
        foreground: foreground
            .map(|color| {
                [
                    color.red as f32 / 255.0,
                    color.green as f32 / 255.0,
                    color.blue as f32 / 255.0,
                    1.0,
                ]
            })
            .unwrap_or([0.91, 0.92, 0.93, 1.0]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unterm_engine::{CellStyle, StyledCell, StyledScreenLine};

    fn font() -> Option<TerminalFont> {
        TerminalFont::open(16).ok()
    }

    fn snapshot(text: &str) -> StyledScreenSnapshot {
        StyledScreenSnapshot {
            lines: vec![StyledScreenLine {
                row: 0,
                wrapped: false,
                cells: text
                    .chars()
                    .map(|ch| StyledCell {
                        ch,
                        style: CellStyle::default(),
                        width: 1,
                    })
                    .collect(),
            }],
            cursor: unterm_engine::CursorSnapshot {
                x: 0,
                y: 0,
                visible: true,
                shape: "block".to_string(),
            },
            cols: text.chars().count(),
            rows: 1,
            scrollback_rows: 0,
            revision: 1,
            dirty_rows: None,
        }
    }

    #[test]
    fn a_cell_is_as_wide_as_the_font_says() {
        let Some(font) = font() else {
            return;
        };

        // A grid that does not match its glyphs clips them or leaves gaps, and
        // either is visible on every character.
        assert!(font.metrics().width > 0.0);
        assert!(font.metrics().height > font.metrics().width);
        assert!(font.metrics().baseline < font.metrics().height);
    }

    #[test]
    fn a_window_of_a_given_size_holds_a_sensible_grid() {
        let Some(font) = font() else {
            return;
        };
        let metrics = font.metrics();

        let (cols, rows) = font.grid_for(metrics.width * 80.0, metrics.height * 24.0);

        assert_eq!((cols, rows), (80, 24));
    }

    #[test]
    fn a_window_sized_to_exactly_n_cells_gets_n() {
        let Some(font) = font() else {
            return;
        };
        let metrics = font.metrics();

        // Dividing by the cell size lands just under the whole number, and
        // flooring that costs a row -- an empty strip along the bottom and a
        // shell that thinks it is shorter than the window.
        for rows in [1usize, 5, 24, 60] {
            assert_eq!(
                font.grid_for(metrics.width * 80.0, metrics.height * rows as f32).1,
                rows
            );
        }
    }

    #[test]
    fn a_window_too_small_for_one_cell_still_asks_for_one() {
        let Some(font) = font() else {
            return;
        };

        // A zero-sized grid makes the PTY reject the resize and the shell draw
        // nothing at all.
        assert_eq!(font.grid_for(1.0, 1.0), (1, 1));
    }

    #[test]
    fn text_becomes_glyph_quads() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };

        let quads = frame_quads(&snapshot("hi"), &mut font, &mut atlas, colors);

        assert_eq!(quads.glyphs.len(), 2);
        // Left to right, one cell apart.
        assert!(quads.glyphs[1].quad.left > quads.glyphs[0].quad.left);
    }

    #[test]
    fn every_glyph_is_rasterized_before_any_quad_is_built() {
        let Some(mut font) = font() else {
            return;
        };
        // An atlas small enough that a line of text has to grow it.
        let mut atlas = GlyphAtlas::new(32, 32);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };

        let quads = frame_quads(&snapshot("abcdefghij"), &mut font, &mut atlas, colors);

        // Growing midway would leave the earlier quads' texture coordinates
        // pointing at the wrong pixels, and those glyphs would come out as
        // slices of their neighbours.
        let height = atlas.height() as f32;
        for glyph in &quads.glyphs {
            let expected_bottom = glyph.tex_bottom * height;
            assert!(
                expected_bottom <= height + 0.001,
                "texture coordinate outside the atlas: {glyph:?}"
            );
        }
    }

    #[test]
    fn spaces_cost_nothing() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };

        let quads = frame_quads(&snapshot("   "), &mut font, &mut atlas, colors);

        assert!(quads.glyphs.is_empty());
        assert!(atlas.is_empty());
    }

    #[test]
    fn colours_come_from_the_config_when_it_states_them() {
        let config = unterm_engine::next_core::config::parse(
            "[colors]\nbackground = \"#102030\"\nforeground = \"#a0b0c0\"",
        )
        .expect("config should parse");

        let colors = colors_from(&config);

        assert!((colors.background[0] - 0x10 as f32 / 255.0).abs() < 0.01);
        assert!((colors.foreground[2] - 0xc0 as f32 / 255.0).abs() < 0.01);
    }

    #[test]
    fn a_config_without_colours_still_gives_readable_ones() {
        let config = unterm_engine::next_core::config::parse("font_size = 13").unwrap();

        let colors = colors_from(&config);

        // Foreground and background must differ, or the window is blank.
        assert_ne!(colors.foreground, colors.background);
    }
}
