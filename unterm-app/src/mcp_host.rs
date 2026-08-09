//! What this front end answers for the MCP surface.
//!
//! The surface itself is engine-neutral -- it reads screens and drives
//! sessions through next-core, which needs no window. Two things it cannot do
//! for itself: draw a pane's scrollback, which needs a font stack, and say
//! what the keys do, which is a front end's own business. Both come through
//! here, which is how this app gets the whole agent-facing API without the
//! surface knowing it exists.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use unterm_engine::next_core::color::{palette_rgb, Rgb};
use unterm_engine::next_core::font_raster::RasterizedGlyph;
use unterm_engine::{
    McpHost, ScrollbackTextRequest, StyledCell, StyledColor, StyledScreenLine,
};

pub struct AppMcpHost;

pub fn install() {
    unterm_engine::set_mcp_host(&AppMcpHost);
}

/// How a pane was asked to be split, until the window has arranged it.
///
/// The split arrives on the MCP server's thread and the arrangement belongs to
/// the one drawing; this is the note left between them. Read once and removed,
/// so a pane closed and its id reused cannot inherit an old answer.
/// The window, for the threads that are not the one drawing it.
///
/// `instance.focus` arrives on the MCP server's thread and has to reach a
/// window the event loop owns. A winit `Window` is shareable, so the handle
/// is kept here rather than a message being posted through the event loop --
/// one fewer moving part on a path an agent calls to say "look at this".
static WINDOW: std::sync::OnceLock<std::sync::Arc<winit::window::Window>> =
    std::sync::OnceLock::new();

/// Called once, when the window exists.
pub fn remember_window(window: std::sync::Arc<winit::window::Window>) {
    let _ = WINDOW.set(window);
}

/// Ask the window to paint, from any thread. A no-op until the window
/// exists; whoever calls before then loses nothing, because the first
/// frame is on its way regardless.
pub fn request_repaint() {
    if let Some(window) = WINDOW.get() {
        window.request_redraw();
    }
}

/// The default colours a cell falls back to when it names none.
const DEFAULT_FG: Rgb = Rgb {
    red: 0xd0,
    green: 0xd0,
    blue: 0xd0,
};
const DEFAULT_BG: Rgb = Rgb {
    red: 0x10,
    green: 0x10,
    blue: 0x10,
};

/// How large a glyph is drawn. Independent of the window's size: a capture is
/// for reading later, not for matching what is on screen right now.
const BASE_PIXEL_SIZE: u32 = 16;
/// The dpi an unspecified request means, matching the surface's own default.
const BASE_DPI: usize = 96;

impl McpHost for AppMcpHost {
    /// This app creates its own window and closes it, so nothing here is
    /// hosted by anyone else.
    fn window_identity(&self) -> unterm_engine::WindowIdentity {
        unterm_engine::WindowIdentity {
            engine: "unterm-app",
            window_owner: "unterm_app",
            native_window_lifecycle: "self_owned",
            uses_host_window: false,
        }
    }

    /// Name the window, so an agent's chosen name is what a person sees in
    /// the taskbar and Alt-Tab -- not only what `instance.list` reports.
    fn set_window_title(&self, title: Option<&str>) -> bool {
        let Some(window) = WINDOW.get() else {
            return false;
        };
        window.set_title(&match title {
            Some(title) => format!("{title} — Unterm"),
            None => "Unterm".to_string(),
        });
        true
    }

    /// Bring the window to the front.
    ///
    /// Minimised windows are restored first: an agent asking for attention
    /// means the window should be visible, and raising one that is minimised
    /// otherwise does nothing at all.
    fn focus_window(&self) -> Result<()> {
        let window = WINDOW.get().context("the window is not open yet")?;
        window.set_minimized(false);
        window.focus_window();
        Ok(())
    }

    fn request_repaint(&self) {
        request_repaint();
    }

    /// This window is the one drawing the queue, so the question goes
    /// straight onto it. Blocks the asking thread until a key resolves
    /// it or it times out -- which is what an unanswered question must
    /// do, or an agent would write before anyone had said yes.
    fn ask_confirmation(&self, request: &Value) -> Result<Value> {
        // The banner is painted by the frame loop, and a resting window
        // schedules no frames: the question sat registered but invisible
        // for its whole timeout, and every agent write died "unanswered"
        // with nothing on screen to answer. Ask for the frame that shows
        // it, and for the one that clears it.
        request_repaint();
        let outcome = unterm_mcp::handler::prompt_locally(request);
        request_repaint();
        match outcome {
            Some(decision) => Ok(json!(decision.to_wire())),
            None => anyhow::bail!("the confirmation went unanswered"),
        }
    }

    /// A rectangle of the desktop.
    fn capture_region(
        &self,
        left: i32,
        top: i32,
        width: usize,
        height: usize,
        include_base64: bool,
    ) -> Result<Value> {
        let region = unterm_services::window_capture::Region {
            left,
            top,
            width,
            height,
        };
        let shot = unterm_services::window_capture::capture_region(region)?;
        write_capture(shot, include_base64)
    }

    /// A picture of the terminal's own window.
    ///
    /// Text is what an agent usually wants, and `screen.text` is cheaper. This
    /// is for the rest of it: which pane has focus, whether the tab bar is
    /// showing, what a selection looks like, whether anything is drawn wrong.
    /// Reading the screen cannot answer any of those.
    fn capture_own_window(
        &self,
        title: Option<&str>,
        pid: Option<u32>,
        include_base64: bool,
    ) -> Result<Value> {
        // Neither filter given means "the window you host", and here is
        // the only place that phrase has an answer: this is the process
        // the window belongs to, whether the surface asking lives here
        // or in a Core.
        let pid = match (title, pid) {
            (None, None) => Some(std::process::id()),
            (_, given) => given,
        };
        let shot = unterm_services::window_capture::capture_window(title, pid)?;
        write_capture(shot, include_base64)
    }

    fn render_scrollback_png(
        &self,
        pane_id: Option<usize>,
        path: &std::path::Path,
        max_rows: usize,
        dpi: usize,
    ) -> Result<Value> {
        let engine = unterm_engine::host_engine();
        let pane_id = match pane_id {
            Some(pane_id) => pane_id,
            None => unterm_engine::SessionEngine::list_sessions(&*engine)?
                .into_iter()
                .find(|session| session.is_active)
                .map(|session| session.id)
                .context("no session is active to capture")?,
        };

        let scrollback = engine.read_styled_scrollback(
            pane_id,
            ScrollbackTextRequest {
                start_line: None,
                end_line: None,
                tail_lines: Some(max_rows.max(1) as i64),
                // Styling comes from the styled cells, not from escapes left
                // in the text.
                escapes: false,
            },
        )?;

        // dpi scales the glyphs, so a capture meant for reading can be asked
        // for at a larger size without the caller doing arithmetic.
        let dpi = if dpi == 0 { BASE_DPI } else { dpi };
        let pixel_size = ((BASE_PIXEL_SIZE as usize * dpi) / BASE_DPI).max(6) as u32;
        let mut fonts = crate::fonts::FontStack::system(pixel_size)
            .context("no font to draw the capture with")?;

        let image = draw(&mut fonts, &scrollback.lines, scrollback.cols, pixel_size);
        image
            .save_with_format(path, image::ImageFormat::Png)
            .with_context(|| format!("write the capture to {}", path.display()))?;

        Ok(json!({
            "path": path.display().to_string(),
            "width": image.width(),
            "height": image.height(),
            "rows": scrollback.lines.len(),
            "cols": scrollback.cols,
            "truncated": scrollback.row_count as usize > scrollback.lines.len(),
            "first_row": scrollback.first_row,
            "session_id": pane_id.to_string(),
            "renderer": {
                "engine": "next-core",
                "renderer": "unterm-app-freetype",
                "uses_wezterm_pane": false,
                "standalone": true,
                "palette": "xterm-256",
                "supported_styles": ["fg", "bg", "bold", "inverse", "theme_palette"],
            },
        }))
    }

    fn key_assignments(&self) -> Vec<Value> {
        // The folded table, so a `[keys]` override is what the agent sees.
        crate::keys::effective_bindings()
            .iter()
            .map(|binding| {
                json!({
                    "table": "default",
                    "key": binding.trigger.name(),
                    "mods": binding.mods.name(),
                    "action": binding.action.name(),
                })
            })
            .collect()
    }
}

/// Draw the lines into an image, one cell per glyph slot.
///
/// A monospace grid, not a shaped run: a capture wants every column where the
/// terminal put it, which is the same reason the live renderer places cells
/// rather than laying out text.
/// The same standalone scrollback render the MCP method performs, reachable
/// from the quick menu: the pane's history to one tall PNG, independent of
/// what the window shows or covers.
pub fn scrollback_png(pane_id: usize, path: &std::path::Path) -> Result<(u32, u32)> {
    let engine = unterm_engine::host_engine();
    let scrollback = engine.read_styled_scrollback(
        pane_id,
        ScrollbackTextRequest {
            start_line: None,
            end_line: None,
            tail_lines: Some(10_000),
            escapes: false,
        },
    )?;
    let mut fonts = crate::fonts::FontStack::system(BASE_PIXEL_SIZE)
        .context("no font to draw the capture with")?;
    let image = draw(
        &mut fonts,
        &scrollback.lines,
        scrollback.cols,
        BASE_PIXEL_SIZE,
    );
    image
        .save_with_format(path, image::ImageFormat::Png)
        .with_context(|| format!("write the capture to {}", path.display()))?;
    Ok((image.width(), image.height()))
}

fn draw(
    fonts: &mut crate::fonts::FontStack,
    lines: &[StyledScreenLine],
    cols: usize,
    pixel_size: u32,
) -> image::RgbaImage {
    // Advance from the font rather than assuming: a font whose digits are
    // narrower than its box would otherwise overlap or leave gaps.
    let cell_width = fonts
        .rasterize('M')
        .map(|(_, glyph)| glyph.advance_x.max(1) as u32)
        .unwrap_or(pixel_size / 2)
        .max(1);
    let cell_height = (pixel_size as f32 * 1.2).ceil() as u32;
    let baseline = (pixel_size as f32 * 0.95).round() as i32;

    let cols = cols.max(1) as u32;
    let rows = lines.len().max(1) as u32;
    let mut image = image::RgbaImage::from_pixel(
        cols * cell_width,
        rows * cell_height,
        image::Rgba([DEFAULT_BG.red, DEFAULT_BG.green, DEFAULT_BG.blue, 0xff]),
    );

    for (row, line) in lines.iter().enumerate() {
        let mut column = 0u32;
        for cell in &line.cells {
            if column >= cols {
                break;
            }
            let (fg, bg) = colors(cell);
            let width = cell.width.max(1) as u32;

            fill(
                &mut image,
                column * cell_width,
                row as u32 * cell_height,
                width * cell_width,
                cell_height,
                bg,
            );
            if !cell.ch.is_whitespace() {
                if let Some((_, glyph)) = fonts.rasterize(cell.ch) {
                    blend_glyph(
                        &mut image,
                        &glyph,
                        (column * cell_width) as i32,
                        (row as u32 * cell_height) as i32 + baseline,
                        fg,
                    );
                }
            }
            column += width;
        }
    }
    image
}

/// The colours a cell is drawn in, inverse already applied.
fn colors(cell: &StyledCell) -> (Rgb, Rgb) {
    let resolve = |color: Option<StyledColor>, fallback: Rgb| match color {
        Some(StyledColor::Rgb(red, green, blue)) => Rgb::new(red, green, blue),
        Some(StyledColor::Palette(index)) => palette_rgb(index),
        None => fallback,
    };
    let mut fg = resolve(cell.style.fg, DEFAULT_FG);
    let mut bg = resolve(cell.style.bg, DEFAULT_BG);
    if cell.style.inverse {
        std::mem::swap(&mut fg, &mut bg);
    }
    if cell.style.bold {
        fg = fg.lighten(0.3);
    }
    if cell.style.faint {
        fg = fg.darken(0.4);
    }
    (fg, bg)
}

fn fill(image: &mut image::RgbaImage, x: u32, y: u32, width: u32, height: u32, color: Rgb) {
    let pixel = image::Rgba([color.red, color.green, color.blue, 0xff]);
    for py in y..(y + height).min(image.height()) {
        for px in x..(x + width).min(image.width()) {
            image.put_pixel(px, py, pixel);
        }
    }
}

/// Composite a glyph's coverage over what is already there.
fn blend_glyph(
    image: &mut image::RgbaImage,
    glyph: &RasterizedGlyph,
    pen_x: i32,
    baseline_y: i32,
    color: Rgb,
) {
    for gy in 0..glyph.height {
        let py = baseline_y - glyph.bearing_y + gy as i32;
        if py < 0 || py as u32 >= image.height() {
            continue;
        }
        for gx in 0..glyph.width {
            let px = pen_x + glyph.bearing_x + gx as i32;
            if px < 0 || px as u32 >= image.width() {
                continue;
            }
            let coverage = glyph.coverage[gy * glyph.width + gx];
            if coverage == 0 {
                continue;
            }
            let under = *image.get_pixel(px as u32, py as u32);
            let mix = |over: u8, under: u8| {
                ((over as u32 * coverage as u32 + under as u32 * (255 - coverage as u32)) / 255)
                    as u8
            };
            image.put_pixel(
                px as u32,
                py as u32,
                image::Rgba([
                    mix(color.red, under[0]),
                    mix(color.green, under[1]),
                    mix(color.blue, under[2]),
                    0xff,
                ]),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unterm_engine::CellStyle;

    fn line(text: &str, style: CellStyle) -> StyledScreenLine {
        StyledScreenLine {
            row: 0,
            wrapped: false,
            cells: text
                .chars()
                .map(|ch| StyledCell {
                    ch,
                    style: style.clone(),
                    width: 1,
                })
                .collect(),
        }
    }

    #[test]
    fn a_drawn_line_is_the_size_of_its_grid() {
        let mut fonts = match crate::fonts::FontStack::system(16) {
            Some(fonts) => fonts,
            None => return,
        };
        let image = draw(&mut fonts, &[line("hello", CellStyle::default())], 5, 16);
        assert_eq!(image.height() % 1, 0);
        assert!(image.width() > 0 && image.height() > 0);
        assert_eq!(image.width() % 5, 0, "five columns of equal width");
    }

    #[test]
    fn text_is_drawn_rather_than_left_as_background() {
        let mut fonts = match crate::fonts::FontStack::system(16) {
            Some(fonts) => fonts,
            None => return,
        };
        let blank = draw(&mut fonts, &[line("    ", CellStyle::default())], 4, 16);
        let text = draw(&mut fonts, &[line("MMMM", CellStyle::default())], 4, 16);
        assert_ne!(
            blank.as_raw(),
            text.as_raw(),
            "drawing letters must change the pixels"
        );
    }

    #[test]
    fn inverse_swaps_the_two_colours() {
        let mut style = CellStyle::default();
        style.fg = Some(StyledColor::Rgb(1, 2, 3));
        style.bg = Some(StyledColor::Rgb(4, 5, 6));
        let plain = colors(&StyledCell {
            ch: 'x',
            style: style.clone(),
            width: 1,
        });
        style.inverse = true;
        let inverted = colors(&StyledCell {
            ch: 'x',
            style,
            width: 1,
        });
        assert_eq!(plain.0, inverted.1);
        assert_eq!(plain.1, inverted.0);
    }

    #[test]
    fn the_keys_reported_are_the_keys_that_act() {
        let reported = AppMcpHost.key_assignments();
        assert_eq!(reported.len(), crate::keys::BINDINGS.len());
        for (value, binding) in reported.iter().zip(crate::keys::BINDINGS) {
            assert_eq!(value["action"], binding.action.name());
            assert_eq!(value["key"], binding.trigger.name());
        }
    }
}

/// Where a capture is written.
///
/// Under the user's own unterm directory rather than the system temp: these
/// are files a person may want to look at again, and a name that says what it
/// is beats one that says `tmp8f2a`.
fn capture_path(kind: &str, extension: &str) -> std::path::PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis())
        .unwrap_or_default();
    let directory = unterm_protocol::state_dir()
        .unwrap_or_else(|| std::env::temp_dir().join(".unterm"))
        .join("captures");
    directory.join(format!("{kind}-{stamp}.{extension}"))
}

/// Write a capture out and describe it.
///
/// One place, so a window capture and a region capture cannot end up in
/// different directories or answer with differently-shaped JSON.
fn write_capture(
    shot: unterm_services::window_capture::WindowImage,
    include_base64: bool,
) -> Result<Value> {
    use base64::Engine as _;

    let buffer = image::RgbaImage::from_raw(shot.width as u32, shot.height as u32, shot.pixels)
        .context("the capture's size and its pixels disagree")?;

    let path = capture_path(shot.mode, "png");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("make {}", parent.display()))?;
    }
    buffer
        .save_with_format(&path, image::ImageFormat::Png)
        .with_context(|| format!("write the capture to {}", path.display()))?;

    let mut reply = json!({
        "path": path.display().to_string(),
        "width": shot.width,
        "height": shot.height,
        // How it was taken. The ways are not equivalent: a screen copy also
        // sees whatever is on top of the window.
        "mode": shot.mode,
    });
    if include_base64 {
        let bytes =
            std::fs::read(&path).with_context(|| format!("read back {}", path.display()))?;
        reply["base64"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
    }
    Ok(reply)
}


/// Serves the Core's reverse-channel calls from this window.
///
/// One translation layer, not a second implementation: every method maps
/// straight onto `AppMcpHost`, so a window answering a remote Core does
/// exactly what it does answering an in-process one. The wire format is
/// raw JSON because `unterm-core` sits below the MCP surface and must
/// not depend on its types.
pub struct AppHostResponder;

impl unterm_core::HostResponder for AppHostResponder {
    fn respond(&self, method: &str, params: &Value) -> Result<Value> {
        let text = |key: &str| params.get(key).and_then(|value| value.as_str());
        let number = |key: &str| params.get(key).and_then(|value| value.as_i64());
        let flag = |key: &str| {
            params
                .get(key)
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        };
        match method {
            "window_identity" => {
                let identity = AppMcpHost.window_identity();
                Ok(json!({
                    "engine": identity.engine,
                    "window_owner": identity.window_owner,
                    "native_window_lifecycle": identity.native_window_lifecycle,
                    "uses_host_window": identity.uses_host_window,
                }))
            }
            "request_repaint" => {
                AppMcpHost.request_repaint();
                Ok(Value::Null)
            }
            "set_window_title" => Ok(json!(AppMcpHost.set_window_title(text("title")))),
            "focus_window" => AppMcpHost.focus_window().map(|_| Value::Null),
            // Same call the in-process host makes, so a question from a
            // remote Core lands on this window's queue exactly as a
            // local one does -- the UI drawing it cannot tell them apart.
            "ask_confirmation" => AppMcpHost.ask_confirmation(params),
            "key_assignments" => Ok(json!(AppMcpHost.key_assignments())),
            "capture_region" => AppMcpHost.capture_region(
                number("left").unwrap_or(0) as i32,
                number("top").unwrap_or(0) as i32,
                number("width").unwrap_or(0).max(0) as usize,
                number("height").unwrap_or(0).max(0) as usize,
                flag("include_base64"),
            ),
            "capture_own_window" => AppMcpHost.capture_own_window(
                text("title"),
                number("pid").map(|pid| pid as u32),
                flag("include_base64"),
            ),
            "capture_external_window" => AppMcpHost.capture_external_window(params),
            "render_scrollback_png" => {
                let path = text("path").context("render_scrollback_png needs a path")?;
                AppMcpHost.render_scrollback_png(
                    number("pane_id").map(|pane| pane as usize),
                    std::path::Path::new(path),
                    number("max_rows").unwrap_or(0).max(0) as usize,
                    number("dpi").unwrap_or(0).max(0) as usize,
                )
            }
            // A Core newer than this window will ask for things it has
            // never heard of. Saying so is the whole contract; the Core
            // degrades from the answer.
            other => anyhow::bail!("this Unterm window does not implement {other}"),
        }
    }
}
