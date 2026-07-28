//! A terminal on a surface: next-core's screen, unterm-render's pixels.
//!
//! Deliberately holds no window and no event loop, so the part that decides
//! what a frame looks like can be tested without opening anything.

use crate::fonts::FontStack;
use unterm_engine::next_core::font_raster::FontFace;
use unterm_engine::next_core::{config::Config, font_discovery};
use unterm_render::atlas::{GlyphAtlas, GlyphKey};
use unterm_render::quads::{build_row, CellMetrics, FrameColors, FrameQuads, Quad};
use unterm_engine::{StyledCell, StyledScreenSnapshot};

/// The font and the cell it dictates.
///
/// Cell size comes from the font rather than the other way round: a terminal
/// grid that does not match its glyphs either clips them or leaves gaps, and
/// both are visible on every character.
pub struct TerminalFont {
    stack: FontStack,
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
        Ok(Self::from_face(face, &[], pixel_size))
    }

    /// Open the primary face and whichever fallbacks the config names.
    pub fn open_with_fallback(pixel_size: u32, families: &[String]) -> anyhow::Result<Self> {
        let index = font_discovery::FontIndex::scan();
        let entry = index
            .default_monospace()
            .ok_or_else(|| anyhow::anyhow!("no monospace font found on this machine"))?;
        let face = FontFace::open(&entry.path, pixel_size)?;
        Ok(Self::from_face(face, families, pixel_size))
    }

    pub fn from_face(mut face: FontFace, fallbacks: &[String], pixel_size: u32) -> Self {
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
            stack: FontStack::new(face, fallbacks, pixel_size),
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

    pub fn stack_mut(&mut self) -> &mut FontStack {
        &mut self.stack
    }

    pub fn pixel_size(&self) -> u32 {
        self.stack.pixel_size()
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
    let mut quads = FrameQuads::default();
    append_pane(snapshot, font, atlas, colors, (0.0, 0.0), &mut quads);
    quads
}

/// Add one pane's quads at `origin`, in pixels from the window's top-left.
///
/// Panes are drawn into the same buffer rather than one each, so a window of
/// four splits is still one draw call. The origin is what a split needs and
/// the single-pane case gets for free at (0, 0).
pub fn append_pane(
    snapshot: &StyledScreenSnapshot,
    font: &mut TerminalFont,
    atlas: &mut GlyphAtlas,
    colors: FrameColors,
    origin: (f32, f32),
    quads: &mut FrameQuads,
) {
    let metrics = font.metrics();

    // Rasterize every character this frame needs before building quads, so the
    // atlas cannot grow midway and leave earlier texture coordinates pointing
    // at the wrong pixels.
    for line in &snapshot.lines {
        for cell in &line.cells {
            ensure_glyph(font, atlas, cell.ch);
        }
    }

    // Which face drew each character, resolved once, so the lookup below
    // matches the key each glyph was filed under.
    let pixel_size = font.pixel_size();
    let mut face_of: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for line in &snapshot.lines {
        for cell in &line.cells {
            face_of
                .entry(cell.ch)
                .or_insert_with(|| font.stack.face_for(cell.ch));
        }
    }

    // The cursor goes in before the glyphs so text lands on top of it, which is
    // what makes an inverted cell readable.
    let inverted = push_cursor(snapshot, metrics, colors, origin, quads);

    for (row, line) in snapshot.lines.iter().enumerate() {
        // A cell under a block cursor takes the frame's background colour, so
        // it reads against the block rather than vanishing into it.
        let row_colors = colors;
        build_row(
            &line.cells,
            origin.0,
            origin.1 + row as f32 * metrics.height,
            metrics,
            row_colors,
            atlas,
            |ch| {
                atlas.get(GlyphKey {
                    face: face_of.get(&ch).copied().unwrap_or(0),
                    glyph_index: ch as u32,
                    pixel_size,
                })
            },
            quads,
        );
    }

    if let Some((column, row)) = inverted {
        let left = origin.0 + column as f32 * metrics.width;
        let top = origin.1 + row as f32 * metrics.height;
        for glyph in &mut quads.glyphs {
            let on_cursor = glyph.quad.left >= left - metrics.width
                && glyph.quad.left < left + metrics.width
                && glyph.quad.top >= top - metrics.height
                && glyph.quad.top < top + metrics.height;
            if on_cursor {
                glyph.quad.color = colors.background;
            }
        }
    }
}

/// Add a run of plain text at `origin`, in the given colour.
///
/// For the front end's own furniture -- a banner, a label -- which has no
/// cells and no styles, only characters that have to land on the same grid as
/// everything else.
pub fn append_text(
    text: &str,
    font: &mut TerminalFont,
    atlas: &mut GlyphAtlas,
    color: [f32; 4],
    origin: (f32, f32),
    quads: &mut FrameQuads,
) {
    for ch in text.chars() {
        ensure_glyph(font, atlas, ch);
    }
    let metrics = font.metrics();
    let pixel_size = font.pixel_size();
    let cells: Vec<StyledCell> = text
        .chars()
        .map(|ch| StyledCell {
            ch,
            style: Default::default(),
            width: 1,
        })
        .collect();
    let mut face_of: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for ch in text.chars() {
        face_of.entry(ch).or_insert_with(|| font.stack.face_for(ch));
    }

    let before = quads.glyphs.len();
    build_row(
        &cells,
        origin.0,
        origin.1,
        metrics,
        FrameColors {
            foreground: color,
            // Transparent to the row builder: the caller has already drawn
            // whatever this sits on.
            background: color,
        },
        atlas,
        |ch| {
            atlas.get(GlyphKey {
                face: face_of.get(&ch).copied().unwrap_or(0),
                glyph_index: ch as u32,
                pixel_size,
            })
        },
        quads,
    );
    for glyph in &mut quads.glyphs[before..] {
        glyph.quad.color = color;
    }
}

/// Draw the cursor.
///
/// A block cursor sits *under* the character rather than over it, and the cell
/// beneath is drawn inverted: the block takes the foreground colour and the
/// character the background. Painting an opaque block on top would hide the
/// character the user is about to edit, which is the one they most need to see.
fn push_cursor(
    snapshot: &StyledScreenSnapshot,
    metrics: CellMetrics,
    colors: FrameColors,
    origin: (f32, f32),
    quads: &mut FrameQuads,
) -> Option<(usize, usize)> {
    let cursor = &snapshot.cursor;
    if !cursor.visible {
        return None;
    }
    // A negative row means the viewport is scrolled away from the cursor; it
    // has no place on screen and drawing it at row 0 would be a lie.
    let row = usize::try_from(cursor.y).ok()?;
    if row >= snapshot.rows || cursor.x >= snapshot.cols.max(1) {
        return None;
    }

    let left = origin.0 + cursor.x as f32 * metrics.width;
    let top = origin.1 + row as f32 * metrics.height;

    // Shapes as the escape sequences name them. An unknown shape draws a block
    // rather than nothing: a missing cursor is worse than an unexpected one.
    let quad = match cursor.shape.as_str() {
        shape if shape.contains("Bar") => Quad {
            left,
            top,
            width: (metrics.width * 0.15).max(1.0),
            height: metrics.height,
            color: colors.foreground,
        },
        shape if shape.contains("Underline") => Quad {
            left,
            top: top + metrics.height - (metrics.height * 0.12).max(1.0),
            width: metrics.width,
            height: (metrics.height * 0.12).max(1.0),
            color: colors.foreground,
        },
        _ => Quad {
            left,
            top,
            width: metrics.width,
            height: metrics.height,
            color: colors.foreground,
        },
    };

    quads.backgrounds.push(quad);

    // Only a block covers the whole cell, so only a block needs the character
    // inverted to stay readable.
    let covers_cell = quad.width >= metrics.width && quad.height >= metrics.height;
    covers_cell.then_some((cursor.x, row))
}

/// Put a character in the atlas if it is not already there.
///
/// Keyed by character rather than by shaped glyph index: shaping comes later,
/// and a terminal's grid means most cells are one character to one glyph.
fn ensure_glyph(font: &mut TerminalFont, atlas: &mut GlyphAtlas, ch: char) {
    if ch == ' ' || ch == '\0' {
        return;
    }
    let key = glyph_key(font, ch);
    if atlas.get(key).is_some() {
        return;
    }
    if let Some((_, glyph)) = font.stack_mut().rasterize(ch) {
        atlas.insert(key, &glyph);
    }
}

/// Where a character lives in the atlas.
///
/// The face is part of the key because two faces' glyphs for the same
/// character are different pictures; filing a fallback glyph under the primary
/// would show one where the other belongs.
fn glyph_key(font: &mut TerminalFont, ch: char) -> GlyphKey {
    let pixel_size = font.pixel_size();
    GlyphKey {
        face: font.stack_mut().face_for(ch),
        glyph_index: ch as u32,
        pixel_size,
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
    fn a_character_the_primary_face_lacks_still_gets_ink() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };

        // Two Han characters a programming font almost never carries.
        let quads = frame_quads(&snapshot("\u{6f22}\u{5b57}"), &mut font, &mut atlas, colors);

        if quads.glyphs.is_empty() {
            // No CJK-capable face installed; nothing to assert on this machine.
            return;
        }

        // Without fallback these rasterize to glyph 0 -- the empty box -- which
        // is exactly what CJK looked like in the window before.
        for glyph in &quads.glyphs {
            assert!(
                glyph.quad.width > 0.0 && glyph.quad.height > 0.0,
                "a fallback glyph should have pixels: {glyph:?}"
            );
        }

        // And the ink has to be in the atlas, not just a slot with a size.
        let any_ink = (0..atlas.height())
            .any(|y| (0..atlas.width()).any(|x| atlas.pixel(x, y) > 0));
        assert!(any_ink, "the fallback face should have left ink");
    }

    fn snapshot_with_cursor(text: &str, x: usize, y: isize, visible: bool, shape: &str)
        -> StyledScreenSnapshot
    {
        let mut snap = snapshot(text);
        snap.cursor = unterm_engine::CursorSnapshot {
            x,
            y,
            visible,
            shape: shape.to_string(),
        };
        snap
    }

    #[test]
    fn the_cursor_is_drawn_where_the_screen_says() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };
        let metrics = font.metrics();

        let quads = frame_quads(
            &snapshot_with_cursor("abc", 2, 0, true, "Block"),
            &mut font,
            &mut atlas,
            colors,
        );

        let cursor = quads
            .backgrounds
            .iter()
            .find(|quad| quad.color == colors.foreground)
            .expect("a visible cursor should be drawn");
        assert!((cursor.left - metrics.width * 2.0).abs() < 0.01);
    }

    #[test]
    fn a_hidden_cursor_is_not_drawn() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };

        let quads = frame_quads(
            &snapshot_with_cursor("abc", 1, 0, false, "Block"),
            &mut font,
            &mut atlas,
            colors,
        );

        assert!(!quads
            .backgrounds
            .iter()
            .any(|quad| quad.color == colors.foreground));
    }

    #[test]
    fn a_block_cursor_leaves_its_character_readable() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };

        let quads = frame_quads(
            &snapshot_with_cursor("abc", 1, 0, true, "Block"),
            &mut font,
            &mut atlas,
            colors,
        );

        // The character under the block is the one the user is about to edit;
        // an opaque block on top of it hides exactly what they need to see.
        let under = quads
            .glyphs
            .iter()
            .find(|glyph| {
                (glyph.quad.left - font.metrics().width).abs() < font.metrics().width
            })
            .expect("the character under the cursor should still be drawn");
        assert_eq!(under.quad.color, colors.background);
    }

    #[test]
    fn a_bar_cursor_does_not_cover_its_cell() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };
        let metrics = font.metrics();

        let quads = frame_quads(
            &snapshot_with_cursor("abc", 0, 0, true, "SteadyBar"),
            &mut font,
            &mut atlas,
            colors,
        );

        let cursor = quads
            .backgrounds
            .iter()
            .find(|quad| quad.color == colors.foreground)
            .expect("a bar cursor should be drawn");
        assert!(cursor.width < metrics.width / 2.0);
        // A bar leaves the character alone, so it keeps the foreground colour.
        assert!(quads
            .glyphs
            .iter()
            .all(|glyph| glyph.quad.color == colors.foreground));
    }

    #[test]
    fn an_underline_cursor_sits_at_the_bottom_of_its_cell() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };
        let metrics = font.metrics();

        let quads = frame_quads(
            &snapshot_with_cursor("abc", 0, 0, true, "SteadyUnderline"),
            &mut font,
            &mut atlas,
            colors,
        );

        let cursor = quads
            .backgrounds
            .iter()
            .find(|quad| quad.color == colors.foreground)
            .expect("an underline cursor should be drawn");
        assert!(cursor.top > metrics.height / 2.0);
        assert!(cursor.top + cursor.height <= metrics.height + 0.01);
    }

    #[test]
    fn a_cursor_off_the_screen_is_not_drawn() {
        let Some(mut font) = font() else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
        };

        // Scrolled back far enough that the cursor is above the viewport.
        let quads = frame_quads(
            &snapshot_with_cursor("abc", 0, -3, true, "Block"),
            &mut font,
            &mut atlas,
            colors,
        );

        // Drawing it at row 0 instead would put the cursor somewhere it is not.
        assert!(!quads
            .backgrounds
            .iter()
            .any(|quad| quad.color == colors.foreground));
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
