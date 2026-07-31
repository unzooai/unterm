use super::{command::RuntimeCommand, scheduler};
use crate::{
    CursorSnapshot, RenderFrameSnapshot, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, StyledScreenSnapshot, StyledScrollbackSnapshot,
};
use anyhow::Result;

pub(in crate::next_core) fn scroll_viewport_to(pane_id: usize, target: isize) -> Result<()> {
    scheduler::scroll_viewport_to(pane_id, target)
}

pub(in crate::next_core) fn scroll_viewport_by(pane_id: usize, delta: isize) -> Result<()> {
    scheduler::scroll_viewport_by(pane_id, delta)
}

pub(in crate::next_core) fn scroll_viewport_to_prompt(pane_id: usize, amount: isize) -> Result<()> {
    scheduler::scroll_viewport_to_prompt(pane_id, amount)
}

pub(in crate::next_core) fn pane_modes(pane_id: usize) -> Result<crate::PaneModesSnapshot> {
    scheduler::pane_modes(pane_id)
}

pub(in crate::next_core) fn erase_scrollback(pane_id: usize, include_viewport: bool) -> Result<()> {
    scheduler::erase_scrollback(pane_id, include_viewport)
}

pub(in crate::next_core) fn screen_revision(pane_id: usize) -> Result<u64> {
    scheduler::screen_revision(pane_id)
}

pub(in crate::next_core) fn read_screen(pane_id: usize) -> Result<ScreenSnapshot> {
    scheduler::read_screen(pane_id)
}

pub(in crate::next_core) fn read_styled_screen(pane_id: usize) -> Result<StyledScreenSnapshot> {
    scheduler::read_styled_screen(pane_id)
}

pub(in crate::next_core) fn read_render_frame(
    pane_id: usize,
    since_revision: Option<u64>,
) -> Result<RenderFrameSnapshot> {
    scheduler::read_render_frame(pane_id, since_revision)
}

pub(in crate::next_core) fn read_visible_text(pane_id: usize) -> Result<String> {
    scheduler::read_visible_text(pane_id)
}

pub(in crate::next_core) fn read_lines(
    pane_id: usize,
    start: i64,
    count: usize,
) -> Result<Vec<ScreenLine>> {
    scheduler::read_lines(pane_id, start, count)
}

pub(in crate::next_core) fn read_scrollback(pane_id: usize, limit: usize) -> Result<Vec<String>> {
    scheduler::read_scrollback(pane_id, limit)
}

pub(in crate::next_core) fn read_scrollback_text(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<ScrollbackTextSnapshot> {
    scheduler::read_scrollback_text(pane_id, request)
}

pub(in crate::next_core) fn read_styled_scrollback(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<StyledScrollbackSnapshot> {
    scheduler::read_styled_scrollback(pane_id, request)
}

pub(in crate::next_core) fn search_screen(
    pane_id: usize,
    pattern: &str,
    mode: crate::SearchMode,
    max_results: usize,
) -> Result<Vec<ScreenSearchMatch>> {
    scheduler::search_screen(pane_id, pattern, mode, max_results)
}

pub(in crate::next_core) fn cursor(pane_id: usize) -> Result<CursorSnapshot> {
    scheduler::cursor(pane_id)
}

pub(in crate::next_core) fn write_input(pane_id: usize, input: &str) -> Result<()> {
    scheduler::submit_input(RuntimeCommand::WriteInput {
        pane_id,
        text: input.to_string(),
    })
}

pub(in crate::next_core) fn paste_input(pane_id: usize, text: &str) -> Result<()> {
    scheduler::submit_input(RuntimeCommand::PasteInput {
        pane_id,
        text: text.to_string(),
    })
}

pub(in crate::next_core) fn report_mouse(
    pane_id: usize,
    event: crate::next_core::mouse_encoding::MouseEvent,
) -> Result<()> {
    scheduler::submit_input(RuntimeCommand::ReportMouse { pane_id, event })
}
