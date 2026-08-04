use crate::domain::DomainId;
use crate::pane::{
    CachePolicy, ForEachPaneLogicalLine, LogicalLine, Pane, PaneId, Pattern,
    PerformAssignmentResult, SearchResult, WithPaneLines,
};
use crate::renderable::{RenderableDimensions, StableCursorPosition};
use crate::ExitBehavior;
use crate::Mux;
use async_trait::async_trait;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use portable_pty::CommandBuilder;
use rangeset::RangeSet;
use std::io::{self, Read, Write};
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use termwiz::input::KeyboardEncoding;
use termwiz::surface::{CursorShape, CursorVisibility, Line, SequenceNo};
use url::Url;
use wezterm_term::color::ColorPalette;
use wezterm_term::{
    KeyCode, KeyModifiers, MouseEvent, SemanticZone, StableRowIndex, TerminalConfiguration,
    TerminalSize,
};

struct CoreWriter {
    client: Arc<Mutex<unterm_core::CoreClient>>,
    id: String,
}

struct CoreReader {
    client: Arc<Mutex<unterm_core::CoreClient>>,
    id: String,
    dead: Arc<AtomicBool>,
}
impl Read for CoreReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        loop {
            let value = self.client.lock().request_with_params::<serde_json::Value>(
                "session.read",
                serde_json::json!({"id": self.id, "max_bytes": out.len().max(1), "consume": true}),
            ).map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))?;
            let result = value.result.unwrap_or_default();
            let status = result
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("missing");
            let text = result
                .get("text")
                .and_then(|t| t.as_str().map(str::to_owned))
                .unwrap_or_default();
            if !text.is_empty() {
                let bytes = text.as_bytes();
                let n = bytes.len().min(out.len());
                out[..n].copy_from_slice(&bytes[..n]);
                return Ok(n);
            }
            if status != "running" {
                self.dead.store(true, Ordering::Relaxed);
                return Ok(0);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
impl Write for CoreWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf).to_string();
        let mut c = self.client.lock();
        c.request_with_params::<serde_json::Value>(
            "session.write",
            serde_json::json!({"id": self.id, "data": text}),
        )
        .map(|_| buf.len())
        .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Snapshot of the Core-side terminal state that the GUI needs for
/// rendering decisions between full line fetches.
#[derive(Clone)]
struct CoreMeta {
    seqno: SequenceNo,
    title: String,
    cwd: Option<Url>,
    alt_screen: bool,
    mouse_grabbed: bool,
    cursor: StableCursorPosition,
    cols: usize,
    rows: usize,
    scrollback_rows: usize,
    physical_top: StableRowIndex,
    scrollback_top: StableRowIndex,
}

pub struct CorePane {
    pane_id: PaneId,
    domain_id: DomainId,
    id: String,
    client: Arc<Mutex<unterm_core::CoreClient>>,
    writer: Mutex<CoreWriter>,
    cols: Mutex<usize>,
    rows: Mutex<usize>,
    record_sink: Mutex<Option<Arc<dyn crate::pane::RecordSink>>>,
    dead: Arc<AtomicBool>,
    encoder: Mutex<wezterm_term::Terminal>,
    meta: Mutex<Option<(Instant, CoreMeta)>>,
}

impl CorePane {
    pub fn try_attach_external(
        pane_id: PaneId,
        domain_id: DomainId,
        size: TerminalSize,
        shell: &str,
    ) -> anyhow::Result<Option<Arc<dyn Pane>>> {
        let Some(info) = unterm_core::read_discovery()? else { return Ok(None); };
        let mut client = match unterm_core::CoreClient::connect(&info.endpoint, &info.token) {
            Ok(client) => client,
            Err(_) => return Ok(None),
        };
        let response: unterm_core::Response<unterm_core::SessionInfo> = client.request_with_params(
            "session.external.create",
            serde_json::json!({"shell": shell, "cols": size.cols, "rows": size.rows}),
        )?;
        let session = response.result.ok_or_else(|| anyhow::anyhow!("external core session failed"))?;
        Ok(Some(Arc::new(Self::new(pane_id, domain_id, session.id, client, size))))
    }

    pub fn try_spawn(
        pane_id: PaneId,
        domain_id: DomainId,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Option<Arc<dyn Pane>>> {
        Self::try_spawn_with_params(
            pane_id,
            domain_id,
            size,
            command,
            command_dir,
            serde_json::Value::Null,
        )
    }

    pub fn try_spawn_with_params(
        pane_id: PaneId,
        domain_id: DomainId,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        extra: serde_json::Value,
    ) -> anyhow::Result<Option<Arc<dyn Pane>>> {
        let Some(info) = unterm_core::read_discovery()? else {
            return Ok(None);
        };
        let mut client = match unterm_core::CoreClient::connect(&info.endpoint, &info.token) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        let command_cwd = command
            .as_ref()
            .and_then(|cmd| cmd.get_cwd().map(|cwd| cwd.to_string_lossy().into_owned()));
        let cwd = command_dir.or(command_cwd).unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| Path::new(".").to_path_buf())
                .to_string_lossy()
                .to_string()
        });
        let argv = command.map(|c| {
            c.get_argv()
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        });
        let mut params = serde_json::json!({"cwd":cwd,"shell":"shell","argv":argv,"cols":size.cols,"rows":size.rows});
        if let (Some(base), Some(extra)) = (params.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                base.insert(key.clone(), value.clone());
            }
        }
        let method = if params.get("transport").and_then(|v| v.as_str()) == Some("serial") {
            "session.serial.create"
        } else {
            "session.create"
        };
        let response: unterm_core::Response<unterm_core::SessionInfo> =
            client.request_with_params(method, params)?;
        let session = response
            .result
            .ok_or_else(|| anyhow::anyhow!("unterm-core session.create failed"))?;
        Ok(Some(Arc::new(Self::new(
            pane_id, domain_id, session.id, client, size,
        ))))
    }
    pub fn new(
        pane_id: PaneId,
        domain_id: DomainId,
        id: String,
        client: unterm_core::CoreClient,
        size: TerminalSize,
    ) -> Self {
        let client = Arc::new(Mutex::new(client));
        let writer = CoreWriter {
            client: client.clone(),
            id: id.clone(),
        };
        let encoder_writer = CoreWriter {
            client: client.clone(),
            id: id.clone(),
        };
        let encoder = wezterm_term::Terminal::new(
            size,
            Arc::new(config::TermConfig::new()),
            "unterm-core",
            config::wezterm_version(),
            Box::new(encoder_writer),
        );
        Self {
            pane_id,
            domain_id,
            id,
            client,
            writer: Mutex::new(writer),
            cols: Mutex::new(size.cols as usize),
            rows: Mutex::new(size.rows as usize),
            record_sink: Mutex::new(None),
            dead: Arc::new(AtomicBool::new(false)),
            encoder: Mutex::new(encoder),
            meta: Mutex::new(None),
        }
    }
    pub fn set_record_sink(&self, sink: Option<Arc<dyn crate::pane::RecordSink>>) {
        *self.record_sink.lock() = sink;
    }

    pub fn core_session_id(&self) -> &str {
        &self.id
    }
    fn fallback_meta(&self) -> CoreMeta {
        let rows = *self.rows.lock();
        CoreMeta {
            seqno: 0,
            title: String::new(),
            cwd: None,
            alt_screen: false,
            mouse_grabbed: false,
            cursor: StableCursorPosition {
                x: 0,
                y: 0,
                shape: CursorShape::Default,
                visibility: CursorVisibility::Visible,
            },
            cols: *self.cols.lock(),
            rows,
            scrollback_rows: rows,
            physical_top: 0,
            scrollback_top: 0,
        }
    }

    /// Fetch (with a short-lived cache) the render metadata for this
    /// session from the Core. Several Pane trait methods are called
    /// multiple times per frame; the cache keeps that to one IPC
    /// round-trip per frame instead of one per call.
    fn fetch_meta(&self) -> CoreMeta {
        const META_TTL: Duration = Duration::from_millis(25);
        {
            let cache = self.meta.lock();
            if let Some((at, meta)) = cache.as_ref() {
                if at.elapsed() < META_TTL {
                    return meta.clone();
                }
            }
        }
        let value = match self.client.lock().request_with_params::<serde_json::Value>(
            "session.screen",
            serde_json::json!({"id": self.id, "meta_only": true}),
        ) {
            Ok(v) => v,
            Err(_) => return self.fallback_meta(),
        };
        let result = value.result.unwrap_or_default();
        if !result
            .get("found")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return self.fallback_meta();
        }
        let cols = result
            .get("cols")
            .and_then(|v| v.as_u64())
            .unwrap_or(*self.cols.lock() as u64) as usize;
        let rows = result
            .get("rows")
            .and_then(|v| v.as_u64())
            .unwrap_or(*self.rows.lock() as u64) as usize;
        let physical_top = result
            .get("physical_top")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as StableRowIndex;
        let cursor = result
            .get("cursor")
            .cloned()
            .and_then(|v| serde_json::from_value::<wezterm_term::CursorPosition>(v).ok())
            .map(|cursor| StableCursorPosition {
                x: cursor.x,
                y: physical_top + cursor.y as StableRowIndex,
                shape: cursor.shape,
                visibility: cursor.visibility,
            })
            .unwrap_or_else(|| StableCursorPosition {
                x: 0,
                y: physical_top,
                shape: CursorShape::Default,
                visibility: CursorVisibility::Visible,
            });
        let meta = CoreMeta {
            seqno: result
                .get("seqno")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as SequenceNo,
            title: result
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            cwd: result
                .get("cwd")
                .and_then(|v| v.as_str())
                .and_then(|s| Url::parse(s).ok()),
            alt_screen: result
                .get("alt_screen")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            mouse_grabbed: result
                .get("mouse_grabbed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            cursor,
            cols,
            rows,
            scrollback_rows: result
                .get("scrollback_rows")
                .and_then(|v| v.as_u64())
                .unwrap_or(rows as u64) as usize,
            physical_top,
            scrollback_top: result
                .get("scrollback_top")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as StableRowIndex,
        };
        *self.meta.lock() = Some((Instant::now(), meta.clone()));
        meta
    }
}

#[async_trait(?Send)]
impl Pane for CorePane {
    fn pane_id(&self) -> PaneId {
        self.pane_id
    }
    fn get_cursor_position(&self) -> StableCursorPosition {
        self.fetch_meta().cursor
    }
    fn get_current_seqno(&self) -> SequenceNo {
        self.fetch_meta().seqno
    }
    fn get_changed_since(
        &self,
        lines: Range<StableRowIndex>,
        seqno: SequenceNo,
    ) -> RangeSet<StableRowIndex> {
        let mut set = RangeSet::new();
        let value = self.client.lock().request_with_params::<serde_json::Value>(
            "session.changed",
            serde_json::json!({
                "id": self.id,
                "start": lines.start,
                "end": lines.end,
                "seqno": seqno,
            }),
        );
        match value {
            Ok(v) => {
                let result = v.result.unwrap_or_default();
                if result
                    .get("found")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    if let Some(rows) = result.get("rows").and_then(|v| v.as_array()) {
                        for row in rows {
                            if let Some(row) = row.as_i64() {
                                set.add(row as StableRowIndex);
                            }
                        }
                    }
                } else {
                    set.add_range(lines);
                }
            }
            Err(_) => set.add_range(lines),
        }
        set
    }
    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
        let value = self.client.lock().request_with_params::<serde_json::Value>(
            "session.lines",
            serde_json::json!({"id": self.id, "start": lines.start, "end": lines.end}),
        );
        match value {
            Ok(v) => {
                let result = v.result.unwrap_or_default();
                let first_row = result
                    .get("first_row")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(lines.start as i64) as StableRowIndex;
                let parsed: Vec<Line> = result
                    .get("lines")
                    .cloned()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                (first_row, parsed)
            }
            Err(_) => (lines.start, vec![]),
        }
    }
    fn with_lines_mut(&self, lines: Range<StableRowIndex>, with_lines: &mut dyn WithPaneLines) {
        crate::pane::impl_with_lines_via_get_lines(self, lines, with_lines)
    }
    fn for_each_logical_line_in_stable_range_mut(
        &self,
        lines: Range<StableRowIndex>,
        for_line: &mut dyn ForEachPaneLogicalLine,
    ) {
        crate::pane::impl_for_each_logical_line_via_get_logical_lines(self, lines, for_line)
    }
    fn get_logical_lines(&self, lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
        crate::pane::impl_get_logical_lines_via_get_lines(self, lines)
    }
    fn get_dimensions(&self) -> RenderableDimensions {
        let meta = self.fetch_meta();
        RenderableDimensions {
            cols: meta.cols,
            viewport_rows: meta.rows,
            scrollback_rows: meta.scrollback_rows,
            physical_top: meta.physical_top,
            scrollback_top: meta.scrollback_top,
            ..Default::default()
        }
    }
    fn get_title(&self) -> String {
        let title = self.fetch_meta().title;
        if title.is_empty() {
            "Shell".into()
        } else {
            title
        }
    }
    fn send_paste(&self, text: &str) -> anyhow::Result<()> {
        self.writer.lock().write_all(text.as_bytes())?;
        Ok(())
    }
    fn reader(&self) -> anyhow::Result<Option<Box<dyn Read + Send>>> {
        Ok(Some(Box::new(CoreReader {
            client: self.client.clone(),
            id: self.id.clone(),
            dead: self.dead.clone(),
        })))
    }
    fn writer(&self) -> MappedMutexGuard<'_, dyn Write> {
        MutexGuard::map(self.writer.lock(), |w| w as &mut dyn Write)
    }
    fn resize(&self, size: TerminalSize) -> anyhow::Result<()> {
        *self.cols.lock() = size.cols as usize;
        *self.rows.lock() = size.rows as usize;
        *self.meta.lock() = None;
        self.client
            .lock()
            .request_with_params::<serde_json::Value>(
                "session.resize",
                serde_json::json!({"id":self.id,"cols":size.cols,"rows":size.rows}),
            )?;
        Ok(())
    }
    fn key_down(&self, key: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
        self.encoder.lock().key_down(key, mods)
    }
    fn key_up(&self, key: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
        self.encoder.lock().key_up(key, mods)
    }
    fn perform_assignment(
        &self,
        _assignment: &config::keyassignment::KeyAssignment,
    ) -> PerformAssignmentResult {
        PerformAssignmentResult::Unhandled
    }
    fn mouse_event(&self, event: MouseEvent) -> anyhow::Result<()> {
        self.encoder.lock().mouse_event(event)
    }
    fn perform_actions(&self, _actions: Vec<termwiz::escape::Action>) {
        Mux::get().notify(crate::MuxNotification::PaneOutput(self.pane_id));
    }
    fn is_dead(&self) -> bool {
        self.dead.load(Ordering::Relaxed)
    }
    fn kill(&self) {
        let _ = self.client.lock().request_with_params::<serde_json::Value>(
            "session.close",
            serde_json::json!({"id":self.id}),
        );
    }
    fn palette(&self) -> ColorPalette {
        self.encoder.lock().palette()
    }
    fn domain_id(&self) -> DomainId {
        self.domain_id
    }
    fn get_keyboard_encoding(&self) -> KeyboardEncoding {
        KeyboardEncoding::Xterm
    }
    fn is_mouse_grabbed(&self) -> bool {
        self.fetch_meta().mouse_grabbed
    }
    fn is_alt_screen_active(&self) -> bool {
        self.fetch_meta().alt_screen
    }
    fn set_config(&self, _config: Arc<dyn TerminalConfiguration>) {}
    fn get_current_working_dir(&self, _policy: CachePolicy) -> Option<Url> {
        self.fetch_meta().cwd
    }
    fn exit_behavior(&self) -> Option<ExitBehavior> {
        Some(ExitBehavior::CloseOnCleanExit)
    }
    async fn search(
        &self,
        _pattern: Pattern,
        _range: Range<StableRowIndex>,
        _limit: Option<u32>,
    ) -> anyhow::Result<Vec<SearchResult>> {
        Ok(vec![])
    }
    fn get_semantic_zones(&self) -> anyhow::Result<Vec<SemanticZone>> {
        let value = self.client.lock().request_with_params::<serde_json::Value>(
            "session.zones",
            serde_json::json!({"id": self.id}),
        )?;
        let result = value.result.unwrap_or_default();
        Ok(result
            .get("zones")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default())
    }
    fn record_sink(&self) -> Option<Arc<dyn crate::pane::RecordSink>> {
        self.record_sink.lock().clone()
    }
}
