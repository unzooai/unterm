//! Fleet launch palette — type the task, pick a crew, Enter to launch.
//!
//! One card, no sub-menus: the input line is the task prompt, the rows
//! are crew presets (built from the agents actually installed on PATH).
//! Enter prechecks the repo (clean git worktree) synchronously and shows
//! the failure inline; the actual worktree + tab spawning runs on a
//! worker thread because the mux spawn resolves on the main thread.

use crate::cockpit::fleet;
use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::render::corners::{TOP_LEFT_ROUNDED_CORNER, TOP_RIGHT_ROUNDED_CORNER};
use crate::termwindow::{DimensionContext, TermWindow, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::Dimension;
use std::cell::RefCell;
use std::path::PathBuf;
use wezterm_term::{KeyCode, KeyModifiers};
use window::color::LinearRgba;
use window::WindowOps;

#[derive(Clone)]
struct Preset {
    label: String,
    agents: Vec<String>,
}

pub struct FleetPalette {
    base: PathBuf,
    input: RefCell<String>,
    /// In-progress IME composition (pinyin etc.), rendered inline after
    /// the committed input; committed text arrives via key_down.
    composing: RefCell<Option<String>>,
    presets: Vec<Preset>,
    selected: RefCell<usize>,
    error: RefCell<Option<&'static str>>,
    element: RefCell<Option<Vec<ComputedElement>>>,
    /// Pixel position of the input caret, captured during compute() so
    /// the OS IME candidate window can be anchored to it.
    ime_rect: RefCell<Option<::window::Rect>>,
}

fn build_presets() -> Vec<Preset> {
    let installed = fleet::installed_agents();
    let has = |a: &str| installed.contains(&a);
    let mut presets = Vec::new();
    if has("claude") {
        presets.push(Preset {
            label: "claude ×2".into(),
            agents: vec!["claude".into(), "claude".into()],
        });
        presets.push(Preset {
            label: "claude ×3".into(),
            agents: vec!["claude".into(); 3],
        });
    }
    if has("claude") && has("codex") {
        presets.push(Preset {
            label: "claude + codex".into(),
            agents: vec!["claude".into(), "codex".into()],
        });
    }
    if has("claude") && has("codex") && has("gemini") {
        presets.push(Preset {
            label: "claude + codex + gemini".into(),
            agents: vec!["claude".into(), "codex".into(), "gemini".into()],
        });
    }
    for a in &installed {
        presets.push(Preset {
            label: format!("{a} ×1"),
            agents: vec![a.to_string()],
        });
    }
    presets
}

impl FleetPalette {
    pub fn new(base: PathBuf) -> Self {
        Self {
            base,
            input: RefCell::new(String::new()),
            composing: RefCell::new(None),
            presets: build_presets(),
            selected: RefCell::new(0),
            error: RefCell::new(None),
            element: RefCell::new(None),
            ime_rect: RefCell::new(None),
        }
    }

    fn invalidate(&self, term_window: &TermWindow) {
        self.element.borrow_mut().take();
        if let Some(window) = term_window.window.as_ref() {
            window.invalidate();
        }
    }

    fn move_selection(&self, delta: isize, term_window: &TermWindow) {
        if self.presets.is_empty() {
            return;
        }
        let len = self.presets.len();
        let mut sel = self.selected.borrow_mut();
        *sel = (*sel as isize + delta).rem_euclid(len as isize) as usize;
        drop(sel);
        self.invalidate(term_window);
    }

    pub fn hover_select(&self, idx: usize, term_window: &TermWindow) {
        if idx < self.presets.len() && *self.selected.borrow() != idx {
            *self.selected.borrow_mut() = idx;
            self.invalidate(term_window);
        }
    }

    pub fn activate(&self, idx: usize, term_window: &mut TermWindow) {
        let task = self.input.borrow().trim().to_string();
        if task.is_empty() {
            *self.error.borrow_mut() = Some("cockpit.fleet_placeholder");
            self.invalidate(term_window);
            return;
        }
        let Some(preset) = self.presets.get(idx).cloned() else {
            return;
        };
        if let Err(key) = fleet::precheck(&self.base) {
            *self.error.borrow_mut() = Some(key);
            self.invalidate(term_window);
            return;
        }
        let base = self.base.clone();
        term_window.cancel_modal();
        // Worktree creation + tab spawning must run off the main thread:
        // the mux spawn future resolves here, and launch() blocks on it.
        std::thread::Builder::new()
            .name("fleet-launch".into())
            .spawn(move || match fleet::launch(&base, &task, &preset.agents) {
                Ok(f) => log::info!("fleet {} launched with {} members", f.id, f.members.len()),
                Err(err) => log::error!("fleet launch failed: {err:#}"),
            })
            .ok();
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
        let red = LinearRgba::with_srgba(0xe0, 0x6c, 0x75, 0xff);
        let hover_bg = LinearRgba::with_srgba(0x3d, 0x3d, 0x3d, 0xff);
        let field_bg = LinearRgba::with_srgba(0x1c, 0x1c, 0x1c, 0xff);

        let input = self.input.borrow().clone();
        let selected = *self.selected.borrow();
        let card_width = (520. * pt)
            .min(term_window.dimensions.pixel_width as f32 - 32. * pt)
            .round();

        let mut children: Vec<Element> = vec![];

        children.push(
            Element::new(
                &font,
                ElementContent::Text(crate::i18n::t("cockpit.fleet_title")),
            )
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: dim.into(),
            })
            .padding(BoxDimension {
                left: Dimension::Pixels(12. * pt),
                right: Dimension::Pixels(12. * pt),
                top: Dimension::Pixels(8. * pt),
                bottom: Dimension::Pixels(4. * pt),
            })
            .min_width(Some(Dimension::Percent(1.)))
            .display(DisplayType::Block),
        );

        // Task input field. Pasted newlines fold to ␤ for the one-line
        // display; the stored task (what the agent receives) keeps them.
        let composing = self.composing.borrow().clone().unwrap_or_default();
        let shown = if input.is_empty() && composing.is_empty() {
            crate::i18n::t("cockpit.fleet_placeholder")
        } else {
            input.replace('\n', "\u{2424}").replace('\r', "")
        };
        let input_color = if input.is_empty() && composing.is_empty() {
            dim
        } else {
            fg
        };
        let field = Element::new(
            &font,
            ElementContent::Children(vec![
                Element::new(&font, ElementContent::Text("\u{eb44} ".to_string())).colors(
                    ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: teal.into(),
                    },
                ),
                Element::new(&font, ElementContent::Text(shown.clone())).colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: input_color.into(),
                }),
                // IME composition preview: distinct tint so the pinyin
                // reads as "not yet committed".
                Element::new(&font, ElementContent::Text(composing.clone())).colors(
                    ElementColors {
                        border: BorderColor::default(),
                        bg: teal.mul_alpha(0.22).into(),
                        text: teal.into(),
                    },
                ),
                Element::new(&font, ElementContent::Text("▏".to_string())).colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: teal.into(),
                }),
            ]),
        )
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
                    bottom: Dimension::Pixels(6. * pt),
                })
                .min_width(Some(Dimension::Percent(1.)))
                .display(DisplayType::Block),
        );

        // Anchor the IME candidate window at the input caret. The layout
        // isn't computed yet, so derive the caret position from the same
        // constants the elements above use: card top (12% of the window)
        // + title row (8pt pad + cell + 4pt pad) + field container top
        // pad (2pt) + field padding/border (~7pt), and x from the card's
        // left edge + paddings + the glyph/text width so far.
        {
            let card_x = (term_window.dimensions.pixel_width as f32 - card_width) / 2.;
            let card_y = (term_window.dimensions.pixel_height as f32 * 0.12).round();
            let cell_w = metrics.cell_size.width as f32;
            let cell_h = metrics.cell_size.height as f32;
            let text_cols = termwiz::cell::unicode_column_width(&shown, None)
                + termwiz::cell::unicode_column_width(&composing, None)
                + 2; // "⛵ " prefix
            let caret_x = card_x + (12. + 10. + 10.) * pt + text_cols as f32 * cell_w;
            let caret_y = card_y + (8. + 4. + 2. + 7.) * pt + cell_h;
            self.ime_rect.borrow_mut().replace(::window::Rect::new(
                euclid::point2(caret_x as isize, caret_y as isize),
                euclid::size2(cell_w as isize, cell_h as isize),
            ));
        }

        // Error line (repo not clean / not a repo / empty task).
        if let Some(err_key) = *self.error.borrow() {
            children.push(
                Element::new(&font, ElementContent::Text(crate::i18n::t(err_key)))
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: red.into(),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Pixels(12. * pt),
                        right: Dimension::Pixels(12. * pt),
                        top: Dimension::Pixels(2. * pt),
                        bottom: Dimension::Pixels(2. * pt),
                    })
                    .min_width(Some(Dimension::Percent(1.)))
                    .display(DisplayType::Block),
            );
        }

        for (idx, preset) in self.presets.iter().enumerate() {
            let is_sel = idx == selected;
            children.push(
                Element::new(&font, ElementContent::Text(preset.label.clone()))
                    .item_type(UIItemType::FleetPaletteRow(idx))
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: if is_sel {
                            hover_bg.into()
                        } else {
                            LinearRgba::TRANSPARENT.into()
                        },
                        text: fg.into(),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Pixels(12. * pt),
                        right: Dimension::Pixels(12. * pt),
                        top: Dimension::Pixels(4. * pt),
                        bottom: Dimension::Pixels(4. * pt),
                    })
                    .min_width(Some(Dimension::Percent(1.)))
                    .display(DisplayType::Block),
            );
        }
        if self.presets.is_empty() {
            children.push(
                Element::new(
                    &font,
                    ElementContent::Text("claude / codex / gemini / aider — none on PATH".into()),
                )
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                })
                .padding(BoxDimension {
                    left: Dimension::Pixels(12. * pt),
                    right: Dimension::Pixels(12. * pt),
                    top: Dimension::Pixels(4. * pt),
                    bottom: Dimension::Pixels(4. * pt),
                })
                .min_width(Some(Dimension::Percent(1.)))
                .display(DisplayType::Block),
            );
        }

        children.push(
            Element::new(
                &font,
                ElementContent::Text(crate::i18n::t("cockpit.fleet_footer")),
            )
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: dim.into(),
            })
            .padding(BoxDimension {
                left: Dimension::Pixels(12. * pt),
                right: Dimension::Pixels(12. * pt),
                top: Dimension::Pixels(6. * pt),
                bottom: Dimension::Pixels(8. * pt),
            })
            .min_width(Some(Dimension::Percent(1.)))
            .display(DisplayType::Block),
        );

        let card = Element::new(&font, ElementContent::Children(children))
            .item_type(UIItemType::PopupMenuCard)
            .colors(ElementColors {
                border: BorderColor::new(fg.mul_alpha(0.18)),
                bg: bg.into(),
                text: fg.into(),
            })
            .border(BoxDimension::new(Dimension::Pixels(1.)))
            .border_corners(Some(Corners {
                top_left: SizedPoly {
                    width: Dimension::Pixels(8. * pt),
                    height: Dimension::Pixels(8. * pt),
                    poly: TOP_LEFT_ROUNDED_CORNER,
                },
                top_right: SizedPoly {
                    width: Dimension::Pixels(8. * pt),
                    height: Dimension::Pixels(8. * pt),
                    poly: TOP_RIGHT_ROUNDED_CORNER,
                },
                bottom_left: SizedPoly::none(),
                bottom_right: SizedPoly::none(),
            }))
            .min_width(Some(Dimension::Pixels(card_width)))
            .max_width(Some(Dimension::Pixels(card_width)))
            .display(DisplayType::Block);

        let dims = term_window.dimensions;
        let computed = term_window.compute_element(
            &LayoutContext {
                height: DimensionContext {
                    dpi: dims.dpi as f32,
                    pixel_max: dims.pixel_height as f32,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: DimensionContext {
                    dpi: dims.dpi as f32,
                    pixel_max: dims.pixel_width as f32,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(
                    (dims.pixel_width as f32 - card_width) / 2.,
                    (dims.pixel_height as f32 * 0.12).round(),
                    card_width,
                    dims.pixel_height as f32 * 0.76,
                ),
                metrics: &metrics,
                gl_state: term_window.render_state.as_ref().unwrap(),
                zindex: 100,
            },
            &card,
        )?;
        Ok(vec![computed])
    }
}

impl FleetPalette {
    /// Append pasted text to the task input. Newlines are kept in the
    /// stored task (the agent receives them verbatim); the input line
    /// renders them folded (see compute()).
    pub fn append_input(&self, text: &str, term_window: &TermWindow) {
        self.input.borrow_mut().push_str(text);
        *self.error.borrow_mut() = None;
        self.invalidate(term_window);
    }
}

impl Modal for FleetPalette {
    fn perform_assignment(
        &self,
        assignment: &config::keyassignment::KeyAssignment,
        term_window: &mut TermWindow,
    ) -> bool {
        use config::keyassignment::{ClipboardPasteSource, KeyAssignment};
        match assignment {
            KeyAssignment::PasteFrom(source) => {
                let clipboard = match source {
                    ClipboardPasteSource::Clipboard => window::Clipboard::Clipboard,
                    ClipboardPasteSource::PrimarySelection => window::Clipboard::PrimarySelection,
                };
                let Some(win) = term_window.window.as_ref().cloned() else {
                    return true;
                };
                // Same background-read pattern as paste_from_clipboard:
                // macOS pasteboards answer synchronously cross-process, so
                // never read them on the GUI thread.
                let win_for_read = win.clone();
                let reader = promise::spawn::spawn_into_new_thread(move || {
                    promise::spawn::block_on(win_for_read.get_clipboard(clipboard))
                });
                promise::spawn::spawn(async move {
                    if let Ok(clip) = reader.await {
                        win.notify(crate::termwindow::TermWindowNotif::Apply(Box::new(
                            move |tw| {
                                let modal = tw.modal.borrow().clone();
                                if let Some(modal) = modal {
                                    if let Some(fp) = modal.downcast_ref::<FleetPalette>() {
                                        fp.append_input(&clip, tw);
                                    }
                                }
                            },
                        )));
                    }
                })
                .detach();
                true
            }
            _ => false,
        }
    }

    fn advise_compose(&self, status: &::window::DeadKeyStatus) -> bool {
        *self.composing.borrow_mut() = match status {
            ::window::DeadKeyStatus::Composing(s) => Some(s.clone()),
            ::window::DeadKeyStatus::None => None,
        };
        self.element.borrow_mut().take();
        true
    }

    fn ime_cursor_rect(&self, _term_window: &TermWindow) -> Option<::window::Rect> {
        *self.ime_rect.borrow()
    }

    fn mouse_event(
        &self,
        _event: wezterm_term::MouseEvent,
        _term_window: &mut TermWindow,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn key_down(
        &self,
        key: KeyCode,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool> {
        match (key, mods) {
            (KeyCode::Escape, KeyModifiers::NONE) => {
                term_window.cancel_modal();
            }
            (KeyCode::UpArrow, KeyModifiers::NONE) => self.move_selection(-1, term_window),
            (KeyCode::DownArrow, KeyModifiers::NONE) => self.move_selection(1, term_window),
            (KeyCode::Enter, KeyModifiers::NONE) => {
                let sel = *self.selected.borrow();
                self.activate(sel, term_window);
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                self.input.borrow_mut().pop();
                *self.error.borrow_mut() = None;
                self.invalidate(term_window);
            }
            (KeyCode::Char('u'), KeyModifiers::CTRL) => {
                self.input.borrow_mut().clear();
                *self.error.borrow_mut() = None;
                self.invalidate(term_window);
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                if !c.is_control() {
                    self.input.borrow_mut().push(c);
                    *self.error.borrow_mut() = None;
                    self.invalidate(term_window);
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<std::cell::Ref<'_, [ComputedElement]>> {
        if self.element.borrow().is_none() {
            let computed = self.compute(term_window)?;
            self.element.borrow_mut().replace(computed);
        }
        Ok(std::cell::Ref::map(self.element.borrow(), |v| {
            v.as_ref().unwrap().as_slice()
        }))
    }

    fn reconfigure(&self, _term_window: &mut TermWindow) {
        self.element.borrow_mut().take();
    }
}
