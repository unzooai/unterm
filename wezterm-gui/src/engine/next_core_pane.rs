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

/// How often the change watcher samples the session's revision.
///
/// The mux normally learns about output from its PTY reader thread; next-core
/// runs its own pump, so nothing would tell the GUI to repaint. Sampling at
/// roughly frame rate keeps a next-core pane as live as a local one without
/// spinning.
const CHANGE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(8);

pub struct NextCorePane {
    pane_id: PaneId,
    session_id: usize,
    domain_id: DomainId,
    writer: Mutex<NextCorePaneWriter>,
    /// Cleared on drop to stop the change watcher.
    watching: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Latest screen revision the change watcher saw. Sampled rather than
    /// queried, so the per-frame `has_unseen_output` check costs an atomic
    /// load instead of a trip through the engine's command queue.
    last_revision: std::sync::Arc<std::sync::atomic::AtomicU64>,
    focused: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Revision at the moment focus was lost. Output past this point is what
    /// the user has not seen.
    revision_at_focus_loss: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl NextCorePane {
    pub fn new(pane_id: PaneId, session_id: usize, domain_id: DomainId) -> Self {
        Self {
            pane_id,
            session_id,
            domain_id,
            writer: Mutex::new(NextCorePaneWriter { session_id }),
            watching: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_revision: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            // Starts unfocused, matching a wezterm terminal: a pane opened in
            // the background should show its output as unseen until looked at.
            focused: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            revision_at_focus_loss: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Tell the mux about output so the GUI repaints.
    ///
    /// A local pane gets this from the mux's PTY reader thread. next-core owns
    /// its own pump and hands the mux no reader, so without this a next-core
    /// pane would sit showing stale content until some unrelated event forced
    /// a repaint.
    fn start_change_watcher(&self) {
        use std::sync::atomic::Ordering;

        if self.watching.swap(true, Ordering::AcqRel) {
            return;
        }
        let watching = std::sync::Arc::clone(&self.watching);
        let last_revision = std::sync::Arc::clone(&self.last_revision);
        let session_id = self.session_id;
        let pane_id = self.pane_id;
        let spawned = std::thread::Builder::new()
            .name(format!("next-core-pane-{pane_id}"))
            .spawn(move || {
                let mut seen = None;
                while watching.load(Ordering::Acquire) {
                    match next_core().screen_revision(session_id) {
                        Ok(revision) => {
                            if seen != Some(revision) {
                                seen = Some(revision);
                                last_revision.store(revision, Ordering::Release);
                                // Only when a mux exists. Without one there is
                                // nobody to repaint, and the notify path falls
                                // through to the main-thread scheduler, which
                                // panics on a background thread if the GUI
                                // event loop is not running -- during teardown,
                                // or in a test.
                                if mux::Mux::try_get().is_some() {
                                    mux::Mux::notify_from_any_thread(
                                        mux::MuxNotification::PaneOutput(pane_id),
                                    );
                                }
                            }
                        }
                        // The session is gone; the pane is on its way out and
                        // the mux will drop it.
                        Err(_) => break,
                    }
                    std::thread::sleep(CHANGE_POLL_INTERVAL);
                }
            });
        if let Err(err) = spawned {
            // Without the watcher the pane still works, it just will not
            // repaint on its own, so this must not pass silently.
            self.watching.store(false, Ordering::Release);
            log::error!(
                "next-core pane {pane_id} change watcher failed to start; \
                 the pane will not repaint on output: {err:#}"
            );
        }
    }

    /// Create the next-core session that will back a mux pane.
    ///
    /// The pane owns the session from here on: dropping it destroys the
    /// session, which is what keeps a closed pane from leaving a PTY running.
    pub fn spawn(
        pane_id: PaneId,
        size: TerminalSize,
        command: Option<portable_pty::CommandBuilder>,
        command_dir: Option<String>,
        domain_id: DomainId,
        env: Vec<(String, String)>,
    ) -> anyhow::Result<Self> {
        let session = next_core().create_session(crate::engine::CreateSessionRequest {
            cols: size.cols.max(1),
            rows: size.rows.max(1),
            command_dir,
            command,
            env,
            launch_policy: crate::engine::LaunchPolicySnapshot::default(),
        })?;
        let pane = Self::new(pane_id, session.id, domain_id);
        pane.start_change_watcher();
        Ok(pane)
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

/// Whether mux panes should be next-core sessions rather than local PTY
/// terminals.
///
/// Separate from `UNTERM_NEXT_CORE_WEBGPU_PANE`: that one paints next-core
/// over a local pane, this one replaces the pane outright. Unset or
/// unrecognized leaves the local terminal in charge.
pub fn next_core_panes_enabled_from_env(value: Option<&str>) -> bool {
    matches!(
        value.map(|raw| raw.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on" | "next-core")
    )
}

pub fn next_core_panes_enabled() -> bool {
    next_core_panes_enabled_from_env(std::env::var("UNTERM_NEXT_CORE_PANE").ok().as_deref())
}

/// Install `NextCorePane` as the mux's pane implementation, if enabled.
///
/// Called once at startup. The mux holds no policy about when an alternate
/// pane is wanted, so the decision is made here.
pub fn install_pane_factory_if_enabled() {
    if !next_core_panes_enabled() {
        return;
    }
    let result = mux::pane::set_alternate_pane_factory(Box::new(
        |pane_id, size, command, command_dir, domain_id| {
            // Same proxy env the local spawn path injects: a next-core pane
            // must not silently bypass the user's proxy configuration.
            let env = crate::spawn::read_unterm_proxy_env().unwrap_or_default();
            let pane =
                NextCorePane::spawn(pane_id, size, command, command_dir, domain_id, env)?;
            Ok(Some(std::sync::Arc::new(pane) as std::sync::Arc<dyn Pane>))
        },
    ));
    match result {
        Ok(()) => log::info!("next-core panes enabled: mux panes are next-core sessions"),
        Err(err) => log::warn!("next-core pane factory not installed: {err:#}"),
    }
}

/// Map next-core's cursor shape name onto the renderer's enum.
///
/// next-core names its shapes after the DECSCUSR forms, which are the same
/// names termwiz uses. An unknown name falls back to the configured default
/// rather than guessing a shape the application did not ask for.
fn cursor_shape_from_name(name: &str) -> termwiz::surface::CursorShape {
    use termwiz::surface::CursorShape;
    match name {
        "BlinkingBlock" => CursorShape::BlinkingBlock,
        "SteadyBlock" => CursorShape::SteadyBlock,
        "BlinkingUnderline" => CursorShape::BlinkingUnderline,
        "SteadyUnderline" => CursorShape::SteadyUnderline,
        "BlinkingBar" => CursorShape::BlinkingBar,
        "SteadyBar" => CursorShape::SteadyBar,
        _ => CursorShape::Default,
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
    if line.wrapped {
        // Logical-line reassembly reads this bit. Without it a soft-wrapped
        // command copies out with a newline spliced into the middle.
        rendered.set_last_cell_was_wrapped(true, 0);
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

impl Drop for NextCorePane {
    fn drop(&mut self) {
        self.watching
            .store(false, std::sync::atomic::Ordering::Release);
        // The pane owns its session, so a closed pane must not leave the PTY
        // running. Already-destroyed is fine: the mux can drop a pane after
        // an explicit session teardown.
        if let Err(err) = next_core().destroy_session(self.session_id) {
            log::debug!(
                "next-core session {} for pane {} was already gone at drop: {err:#}",
                self.session_id,
                self.pane_id
            );
        }
    }
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
            shape: cursor_shape_from_name(&screen.cursor.shape),
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

    fn focus_changed(&self, focused: bool) {
        use std::sync::atomic::Ordering;

        self.focused.store(focused, Ordering::Release);
        if !focused {
            // Anything past this revision is output the user has not seen.
            self.revision_at_focus_loss
                .store(self.last_revision.load(Ordering::Acquire), Ordering::Release);
        }
    }

    fn has_unseen_output(&self) -> bool {
        use std::sync::atomic::Ordering;

        // Same rule a wezterm terminal uses: unfocused, and the sequence has
        // moved since focus was lost.
        !self.focused.load(Ordering::Acquire)
            && self.last_revision.load(Ordering::Acquire)
                > self.revision_at_focus_loss.load(Ordering::Acquire)
    }

    fn erase_scrollback(&self, erase_mode: config::keyassignment::ScrollbackEraseMode) {
        use config::keyassignment::ScrollbackEraseMode;
        let include_viewport = matches!(erase_mode, ScrollbackEraseMode::ScrollbackAndViewport);
        if let Err(err) = next_core().erase_scrollback(self.session_id, include_viewport) {
            log::warn!(
                "next-core erase_scrollback for pane {} (session {}) failed: {err:#}",
                self.pane_id,
                self.session_id
            );
        }
    }

    fn kill(&self) {
        // The trait's default is a no-op, which would leave the shell running
        // until the last Arc to this pane happened to drop. Destroying the
        // session takes the PTY and its child with it.
        if let Err(err) = next_core().destroy_session(self.session_id) {
            log::debug!(
                "next-core session {} for pane {} was already gone at kill: {err:#}",
                self.session_id,
                self.pane_id
            );
        }
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
        // The user's configured colours, the same source a local pane resolves
        // through. Falling back to ColorPalette::default() here would render
        // every next-core pane in stock colours regardless of the theme.
        wezterm_term::TerminalConfiguration::color_palette(&config::TermConfig::new())
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
            wrapped: false,
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
            wrapped: false,
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
            wrapped: false,
            cells: vec![styled('a', 1), styled('b', 1), styled('c', 1)],
        };

        let rendered = styled_line_to_line(&line, 2);

        assert_eq!(rendered.len(), 3);
        assert_eq!(rendered.as_str().trim_end(), "abc");
    }

    /// Logical-line reassembly — which is what copy uses — keys off the wrap
    /// marker. Losing it splices a newline into the middle of a copied
    /// command that merely happened to be too long for the window.
    #[test]
    fn cursor_shapes_round_trip_through_their_names() {
        use termwiz::surface::CursorShape;

        // next-core names its shapes after the DECSCUSR forms, so every
        // termwiz variant must have a name that maps back to it. A silent
        // fallback here would show the wrong cursor for an app that asked
        // for a bar or an underline.
        for (name, expected) in [
            ("BlinkingBlock", CursorShape::BlinkingBlock),
            ("SteadyBlock", CursorShape::SteadyBlock),
            ("BlinkingUnderline", CursorShape::BlinkingUnderline),
            ("SteadyUnderline", CursorShape::SteadyUnderline),
            ("BlinkingBar", CursorShape::BlinkingBar),
            ("SteadyBar", CursorShape::SteadyBar),
            ("Default", CursorShape::Default),
        ] {
            assert_eq!(cursor_shape_from_name(name), expected, "{name}");
        }
        assert_eq!(cursor_shape_from_name("nonsense"), CursorShape::Default);
    }

    #[test]
    fn a_wrapped_row_marks_its_last_cell() {
        let wrapped = StyledScreenLine {
            row: 0,
            wrapped: true,
            cells: vec![styled('a', 1), styled('b', 1)],
        };
        let unwrapped = StyledScreenLine {
            row: 1,
            wrapped: false,
            cells: vec![styled('c', 1)],
        };

        assert!(styled_line_to_line(&wrapped, 2).last_cell_was_wrapped());
        assert!(!styled_line_to_line(&unwrapped, 2).last_cell_was_wrapped());
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

    /// The pane owns its session's lifetime. If dropping it did not destroy
    /// the session, every closed pane would leave a PTY and a shell running.
    #[test]
    fn spawning_a_pane_creates_a_session_and_dropping_it_destroys_one() -> anyhow::Result<()> {
        let pane = NextCorePane::spawn(
            7,
            TerminalSize {
                cols: 40,
                rows: 6,
                ..Default::default()
            },
            None,
            None,
            0,
            Vec::new(),
        )?;
        let session_id = pane.session_id();

        assert!(
            next_core().get_session(session_id).is_ok(),
            "spawn must leave a live session behind the pane"
        );
        assert!(!pane.is_dead());

        drop(pane);

        assert!(
            next_core().get_session(session_id).is_err(),
            "dropping the pane must destroy its session, or the PTY leaks"
        );
        Ok(())
    }

    /// Closing a pane calls `kill`. The trait default is a no-op, which would
    /// leave the shell running until the last Arc happened to drop.
    #[test]
    fn killing_a_pane_ends_its_session() -> anyhow::Result<()> {
        let pane = NextCorePane::spawn(
            8,
            TerminalSize {
                cols: 40,
                rows: 6,
                ..Default::default()
            },
            None,
            None,
            0,
            Vec::new(),
        )?;
        let session_id = pane.session_id();
        assert!(next_core().get_session(session_id).is_ok());

        pane.kill();

        assert!(
            next_core().get_session(session_id).is_err(),
            "kill must end the session, not wait for the pane to be dropped"
        );
        assert!(pane.is_dead(), "a killed pane must read as dead");
        Ok(())
    }

    /// The tab bar's unseen-output marker: output that arrived while the pane
    /// was not being looked at.
    #[test]
    fn unseen_output_tracks_revisions_since_focus_was_lost() -> anyhow::Result<()> {
        use std::sync::atomic::Ordering;

        let session = next_core().create_session(CreateSessionRequest {
            cols: 40,
            rows: 6,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: LaunchPolicySnapshot::default(),
        })?;
        let pane = NextCorePane::new(9, session.id, 0);

        // Focused: whatever arrives, the user is looking at it.
        pane.focus_changed(true);
        pane.last_revision.store(5, Ordering::Release);
        assert!(!pane.has_unseen_output());

        // Losing focus takes a watermark; nothing new yet.
        pane.focus_changed(false);
        assert!(!pane.has_unseen_output());

        // Output past the watermark is unseen.
        pane.last_revision.store(6, Ordering::Release);
        assert!(pane.has_unseen_output());

        // Looking at the pane clears it, and it stays clear while focused.
        pane.focus_changed(true);
        assert!(!pane.has_unseen_output());
        pane.last_revision.store(7, Ordering::Release);
        assert!(!pane.has_unseen_output());

        next_core().destroy_session(session.id)?;
        Ok(())
    }

    /// next-core's layout tree must agree with the mux Tab it is meant to
    /// replace, cell for cell.
    ///
    /// Swapping the two is only safe if they lay panes out identically; a
    /// one-cell disagreement would move every divider on screen, and would
    /// show up as a rendering bug long after the swap. Compare them directly
    /// instead, on the real Tab rather than a reimplementation of it.
    fn compare_layout_with_mux(
        axis_request: mux::tab::SplitDirection,
        axis: unterm_engine::next_core::layout::SplitAxis,
    ) -> anyhow::Result<()> {
        use unterm_engine::next_core::layout::Layout;
        use wezterm_term::TerminalSize;

        let size = TerminalSize {
            rows: 24,
            cols: 80,
            pixel_width: 800,
            pixel_height: 600,
            dpi: 96,
        };

        let first = NextCorePane::spawn(mux::pane::alloc_pane_id(), size, None, None, 0, Vec::new())?;
        let first_id = first.pane_id();
        let tab = mux::tab::Tab::new(&size);
        let first: std::sync::Arc<dyn Pane> = std::sync::Arc::new(first);
        tab.assign_pane(&first);

        let request = mux::tab::SplitRequest {
            direction: axis_request,
            ..Default::default()
        };
        let split = tab
            .compute_split_size(0, request)
            .ok_or_else(|| anyhow::anyhow!("mux declined to compute a split size"))?;
        let second =
            NextCorePane::spawn(mux::pane::alloc_pane_id(), split.second, None, None, 0, Vec::new())?;
        let second_id = second.pane_id();
        let second: std::sync::Arc<dyn Pane> = std::sync::Arc::new(second);
        tab.split_and_insert(0, request, std::sync::Arc::clone(&second))?;

        let mux_panes = tab.iter_panes();
        assert_eq!(mux_panes.len(), 2, "the split should have produced 2 panes");

        let mut layout = Layout::new(first_id);
        layout.split(first_id, second_id, axis, 0.5)?;
        let ours = layout.positions(size.cols as usize, size.rows as usize);

        for (mux_pane, our_pane) in mux_panes.iter().zip(&ours) {
            assert_eq!(
                (
                    mux_pane.left,
                    mux_pane.top,
                    mux_pane.width,
                    mux_pane.height
                ),
                (
                    our_pane.rect.left,
                    our_pane.rect.top,
                    our_pane.rect.width,
                    our_pane.rect.height
                ),
                "layout disagrees with mux for pane index {}",
                mux_pane.index
            );
        }

        Ok(())
    }

    /// Adopt a real mux Tab's layout into next-core's tree.
    ///
    /// This is the move the wiring makes: mux has the tab, next-core needs the
    /// same arrangement. Rectangles are the only thing both sides agree on, so
    /// the tree is recovered from them and must reproduce them exactly --
    /// anything else shifts panes on screen at the moment of the swap.
    #[test]
    fn a_mux_tab_layout_can_be_adopted_by_next_core() -> anyhow::Result<()> {
        use unterm_engine::next_core::layout::{Layout, PositionedPane};
        use wezterm_term::TerminalSize;

        let size = TerminalSize {
            rows: 24,
            cols: 80,
            pixel_width: 800,
            pixel_height: 600,
            dpi: 96,
        };
        let tab = mux::tab::Tab::new(&size);
        let first: std::sync::Arc<dyn Pane> = std::sync::Arc::new(NextCorePane::spawn(
            mux::pane::alloc_pane_id(),
            size,
            None,
            None,
            0,
            Vec::new(),
        )?);
        tab.assign_pane(&first);

        // Two splits on different axes, so the recovered tree has to be
        // nested rather than a single cut.
        for (index, direction) in [
            (0, mux::tab::SplitDirection::Horizontal),
            (1, mux::tab::SplitDirection::Vertical),
        ] {
            let request = mux::tab::SplitRequest {
                direction,
                ..Default::default()
            };
            let split = tab
                .compute_split_size(index, request)
                .ok_or_else(|| anyhow::anyhow!("mux declined to split pane {index}"))?;
            let pane: std::sync::Arc<dyn Pane> = std::sync::Arc::new(NextCorePane::spawn(
                mux::pane::alloc_pane_id(),
                split.second,
                None,
                None,
                0,
                Vec::new(),
            )?);
            tab.split_and_insert(index, request, pane)?;
        }

        let mux_panes = tab.iter_panes();
        assert_eq!(mux_panes.len(), 3);

        let adopted: Vec<PositionedPane> = mux_panes
            .iter()
            .map(|pos| PositionedPane {
                pane_id: pos.pane.pane_id(),
                rect: unterm_engine::next_core::layout::PaneRect {
                    left: pos.left,
                    top: pos.top,
                    width: pos.width,
                    height: pos.height,
                },
            })
            .collect();

        let layout = Layout::from_positions(&adopted)
            .ok_or_else(|| anyhow::anyhow!("could not adopt the mux layout: {adopted:?}"))?;

        assert_eq!(
            layout.positions(size.cols as usize, size.rows as usize),
            adopted,
            "adopted tree lays out differently than the mux tab it came from"
        );
        Ok(())
    }

    #[test]
    fn layout_tree_matches_mux_tab_geometry() -> anyhow::Result<()> {
        use unterm_engine::next_core::layout::SplitAxis;

        // Both axes: the arithmetic differs (80 columns splits 39/40, 24 rows
        // splits 11/12), so agreeing on one says nothing about the other.
        compare_layout_with_mux(mux::tab::SplitDirection::Horizontal, SplitAxis::Horizontal)?;
        compare_layout_with_mux(mux::tab::SplitDirection::Vertical, SplitAxis::Vertical)?;
        Ok(())
    }

    #[test]
    fn pane_factory_flag_needs_an_explicit_opt_in() {
        for value in ["1", "true", "yes", "on", "next-core", " TRUE "] {
            assert!(
                next_core_panes_enabled_from_env(Some(value)),
                "{value:?} should enable next-core panes"
            );
        }
        // Unset or unrecognized must leave the local terminal in charge:
        // this flag changes what a pane fundamentally *is*.
        for value in [None, Some(""), Some("0"), Some("off"), Some("replace")] {
            assert!(
                !next_core_panes_enabled_from_env(value),
                "{value:?} should not enable next-core panes"
            );
        }
    }

    /// The pane has no mux PTY reader, so without its own change watcher the
    /// GUI would never be told to repaint. Drive real output and check the
    /// revision the watcher samples actually advances.
    #[test]
    fn session_revision_advances_when_output_arrives() -> anyhow::Result<()> {
        let session = next_core().create_session(CreateSessionRequest {
            cols: 40,
            rows: 6,
            command_dir: None,
            command: None,
            env: Vec::new(),
            launch_policy: LaunchPolicySnapshot::default(),
        })?;

        let before = next_core().screen_revision(session.id)?;
        next_core().write_input(session.id, "echo nc-pane-revision\r")?;

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let advanced = loop {
            if next_core().screen_revision(session.id)? > before {
                break true;
            }
            if std::time::Instant::now() >= deadline {
                break false;
            }
            std::thread::sleep(CHANGE_POLL_INTERVAL);
        };

        next_core().destroy_session(session.id)?;
        assert!(
            advanced,
            "screen revision never advanced, so the change watcher would never \
             tell the mux to repaint"
        );
        Ok(())
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
