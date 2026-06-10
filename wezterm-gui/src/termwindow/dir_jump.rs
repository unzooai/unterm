//! Directory jump palette (v0.40 "B"): Warp-grade go-to-directory.
//!
//! Summoned from the ⌄ menu, Ctrl+Shift+O, or the top-bar ⌸ button. Type to
//! fuzzy-match across recent/project directories and the live subdirectories
//! of the current browse root (initially the active pane's cwd). Enter cds
//! the pane (shell-aware quoting via cd_command_for_pane), Cmd/Ctrl+Enter
//! opens the directory in a new tab, Tab descends into the selection,
//! Backspace on an empty query ascends to the parent, Cmd+O falls back to
//! the system folder picker. Fully mouse-operable: hover highlights rows
//! (renderer-native), click jumps. Layout follows the same Warp-measured
//! point spec as popup_menu.rs.

use crate::overlay::selector::{matcher_pattern, matcher_score};
use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::{DimensionContext, TermWindow, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::keyassignment::{KeyAssignment, SpawnCommand};
use config::Dimension;
use mux::pane::PaneId;
use std::cell::{Ref, RefCell};
use std::io::Write;
use std::path::{Path, PathBuf};
use wezterm_term::{KeyCode, KeyModifiers, MouseEvent};
use window::color::LinearRgba;
use window::WindowOps;

#[derive(Copy, Clone, PartialEq)]
enum Section {
    Recent,
    SubDir,
}

struct DirItem {
    name: String,
    path: PathBuf,
    section: Section,
}

pub struct DirJump {
    pane_id: PaneId,
    base: RefCell<PathBuf>,
    input: RefCell<String>,
    items: RefCell<Vec<DirItem>>,
    /// Display order → index into `items`, after fuzzy filtering.
    visible: RefCell<Vec<usize>>,
    /// Index into `visible`.
    selected: RefCell<usize>,
    element: RefCell<Option<Vec<ComputedElement>>>,
}

const MAX_VISIBLE: usize = 14;

fn load_recents() -> Vec<PathBuf> {
    let path = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("projects.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return vec![];
    };
    let mut out = vec![];
    if let Some(arr) = value.as_array() {
        for v in arr {
            if let Some(s) = v.as_str() {
                let p = PathBuf::from(s);
                if p.is_dir() {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn subdirs_of(base: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(base)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .take(400)
                .collect()
        })
        .unwrap_or_default();
    dirs.sort_by(|a, b| {
        // dotdirs sort after normal dirs, then lexicographic
        let ah = a.file_name().map_or(false, |n| {
            n.to_string_lossy().starts_with('.')
        });
        let bh = b.file_name().map_or(false, |n| {
            n.to_string_lossy().starts_with('.')
        });
        ah.cmp(&bh).then_with(|| a.cmp(b))
    });
    dirs
}

fn display_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}

fn tilde_path(p: &Path) -> String {
    let s = p.display().to_string();
    if let Some(home) = dirs_next::home_dir() {
        let h = home.display().to_string();
        if let Some(rest) = s.strip_prefix(&h) {
            return format!("~{rest}");
        }
    }
    s
}

impl DirJump {
    pub fn new(pane_id: PaneId, base: PathBuf) -> Self {
        let me = Self {
            pane_id,
            base: RefCell::new(base),
            input: RefCell::new(String::new()),
            items: RefCell::new(vec![]),
            visible: RefCell::new(vec![]),
            selected: RefCell::new(0),
            element: RefCell::new(None),
        };
        me.reload();
        me
    }

    fn reload(&self) {
        let base = self.base.borrow().clone();
        let mut items = vec![];
        for p in load_recents() {
            items.push(DirItem {
                name: display_name(&p),
                path: p,
                section: Section::Recent,
            });
        }
        for p in subdirs_of(&base) {
            items.push(DirItem {
                name: display_name(&p),
                path: p,
                section: Section::SubDir,
            });
        }
        *self.items.borrow_mut() = items;
        self.refilter();
    }

    fn refilter(&self) {
        let input = self.input.borrow();
        let items = self.items.borrow();
        let mut visible: Vec<usize> = vec![];
        if input.is_empty() {
            // Recents first, then subdirs, natural order.
            for (i, _) in items.iter().enumerate().filter(|(_, it)| it.section == Section::Recent) {
                visible.push(i);
            }
            for (i, _) in items.iter().enumerate().filter(|(_, it)| it.section == Section::SubDir) {
                visible.push(i);
            }
        } else {
            let pattern = matcher_pattern(&input);
            let mut scored: Vec<(u32, usize)> = items
                .iter()
                .enumerate()
                .filter_map(|(i, it)| {
                    let hay = if it.section == Section::Recent {
                        format!("{} {}", it.name, it.path.display())
                    } else {
                        it.name.clone()
                    };
                    matcher_score(&pattern, &hay).map(|s| (s, i))
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            visible = scored.into_iter().map(|(_, i)| i).collect();
        }
        visible.truncate(MAX_VISIBLE);
        *self.visible.borrow_mut() = visible;
        *self.selected.borrow_mut() = 0;
        self.element.borrow_mut().take();
    }

    fn invalidate(&self, term_window: &TermWindow) {
        self.element.borrow_mut().take();
        if let Some(window) = term_window.window.as_ref() {
            window.invalidate();
        }
    }

    fn selected_path(&self, display_idx: usize) -> Option<PathBuf> {
        let visible = self.visible.borrow();
        let items = self.items.borrow();
        visible
            .get(display_idx)
            .and_then(|&i| items.get(i))
            .map(|it| it.path.clone())
    }

    /// cd the pane (or spawn a new tab there with `new_tab`) and close.
    pub fn activate(&self, display_idx: usize, term_window: &mut TermWindow, new_tab: bool) {
        let Some(path) = self.selected_path(display_idx) else {
            return;
        };
        term_window.cancel_modal();
        if new_tab {
            let spawn = SpawnCommand {
                cwd: Some(path),
                ..Default::default()
            };
            if let Some(pane) = term_window.get_active_pane_or_overlay() {
                let _ = term_window
                    .perform_key_assignment(&pane, &KeyAssignment::SpawnCommandInNewTab(spawn));
            }
            return;
        }
        let Some(pane) = term_window.get_active_pane_no_overlay() else {
            return;
        };
        if pane.pane_id() != self.pane_id {
            // The pane we were summoned for is gone; cd the active one.
        }
        let cmd = super::cd_command_for_pane(&pane, &path);
        {
            let mut writer = pane.writer();
            if let Err(err) = writer.write_all(cmd.as_bytes()) {
                log::warn!("dir jump: could not inject cd: {err:#}");
            }
        }
    }

    /// Tab: descend into the selected directory and keep browsing.
    fn descend(&self, term_window: &TermWindow) {
        let sel = *self.selected.borrow();
        if let Some(path) = self.selected_path(sel) {
            *self.base.borrow_mut() = path;
            self.input.borrow_mut().clear();
            self.reload();
            self.invalidate(term_window);
        }
    }

    /// Backspace on empty input: ascend to parent.
    fn ascend(&self, term_window: &TermWindow) {
        let parent = self.base.borrow().parent().map(|p| p.to_path_buf());
        if let Some(parent) = parent {
            *self.base.borrow_mut() = parent;
            self.reload();
            self.invalidate(term_window);
        }
    }

    fn move_selection(&self, delta: isize, term_window: &TermWindow) {
        let len = self.visible.borrow().len();
        if len == 0 {
            return;
        }
        let cur = *self.selected.borrow() as isize;
        let next = (cur + delta).rem_euclid(len as isize) as usize;
        *self.selected.borrow_mut() = next;
        self.invalidate(term_window);
    }

    fn compute(&self, term_window: &mut TermWindow) -> anyhow::Result<Vec<ComputedElement>> {
        let font = term_window
            .fonts
            .title_font()
            .expect("to resolve title font");
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let pt = term_window.dimensions.dpi as f32 / 72.0;

        let bg = LinearRgba::with_srgba(0x20, 0x20, 0x20, 0xff);
        let fg = LinearRgba::with_srgba(0xf2, 0xf2, 0xf0, 0xff);
        let dim = LinearRgba::with_srgba(0x9b, 0x9b, 0x98, 0xff);
        let teal = LinearRgba::with_srgba(0x6f, 0xcc, 0xb8, 0xff);
        let hover_bg = LinearRgba::with_srgba(0x34, 0x34, 0x34, 0xff);

        let input = self.input.borrow().clone();
        let base = tilde_path(&self.base.borrow());
        let selected = *self.selected.borrow();

        let mut children: Vec<Element> = vec![];

        // Header: prompt glyph + query + caret, base path right-aligned.
        let header_row = vec![
            Element::new(&font, ElementContent::Text("⌸ ".to_string())).colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: teal.into(),
            }),
            Element::new(&font, ElementContent::Text(format!("{input}▏"))).colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: fg.into(),
            }),
            Element::new(&font, ElementContent::Text(base))
                .float(Float::Right)
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                }),
        ];
        children.push(
            Element::new(&font, ElementContent::Children(header_row))
                .padding(BoxDimension {
                    left: Dimension::Pixels(14. * pt),
                    right: Dimension::Pixels(14. * pt),
                    top: Dimension::Pixels(8. * pt),
                    bottom: Dimension::Pixels(8. * pt),
                })
                .min_width(Some(Dimension::Percent(1.)))
                .display(DisplayType::Block),
        );
        // Hairline under the header.
        children.push(
            Element::new(&font, ElementContent::Text(String::new()))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .line_height(Some(0.08))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(0.12).into(),
                    text: fg.into(),
                }),
        );

        let visible = self.visible.borrow();
        let items = self.items.borrow();
        let mut last_section: Option<Section> = None;
        for (display_idx, &item_idx) in visible.iter().enumerate() {
            let item = &items[item_idx];
            // Section caption when the section changes (only for empty query).
            if input.is_empty() && last_section != Some(item.section) {
                last_section = Some(item.section);
                let caption = match item.section {
                    Section::Recent => crate::i18n::t("dirjump.recent"),
                    Section::SubDir => crate::i18n::t("dirjump.subdirs"),
                };
                children.push(
                    Element::new(&font, ElementContent::Text(caption))
                        .display(DisplayType::Block)
                        .padding(BoxDimension {
                            left: Dimension::Pixels(14. * pt),
                            right: Dimension::Pixels(14. * pt),
                            top: Dimension::Pixels(6. * pt),
                            bottom: Dimension::Pixels(2. * pt),
                        })
                        .colors(ElementColors {
                            border: BorderColor::default(),
                            bg: LinearRgba::TRANSPARENT.into(),
                            text: dim.into(),
                        }),
                );
            }

            let (row_bg, row_fg): (InheritableColor, InheritableColor) =
                if display_idx == selected {
                    (hover_bg.into(), fg.into())
                } else {
                    (LinearRgba::TRANSPARENT.into(), fg.into())
                };
            let mut row = vec![Element::new(
                &font,
                ElementContent::Text(item.name.clone()),
            )];
            row.push(
                Element::new(&font, ElementContent::Text(tilde_path(&item.path)))
                    .float(Float::Right)
                    .padding(BoxDimension {
                        left: Dimension::Pixels(24. * pt),
                        right: Dimension::Pixels(0.),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
                    .zindex(10)
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: row_bg.clone(),
                        text: dim.into(),
                    }),
            );
            children.push(
                Element::new(&font, ElementContent::Children(row))
                    .item_type(UIItemType::DirJumpRow(display_idx))
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: row_bg,
                        text: row_fg,
                    })
                    .hover_colors(Some(ElementColors {
                        border: BorderColor::default(),
                        bg: hover_bg.into(),
                        text: fg.into(),
                    }))
                    .padding(BoxDimension {
                        left: Dimension::Pixels(21. * pt),
                        right: Dimension::Pixels(14. * pt),
                        top: Dimension::Pixels(5. * pt),
                        bottom: Dimension::Pixels(5. * pt),
                    })
                    .min_width(Some(Dimension::Percent(1.)))
                    .display(DisplayType::Block),
            );
        }

        if visible.is_empty() {
            children.push(
                Element::new(
                    &font,
                    ElementContent::Text(crate::i18n::t("dirjump.empty")),
                )
                .display(DisplayType::Block)
                .padding(BoxDimension {
                    left: Dimension::Pixels(14. * pt),
                    right: Dimension::Pixels(14. * pt),
                    top: Dimension::Pixels(8. * pt),
                    bottom: Dimension::Pixels(8. * pt),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                }),
            );
        }

        // Footer hints.
        children.push(
            Element::new(&font, ElementContent::Text(String::new()))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .line_height(Some(0.08))
                .margin(BoxDimension {
                    left: Dimension::Pixels(0.),
                    right: Dimension::Pixels(0.),
                    top: Dimension::Pixels(4. * pt),
                    bottom: Dimension::Pixels(0.),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(0.12).into(),
                    text: fg.into(),
                }),
        );
        children.push(
            Element::new(
                &font,
                ElementContent::Text(crate::i18n::t("dirjump.hints")),
            )
            .display(DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(14. * pt),
                right: Dimension::Pixels(14. * pt),
                top: Dimension::Pixels(6. * pt),
                bottom: Dimension::Pixels(2. * pt),
            })
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: dim.into(),
            }),
        );

        let card = Element::new(&font, ElementContent::Children(children))
            .item_type(UIItemType::PopupMenuCard)
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
        let width = (480. * pt).min(dimensions.pixel_width as f32 - 32. * pt).round();
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

impl Modal for DirJump {
    fn mouse_event(&self, _event: MouseEvent, _term_window: &mut TermWindow) -> anyhow::Result<()> {
        Ok(())
    }

    fn key_down(
        &self,
        key: KeyCode,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool> {
        match (key, mods) {
            (KeyCode::Escape, KeyModifiers::NONE) => term_window.cancel_modal(),
            (KeyCode::UpArrow, KeyModifiers::NONE) => self.move_selection(-1, term_window),
            (KeyCode::DownArrow, KeyModifiers::NONE) => self.move_selection(1, term_window),
            (KeyCode::Tab, KeyModifiers::NONE) => self.descend(term_window),
            (KeyCode::Enter, KeyModifiers::NONE) => {
                let sel = *self.selected.borrow();
                self.activate(sel, term_window, false);
            }
            (KeyCode::Enter, KeyModifiers::SUPER) | (KeyCode::Enter, KeyModifiers::CTRL) => {
                let sel = *self.selected.borrow();
                self.activate(sel, term_window, true);
            }
            (KeyCode::Char('o'), KeyModifiers::SUPER) => {
                let pane_id = self.pane_id;
                term_window.cancel_modal();
                term_window.change_working_directory_for_pane(pane_id);
            }
            (KeyCode::Char('u'), KeyModifiers::CTRL) => {
                self.input.borrow_mut().clear();
                self.refilter();
                self.invalidate(term_window);
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                let emptied = {
                    let mut input = self.input.borrow_mut();
                    if input.pop().is_none() {
                        true
                    } else {
                        false
                    }
                };
                if emptied {
                    self.ascend(term_window);
                } else {
                    self.refilter();
                    self.invalidate(term_window);
                }
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.input.borrow_mut().push(c);
                self.refilter();
                self.invalidate(term_window);
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
