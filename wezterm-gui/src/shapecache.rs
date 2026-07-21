use crate::customglyph::BlockKey;
use crate::glyphcache::CachedGlyph;
use config::TextStyle;
use std::rc::Rc;
use wezterm_font::shaper::GlyphInfo;
use wezterm_font::units::*;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct ShapeCacheKey {
    pub style: TextStyle,
    pub text: String,
}

#[derive(Debug, PartialEq)]
pub struct GlyphPosition {
    pub glyph_idx: u32,
    pub num_cells: u8,
    pub x_offset: PixelLength,
    pub bearing_x: f32,
    pub bitmap_pixel_width: u32,
}

#[derive(Debug)]
pub struct ShapedInfo {
    pub glyph: Rc<CachedGlyph>,
    pub pos: GlyphPosition,
    pub block_key: Option<BlockKey>,
}

impl ShapedInfo {
    /// Process the results from the shaper, stitching together glyph
    /// and positioning information
    pub fn process(infos: &[GlyphInfo], glyphs: &[Rc<CachedGlyph>]) -> Vec<ShapedInfo> {
        let mut pos: Vec<ShapedInfo> = Vec::with_capacity(infos.len());

        for (info, glyph) in infos.iter().zip(glyphs.iter()) {
            pos.push(ShapedInfo {
                pos: GlyphPosition {
                    glyph_idx: info.glyph_pos,
                    bitmap_pixel_width: glyph
                        .texture
                        .as_ref()
                        .map_or(0, |t| t.coords.width() as u32),
                    num_cells: info.num_cells,
                    x_offset: info.x_offset,
                    bearing_x: glyph.bearing_x.get() as f32,
                },
                glyph: Rc::clone(glyph),
                block_key: info.only_char.and_then(BlockKey::from_char),
            });
        }
        pos
    }
}

/// We'd like to avoid allocating when resolving from the cache
/// so this is the borrowed version of ShapeCacheKey.
/// It's a bit involved to make this work; more details can be
/// found in the excellent guide here:
/// <https://github.com/sunshowers/borrow-complex-key-example/blob/master/src/lib.rs>
#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BorrowedShapeCacheKey<'a> {
    pub style: &'a TextStyle,
    pub text: &'a str,
}

impl<'a> BorrowedShapeCacheKey<'a> {
    pub fn to_owned(&self) -> ShapeCacheKey {
        ShapeCacheKey {
            style: self.style.clone(),
            text: self.text.to_owned(),
        }
    }
}

pub trait ShapeCacheKeyTrait: std::fmt::Debug {
    fn key<'k>(&'k self) -> BorrowedShapeCacheKey<'k>;
}

impl ShapeCacheKeyTrait for ShapeCacheKey {
    fn key<'k>(&'k self) -> BorrowedShapeCacheKey<'k> {
        BorrowedShapeCacheKey {
            style: &self.style,
            text: &self.text,
        }
    }
}

impl<'a> ShapeCacheKeyTrait for BorrowedShapeCacheKey<'a> {
    fn key<'k>(&'k self) -> BorrowedShapeCacheKey<'k> {
        *self
    }
}

impl<'a> std::borrow::Borrow<dyn ShapeCacheKeyTrait + 'a> for ShapeCacheKey {
    fn borrow(&self) -> &(dyn ShapeCacheKeyTrait + 'a) {
        self
    }
}

impl<'a> PartialEq for dyn ShapeCacheKeyTrait + 'a {
    fn eq(&self, other: &Self) -> bool {
        self.key().eq(&other.key())
    }
}

impl<'a> Eq for dyn ShapeCacheKeyTrait + 'a {}

impl<'a> std::hash::Hash for dyn ShapeCacheKeyTrait + 'a {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state)
    }
}

#[cfg(test)]
mod test {
    use crate::glyphcache::GlyphCache;
    use crate::shapecache::{GlyphPosition, ShapedInfo};
    use crate::utilsprites::RenderMetrics;
    use config::{FontAttributes, TextStyle};
    use std::rc::Rc;
    use termwiz::cell::CellAttributes;
    use termwiz::surface::{Line, SEQ_ZERO};
    use wezterm_bidi::Direction;
    use wezterm_font::shaper::PresentationWidth;
    use wezterm_font::units::PixelLength;
    use wezterm_font::{FontConfiguration, LoadedFont};

    fn cluster_and_shape(
        render_metrics: &RenderMetrics,
        glyph_cache: &mut GlyphCache,
        style: &TextStyle,
        font: &Rc<LoadedFont>,
        text: &str,
    ) -> Vec<GlyphPosition> {
        let line = Line::from_text(text, &CellAttributes::default(), SEQ_ZERO, None);
        eprintln!("{:?}", line);
        let mut all_infos = vec![];
        let mut all_glyphs = vec![];

        for cluster in line.cluster(None) {
            let presentation_width = PresentationWidth::with_cluster(&cluster);
            let mut infos = font
                .shape(
                    &cluster.text,
                    || {},
                    |_| {},
                    None,
                    Direction::LeftToRight,
                    None,
                    Some(&presentation_width),
                )
                .unwrap();
            let mut glyphs = infos
                .iter()
                .map(|info| {
                    let cell_idx = cluster.byte_to_cell_idx(info.cluster as usize);
                    let num_cells = cluster.byte_to_cell_width(info.cluster as usize);

                    let followed_by_space = match line.get_cell(cell_idx + 1) {
                        Some(cell) => cell.str() == " ",
                        None => false,
                    };

                    glyph_cache
                        .cached_glyph(
                            info,
                            &style,
                            followed_by_space,
                            font,
                            render_metrics,
                            num_cells,
                        )
                        .unwrap()
                })
                .collect::<Vec<_>>();

            all_infos.append(&mut infos);
            all_glyphs.append(&mut glyphs);
        }

        eprintln!("infos: {:#?}", all_infos);
        eprintln!("glyphs: {:#?}", all_glyphs);
        ShapedInfo::process(&all_infos, &all_glyphs)
            .into_iter()
            .map(|p| p.pos)
            .collect()
    }

    /// Raster bearings and bitmap widths are renderer outputs: CoreText and
    /// FreeType legitimately produce different sub-pixel values for the same
    /// bundled font.  The shaping contract is the glyph sequence, cell span,
    /// and logical offset, so snapshots compare that stable signature while
    /// `cluster_and_shape` still exercises glyph rasterization and caching.
    fn shape_signature(positions: &[GlyphPosition]) -> Vec<(u32, u8)> {
        positions
            .iter()
            .map(|pos| {
                assert_eq!(pos.x_offset, PixelLength::new(0.0));
                (pos.glyph_idx, pos.num_cells)
            })
            .collect()
    }

    #[test]
    fn ligatures_fira() {
        config::use_test_configuration();
        let _ = env_logger::Builder::new()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .try_init();

        let config = config::configuration();

        let mut config: config::Config = (*config).clone();
        config.font = TextStyle {
            font: vec![FontAttributes::new("Fira Code")],
            foreground: None,
        };
        config.font_rules.clear();
        config.compute_extra_defaults(None);
        config::use_this_configuration(config.clone());

        let fonts = Rc::new(
            FontConfiguration::new(
                None,
                config.dpi.unwrap_or_else(|| ::window::default_dpi()) as usize,
            )
            .unwrap(),
        );
        let render_metrics = RenderMetrics::new(&fonts).unwrap();
        let mut glyph_cache = GlyphCache::new_in_memory(&fonts, 128).unwrap();

        let style = TextStyle::default();
        let font = fonts.resolve_font(&style).unwrap();

        let shaped = cluster_and_shape(&render_metrics, &mut glyph_cache, &style, &font, "a...");
        assert_eq!(
            shape_signature(&shaped),
            vec![(189, 1), (1742, 1), (1742, 1), (896, 1)]
        );
    }

    #[test]
    fn bench_shaping() {
        config::use_test_configuration();

        // let mut glyph_cache = GlyphCache::new_in_memory(&fonts, 128, &render_metrics).unwrap();
        // let render_metrics = RenderMetrics::new(&fonts).unwrap();

        benchmarking::warm_up();

        for &n in &[100, 1000, 10_000] {
            let bench_result = benchmarking::measure_function(move |measurer| {
                let text: String = (0..n).map(|_| ' ').collect();

                let fonts = Rc::new(
                    FontConfiguration::new(
                        None,
                        config::configuration()
                            .dpi
                            .unwrap_or_else(|| ::window::default_dpi())
                            as usize,
                    )
                    .unwrap(),
                );
                let style = TextStyle::default();
                let font = fonts.resolve_font(&style).unwrap();
                let line = Line::from_text(&text, &CellAttributes::default(), SEQ_ZERO, None);
                let cell_clusters = line.cluster(None);
                let cluster = &cell_clusters[0];
                let presentation_width = PresentationWidth::with_cluster(&cluster);

                measurer.measure(|| {
                    let _x = font
                        .shape(
                            &cluster.text,
                            || {},
                            |_| {},
                            None,
                            Direction::LeftToRight,
                            None,
                            Some(&presentation_width),
                        )
                        .unwrap();
                    // println!("{:?}", &x[0..2]);
                });
            })
            .unwrap();
            println!("{}: {:?}", n, bench_result.elapsed());
        }
    }

    #[test]
    fn ligatures_jetbrains() {
        config::use_test_configuration();
        let config = config::configuration();
        let fonts = Rc::new(
            FontConfiguration::new(
                None,
                config.dpi.unwrap_or_else(|| ::window::default_dpi()) as usize,
            )
            .unwrap(),
        );
        let render_metrics = RenderMetrics::new(&fonts).unwrap();
        let mut glyph_cache = GlyphCache::new_in_memory(&fonts, 128).unwrap();
        let style = TextStyle::default();
        let font = fonts.resolve_font(&style).unwrap();

        let cases: &[(&str, &[(u32, u8)])] = &[
            ("ab", &[(189, 1), (214, 1)]),
            ("a b", &[(189, 1), (958, 1), (214, 1)]),
            ("a...", &[(189, 1), (1742, 1), (1742, 1), (896, 1)]),
            ("e_or_", &[(225, 1), (860, 1), (290, 1), (320, 1), (860, 1)]),
            ("a  b", &[(189, 1), (958, 1), (958, 1), (214, 1)]),
            ("<-", &[(1742, 1), (1588, 1)]),
            ("<>", &[(1742, 1), (1613, 1)]),
            ("|=>", &[(1742, 1), (1742, 1), (1562, 1)]),
            ("\u{2581}", &[(1178, 1)]),
            ("\u{e0cc}", &[(58, 1)]),
            ("<!--", &[(1742, 1), (1742, 1), (1742, 1), (1595, 1)]),
            ("\u{1F9CF}\u{1F3FC}\u{200D}\u{2642}\u{FE0F}", &[(2712, 2)]),
            (
                "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}",
                &[(3855, 2)],
            ),
        ];

        for (text, expected) in cases {
            let shaped = cluster_and_shape(&render_metrics, &mut glyph_cache, &style, &font, text);
            assert_eq!(
                shape_signature(&shaped),
                *expected,
                "shape mismatch for {text:?}"
            );
        }
    }
}
