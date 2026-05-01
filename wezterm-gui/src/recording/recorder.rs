//! Per-pane recorder. Holds a buffered queue of events, flushes to a
//! `.log` file every 5 seconds or every 100 events (whichever first),
//! tracks counters, and handles rotation.
//!
//! Markdown is *not* rendered here on every event; it is rendered on
//! demand via `read_session_markdown` / `export_pane_markdown` / when
//! the recording stops.

use super::index::{self, IndexEntry};
use super::render::{self, RenderConfig, RenderOutput};
use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use chrono::Utc;
use mux::pane::{Pane, PaneId, RecordSink};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use termwiz::escape::osc::FinalTermSemanticPrompt;
use termwiz::escape::OperatingSystemCommand;

const FLUSH_BYTES_THRESHOLD: usize = 64 * 1024;
const FLUSH_EVENT_THRESHOLD: usize = 100;
const FLUSH_INTERVAL: Duration = Duration::from_secs(5);
#[allow(dead_code)]
const ROTATE_BYTES: u64 = 5 * 1024 * 1024;
#[allow(dead_code)]
const ROTATE_BLOCKS: u64 = 1000;

/// Persistent config loaded from `~/.unterm/recording.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingConfig {
    #[serde(default)]
    pub recording: RecordingFlags,
    #[serde(default)]
    pub redaction: RedactionFlags,
    #[serde(default = "default_idle_minutes")]
    pub idle_rotate_minutes: u64,
}

fn default_idle_minutes() -> u64 {
    5
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingFlags {
    #[serde(default)]
    pub enabled: bool,
}

impl Default for RecordingFlags {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RedactionFlags {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub custom_patterns: Vec<String>,
}

impl Default for RedactionFlags {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_patterns: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            recording: RecordingFlags::default(),
            redaction: RedactionFlags::default(),
            idle_rotate_minutes: default_idle_minutes(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("recording.json")
}

pub fn load_config() -> RecordingConfig {
    let p = config_path();
    if !p.exists() {
        return RecordingConfig::default();
    }
    match std::fs::read_to_string(&p) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => RecordingConfig::default(),
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct RecordingMetrics {
    pub started_at: Option<i64>,
    pub block_count: u64,
    pub total_lines: u64,
    pub bytes: u64,
}

/// Inner mutable state of a recorder.
#[allow(dead_code)]
struct RecorderInner {
    pane_id: PaneId,
    tab_id: u64,
    session_id: String,
    project_path: Option<String>,
    project_slug: String,
    started_at_iso: String,
    started_at_micros: i64,
    log_path: PathBuf,
    md_path: PathBuf,
    pending: Vec<u8>,
    pending_event_count: usize,
    last_flush: Instant,
    last_event_at: Instant,
    block_count: u64,
    bytes_raw: u64,
    trace_ids: Vec<String>,
    parent_session_id: Option<String>,
    /// Whether we've ever observed an OSC 133 event for this session.
    osc133_active: bool,
    /// True after `stop()` has run; subsequent writes are dropped.
    stopped: bool,
    /// True after a rotation has been queued; the read loop should
    /// detach this sink and a new one (the parent) will handle further
    /// writes. We don't auto-rotate from inside the lock; instead, we
    /// flag and let the caller drive the new session creation.
    rotate_pending: bool,
    /// In-memory tracker for prompt-input segmentation.
    in_prompt: bool,
    in_input: bool,
}

impl RecorderInner {
    fn flush(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .with_context(|| format!("open log {}", self.log_path.display()))?;
        f.write_all(&self.pending)
            .with_context(|| format!("write log {}", self.log_path.display()))?;
        self.pending.clear();
        self.pending_event_count = 0;
        self.last_flush = Instant::now();
        Ok(())
    }

    fn append_event(&mut self, event: &str, payload: &[u8]) {
        let micros = chrono::Utc::now().timestamp_micros();
        let b64 = base64::engine::general_purpose::STANDARD.encode(payload);
        let line = format!("{}\t{}\t{}\n", micros, event, b64);
        self.pending.extend_from_slice(line.as_bytes());
        self.pending_event_count += 1;
        self.last_event_at = Instant::now();
        self.bytes_raw += payload.len() as u64;

        if self.pending.len() >= FLUSH_BYTES_THRESHOLD
            || self.pending_event_count >= FLUSH_EVENT_THRESHOLD
            || self.last_flush.elapsed() >= FLUSH_INTERVAL
        {
            if let Err(e) = self.flush() {
                log::warn!("recording flush failed: {e}");
            }
        }
    }

    #[allow(dead_code)]
    fn current_size(&self) -> u64 {
        self.bytes_raw + self.pending.len() as u64
    }

    /// Returns true if rotation should fire based on current counters.
    #[allow(dead_code)]
    fn should_rotate(&self, idle_after: Duration) -> bool {
        if self.rotate_pending {
            return false;
        }
        if self.bytes_raw >= ROTATE_BYTES {
            return true;
        }
        if self.block_count >= ROTATE_BLOCKS {
            return true;
        }
        if self.last_event_at.elapsed() >= idle_after {
            return true;
        }
        false
    }
}

/// The public-facing recorder; cheap to clone (an Arc).
pub struct PaneRecorder {
    inner: Mutex<RecorderInner>,
}

impl PaneRecorder {
    fn build_index_entry(&self, ended_at: Option<String>, exit_reason: Option<&str>) -> IndexEntry {
        let inner = self.inner.lock();
        IndexEntry {
            unterm_session_id: inner.session_id.clone(),
            tab_id: inner.tab_id,
            project_path: inner.project_path.clone(),
            project_slug: inner.project_slug.clone(),
            started_at: inner.started_at_iso.clone(),
            ended_at,
            block_count: inner.block_count,
            total_lines: 0,
            bytes_raw: inner.bytes_raw,
            log_path: inner.log_path.display().to_string(),
            md_path: inner.md_path.display().to_string(),
            exit_reason: exit_reason.map(|s| s.to_string()),
            parent_session_id: inner.parent_session_id.clone(),
            osc133_active: inner.osc133_active,
            redaction_active: load_config().redaction.enabled,
            redaction_count: 0,
            trace_ids: inner.trace_ids.clone(),
        }
    }

    pub fn flush_now(&self) {
        let mut inner = self.inner.lock();
        if let Err(e) = inner.flush() {
            log::warn!("recording flush_now: {e}");
        }
    }

    pub fn metrics(&self) -> RecordingMetrics {
        let inner = self.inner.lock();
        RecordingMetrics {
            started_at: Some(inner.started_at_micros),
            block_count: inner.block_count,
            total_lines: 0,
            bytes: inner.bytes_raw,
        }
    }

    pub fn paths(&self) -> (PathBuf, PathBuf, String) {
        let inner = self.inner.lock();
        (
            inner.log_path.clone(),
            inner.md_path.clone(),
            inner.session_id.clone(),
        )
    }

    pub fn add_trace(&self, trace_id: String) -> Vec<String> {
        let mut inner = self.inner.lock();
        if !inner.trace_ids.iter().any(|t| t == &trace_id) {
            inner.trace_ids.push(trace_id);
        }
        inner.trace_ids.clone()
    }
}

impl RecordSink for PaneRecorder {
    fn write_bytes(&self, bytes: &[u8]) {
        let mut inner = self.inner.lock();
        if inner.stopped {
            return;
        }
        inner.append_event("out", bytes);
    }

    fn on_osc133(&self, event: &OperatingSystemCommand) {
        let OperatingSystemCommand::FinalTermSemanticPrompt(p) = event else {
            return;
        };
        let mut inner = self.inner.lock();
        if inner.stopped {
            return;
        }
        inner.osc133_active = true;
        match p {
            FinalTermSemanticPrompt::FreshLineAndStartPrompt { .. }
            | FinalTermSemanticPrompt::StartPrompt(_) => {
                inner.append_event("block_prompt", &[]);
                inner.in_prompt = true;
                inner.in_input = false;
            }
            FinalTermSemanticPrompt::MarkEndOfPromptAndStartOfInputUntilEndOfLine
            | FinalTermSemanticPrompt::MarkEndOfPromptAndStartOfInputUntilNextMarker => {
                inner.append_event("block_input", &[]);
                inner.in_prompt = false;
                inner.in_input = true;
            }
            FinalTermSemanticPrompt::MarkEndOfInputAndStartOfOutput { .. } => {
                inner.append_event("block_output", &[]);
                inner.in_prompt = false;
                inner.in_input = false;
            }
            FinalTermSemanticPrompt::CommandStatus { status, .. } => {
                let payload = status.to_string();
                inner.append_event("block_exit", payload.as_bytes());
                inner.block_count += 1;
            }
            _ => {}
        }
    }
}

/// Global table of active recorders, keyed by pane id.
fn registry() -> &'static Mutex<HashMap<PaneId, Arc<PaneRecorder>>> {
    static R: OnceLock<Mutex<HashMap<PaneId, Arc<PaneRecorder>>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct StartResult {
    pub session_id: String,
    pub log_path: String,
    pub md_path: String,
}

pub struct StopResult {
    pub session_id: String,
    pub ended_at: String,
    pub block_count: u64,
    pub exit_reason: String,
    pub md_path: String,
}

fn project_info(pane: &Arc<dyn Pane>) -> (Option<String>, String) {
    let cwd = pane.get_current_working_dir(mux::pane::CachePolicy::AllowStale);
    if let Some(url) = cwd {
        if let Ok(path) = url.to_file_path() {
            let abs = path.display().to_string();
            let slug = path
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|s| !s.is_empty())
                .map(sanitize_slug)
                .unwrap_or_else(|| "_orphan".to_string());
            return (Some(abs), slug);
        }
    }
    (None, "_orphan".to_string())
}

fn sanitize_slug(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '-' })
        .collect()
}

fn timestamp_components() -> (String, String, String) {
    let now = chrono::Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let hms = now.format("%H%M%S").to_string();
    let iso = now.to_rfc3339();
    (date, hms, iso)
}

fn build_paths(
    project_path: Option<&str>,
    project_slug: &str,
    tab_id: u64,
) -> (PathBuf, PathBuf, String, String) {
    let (date, hms, iso) = timestamp_components();
    // Prefer storing recordings inside the project directory itself so they
    // travel with the project (git, archive, share). Fall back to the
    // user-global `~/.unterm/sessions/_orphan/` when there's no project or
    // the project dir is read-only / not writable for any reason.
    let dir = preferred_session_dir(project_path, project_slug, &date);
    let _ = std::fs::create_dir_all(&dir);
    let stem = format!("tab-{}-{}", tab_id, hms);
    let log_path = dir.join(format!("{}.log", stem));
    let md_path = dir.join(format!("{}.md", stem));
    (log_path, md_path, iso, stem)
}

fn preferred_session_dir(
    project_path: Option<&str>,
    project_slug: &str,
    date: &str,
) -> PathBuf {
    if let Some(p) = project_path {
        let path = PathBuf::from(p);
        let in_project = path.join(".unterm").join("sessions").join(date);
        // Only use project-local storage when we can actually write there.
        // Probe by attempting to create the directory; revert on failure.
        if std::fs::create_dir_all(&in_project).is_ok()
            && is_dir_writable(&in_project)
        {
            return in_project;
        }
        log::info!(
            "project dir {} not writable for recording; falling back to ~/.unterm/sessions",
            path.display()
        );
    }
    let slug = if project_slug.is_empty() {
        "_orphan"
    } else {
        project_slug
    };
    index::sessions_root().join(slug).join(date)
}

fn is_dir_writable(dir: &std::path::Path) -> bool {
    // Cheap probe: try to create a hidden tempfile, write 1 byte, delete it.
    let probe = dir.join(".unterm-write-probe");
    match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(mut f) => {
            use std::io::Write;
            let ok = f.write_all(b"u").is_ok();
            drop(f);
            let _ = std::fs::remove_file(&probe);
            ok
        }
        Err(_) => false,
    }
}

/// Kick off recording for a pane. Returns the new session id and paths.
pub fn start_recording(pane_id: PaneId) -> Result<StartResult> {
    let mux = mux::Mux::try_get().ok_or_else(|| anyhow!("Mux not available"))?;
    let pane = mux
        .get_pane(pane_id)
        .ok_or_else(|| anyhow!("Pane {} not found", pane_id))?;

    {
        let reg = registry().lock();
        if reg.contains_key(&pane_id) {
            return Err(anyhow!("Recording already active for pane {}", pane_id));
        }
    }

    let (project_path, project_slug) = project_info(&pane);
    let tab_id = pane.pane_id() as u64;
    let (log_path, md_path, started_at_iso, _stem) =
        build_paths(project_path.as_deref(), &project_slug, tab_id);
    let session_id = uuid::Uuid::new_v4().to_string();

    // Touch the log file so subsequent reads succeed even if no bytes
    // have arrived yet.
    if let Err(e) = File::create(&log_path) {
        return Err(anyhow!("create log {}: {}", log_path.display(), e));
    }

    let recorder = Arc::new(PaneRecorder {
        inner: Mutex::new(RecorderInner {
            pane_id,
            tab_id,
            session_id: session_id.clone(),
            project_path: project_path.clone(),
            project_slug: project_slug.clone(),
            started_at_iso: started_at_iso.clone(),
            started_at_micros: chrono::Utc::now().timestamp_micros(),
            log_path: log_path.clone(),
            md_path: md_path.clone(),
            pending: Vec::with_capacity(FLUSH_BYTES_THRESHOLD),
            pending_event_count: 0,
            last_flush: Instant::now(),
            last_event_at: Instant::now(),
            block_count: 0,
            bytes_raw: 0,
            trace_ids: Vec::new(),
            parent_session_id: None,
            osc133_active: false,
            stopped: false,
            rotate_pending: false,
            in_prompt: false,
            in_input: false,
        }),
    });

    // Register in the LocalPane via downcast.
    let any = pane.clone();
    let local = any
        .downcast_arc::<mux::localpane::LocalPane>()
        .map_err(|_| anyhow!("Pane {} is not a LocalPane (recording is only supported for local panes)", pane_id))?;
    local.set_record_sink(Some(recorder.clone() as Arc<dyn RecordSink>));

    registry().lock().insert(pane_id, recorder.clone());

    // Append a stub entry to index.json so list operations see it
    // immediately; we rewrite on stop.
    let initial_entry = IndexEntry {
        unterm_session_id: session_id.clone(),
        tab_id,
        project_path,
        project_slug,
        started_at: started_at_iso,
        ended_at: None,
        block_count: 0,
        total_lines: 0,
        bytes_raw: 0,
        log_path: log_path.display().to_string(),
        md_path: md_path.display().to_string(),
        exit_reason: None,
        parent_session_id: None,
        osc133_active: false,
        redaction_active: load_config().redaction.enabled,
        redaction_count: 0,
        trace_ids: Vec::new(),
    };
    index::upsert_entry(initial_entry).ok();

    Ok(StartResult {
        session_id,
        log_path: log_path.display().to_string(),
        md_path: md_path.display().to_string(),
    })
}

pub fn stop_recording(pane_id: PaneId) -> Result<StopResult> {
    let recorder = {
        let mut reg = registry().lock();
        reg.remove(&pane_id)
            .ok_or_else(|| anyhow!("No active recording for pane {}", pane_id))?
    };

    // Detach sink from the LocalPane.
    if let Some(mux) = mux::Mux::try_get() {
        if let Some(pane) = mux.get_pane(pane_id) {
            if let Ok(local) = pane.downcast_arc::<mux::localpane::LocalPane>() {
                local.set_record_sink(None);
            }
        }
    }

    // Final flush + mark stopped.
    {
        let mut inner = recorder.inner.lock();
        inner.stopped = true;
        inner.flush().context("final flush")?;
    }

    let ended_at = Utc::now().to_rfc3339();
    let exit_reason = "recording_stopped".to_string();
    let mut entry = recorder.build_index_entry(Some(ended_at.clone()), Some(&exit_reason));

    // Render markdown to its destination path so the file exists when
    // the user wants it.
    let cfg = load_config();
    let render_cfg = RenderConfig {
        redaction_enabled: cfg.redaction.enabled,
        custom_patterns: cfg.redaction.custom_patterns.clone(),
    };
    let (log_path, md_path, session_id) = recorder.paths();
    match render::render_log(&log_path, &entry, &render_cfg) {
        Ok(out) => {
            entry.block_count = out.block_count;
            entry.total_lines = out.total_lines;
            entry.bytes_raw = out.bytes_raw;
            entry.osc133_active = out.osc133_active;
            entry.redaction_count = out.redaction_count;
            if let Some(parent) = md_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&md_path, out.markdown.as_bytes()) {
                log::warn!("write markdown {}: {e}", md_path.display());
            }
        }
        Err(e) => {
            log::warn!("render markdown for {}: {e}", session_id);
        }
    }

    let block_count = entry.block_count;
    index::upsert_entry(entry).ok();

    Ok(StopResult {
        session_id,
        ended_at,
        block_count,
        exit_reason,
        md_path: md_path.display().to_string(),
    })
}

pub fn current_session(pane_id: PaneId) -> Option<Arc<PaneRecorder>> {
    registry().lock().get(&pane_id).cloned()
}

pub fn recording_status(pane_id: PaneId) -> serde_json::Value {
    let reg = registry().lock();
    if let Some(r) = reg.get(&pane_id) {
        let m = r.metrics();
        let inner = r.inner.lock();
        serde_json::json!({
            "enabled": true,
            "session_id": inner.session_id,
            "started_at": inner.started_at_iso,
            "block_count": m.block_count,
            "bytes": m.bytes,
        })
    } else {
        serde_json::json!({"enabled": false})
    }
}

pub fn attach_trace(pane_id: PaneId, trace_id: String) -> Result<Vec<String>> {
    let reg = registry().lock();
    let r = reg
        .get(&pane_id)
        .ok_or_else(|| anyhow!("No active recording for pane {}", pane_id))?;
    Ok(r.add_trace(trace_id))
}

pub fn list_sessions(project_filter: Option<&str>) -> Result<Vec<IndexEntry>> {
    let entries = index::load_index()?;
    let filtered: Vec<IndexEntry> = entries
        .into_iter()
        .filter(|e| match project_filter {
            Some(p) => {
                e.project_slug == p
                    || e.project_path.as_deref().map(|x| x == p).unwrap_or(false)
            }
            None => true,
        })
        .collect();
    Ok(filtered)
}

/// Render a session's markdown on demand by reading its log file.
pub fn read_session_markdown(session_id: &str) -> Result<String> {
    let entry = index::find_entry(session_id)?
        .ok_or_else(|| anyhow!("Unknown session_id {}", session_id))?;
    let log_path = Path::new(&entry.log_path);
    let cfg = load_config();
    let render_cfg = RenderConfig {
        redaction_enabled: cfg.redaction.enabled,
        custom_patterns: cfg.redaction.custom_patterns.clone(),
    };
    let out = render::render_log(log_path, &entry, &render_cfg)?;
    Ok(out.markdown)
}

/// One-shot export from a live pane regardless of whether recording is
/// active. We use the pane's scrollback + semantic zones to synthesize
/// markdown directly.
pub fn export_pane_markdown(pane_id: PaneId, target: Option<PathBuf>) -> Result<(PathBuf, RenderOutput)> {
    let mux = mux::Mux::try_get().ok_or_else(|| anyhow!("Mux not available"))?;
    let pane = mux
        .get_pane(pane_id)
        .ok_or_else(|| anyhow!("Pane {} not found", pane_id))?;

    // If recording is active, prefer rendering from the .log (it's the
    // authoritative source of truth).
    if let Some(rec) = current_session(pane_id) {
        rec.flush_now();
        let (log_path, _md_path, session_id) = rec.paths();
        let cfg = load_config();
        let render_cfg = RenderConfig {
            redaction_enabled: cfg.redaction.enabled,
            custom_patterns: cfg.redaction.custom_patterns.clone(),
        };
        let entry = index::find_entry(&session_id)?
            .ok_or_else(|| anyhow!("session entry missing"))?;
        let out = render::render_log(&log_path, &entry, &render_cfg)?;
        let dest = target.unwrap_or_else(|| {
            index::sessions_root().join(format!("export-{}.md", session_id))
        });
        if let Some(p) = dest.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        std::fs::write(&dest, out.markdown.as_bytes())
            .with_context(|| format!("write {}", dest.display()))?;
        return Ok((dest, out));
    }

    // No active recording: synthesize from scrollback + semantic zones.
    let dims = pane.get_dimensions();
    let bottom = dims.physical_top + dims.viewport_rows as isize;
    let first_row =
        (bottom - dims.scrollback_rows as isize).max(0);
    let last_row = bottom;
    let (_first, lines) = pane.get_lines(first_row..last_row);
    let scroll_text = lines
        .iter()
        .map(|l| l.as_str().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let (project_path, project_slug) = project_info(&pane);
    let tab_id = pane.pane_id() as u64;
    let (_log_path_unused, md_path, started_at_iso, _stem) =
        build_paths(project_path.as_deref(), &project_slug, tab_id);
    let session_id = uuid::Uuid::new_v4().to_string();
    let cfg = load_config();
    let render_cfg = RenderConfig {
        redaction_enabled: cfg.redaction.enabled,
        custom_patterns: cfg.redaction.custom_patterns.clone(),
    };

    // Build a stub log file containing the scrollback so the renderer
    // can run uniformly.
    let log_path = md_path.with_extension("log");
    let micros = Utc::now().timestamp_micros();
    let b64 = base64::engine::general_purpose::STANDARD.encode(scroll_text.as_bytes());
    let log_line = format!("{}\tout\t{}\n", micros, b64);
    if let Some(p) = log_path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(&log_path, log_line)?;

    // Try semantic zones: emit synthetic OSC 133 events per zone.
    if let Ok(zones) = pane.get_semantic_zones() {
        if !zones.is_empty() {
            let mut bytes: Vec<u8> = Vec::new();
            for zone in zones {
                let micros = Utc::now().timestamp_micros();
                let event = match zone.semantic_type {
                    wezterm_term::SemanticType::Prompt => "block_prompt",
                    wezterm_term::SemanticType::Input => "block_input",
                    wezterm_term::SemanticType::Output => "block_output",
                };
                let line = format!("{}\t{}\t\n", micros, event);
                bytes.extend_from_slice(line.as_bytes());
            }
            // Append the scrollback as the actual output bytes
            let micros2 = Utc::now().timestamp_micros();
            let b64 = base64::engine::general_purpose::STANDARD.encode(scroll_text.as_bytes());
            bytes.extend_from_slice(format!("{}\tout\t{}\n", micros2, b64).as_bytes());
            std::fs::write(&log_path, &bytes)?;
        }
    }

    let entry = IndexEntry {
        unterm_session_id: session_id.clone(),
        tab_id,
        project_path,
        project_slug,
        started_at: started_at_iso,
        ended_at: Some(Utc::now().to_rfc3339()),
        block_count: 0,
        total_lines: 0,
        bytes_raw: scroll_text.len() as u64,
        log_path: log_path.display().to_string(),
        md_path: md_path.display().to_string(),
        exit_reason: Some("user_export".to_string()),
        parent_session_id: None,
        osc133_active: false,
        redaction_active: cfg.redaction.enabled,
        redaction_count: 0,
        trace_ids: Vec::new(),
    };
    let out = render::render_log(&log_path, &entry, &render_cfg)?;
    let dest = target.unwrap_or(md_path);
    if let Some(p) = dest.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(&dest, out.markdown.as_bytes())?;

    let mut entry2 = entry;
    entry2.block_count = out.block_count;
    entry2.total_lines = out.total_lines;
    entry2.osc133_active = out.osc133_active;
    entry2.redaction_count = out.redaction_count;
    index::upsert_entry(entry2).ok();

    Ok((dest, out))
}
