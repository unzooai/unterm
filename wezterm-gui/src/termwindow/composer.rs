//! Composer + Prompt Queue (roadmap mid-term).
//!
//! A modal overlay (toggled by Ctrl+Shift+J or the command palette) that lets
//! the user draft prompts and stack them into an ordered queue, then dispatch
//! them to the active pane one at a time. Each queued prompt is written to the
//! pane's PTY followed by Enter; the runner then waits until the pane goes
//! idle — no change in the pane's sequence number for a short debounce window —
//! before sending the next prompt. A Stop halts an in-flight run.
//!
//! The queue itself lives on `TermWindow` (see `ComposerState`) so it survives
//! closing and re-opening the overlay within a session; this modal is a thin
//! view over that state plus a computed-element cache. All the enqueue/run/idle
//! logic lives in `TermWindow` (the `composer_*` methods in `mod.rs`) because
//! the run loop needs `&mut TermWindow` to reach the mux and to reschedule
//! itself via the window's notify/apply channel.

use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::{DimensionContext, TermWindow};
use crate::utilsprites::RenderMetrics;
use config::Dimension;
use mux::pane::PaneId;
use std::cell::{Ref, RefCell};
use std::time::Instant;
use termwiz::cell::unicode_column_width;
use termwiz::surface::SequenceNo;
use wezterm_term::{KeyCode, KeyModifiers, MouseEvent};
use window::color::LinearRgba;

/// How often the idle-poll timer fires while a run is in flight.
pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(120);
/// The pane must produce no new output for at least this long before we
/// consider the current prompt "done" and send the next one.
pub const IDLE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(600);
/// Minimum time to wait after sending a prompt before idle can be declared,
/// so we don't race ahead in the brief gap before an agent starts responding.
pub const MIN_GRACE: std::time::Duration = std::time::Duration::from_millis(350);

const MAX_ROW_COLS: usize = 64;

/// Live state for an in-flight run.
pub struct RunState {
    /// Bumped every time a run starts / stops; scheduled poll callbacks that
    /// carry a stale generation are ignored, so a Stop cleanly abandons them.
    pub generation: u64,
    pub pane_id: PaneId,
    pub last_seqno: SequenceNo,
    /// When the pane's seqno last changed (basis for the idle debounce).
    pub last_change: Instant,
    /// When the current prompt was written to the PTY.
    pub sent_at: Instant,
}

/// Composer/queue state, owned by `TermWindow` so it persists across opening
/// and closing the overlay within a session.
#[derive(Default)]
pub struct ComposerState {
    /// Ordered queue of prompts; front is next/currently-running.
    pub queue: Vec<String>,
    /// The prompt currently being typed (not yet enqueued).
    pub draft: String,
    /// Highlighted queue row (for removal).
    pub selected: usize,
    /// Present while a run is in flight.
    pub run: Option<RunState>,
    /// Monotonic run counter (see `RunState::generation`).
    pub generation: u64,
    /// Transient status line shown at the bottom of the overlay.
    pub status: Option<String>,
}

impl ComposerState {
    pub fn is_running(&self) -> bool {
        self.run.is_some()
    }
}

/// First line of a prompt, column-truncated for the queue list.
fn row_label(s: &str) -> String {
    let line = s.lines().next().unwrap_or("");
    if unicode_column_width(line, None) <= MAX_ROW_COLS {
        return line.to_string();
    }
    let mut out = String::new();
    let mut cols = 0usize;
    for ch in line.chars() {
        let w = unicode_column_width(&ch.to_string(), None);
        if cols + w > MAX_ROW_COLS.saturating_sub(1) {
            break;
        }
        out.push(ch);
        cols += w;
    }
    out.push('…');
    out
}

/// The overlay: a thin view over `TermWindow.composer`, plus a computed-element
/// cache. No queue data lives here.
pub struct Composer {
    element: RefCell<Option<Vec<ComputedElement>>>,
}

impl Composer {
    pub fn new() -> Self {
        Self {
            element: RefCell::new(None),
        }
    }

    fn compute(&self, term_window: &mut TermWindow) -> anyhow::Result<Vec<ComputedElement>> {
        let font = term_window
            .fonts
            .title_font()
            .expect("to resolve title font");
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let pt = term_window.dimensions.dpi as f32 / 72.0;

        let bg = LinearRgba::with_srgba(0x2a, 0x2a, 0x2a, 0xff);
        let fg = LinearRgba::with_srgba(0xf2, 0xf2, 0xf0, 0xff);
        let dim = LinearRgba::with_srgba(0xac, 0xac, 0xa8, 0xff);
        let teal = LinearRgba::with_srgba(0x6f, 0xcc, 0xb8, 0xff);
        let amber = LinearRgba::with_srgba(0xe6, 0xb4, 0x50, 0xff);
        let hover_bg = LinearRgba::with_srgba(0x3d, 0x3d, 0x3d, 0xff);
        let field_bg = LinearRgba::with_srgba(0x1c, 0x1c, 0x1c, 0xff);

        let state = term_window.composer.borrow();
        let running = state.is_running();

        let text_el = |s: String, color: LinearRgba| {
            Element::new(&font, ElementContent::Text(s)).colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: color.into(),
            })
        };

        let mut children: Vec<Element> = vec![];

        // Title.
        children.push(
            text_el("Composer — Prompt Queue".to_string(), fg)
                .display(DisplayType::Block)
                .padding(BoxDimension {
                    left: Dimension::Pixels(14. * pt),
                    right: Dimension::Pixels(14. * pt),
                    top: Dimension::Pixels(8. * pt),
                    bottom: Dimension::Pixels(4. * pt),
                }),
        );

        // Draft input field. Multi-line: one Block per line so embedded
        // newlines render; caret rides the last line.
        let draft_lines: Vec<&str> = if state.draft.is_empty() {
            vec![]
        } else {
            state.draft.split('\n').collect()
        };
        let mut field_kids: Vec<Element> = vec![];
        if draft_lines.is_empty() {
            field_kids.push(
                Element::new(
                    &font,
                    ElementContent::Children(vec![
                        text_el("Type a prompt…  ".to_string(), dim),
                        text_el("▏".to_string(), teal),
                    ]),
                )
                .display(DisplayType::Block),
            );
        } else {
            let last = draft_lines.len() - 1;
            for (i, line) in draft_lines.iter().enumerate() {
                let mut row: Vec<Element> = vec![text_el((*line).to_string(), fg)];
                if i == last {
                    row.push(text_el("▏".to_string(), teal));
                }
                field_kids.push(
                    Element::new(&font, ElementContent::Children(row)).display(DisplayType::Block),
                );
            }
        }
        let field = Element::new(&font, ElementContent::Children(field_kids))
            .colors(ElementColors {
                border: BorderColor::new(fg.mul_alpha(0.14)),
                bg: field_bg.into(),
                text: fg.into(),
            })
            .border(BoxDimension::new(Dimension::Pixels(1.)))
            .padding(BoxDimension {
                left: Dimension::Pixels(10. * pt),
                right: Dimension::Pixels(10. * pt),
                top: Dimension::Pixels(6. * pt),
                bottom: Dimension::Pixels(6. * pt),
            })
            .min_width(Some(Dimension::Percent(1.)))
            .display(DisplayType::Block);
        children.push(
            Element::new(&font, ElementContent::Children(vec![field]))
                .padding(BoxDimension {
                    left: Dimension::Pixels(10. * pt),
                    right: Dimension::Pixels(10. * pt),
                    top: Dimension::Pixels(2. * pt),
                    bottom: Dimension::Pixels(4. * pt),
                })
                .min_width(Some(Dimension::Percent(1.)))
                .display(DisplayType::Block),
        );

        children.push(
            text_el(
                "Enter: enqueue    Shift+Enter: newline".to_string(),
                dim,
            )
            .display(DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(14. * pt),
                right: Dimension::Pixels(14. * pt),
                top: Dimension::Pixels(0.),
                bottom: Dimension::Pixels(4. * pt),
            }),
        );

        // Queue list.
        if state.queue.is_empty() {
            children.push(
                text_el("Queue is empty.".to_string(), dim)
                    .display(DisplayType::Block)
                    .padding(BoxDimension {
                        left: Dimension::Pixels(14. * pt),
                        right: Dimension::Pixels(14. * pt),
                        top: Dimension::Pixels(6. * pt),
                        bottom: Dimension::Pixels(6. * pt),
                    }),
            );
        } else {
            for (i, prompt) in state.queue.iter().enumerate() {
                let is_current = running && i == 0;
                let selected = i == state.selected;
                let marker = if is_current { "▶ " } else { "  " };
                let label = format!("{marker}{}. {}", i + 1, row_label(prompt));
                let (row_bg, row_fg): (InheritableColor, InheritableColor) = if selected {
                    (hover_bg.into(), fg.into())
                } else {
                    (LinearRgba::TRANSPARENT.into(), fg.into())
                };
                let mut row_border = BorderColor::default();
                if selected {
                    row_border.left = teal;
                }
                if is_current {
                    row_border.left = amber;
                }
                children.push(
                    Element::new(
                        &font,
                        ElementContent::Text(label),
                    )
                    .colors(ElementColors {
                        border: row_border,
                        bg: row_bg,
                        text: if is_current { amber.into() } else { row_fg },
                    })
                    .border(BoxDimension {
                        left: Dimension::Pixels(2. * pt),
                        right: Dimension::Pixels(0.),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Pixels(12. * pt),
                        right: Dimension::Pixels(14. * pt),
                        top: Dimension::Pixels(4. * pt),
                        bottom: Dimension::Pixels(4. * pt),
                    })
                    .min_width(Some(Dimension::Percent(1.)))
                    .display(DisplayType::Block),
                );
            }
        }

        // Status line.
        if let Some(status) = &state.status {
            let color = if running { amber } else { teal };
            children.push(
                text_el(status.clone(), color)
                    .display(DisplayType::Block)
                    .padding(BoxDimension {
                        left: Dimension::Pixels(14. * pt),
                        right: Dimension::Pixels(14. * pt),
                        top: Dimension::Pixels(6. * pt),
                        bottom: Dimension::Pixels(2. * pt),
                    }),
            );
        }

        // Divider.
        children.push(
            Element::new(&font, ElementContent::Text(String::new()))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .line_height(Some(0.08))
                .margin(BoxDimension {
                    left: Dimension::Pixels(0.),
                    right: Dimension::Pixels(0.),
                    top: Dimension::Pixels(6. * pt),
                    bottom: Dimension::Pixels(0.),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(0.12).into(),
                    text: fg.into(),
                }),
        );

        // Footer hints.
        let hints = if running {
            "Running…   Ctrl+S / Esc: stop"
        } else {
            "Ctrl+Enter: run    Del: remove    Ctrl+K: clear all    Esc: close"
        };
        children.push(
            text_el(hints.to_string(), dim)
                .display(DisplayType::Block)
                .padding(BoxDimension {
                    left: Dimension::Pixels(14. * pt),
                    right: Dimension::Pixels(14. * pt),
                    top: Dimension::Pixels(6. * pt),
                    bottom: Dimension::Pixels(2. * pt),
                }),
        );

        drop(state);

        let card = Element::new(&font, ElementContent::Children(children))
            .item_type(crate::termwindow::UIItemType::PopupMenuCard)
            .colors(ElementColors {
                border: BorderColor::new(LinearRgba::with_srgba(0x00, 0x00, 0x00, 0xb0)),
                bg: bg.into(),
                text: fg.into(),
            })
            .padding(BoxDimension {
                left: Dimension::Pixels(0.),
                right: Dimension::Pixels(0.),
                top: Dimension::Pixels(6. * pt),
                bottom: Dimension::Pixels(6. * pt),
            })
            .border(BoxDimension::new(Dimension::Pixels(1.)))
            .border_corners(Some(Corners {
                top_left: SizedPoly {
                    width: Dimension::Pixels(6. * pt),
                    height: Dimension::Pixels(6. * pt),
                    poly: TOP_LEFT_ROUNDED_CORNER,
                },
                top_right: SizedPoly {
                    width: Dimension::Pixels(6. * pt),
                    height: Dimension::Pixels(6. * pt),
                    poly: TOP_RIGHT_ROUNDED_CORNER,
                },
                bottom_left: SizedPoly {
                    width: Dimension::Pixels(6. * pt),
                    height: Dimension::Pixels(6. * pt),
                    poly: BOTTOM_LEFT_ROUNDED_CORNER,
                },
                bottom_right: SizedPoly {
                    width: Dimension::Pixels(6. * pt),
                    height: Dimension::Pixels(6. * pt),
                    poly: BOTTOM_RIGHT_ROUNDED_CORNER,
                },
            }));

        let dimensions = term_window.dimensions;
        let border = term_window.get_os_border();
        let top_bar_height = if term_window.show_tab_bar && !term_window.config.tab_bar_at_bottom {
            term_window.tab_bar_pixel_height().unwrap_or(0.)
        } else {
            0.
        };
        let width = (520. * pt)
            .min(dimensions.pixel_width as f32 - 32. * pt)
            .round();
        let x = ((dimensions.pixel_width as f32 - width) / 2.).round();
        let y = (top_bar_height + border.top.get() as f32 + 28. * pt).round();

        let computed = term_window.compute_element(
            &LayoutContext {
                height: DimensionContext {
                    dpi: dimensions.dpi as f32,
                    pixel_max: dimensions.pixel_height as f32,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: DimensionContext {
                    dpi: dimensions.dpi as f32,
                    pixel_max: dimensions.pixel_width as f32,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(x, y, width, dimensions.pixel_height as f32 - y),
                metrics: &metrics,
                gl_state: term_window.render_state.as_ref().unwrap(),
                zindex: 100,
            },
            &card,
        )?;
        Ok(vec![computed])
    }
}

impl Modal for Composer {
    fn mouse_event(&self, _event: MouseEvent, _term_window: &mut TermWindow) -> anyhow::Result<()> {
        Ok(())
    }

    fn key_down(
        &self,
        key: KeyCode,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool> {
        use KeyModifiers as M;
        match (key, mods) {
            (KeyCode::Escape, _) => {
                if term_window.composer.borrow().is_running() {
                    term_window.composer_stop();
                } else {
                    term_window.cancel_modal();
                }
            }
            // Ctrl/Cmd+Enter runs the queue.
            (KeyCode::Enter, M::CTRL) | (KeyCode::Enter, M::SUPER) => {
                term_window.composer_run_start();
            }
            // Shift+Enter inserts a newline into the draft.
            (KeyCode::Enter, M::SHIFT) => {
                term_window.composer.borrow_mut().draft.push('\n');
                term_window.invalidate_composer();
            }
            // Enter enqueues the draft.
            (KeyCode::Enter, M::NONE) => {
                term_window.composer_enqueue();
            }
            (KeyCode::UpArrow, M::NONE) => term_window.composer_move_selection(-1),
            (KeyCode::DownArrow, M::NONE) => term_window.composer_move_selection(1),
            (KeyCode::Delete, _) => term_window.composer_remove_selected(),
            (KeyCode::Char('d'), M::CTRL) | (KeyCode::Char('D'), M::CTRL) => {
                term_window.composer_remove_selected()
            }
            (KeyCode::Char('k'), M::CTRL) | (KeyCode::Char('K'), M::CTRL) => {
                term_window.composer_clear()
            }
            (KeyCode::Char('s'), M::CTRL) | (KeyCode::Char('S'), M::CTRL) => {
                term_window.composer_stop()
            }
            (KeyCode::Backspace, M::NONE) => {
                term_window.composer.borrow_mut().draft.pop();
                term_window.invalidate_composer();
            }
            (KeyCode::Char(c), M::NONE) | (KeyCode::Char(c), M::SHIFT) => {
                term_window.composer.borrow_mut().draft.push(c);
                term_window.invalidate_composer();
            }
            _ => {}
        }
        Ok(true)
    }

    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<Ref<'_, [ComputedElement]>> {
        if self.element.borrow().is_none() {
            let element = self.compute(term_window)?;
            self.element.borrow_mut().replace(element);
        }
        Ok(Ref::map(self.element.borrow(), |v| {
            v.as_ref().unwrap().as_slice()
        }))
    }

    fn reconfigure(&self, _term_window: &mut TermWindow) {
        self.element.borrow_mut().take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_label_takes_first_line_and_truncates() {
        assert_eq!(row_label("hello world"), "hello world");
        assert_eq!(row_label("first\nsecond"), "first");
        let long = "x".repeat(100);
        let out = row_label(&long);
        assert!(out.ends_with('…'));
        assert!(unicode_column_width(&out, None) <= MAX_ROW_COLS);
    }
}
