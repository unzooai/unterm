//! A terminal on a surface: next-core's screen, unterm-render's pixels.
//!
//! Deliberately holds no window and no event loop, so the part that decides
//! what a frame looks like can be tested without opening anything.

use crate::fonts::FontStack;
use unterm_engine::next_core::font_raster::FontFace;
use unterm_engine::next_core::{config::Config, font_discovery};
use unterm_engine::{StyledCell, StyledScreenSnapshot};
use unterm_render::atlas::{GlyphAtlas, GlyphKey};
use unterm_render::quads::{build_row, CellMetrics, FrameColors, FrameQuads, Quad};

/// Pixels per em for a size in points on a display at `scale`.
///
/// A point is 1/72 inch, and winit reports scale against 96 dpi -- so the
/// pixel size is points * 96 * scale / 72. The previous front end did the same
/// arithmetic; this one skipped it and used the point size as pixels, which on
/// a 1.5x display drew every glyph at half the size it should be. That is most
/// of what "it still looks very different" was.
pub fn pixels_for_points(points: f32, scale: f32) -> u32 {
    const POINTS_PER_INCH: f32 = 72.0;
    const NOMINAL_DPI: f32 = 96.0;
    // Never zero: a face opened at no pixels rasterizes nothing, and the
    // window comes up blank with no error to explain it.
    (((points.max(1.0) * NOMINAL_DPI * scale.max(0.1)) / POINTS_PER_INCH).round() as u32).max(1)
}

/// How a cursor is drawn when the program has not asked for a shape.
///
/// The config's own names, as the previous front end spelled them:
/// `SteadyBlock`, `BlinkingBlock`, `SteadyUnderline`, `BlinkingUnderline`,
/// `SteadyBar`, `BlinkingBar`. A program's own escape sequence still wins --
/// this is only what it looks like when nothing has said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CursorStyle {
    pub shape: CursorShape,
    pub blinking: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Underline,
    Bar,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self {
            shape: CursorShape::Block,
            blinking: false,
        }
    }
}

impl CursorStyle {
    /// Parse one of the config's names. Anything else is the default rather
    /// than an error: a typo should not stop the terminal opening, and a block
    /// cursor is the one nobody is surprised by.
    pub fn parse(name: &str) -> Self {
        let lowered = name.trim().to_lowercase();
        let blinking = lowered.starts_with("blinking");
        let shape = if lowered.ends_with("underline") {
            CursorShape::Underline
        } else if lowered.ends_with("bar") {
            CursorShape::Bar
        } else {
            CursorShape::Block
        };
        if !lowered.starts_with("blinking") && !lowered.starts_with("steady") {
            return Self::default();
        }
        Self { shape, blinking }
    }

    /// The style from the config, and how fast it blinks.
    ///
    /// Zero milliseconds means no blinking, which is how the setting has
    /// always been turned off.
    pub fn from_config(config: &Config) -> (Self, u64) {
        let style = config
            .str_of("default_cursor_style")
            .ok()
            .flatten()
            .map(|name| Self::parse(&name))
            .unwrap_or_default();
        let rate = config
            .float_of("cursor_blink_rate")
            .ok()
            .flatten()
            .map(|value| value.max(0.0) as u64)
            .unwrap_or(800);
        (style, rate)
    }
}

/// Whether a blinking cursor is showing at this moment.
///
/// Half the period on, half off. A rate of zero is not a blink of no length --
/// it is the setting turned off, and the cursor stays put.
pub fn blink_is_on(elapsed_ms: u128, rate_ms: u64) -> bool {
    if rate_ms == 0 {
        return true;
    }
    (elapsed_ms / rate_ms as u128) % 2 == 0
}

/// How far the cell is stretched around its glyphs.
///
/// Both default to one, which is the font's own metrics. They exist because a
/// terminal's readability is as much about the space between lines as about
/// the letters, and the config has always been able to say so.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub line_height: f32,
    pub cell_width: f32,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            line_height: 1.0,
            cell_width: 1.0,
        }
    }
}

impl Shape {
    /// From the config, refusing anything that would collapse the cell.
    ///
    /// A zero or negative multiplier is a grid with no rows, which divides by
    /// nothing and draws nothing; the clamp keeps a typo from producing a
    /// blank window with no explanation.
    pub fn from_config(config: &Config) -> Self {
        let number = |key: &str, fallback: f32| {
            config
                .float_of(key)
                .ok()
                .flatten()
                .map(|value| (value as f32).clamp(0.5, 4.0))
                .unwrap_or(fallback)
        };
        Self {
            line_height: number("line_height", 1.0),
            cell_width: number("cell_width", 1.0),
        }
    }
}

/// The font and the cell it dictates.
///
/// Cell size comes from the font rather than the other way round: a terminal
/// grid that does not match its glyphs either clips them or leaves gaps, and
/// both are visible on every character.
pub struct TerminalFont {
    stack: FontStack,
    metrics: CellMetrics,
    /// Which stack this is, for the atlas.
    ///
    /// Two fonts are open at once: the terminal's, and the chrome's at its own
    /// size. A face index means nothing without saying whose stack it indexes,
    /// and without this the two overwrite each other in the atlas whenever they
    /// land on the same pixel size.
    stack_id: u8,
}

impl TerminalFont {
    /// Open the machine's default monospace face at `pixel_size`.
    #[cfg(test)]
    pub fn open(pixel_size: u32) -> anyhow::Result<Self> {
        let index = font_discovery::FontIndex::scan();
        let entry = index
            .default_monospace()
            .ok_or_else(|| anyhow::anyhow!("no monospace font found on this machine"))?;
        let face = FontFace::open(&entry.path, pixel_size)?;
        Ok(Self::from_face(face, &[], pixel_size))
    }

    /// Open a named family, or the machine's default monospace.
    ///
    /// A name that is not installed falls back rather than refusing to start:
    /// a config naming a font from another machine is the ordinary case, and a
    /// terminal that will not open is no way to find out about it.
    pub fn open_named(
        family: Option<&str>,
        pixel_size: u32,
        fallbacks: &[String],
        shape: Shape,
    ) -> anyhow::Result<Self> {
        let index = font_discovery::FontIndex::scan();
        let entry = family
            .and_then(|name| index.family(name).first())
            .or_else(|| index.default_monospace())
            .ok_or_else(|| anyhow::anyhow!("no monospace font found on this machine"))?;
        if let Some(name) = family {
            if index.family(name).is_empty() {
                log::warn!("font {name:?} is not installed; using the default");
            }
        }
        let face = FontFace::open(&entry.path, pixel_size)?;
        Ok(Self::from_face_shaped(face, fallbacks, pixel_size, shape))
    }

    #[cfg(test)]
    pub fn from_face(face: FontFace, fallbacks: &[String], pixel_size: u32) -> Self {
        Self::from_face_shaped(face, fallbacks, pixel_size, Shape::default())
    }

    /// As `from_face`, with the cell stretched by the config's multipliers.
    pub fn from_face_shaped(
        mut face: FontFace,
        fallbacks: &[String],
        pixel_size: u32,
        shape: Shape,
    ) -> Self {
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

        // The cell can be stretched without changing the glyphs: `line_height`
        // opens the text up without making it bigger, and `cell_width` is how
        // a narrow font is given room to breathe. The baseline moves with the
        // extra height so the text stays centred in the taller cell rather
        // than sitting on its old line with a gap underneath.
        let height = height * shape.line_height;
        let baseline = baseline + (height - baseline) * (shape.line_height - 1.0) * 0.5;

        Self {
            stack: FontStack::new(face, fallbacks, pixel_size),
            metrics: CellMetrics {
                width: advance * shape.cell_width,
                height,
                baseline,
            },
            stack_id: 0,
        }
    }

    /// The same face, filed in the atlas under a different stack.
    ///
    /// Used for the chrome's own font: it is a different stack at a different
    /// size, and the atlas has to be able to tell the two apart.
    pub fn as_stack(mut self, stack_id: u8) -> Self {
        self.stack_id = stack_id;
        self
    }

    pub fn stack_id(&self) -> u8 {
        self.stack_id
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
#[cfg(test)]
pub fn frame_quads(
    snapshot: &StyledScreenSnapshot,
    font: &mut TerminalFont,
    atlas: &mut GlyphAtlas,
    colors: FrameColors,
) -> FrameQuads {
    let mut quads = FrameQuads::default();
    append_pane(
        snapshot,
        font,
        atlas,
        colors,
        (0.0, 0.0),
        true,
        CursorStyle::default(),
        &mut quads,
    );
    quads
}

/// Add one pane's quads at `origin`, in pixels from the window's top-left.
///
/// Panes are drawn into the same buffer rather than one each, so a window of
/// four splits is still one draw call. The origin is what a split needs and
/// the single-pane case gets for free at (0, 0).
#[allow(clippy::too_many_arguments)]
pub fn append_pane(
    snapshot: &StyledScreenSnapshot,
    font: &mut TerminalFont,
    atlas: &mut GlyphAtlas,
    colors: FrameColors,
    origin: (f32, f32),
    solid_cursor: bool,
    cursor: CursorStyle,
    quads: &mut FrameQuads,
) {
    let metrics = font.metrics();

    // Shape every row and rasterize everything this frame needs *before*
    // building any quads. The atlas grows, and growing it renormalizes every
    // texture coordinate -- so a glyph placed before a later one made the
    // atlas bigger ends up sampling the wrong pixels. It shows up as
    // characters missing from the middle of a word, which is how this was
    // found: "example.com" rendered as "exampl  com".
    let mut shaped_rows: Vec<Vec<(usize, ShapedRun)>> = Vec::with_capacity(snapshot.lines.len());
    for line in &snapshot.lines {
        shaped_rows.push(shape_row(&line.cells, font));
    }
    for line in &snapshot.lines {
        for cell in &line.cells {
            ensure_glyph(font, atlas, cell.ch);
        }
    }
    for row in &shaped_rows {
        for (face, run) in row {
            for glyph in &run.glyphs {
                ensure_shaped_glyph(font, atlas, *face, glyph.glyph_index);
            }
        }
    }

    // Which face drew each character, resolved once, so the lookup below
    // matches the key each glyph was filed under.
    let pixel_size = font.pixel_size();
    let stack_id = font.stack_id();
    let mut face_of: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    let mut index_of: std::collections::HashMap<char, u32> = std::collections::HashMap::new();
    for line in &snapshot.lines {
        for cell in &line.cells {
            let face = *face_of
                .entry(cell.ch)
                .or_insert_with(|| font.stack.face_for(cell.ch));
            index_of.entry(cell.ch).or_insert_with(|| {
                font.stack
                    .glyph_index_for(face, cell.ch)
                    .unwrap_or_default()
            });
        }
    }

    // The cursor goes in before the glyphs so text lands on top of it, which is
    // what makes an inverted cell readable.
    let inverted = push_cursor(
        snapshot,
        metrics,
        colors,
        origin,
        solid_cursor,
        cursor,
        quads,
    );

    for (row, line) in snapshot.lines.iter().enumerate() {
        let top = origin.1 + row as f32 * metrics.height;

        // Backgrounds and any cell the shaper could not draw. Shaping runs
        // first so it can claim the columns it drew; what it leaves is drawn
        // a character at a time, which is right for a font with no ligatures
        // and the only option for one the shaper will not open.
        let shaped = place_shaped_row(
            &shaped_rows[row],
            &line.cells,
            origin.0,
            top,
            metrics,
            colors,
            pixel_size,
            stack_id,
            atlas,
            quads,
        );
        build_row(
            &line.cells,
            origin.0,
            top,
            metrics,
            colors,
            atlas,
            |ch| {
                let face = face_of.get(&ch).copied().unwrap_or(0);
                atlas.get(GlyphKey {
                    stack: stack_id,
                    face,
                    glyph_index: index_of.get(&ch).copied().unwrap_or_default(),
                    pixel_size,
                })
            },
            quads,
            &shaped,
        );
    }

    if let Some((column, row)) = inverted {
        // The cursor's own cell, and nothing around it. A glyph is placed by
        // its bearing, so it can start slightly left of its cell and its top
        // sits well above the cell's -- which is why this compares against
        // the cell the glyph *belongs to* rather than against a box drawn
        // around the cursor. A window of plus-or-minus one cell, which is
        // what this used to be, silently painted the row above the cursor in
        // the background colour: characters directly over the prompt
        // disappeared, and only there.
        let cell_of = |left: f32, top: f32| {
            (
                ((left - origin.0) / metrics.width.max(1.0))
                    .floor()
                    .max(0.0) as usize,
                ((top - origin.1) / metrics.height.max(1.0))
                    .floor()
                    .max(0.0) as usize,
            )
        };
        for glyph in &mut quads.glyphs {
            // Measured from the glyph's baseline row rather than its top: a
            // tall glyph starts above its own cell.
            let baseline_top = glyph.quad.top + glyph.quad.height;
            let (glyph_column, glyph_row) = cell_of(glyph.quad.left, baseline_top - 1.0);
            if glyph_column == column && glyph_row == row {
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
    // A character's real width, not one cell each. Anything but Latin has
    // double-width characters in it, and a label that calls them one cell
    // wide draws the next character on top of the last -- which is what the
    // quick menu looked like the first time it was translated.
    let cells: Vec<StyledCell> = text
        .chars()
        .map(|ch| StyledCell {
            ch,
            style: Default::default(),
            width: column_width(ch),
        })
        .collect();
    // Keyed the same way `ensure_glyph` filed them. Working the key out
    // separately here is how the front end's own text came to be looked up by
    // code point while it was stored by glyph index: every label, banner and
    // bar drew whatever glyph happened to have that number, which is not the
    // letter asked for and often nothing at all.
    let mut key_of: std::collections::HashMap<char, GlyphKey> = std::collections::HashMap::new();
    for ch in text.chars() {
        if let std::collections::hash_map::Entry::Vacant(slot) = key_of.entry(ch) {
            slot.insert(glyph_key(font, ch));
        }
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
            // Furniture names its own colour; nothing here is themed.
            palette: &unterm_render::quads::DEFAULT_PALETTE,
        },
        atlas,
        |ch| key_of.get(&ch).and_then(|key| atlas.get(*key)),
        quads,
        // Plain text, drawn a character at a time: the front end's own
        // furniture has no ligatures to find.
        &std::collections::HashSet::new(),
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
    // Whether to draw the solid cursor rather than its outline. Two things
    // make it hollow: the pane not having the keyboard, and a blinking cursor
    // being mid-blink. An outline in both cases rather than nothing at all --
    // a cursor that vanishes entirely is one people lose track of, and the
    // outline still says where it is.
    solid: bool,
    style: CursorStyle,
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

    // This is how the previous front end said which pane had the keyboard --
    // rather than dimming the pane itself, which leaves a visible brightness
    // step down the split seam and into the status bar.
    if !solid {
        quads.backgrounds.extend(unterm_render::strokes::rectangle(
            left,
            top,
            metrics.width,
            metrics.height,
            (metrics.height / 14.0).round().max(1.0),
            colors.foreground,
        ));
        // Nothing is inverted: the character underneath stays as it was.
        return None;
    }

    // Shapes as the escape sequences name them, falling back to what the
    // config asked for. An unknown shape draws a block rather than nothing: a
    // missing cursor is worse than an unexpected one.
    let named = if cursor.shape.trim().is_empty() {
        match style.shape {
            CursorShape::Bar => "Bar",
            CursorShape::Underline => "Underline",
            CursorShape::Block => "Block",
        }
    } else {
        cursor.shape.as_str()
    };
    let quad = match named {
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
/// How many cells a character occupies.
///
/// From termwiz's tables rather than a guess at ranges: the kernel measures
/// the grid the same way, and furniture that disagrees with the grid it sits
/// on is furniture in the wrong place.
pub fn column_width(ch: char) -> usize {
    let mut buffer = [0u8; 4];
    termwiz::cell::unicode_column_width(ch.encode_utf8(&mut buffer), None).max(1)
}

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

/// A run of a row, shaped.
pub struct ShapedRun {
    run: crate::shape::Run,
    glyphs: Vec<unterm_engine::next_core::font_shaper::ShapedGlyph>,
}

/// Shape every run of a row, without touching the atlas.
///
/// Separate from placing so that a whole frame can be shaped and rasterized
/// before any quad is built: the atlas grows, and growing it invalidates the
/// texture coordinates of everything already placed.
fn shape_row(cells: &[StyledCell], font: &mut TerminalFont) -> Vec<(usize, ShapedRun)> {
    let runs = {
        let stack = font.stack_mut();
        crate::shape::runs(cells, |ch| stack.face_for(ch))
    };

    let mut out = Vec::new();
    for run in runs {
        let Some(first) = run.text.chars().next() else {
            continue;
        };
        let face = font.stack_mut().face_for(first);
        let Some(glyphs) = font.stack_mut().shape(face, &run.text) else {
            continue;
        };
        out.push((face, ShapedRun { run, glyphs }));
    }
    out
}

/// Place a row's shaped glyphs, and say which columns they covered.
///
/// Glyphs go at the cell their cluster came from rather than at the pen
/// position shaping would have used. That is what keeps a ligature inside the
/// columns its characters occupied, and keeps a font whose advances drift from
/// the cell width from pulling the row out of alignment.
#[allow(clippy::too_many_arguments)]
fn place_shaped_row(
    rows: &[(usize, ShapedRun)],
    cells: &[StyledCell],
    left_origin: f32,
    top: f32,
    metrics: CellMetrics,
    colors: FrameColors,
    pixel_size: u32,
    stack_id: u8,
    atlas: &GlyphAtlas,
    quads: &mut FrameQuads,
) -> std::collections::HashSet<usize> {
    let mut drawn = std::collections::HashSet::new();

    for (face, shaped) in rows {
        for glyph in &shaped.glyphs {
            let column = shaped.run.column_of(glyph.cluster as usize);
            let Some(slot) = atlas.get(GlyphKey {
                stack: stack_id,
                face: *face,
                glyph_index: glyph.glyph_index,
                pixel_size,
            }) else {
                // Not in the atlas: leave the column to the per-character
                // path rather than claiming it and drawing nothing.
                continue;
            };
            drawn.insert(column);
            if slot.width == 0 || slot.height == 0 {
                continue;
            }
            let cell = cells_at(cells, column);
            let (foreground, _) =
                unterm_render::quads::resolve_style(cell.map(|cell| &cell.style), colors);
            quads.glyphs.push(unterm_render::quads::glyph_quad(
                slot,
                left_origin + column as f32 * metrics.width + glyph.x_offset as f32,
                top + metrics.baseline - glyph.y_offset as f32,
                foreground,
                atlas,
            ));
        }
    }
    drawn
}

/// The cell at a column, counting wide cells as the columns they occupy.
fn cells_at(cells: &[StyledCell], column: usize) -> Option<&StyledCell> {
    let mut at = 0usize;
    for cell in cells {
        let width = cell.width.max(1);
        if column < at + width {
            return Some(cell);
        }
        at += width;
    }
    None
}

/// Put a shaped glyph in the atlas, by the index the shaper reported.
///
/// A shaped glyph has no character: a ligature is one glyph for several, and
/// a positional form is a different glyph for the same one. So it is filed by
/// face and index, which is what the key was always made of.
fn ensure_shaped_glyph(
    font: &mut TerminalFont,
    atlas: &mut GlyphAtlas,
    face: usize,
    glyph_index: u32,
) -> bool {
    let key = GlyphKey {
        stack: font.stack_id(),
        face,
        glyph_index,
        pixel_size: font.pixel_size(),
    };
    if atlas.get(key).is_some() {
        return true;
    }
    match font.stack_mut().rasterize_index(face, glyph_index) {
        Some(glyph) => {
            atlas.insert(key, &glyph);
            true
        }
        None => false,
    }
}

/// Where a character lives in the atlas.
///
/// The face is part of the key because two faces' glyphs for the same
/// character are different pictures; filing a fallback glyph under the primary
/// would show one where the other belongs.
fn glyph_key(font: &mut TerminalFont, ch: char) -> GlyphKey {
    let pixel_size = font.pixel_size();
    let stack = font.stack_id();
    let face = font.stack_mut().face_for(ch);
    GlyphKey {
        stack,
        face,
        // The face's own index for the character, not its code point. The
        // shaped path files glyphs by real index, and a code point standing
        // in for one collides with whatever glyph actually has that number:
        // the two entries overwrite each other and characters disappear from
        // the middle of a word.
        glyph_index: font
            .stack_mut()
            .glyph_index_for(face, ch)
            .unwrap_or_default(),
        pixel_size,
    }
}

/// Frame colours: the theme, with the config's own colours on top.
///
/// A theme is the whole scheme -- background, foreground and the sixteen
/// colours programs ask for -- and `colors.background` / `colors.foreground`
/// still win where they are set, because someone who wrote those meant them.
pub fn colors_from(config: &Config) -> FrameColors {
    use unterm_engine::next_core::color::parse_hex;

    // The config first, then whatever was last chosen in the picker, then the
    // default. One order, decided here, so the picker and the config file
    // cannot each believe they won.
    let theme = config
        .str_of("theme")
        .ok()
        .flatten()
        .and_then(|id| crate::theme::by_id(&id))
        .or_else(|| crate::theme::remembered().and_then(|id| crate::theme::by_id(&id)))
        .unwrap_or_else(crate::theme::default_theme);

    let from_config = |key: &str| {
        config
            .str_of(key)
            .ok()
            .flatten()
            .and_then(parse_hex)
            .map(|color| {
                [
                    color.red as f32 / 255.0,
                    color.green as f32 / 255.0,
                    color.blue as f32 / 255.0,
                    1.0,
                ]
            })
    };

    FrameColors {
        background: from_config("colors.background").unwrap_or(theme.background),
        foreground: from_config("colors.foreground").unwrap_or(theme.foreground),
        palette: &theme.ansi,
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
            mouse: Default::default(),
            bells: 0,
            focus_reporting: false,
            clipboard_request: None,
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
                font.grid_for(metrics.width * 80.0, metrics.height * rows as f32)
                    .1,
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
        let any_ink =
            (0..atlas.height()).any(|y| (0..atlas.width()).any(|x| atlas.pixel(x, y) > 0));
        assert!(any_ink, "the fallback face should have left ink");
    }

    fn snapshot_with_cursor(
        text: &str,
        x: usize,
        y: isize,
        visible: bool,
        shape: &str,
    ) -> StyledScreenSnapshot {
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
            .find(|glyph| (glyph.quad.left - font.metrics().width).abs() < font.metrics().width)
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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
            palette: &unterm_render::quads::DEFAULT_PALETTE,
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

#[cfg(test)]
mod shaped_row_tests {
    use super::*;
    use unterm_engine::{CellStyle, StyledCell};

    /// Shape a row and place it, as `append_pane` does in two passes.
    fn shape_and_place(
        cells: &[StyledCell],
        font: &mut TerminalFont,
        atlas: &mut GlyphAtlas,
        colors: FrameColors,
        quads: &mut FrameQuads,
    ) -> std::collections::HashSet<usize> {
        let rows = shape_row(cells, font);
        for (face, run) in &rows {
            for glyph in &run.glyphs {
                ensure_shaped_glyph(font, atlas, *face, glyph.glyph_index);
            }
        }
        let metrics = font.metrics();
        let pixel_size = font.pixel_size();
        place_shaped_row(
            &rows, cells, 0.0, 0.0, metrics, colors, pixel_size, 0, atlas, quads,
        )
    }

    fn cells(text: &str) -> Vec<StyledCell> {
        text.chars()
            .map(|ch| StyledCell {
                ch,
                style: CellStyle::default(),
                width: if ch.is_ascii() { 1 } else { 2 },
            })
            .collect()
    }

    /// The shaper draws the row, not the per-character fallback.
    ///
    /// Both paths produce readable text, so a silent fall back to the old one
    /// looks identical on screen -- and takes ligatures and every complex
    /// script with it. What is checked here is that shaping claimed the
    /// columns, which is the only externally visible difference.
    #[test]
    fn shaping_claims_the_columns_it_drew() {
        let Ok(mut font) = TerminalFont::open(16) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
            palette: &unterm_render::quads::DEFAULT_PALETTE,
        };
        let mut quads = FrameQuads::default();

        let drawn = shape_and_place(&cells("hello"), &mut font, &mut atlas, colors, &mut quads);

        assert_eq!(
            drawn.len(),
            5,
            "every column of a plain word should come from the shaper"
        );
        assert_eq!(quads.glyphs.len(), 5);
    }

    #[test]
    fn a_fallback_face_is_shaped_by_that_face() {
        let Ok(mut font) = TerminalFont::open(16) else {
            return;
        };
        if font.stack_mut().rasterize('中').is_none() {
            return;
        }
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
            palette: &unterm_render::quads::DEFAULT_PALETTE,
        };
        let mut quads = FrameQuads::default();

        let drawn = shape_and_place(&cells("中文"), &mut font, &mut atlas, colors, &mut quads);

        // Two characters, two columns apart -- a wide cell takes two.
        assert!(drawn.contains(&0), "first character drawn");
        assert!(drawn.contains(&2), "second character at column 2, not 1");
        assert_eq!(quads.glyphs.len(), 2);
    }
}

#[cfg(test)]
mod missing_glyph_regression {
    use super::*;
    use unterm_engine::{CellStyle, CursorSnapshot, StyledCell, StyledScreenLine};

    /// Every non-blank column of a line gets a glyph.
    ///
    /// "see https://example.com here" rendered as "see https://exampl  com
    /// here" -- two characters silently gone from the middle of a word. This
    /// is the whole pipeline, because each piece checked out on its own.
    #[test]
    fn every_column_of_a_line_is_drawn() {
        for pixel_size in [12, 13, 14, 16, 18, 20] {
            check_every_column_drawn(pixel_size);
        }
    }

    fn check_every_column_drawn(pixel_size: u32) {
        let Ok(mut font) = TerminalFont::open(pixel_size) else {
            return;
        };
        // Three rows, because the same text drew correctly on the first row
        // and lost two characters on the third.
        let rows = [
            "see https://example.com here",
            "aaa bbbbbbbbbbbbbbbbbbb ccc",
            "see https://exampleXcom here",
        ];
        let text = rows[2];
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
            palette: &unterm_render::quads::DEFAULT_PALETTE,
        };
        let mut quads = FrameQuads::default();

        let snapshot = StyledScreenSnapshot {
            lines: rows
                .iter()
                .enumerate()
                .map(|(index, line)| StyledScreenLine {
                    row: index as i64,
                    wrapped: false,
                    cells: line
                        .chars()
                        .map(|ch| StyledCell {
                            ch,
                            style: CellStyle::default(),
                            width: 1,
                        })
                        .collect(),
                })
                .collect(),
            cursor: CursorSnapshot {
                x: 0,
                y: 99,
                visible: false,
                shape: "Default".to_string(),
            },
            cols: text.len(),
            rows: rows.len(),
            scrollback_rows: 0,
            revision: 1,
            dirty_rows: None,
            mouse: Default::default(),
            bells: 0,
            focus_reporting: false,
            clipboard_request: None,
        };

        append_pane(
            &snapshot,
            &mut font,
            &mut atlas,
            colors,
            (0.0, 0.0),
            true,
            CursorStyle::default(),
            &mut quads,
        );

        let metrics = font.metrics();
        let last_row_top = 2.0 * metrics.height;
        let drawn: std::collections::HashSet<usize> = quads
            .glyphs
            .iter()
            .filter(|glyph| glyph.quad.top >= last_row_top - metrics.height * 0.5)
            .map(|glyph| (glyph.quad.left / metrics.width).round() as usize)
            .collect();
        let missing: Vec<(usize, char)> = text
            .chars()
            .enumerate()
            .filter(|(column, ch)| *ch != ' ' && !drawn.contains(column))
            .collect();
        assert!(
            missing.is_empty(),
            "at {pixel_size}px, columns with no glyph: {missing:?}"
        );
    }
}

#[cfg(test)]
mod cursor_inversion_tests {
    use super::*;
    use unterm_engine::{CellStyle, CursorSnapshot, StyledCell, StyledScreenLine};

    fn snapshot(rows: &[&str], cursor: (usize, isize)) -> StyledScreenSnapshot {
        StyledScreenSnapshot {
            lines: rows
                .iter()
                .enumerate()
                .map(|(index, line)| StyledScreenLine {
                    row: index as i64,
                    wrapped: false,
                    cells: line
                        .chars()
                        .map(|ch| StyledCell {
                            ch,
                            style: CellStyle::default(),
                            width: 1,
                        })
                        .collect(),
                })
                .collect(),
            cursor: CursorSnapshot {
                x: cursor.0,
                y: cursor.1,
                visible: true,
                shape: "Default".to_string(),
            },
            cols: rows.iter().map(|line| line.len()).max().unwrap_or(1),
            rows: rows.len(),
            scrollback_rows: 0,
            revision: 1,
            dirty_rows: None,
            mouse: Default::default(),
            bells: 0,
            focus_reporting: false,
            clipboard_request: None,
        }
    }

    /// The block cursor inverts its own cell and no other.
    ///
    /// It used to invert everything within one cell in each direction, which
    /// painted the row above the cursor in the background colour: characters
    /// sitting directly over the prompt disappeared, and only there. It looked
    /// like a shaping bug for a long time because the text was fine, the
    /// quads were fine, and the pixels were not.
    #[test]
    fn the_cursor_inverts_its_own_cell_and_no_other() {
        let Ok(mut font) = TerminalFont::open(16) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let colors = FrameColors {
            foreground: [1.0; 4],
            background: [0.0, 0.0, 0.0, 1.0],
            palette: &unterm_render::quads::DEFAULT_PALETTE,
        };
        let mut quads = FrameQuads::default();

        // Text on the row above, cursor at column 2 of the row below.
        let snapshot = snapshot(&["abcde", "abcde"], (2, 1));
        append_pane(
            &snapshot,
            &mut font,
            &mut atlas,
            colors,
            (0.0, 0.0),
            true,
            CursorStyle::default(),
            &mut quads,
        );

        let metrics = font.metrics();
        let above: Vec<_> = quads
            .glyphs
            .iter()
            .filter(|glyph| glyph.quad.top + glyph.quad.height < metrics.height * 1.5)
            .collect();
        assert_eq!(above.len(), 5, "the row above should be fully drawn");
        for glyph in above {
            assert_eq!(
                glyph.quad.color, colors.foreground,
                "a glyph on the row above the cursor was painted the background colour"
            );
        }

        let inverted = quads
            .glyphs
            .iter()
            .filter(|glyph| glyph.quad.color == colors.background)
            .count();
        assert_eq!(inverted, 1, "exactly the cursor's own cell");
    }
}

#[cfg(test)]
mod furniture_tests {
    use super::*;

    /// The front end's own text has to be looked up the way it was stored.
    ///
    /// These were two separate calculations, and they drifted: `ensure_glyph`
    /// filed by the face's real glyph index while `append_text` asked by code
    /// point. Every label, banner and bar then drew whichever glyph happened
    /// to carry that number -- a scattering of wrong letters where the status
    /// bar's path should have been. One key, worked out in one place.
    #[test]
    fn a_labels_glyphs_are_found_where_they_were_put() {
        let Ok(mut font) = TerminalFont::open(16) else {
            return; // No usable system font on this machine.
        };
        let mut atlas = GlyphAtlas::new(256, 256);

        let text = r"D:\code\unterm";
        for ch in text.chars() {
            ensure_glyph(&mut font, &mut atlas, ch);
        }
        for ch in text.chars().filter(|ch| *ch != ' ') {
            let key = glyph_key(&mut font, ch);
            assert!(
                atlas.get(key).is_some(),
                "{ch:?} was stored under a key nothing asks for"
            );
        }
    }

    /// And the text actually reaches the frame: a lookup that quietly misses
    /// produces no glyph and no error, which is exactly how this hid.
    #[test]
    fn a_label_draws_one_glyph_per_visible_character() {
        let Ok(mut font) = TerminalFont::open(16) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let mut quads = FrameQuads::default();

        append_text(
            "mcp:7",
            &mut font,
            &mut atlas,
            [1.0, 1.0, 1.0, 1.0],
            (0.0, 0.0),
            &mut quads,
        );
        assert_eq!(quads.glyphs.len(), 5, "one per character, none dropped");
    }

    /// Laid out left to right on the grid, so a bar's columns line up with
    /// the terminal's.
    #[test]
    fn a_labels_characters_advance_by_one_cell_each() {
        let Ok(mut font) = TerminalFont::open(16) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let mut quads = FrameQuads::default();
        let width = font.metrics().width;

        append_text(
            "abc",
            &mut font,
            &mut atlas,
            [1.0, 1.0, 1.0, 1.0],
            (0.0, 0.0),
            &mut quads,
        );
        let lefts: Vec<f32> = quads.glyphs.iter().map(|g| g.quad.left).collect();
        assert_eq!(lefts.len(), 3);
        for (index, left) in lefts.iter().enumerate() {
            let expected = index as f32 * width;
            assert!(
                (left - expected).abs() <= width,
                "character {index} sits at {left}, not near {expected}"
            );
        }
    }
}

#[cfg(test)]
mod width_tests {
    use super::*;

    /// A label in Chinese, Japanese or Korean is mostly double-width
    /// characters. Drawing each one cell wide puts the next character on top
    /// of the last, which is what the translated quick menu looked like.
    #[test]
    fn a_wide_character_takes_two_cells() {
        assert_eq!(column_width('中'), 2);
        assert_eq!(column_width('あ'), 2);
        assert_eq!(column_width('한'), 2);
    }

    #[test]
    fn latin_takes_one() {
        assert_eq!(column_width('a'), 1);
        assert_eq!(column_width(' '), 1);
        assert_eq!(column_width('→'), 1);
    }

    /// Never zero: a combining mark that advances nothing would stack the
    /// whole rest of the label in one cell.
    #[test]
    fn nothing_is_narrower_than_a_cell() {
        for ch in ['\u{0301}', '\u{200b}', '\u{0}'] {
            assert!(column_width(ch) >= 1, "{ch:?} advanced nothing");
        }
    }

    /// And the label lays out accordingly: two Latin characters after a wide
    /// one start three cells in, not two.
    #[test]
    fn a_label_after_a_wide_character_is_not_drawn_on_top_of_it() {
        let Ok(mut font) = TerminalFont::open(16) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(256, 256);
        let mut quads = FrameQuads::default();
        let width = font.metrics().width;

        append_text(
            "中a",
            &mut font,
            &mut atlas,
            [1.0; 4],
            (0.0, 0.0),
            &mut quads,
        );
        assert_eq!(quads.glyphs.len(), 2);
        let gap = quads.glyphs[1].quad.left - quads.glyphs[0].quad.left;
        assert!(
            gap >= width * 1.5,
            "the Latin character sits {gap} from the wide one, cell is {width}"
        );
    }
}

#[cfg(test)]
mod dpi_tests {
    use super::*;

    /// 13pt is 17px on an ordinary display -- not 13. Using the point size as
    /// pixels is a third smaller than asked for, and half on a scaled panel.
    #[test]
    fn points_become_more_pixels_than_points() {
        assert_eq!(pixels_for_points(13.0, 1.0), 17);
        assert_eq!(pixels_for_points(12.0, 1.0), 16);
    }

    /// And the scale multiplies. This is the one that was missing entirely.
    #[test]
    fn a_scaled_display_gets_proportionally_more_pixels() {
        assert_eq!(pixels_for_points(13.0, 2.0), 35);
        assert_eq!(
            pixels_for_points(13.0, 1.5),
            26,
            "a 1.5x panel at 13pt, which was being drawn at 13px"
        );
    }

    /// Never zero, whatever nonsense arrives: a face opened at zero pixels
    /// rasterizes nothing and the window is blank.
    #[test]
    fn nothing_rounds_away_to_no_font_at_all() {
        assert!(pixels_for_points(0.0, 1.0) > 0);
        assert!(pixels_for_points(13.0, 0.0) > 0);
        assert!(pixels_for_points(-5.0, -5.0) > 0);
    }
}

#[cfg(test)]
mod shape_tests {
    use super::*;

    fn face() -> Option<FontFace> {
        let index = font_discovery::FontIndex::scan();
        FontFace::open(&index.default_monospace()?.path, 20).ok()
    }

    /// The default is the font's own metrics, untouched.
    #[test]
    fn the_default_shape_changes_nothing() {
        let Some(plain) = face().map(|f| TerminalFont::from_face(f, &[], 20)) else {
            return;
        };
        let Some(shaped) =
            face().map(|f| TerminalFont::from_face_shaped(f, &[], 20, Shape::default()))
        else {
            return;
        };
        assert_eq!(plain.metrics(), shaped.metrics());
    }

    /// A taller line is taller, and no wider.
    #[test]
    fn line_height_opens_the_rows_without_widening_them() {
        let Some(plain) = face().map(|f| TerminalFont::from_face(f, &[], 20)) else {
            return;
        };
        let shape = Shape {
            line_height: 1.4,
            cell_width: 1.0,
        };
        let tall = TerminalFont::from_face_shaped(face().unwrap(), &[], 20, shape);
        assert!(tall.metrics().height > plain.metrics().height);
        assert_eq!(tall.metrics().width, plain.metrics().width);
    }

    /// And a wider cell is wider, and no taller.
    #[test]
    fn cell_width_opens_the_columns_without_heightening_them() {
        let Some(plain) = face().map(|f| TerminalFont::from_face(f, &[], 20)) else {
            return;
        };
        let shape = Shape {
            line_height: 1.0,
            cell_width: 1.3,
        };
        let wide = TerminalFont::from_face_shaped(face().unwrap(), &[], 20, shape);
        assert!(wide.metrics().width > plain.metrics().width);
        assert_eq!(wide.metrics().height, plain.metrics().height);
    }

    /// The text stays inside the taller cell rather than sitting on its old
    /// line with the gap all underneath.
    #[test]
    fn a_taller_line_keeps_its_text_off_the_bottom() {
        let Some(_) = face() else { return };
        let shape = Shape {
            line_height: 1.6,
            cell_width: 1.0,
        };
        let tall = TerminalFont::from_face_shaped(face().unwrap(), &[], 20, shape);
        let metrics = tall.metrics();
        assert!(metrics.baseline < metrics.height, "{metrics:?}");
        assert!(metrics.baseline > metrics.height * 0.4, "{metrics:?}");
    }
}

#[cfg(test)]
mod focus_cursor_tests {
    use super::*;
    use unterm_engine::{CellStyle, CursorSnapshot, StyledCell, StyledScreenLine};

    fn screen() -> StyledScreenSnapshot {
        StyledScreenSnapshot {
            lines: vec![StyledScreenLine {
                row: 0,
                wrapped: false,
                cells: "ab"
                    .chars()
                    .map(|ch| StyledCell {
                        ch,
                        style: CellStyle::default(),
                        width: 1,
                    })
                    .collect(),
            }],
            cursor: CursorSnapshot {
                x: 0,
                y: 0,
                visible: true,
                shape: "block".to_string(),
            },
            cols: 2,
            rows: 1,
            scrollback_rows: 0,
            revision: 1,
            dirty_rows: None,
            mouse: Default::default(),
            bells: 0,
            focus_reporting: false,
            clipboard_request: None,
        }
    }

    fn drawn(focused: bool) -> Option<FrameQuads> {
        let mut font = TerminalFont::open(16).ok()?;
        let mut atlas = GlyphAtlas::new(256, 256);
        let mut quads = FrameQuads::default();
        let snapshot = screen();
        append_pane(
            &snapshot,
            &mut font,
            &mut atlas,
            FrameColors {
                foreground: [1.0; 4],
                background: [0.0, 0.0, 0.0, 1.0],
                palette: &unterm_render::quads::DEFAULT_PALETTE,
            },
            (0.0, 0.0),
            focused,
            CursorStyle::default(),
            &mut quads,
        );
        Some(quads)
    }

    /// Which pane has the keyboard is said by the cursor, not by dimming the
    /// pane: dimming leaves a brightness step down the split seam and into the
    /// status bar, which is why the previous front end chose this instead.
    #[test]
    fn an_unfocused_pane_gets_the_outline_of_its_cursor() {
        let Some(focused) = drawn(true) else {
            return; // No usable system font on this machine.
        };
        let unfocused = drawn(false).unwrap();

        let solid = |quads: &FrameQuads| {
            quads
                .backgrounds
                .iter()
                .any(|quad| quad.width > 4.0 && quad.height > 8.0 && quad.color == [1.0; 4])
        };
        assert!(
            solid(&focused),
            "the focused pane's cursor is a solid block"
        );
        assert!(!solid(&unfocused), "the unfocused one's is not");
        assert!(
            unfocused.backgrounds.len() > 1,
            "but it is drawn: {:?}",
            unfocused.backgrounds
        );
    }

    /// And the character under it stays as it was: an outline does not
    /// invert, so an unfocused pane reads exactly as its text.
    #[test]
    fn an_outlined_cursor_does_not_invert_its_character() {
        let Some(focused) = drawn(true) else {
            return;
        };
        let unfocused = drawn(false).unwrap();
        let inverted = |quads: &FrameQuads| {
            quads
                .glyphs
                .iter()
                .any(|glyph| glyph.quad.color == [0.0, 0.0, 0.0, 1.0])
        };
        assert!(inverted(&focused), "the focused cursor inverts its cell");
        assert!(!inverted(&unfocused), "the outlined one leaves it alone");
    }
}

#[cfg(test)]
mod cursor_style_tests {
    use super::*;

    /// The config's own six names.
    #[test]
    fn every_named_style_parses_to_what_it_says() {
        let cases = [
            ("SteadyBlock", CursorShape::Block, false),
            ("BlinkingBlock", CursorShape::Block, true),
            ("SteadyUnderline", CursorShape::Underline, false),
            ("BlinkingUnderline", CursorShape::Underline, true),
            ("SteadyBar", CursorShape::Bar, false),
            ("BlinkingBar", CursorShape::Bar, true),
        ];
        for (name, shape, blinking) in cases {
            let style = CursorStyle::parse(name);
            assert_eq!(style.shape, shape, "{name}");
            assert_eq!(style.blinking, blinking, "{name}");
        }
    }

    /// Case is not the point: a config written by hand says `steadybar` as
    /// often as `SteadyBar`.
    #[test]
    fn the_names_are_not_case_sensitive() {
        assert_eq!(CursorStyle::parse("steadybar").shape, CursorShape::Bar);
        assert_eq!(CursorStyle::parse("BLINKINGBAR").blinking, true);
    }

    /// A typo is a block cursor, not a refusal to open: nobody is surprised by
    /// a block, and a terminal that will not start is no way to report one.
    #[test]
    fn an_unknown_name_is_the_ordinary_cursor() {
        assert_eq!(CursorStyle::parse("wobbly"), CursorStyle::default());
        assert_eq!(CursorStyle::parse(""), CursorStyle::default());
        assert_eq!(CursorStyle::default().shape, CursorShape::Block);
    }

    /// Half the period on, half off.
    #[test]
    fn a_blink_is_on_for_half_its_period() {
        assert!(blink_is_on(0, 800));
        assert!(blink_is_on(799, 800));
        assert!(!blink_is_on(800, 800));
        assert!(!blink_is_on(1599, 800));
        assert!(blink_is_on(1600, 800));
    }

    /// A rate of zero is the setting turned off, not a blink of no length --
    /// which would divide by nothing and flicker every frame.
    #[test]
    fn a_rate_of_zero_leaves_the_cursor_alone() {
        for elapsed in [0, 1, 999_999] {
            assert!(blink_is_on(elapsed, 0), "at {elapsed}");
        }
    }
}

/// Draw chrome text at the font's own advances, and say how wide it came out.
///
/// Not on the terminal's grid. Chrome is drawn in a proportional face, and
/// placing each character on a fixed cell spreads a word out into `u n t e r m`
/// -- which is exactly what the sidebar looked like the first time it was drawn
/// this way. Every glyph goes where its own advance puts it.
///
/// `top` is the top of the row; the baseline is taken from the font, so a row
/// of text and a row of icons sit on the same line.
pub fn append_chrome_text(
    text: &str,
    font: &mut TerminalFont,
    atlas: &mut GlyphAtlas,
    color: [f32; 4],
    origin: (f32, f32),
    quads: &mut FrameQuads,
) -> f32 {
    for ch in text.chars() {
        ensure_glyph(font, atlas, ch);
    }
    let baseline = font.metrics().baseline;
    let mut pen = origin.0;
    for ch in text.chars() {
        if ch == ' ' {
            pen += space_advance(font);
            continue;
        }
        let key = glyph_key(font, ch);
        let Some(slot) = atlas.get(key) else {
            pen += space_advance(font);
            continue;
        };
        if slot.width > 0 && slot.height > 0 {
            quads.glyphs.push(unterm_render::quads::glyph_quad(
                slot,
                pen,
                origin.1 + baseline,
                color,
                atlas,
            ));
        }
        pen += slot.advance_x as f32;
    }
    pen - origin.0
}

/// How wide chrome text will be, without drawing it.
///
/// Measured the same way it is drawn, so a label that is measured as fitting
/// fits. Anything measured on the terminal's grid instead is wrong for a
/// proportional face, in whichever direction the face happens to differ.
pub fn chrome_text_width(text: &str, font: &mut TerminalFont, atlas: &mut GlyphAtlas) -> f32 {
    for ch in text.chars() {
        ensure_glyph(font, atlas, ch);
    }
    let mut width = 0.0;
    for ch in text.chars() {
        if ch == ' ' {
            width += space_advance(font);
            continue;
        }
        width += match atlas.get(glyph_key(font, ch)) {
            Some(slot) => slot.advance_x as f32,
            None => space_advance(font),
        };
    }
    width
}

/// A space's advance in this face, falling back to a third of the size for a
/// face that reports none.
fn space_advance(font: &mut TerminalFont) -> f32 {
    match font.stack_mut().rasterize(' ') {
        Some((_, glyph)) if glyph.advance_x > 0 => glyph.advance_x as f32,
        _ => font.pixel_size() as f32 * 0.32,
    }
}

#[cfg(test)]
mod chrome_text_tests {
    use super::*;

    /// Chrome text is placed at the face's own advances. On a fixed cell grid a
    /// proportional word comes out as `u n t e r m`, which is what the sidebar
    /// looked like the first time it was drawn that way.
    #[test]
    fn a_narrow_letter_takes_less_room_than_a_wide_one() {
        let Ok(mut font) = crate::chrome_font::open(&[], 1.0) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(512, 512);
        let narrow = chrome_text_width("iiii", &mut font, &mut atlas);
        let wide = chrome_text_width("WWWW", &mut font, &mut atlas);
        assert!(
            wide > narrow,
            "four W came out no wider than four i: {wide} vs {narrow}"
        );
    }

    /// And what is measured is what is drawn, or a label that measures as
    /// fitting runs off the end of its row.
    #[test]
    fn what_is_measured_is_what_is_drawn() {
        let Ok(mut font) = crate::chrome_font::open(&[], 1.0) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(512, 512);
        let mut quads = FrameQuads::default();
        let measured = chrome_text_width("unterm", &mut font, &mut atlas);
        let drawn = append_chrome_text(
            "unterm",
            &mut font,
            &mut atlas,
            [1.0; 4],
            (0.0, 0.0),
            &mut quads,
        );
        assert!((measured - drawn).abs() < 0.5, "{measured} then {drawn}");
        assert!(!quads.glyphs.is_empty(), "nothing was drawn");
    }

    /// Every glyph lands inside the run it was asked for, left to right.
    #[test]
    fn glyphs_run_left_to_right_from_the_origin() {
        let Ok(mut font) = crate::chrome_font::open(&[], 1.0) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(512, 512);
        let mut quads = FrameQuads::default();
        let width = append_chrome_text(
            "abcdef",
            &mut font,
            &mut atlas,
            [1.0; 4],
            (100.0, 50.0),
            &mut quads,
        );
        let mut previous = 0.0f32;
        for glyph in &quads.glyphs {
            assert!(
                glyph.quad.left >= 99.0,
                "{:?} is before the origin",
                glyph.quad
            );
            assert!(
                glyph.quad.left <= 100.0 + width + 2.0,
                "{:?} is past the run",
                glyph.quad
            );
            assert!(
                glyph.quad.left >= previous - 1.0,
                "the glyphs went backwards"
            );
            previous = glyph.quad.left;
        }
    }

    /// A space moves the pen without drawing anything.
    #[test]
    fn a_space_advances_without_drawing() {
        let Ok(mut font) = crate::chrome_font::open(&[], 1.0) else {
            return;
        };
        let mut atlas = GlyphAtlas::new(512, 512);
        let mut quads = FrameQuads::default();
        let width =
            append_chrome_text(" ", &mut font, &mut atlas, [1.0; 4], (0.0, 0.0), &mut quads);
        assert!(width > 0.0, "a space took no room");
        assert!(quads.glyphs.is_empty(), "a space drew something");
    }
}
