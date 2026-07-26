//! Scrolling ("long") screenshots, both directions:
//!
//! 1. [`render_scrollback_png`] — INSIDE the terminal. We own the text model
//!    and the font stack, so a "scrolling screenshot" of a pane never needs
//!    pixel capture at all: we re-render the entire scrollback headlessly
//!    with the same fonts the GUI uses (shape each attribute cluster,
//!    rasterize glyphs CPU-side, composite per text row) and stream the
//!    rows straight into a PNG encoder so memory stays bounded no matter
//!    how long the history is. Works even when the window is occluded,
//!    minimized, or on another Space.
//!
//! 2. [`external::scroll_capture_window`] (macOS) — OUTSIDE the terminal.
//!    Long-screenshot any other app's window: synthesize wheel events at the
//!    window's center, capture each frame with `screencapture -l`, and
//!    stitch frames by exact row-hash matching (terminals get math, other
//!    apps get signal processing). Fixed chrome (title bars, sticky
//!    headers/footers) is detected from the first frame pair and excluded
//!    from matching, then re-attached once at the seams.

use anyhow::{anyhow, Context, Result};
use config::ConfigHandle;
use mux::pane::Pane;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use termwiz::cell::Underline;
use termwiz::color::ColorAttribute;
use unterm_engine::{StyledColor, StyledScreenLine};
use wezterm_font::shaper::{Direction, PresentationWidth};
use wezterm_font::{FontConfiguration, LoadedFont, RasterizedGlyph};
use wezterm_term::color::ColorPalette;
use wezterm_term::TerminalConfiguration;

pub struct ScrollbackPngOptions {
    /// Cap on history rows rendered. When the scrollback is longer we keep
    /// the TAIL (most recent rows) — that is what a human reaching for a
    /// long screenshot wants.
    pub max_rows: usize,
    /// Raster dpi. 144 ≈ retina-quality; 72 = compact.
    pub dpi: usize,
}

impl Default for ScrollbackPngOptions {
    fn default() -> Self {
        Self {
            max_rows: 10_000,
            dpi: if cfg!(target_os = "macos") { 144 } else { 96 },
        }
    }
}

pub struct ScrollbackPng {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub rows: usize,
    pub cols: usize,
    pub truncated: bool,
    pub first_row: isize,
}

/// Render engine-owned plain-text scrollback to PNG. This is the `next-core`
/// bridge while styled cell parity is still evolving: it uses the same font
/// stack and PNG streaming path as the pane renderer, but applies the default
/// terminal colors to every cell.
#[allow(dead_code)]
pub fn render_plain_scrollback_png(
    lines: &[String],
    cols: usize,
    first_row: i64,
    truncated: bool,
    out_path: &Path,
    opts: &ScrollbackPngOptions,
) -> Result<ScrollbackPng> {
    let rows = lines.len().max(1);
    let text_cols = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let cols = cols.max(text_cols).max(1);

    let fonts = Rc::new(FontConfiguration::new(None, opts.dpi)?);
    let font = fonts.default_font()?;
    let metrics = fonts.default_font_metrics()?;
    let cell_w = metrics.cell_width.get();
    let cell_h = metrics.cell_height.get().ceil().max(1.0);
    let baseline = metrics.cell_height.get() + metrics.descender.get();

    let width_px = (cols as f64 * cell_w).ceil() as u32;
    let height_px = rows as u32 * cell_h as u32;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let file = std::fs::File::create(out_path)
        .with_context(|| format!("create {}", out_path.display()))?;
    let bufw = std::io::BufWriter::new(file);
    let mut enc = png::Encoder::new(bufw, width_px, height_px);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().context("write png header")?;
    let mut stream = writer.stream_writer().context("png stream writer")?;

    let mut band = BandCanvas::new(width_px as usize, cell_h as usize);
    let mut raster_cache: HashMap<(usize, usize, u32), Rc<RasterizedGlyph>> = HashMap::new();
    let font_key = Rc::as_ptr(&font) as usize;
    let default_bg = (0x0b, 0x10, 0x1d);
    let default_fg = (0xd8, 0xde, 0xe9);

    for idx in 0..rows {
        band.fill(default_bg);
        let line = lines.get(idx).map(String::as_str).unwrap_or("");
        let line = line.replace('\t', "    ");
        let glyphs = match font.blocking_shape(&line, None, Direction::LeftToRight, None, None) {
            Ok(glyphs) => glyphs,
            Err(_) => Vec::new(),
        };
        let mut pen_x = 0.0;
        for info in &glyphs {
            if !info.is_space {
                let key = (font_key, info.font_idx, info.glyph_pos);
                let glyph = match raster_cache.get(&key) {
                    Some(glyph) => Rc::clone(glyph),
                    None => {
                        let glyph = Rc::new(font.rasterize_glyph(info.glyph_pos, info.font_idx)?);
                        raster_cache.insert(key, Rc::clone(&glyph));
                        glyph
                    }
                };
                if glyph.width > 0 && glyph.height > 0 {
                    let x0 = pen_x + info.x_offset.get() + glyph.bearing_x.get();
                    let y0 = baseline - info.y_offset.get() - glyph.bearing_y.get();
                    band.blit_glyph(&glyph, x0, y0, default_fg, 1.0);
                }
            }
            pen_x += info.x_advance.get();
            if pen_x > width_px as f64 {
                break;
            }
        }

        use std::io::Write as _;
        stream.write_all(&band.data).context("write png band")?;
    }

    stream.finish().context("finish png stream")?;

    Ok(ScrollbackPng {
        path: out_path.to_path_buf(),
        width: width_px,
        height: height_px,
        rows,
        cols,
        truncated,
        first_row: first_row as isize,
    })
}

/// Render engine-owned styled scrollback to PNG. This keeps `next-core`
/// capture independent from WezTerm panes while preserving cell-level colors
/// and basic decorations from the next-core screen buffer.
pub fn render_styled_scrollback_png(
    lines: &[StyledScreenLine],
    cols: usize,
    first_row: i64,
    truncated: bool,
    out_path: &Path,
    opts: &ScrollbackPngOptions,
) -> Result<ScrollbackPng> {
    let rows = lines.len().max(1);
    let text_cols = lines
        .iter()
        .map(|line| line.cells.iter().map(|cell| cell.width).sum::<usize>())
        .max()
        .unwrap_or(0);
    let cols = cols.max(text_cols).max(1);

    let fonts = Rc::new(FontConfiguration::new(None, opts.dpi)?);
    let font = fonts.default_font()?;
    let metrics = fonts.default_font_metrics()?;
    let cell_w = metrics.cell_width.get();
    let cell_h = metrics.cell_height.get().ceil().max(1.0);
    let baseline = metrics.cell_height.get() + metrics.descender.get();

    let width_px = (cols as f64 * cell_w).ceil() as u32;
    let height_px = rows as u32 * cell_h as u32;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let file = std::fs::File::create(out_path)
        .with_context(|| format!("create {}", out_path.display()))?;
    let bufw = std::io::BufWriter::new(file);
    let mut enc = png::Encoder::new(bufw, width_px, height_px);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().context("write png header")?;
    let mut stream = writer.stream_writer().context("png stream writer")?;

    let mut band = BandCanvas::new(width_px as usize, cell_h as usize);
    let mut raster_cache: HashMap<(usize, usize, u32), Rc<RasterizedGlyph>> = HashMap::new();
    let font_key = Rc::as_ptr(&font) as usize;
    let config: ConfigHandle = config::configuration();
    let palette = config::TermConfig::with_config(config).color_palette();
    let default_bg = srgb8(palette.resolve_bg(ColorAttribute::Default));
    let default_fg = srgb8(palette.resolve_fg(ColorAttribute::Default));

    for idx in 0..rows {
        band.fill(default_bg);
        let mut cell_x = 0usize;
        if let Some(line) = lines.get(idx) {
            for cell in &line.cells {
                let width = cell.width.max(1);
                let mut fg = resolve_styled_color(cell.style.fg, default_fg, &palette);
                let mut bg = resolve_styled_color(cell.style.bg, default_bg, &palette);
                if cell.style.inverse {
                    std::mem::swap(&mut fg, &mut bg);
                }

                let cx0 = (cell_x as f64 * cell_w).round() as isize;
                let cw = (width as f64 * cell_w).ceil() as isize;
                if cell.style.bg.is_some() || cell.style.inverse {
                    band.fill_rect(cx0, 0, cw, cell_h as isize, bg);
                }

                if !cell.ch.is_whitespace() {
                    let text = cell.ch.to_string();
                    let glyphs = match font.blocking_shape(
                        &text,
                        None,
                        Direction::LeftToRight,
                        None,
                        None,
                    ) {
                        Ok(glyphs) => glyphs,
                        Err(_) => Vec::new(),
                    };
                    for info in &glyphs {
                        if info.is_space {
                            continue;
                        }
                        let key = (font_key, info.font_idx, info.glyph_pos);
                        let glyph = match raster_cache.get(&key) {
                            Some(glyph) => Rc::clone(glyph),
                            None => {
                                let glyph =
                                    Rc::new(font.rasterize_glyph(info.glyph_pos, info.font_idx)?);
                                raster_cache.insert(key, Rc::clone(&glyph));
                                glyph
                            }
                        };
                        if glyph.width == 0 || glyph.height == 0 {
                            continue;
                        }
                        let scale = if glyph.is_scaled {
                            1.0
                        } else {
                            let max_w = width as f64 * cell_w;
                            (max_w / glyph.width as f64)
                                .min(cell_h / glyph.height as f64)
                                .min(1.0)
                        };
                        let x0 = cell_x as f64 * cell_w
                            + info.x_offset.get()
                            + glyph.bearing_x.get() * scale;
                        let y0 = baseline - info.y_offset.get() - glyph.bearing_y.get() * scale;
                        band.blit_glyph(&glyph, x0, y0, fg, scale);
                    }
                }

                if cell.style.underline {
                    let thickness = metrics.underline_thickness.get().round().max(1.0) as isize;
                    let uy = (baseline - metrics.underline_position.get()).round() as isize;
                    band.fill_rect(cx0, uy.min(cell_h as isize - thickness), cw, thickness, fg);
                }

                cell_x += cell.width;
            }
        }

        use std::io::Write as _;
        stream.write_all(&band.data).context("write png band")?;
    }

    stream.finish().context("finish png stream")?;

    Ok(ScrollbackPng {
        path: out_path.to_path_buf(),
        width: width_px,
        height: height_px,
        rows,
        cols,
        truncated,
        first_row: first_row as isize,
    })
}

fn resolve_styled_color(
    color: Option<StyledColor>,
    default: (u8, u8, u8),
    palette: &ColorPalette,
) -> (u8, u8, u8) {
    match color {
        Some(StyledColor::Rgb(r, g, b)) => (r, g, b),
        Some(StyledColor::Palette(idx)) => {
            srgb8(palette.resolve_fg(ColorAttribute::PaletteIndex(idx)))
        }
        None => default,
    }
}

/// sRGB u8 -> linear f32 lookup, built once. Gamma-correct text blending is
/// what separates "screenshot" from "thin gray smudge" on dark themes.
fn srgb_to_linear_lut() -> [f32; 256] {
    let mut lut = [0f32; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        let c = i as f32 / 255.0;
        *v = if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        };
    }
    lut
}

fn linear_to_srgb8(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let c = if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0 + 0.5) as u8
}

struct BandCanvas {
    width: usize,
    height: usize,
    /// RGBA8, row-major.
    data: Vec<u8>,
    lut: [f32; 256],
}

impl BandCanvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0u8; width * height * 4],
            lut: srgb_to_linear_lut(),
        }
    }

    fn fill(&mut self, rgb: (u8, u8, u8)) {
        for px in self.data.chunks_exact_mut(4) {
            px[0] = rgb.0;
            px[1] = rgb.1;
            px[2] = rgb.2;
            px[3] = 0xff;
        }
    }

    fn fill_rect(&mut self, x0: isize, y0: isize, w: isize, h: isize, rgb: (u8, u8, u8)) {
        let x_start = x0.max(0) as usize;
        let y_start = y0.max(0) as usize;
        let x_end = ((x0 + w).max(0) as usize).min(self.width);
        let y_end = ((y0 + h).max(0) as usize).min(self.height);
        for y in y_start..y_end {
            let row = y * self.width * 4;
            for x in x_start..x_end {
                let o = row + x * 4;
                self.data[o] = rgb.0;
                self.data[o + 1] = rgb.1;
                self.data[o + 2] = rgb.2;
                self.data[o + 3] = 0xff;
            }
        }
    }

    /// Composite one rasterized glyph at (x0, y0) (top-left of the bitmap).
    /// Monochrome glyphs are tinted with `fg` using linear-light blending
    /// driven by the rasterizer's coverage; color glyphs (emoji) composite
    /// premultiplied-over.
    fn blit_glyph(&mut self, g: &RasterizedGlyph, x0: f64, y0: f64, fg: (u8, u8, u8), scale: f64) {
        let fg_lin = [
            self.lut[fg.0 as usize],
            self.lut[fg.1 as usize],
            self.lut[fg.2 as usize],
        ];
        let out_w = (g.width as f64 * scale).round().max(1.0) as usize;
        let out_h = (g.height as f64 * scale).round().max(1.0) as usize;
        for oy in 0..out_h {
            let dy = y0 as isize + oy as isize;
            if dy < 0 || dy as usize >= self.height {
                continue;
            }
            // nearest-row sample when scaling (scale==1.0 is the hot path)
            let sy = if scale == 1.0 {
                oy
            } else {
                ((oy as f64 / scale) as usize).min(g.height - 1)
            };
            for ox in 0..out_w {
                let dx = x0 as isize + ox as isize;
                if dx < 0 || dx as usize >= self.width {
                    continue;
                }
                let sx = if scale == 1.0 {
                    ox
                } else {
                    ((ox as f64 / scale) as usize).min(g.width - 1)
                };
                let so = (sy * g.width + sx) * 4;
                let a = g.data[so + 3] as f32 / 255.0;
                if a <= 0.0 {
                    continue;
                }
                let dst = (dy as usize * self.width + dx as usize) * 4;
                if g.has_color {
                    // premultiplied source over opaque dst, srgb-approx
                    for c in 0..3 {
                        let s = g.data[so + c] as f32 / 255.0;
                        let d = self.data[dst + c] as f32 / 255.0;
                        self.data[dst + c] = ((s + d * (1.0 - a)) * 255.0 + 0.5) as u8;
                    }
                } else {
                    for c in 0..3 {
                        let d = self.lut[self.data[dst + c] as usize];
                        let v = fg_lin[c] * a + d * (1.0 - a);
                        self.data[dst + c] = linear_to_srgb8(v);
                    }
                }
                self.data[dst + 3] = 0xff;
            }
        }
    }
}

fn srgb8(t: termwiz::color::SrgbaTuple) -> (u8, u8, u8) {
    let (r, g, b, _a) = t.to_srgb_u8();
    (r, g, b)
}

/// Render the pane's full scrollback (plus the live viewport) to `out_path`
/// as one tall PNG. Safe to call from any thread; does not touch the GUI.
pub fn render_scrollback_png(
    pane: &Arc<dyn Pane>,
    out_path: &Path,
    opts: &ScrollbackPngOptions,
) -> Result<ScrollbackPng> {
    let dims = pane.get_dimensions();
    let viewport_bottom = dims.physical_top + dims.viewport_rows as isize;
    let total_rows = (viewport_bottom - dims.scrollback_top).max(0) as usize;
    let truncated = total_rows > opts.max_rows;
    let start = if truncated {
        viewport_bottom - opts.max_rows as isize
    } else {
        dims.scrollback_top
    };
    let (first_row, lines) = pane.get_lines(start..viewport_bottom);
    let rows = lines.len();
    if rows == 0 {
        return Err(anyhow!("pane has no content to render"));
    }

    let config: ConfigHandle = config::configuration();
    let fonts = Rc::new(FontConfiguration::new(None, opts.dpi)?);
    let metrics = fonts.default_font_metrics()?;
    let cell_w = metrics.cell_width.get();
    let cell_h = metrics.cell_height.get().ceil().max(1.0);
    let baseline = metrics.cell_height.get() + metrics.descender.get();

    let palette = pane.palette();
    let default_bg = srgb8(palette.resolve_bg(termwiz::color::ColorAttribute::Default));

    let width_px = (dims.cols as f64 * cell_w).ceil() as u32;
    let height_px = rows as u32 * cell_h as u32;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let file = std::fs::File::create(out_path)
        .with_context(|| format!("create {}", out_path.display()))?;
    let bufw = std::io::BufWriter::new(file);
    let mut enc = png::Encoder::new(bufw, width_px, height_px);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().context("write png header")?;
    let mut stream = writer.stream_writer().context("png stream writer")?;

    let mut band = BandCanvas::new(width_px as usize, cell_h as usize);
    // (font Rc ptr, font_idx, glyph_pos) -> rasterized glyph
    let mut raster_cache: HashMap<(usize, usize, u32), Rc<RasterizedGlyph>> = HashMap::new();

    for line in &lines {
        band.fill(default_bg);

        for cluster in line.cluster(None) {
            let attrs = &cluster.attrs;
            let mut fg = srgb8(palette.resolve_fg(attrs.foreground()));
            let mut bg = srgb8(palette.resolve_bg(attrs.background()));
            if attrs.reverse() {
                std::mem::swap(&mut fg, &mut bg);
            }

            let cx0 = (cluster.first_cell_idx as f64 * cell_w).round() as isize;
            let cw = (cluster.width as f64 * cell_w).ceil() as isize;

            let needs_bg_fill = attrs.reverse()
                || !matches!(attrs.background(), termwiz::color::ColorAttribute::Default);
            if needs_bg_fill {
                band.fill_rect(cx0, 0, cw, cell_h as isize, bg);
            }

            if !cluster.text.trim().is_empty() {
                let style = fonts.match_style(&config, attrs);
                let font: Rc<LoadedFont> = fonts.resolve_font(style)?;
                let font_key = Rc::as_ptr(&font) as usize;
                let pw = PresentationWidth::with_cluster(&cluster);
                let shape_once = || {
                    font.shape(
                        &cluster.text,
                        || {},
                        |_| {},
                        Some(cluster.presentation),
                        cluster.direction,
                        None,
                        Some(&pw),
                    )
                };
                // A freshly resolved fallback font makes the first shape call
                // return ClearShapeCache; retrying immediately succeeds.
                let infos = match shape_once() {
                    Ok(infos) => infos,
                    Err(_) => match shape_once() {
                        Ok(infos) => infos,
                        Err(_) => continue,
                    },
                };
                for info in &infos {
                    if info.is_space {
                        continue;
                    }
                    let cell_idx = cluster.byte_to_cell_idx(info.cluster as usize);
                    let num_cells = cluster.byte_to_cell_width(info.cluster as usize).max(1);
                    let key = (font_key, info.font_idx, info.glyph_pos);
                    let g = match raster_cache.get(&key) {
                        Some(g) => Rc::clone(g),
                        None => {
                            let g = Rc::new(
                                font.rasterize_glyph(info.glyph_pos, info.font_idx)
                                    .unwrap_or(RasterizedGlyph {
                                        data: vec![],
                                        height: 0,
                                        width: 0,
                                        bearing_x: wezterm_font::units::PixelLength::new(0.),
                                        bearing_y: wezterm_font::units::PixelLength::new(0.),
                                        has_color: false,
                                        is_scaled: true,
                                    }),
                            );
                            raster_cache.insert(key, Rc::clone(&g));
                            g
                        }
                    };
                    if g.width == 0 || g.height == 0 {
                        continue;
                    }
                    // Unscaled bitmaps (color-emoji strikes, odd fallback
                    // fonts) get fitted into their cell span.
                    let scale = if g.is_scaled {
                        1.0
                    } else {
                        let max_w = num_cells as f64 * cell_w;
                        (max_w / g.width as f64)
                            .min(cell_h / g.height as f64)
                            .min(1.0)
                    };
                    let x0 =
                        cell_idx as f64 * cell_w + info.x_offset.get() + g.bearing_x.get() * scale;
                    let y0 = baseline - info.y_offset.get() - g.bearing_y.get() * scale;
                    band.blit_glyph(&g, x0, y0, fg, scale);
                }
            }

            if attrs.underline() != Underline::None {
                let thickness = metrics.underline_thickness.get().round().max(1.0) as isize;
                let uy = (baseline - metrics.underline_position.get()).round() as isize;
                band.fill_rect(cx0, uy.min(cell_h as isize - thickness), cw, thickness, fg);
            }
            if attrs.strikethrough() {
                let thickness = metrics.underline_thickness.get().round().max(1.0) as isize;
                let sy = (baseline * 0.65).round() as isize;
                band.fill_rect(cx0, sy, cw, thickness, fg);
            }
        }

        use std::io::Write as _;
        stream.write_all(&band.data).context("write png band")?;
    }

    stream.finish().context("finish png stream")?;

    Ok(ScrollbackPng {
        path: out_path.to_path_buf(),
        width: width_px,
        height: height_px,
        rows,
        cols: dims.cols,
        truncated,
        first_row,
    })
}

/// Default output directory shared with the other capture surfaces.
pub fn scrollshot_output_dir() -> Result<PathBuf> {
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("screenshots");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

// ---------------------------------------------------------------------------
// External (other-app) scrolling capture — macOS
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
pub mod external {
    use super::*;
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::display::{
        kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
        CGDisplay, CGWindowListCopyWindowInfo,
    };
    use core_graphics::event::{CGEvent, CGEventTapLocation, ScrollEventUnit};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(
            options: core_foundation::dictionary::CFDictionaryRef,
        ) -> bool;
        static kAXTrustedCheckOptionPrompt: core_foundation::string::CFStringRef;
    }

    /// Check (and, on first failure, actively request) the two macOS
    /// permissions scroll-capture needs. `screencapture` spawned as a child
    /// process fails *silently* without Screen Recording — it never raises
    /// the TCC prompt itself — so we must preflight via the framework calls,
    /// which both prompt and register Unterm in System Settings.
    pub fn ensure_permissions() -> Result<()> {
        let screen_ok = unsafe { CGPreflightScreenCaptureAccess() };
        if !screen_ok {
            unsafe {
                CGRequestScreenCaptureAccess();
            }
            return Err(anyhow!(
                "Unterm needs the macOS Screen Recording permission to capture other \
                 windows. A system prompt was just raised — approve it (System Settings \
                 → Privacy & Security → Screen Recording → Unterm), then retry. \
                 macOS requires an app restart after granting."
            ));
        }
        let ax_ok = unsafe {
            use core_foundation::base::TCFType;
            use core_foundation::boolean::CFBoolean;
            use core_foundation::dictionary::CFDictionary;
            use core_foundation::string::CFString;
            let key: CFString = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
            let opts = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
            AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef())
        };
        if !ax_ok {
            return Err(anyhow!(
                "Unterm needs the macOS Accessibility permission to synthesize scroll \
                 events. A system prompt was just raised — approve it (System Settings \
                 → Privacy & Security → Accessibility → Unterm), then retry."
            ));
        }
        Ok(())
    }

    /// Resolve a pid to its executable name (`ps -o comm=`), because
    /// kCGWindowOwnerName is *localized* (e.g. TextEdit is 文本编辑 on a
    /// Chinese system) and users/agents match on the English name.
    fn process_name(pid: u32) -> String {
        std::process::Command::new("/bin/ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .ok()
            .map(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                s.rsplit('/').next().unwrap_or(&s).to_string()
            })
            .unwrap_or_default()
    }

    #[derive(Debug, Clone)]
    pub struct TargetWindow {
        pub window_id: u32,
        pub pid: u32,
        pub app: String,
        pub title: String,
        /// Window bounds in screen points.
        pub x: f64,
        pub y: f64,
        pub w: f64,
        pub h: f64,
    }

    pub struct ScrollCaptureOptions {
        pub max_frames: usize,
        /// Fraction of the window height scrolled between frames.
        pub step_frac: f64,
        /// Delay after each synthetic scroll before capturing.
        pub settle_ms: u64,
        /// Raise the target window first so wheel events reach it.
        pub activate: bool,
        /// Best-effort scroll back to the top position afterwards.
        pub restore_scroll: bool,
    }

    impl Default for ScrollCaptureOptions {
        fn default() -> Self {
            Self {
                max_frames: 25,
                step_frac: 0.6,
                settle_ms: 350,
                activate: true,
                restore_scroll: true,
            }
        }
    }

    pub struct ScrollCaptureResult {
        pub path: PathBuf,
        pub width: u32,
        pub height: u32,
        pub frames: usize,
        pub window: TargetWindow,
        pub hint: Option<String>,
    }

    fn dict_f64(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<f64> {
        dict.find(&CFString::new(key))
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_f64())
    }

    fn list_windows() -> Result<Vec<TargetWindow>> {
        let info: CFArray<CFDictionary<CFString, CFType>> = unsafe {
            let raw = CGWindowListCopyWindowInfo(
                kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
                kCGNullWindowID,
            );
            if raw.is_null() {
                return Ok(vec![]);
            }
            CFArray::wrap_under_create_rule(raw)
        };

        let pid_key = CFString::from_static_string("kCGWindowOwnerPID");
        let id_key = CFString::from_static_string("kCGWindowNumber");
        let name_key = CFString::from_static_string("kCGWindowName");
        let owner_key = CFString::from_static_string("kCGWindowOwnerName");
        let layer_key = CFString::from_static_string("kCGWindowLayer");
        let bounds_key = CFString::from_static_string("kCGWindowBounds");

        let mut out = vec![];
        for entry in info.iter() {
            let layer: i64 = entry
                .find(&layer_key)
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i64())
                .unwrap_or(-1);
            // layer 0 = ordinary app windows; everything else is chrome
            // (menubar, dock, overlays) that can't be scroll-captured.
            if layer != 0 {
                continue;
            }
            let pid = entry
                .find(&pid_key)
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i64())
                .unwrap_or(-1);
            let window_id = entry
                .find(&id_key)
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i64())
                .unwrap_or(0);
            if pid <= 0 || window_id <= 0 {
                continue;
            }
            let title = entry
                .find(&name_key)
                .and_then(|v| v.downcast::<CFString>())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let app = entry
                .find(&owner_key)
                .and_then(|v| v.downcast::<CFString>())
                .map(|s| s.to_string())
                .unwrap_or_default();
            // kCGWindowBounds is a CFDictionary {X, Y, Width, Height}; the
            // typed downcast path needs ConcreteCFType which CFDictionary
            // doesn't implement, so type-check + re-wrap manually.
            let Some(bounds) = entry.find(&bounds_key).and_then(|v| {
                if v.type_of() == CFDictionary::<CFString, CFType>::type_id() {
                    Some(unsafe {
                        CFDictionary::<CFString, CFType>::wrap_under_get_rule(
                            v.as_CFTypeRef() as core_foundation::dictionary::CFDictionaryRef
                        )
                    })
                } else {
                    None
                }
            }) else {
                continue;
            };
            let (Some(x), Some(y), Some(w), Some(h)) = (
                dict_f64(&bounds, "X"),
                dict_f64(&bounds, "Y"),
                dict_f64(&bounds, "Width"),
                dict_f64(&bounds, "Height"),
            ) else {
                continue;
            };
            out.push(TargetWindow {
                window_id: window_id as u32,
                pid: pid as u32,
                app,
                title,
                x,
                y,
                w,
                h,
            });
        }
        Ok(out)
    }

    /// Find a scroll-capture target. Filters AND together; `app`/`title`
    /// are case-insensitive substring matches. Own windows are excluded.
    pub fn find_target(
        pid: Option<u32>,
        app: Option<&str>,
        title: Option<&str>,
    ) -> Result<TargetWindow> {
        let own = std::process::id();
        let wins = list_windows()?;
        let mut name_cache: HashMap<u32, String> = HashMap::new();
        wins.into_iter()
            .filter(|w| w.pid != own)
            .filter(|w| w.w >= 200.0 && w.h >= 150.0)
            .find(|w| {
                pid.map_or(true, |p| w.pid == p)
                    && app.map_or(true, |a| {
                        let needle = a.to_lowercase();
                        // owner name is localized; also try the executable name
                        w.app.to_lowercase().contains(&needle) || {
                            let comm = name_cache
                                .entry(w.pid)
                                .or_insert_with(|| process_name(w.pid));
                            comm.to_lowercase().contains(&needle)
                        }
                    })
                    && title.map_or(true, |t| w.title.to_lowercase().contains(&t.to_lowercase()))
            })
            .ok_or_else(|| {
                anyhow!("no on-screen window matched (pid={pid:?} app={app:?} title={title:?})")
            })
    }

    /// The frontmost ordinary window under the current mouse pointer
    /// (excluding Unterm's own windows). Used by the GUI "point at a window,
    /// we'll long-shot it" flow.
    pub fn window_under_cursor() -> Result<TargetWindow> {
        let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| anyhow!("CGEventSource"))?;
        let loc = CGEvent::new(src)
            .map_err(|_| anyhow!("CGEvent::new"))?
            .location();
        let own = std::process::id();
        list_windows()?
            .into_iter()
            .filter(|w| w.pid != own)
            .find(|w| loc.x >= w.x && loc.x < w.x + w.w && loc.y >= w.y && loc.y < w.y + w.h)
            .ok_or_else(|| anyhow!("no window under the pointer"))
    }

    fn capture_window_frame(window_id: u32, path: &Path) -> Result<()> {
        let status = std::process::Command::new("/usr/sbin/screencapture")
            .args(["-x", "-o", "-t", "png", "-l", &window_id.to_string()])
            .arg(path)
            .status()
            .context("invoke screencapture -l")?;
        if !status.success() || !path.exists() {
            return Err(anyhow!("screencapture -l {window_id} failed"));
        }
        Ok(())
    }

    fn post_scroll(src: &CGEventSource, delta_points: i32) -> Result<()> {
        let ev =
            CGEvent::new_scroll_event(src.clone(), ScrollEventUnit::PIXEL, 1, delta_points, 0, 0)
                .map_err(|_| anyhow!("CGEvent::new_scroll_event"))?;
        ev.post(CGEventTapLocation::HID);
        Ok(())
    }

    /// Hash one pixel row, sampling every 4th pixel and excluding the right
    /// edge where macOS overlay scrollbars fade in/out during scrolling.
    fn row_hashes(img: &image::RgbaImage, right_exclude: u32) -> Vec<u64> {
        let w = img.width().saturating_sub(right_exclude).max(1);
        let mut out = Vec::with_capacity(img.height() as usize);
        for y in 0..img.height() {
            let mut h: u64 = 0xcbf29ce484222325;
            let mut x = 0;
            while x < w {
                let p = img.get_pixel(x, y);
                for c in 0..3 {
                    h ^= p.0[c] as u64;
                    h = h.wrapping_mul(0x100000001b3);
                }
                x += 4;
            }
            out.push(h);
        }
        out
    }

    /// How far the content moved up between `prev` and `cur` (in pixel rows),
    /// matching only rows whose hash is unique within `prev` so that blank /
    /// repeating regions can't fake a match. Returns (dy, matched_distinct).
    fn find_scroll_dy(
        prev: &[u64],
        cur: &[u64],
        fixed_top: usize,
        fixed_bottom: usize,
    ) -> (usize, usize) {
        let h = prev.len().min(cur.len());
        if h <= fixed_top + fixed_bottom + 32 {
            return (0, 0);
        }
        let span = h - fixed_bottom;
        // distinct rows of prev within the scrollable region
        let mut counts: HashMap<u64, u32> = HashMap::new();
        for &v in &prev[fixed_top..span] {
            *counts.entry(v).or_insert(0) += 1;
        }
        let min_overlap = ((span - fixed_top) / 5).max(32);
        let mut best = (0usize, 0usize);
        for dy in 0..(span - fixed_top).saturating_sub(min_overlap) {
            let mut matched = 0usize;
            let mut y = fixed_top + dy;
            while y < span {
                if counts.get(&prev[y]) == Some(&1) && prev[y] == cur[y - dy] {
                    matched += 1;
                }
                y += 2;
            }
            if matched > best.1 {
                best = (dy, matched);
            }
        }
        best
    }

    pub fn scroll_capture_window(
        target: &TargetWindow,
        out_path: &Path,
        opts: &ScrollCaptureOptions,
    ) -> Result<ScrollCaptureResult> {
        ensure_permissions()?;

        let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| anyhow!("CGEventSource::new failed"))?;

        if opts.activate {
            let _ = std::process::Command::new("osascript")
                .args([
                    "-e",
                    &format!(
                        "tell application \"System Events\" to set frontmost of \
                         (first process whose unix id is {}) to true",
                        target.pid
                    ),
                ])
                .status();
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        // Park the pointer over the window's content so wheel events route
        // there; remember where it was so we can put it back.
        let prev_loc = CGEvent::new(src.clone()).ok().map(|e| e.location());
        let park = CGPoint::new(target.x + target.w / 2.0, target.y + target.h * 0.55);
        let _ = CGDisplay::warp_mouse_cursor_position(park);
        let _ = CGDisplay::associate_mouse_and_mouse_cursor_position(true);
        std::thread::sleep(std::time::Duration::from_millis(80));

        let tmp = std::env::temp_dir().join(format!("unterm-scrollshot-{}", std::process::id()));
        std::fs::create_dir_all(&tmp)?;
        let frame_path = |i: usize| tmp.join(format!("frame{i:03}.png"));

        let restore_pointer = |loc: Option<CGPoint>| {
            if let Some(p) = loc {
                let _ = CGDisplay::warp_mouse_cursor_position(p);
                let _ = CGDisplay::associate_mouse_and_mouse_cursor_position(true);
            }
        };

        let run = || -> Result<(Vec<u8>, u32, u32, usize, Option<String>, i64)> {
            capture_window_frame(target.window_id, &frame_path(0))?;
            let f0 = image::open(frame_path(0))
                .context("decode frame 0")?
                .into_rgba8();
            let (w, h) = (f0.width(), f0.height());
            let scale = (w as f64 / target.w).max(1.0);
            let right_exclude = (24.0 * scale) as u32;
            let step_points = ((target.h * opts.step_frac) as i32).max(40);

            let mut prev_img = f0;
            let mut prev_hash = row_hashes(&prev_img, right_exclude);
            let mut bands: Vec<Vec<u8>> = vec![];
            let mut fixed_top = 0usize;
            let mut fixed_bottom = 0usize;
            let mut frames = 1usize;
            let mut zero_streak = 0usize;
            let mut total_dy = 0usize;
            let mut hint = None;

            for i in 1..opts.max_frames.max(2) {
                post_scroll(&src, -step_points)?;
                std::thread::sleep(std::time::Duration::from_millis(opts.settle_ms));
                capture_window_frame(target.window_id, &frame_path(i))?;
                let cur_img = image::open(frame_path(i))?.into_rgba8();
                if cur_img.width() != w || cur_img.height() != h {
                    return Err(anyhow!(
                        "window was resized mid-capture ({}x{} -> {}x{})",
                        w,
                        h,
                        cur_img.width(),
                        cur_img.height()
                    ));
                }
                let cur_hash = row_hashes(&cur_img, right_exclude);
                frames += 1;

                if frames == 2 {
                    // Fixed chrome detection from the first scrolled pair:
                    // identical prefix = title bar / sticky header, identical
                    // suffix = status bar / sticky footer.
                    fixed_top = prev_hash
                        .iter()
                        .zip(cur_hash.iter())
                        .take_while(|(a, b)| a == b)
                        .count()
                        .min(h as usize / 3);
                    fixed_bottom = prev_hash
                        .iter()
                        .rev()
                        .zip(cur_hash.iter().rev())
                        .take_while(|(a, b)| a == b)
                        .count()
                        .min(h as usize / 3);
                }

                let (dy, matched) = find_scroll_dy(&prev_hash, &cur_hash, fixed_top, fixed_bottom);
                if dy == 0 || matched < 8 {
                    zero_streak += 1;
                    if zero_streak >= 2 {
                        if frames <= 3 && total_dy == 0 {
                            hint = Some(
                                "no scroll movement detected — the window may not scroll at its \
                                 center, or Unterm lacks the macOS Accessibility permission \
                                 (System Settings → Privacy & Security → Accessibility)"
                                    .to_string(),
                            );
                        }
                        break;
                    }
                    continue;
                }
                zero_streak = 0;
                total_dy += dy;

                // append the freshly revealed rows (just above the fixed footer)
                let y_from = (h as usize - fixed_bottom).saturating_sub(dy);
                let y_to = h as usize - fixed_bottom;
                let mut band = Vec::with_capacity((y_to - y_from) * w as usize * 4);
                for y in y_from..y_to {
                    let row = &cur_img.as_raw()[(y * w as usize) * 4..(y + 1) * w as usize * 4];
                    band.extend_from_slice(row);
                }
                bands.push(band);
                prev_img = cur_img;
                prev_hash = cur_hash;
            }

            // canvas = first frame minus footer, all appended bands, then the
            // footer from the last frame so fixed bottom chrome appears once.
            let head_rows = h as usize - fixed_bottom;
            let total_height = head_rows
                + bands
                    .iter()
                    .map(|b| b.len() / (w as usize * 4))
                    .sum::<usize>()
                + fixed_bottom;

            let f0 = image::open(frame_path(0))?.into_rgba8();
            let file = std::fs::File::create(out_path)
                .with_context(|| format!("create {}", out_path.display()))?;
            let bufw = std::io::BufWriter::new(file);
            let mut enc = png::Encoder::new(bufw, w, total_height as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header()?;
            let mut stream = writer.stream_writer()?;
            use std::io::Write as _;
            stream.write_all(&f0.as_raw()[..head_rows * w as usize * 4])?;
            for band in &bands {
                stream.write_all(band)?;
            }
            stream.write_all(&prev_img.as_raw()[(h as usize - fixed_bottom) * w as usize * 4..])?;
            stream.finish()?;

            let scrolled_points = (total_dy as f64 / scale) as i64;
            Ok((
                vec![],
                w,
                total_height as u32,
                frames,
                hint,
                scrolled_points,
            ))
        };

        let result = run();

        // Best-effort: scroll the window back up to where it started.
        if opts.restore_scroll {
            if let Ok((_, _, _, _, _, scrolled_points)) = &result {
                let mut remaining = *scrolled_points;
                while remaining > 0 {
                    let chunk = remaining.min(800) as i32;
                    let _ = post_scroll(&src, chunk);
                    remaining -= chunk as i64;
                    std::thread::sleep(std::time::Duration::from_millis(60));
                }
            }
        }
        restore_pointer(prev_loc);
        let _ = std::fs::remove_dir_all(&tmp);

        let (_, w, total_h, frames, hint, _) = result?;
        Ok(ScrollCaptureResult {
            path: out_path.to_path_buf(),
            width: w,
            height: total_h,
            frames,
            window: target.clone(),
            hint,
        })
    }
}
