//! How much scrollback to put in a screenshot, and how sharply.
//!
//! Plain settings, kept apart from the renderer that uses them: the renderer
//! is built on one front end's font stack, but what to render is a question
//! anyone can ask.

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
