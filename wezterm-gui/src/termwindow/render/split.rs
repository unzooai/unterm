use crate::termwindow::render::TripleLayerQuadAllocator;
use crate::termwindow::{UIItem, UIItemType};
use mux::pane::Pane;
use mux::tab::{PositionedSplit, SplitDirection};
use std::sync::Arc;

impl crate::TermWindow {
    pub fn paint_split(
        &mut self,
        layers: &mut TripleLayerQuadAllocator,
        split: &PositionedSplit,
        pane: &Arc<dyn Pane>,
    ) -> anyhow::Result<()> {
        let palette = pane.palette();
        let foreground = palette.split.to_linear();
        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;
        // Thicker than the 1px hairline upstream draws: the divider is now the
        // primary way the two pane areas are told apart (no per-pane frame /
        // accent), so make it clearly visible. Kept in the theme's `split`
        // color so it stays coherent across light/dark schemes. Centered on
        // the gutter so thickening doesn't shift it onto either pane.
        let divider_thickness = (self.render_metrics.underline_height as f32 * 2.5).max(3.0);

        let border = self.get_os_border();
        let first_row_offset = if self.show_tab_bar && !self.config.tab_bar_at_bottom {
            self.tab_bar_pixel_height()?
        } else {
            0.
        } + border.top.get() as f32;

        let (padding_left, padding_top) = self.padding_left_top();

        let pos_y = split.top as f32 * cell_height + first_row_offset + padding_top;
        let pos_x = split.left as f32 * cell_width + padding_left + border.left.get() as f32;

        if split.direction == SplitDirection::Horizontal {
            // The divider is drawn ~1 cell taller than the pane rows (the
            // `1. +` plus the half-cell start offset) so it visually bridges
            // the half-cell gutter the pane backgrounds use. Upstream WezTerm
            // has no bottom chrome, but Unterm paints a status bar (and an
            // optional suggest bar) below the panes — left unclamped, that
            // extra half-cell punches a vertical line straight through the
            // status-bar text. Clamp the bottom to the top of the status bar.
            // End the divider at the pane's actual content bottom
            // (pos_y + rows·cell), not the upstream `(1.+size)·cell` height.
            // That extra cell — plus Unterm's status bar sitting one cell off
            // the window bottom — made the divider's tail land right on the
            // status-bar text row, slicing through it (e.g. "t|heme:classic").
            // Clamping to the content bottom leaves the status bar untouched.
            let divider_top = pos_y - (cell_height / 2.0);
            let pane_content_bottom = pos_y + split.size as f32 * cell_height;
            let divider_h = (pane_content_bottom - divider_top).max(0.0);
            self.filled_rectangle(
                layers,
                2,
                euclid::rect(
                    pos_x + (cell_width / 2.0) - (divider_thickness / 2.0),
                    divider_top,
                    divider_thickness,
                    divider_h,
                ),
                foreground,
            )?;
            self.ui_items.push(UIItem {
                x: border.left.get() as usize
                    + padding_left as usize
                    + (split.left * cell_width as usize),
                width: cell_width as usize,
                y: padding_top as usize
                    + first_row_offset as usize
                    + split.top * cell_height as usize,
                height: split.size * cell_height as usize,
                pane_id: None,
                item_type: UIItemType::Split(split.clone()),
            });
        } else {
            self.filled_rectangle(
                layers,
                2,
                euclid::rect(
                    pos_x - (cell_width / 2.0),
                    pos_y + (cell_height / 2.0) - (divider_thickness / 2.0),
                    (1.0 + split.size as f32) * cell_width,
                    divider_thickness,
                ),
                foreground,
            )?;
            self.ui_items.push(UIItem {
                x: border.left.get() as usize
                    + padding_left as usize
                    + (split.left * cell_width as usize),
                width: split.size * cell_width as usize,
                y: padding_top as usize
                    + first_row_offset as usize
                    + split.top * cell_height as usize,
                height: cell_height as usize,
                pane_id: None,
                item_type: UIItemType::Split(split.clone()),
            });
        }

        Ok(())
    }
}
