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

/// How the runner reacts when the active pane goes idle mid-run. Cycled with
/// Tab inside the overlay and held in `ComposerState` (runtime-only; there is
/// intentionally no config-file binding — the installed config is hard to
/// update).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdvanceMode {
    /// The default. On idle, inspect the pane's bottom lines: auto-approve a
    /// confident Yes/No confirmation, pause on a genuine multi-choice (or when
    /// unsure), otherwise advance to the next queued prompt.
    #[default]
    AutoApprove,
    /// Legacy behavior: idle always advances to the next prompt, no inspection.
    AutoNext,
    /// Idle pauses the run; the user presses Ctrl+Enter to send the next prompt.
    Manual,
}

impl AdvanceMode {
    /// Next mode in the Tab cycle.
    pub fn cycle(self) -> Self {
        match self {
            AdvanceMode::AutoApprove => AdvanceMode::AutoNext,
            AdvanceMode::AutoNext => AdvanceMode::Manual,
            AdvanceMode::Manual => AdvanceMode::AutoApprove,
        }
    }

    /// Short label for the footer hints.
    pub fn label(self) -> &'static str {
        match self {
            AdvanceMode::AutoApprove => "Auto-approve",
            AdvanceMode::AutoNext => "Auto-next",
            AdvanceMode::Manual => "Manual",
        }
    }
}

/// What the runner should do at an idle point, decided by inspecting the pane's
/// bottom lines (only used in `AutoApprove` mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    /// A ❯ select-list whose highlighted option is a "Yes" — press Enter.
    ConfirmEnter,
    /// A `(y/n)`-style text confirmation — type `y` then Enter.
    ConfirmYes,
    /// A genuine multi-choice selection, or anything we can't confidently
    /// classify — pause the run rather than guess.
    Pause,
    /// No interactive prompt detected (looks done) — advance to the next prompt.
    Done,
}

/// Where an in-flight run currently sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    /// Normal: on idle, make an advance decision.
    Running,
    /// AutoApprove paused on a multi-choice / ambiguous prompt. `moved` flips to
    /// true once the pane emits new output (the user is operating it); the next
    /// idle after movement re-evaluates the pane.
    AutoPaused { moved: bool },
    /// Manual mode paused; waits for the user to press Ctrl+Enter.
    ManualPaused,
}

impl RunPhase {
    pub fn is_paused(self) -> bool {
        !matches!(self, RunPhase::Running)
    }
}

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
    /// Whether we are running, or paused waiting on the pane / the user.
    pub phase: RunPhase,
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
    /// How the runner reacts when the pane goes idle mid-run. Persists across
    /// overlay toggles within a session; cycled with Tab.
    pub advance_mode: AdvanceMode,
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

/// A parsed row of an interactive select list.
struct SelectOption<'a> {
    /// Whether this row carried the highlight cursor glyph (❯ / ▶ / ● / >).
    has_cursor: bool,
    /// The option's visible label, with cursor + numeric/letter marker stripped.
    text: &'a str,
}

/// Cursor glyphs that mark the highlighted row of an interactive select list.
const CURSOR_GLYPHS: [char; 6] = ['❯', '▶', '➤', '→', '●', '>'];

/// If `line` begins (after leading whitespace) with a cursor glyph followed by
/// non-empty text, return that trailing text.
fn strip_cursor(line: &str) -> Option<&str> {
    let t = line.trim_start();
    for g in CURSOR_GLYPHS {
        if let Some(rest) = t.strip_prefix(g) {
            let rest = rest.trim_start();
            if !rest.is_empty() {
                return Some(rest);
            }
        }
    }
    None
}

/// Strip a leading list marker (`1.`, `2)`, `a.`, `b)`) and return the label.
/// Only ASCII markers are recognized; the label after may be any text.
fn strip_marker(s: &str) -> Option<&str> {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        // Allow a single-letter marker such as "a)".
        if bytes.first().map_or(false, |b| b.is_ascii_alphabetic()) {
            i = 1;
        } else {
            return None;
        }
    }
    if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b')') {
        return Some(s[i + 1..].trim_start());
    }
    None
}

/// Parse one line as a select-list option. A line qualifies only if it carries
/// a cursor glyph or a numeric/letter marker — ordinary output is ignored.
fn parse_option(line: &str) -> Option<SelectOption<'_>> {
    if line.trim().is_empty() {
        return None;
    }
    let (has_cursor, rest) = match strip_cursor(line) {
        Some(r) => (true, r),
        None => (false, line.trim()),
    };
    let marker = strip_marker(rest);
    if !has_cursor && marker.is_none() {
        return None;
    }
    Some(SelectOption {
        has_cursor,
        text: marker.unwrap_or(rest).trim(),
    })
}

/// True when `s`'s first word is `word` (case-insensitive), i.e. `word` is a
/// prefix that is not merely the start of a longer word ("no" matches "no," but
/// not "none").
fn starts_word(s: &str, word: &str) -> bool {
    let s = s.trim().to_ascii_lowercase();
    match s.strip_prefix(word) {
        Some(rest) => rest.chars().next().map_or(true, |c| !c.is_alphanumeric()),
        None => false,
    }
}

fn is_yes(s: &str) -> bool {
    starts_word(s, "yes") || s.trim().eq_ignore_ascii_case("y")
}

fn is_no(s: &str) -> bool {
    starts_word(s, "no") || s.trim().eq_ignore_ascii_case("n")
}

/// Inspect the pane's bottom lines (already stripped to plain, trailing-trimmed
/// strings) and decide what the runner should do. Conservative by design:
/// auto-approve only fires on a confident Yes/No match, and anything ambiguous
/// resolves to `Pause` rather than a guessed keystroke.
pub fn classify_prompt(lines: &[String]) -> PromptKind {
    // Collect any select-list options present in the bottom lines.
    let opts: Vec<SelectOption> = lines.iter().filter_map(|l| parse_option(l)).collect();
    let has_cursor = opts.iter().any(|o| o.has_cursor);

    // An interactive select list needs a highlight cursor and at least two
    // options (a lone cursor line is too weak a signal to act on).
    if has_cursor && opts.len() >= 2 {
        let cursor_on_yes = opts
            .iter()
            .find(|o| o.has_cursor)
            .map_or(false, |o| is_yes(o.text));
        let all_yes_no = opts.iter().all(|o| is_yes(o.text) || is_no(o.text));
        if cursor_on_yes && all_yes_no {
            // Every option is Yes/No-like and the highlight sits on a Yes —
            // this is a confirmation; approve by pressing Enter.
            return PromptKind::ConfirmEnter;
        }
        // Genuine multi-choice, or a Yes/No list whose default is not Yes:
        // pause and let the user drive.
        return PromptKind::Pause;
    }

    // No select list — look for a (y/n)-style text confirmation near the very
    // bottom (last few non-empty lines) to avoid matching stale scrollback.
    for l in lines.iter().filter(|l| !l.trim().is_empty()).rev().take(3) {
        let low = l.to_ascii_lowercase();
        if low.contains("y/n") || low.contains("yes/no") {
            return PromptKind::ConfirmYes;
        }
    }

    PromptKind::Done
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
        let advance_mode = state.advance_mode;

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

        // Footer hints. The advance mode is always shown and cycled with Tab.
        let mode = advance_mode.label();
        let hints = if running {
            format!("Mode: {mode}   Tab: cycle   Ctrl+Enter: next   Ctrl+S / Esc: stop")
        } else {
            format!(
                "Mode: {mode}   Tab: cycle   Ctrl+Enter: run   Del: remove   Ctrl+K: clear   Esc: close"
            )
        };
        children.push(
            text_el(hints, dim)
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
            // Tab cycles the advance mode (Auto-approve / Auto-next / Manual).
            (KeyCode::Tab, M::NONE) => {
                {
                    let mut c = term_window.composer.borrow_mut();
                    c.advance_mode = c.advance_mode.cycle();
                }
                term_window.invalidate_composer();
            }
            // Ctrl/Cmd+Enter runs the queue, or — while paused mid-run —
            // force-advances to the next prompt.
            (KeyCode::Enter, M::CTRL) | (KeyCode::Enter, M::SUPER) => {
                if term_window.composer.borrow().is_running() {
                    term_window.composer_force_advance();
                } else {
                    term_window.composer_run_start();
                }
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

    fn block(s: &str) -> Vec<String> {
        s.lines().map(|l| l.trim_end().to_string()).collect()
    }

    #[test]
    fn classifies_claude_code_yes_no_confirm() {
        // Claude Code confirmation: cursor defaults to option 1 ("Yes"), and
        // every option is Yes/No-like → approve with Enter.
        let b = block(
            "Do you want to make this edit?\n\
             ❯ 1. Yes\n  \
             2. Yes, and don't ask again this session\n  \
             3. No, and tell Claude what to do differently",
        );
        assert_eq!(classify_prompt(&b), PromptKind::ConfirmEnter);
    }

    #[test]
    fn classifies_yn_text_prompt() {
        assert_eq!(
            classify_prompt(&block("Overwrite existing file? (y/N)")),
            PromptKind::ConfirmYes
        );
        assert_eq!(
            classify_prompt(&block("Proceed? [Y/n]")),
            PromptKind::ConfirmYes
        );
        assert_eq!(
            classify_prompt(&block("Continue (yes/no)?")),
            PromptKind::ConfirmYes
        );
    }

    #[test]
    fn multi_choice_menu_pauses() {
        // A genuine pick-one menu (non-Yes/No options) must pause, never send.
        let b = block(
            "Select a branch to check out:\n\
             ❯ 1. main\n  \
             2. develop\n  \
             3. feature/login\n  \
             4. release/1.2",
        );
        assert_eq!(classify_prompt(&b), PromptKind::Pause);
    }

    #[test]
    fn yes_no_list_defaulting_to_no_pauses() {
        // All options are Yes/No-like but the cursor is on "No" — don't approve.
        let b = block(
            "Delete everything?\n  \
             1. Yes\n\
             ❯ 2. No",
        );
        assert_eq!(classify_prompt(&b), PromptKind::Pause);
    }

    #[test]
    fn shell_prompt_looks_done() {
        let b = block("$ ls\nfoo.txt  bar.txt\nuser@host:~/project$ ");
        assert_eq!(classify_prompt(&b), PromptKind::Done);
        // "none" must not be read as a "no".
        assert!(!is_no("none of the above"));
        assert!(is_no("No, thanks"));
        assert!(is_yes("Yes, and don't ask again"));
    }
}
