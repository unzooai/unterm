#![cfg(target_os = "macos")]
//! CoreText glyph rasterizer for macOS.
//!
//! wezterm rasterizes glyphs through FreeType on every platform. On
//! macOS that renders noticeably softer than the system's own CoreText
//! pipeline — the long-standing gap users see versus Terminal.app and
//! iTerm2. This rasterizer draws glyphs with CoreText + CoreGraphics so
//! text matches the native macOS look: grayscale-antialiased, gamma
//! blended, no LCD color fringing.
//!
//! The output matches the contract the rest of the font stack expects
//! from [`FontRasterizer`]: a premultiplied-RGBA bitmap with top-left
//! origin, plus left/top bearings in pixels.

use crate::parser::ParsedFont;
use crate::rasterizer::{FontRasterizer, RasterizedGlyph};
use crate::units::PixelLength;
use anyhow::anyhow;
use core_graphics::base::CGGlyph;
use core_graphics::color_space::CGColorSpace;
use core_graphics::context::CGContext;
use core_graphics::data_provider::CGDataProvider;
use core_graphics::font::CGFont;
use core_graphics::geometry::{CGPoint, CGRect, CGSize};
use core_text::font::{new_from_CGFont, CTFont};
use std::cell::RefCell;
use std::sync::Arc;

/// Transparent padding (in pixels) added around the glyph ink box so
/// antialiased edges aren't clipped.
const PAD: usize = 1;

pub struct CoreTextRasterizer {
    cg_font: CGFont,
    scale: f64,
    /// CTFont is size-specific; cache the most recently used pixel size
    /// so a run of same-size glyphs doesn't rebuild it each call.
    cached: RefCell<Option<(u64, CTFont)>>,
}

impl CoreTextRasterizer {
    pub fn from_locator(parsed: &ParsedFont) -> anyhow::Result<Self> {
        let data = parsed.handle.source.load_data()?;
        let provider = CGDataProvider::from_buffer(Arc::new(data.into_owned()));
        let cg_font = CGFont::from_data_provider(provider).map_err(|_| {
            anyhow!(
                "CGFont::from_data_provider failed for {}",
                parsed.handle.source.name_or_path_str()
            )
        })?;
        Ok(Self {
            cg_font,
            scale: parsed.scale.unwrap_or(1.),
            cached: RefCell::new(None),
        })
    }

    fn ct_font(&self, pixel_size: f64) -> CTFont {
        let key = pixel_size.to_bits();
        let mut cache = self.cached.borrow_mut();
        if let Some((k, font)) = cache.as_ref() {
            if *k == key {
                return font.clone();
            }
        }
        let font = new_from_CGFont(&self.cg_font, pixel_size);
        *cache = Some((key, font.clone()));
        font
    }
}

impl FontRasterizer for CoreTextRasterizer {
    fn rasterize_glyph(
        &self,
        glyph_pos: u32,
        size: f64,
        dpi: u32,
    ) -> anyhow::Result<RasterizedGlyph> {
        // `size` is in points; a CTFont at this many pixels makes 1 unit
        // = 1 device pixel, so all the metrics come back in pixels.
        let pixel_size = size * self.scale * (dpi as f64) / 72.0;
        if pixel_size <= 0.0 {
            return Ok(empty_glyph());
        }
        let ct_font = self.ct_font(pixel_size);
        raster_glyph(&ct_font, glyph_pos as CGGlyph)
    }
}

fn empty_glyph() -> RasterizedGlyph {
    RasterizedGlyph {
        data: vec![],
        height: 0,
        width: 0,
        bearing_x: PixelLength::new(0.),
        bearing_y: PixelLength::new(0.),
        has_color: false,
        is_scaled: true,
    }
}

/// Rasterize a single glyph from a pixel-sized CTFont into the
/// premultiplied-RGBA, top-left-origin bitmap the font stack expects.
fn raster_glyph(ct_font: &CTFont, glyph: CGGlyph) -> anyhow::Result<RasterizedGlyph> {
    // Ink bounding box relative to the pen origin on the baseline:
    // origin.x = left bearing, origin.y = descent below baseline
    // (negative for descenders), size = ink extent.
    let bbox = ct_font.get_bounding_rects_for_glyphs(0, &[glyph]);
    let ink_w = bbox.size.width;
    let ink_h = bbox.size.height;
    if !(ink_w > 0.0) || !(ink_h > 0.0) {
        // Whitespace / empty glyph — nothing to draw.
        return Ok(empty_glyph());
    }

    let width = ink_w.ceil() as usize + 2 * PAD;
    let height = ink_h.ceil() as usize + 2 * PAD;

    // 8-bit device-gray bitmap, no alpha channel: the pixel value is the
    // coverage of white ink on a black ground, gamma-blended the way
    // macOS does it.
    let color_space = CGColorSpace::create_device_gray();
    let mut ctx = CGContext::create_bitmap_context(
        None,
        width,
        height,
        8,
        width, // bytes_per_row; 1 byte/pixel for gray
        &color_space,
        0, // kCGImageAlphaNone
    );

    // Black ground, white ink, native grayscale AA (no subpixel / LCD
    // smoothing — that's what produces the color fringing we avoid).
    ctx.set_should_antialias(true);
    ctx.set_should_smooth_fonts(false);
    ctx.set_allows_font_smoothing(false);
    ctx.set_gray_fill_color(0.0, 1.0);
    ctx.fill_rect(CGRect::new(
        &CGPoint::new(0.0, 0.0),
        &CGSize::new(width as f64, height as f64),
    ));
    ctx.set_gray_fill_color(1.0, 1.0);

    // Place the glyph so its ink box bottom-left sits at (PAD, PAD).
    // CoreGraphics is y-up with the origin at the bottom-left.
    let pos = CGPoint::new(PAD as f64 - bbox.origin.x, PAD as f64 - bbox.origin.y);
    ct_font.draw_glyphs(&[glyph], &[pos], ctx.clone());

    let bytes_per_row = ctx.bytes_per_row();
    let src = ctx.data();

    // The CGBitmapContext buffer is already stored top-down (memory row 0
    // is the top of the image), matching the top-left origin our consumers
    // expect — no flip needed. For white ink the coverage `g` is both the
    // premultiplied color and the alpha.
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        let src_row = y * bytes_per_row;
        let dst_row = y * width * 4;
        for x in 0..width {
            let g = src[src_row + x];
            let d = dst_row + x * 4;
            rgba[d] = g;
            rgba[d + 1] = g;
            rgba[d + 2] = g;
            rgba[d + 3] = g;
        }
    }

    Ok(RasterizedGlyph {
        data: rgba,
        width,
        height,
        // Bitmap left edge = ink left − PAD from the pen origin.
        bearing_x: PixelLength::new(bbox.origin.x - PAD as f64),
        // Bitmap top edge = ink top + PAD above the baseline.
        bearing_y: PixelLength::new(bbox.origin.y + ink_h + PAD as f64),
        has_color: false,
        is_scaled: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rasterize the letter 'F' and dump it as ASCII art so the glyph
    /// orientation (no mirroring / no upside-down) is unambiguous.
    #[test]
    fn glyph_f_orientation() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../assets/fonts/JetBrainsMono-Bold.ttf"
        );
        let bytes = std::fs::read(path).expect("read test font");
        let provider = CGDataProvider::from_buffer(Arc::new(bytes));
        let cg_font = CGFont::from_data_provider(provider).expect("CGFont");
        let ct_font = new_from_CGFont(&cg_font, 32.0);

        // Map 'F' to its glyph index.
        let ch: u16 = b'F' as u16;
        let mut glyphs = [0u16; 1];
        let ok = unsafe {
            ct_font.get_glyphs_for_characters(&ch, glyphs.as_mut_ptr(), 1)
        };
        assert!(ok, "glyph lookup for 'F'");
        let glyph = glyphs[0];

        let g = raster_glyph(&ct_font, glyph).expect("raster");
        eprintln!("F: {}x{}", g.width, g.height);
        for y in 0..g.height {
            let mut line = String::new();
            for x in 0..g.width {
                let a = g.data[(y * g.width + x) * 4 + 3];
                line.push(if a > 128 { '#' } else { '.' });
            }
            eprintln!("{line}");
        }
    }
}
