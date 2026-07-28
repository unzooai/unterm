//! Glyph rasterization for next-core, straight onto FreeType.
//!
//! The GPU path needs one thing from a font: given a character and a pixel
//! size, an 8-bit coverage bitmap plus the metrics to place it. `wezterm-font`
//! can do that, but it carries ten thousand lines of terminal-specific font
//! policy — fallback chains, config plumbing, shaping caches — none of which
//! next-core wants to inherit. FreeType is the library underneath it, and
//! talking to it directly is what the plan calls for: own the architecture,
//! reuse the mature library.
//!
//! Deliberately *not* here: font discovery and shaping. This turns a font file
//! into glyphs; choosing which file, and mapping text to glyph ids, are
//! separate problems.

use anyhow::{anyhow, Context, Result};
use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;

use freetype::{
    FT_Done_Face, FT_Done_FreeType, FT_Face, FT_Init_FreeType, FT_Library, FT_Load_Char,
    FT_Set_Pixel_Sizes, FT_LOAD_RENDER, FT_Get_Char_Index};

/// A rasterized glyph: 8-bit coverage plus where to put it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterizedGlyph {
    /// One byte of coverage per pixel, row-major, `width * height` long.
    pub coverage: Vec<u8>,
    pub width: usize,
    pub height: usize,
    /// Offset from the pen position to the bitmap's left edge.
    pub bearing_x: i32,
    /// Offset from the baseline up to the bitmap's top edge.
    pub bearing_y: i32,
    /// How far the pen advances after drawing, in pixels.
    pub advance_x: i32,
}

impl RasterizedGlyph {
    /// Expand coverage into RGBA, with the coverage in the alpha channel.
    ///
    /// The renderer tints with the cell's foreground colour and multiplies by
    /// alpha, so the colour channels only need to be non-zero; writing white
    /// keeps the texture readable in a debugger.
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(self.coverage.len() * 4);
        for value in &self.coverage {
            rgba.extend_from_slice(&[0xff, 0xff, 0xff, *value]);
        }
        rgba
    }
}

/// Owns the FreeType library handle.
///
/// FreeType's library and face handles are not `Sync`: a face borrows from its
/// library and neither may be touched from two threads at once. The handle is
/// kept private and every use goes through `&mut self`, so the borrow checker
/// enforces that for us.
struct Library {
    raw: FT_Library,
}

// SAFETY: the raw handle is never handed out, and every method that touches it
// takes `&mut`, so no two threads can be inside FreeType at the same time.
unsafe impl Send for Library {}

impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: `raw` came from a successful FT_Init_FreeType and is dropped
        // exactly once. Faces hold an Arc to this, so none are alive here.
        unsafe {
            FT_Done_FreeType(self.raw);
        }
    }
}

/// A font face loaded from a file, sized in pixels.
pub struct FontFace {
    // Kept alive for as long as the face: FreeType faces borrow from their
    // library, so dropping the library first would dangle.
    _library: Arc<Library>,
    face: FT_Face,
    pixel_size: u32,
}

// SAFETY: same argument as `Library` -- the handle is private and every use
// goes through `&mut self`.
unsafe impl Send for FontFace {}

impl Drop for FontFace {
    fn drop(&mut self) {
        // SAFETY: `face` came from a successful FT_New_Face and is dropped
        // exactly once, before the library it borrows from.
        unsafe {
            FT_Done_Face(self.face);
        }
    }
}

impl FontFace {
    /// Load `path` and size it to `pixel_size` pixels per em.
    pub fn open(path: &Path, pixel_size: u32) -> Result<Self> {
        let library = Arc::new(Library::init()?);
        let path_c = CString::new(path.as_os_str().to_string_lossy().as_bytes())
            .with_context(|| format!("font path is not representable as C string: {path:?}"))?;

        let mut face: FT_Face = std::ptr::null_mut();
        // SAFETY: `library.raw` is live, `path_c` outlives the call, and
        // `face` is a valid out-pointer.
        let err = unsafe { FT_New_Face(library.raw, path_c.as_ptr(), 0, &mut face) };
        if err != 0 {
            return Err(anyhow!("FT_New_Face({path:?}) failed with error {err}"));
        }

        let mut face = Self {
            _library: library,
            face,
            pixel_size: 0,
        };
        face.set_pixel_size(pixel_size)?;
        Ok(face)
    }

    /// The underlying FreeType face.
    ///
    /// For the shaper, which builds a HarfBuzz font from it. Crate-internal:
    /// the handle is not `Sync` and callers outside next-core have no way to
    /// uphold that.
    pub(crate) fn raw(&self) -> FT_Face {
        self.face
    }

    pub fn pixel_size(&self) -> u32 {
        self.pixel_size
    }

    /// Resize the face. Cheap enough to call per frame, but the caller is
    /// expected to cache rasterized glyphs, not re-rasterize them.
    pub fn set_pixel_size(&mut self, pixel_size: u32) -> Result<()> {
        let pixel_size = pixel_size.max(1);
        // SAFETY: `self.face` is live for the lifetime of `self`.
        let err = unsafe { FT_Set_Pixel_Sizes(self.face, 0, pixel_size) };
        if err != 0 {
            return Err(anyhow!(
                "FT_Set_Pixel_Sizes({pixel_size}) failed with error {err}"
            ));
        }
        self.pixel_size = pixel_size;
        Ok(())
    }

    /// What this face claims: family, style, and whether it is monospace.
    ///
    /// Returns `None` when the face reports no family name, which means the
    /// file is not something we can meaningfully offer to a user.
    pub fn describe(&self) -> Option<(String, String, bool)> {
        // SAFETY: `self.face` is live; FreeType keeps these strings alive for
        // the lifetime of the face, and they are NUL-terminated C strings.
        unsafe {
            let face = &*self.face;
            if face.family_name.is_null() {
                return None;
            }
            let family = std::ffi::CStr::from_ptr(face.family_name)
                .to_string_lossy()
                .into_owned();
            let style = if face.style_name.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(face.style_name)
                    .to_string_lossy()
                    .into_owned()
            };
            let monospace = face.face_flags & (freetype::FT_FACE_FLAG_FIXED_WIDTH as freetype::FT_Long) != 0;
            Some((family, style, monospace))
        }
    }

    /// Rasterize `ch` at the current pixel size.
    ///
    /// A glyph with no outline — a space, or a character this face does not
    /// cover — rasterizes to an empty bitmap with a real advance, which is
    /// what the layout wants: the pen still moves.
    /// The face's glyph for `ch`, or None when it has none.
    ///
    /// This is the question fallback turns on, and it has to be asked *before*
    /// rasterizing: `FT_Load_Char` does not fail on a missing character, it
    /// quietly loads glyph 0 and renders the empty box. A renderer that skips
    /// this check draws boxes instead of falling through to a face that has
    /// the character -- which is exactly what CJK looked like here.
    pub fn glyph_index_for(&self, ch: char) -> Option<u32> {
        // SAFETY: `self.face` is live for the lifetime of self.
        let index = unsafe { FT_Get_Char_Index(self.face, ch as freetype::FT_ULong) };
        (index != 0).then_some(index as u32)
    }

    /// Whether this face can draw `ch` itself.
    pub fn has_glyph(&self, ch: char) -> bool {
        self.glyph_index_for(ch).is_some()
    }

    pub fn rasterize(&mut self, ch: char) -> Result<RasterizedGlyph> {
        // SAFETY: `self.face` is live; FT_LOAD_RENDER asks FreeType to
        // rasterize into the face's glyph slot in the same call.
        let err =
            unsafe { FT_Load_Char(self.face, ch as freetype::FT_ULong, FT_LOAD_RENDER as i32) };
        if err != 0 {
            return Err(anyhow!("FT_Load_Char({ch:?}) failed with error {err}"));
        }
        self.read_rendered_slot()
    }

    /// Rasterize by glyph index rather than character.
    ///
    /// This is the entry point the shaper feeds: shaping maps text to glyph
    /// ids, and a ligature or a positional form has no character to look up.
    pub fn rasterize_glyph_index(&mut self, glyph_index: u32) -> Result<RasterizedGlyph> {
        // SAFETY: `self.face` is live. An index past the face's glyph count is
        // rejected by FreeType with an error rather than read out of bounds.
        let err = unsafe {
            freetype::FT_Load_Glyph(
                self.face,
                glyph_index as freetype::FT_UInt,
                FT_LOAD_RENDER as i32,
            )
        };
        if err != 0 {
            return Err(anyhow!(
                "FT_Load_Glyph({glyph_index}) failed with error {err}"
            ));
        }
        self.read_rendered_slot()
    }

    /// Read whatever the last load rendered into the face's glyph slot.
    fn read_rendered_slot(&mut self) -> Result<RasterizedGlyph> {
        // SAFETY: after a successful FT_Load_Char the slot and its bitmap are
        // populated and remain valid until the next load on this face.
        let (bitmap, bearing_x, bearing_y, advance_x, buffer) = unsafe {
            let slot = &*(*self.face).glyph;
            let bitmap = slot.bitmap;
            (
                bitmap,
                slot.bitmap_left,
                slot.bitmap_top,
                // 26.6 fixed point: whole pixels live above the low 6 bits.
                (slot.advance.x.font_units() >> 6) as i32,
                bitmap.buffer,
            )
        };

        let width = bitmap.width as usize;
        let height = bitmap.rows as usize;
        let pitch = bitmap.pitch;

        let coverage = if width == 0 || height == 0 || buffer.is_null() {
            Vec::new()
        } else {
            let mut coverage = Vec::with_capacity(width * height);
            for row in 0..height {
                // A negative pitch means the bitmap is stored bottom-up.
                let offset = if pitch >= 0 {
                    (row as isize) * (pitch as isize)
                } else {
                    ((height - 1 - row) as isize) * (-pitch as isize)
                };
                // SAFETY: FreeType guarantees `rows * |pitch|` readable bytes
                // from `buffer`, and `offset + width` stays inside that.
                let row_bytes = unsafe { std::slice::from_raw_parts(buffer.offset(offset), width) };
                coverage.extend_from_slice(row_bytes);
            }
            coverage
        };

        Ok(RasterizedGlyph {
            coverage,
            width,
            height,
            bearing_x,
            bearing_y,
            advance_x,
        })
    }
}

impl Library {
    fn init() -> Result<Self> {
        let mut raw: FT_Library = std::ptr::null_mut();
        // SAFETY: `raw` is a valid out-pointer; on success FreeType fills it.
        let err = unsafe { FT_Init_FreeType(&mut raw) };
        if err != 0 {
            return Err(anyhow!("FT_Init_FreeType failed with error {err}"));
        }
        Ok(Self { raw })
    }
}

use freetype::FT_New_Face;

#[cfg(test)]
mod tests {
    use super::*;

    /// A font file that exists on the test machine.
    ///
    /// Discovery is out of scope for this module, so the tests take the
    /// platform's best-known path and skip when it is absent rather than
    /// failing on a machine that simply keeps its fonts elsewhere.
    fn test_font() -> Option<std::path::PathBuf> {
        let candidates: &[&str] = if cfg!(windows) {
            &[
                r"C:\Windows\Fonts\consola.ttf",
                r"C:\Windows\Fonts\cour.ttf",
                r"C:\Windows\Fonts\arial.ttf",
            ]
        } else if cfg!(target_os = "macos") {
            &["/System/Library/Fonts/Menlo.ttc", "/Library/Fonts/Arial.ttf"]
        } else {
            &[
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
                "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            ]
        };
        candidates
            .iter()
            .map(std::path::PathBuf::from)
            .find(|path| path.exists())
    }

    #[test]
    fn rasterizes_a_glyph_with_partial_coverage() {
        let Some(path) = test_font() else {
            eprintln!("no system font found; skipping");
            return;
        };
        let mut face = FontFace::open(&path, 32).expect("open font");
        assert_eq!(face.pixel_size(), 32);

        let glyph = face.rasterize('M').expect("rasterize M");

        assert!(glyph.width > 0 && glyph.height > 0, "M should have an outline");
        assert_eq!(glyph.coverage.len(), glyph.width * glyph.height);
        assert!(glyph.advance_x > 0, "the pen must advance past an M");

        // The point of rasterizing: a shape, not a filled box. Both fully
        // covered and fully empty pixels must be present, or the glyph would
        // render as a solid rectangle.
        assert!(
            glyph.coverage.iter().any(|c| *c > 0),
            "glyph has no ink at all"
        );
        assert!(
            glyph.coverage.iter().any(|c| *c == 0),
            "glyph is solid; coverage is not a mask"
        );
    }

    #[test]
    fn a_space_has_no_ink_but_still_advances() {
        let Some(path) = test_font() else {
            eprintln!("no system font found; skipping");
            return;
        };
        let mut face = FontFace::open(&path, 24).expect("open font");

        let glyph = face.rasterize(' ').expect("rasterize space");

        assert!(glyph.coverage.iter().all(|c| *c == 0));
        assert!(
            glyph.advance_x > 0,
            "a space must still move the pen, or text would pile up"
        );
    }

    #[test]
    fn resizing_changes_the_rasterized_size() {
        let Some(path) = test_font() else {
            eprintln!("no system font found; skipping");
            return;
        };
        let mut face = FontFace::open(&path, 16).expect("open font");
        let small = face.rasterize('W').expect("rasterize small W");

        face.set_pixel_size(48).expect("resize");
        let large = face.rasterize('W').expect("rasterize large W");

        assert!(
            large.height > small.height && large.width > small.width,
            "48px W ({}x{}) should be bigger than 16px ({}x{})",
            large.width,
            large.height,
            small.width,
            small.height
        );
    }

    #[test]
    fn rgba_puts_coverage_in_the_alpha_channel() {
        let glyph = RasterizedGlyph {
            coverage: vec![0, 128, 255],
            width: 3,
            height: 1,
            bearing_x: 0,
            bearing_y: 0,
            advance_x: 3,
        };

        let rgba = glyph.to_rgba();

        assert_eq!(rgba.len(), 12);
        // The renderer multiplies its own colour by this alpha, so the
        // coverage has to land there and nowhere else.
        assert_eq!(&rgba[0..4], &[0xff, 0xff, 0xff, 0]);
        assert_eq!(&rgba[4..8], &[0xff, 0xff, 0xff, 128]);
        assert_eq!(&rgba[8..12], &[0xff, 0xff, 0xff, 255]);
    }

    #[test]
    fn a_missing_font_file_is_an_error_not_a_panic() {
        let err = match FontFace::open(std::path::Path::new("does-not-exist.ttf"), 16) {
            Ok(_) => panic!("opening a missing font must fail"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("FT_New_Face"));
    }
}
