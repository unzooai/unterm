//! Mouse-operable popup menu (v0.40 foundation).
//!
//! Replaces the keyboard-only cell-grid settings menu with a real floating
//! menu: hover highlights, click executes, click-outside or Esc closes, and
//! ↑/↓/Enter still work. Rendered through the box-model as a Modal, anchored
//! under the tab bar's ⌄ button. Rows are tagged `UIItemType::PopupMenuRow`
//! so the existing `ui_items` hit-testing covers them; the actual mouse
//! routing lives in `mouseevent.rs` (the upstream Modal trait's mouse hook
//! was never wired, and other modals stay keyboard-only for now).

use crate::customglyph::Poly;
use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::ui_icons;
use crate::termwindow::{DimensionContext, TermWindow, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::keyassignment::{KeyAssignment, Pattern, SpawnCommand, SpawnTabDomain};
use config::Dimension;
use mux::pane::PaneId;
use std::cell::{Ref, RefCell};
use termwiz::cell::unicode_column_width;
use wezterm_term::color::ColorAttribute;
use wezterm_term::{KeyCode, KeyModifiers, MouseEvent};
use window::color::LinearRgba;
use window::WindowOps;

pub enum MenuAction {
    Assign(KeyAssignment),
    DirJump,
    ChangeCwd,
    ToggleRecording,
    ExportSession,
    OpenWebSettings,
    /// Open an arbitrary URL via the OS browser. Used for the "About"
    /// footer link to the author's site so it doesn't ride along with
    /// the existing Web Settings action.
    OpenUrl(String),
    ToggleTreeSidebar,
    ToggleLeftTabBar,
    /// In-terminal long screenshot: whole scrollback -> one tall PNG.
    CaptureScrollback,
    /// External long screenshot: point at another window, scroll + stitch.
    ScrollShotExternal,
    CopyText(String),
    RevealPath(std::path::PathBuf),
    NewTabAt(std::path::PathBuf),
    CdTo(std::path::PathBuf),
}

pub struct MenuEntry {
    label: String,
    accel: &'static str,
    icon: Option<&'static [Poly]>,
    /// `None` renders as a separator line.
    action: Option<MenuAction>,
}

fn text_cols(text: &str) -> usize {
    unicode_column_width(text, None)
}

fn entry_cols(entry: &MenuEntry) -> usize {
    text_cols(&entry.label) + text_cols(entry.accel) + 6
}

pub struct PopupMenu {
    entries: Vec<MenuEntry>,
    pane_id: PaneId,
    element: RefCell<Option<Vec<ComputedElement>>>,
    hover: RefCell<Option<usize>>,
    /// Pixel anchor (window coords). None = under the tab bar's ⌄ button.
    anchor: Option<(f32, f32)>,
}

impl PopupMenu {
    pub fn build_default(pane_id: PaneId) -> Self {
        let recording_on = crate::recording::recorder::current_session(pane_id).is_some();
        let recording_label = if recording_on {
            crate::i18n::t("settings.menu.recording_on")
        } else {
            crate::i18n::t("settings.menu.recording_off")
        };
        let entry = |label: String,
                     accel: &'static str,
                     icon: &'static [Poly],
                     action: MenuAction| MenuEntry {
            label,
            accel,
            icon: Some(icon),
            action: Some(action),
        };
        let sep = || MenuEntry {
            label: String::new(),
            accel: "",
            icon: None,
            action: None,
        };
        let entries = vec![
            entry(
                crate::i18n::t("menu.new_tab"),
                "Ctrl+Shift+T",
                ui_icons::ICON_PLUS,
                MenuAction::Assign(KeyAssignment::SpawnTab(SpawnTabDomain::CurrentPaneDomain)),
            ),
            entry(
                crate::i18n::t("settings.menu.split_right"),
                "Ctrl+Shift+E",
                ui_icons::ICON_SPLIT,
                MenuAction::Assign(KeyAssignment::SplitHorizontal(SpawnCommand::default())),
            ),
            entry(
                crate::i18n::t("menu.dir_jump"),
                "Ctrl+Shift+O",
                ui_icons::ICON_FOLDER,
                MenuAction::DirJump,
            ),
            entry(
                crate::i18n::t("menu.tree_sidebar"),
                "Ctrl+Shift+B",
                ui_icons::ICON_TREE,
                MenuAction::ToggleTreeSidebar,
            ),
            entry(
                crate::i18n::t("menu.left_tabs"),
                "",
                ui_icons::ICON_SPLIT,
                MenuAction::ToggleLeftTabBar,
            ),
            entry(
                crate::i18n::t("menu.find"),
                "Ctrl+Shift+F",
                ui_icons::ICON_SEARCH,
                MenuAction::Assign(KeyAssignment::Search(Pattern::CaseSensitiveString(
                    String::new(),
                ))),
            ),
            sep(),
            entry(
                crate::i18n::t("menu.command_palette"),
                "Ctrl+Shift+P",
                ui_icons::ICON_PROMPT,
                MenuAction::Assign(KeyAssignment::ActivateCommandPalette),
            ),
            sep(),
            entry(
                recording_label,
                "",
                ui_icons::ICON_RECORD,
                MenuAction::ToggleRecording,
            ),
            entry(
                crate::i18n::t("settings.menu.export_session"),
                "",
                ui_icons::ICON_EXPORT,
                MenuAction::ExportSession,
            ),
            entry(
                crate::i18n::t("menu.capture_scrollback"),
                "",
                ui_icons::ICON_LONGSHOT,
                MenuAction::CaptureScrollback,
            ),
            entry(
                crate::i18n::t("menu.scrollshot_window"),
                "",
                ui_icons::ICON_WINSCROLL,
                MenuAction::ScrollShotExternal,
            ),
            sep(),
            entry(
                crate::i18n::t("settings.menu.web_settings"),
                "",
                ui_icons::ICON_SLIDERS,
                MenuAction::OpenWebSettings,
            ),
            // About. Lives at the very bottom so it never crowds the
            // action items above. Three lines: the version (click to copy
            // the full build id for bug reports), the official site, and
            // the producer link.
            sep(),
            entry(
                format!("Unterm v{}", env!("CARGO_PKG_VERSION")),
                "",
                ui_icons::ICON_PROMPT,
                MenuAction::CopyText(config::wezterm_version().to_string()),
            ),
            entry(
                crate::i18n::t("menu.about_website"),
                "",
                ui_icons::ICON_EXPORT,
                MenuAction::OpenUrl("https://unterm.app".to_string()),
            ),
            entry(
                crate::i18n::t("menu.about_producer"),
                "",
                ui_icons::ICON_EXPORT,
                MenuAction::OpenUrl("https://doaipm.com".to_string()),
            ),
        ];
        Self {
            entries,
            pane_id,
            element: RefCell::new(None),
            hover: RefCell::new(None),
            anchor: None,
        }
    }

    /// A context menu with explicit entries, anchored at a pixel position
    /// (e.g. the right-clicked tree row).
    pub fn context(pane_id: PaneId, anchor: (f32, f32), items: Vec<(String, MenuAction)>) -> Self {
        let entries = items
            .into_iter()
            .map(|(label, action)| MenuEntry {
                label,
                accel: "",
                icon: None,
                action: Some(action),
            })
            .collect();
        Self {
            entries,
            pane_id,
            element: RefCell::new(None),
            hover: RefCell::new(None),
            anchor: Some(anchor),
        }
    }

    pub fn precompute(&self, term_window: &mut TermWindow) -> anyhow::Result<()> {
        if self.element.borrow().is_none() {
            let element = self.compute(term_window)?;
            self.element.borrow_mut().replace(element);
        }
        Ok(())
    }

    pub fn clear_precomputed(&self) {
        self.element.borrow_mut().take();
    }

    pub fn pane_id(&self) -> PaneId {
        self.pane_id
    }

    fn first_selectable(&self) -> Option<usize> {
        self.entries.iter().position(|e| e.action.is_some())
    }

    fn step_hover(&self, delta: isize) {
        let len = self.entries.len() as isize;
        if len == 0 {
            return;
        }
        let mut idx = self
            .hover
            .borrow()
            .map(|i| i as isize)
            .unwrap_or(if delta > 0 { -1 } else { 0 });
        for _ in 0..len {
            idx = (idx + delta).rem_euclid(len);
            if self.entries[idx as usize].action.is_some() {
                self.hover.borrow_mut().replace(idx as usize);
                self.element.borrow_mut().take();
                return;
            }
        }
    }

    /// Mouse-move hover from mouseevent.rs. Invalidates only on change.
    pub fn set_hover(&self, idx: Option<usize>, term_window: &TermWindow) {
        if *self.hover.borrow() == idx {
            return;
        }
        *self.hover.borrow_mut() = idx;
        if let Some(window) = term_window.window.as_ref() {
            window.invalidate();
        }
    }

    /// Execute entry `idx` (from click or Enter) and close the menu.
    pub fn activate(&self, idx: usize, term_window: &mut TermWindow) {
        let Some(entry) = self.entries.get(idx) else {
            return;
        };
        let Some(action) = &entry.action else {
            return;
        };
        term_window.cancel_modal();
        let pane_id = self.pane_id;
        match action {
            MenuAction::Assign(assignment) => {
                if let Some(pane) = term_window.get_active_pane_or_overlay() {
                    if let Err(err) = term_window.perform_key_assignment(&pane, assignment) {
                        log::error!("popup menu action failed: {err:#}");
                    }
                }
            }
            MenuAction::DirJump => term_window.show_dir_jump(),
            MenuAction::ChangeCwd => term_window.change_working_directory_for_pane(pane_id),
            MenuAction::ToggleRecording => term_window.toggle_session_recording(pane_id),
            MenuAction::ExportSession => term_window.export_current_session(pane_id),
            MenuAction::OpenWebSettings => term_window.open_web_settings(),
            MenuAction::OpenUrl(url) => {
                wezterm_open_url::open_url(&url);
            }
            MenuAction::ToggleTreeSidebar => term_window.toggle_tree_sidebar(),
            MenuAction::ToggleLeftTabBar => term_window.toggle_left_tab_bar(),
            MenuAction::CaptureScrollback => {
                if let Some(pane) = term_window.get_active_pane_no_overlay() {
                    super::mouseevent::capture_scrollback_and_announce(&pane);
                }
            }
            MenuAction::ScrollShotExternal => {
                if let Some(pane) = term_window.get_active_pane_no_overlay() {
                    super::mouseevent::scrollshot_external_and_announce(&pane);
                }
            }
            MenuAction::CopyText(text) => {
                use config::keyassignment::ClipboardCopyDestination;
                term_window.copy_to_clipboard(
                    ClipboardCopyDestination::ClipboardAndPrimarySelection,
                    text.clone(),
                );
            }
            MenuAction::RevealPath(path) => {
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open")
                    .arg("-R")
                    .arg(path)
                    .spawn();
                #[cfg(all(unix, not(target_os = "macos")))]
                let _ = std::process::Command::new("xdg-open")
                    .arg(path.parent().unwrap_or(path))
                    .spawn();
                #[cfg(windows)]
                let _ = std::process::Command::new("explorer")
                    .arg(format!("/select,{}", path.display()))
                    .spawn();
            }
            MenuAction::NewTabAt(path) => {
                let spawn = SpawnCommand {
                    cwd: Some(path.clone()),
                    ..Default::default()
                };
                if let Some(pane) = term_window.get_active_pane_or_overlay() {
                    let _ = term_window
                        .perform_key_assignment(&pane, &KeyAssignment::SpawnCommandInNewTab(spawn));
                }
            }
            MenuAction::CdTo(path) => {
                if let Some(pane) = term_window.get_active_pane_no_overlay() {
                    let cmd = super::cd_command_for_pane(&pane, path);
                    let mut writer = pane.writer();
                    if let Err(err) = writer.write_all(cmd.as_bytes()) {
                        log::warn!("context cd failed: {err:#}");
                    }
                }
            }
        }
    }

    fn compute(&self, term_window: &mut TermWindow) -> anyhow::Result<Vec<ComputedElement>> {
        let started = std::time::Instant::now();
        let font = term_window
            .fonts
            .default_font()
            .expect("to resolve default font");
        let font_ms = started.elapsed().as_millis();
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let hover = *self.hover.borrow();
        // Warp-derived spec, in points (measured constants: card V-pad 9pt,
        // row V-pad 5pt / H-pad 14pt with 1.5x left inset, separator margin
        // 4pt, radius 6pt, shadow #000@19%). Scaled by dpi to physical px.
        let pt = term_window.dimensions.dpi as f32 / 72.0;

        let palette = term_window.palette().clone();
        let scheme_bg = palette.background.to_linear();
        let scheme_fg = palette.foreground.to_linear();
        let luma = |c: LinearRgba| 0.2126 * c.0 + 0.7152 * c.1 + 0.0722 * c.2;
        let mix = |a: LinearRgba, b: LinearRgba, t: f32| {
            LinearRgba::with_components(
                a.0 * (1. - t) + b.0 * t,
                a.1 * (1. - t) + b.1 * t,
                a.2 * (1. - t) + b.2 * t,
                1.,
            )
        };
        let is_light = luma(scheme_bg) > 0.48;
        let bg = if is_light {
            mix(scheme_bg, scheme_fg, 0.07)
        } else {
            mix(scheme_bg, scheme_fg, 0.10)
        };
        let fg = scheme_fg;
        let accel_fg = fg.mul_alpha(if is_light { 0.62 } else { 0.66 });
        let hover_bg = if is_light {
            mix(bg, scheme_fg, 0.11)
        } else {
            mix(bg, scheme_fg, 0.16)
        };
        let teal = palette
            .resolve_fg(ColorAttribute::PaletteIndex(14))
            .to_linear();
        let border_color = if is_light {
            fg.mul_alpha(0.16)
        } else {
            LinearRgba::with_srgba(0x00, 0x00, 0x00, 0x90)
        };

        let mut rows: Vec<Element> = vec![];
        for (idx, e) in self.entries.iter().enumerate() {
            if e.action.is_none() {
                rows.push(
                    Element::new(&font, ElementContent::Text(String::new()))
                        .display(DisplayType::Block)
                        .min_width(Some(Dimension::Percent(1.)))
                        .line_height(Some(0.12))
                        .margin(BoxDimension {
                            left: Dimension::Pixels(0.),
                            right: Dimension::Pixels(0.),
                            top: Dimension::Pixels(4. * pt),
                            bottom: Dimension::Pixels(4. * pt),
                        })
                        .colors(ElementColors {
                            border: BorderColor::default(),
                            bg: fg.mul_alpha(0.10).into(),
                            text: fg.into(),
                        }),
                );
                continue;
            }
            let (row_bg, row_fg): (InheritableColor, InheritableColor) = if hover == Some(idx) {
                (hover_bg.into(), fg.into())
            } else {
                (bg.into(), fg.into())
            };
            let mut row: Vec<Element> = vec![];
            if let Some(icon) = e.icon {
                row.push(
                    Element::new(
                        &font,
                        ElementContent::Poly {
                            line_width: metrics.underline_height.max(2),
                            poly: SizedPoly {
                                poly: icon,
                                width: Dimension::Pixels(11. * pt),
                                height: Dimension::Pixels(11. * pt),
                            },
                        },
                    )
                    .vertical_align(VerticalAlign::Middle)
                    .padding(BoxDimension {
                        left: Dimension::Pixels(0.),
                        right: Dimension::Pixels(8. * pt),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: row_fg.clone(),
                    }),
                );
            }
            row.push(Element::new(&font, ElementContent::Text(e.label.clone())));
            if !e.accel.is_empty() {
                row.push(
                    Element::new(&font, ElementContent::Text(e.accel.to_string()))
                        .float(Float::Right)
                        .padding(BoxDimension {
                            left: Dimension::Cells(2.),
                            right: Dimension::Cells(0.25),
                            top: Dimension::Cells(0.),
                            bottom: Dimension::Cells(0.),
                        })
                        .zindex(10)
                        .colors(ElementColors {
                            border: BorderColor::default(),
                            bg: LinearRgba::TRANSPARENT.into(),
                            text: accel_fg.into(),
                        }),
                );
            }
            let mut row_border = BorderColor::default();
            if hover == Some(idx) {
                row_border.left = teal;
            }
            rows.push(
                Element::new(&font, ElementContent::Children(row))
                    .item_type(UIItemType::PopupMenuRow(idx))
                    .colors(ElementColors {
                        border: row_border,
                        bg: row_bg,
                        text: row_fg,
                    })
                    .hover_colors(if e.action.is_some() {
                        let mut hover_border = BorderColor::default();
                        hover_border.left = teal;
                        Some(ElementColors {
                            border: hover_border,
                            bg: hover_bg.into(),
                            text: fg.into(),
                        })
                    } else {
                        None
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
                        top: Dimension::Pixels(5. * pt),
                        bottom: Dimension::Pixels(5. * pt),
                    })
                    .min_width(Some(Dimension::Percent(1.)))
                    .display(DisplayType::Block),
            );
        }

        let element = Element::new(&font, ElementContent::Children(rows))
            .item_type(UIItemType::PopupMenuCard)
            .colors(ElementColors {
                border: BorderColor::new(border_color),
                bg: bg.into(),
                text: fg.into(),
            })
            .padding(BoxDimension {
                left: Dimension::Pixels(0.),
                right: Dimension::Pixels(0.),
                top: Dimension::Pixels(9. * pt),
                bottom: Dimension::Pixels(9. * pt),
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
        let top_pixel_y = match self.anchor {
            Some((_, y)) => y.round(),
            None => (top_bar_height + border.top.get() as f32 + 4.).round(),
        };

        // Width: longest label + accel estimate, in cells; clamped to window.
        let max_cols = self.entries.iter().map(entry_cols).max().unwrap_or(24) as f32;
        let cell_w = metrics.cell_size.width as f32;
        let desired_pixel_width = (max_cols * cell_w)
            .min(dimensions.pixel_width as f32 - 16.)
            .round();
        // Anchor under the tab bar's ⌄ button (its ui_item bounds are in
        // window pixels); fall back to the right edge if it isn't on screen.
        let anchor_x = self.anchor.map(|(x, _)| x).or_else(|| {
            term_window
                .ui_items
                .iter()
                .find(|item| {
                    matches!(
                        item.item_type,
                        UIItemType::TabBar(crate::tabbar::TabBarItem::MenuButton)
                    )
                })
                .map(|item| item.x as f32)
        });
        let x = anchor_x
            .unwrap_or(dimensions.pixel_width as f32)
            .min(
                dimensions.pixel_width as f32
                    - border.right.get() as f32
                    - desired_pixel_width
                    - 8.,
            )
            .max(8.)
            .round();

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
                bounds: euclid::rect(
                    x,
                    top_pixel_y,
                    desired_pixel_width,
                    dimensions.pixel_height as f32 - top_pixel_y,
                ),
                metrics: &metrics,
                gl_state: term_window.render_state.as_ref().unwrap(),
                zindex: 100,
            },
            &element,
        )?;

        let total_ms = started.elapsed().as_millis();
        if total_ms > 16 {
            log::debug!("popup menu compute: font {font_ms}ms, total {total_ms}ms");
        }
        Ok(vec![computed])
    }
}

impl Modal for PopupMenu {
    fn mouse_event(&self, _event: MouseEvent, _term_window: &mut TermWindow) -> anyhow::Result<()> {
        // Real mouse routing happens in mouseevent.rs via ui_items hit-testing.
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
            (KeyCode::UpArrow, KeyModifiers::NONE) => {
                self.step_hover(-1);
                if let Some(window) = term_window.window.as_ref() {
                    window.invalidate();
                }
            }
            (KeyCode::DownArrow, KeyModifiers::NONE) => {
                self.step_hover(1);
                if let Some(window) = term_window.window.as_ref() {
                    window.invalidate();
                }
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                let idx = self.hover.borrow().or_else(|| self.first_selectable());
                if let Some(idx) = idx {
                    self.activate(idx, term_window);
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<Ref<'_, [ComputedElement]>> {
        self.precompute(term_window)?;
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
    fn menu_width_counts_cjk_as_wide_cells() {
        let entry = MenuEntry {
            label: "打开设置".to_string(),
            accel: "Ctrl+,",
            icon: None,
            action: None,
        };

        assert_eq!(text_cols("打开设置"), 8);
        assert_eq!(entry_cols(&entry), 8 + 6 + 6);
    }
}
