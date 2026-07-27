use super::super::screen_dispatch;
use super::{command::RuntimeCommand, scheduler};
use crate::{
    CursorSnapshot, RenderFrameSnapshot, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, StyledScreenSnapshot, StyledScrollbackSnapshot,
};
use anyhow::Result;

pub(in crate::next_core) fn scroll_viewport_to(pane_id: usize, target: isize) -> Result<()> {
    screen_dispatch::scroll_viewport_to(pane_id, target)
}

pub(in crate::next_core) fn read_screen(pane_id: usize) -> Result<ScreenSnapshot> {
    screen_dispatch::read_plain_viewport(pane_id)
}

pub(in crate::next_core) fn read_styled_screen(pane_id: usize) -> Result<StyledScreenSnapshot> {
    screen_dispatch::read_styled_viewport(pane_id)
}

pub(in crate::next_core) fn read_render_frame(
    pane_id: usize,
    since_revision: Option<u64>,
) -> Result<RenderFrameSnapshot> {
    screen_dispatch::read_render_frame(pane_id, since_revision)
}

pub(in crate::next_core) fn read_visible_text(pane_id: usize) -> Result<String> {
    screen_dispatch::read_visible_text(pane_id)
}

pub(in crate::next_core) fn read_lines(
    pane_id: usize,
    start: i64,
    count: usize,
) -> Result<Vec<ScreenLine>> {
    screen_dispatch::read_lines(pane_id, start, count)
}

pub(in crate::next_core) fn read_scrollback(pane_id: usize, limit: usize) -> Result<Vec<String>> {
    screen_dispatch::read_scrollback(pane_id, limit)
}

pub(in crate::next_core) fn read_scrollback_text(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<ScrollbackTextSnapshot> {
    screen_dispatch::read_scrollback_text(pane_id, request)
}

pub(in crate::next_core) fn read_styled_scrollback(
    pane_id: usize,
    request: ScrollbackTextRequest,
) -> Result<StyledScrollbackSnapshot> {
    screen_dispatch::read_styled_scrollback(pane_id, request)
}

pub(in crate::next_core) fn search_screen(
    pane_id: usize,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<ScreenSearchMatch>> {
    screen_dispatch::search(pane_id, pattern, max_results)
}

pub(in crate::next_core) fn cursor(pane_id: usize) -> Result<CursorSnapshot> {
    screen_dispatch::cursor(pane_id)
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
