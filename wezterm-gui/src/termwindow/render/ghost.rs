//! Render the per-pane ghost-text overlay — fish-style grey
//! "predictive completion" drawn directly to the right of the
//! cursor. Lives in its own paint pass so it can run *after* the
//! pane finishes drawing (and therefore sit above the pane's own
//! cells in z-order) without touching `paint_pane`'s already-busy
//! render path.

use crate::quad::TripleLayerQuadAllocator;
use crate::termwindow::render::RenderScreenLineParams;
use mux::renderable::RenderableDimensions;
use mux::tab::PositionedPane;
use termwiz::cell::CellAttributes;
use termwiz::color::SrgbaTuple;
use termwiz::surface::line::Line;
use wezterm_term::color::ColorAttribute;
use window::color::LinearRgba;

impl crate::TermWindow {
    /// Draw the active pane's pending ghost-text prediction (if any)
    /// as dim grey characters immediately to the right of the
    /// cursor. No-op when:
    ///   * no pane is active,
    ///   * the active pane has no ghost prediction queued,
    ///   * the cursor is off-screen (scrolled away from live data).
    pub fn paint_ghost_text(
        &mut self,
        layers: &mut TripleLayerQuadAllocator,
        positioned: &[PositionedPane],
    ) -> anyhow::Result<()> {
        // Find the active positioned pane. We need its split
        // offsets to translate cell coords → window pixels;
        // `get_active_pane_or_overlay` alone doesn't give us that.
        let Some(pos) = positioned.iter().find(|p| p.is_active) else {
            return Ok(());
        };

        let pane_id = pos.pane.pane_id() as u64;
        let Some((prefix, ghost)) = unterm_services::ghost_text::current_ghost(pane_id) else {
            return Ok(());
        };
        if ghost.is_empty() {
            return Ok(());
        }

        let cursor = pos.pane.get_cursor_position();

        // If the shell publishes OSC 133 semantic zones, suppress the
        // ghost only when the cursor clearly sits inside an `Output`
        // zone — that's the TUI / command-output case where an overlay
        // would be visually wrong. We deliberately do NOT require the
        // cursor to be inside an `Input` zone: shell integrations are
        // wildly inconsistent about marking input (p10k and several
        // oh-my-zsh setups emit Prompt/Output but never close or even
        // open the Input zone), and the old require-Input gate made the
        // whole feature silently dead on exactly those machines while
        // working fine on shells with no OSC 133 at all ("works on some
        // computers, not others"). Prompt zones, unmarked cells, and
        // empty zone lists all show the ghost.
        if let Ok(zones) = pos.pane.get_semantic_zones() {
            if !zones.is_empty() {
                let containing = zones
                    .iter()
                    .rev()
                    .find(|z| zone_contains_cell(z, cursor.x as isize, cursor.y));
                if matches!(
                    containing.map(|z| z.semantic_type),
                    Some(wezterm_term::SemanticType::Output)
                ) {
                    return Ok(());
                }
            }
        }
        let dims = pos.pane.get_dimensions();
        let viewport_top = self
            .get_viewport(pos.pane.pane_id())
            .unwrap_or(dims.physical_top);

        // Cursor screen row (relative to viewport top). If the user
        // has scrolled the viewport away from live data, the cursor
        // row may not be visible — in that case there's nowhere
        // sensible to draw the ghost.
        let cursor_screen_row = cursor.y - viewport_top;
        if cursor_screen_row < 0 || cursor_screen_row >= pos.height as isize {
            return Ok(());
        }

        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;

        // Pane top-left pixel within the window, shared with `paint_pane` so
        // the overlay registers with the underlying pane characters.
        let (pane_left_pixel, pane_top_pixel) = self.pane_origin_pixels(pos);

        // Truncate the ghost so it doesn't visually wrap past the
        // pane's right edge — drawing past the edge would either
        // get clipped by the window or, worse, overflow into a
        // neighbouring pane.
        let cursor_pixel_x = pane_left_pixel + cursor.x as f32 * cell_width;
        let cursor_pixel_y = pane_top_pixel + cursor_screen_row as f32 * cell_height;
        let pane_right_pixel = pane_left_pixel + pos.pixel_width as f32;
        let max_ghost_pixels = (pane_right_pixel - cursor_pixel_x).max(0.0);
        if max_ghost_pixels < cell_width {
            return Ok(());
        }
        let max_ghost_cols = (max_ghost_pixels / cell_width).floor() as usize;
        let truncated = truncate_ghost(&ghost, max_ghost_cols);
        if truncated.is_empty() {
            return Ok(());
        }

        // Dim grey foreground. We don't paint a background — the
        // overlay sits on top of whatever the pane already drew
        // (typically blank cells), so transparency reads naturally.
        let (grey_r, grey_g, grey_b) = ghost_text_color(self.config.window_background_opacity);
        let mut attrs = CellAttributes::blank();
        attrs.set_foreground(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
            grey_r, grey_g, grey_b, 1.0,
        )));
        attrs.set_italic(true);
        let line = Line::from_text(&truncated, &attrs, 0, None);

        let palette = self.palette().clone();
        let window_is_transparent =
            !self.window_background.is_empty() || self.config.window_background_opacity != 1.0;
        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();
        let fg = LinearRgba::with_components(grey_r, grey_g, grey_b, 1.0);
        let default_bg = LinearRgba::with_components(0., 0., 0., 0.);

        // We render starting at the cursor's *cell column*, so the
        // first ghost character lines up perfectly with where the
        // next user keystroke would land.
        let ghost_cols = truncated.chars().count().max(1);
        let pixel_width_for_ghost = ghost_cols as f32 * cell_width;

        // Suppress the unused-variable lint for `prefix` — it's
        // captured purely as documentation here; the actual cursor
        // alignment is done via `cursor.x` above, not the prefix
        // length, so the prefix is debug context, not arithmetic.
        let _ = prefix;

        self.render_screen_line(
            RenderScreenLineParams {
                top_pixel_y: cursor_pixel_y,
                left_pixel_x: cursor_pixel_x,
                pixel_width: pixel_width_for_ghost,
                stable_line_idx: None,
                line: &line,
                selection: 0..0,
                cursor: &Default::default(),
                palette: &palette,
                dims: &RenderableDimensions {
                    cols: ghost_cols,
                    physical_top: 0,
                    scrollback_rows: 0,
                    scrollback_top: 0,
                    viewport_rows: 1,
                    dpi: self.terminal_size.dpi,
                    pixel_height: cell_height as usize,
                    pixel_width: pixel_width_for_ghost as usize,
                    reverse_video: false,
                },
                config: &self.config,
                cursor_border_color: LinearRgba::default(),
                foreground: fg,
                pane: None,
                is_active: true,
                selection_fg: LinearRgba::default(),
                selection_bg: LinearRgba::default(),
                cursor_fg: LinearRgba::default(),
                cursor_bg: LinearRgba::default(),
                cursor_is_default_color: true,
                white_space,
                filled_box,
                window_is_transparent,
                default_bg,
                style: None,
                font: None,
                use_pixel_positioning: self.config.experimental_pixel_positioning,
                render_metrics: self.render_metrics,
                shape_key: None,
                password_input: false,
            },
            layers,
        )?;

        Ok(())
    }
}

/// True when (x, y) — cell column, stable row — falls inside `zone`.
/// SemanticZone bounds are inclusive on `start_*` and exclusive at
/// the end of the LAST row, so we treat the zone as a flat range of
/// rows: anything between `start_y..=end_y` is inside vertically;
/// horizontally we accept any column on intermediate rows and bound
/// the start/end rows by their respective x offsets.
fn zone_contains_cell(
    zone: &wezterm_term::SemanticZone,
    x: isize,
    y: wezterm_term::StableRowIndex,
) -> bool {
    if y < zone.start_y || y > zone.end_y {
        return false;
    }
    if y == zone.start_y && x < zone.start_x as isize {
        return false;
    }
    if y == zone.end_y && x > zone.end_x as isize {
        return false;
    }
    true
}

/// Truncate ghost text so its column width fits in `max_cols`.
/// Returns the leading portion that fits (no ellipsis added — a
/// ghost is *predicted text*, an ellipsis would suggest "and more"
/// where the more is exactly what the user is about to type).
fn truncate_ghost(text: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let total = text.chars().count();
    if total <= max_cols {
        return text.to_string();
    }
    text.chars().take(max_cols).collect()
}

/// Foreground RGB for ghost text. Returns float channels in [0, 1].
/// We bias darker on opaque windows (so the ghost reads as dim grey)
/// and slightly brighter on transparent windows (where dark grey
/// would just disappear into the desktop background).
fn ghost_text_color(window_opacity: f32) -> (f32, f32, f32) {
    if window_opacity < 1.0 {
        (0.65, 0.65, 0.70)
    } else {
        (0.50, 0.50, 0.55)
    }
}
