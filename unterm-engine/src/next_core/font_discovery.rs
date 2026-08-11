//! Finding a font file to rasterize.
//!
//! `font_raster` turns a *file* into glyphs; something has to decide which
//! file. wezterm-font answers that with fontconfig, CoreText, DirectWrite, and
//! a configuration language on top. next-core answers it by reading the
//! platform's font directories and asking FreeType what each file contains —
//! no extra dependency, same answer for the case a terminal actually needs:
//! "give me a monospace face".
//!
//! Honest about its limits: this finds installed font *files*. It does not do
//! fontconfig aliasing, per-script fallback chains, or variable-font instance
//! selection. A terminal needs a monospace face and a way to name a specific
//! one; that is what this provides.

use crate::next_core::font_raster::FontFace;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// What a font file claims about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontEntry {
    pub path: PathBuf,
    /// Which face inside the file: 0 for a plain .ttf, the collection slot
    /// for a .ttc. Opening the path without it lands on whatever face is
    /// first, which for PingFang is Ultralight.
    pub face_index: i64,
    pub family: String,
    pub style: String,
    /// The face reports fixed advance widths, i.e. it is a monospace font.
    pub monospace: bool,
}

/// Directories the platform keeps fonts in.
///
/// User directories come first: a font the user installed for themselves
/// should win over a system one with the same name, which is what every other
/// font system does.
pub fn font_directories() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    #[cfg(windows)]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
        }
        if let Some(windir) = std::env::var_os("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
        dirs.push(PathBuf::from("/Library/Fonts"));
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        // Where macOS keeps fonts it downloaded on demand -- PingFang lives
        // here, not under /System/Library/Fonts. Without it the Chinese
        // fallback lands on Hiragino W0, and every hanzi comes out hairline.
        dirs.push(PathBuf::from(
            "/System/Library/AssetsV2/com_apple_MobileAsset_Font8",
        ));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            dirs.push(home.join(".local/share/fonts"));
            dirs.push(home.join(".fonts"));
        }
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
    }

    dirs.retain(|dir| dir.is_dir());
    dirs
}

fn has_font_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("ttf" | "otf" | "ttc" | "otc")
    )
}

/// Walk `dir` for font files, following subdirectories.
///
/// Linux distributions nest fonts several levels deep, so a flat read would
/// miss most of them. The depth limit stops a symlink loop from hanging
/// startup.
fn collect_font_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    const MAX_DEPTH: usize = 6;
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_font_files(&path, depth + 1, out);
        } else if has_font_extension(&path) {
            out.push(path);
        }
    }
}

/// Read what a font file claims: one entry per face it carries.
///
/// A collection is a family in one file; describing only its first face
/// hands the index a single arbitrary weight -- PingFang's is Ultralight --
/// and `best_in_family` can then never find the regular one.
pub fn describe(path: &Path) -> Vec<FontEntry> {
    // Size is irrelevant for reading metadata, but a face has to be sized
    // before it is usable, so pick something small and valid.
    let Ok(first) = FontFace::open(path, 16) else {
        return Vec::new();
    };
    let count = first.num_faces().max(1);
    let mut entries = Vec::new();
    for face_index in 0..count {
        let face = if face_index == 0 {
            None
        } else {
            match FontFace::open_indexed(path, face_index, 16) {
                Ok(face) => Some(face),
                Err(_) => continue,
            }
        };
        let described = face.as_ref().unwrap_or(&first).describe();
        if let Some((family, style, monospace)) = described {
            entries.push(FontEntry {
                path: path.to_path_buf(),
                face_index,
                family,
                style,
                monospace,
            });
        }
    }
    entries
}

/// Every font the platform has installed, indexed by family name.
///
/// Building this opens every font file, so callers should do it once and keep
/// the result rather than calling it per frame.
pub fn scan_installed_fonts() -> Vec<FontEntry> {
    let mut files = Vec::new();
    for dir in font_directories() {
        collect_font_files(&dir, 0, &mut files);
    }
    files.sort();
    files.dedup();
    files.iter().flat_map(|path| describe(path)).collect()
}

/// Index of installed fonts, keyed by lowercased family name.
#[derive(Clone, Debug, Default)]
pub struct FontIndex {
    by_family: BTreeMap<String, Vec<FontEntry>>,
}

impl FontIndex {
    pub fn from_entries(entries: Vec<FontEntry>) -> Self {
        let mut by_family: BTreeMap<String, Vec<FontEntry>> = BTreeMap::new();
        for entry in entries {
            by_family
                .entry(entry.family.to_lowercase())
                .or_default()
                .push(entry);
        }
        Self { by_family }
    }

    pub fn scan() -> Self {
        Self::from_entries(scan_installed_fonts())
    }

    /// Process-wide installed-font index.
    ///
    /// A GUI startup opens the terminal face, the chrome face, and on scaled
    /// displays reopens both once the window reports its DPI. Scanning the
    /// platform font directories for each of those calls makes startup pay the
    /// same filesystem walk several times.
    pub fn cached() -> &'static Self {
        static INDEX: OnceLock<FontIndex> = OnceLock::new();
        INDEX.get_or_init(Self::scan)
    }

    pub fn is_empty(&self) -> bool {
        self.by_family.is_empty()
    }

    pub fn family_count(&self) -> usize {
        self.by_family.len()
    }

    /// All faces in a family, case-insensitively.
    pub fn family(&self, name: &str) -> &[FontEntry] {
        self.by_family
            .get(&name.to_lowercase())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The best face for `name`, preferring the regular style.
    ///
    /// A family usually ships several files (Regular, Bold, Italic…). Asking
    /// for a family by name and getting Bold Italic would be surprising.
    pub fn best_in_family(&self, name: &str) -> Option<&FontEntry> {
        let faces = self.family(name);
        // The lower the closer to regular. "Roman" is what several shipped
        // faces call it -- Cascadia Mono among them; W3/W4 are the regular
        // weights in Hiragino's numbering, whose W0 is a hairline that a
        // first-wins pick would land on.
        let closeness = |entry: &FontEntry| match entry.style.to_lowercase().as_str() {
            "regular" | "book" | "normal" | "roman" => 0,
            "w3" | "w4" => 1,
            "medium" | "text" => 2,
            _ => 3,
        };
        faces.iter().min_by_key(|entry| closeness(entry))
    }

    /// A monospace face, trying the usual terminal fonts before settling for
    /// any monospace family the machine has.
    ///
    /// Returning something sensible without configuration is the point: a
    /// terminal that cannot find a font does not start.
    pub fn default_monospace(&self) -> Option<&FontEntry> {
        const PREFERRED: &[&str] = &[
            "Cascadia Mono",
            "Cascadia Code",
            "Consolas",
            "SF Mono",
            "Menlo",
            "DejaVu Sans Mono",
            "Liberation Mono",
            "Noto Sans Mono",
            "Ubuntu Mono",
            "Courier New",
        ];
        for name in PREFERRED {
            if let Some(entry) = self.best_in_family(name).filter(|entry| entry.monospace) {
                return Some(entry);
            }
        }
        // Deterministic rather than "whatever the filesystem listed first":
        // the same machine should pick the same fallback every launch.
        self.by_family
            .values()
            .flatten()
            .filter(|entry| entry.monospace)
            .min_by(|a, b| a.family.cmp(&b.family).then_with(|| a.path.cmp(&b.path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_extensions_are_recognized_case_insensitively() {
        for name in ["a.ttf", "b.OTF", "c.ttc", "d.Otc"] {
            assert!(has_font_extension(Path::new(name)), "{}", name);
        }
        for name in ["a.txt", "b.ttf.bak", "c", "d.png"] {
            assert!(!has_font_extension(Path::new(name)), "{}", name);
        }
    }

    #[test]
    fn the_platform_has_font_directories() {
        let dirs = font_directories();
        assert!(
            !dirs.is_empty(),
            "no font directory found; a terminal cannot render text without one"
        );
        assert!(dirs.iter().all(|dir| dir.is_dir()));
    }

    #[test]
    fn scanning_finds_a_usable_monospace_face() {
        let index = FontIndex::scan();
        if index.is_empty() {
            eprintln!("no fonts installed; skipping");
            return;
        }
        assert!(index.family_count() > 1, "expected more than one family");

        let mono = index
            .default_monospace()
            .expect("a machine with fonts should have a monospace face");
        assert!(mono.monospace);
        assert!(!mono.family.is_empty());
        assert!(mono.path.exists());

        // The point of discovery is a file the rasterizer can actually use.
        let mut face = FontFace::open(&mono.path, 24).expect("open the discovered font");
        let glyph = face.rasterize('W').expect("rasterize through it");
        assert!(glyph.width > 0 && glyph.height > 0);
    }

    #[test]
    fn cached_index_is_reused_in_process() {
        assert!(std::ptr::eq(FontIndex::cached(), FontIndex::cached()));
    }

    #[test]
    fn family_lookup_is_case_insensitive_and_prefers_regular() {
        let entries = vec![
            FontEntry {
                path: PathBuf::from("bold.ttf"),
                face_index: 0,
                family: "Test Family".into(),
                style: "Bold".into(),
                monospace: true,
            },
            FontEntry {
                path: PathBuf::from("regular.ttf"),
                face_index: 0,
                family: "Test Family".into(),
                style: "Regular".into(),
                monospace: true,
            },
        ];
        let index = FontIndex::from_entries(entries);

        assert_eq!(index.family("test family").len(), 2);
        assert_eq!(index.family("TEST FAMILY").len(), 2);
        // Asking for a family by name and getting Bold would be surprising.
        assert_eq!(
            index
                .best_in_family("Test Family")
                .map(|e| e.style.as_str()),
            Some("Regular")
        );

        // "Roman" is regular under another name; Cascadia Mono ships that way,
        // so treating it as a non-regular style would leave the default font
        // depending on directory order.
        let roman = FontIndex::from_entries(vec![
            FontEntry {
                path: PathBuf::from("italic.ttf"),
                face_index: 0,
                family: "Roman Family".into(),
                style: "Italic".into(),
                monospace: true,
            },
            FontEntry {
                path: PathBuf::from("roman.ttf"),
                face_index: 0,
                family: "Roman Family".into(),
                style: "Roman".into(),
                monospace: true,
            },
        ]);
        assert_eq!(
            roman
                .best_in_family("Roman Family")
                .map(|e| e.style.as_str()),
            Some("Roman")
        );
        assert!(index.family("nothing here").is_empty());
    }

    #[test]
    fn monospace_fallback_is_deterministic_and_skips_proportional_faces() {
        let entries = vec![
            FontEntry {
                path: PathBuf::from("zzz.ttf"),
                face_index: 0,
                family: "Zzz Mono".into(),
                style: "Regular".into(),
                monospace: true,
            },
            FontEntry {
                path: PathBuf::from("aaa.ttf"),
                face_index: 0,
                family: "Aaa Sans".into(),
                style: "Regular".into(),
                monospace: false,
            },
            FontEntry {
                path: PathBuf::from("mmm.ttf"),
                face_index: 0,
                family: "Mmm Mono".into(),
                style: "Regular".into(),
                monospace: true,
            },
        ];
        let index = FontIndex::from_entries(entries);

        let picked = index.default_monospace().expect("a monospace face");
        // Not the alphabetically-first family overall -- that one is
        // proportional, and a terminal in a proportional font is unusable.
        assert_eq!(picked.family, "Mmm Mono");

        // No monospace at all is None, not a proportional face.
        let only_proportional = FontIndex::from_entries(vec![FontEntry {
            path: PathBuf::from("a.ttf"),
            face_index: 0,
            family: "Aaa Sans".into(),
            style: "Regular".into(),
            monospace: false,
        }]);
        assert!(only_proportional.default_monospace().is_none());
    }
}
