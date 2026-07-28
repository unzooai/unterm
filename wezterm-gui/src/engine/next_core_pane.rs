//! A mux pane backed directly by a next-core session.
//!
//! The `UNTERM_NEXT_CORE_WEBGPU_PANE=replace` path paints next-core over a
//! `LocalPane` and steals its input, which means two shells are alive per
//! pane: the one the user drives and the hidden one the mux still owns. This
//! type removes the second shell — the mux pane *is* the next-core session, so
//! the mux keeps only geometry and lifecycle.
//!
//! Everything here reads through the engine's public surface rather than
//! next-core internals, so the pane stays on the same contract the MCP layer
//! uses.

use crate::engine::{next_core, InputEngine, ScreenEngine, SessionEngine};
use mux::domain::DomainId;
use mux::pane::{
    CachePolicy, ForEachPaneLogicalLine, LogicalLine, Pane, PaneId, WithPaneLines,
};
use mux::renderable::{RenderableDimensions, StableCursorPosition};
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use std::ops::Range;
use termwiz::input::{KeyCode, Modifiers as KeyModifiers};
use termwiz::surface::{Line, SequenceNo};
use unterm_engine::{ScrollbackTextRequest, StyledCell, StyledScreenLine};
use url::Url;
use wezterm_term::color::ColorPalette;
use rangeset::RangeSet;
use wezterm_term::{Cell, Hyperlink, MouseEvent, StableRowIndex, TerminalSize};

/// Forwards `std::io::Write` into a next-core session.
///
/// `Pane::writer` hands callers a raw byte sink. next-core's own input path is
/// the only way in, so the bytes are handed to `write_input`; that applies the
/// session's application-cursor translation, which is the correct behaviour
/// for a terminal writing cursor sequences on the app's behalf.
struct NextCorePaneWriter {
    session_id: usize,
}

impl std::io::Write for NextCorePaneWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Bytes from the GUI are already UTF-8 escape sequences; a split
        // multi-byte sequence would be a caller bug, so surface it rather
        // than silently writing replacement characters.
        let text = std::str::from_utf8(buf).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("next-core pane writer received non-UTF-8 input: {err}"),
            )
        })?;
        next_core()
            .write_input(self.session_id, text)
            .map_err(|err| std::io::Error::other(format!("{err:#}")))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // `write_input` flushes the PTY writer itself.
        Ok(())
    }
}

pub struct NextCorePane {
    pane_id: PaneId,
    session_id: usize,
    domain_id: DomainId,
    writer: Mutex<NextCorePaneWriter>,
}

impl NextCorePane {
    pub fn new(pane_id: PaneId, session_id: usize, domain_id: DomainId) -> Self {
        Self {
            pane_id,
            session_id,
            domain_id,
            writer: Mutex::new(NextCorePaneWriter { session_id }),
        }
    }

    pub fn session_id(&self) -> usize {
        self.session_id
    }

    /// Read a stable row range as styled lines.
    ///
    /// `StableRowIndex` counts from the top of the scrollback, which is
    /// exactly what next-core's scrollback request takes, so the range passes
    /// through unchanged.
    fn styled_rows(&self, lines: Range<StableRowIndex>) -> Option<(StableRowIndex, Vec<Line>)> {
        let request = ScrollbackTextRequest {
            start_line: Some(lines.start as i64),
            end_line: Some(lines.end as i64),
            tail_lines: None,
            escapes: false,
        };
        let snapshot = next_core()
            .read_styled_scrollback(self.session_id, request)
            .ok()?;
        let first = snapshot.first_row as StableRowIndex;
        let cols = snapshot.cols;
        let rendered = snapshot
            .lines
            .iter()
            .map(|line| styled_line_to_line(line, cols))
            .collect();
        Some((first, rendered))
    }
}

/// Convert one next-core styled row into a wezterm `Line`.
fn styled_line_to_line(line: &StyledScreenLine, cols: usize) -> Line {
    // Seqno 0: next-core tracks damage through its own revision counter, and
    // the mux only compares seqnos it issued itself.
    let mut rendered = Line::with_width(cols.max(line.cells.len()), 0);
    let mut idx = 0;
    for cell in &line.cells {
        if idx >= rendered.len() {
            break;
        }
        rendered.set_cell(idx, styled_cell_to_cell(cell), 0);
        // A wide cell occupies its trailing columns; leaving them as blanks
        // keeps the column arithmetic aligned with next-core's own grid.
        idx += cell.width.max(1);
    }
    rendered
}

fn styled_cell_to_cell(cell: &StyledCell) -> Cell {
    let mut attrs = crate::scrollshot::styled_cell_attributes(&cell.style);
    if let Some(hyperlink) = cell.style.hyperlink.as_deref() {
        // OSC 8 links survive the round trip; `styled_cell_attributes` is
        // shared with the PNG renderer, which has no use for them.
        attrs.set_hyperlink(Some(std::sync::Arc::new(Hyperlink::new(hyperlink))));
    }
    Cell::new(cell.ch, attrs)
}

impl Pane for NextCorePane {
    fn pane_id(&self) -> PaneId {
        self.pane_id
    }

    fn get_cursor_position(&self) -> StableCursorPosition {
        let Ok(screen) = next_core().read_screen(self.session_id) else {
            return StableCursorPosition::default();
        };
        StableCursorPosition {
            x: screen.cursor.x,
            y: screen.cursor.y as StableRowIndex,
            shape: termwiz::surface::CursorShape::Default,
            visibility: if screen.cursor.visible {
                termwiz::surface::CursorVisibility::Visible
            } else {
                termwiz::surface::CursorVisibility::Hidden
            },
        }
    }

    fn get_current_seqno(&self) -> SequenceNo {
        // next-core's screen revision is monotonic per session, which is the
        // contract the mux wants from a seqno.
        next_core()
            .read_screen(self.session_id)
            .map(|screen| screen.revision as SequenceNo)
            .unwrap_or(0)
    }

    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
        self.styled_rows(lines.clone())
            .unwrap_or((lines.start, Vec::new()))
    }

    fn with_lines_mut(&self, lines: Range<StableRowIndex>, with_lines: &mut dyn WithPaneLines) {
        let (first, mut rendered) = self.get_lines(lines);
        let mut refs: Vec<&mut Line> = rendered.iter_mut().collect();
        with_lines.with_lines_mut(first, &mut refs);
    }

    fn for_each_logical_line_in_stable_range_mut(
        &self,
        lines: Range<StableRowIndex>,
        for_line: &mut dyn ForEachPaneLogicalLine,
    ) {
        mux::pane::impl_for_each_logical_line_via_get_logical_lines(self, lines, for_line)
    }

    fn get_logical_lines(&self, lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
        mux::pane::impl_get_logical_lines_via_get_lines(self, lines)
    }

    fn get_dimensions(&self) -> RenderableDimensions {
        let Ok(screen) = next_core().read_screen(self.session_id) else {
            return RenderableDimensions::default();
        };
        let scrollback_rows = screen.scrollback_rows.max(screen.rows);
        let physical_top = scrollback_rows.saturating_sub(screen.rows) as StableRowIndex;
        RenderableDimensions {
            cols: screen.cols,
            viewport_rows: screen.rows,
            scrollback_rows,
            physical_top,
            scrollback_top: 0,
            dpi: 0,
            pixel_width: 0,
            pixel_height: 0,
            reverse_video: false,
        }
    }

    fn get_title(&self) -> String {
        next_core()
            .get_session(self.session_id)
            .map(|session| session.title)
            .unwrap_or_default()
    }

    fn send_paste(&self, text: &str) -> anyhow::Result<()> {
        next_core().paste_input(self.session_id, text)
    }

    fn reader(&self) -> anyhow::Result<Option<Box<dyn std::io::Read + Send>>> {
        // next-core owns its PTY reader and feeds its own screen; nothing
        // outside may drain the same fd.
        Ok(None)
    }

    fn writer(&self) -> MappedMutexGuard<'_, dyn std::io::Write> {
        MutexGuard::map(self.writer.lock(), |writer| {
            let w: &mut dyn std::io::Write = writer;
            w
        })
    }

    fn resize(&self, size: TerminalSize) -> anyhow::Result<()> {
        next_core().resize_session(self.session_id, size.cols, size.rows)
    }

    fn key_down(&self, key: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
        let Some(encoded) = unterm_engine::next_core::key_encoding::encode_key(key, mods) else {
            // Modifier keys and SUPER chords produce no PTY input.
            return Ok(());
        };
        next_core().write_input(self.session_id, &encoded)
    }

    fn key_up(&self, _key: KeyCode, _mods: KeyModifiers) -> anyhow::Result<()> {
        // Key-up carries no input outside the kitty and win32 protocols,
        // neither of which next-core implements.
        Ok(())
    }

    fn mouse_event(&self, event: MouseEvent) -> anyhow::Result<()> {
        let Some(event) = crate::termwindow::next_core_mouse_event(&event) else {
            return Ok(());
        };
        next_core().report_mouse(self.session_id, event)
    }

    fn is_dead(&self) -> bool {
        next_core()
            .get_session(self.session_id)
            .map(|session| session.is_dead)
            // A session the engine no longer knows about is gone, which the
            // mux must see as dead or it will keep the pane forever.
            .unwrap_or(true)
    }

    fn palette(&self) -> ColorPalette {
        ColorPalette::default()
    }

    fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    fn is_mouse_grabbed(&self) -> bool {
        next_core()
            .pane_modes(self.session_id)
            .map(|modes| modes.mouse_grabbed)
            .unwrap_or(false)
    }

    fn is_alt_screen_active(&self) -> bool {
        next_core()
            .pane_modes(self.session_id)
            .map(|modes| modes.alt_screen_active)
            .unwrap_or(false)
    }

    fn get_current_working_dir(&self, _policy: CachePolicy) -> Option<Url> {
        let cwd = next_core().get_session(self.session_id).ok()?.shell.cwd?;
        Url::from_directory_path(&cwd).ok()
    }

    fn get_changed_since(
        &self,
        lines: Range<StableRowIndex>,
        seqno: SequenceNo,
    ) -> RangeSet<StableRowIndex> {
        // next-core reports damage through its screen revision, not per-row
        // seqnos. Treating the whole range as changed whenever the revision
        // moved is correct but coarse; a stale seqno must never under-report,
        // or the renderer would keep painting old rows.
        let mut changed = RangeSet::new();
        if self.get_current_seqno() > seqno {
            changed.add_range(lines);
        }
        changed
    }
}

impl NextCorePane {
    /// Write raw bytes to the session. Mirrors `LocalPane::write_bytes`,
    /// which is an inherent method rather than part of the `Pane` trait.
    #[allow(dead_code)]
    pub fn write_bytes(&self, bytes: &[u8]) {
        if let Ok(text) = std::str::from_utf8(bytes) {
            if let Err(err) = next_core().write_input(self.session_id, text) {
                log::warn!(
                    "next-core pane {} (session {}) write_bytes failed: {err:#}",
                    self.pane_id,
                    self.session_id
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{CreateSessionRequest, LaunchPolicySnapshot};
    use unterm_engine::CellStyle;

    fn styled(ch: char, width: usize) -> StyledCell {
        StyledCell {
            ch,
            style: CellStyle::default(),
            width,
        }
    }

    #[test]
    fn styled_line_becomes_a_line_of_the_requested_width() {
        let line = StyledScreenLine {
            row: 0,
            cells: vec![styled('a', 1), styled('b', 1)],
        };

        let rendered = styled_line_to_line(&line, 8);

        assert_eq!(rendered.len(), 8);
        assert_eq!(rendered.as_str().trim_end(), "ab");
    }

    #[test]
    fn wide_cells_consume_their_trailing_column() {
        // A double-width glyph occupies two columns, so the next cell must
        // land at index 2 rather than overwriting the trailing half.
        let line = StyledScreenLine {
            row: 0,
            cells: vec![styled('你', 2), styled('x', 1)],
        };

        let rendered = styled_line_to_line(&line, 4);

        assert_eq!(rendered.get_cell(0).map(|c| c.str().to_string()).as_deref(), Some("你"));
        assert_eq!(rendered.get_cell(2).map(|c| c.str().to_string()).as_deref(), Some("x"));
    }

    #[test]
    fn a_line_wider_than_the_screen_is_not_truncated() {
        // next-core can report a row longer than the reported column count
        // (a resize in flight); dropping cells would lose text.
        let line = StyledScreenLine {
            row: 0,
            cells: vec![styled('a', 1), styled('b', 1), styled('c', 1)],
        };

        let rendered = styled_line_to_line(&line, 2);

        assert_eq!(rendered.len(), 3);
        assert_eq!(rendered.as_str().trim_end(), "abc");
    }

    #[test]
    fn hyperlinks_survive_the_conversion() {
        let mut style = CellStyle::default();
        style.hyperlink = Some("https://example.invalid/".to_string());
        let cell = StyledCell {
            ch: 'l',
            style,
            width: 1,
        };

        let converted = styled_cell_to_cell(&cell);

        assert_eq!(
            converted
                .attrs()
                .hyperlink()
                .map(|link| link.uri().to_string())
                .as_deref(),
            Some("https://example.invalid/")
        );
    }

    /// `get_lines` is what the renderer calls every frame, and it is where a
    /// stable-row-index mistake would show up as text drawn at the wrong row.
    /// Drive a real shell and read its output back through the trait.
    #[test]
    fn get_lines_returns_real_session_output_at_the_reported_rows() -> anyhow::Result<()> {
        let session = next_core().create_session(CreateSessionRequest {
            cols: 60,
            rows: 8,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: LaunchPolicySnapshot::default(),
        })?;
        let pane = NextCorePane::new(2, session.id, 0);

        let marker = "nc-pane-getlines";
        next_core().write_input(session.id, &format!("echo {marker}\r"))?;

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let found = loop {
            let dims = pane.get_dimensions();
            let (first, lines) = pane.get_lines(
                dims.physical_top..dims.physical_top + dims.viewport_rows as StableRowIndex,
            );
            // The row the caller was told the range starts at must be the row
            // it asked for, or every row lands offset on screen.
            assert_eq!(
                first, dims.physical_top,
                "get_lines reported a different first row than requested"
            );
            // The command's own output line, not the echoed command line.
            let hit = lines
                .iter()
                .any(|line| line.as_str().contains(marker) && !line.as_str().contains("echo "));
            if hit {
                break true;
            }
            if std::time::Instant::now() >= deadline {
                break false;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        };

        next_core().destroy_session(session.id)?;
        assert!(found, "shell output never appeared through get_lines");
        Ok(())
    }

    /// The pane reads a real session end to end: dimensions, title, liveness,
    /// and screen text all come from next-core rather than a mux terminal.
    #[test]
    fn pane_reads_a_live_next_core_session() -> anyhow::Result<()> {
        let session = next_core().create_session(CreateSessionRequest {
            cols: 40,
            rows: 6,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: LaunchPolicySnapshot::default(),
        })?;
        let pane = NextCorePane::new(1, session.id, 0);

        assert_eq!(pane.pane_id(), 1);
        assert_eq!(pane.session_id(), session.id);
        assert!(!pane.is_dead(), "a fresh session must not read as dead");

        let dims = pane.get_dimensions();
        assert_eq!(dims.cols, 40);
        assert_eq!(dims.viewport_rows, 6);

        // Modes start clear: nothing has negotiated mouse tracking or the
        // alternate screen yet.
        assert!(!pane.is_mouse_grabbed());
        assert!(!pane.is_alt_screen_active());

        next_core().destroy_session(session.id)?;
        // A destroyed session must read as dead, or the mux keeps the pane.
        assert!(pane.is_dead());
        Ok(())
    }
}
