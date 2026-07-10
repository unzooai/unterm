//! Agent Inbox — the cockpit's "who wants me" palette.
//!
//! One modal card listing every pane that hosts an AI agent, sorted
//! waiting-first (then working / done / idle). Enter or click jumps to
//! that pane, wherever it lives. Two persistent action rows expose the
//! rest of the cockpit: Launch fleet… and Open review.
//!
//! Implementation follows the DirJump modal pattern: RefCell state, a
//! cached ComputedElement tree invalidated on interaction, and a
//! signature check so the card live-refreshes when agent states change
//! under it (the 2s cockpit tick repaints the window; recompute happens
//! only when the snapshot actually changed).

use crate::cockpit::{AgentState, PaneAgentStatus};
use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::render::corners::{
    TOP_LEFT_ROUNDED_CORNER, TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::{DimensionContext, TermWindow, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::Dimension;
use mux::Mux;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use wezterm_term::{KeyCode, KeyModifiers};
use window::color::LinearRgba;
use window::WindowOps;

const MAX_ROWS: usize = 12;

#[derive(Clone)]
pub enum InboxRow {
    Agent {
        status: PaneAgentStatus,
        pane_title: String,
    },
    LaunchFleet,
    OpenReview,
}

pub struct CockpitInbox {
    rows: RefCell<Vec<InboxRow>>,
    selected: RefCell<usize>,
    element: RefCell<Option<Vec<ComputedElement>>>,
    /// Hash of the agent snapshot the cached element was built from.
    sig: RefCell<u64>,
}

fn snapshot_rows() -> (Vec<InboxRow>, u64) {
    let mux = Mux::get();
    let mut hasher = DefaultHasher::new();
    let mut rows: Vec<InboxRow> = crate::cockpit::snapshot()
        .into_iter()
        .take(MAX_ROWS)
        .map(|status| {
            status.pane_id.hash(&mut hasher);
            status.agent.hash(&mut hasher);
            status.state.as_str().hash(&mut hasher);
            status.task_hint.hash(&mut hasher);
            let pane_title = mux
                .get_pane(status.pane_id as mux::pane::PaneId)
                .map(|p| p.get_title())
                .unwrap_or_default();
            InboxRow::Agent { status, pane_title }
        })
        .collect();
    rows.push(InboxRow::LaunchFleet);
    rows.push(InboxRow::OpenReview);
    (rows, hasher.finish())
}

impl CockpitInbox {
    pub fn new() -> Self {
        let (rows, sig) = snapshot_rows();
        Self {
            rows: RefCell::new(rows),
            selected: RefCell::new(0),
            element: RefCell::new(None),
            sig: RefCell::new(sig),
        }
    }

    fn invalidate(&self, term_window: &TermWindow) {
        self.element.borrow_mut().take();
        if let Some(window) = term_window.window.as_ref() {
            window.invalidate();
        }
    }

    fn move_selection(&self, delta: isize, term_window: &TermWindow) {
        let len = self.rows.borrow().len();
        if len == 0 {
            return;
        }
        let mut sel = self.selected.borrow_mut();
        let next = (*sel as isize + delta).rem_euclid(len as isize) as usize;
        *sel = next;
        drop(sel);
        self.invalidate(term_window);
    }

    pub fn hover_select(&self, idx: usize, term_window: &TermWindow) {
        if idx < self.rows.borrow().len() && *self.selected.borrow() != idx {
            *self.selected.borrow_mut() = idx;
            self.invalidate(term_window);
        }
    }

    pub fn activate(&self, idx: usize, term_window: &mut TermWindow) {
        let row = match self.rows.borrow().get(idx) {
            Some(r) => r.clone(),
            None => return,
        };
        term_window.cancel_modal();
        match row {
            InboxRow::Agent { status, .. } => {
                let mux = Mux::get();
                if let Err(err) =
                    mux.focus_pane_and_containing_tab(status.pane_id as mux::pane::PaneId)
                {
                    log::warn!("inbox: focus pane {}: {err:#}", status.pane_id);
                }
            }
            InboxRow::LaunchFleet => {
                term_window.show_fleet_palette();
            }
            InboxRow::OpenReview => {
                term_window.open_web_settings_fragment(Some("review"));
            }
        }
    }

    fn state_style(state: AgentState) -> (&'static str, LinearRgba) {
        match state {
            AgentState::WaitingForUser => {
                ("\u{270b}", LinearRgba::with_srgba(0xe5, 0xc0, 0x7b, 0xff))
            }
            AgentState::Working => ("\u{26a1}", LinearRgba::with_srgba(0x61, 0xaf, 0xef, 0xff)),
            AgentState::Done => ("\u{2713}", LinearRgba::with_srgba(0x98, 0xc3, 0x79, 0xff)),
            AgentState::Idle => ("\u{00b7}", LinearRgba::with_srgba(0xac, 0xac, 0xa8, 0xff)),
        }
    }

    fn row_label(row: &InboxRow) -> String {
        match row {
            InboxRow::Agent { status, pane_title } => {
                let mins = status.since.elapsed().as_secs() / 60;
                let dur = if mins >= 60 {
                    format!("{}h{:02}m", mins / 60, mins % 60)
                } else if mins > 0 {
                    format!("{mins}m")
                } else {
                    format!("{}s", status.since.elapsed().as_secs())
                };
                let hint = status
                    .task_hint
                    .as_deref()
                    .filter(|h| !h.is_empty())
                    .unwrap_or(pane_title.as_str());
                let fleet = status
                    .fleet_id
                    .as_deref()
                    .map(|f| format!("\u{26f5}{f} · "))
                    .unwrap_or_default();
                format!(
                    "{fleet}{}  {} · {} · {}",
                    status.agent,
                    crate::i18n::t(match status.state {
                        AgentState::WaitingForUser => "cockpit.state_waiting",
                        AgentState::Working => "cockpit.state_working",
                        AgentState::Done => "cockpit.state_done",
                        AgentState::Idle => "cockpit.state_idle",
                    }),
                    dur,
                    hint,
                )
            }
            InboxRow::LaunchFleet => format!("\u{26f5} {}", crate::i18n::t("cockpit.launch_fleet")),
            InboxRow::OpenReview => format!("\u{21c4} {}", crate::i18n::t("cockpit.open_review")),
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
        let hover_bg = LinearRgba::with_srgba(0x3d, 0x3d, 0x3d, 0xff);

        let selected = *self.selected.borrow();
        let rows = self.rows.borrow();

        let card_width = (520. * pt)
            .min(term_window.dimensions.pixel_width as f32 - 32. * pt)
            .round();
        let row_cols = (((card_width - (2. + 40. * pt)) / metrics.cell_size.width as f32)
            .floor()
            .max(8.)) as usize;

        let mut children: Vec<Element> = vec![];

        // Title row.
        children.push(
            Element::new(
                &font,
                ElementContent::Text(crate::i18n::t("cockpit.inbox_title")),
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

        for (idx, row) in rows.iter().enumerate() {
            let is_sel = idx == selected;
            let (glyph, accent) = match row {
                InboxRow::Agent { status, .. } => Self::state_style(status.state),
                InboxRow::LaunchFleet | InboxRow::OpenReview => {
                    ("", LinearRgba::with_srgba(0x6f, 0xcc, 0xb8, 0xff))
                }
            };
            let mut label = Self::row_label(row);
            // Ellipsize to the card's width (the box model doesn't clip).
            let cols = termwiz::cell::unicode_column_width(&label, None);
            if cols > row_cols {
                let mut acc = String::new();
                let mut w = 0;
                for ch in label.chars() {
                    let cw = termwiz::cell::unicode_column_width(&ch.to_string(), None);
                    if w + cw > row_cols.saturating_sub(1) {
                        break;
                    }
                    w += cw;
                    acc.push(ch);
                }
                acc.push('\u{2026}');
                label = acc;
            }

            let mut kids = vec![];
            if !glyph.is_empty() {
                kids.push(
                    Element::new(&font, ElementContent::Text(format!("{glyph} "))).colors(
                        ElementColors {
                            border: BorderColor::default(),
                            bg: LinearRgba::TRANSPARENT.into(),
                            text: accent.into(),
                        },
                    ),
                );
            }
            kids.push(
                Element::new(&font, ElementContent::Text(label)).colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: if matches!(row, InboxRow::Agent { .. }) {
                        fg.into()
                    } else {
                        accent.into()
                    },
                }),
            );

            children.push(
                Element::new(&font, ElementContent::Children(kids))
                    .item_type(UIItemType::CockpitInboxRow(idx))
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

        // Footer hint.
        children.push(
            Element::new(
                &font,
                ElementContent::Text(crate::i18n::t("cockpit.inbox_footer")),
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
        let size = term_window.terminal_size;
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
                    (size.pixel_height as f32 * 0.12).round(),
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

impl Modal for CockpitInbox {
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
            (KeyCode::Enter, KeyModifiers::NONE) | (KeyCode::Tab, KeyModifiers::NONE) => {
                let sel = *self.selected.borrow();
                self.activate(sel, term_window);
            }
            _ => {}
        }
        Ok(true)
    }

    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<std::cell::Ref<[ComputedElement]>> {
        // Live refresh: rebuild when the agent snapshot changed.
        let (rows, sig) = snapshot_rows();
        if *self.sig.borrow() != sig {
            *self.rows.borrow_mut() = rows;
            *self.sig.borrow_mut() = sig;
            let len = self.rows.borrow().len();
            let mut sel = self.selected.borrow_mut();
            if *sel >= len {
                *sel = len.saturating_sub(1);
            }
            drop(sel);
            self.element.borrow_mut().take();
        }
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
