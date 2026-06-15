use crate::termwindow::{RenderFrame, TermWindowNotif};
use ::window::bitmaps::atlas::OutOfTextureSpace;
use ::window::WindowOps;
use anyhow::Context;
use smol::Timer;
use std::time::{Duration, Instant};
use wezterm_font::ClearShapeCache;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowImage {
    Yes,
    Scale(usize),
    No,
}

impl crate::TermWindow {
    pub fn paint_impl(&mut self, frame: &mut RenderFrame) {
        self.num_frames += 1;
        // If nothing on screen needs animating, then we can avoid
        // invalidating as frequently
        *self.has_animation.borrow_mut() = None;
        // Start with the assumption that we should allow images to render
        self.allow_images = AllowImage::Yes;

        let start = Instant::now();

        {
            let diff = start.duration_since(self.last_fps_check_time);
            if diff > Duration::from_secs(1) {
                let seconds = diff.as_secs_f32();
                self.fps = self.num_frames as f32 / seconds;
                self.num_frames = 0;
                self.last_fps_check_time = start;
            }
        }

        'pass: for pass in 0.. {
            match self.paint_pass() {
                Ok(_) => match self.render_state.as_mut().unwrap().allocated_more_quads() {
                    Ok(allocated) => {
                        if !allocated {
                            break 'pass;
                        }
                        self.invalidate_fancy_tab_bar();
                        self.invalidate_modal();
                    }
                    Err(err) => {
                        log::error!("{:#}", err);
                        break 'pass;
                    }
                },
                Err(err) => {
                    if let Some(&OutOfTextureSpace {
                        size: Some(size),
                        current_size,
                    }) = err.root_cause().downcast_ref::<OutOfTextureSpace>()
                    {
                        let result = if pass == 0 {
                            // Let's try clearing out the atlas and trying again
                            // self.clear_texture_atlas()
                            log::trace!("recreate_texture_atlas");
                            self.recreate_texture_atlas(Some(current_size))
                        } else {
                            log::trace!("grow texture atlas to {}", size);
                            self.recreate_texture_atlas(Some(size))
                        };
                        self.invalidate_fancy_tab_bar();
                        self.invalidate_modal();

                        if let Err(err) = result {
                            self.allow_images = match self.allow_images {
                                AllowImage::Yes => AllowImage::Scale(2),
                                AllowImage::Scale(2) => AllowImage::Scale(4),
                                AllowImage::Scale(4) => AllowImage::Scale(8),
                                AllowImage::Scale(8) => AllowImage::No,
                                AllowImage::No | _ => {
                                    log::error!(
                                        "Failed to {} texture: {}",
                                        if pass == 0 { "clear" } else { "resize" },
                                        err
                                    );
                                    break 'pass;
                                }
                            };

                            log::info!(
                                "Not enough texture space ({:#}); \
                                     will retry render with {:?}",
                                err,
                                self.allow_images,
                            );
                        }
                    } else if err.root_cause().downcast_ref::<ClearShapeCache>().is_some() {
                        self.invalidate_fancy_tab_bar();
                        self.invalidate_modal();
                        self.shape_generation += 1;
                        self.shape_cache.borrow_mut().clear();
                        self.line_to_ele_shape_cache.borrow_mut().clear();
                    } else {
                        log::error!("paint_pass failed: {:#}", err);
                        break 'pass;
                    }
                }
            }
        }
        log::debug!("paint_impl before call_draw elapsed={:?}", start.elapsed());

        self.call_draw(frame).ok();
        self.last_frame_duration = start.elapsed();
        log::debug!(
            "paint_impl elapsed={:?}, fps={}",
            self.last_frame_duration,
            self.fps
        );
        metrics::histogram!("gui.paint.impl").record(self.last_frame_duration);
        metrics::histogram!("gui.paint.impl.rate").record(1.);

        // If self.has_animation is some, then the last render detected
        // image attachments with multiple frames, so we also need to
        // invalidate the viewport when the next frame is due
        if self.focused.is_some() {
            if let Some(next_due) = *self.has_animation.borrow() {
                let prior = self.scheduled_animation.borrow_mut().take();
                match prior {
                    Some(prior) if prior <= next_due => {
                        // Already due before that time
                    }
                    _ => {
                        self.scheduled_animation.borrow_mut().replace(next_due);
                        let window = self.window.clone().take().unwrap();
                        promise::spawn::spawn(async move {
                            Timer::at(next_due).await;
                            let win = window.clone();
                            window.notify(TermWindowNotif::Apply(Box::new(move |tw| {
                                tw.scheduled_animation.borrow_mut().take();
                                win.invalidate();
                            })));
                        })
                        .detach();
                    }
                }
            }
        }

    }

    /// v0.40: left directory-tree sidebar. Painted between the panes
    /// and the tab bar; the panes have already been shifted right by
    /// tree_sidebar_pixel_width() via the padding injection, so this
    /// draws into reserved gutter space.
    pub fn paint_tree_sidebar(&mut self) -> anyhow::Result<()> {
        use crate::termwindow::box_model::*;
        use crate::termwindow::{DimensionContext, UIItemType};
        use crate::utilsprites::RenderMetrics;
        use ::window::color::LinearRgba;
        use config::Dimension;

        let width = self.tree_sidebar_pixel_width();
        if width <= 0.0 {
            return Ok(());
        }
        // Refresh lazily; redraw is already happening so changed rows just
        // paint this frame.
        if let Some(tree) = self.tree_sidebar.borrow_mut().as_mut() {
            tree.ensure_fresh();
        }

        let font = self.fonts.title_font().expect("title font");
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let pt = self.dimensions.dpi as f32 / 72.0;

        let border = self.get_os_border();
        let top_bar_height = if self.show_tab_bar && !self.config.tab_bar_at_bottom {
            self.tab_bar_pixel_height().unwrap_or(0.)
        } else {
            0.
        };
        // Match the left tab bar / terminal content top so the tree sidebar
        // also leaves the chrome's bottom seam clear instead of butting
        // against it (see left_tab_bar.rs / render/pane.rs).
        let padding_top = self.padding_left_top().1;
        let top = top_bar_height + padding_top + border.top.get() as f32;
        let status_h = if self.config.show_unterm_status_bar {
            self.status_bar_pixel_height()
        } else {
            0.
        };
        let bottom = self.dimensions.pixel_height as f32 - status_h - border.bottom.get() as f32;
        let row_h = metrics.cell_size.height as f32 + 6. * pt;
        let visible_rows = (((bottom - top) / row_h).floor() as usize).saturating_sub(1);

        let bg = LinearRgba::with_srgba(0x16, 0x16, 0x16, 0xff);
        let fg = LinearRgba::with_srgba(0xcf, 0xcf, 0xcd, 0xff);
        let dim = LinearRgba::with_srgba(0x6f, 0x6f, 0x6c, 0xff);
        // Palette-derived accent so the tree sidebar header picks up
        // the scheme's bright-cyan slot. Falls back to a sane teal if
        // the scheme didn't define `brights`.
        let teal = self
            .palette()
            .resolve_fg(wezterm_term::color::ColorAttribute::PaletteIndex(14))
            .to_linear();
        let hover_bg = LinearRgba::with_srgba(0x26, 0x26, 0x26, 0xff);

        let mut children: Vec<Element> = vec![];

        let (root_name, rows_snapshot, scroll_top) = {
            let tree = self.tree_sidebar.borrow();
            let tree = tree.as_ref().unwrap();
            // Header shows enough path to disambiguate sibling names:
            //   `/dev`            → "/dev"        (root-level basename)
            //   `/Volumes/Dev`    → "Volumes/Dev" (last two components)
            //   `/Users/alex/Code/x` → "Code/x"   (last two components)
            // A user pinged this after expanding `/dev` thinking it was
            // their `/Volumes/Dev` workspace; the lone basename gave no
            // hint of which one.
            let name = {
                let p = tree.root.as_path();
                let last = p.file_name().map(|n| n.to_string_lossy().to_string());
                let parent_last = p
                    .parent()
                    .and_then(|pp| pp.file_name())
                    .map(|n| n.to_string_lossy().to_string());
                match (last, parent_last) {
                    (Some(last), Some(parent)) if !parent.is_empty() => {
                        format!("{parent}/{last}")
                    }
                    (Some(last), _) => last,
                    (None, _) => p.display().to_string(),
                }
            };
            let rows: Vec<(String, usize, bool, bool, bool, bool)> = tree
                .rows
                .iter()
                .map(|r| {
                    let name = if r.is_parent {
                        "..".to_string()
                    } else {
                        r.path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default()
                    };
                    (
                        name,
                        r.depth,
                        r.is_dir,
                        r.expanded,
                        r.is_hidden,
                        r.is_parent,
                    )
                })
                .collect();
            (name, rows, tree.scroll_top)
        };

        // Header: ▦ root-name
        children.push(
            Element::new(&font, ElementContent::Text(format!("▦  {root_name}")))
                .item_type(UIItemType::TreeSidebarHeader)
                .hover_colors(Some(ElementColors {
                    border: BorderColor::default(),
                    bg: hover_bg.into(),
                    text: teal.into(),
                }))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .padding(BoxDimension {
                    left: Dimension::Pixels(12. * pt),
                    right: Dimension::Pixels(8. * pt),
                    top: Dimension::Pixels(6. * pt),
                    bottom: Dimension::Pixels(6. * pt),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: teal.into(),
                }),
        );

        for (i, (name, depth, is_dir, expanded, is_hidden, is_parent)) in rows_snapshot
            .iter()
            .enumerate()
            .skip(scroll_top)
            .take(visible_rows)
        {
            let glyph = if *is_parent {
                "↑ "
            } else if *is_dir {
                if *expanded {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                "  "
            };
            let text_color = if *is_hidden { dim } else { fg };
            children.push(
                Element::new(
                    &font,
                    ElementContent::Text(format!("{glyph}{name}")),
                )
                .item_type(UIItemType::TreeSidebarRow(i))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .padding(BoxDimension {
                    left: Dimension::Pixels(12. * pt + (*depth as f32) * 12. * pt),
                    right: Dimension::Pixels(6. * pt),
                    top: Dimension::Pixels(3. * pt),
                    bottom: Dimension::Pixels(3. * pt),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: text_color.into(),
                })
                .hover_colors(Some({
                    let mut hb = BorderColor::default();
                    hb.left = teal;
                    ElementColors {
                        border: hb,
                        bg: hover_bg.into(),
                        text: fg.into(),
                    }
                }))
                .border(BoxDimension {
                    left: Dimension::Pixels(2. * pt),
                    right: Dimension::Pixels(0.),
                    top: Dimension::Pixels(0.),
                    bottom: Dimension::Pixels(0.),
                }),
            );
        }

        let container = Element::new(&font, ElementContent::Children(children))
            .item_type(UIItemType::TreeSidebarBg)
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: bg.into(),
                text: fg.into(),
            })
            .min_width(Some(Dimension::Pixels(width)))
            .min_height(Some(Dimension::Pixels(bottom - top)));

        let computed = self.compute_element(
            &LayoutContext {
                height: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: self.dimensions.pixel_height as f32,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: self.dimensions.pixel_width as f32,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(border.left.get() as f32 + self.left_tab_bar_pixel_width(), top, width, bottom - top),
                metrics: &metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 20,
            },
            &container,
        )?;

        let mut ui_items = computed.ui_items();
        {
            let gl_state = self.render_state.as_ref().unwrap();
            self.render_element(&computed, gl_state, None)?;
        }
        self.ui_items.append(&mut ui_items);
        Ok(())
    }

    pub fn paint_modal(&mut self) -> anyhow::Result<()> {
        if let Some(modal) = self.get_modal() {
            for computed in modal.computed_element(self)?.iter() {
                let mut ui_items = computed.ui_items();

                let gl_state = self.render_state.as_ref().unwrap();
                self.render_element(&computed, gl_state, None)?;

                self.ui_items.append(&mut ui_items);
            }
        }

        Ok(())
    }

    pub fn paint_pass(&mut self) -> anyhow::Result<()> {
        {
            let gl_state = self.render_state.as_ref().unwrap();
            for layer in gl_state.layers.borrow().iter() {
                layer.clear_quad_allocation();
            }
        }

        // Clear out UI item positions; we'll rebuild these as we render
        self.ui_items.clear();
        self.deferred_scrollbar.borrow_mut().clear();

        let panes = self.get_panes_to_render();
        let focused = self.focused.is_some();
        let window_is_transparent =
            !self.window_background.is_empty() || self.config.window_background_opacity != 1.0;

        let start = Instant::now();
        let gl_state = self.render_state.as_ref().unwrap();
        let layer = gl_state
            .layer_for_zindex(0)
            .context("layer_for_zindex(0)")?;
        let mut layers = layer.quad_allocator();
        log::trace!("quad map elapsed {:?}", start.elapsed());
        metrics::histogram!("quad.map").record(start.elapsed());

        let mut paint_terminal_background = false;

        // Render the full window background
        match (self.window_background.is_empty(), self.allow_images) {
            (false, AllowImage::Yes | AllowImage::Scale(_)) => {
                let bg_color = self.palette().background.to_linear();

                let top = panes
                    .iter()
                    .find(|p| p.is_active)
                    .map(|p| match self.get_viewport(p.pane.pane_id()) {
                        Some(top) => top,
                        None => p.pane.get_dimensions().physical_top,
                    })
                    .unwrap_or(0);

                let loaded_any = self
                    .render_backgrounds(bg_color, top)
                    .context("render_backgrounds")?;

                if !loaded_any {
                    // Either there was a problem loading the background(s)
                    // or they haven't finished loading yet.
                    // Use the regular terminal background until that changes.
                    paint_terminal_background = true;
                }
            }
            _ if window_is_transparent => {
                // Avoid doubling up the background color: the panes
                // will render out through the padding so there
                // should be no gaps that need filling in
            }
            _ => {
                paint_terminal_background = true;
            }
        }

        if paint_terminal_background {
            // Regular window background color
            let background = if panes.len() == 1 {
                // If we're the only pane, use the pane's palette
                // to draw the padding background
                panes[0].pane.palette().background
            } else {
                self.palette().background
            }
            .to_linear()
            .mul_alpha(self.config.window_background_opacity);

            self.filled_rectangle(
                &mut layers,
                0,
                euclid::rect(
                    0.,
                    0.,
                    self.dimensions.pixel_width as f32,
                    self.dimensions.pixel_height as f32,
                ),
                background,
            )
            .context("filled_rectangle for window background")?;
        }

        let multi_pane = panes.len() > 1;
        for pos in &panes {
            if pos.is_active {
                self.update_text_cursor(pos);
                if focused {
                    pos.pane.advise_focus();
                    mux::Mux::get().record_focus_for_current_identity(pos.pane.pane_id());
                }
            }
            self.paint_pane(pos, &mut layers).context("paint_pane")?;
        }
        if multi_pane {
            // Render the per-pane × close button only when there's >1 pane;
            // a single pane would just have a button no one needs.
            for pos in &panes {
                self.paint_pane_close_button(pos, &mut layers)
                    .context("paint_pane_close_button")?;
            }
        }

        if let Some(pane) = self.get_active_pane_or_overlay() {
            let splits = self.get_splits();
            for split in &splits {
                self.paint_split(&mut layers, split, &pane)
                    .context("paint_split")?;
            }
        }

        // Flush scrollbar fills queued during pane painting — drawn after the
        // splits so the divider-riding inner bar sits on top of the line.
        {
            let queued: Vec<_> = self.deferred_scrollbar.borrow_mut().drain(..).collect();
            for (rect, color) in queued {
                self.filled_rectangle(&mut layers, 2, rect, color)
                    .context("deferred scrollbar fill")?;
            }
        }

        self.paint_left_tab_bar().context("paint_left_tab_bar")?;
        self.paint_tree_sidebar().context("paint_tree_sidebar")?;

        if self.show_tab_bar {
            self.paint_tab_bar(&mut layers).context("paint_tab_bar")?;
            // (Stats segments now flow inside the tab bar render —
            // see fancy_tab_bar.rs — so they share the chrome row
            // with the icons instead of overlapping them on a
            // separate zindex.)
        }

        self.paint_ghost_text(&mut layers)
            .context("paint_ghost_text")?;

        self.paint_suggest_bar(&mut layers)
            .context("paint_suggest_bar")?;

        self.paint_status_bar(&mut layers)
            .context("paint_status_bar")?;

        self.paint_window_borders(&mut layers)
            .context("paint_window_borders")?;
        drop(layers);
        self.paint_modal().context("paint_modal")?;

        Ok(())
    }
}
