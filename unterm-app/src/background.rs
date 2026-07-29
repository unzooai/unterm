//! A picture behind the terminal.
//!
//! Read from the config the previous front end used, under the same names:
//! `window_background_image` for the file and `window_background_opacity` for
//! how much of it shows through.
//!
//! Scaled to cover the window, keeping its proportions and cropping whatever
//! does not fit. The alternative -- stretching to fit -- makes every photograph
//! wrong in a way people notice without being able to say why, and letterboxing
//! puts bars of a different colour down the sides of a terminal.
//!
//! Opacity is not a preference here so much as a requirement: a picture at full
//! strength is a picture you cannot read text on, and the whole point of the
//! feature is to still be a terminal afterwards.

/// The most of a picture that will be shown.
///
/// A background is behind text, and text has to win. Past this the terminal
/// stops being readable, and somebody who wanted a picture at full strength
/// wanted a wallpaper.
const MAX_OPACITY: f32 = 0.5;

/// A loaded picture.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub opacity: f32,
}

/// Load whatever the config names, if it names anything.
pub fn configured(config: &unterm_engine::next_core::config::Config) -> Option<Image> {
    let path = config
        .str_of("window_background_image")
        .ok()
        .flatten()
        .filter(|path| !path.trim().is_empty())?;
    let opacity = config
        .float_of("window_background_opacity")
        .ok()
        .flatten()
        .map(|value| value as f32)
        .unwrap_or(0.25);
    load(std::path::Path::new(&path), opacity)
}

/// Load a picture from disk.
///
/// A path that is not there, or is not a picture, is nothing rather than a
/// refusal to start: a config carried from another machine names files that
/// are not on this one, and a terminal that will not open is no way to say so.
pub fn load(path: &std::path::Path, opacity: f32) -> Option<Image> {
    match image::open(path) {
        Ok(loaded) => {
            let rgba = loaded.to_rgba8();
            Some(Image {
                width: rgba.width(),
                height: rgba.height(),
                rgba: rgba.into_raw(),
                opacity: clamp_opacity(opacity),
            })
        }
        Err(err) => {
            log::warn!("could not read the background image {path:?}: {err}");
            None
        }
    }
}

/// How much of the picture shows through.
pub fn clamp_opacity(opacity: f32) -> f32 {
    if !opacity.is_finite() {
        return 0.0;
    }
    opacity.clamp(0.0, MAX_OPACITY)
}

/// Which part of the picture fills a window of this shape.
///
/// Returns texture coordinates in 0..1: the middle of the picture, cropped on
/// whichever axis has more than the window needs. Cropping from the middle
/// rather than a corner because a photograph's subject is usually in it.
pub fn cover(image: (u32, u32), window: (f32, f32)) -> [f32; 4] {
    let (image_width, image_height) = (image.0.max(1) as f32, image.1.max(1) as f32);
    let (window_width, window_height) = (window.0.max(1.0), window.1.max(1.0));

    let image_ratio = image_width / image_height;
    let window_ratio = window_width / window_height;

    if image_ratio > window_ratio {
        // Wider than it needs to be: take a slice out of the middle.
        let wanted = window_ratio / image_ratio;
        let margin = (1.0 - wanted) / 2.0;
        [margin, 0.0, 1.0 - margin, 1.0]
    } else {
        let wanted = image_ratio / window_ratio;
        let margin = (1.0 - wanted) / 2.0;
        [0.0, margin, 1.0, 1.0 - margin]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text has to win. A picture at full strength is a picture you cannot
    /// read a terminal on, and the point of the feature is to still be one.
    #[test]
    fn opacity_never_reaches_the_point_where_text_is_lost() {
        assert_eq!(clamp_opacity(1.0), MAX_OPACITY);
        assert_eq!(clamp_opacity(5.0), MAX_OPACITY);
        assert_eq!(clamp_opacity(0.2), 0.2);
        assert_eq!(clamp_opacity(-1.0), 0.0);
        assert_eq!(clamp_opacity(f32::NAN), 0.0);
    }

    /// A picture the same shape as the window is used whole.
    #[test]
    fn a_picture_that_already_fits_is_not_cropped() {
        let uv = cover((1600, 900), (800.0, 450.0));
        assert!((uv[0] - 0.0).abs() < 1e-5, "{uv:?}");
        assert!((uv[1] - 0.0).abs() < 1e-5, "{uv:?}");
        assert!((uv[2] - 1.0).abs() < 1e-5, "{uv:?}");
        assert!((uv[3] - 1.0).abs() < 1e-5, "{uv:?}");
    }

    /// A wide picture in a tall window loses its sides, not its top: stretching
    /// instead is what makes every photograph look subtly wrong.
    #[test]
    fn a_wide_picture_is_cropped_at_the_sides() {
        let uv = cover((2000, 500), (500.0, 500.0));
        assert!(uv[0] > 0.0 && uv[2] < 1.0, "the sides were kept: {uv:?}");
        assert!((uv[1] - 0.0).abs() < 1e-5, "the top was cropped: {uv:?}");
        assert!((uv[3] - 1.0).abs() < 1e-5, "the bottom was cropped: {uv:?}");
    }

    #[test]
    fn a_tall_picture_is_cropped_at_the_top_and_bottom() {
        let uv = cover((500, 2000), (500.0, 500.0));
        assert!(uv[1] > 0.0 && uv[3] < 1.0, "nothing was cropped: {uv:?}");
        assert!((uv[0] - 0.0).abs() < 1e-5, "{uv:?}");
        assert!((uv[2] - 1.0).abs() < 1e-5, "{uv:?}");
    }

    /// From the middle, because a photograph's subject is usually in it.
    #[test]
    fn cropping_takes_from_the_middle() {
        let uv = cover((2000, 500), (500.0, 500.0));
        let left = uv[0];
        let right = 1.0 - uv[2];
        assert!((left - right).abs() < 1e-5, "lopsided: {uv:?}");
    }

    /// Nothing here divides by a window that has not been laid out yet.
    #[test]
    fn a_window_with_no_size_yet_does_not_divide_by_it() {
        for uv in [
            cover((100, 100), (0.0, 0.0)),
            cover((0, 0), (100.0, 100.0)),
            cover((0, 100), (100.0, 0.0)),
        ] {
            assert!(uv.iter().all(|value| value.is_finite()), "{uv:?}");
            assert!(uv.iter().all(|value| (0.0..=1.0).contains(value)), "{uv:?}");
        }
    }

    /// A file that is not there is nothing, not a refusal to start: a config
    /// carried from another machine names files that are not on this one.
    #[test]
    fn a_missing_file_is_nothing_rather_than_a_failure() {
        let missing = std::path::Path::new("no-such-picture-anywhere.png");
        assert!(load(missing, 0.2).is_none());
    }

    /// And a real one loads with its own size.
    #[test]
    fn a_real_picture_loads() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("背景.png");
        let mut pixels = image::RgbaImage::new(4, 2);
        for pixel in pixels.pixels_mut() {
            *pixel = image::Rgba([10, 20, 30, 255]);
        }
        pixels.save(&path).expect("write a picture");

        let loaded = load(&path, 0.9).expect("a picture that exists loads");
        assert_eq!((loaded.width, loaded.height), (4, 2));
        assert_eq!(loaded.rgba.len(), 4 * 2 * 4);
        assert_eq!(loaded.opacity, MAX_OPACITY, "opacity was not clamped");
    }
}
