//! MCP request handler — bridges JSON-RPC methods to WezTerm's Mux API.
//! Implements all methods required by unterm-cli compatibility.

use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use config::keyassignment::SpawnTabDomain;
use mux::pane::Pane;
use mux::Mux;
use parking_lot::Mutex;
use portable_pty::CommandBuilder;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ffi::OsString;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use window::WindowOps;

/// Audit log entry
#[derive(Clone, serde::Serialize)]
struct AuditEntry {
    timestamp: String,
    method: String,
    session_id: Option<String>,
    detail: String,
    allowed: bool,
    /// Agent label captured at audit time — either the value the
    /// client set via `agent.identify` or `"anonymous"` if it never
    /// identified itself. Lets the audit UI group by agent without
    /// having to cross-reference connection IDs.
    agent: String,
}

std::thread_local! {
    /// Connection ID of the request currently being handled on this
    /// thread. `handle()` writes this on entry and clears it on exit
    /// (via the RAII guard below). Audit-writing helpers read it to
    /// stamp entries with the calling agent's name.
    static CURRENT_CONN_ID: std::cell::RefCell<Option<u64>> =
        std::cell::RefCell::new(None);
}

/// RAII guard that clears the thread-local connection ID even if the
/// handler panics. Keeps audit attribution from leaking between
/// successive requests on the same thread.
struct ConnectionScope;

impl Drop for ConnectionScope {
    fn drop(&mut self) {
        CURRENT_CONN_ID.with(|cell| *cell.borrow_mut() = None);
    }
}

fn current_agent_label() -> String {
    let conn_id = CURRENT_CONN_ID.with(|cell| *cell.borrow());
    if let Some(id) = conn_id {
        let state = mcp_state().lock();
        if let Some(identity) = state.agents_by_connection.get(&id) {
            return identity.name.clone();
        }
    }
    "anonymous".to_string()
}

/// Pull the agent label off the most recent audit entry. Used in
/// `audit()` to mark "first PTY write from this agent" — by the time
/// we get here, the new entry has already been pushed with its
/// `agent` field set by `current_agent_label()`, so re-doing the
/// thread-local lookup would just produce the same value at the cost
/// of re-acquiring the same lock.
fn entry_agent_from_last_audit(state: &McpState) -> String {
    state
        .audit_log
        .last()
        .map(|e| e.agent.clone())
        .unwrap_or_else(|| "anonymous".to_string())
}

fn shell_command_builder(command: &str) -> CommandBuilder {
    #[cfg(windows)]
    {
        let mut builder = CommandBuilder::new("cmd.exe");
        builder.arg("/C");
        builder.arg(command);
        builder
    }
    #[cfg(not(windows))]
    {
        let shell = std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));
        let mut builder = CommandBuilder::new(shell);
        builder.arg("-lc");
        builder.arg(command);
        builder
    }
}

fn apply_profile_env_to_builder(
    cmd_builder: &mut Option<CommandBuilder>,
    profile: &str,
) -> Result<String> {
    let registry = unterm_profile::ProfileRegistry::load().context("load profile registry")?;
    let (profile_id, _) = registry
        .resolve(profile)
        .ok_or_else(|| anyhow!("profile not found or ambiguous: {profile}"))?;
    let profile_id = profile_id.to_string();
    let store = unterm_profile::default_store().context("open profile secret store")?;
    let env = registry
        .resolve_env(store.as_ref(), &profile_id)
        .with_context(|| format!("resolve profile env for {profile_id}"))?;
    if !env.is_empty() {
        let builder = cmd_builder.get_or_insert_with(CommandBuilder::new_default_prog);
        for (key, value) in env {
            builder.env(key, value);
        }
    }
    Ok(profile_id)
}

fn cwd_url_to_path(raw: &str) -> Option<String> {
    if raw.trim().is_empty() {
        return None;
    }
    if let Ok(url) = url::Url::parse(raw) {
        if url.scheme() == "file" {
            return url
                .to_file_path()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
        }
    }
    Some(raw.to_string())
}

/// Command execution policy
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CommandPolicy {
    enabled: bool,
    blocked_patterns: Vec<String>,
    allowed_patterns: Vec<String>,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            blocked_patterns: Vec::new(),
            allowed_patterns: Vec::new(),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct ProxyNodeConfig {
    name: String,
    url: String,
    /// Transient probe results — recomputed on every `proxy.status`, never
    /// trusted from disk (a node's reachability/latency now has nothing to do
    /// with what was true last time it was written). `skip_serializing` keeps
    /// proxy.json to just name+url so hand-edits stay clean.
    #[serde(skip_serializing)]
    latency_ms: Option<u64>,
    #[serde(skip_serializing)]
    available: bool,
}

impl Default for ProxyNodeConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            url: String::new(),
            latency_ms: None,
            available: false,
        }
    }
}

/// Auto-rotation: keep a user-chosen pool of nodes healthy. A background
/// monitor probes the current node every `interval_secs`; when it goes
/// unreachable, it probes the whole pool and switches to the fastest live one.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct RotationSettings {
    enabled: bool,
    /// Clash/mihomo Selector group to rotate within. When non-empty, rotation
    /// runs in *clash mode*: `pool` holds node names inside this group and we
    /// fail over by switching the group's selection via the controller API.
    /// When empty, rotation runs in legacy *endpoint mode* (`pool` holds
    /// `proxy.nodes` names and we swap the injected HTTP_PROXY url).
    group: String,
    /// Node names eligible for rotation. Empty pool disables rotation even if
    /// `enabled` (nothing to rotate to).
    pool: Vec<String>,
    /// Seconds between health checks of the active node.
    interval_secs: u64,
}

impl Default for RotationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            group: String::new(),
            pool: Vec::new(),
            interval_secs: 30,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct ProxySettings {
    enabled: bool,
    mode: String,
    http_proxy: Option<String>,
    socks_proxy: Option<String>,
    no_proxy: String,
    current_node: Option<String>,
    nodes: Vec<ProxyNodeConfig>,
    rotation: RotationSettings,
    /// Manual Clash/mihomo controller override (host:port) for when
    /// auto-discovery can't find it — e.g. on Windows where there's no Unix
    /// socket and the controller isn't in a scanned config. Empty = auto.
    #[serde(default)]
    clash_controller: String,
    /// Bearer secret for the manual controller (if the API requires one).
    #[serde(default)]
    clash_secret: String,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "off".to_string(),
            http_proxy: None,
            socks_proxy: None,
            no_proxy: "localhost,127.0.0.1,::1".to_string(),
            current_node: None,
            nodes: Vec::new(),
            rotation: RotationSettings::default(),
            clash_controller: String::new(),
            clash_secret: String::new(),
        }
    }
}

/// What an MCP client tells us about itself via `agent.identify`.
/// Identity is *self-asserted* — no cryptographic verification, just a
/// label the agent uses so audit logs and the (future) confirmation
/// flow can group by "claude-code" vs "unterm-cli" vs "anonymous".
#[derive(Clone, serde::Serialize)]
pub struct AgentIdentity {
    pub name: String,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
    /// Source address of the TCP connection (`127.0.0.1:port`). Useful
    /// for distinguishing two concurrent connections claiming the same
    /// agent name.
    pub peer_addr: String,
    /// RFC3339 timestamp of when `agent.identify` was called on this
    /// connection.
    pub identified_at: String,
}

/// Per-connection context passed in to every `handle()` call. Lets
/// methods that need it (`agent.identify`, audit annotators) tie the
/// request back to a specific TCP connection without resorting to
/// thread-locals at the call site.
pub struct ConnectionContext {
    pub conn_id: u64,
    pub peer_addr: String,
}

impl ConnectionContext {
    /// Synthetic context for in-process callers that aren't talking to
    /// the MCP TCP server — e.g. the web settings HTTP handlers that
    /// dispatch via the same `McpHandler` instance. `conn_id` 0 is
    /// reserved (the TCP server starts allocating at 1).
    pub fn internal(source: &str) -> Self {
        Self {
            conn_id: 0,
            peer_addr: format!("internal:{source}"),
        }
    }
}

/// Decision the user (or a timeout) returns to a pending MCP
/// confirmation. `AlwaysAllow` additionally remembers the agent so
/// future calls by the same agent bypass the banner.
#[derive(Debug, Clone, Copy)]
pub enum ConfirmationDecision {
    Allow,
    Block,
    AlwaysAllow,
}

/// Internal result of `gate_pty_write`. Callers either proceed (with
/// audit + write) or return an error to the MCP client.
enum GateOutcome {
    Allow,
    Block,
}

/// Pending confirmation request. The MCP-worker thread parks on
/// `responder.recv_timeout(...)` waiting for the GUI thread to take
/// this off the queue and `send` a decision.
struct PendingConfirmation {
    id: u64,
    agent: String,
    input_preview: String,
    pane_id: u64,
    method: String,
    requested_at: String,
    responder: std::sync::mpsc::SyncSender<ConfirmationDecision>,
}

/// Read-only view of a pending confirmation for the GUI. Doesn't
/// carry the `SyncSender` (it's neither Clone nor Send-friendly to
/// share), so the resolve API is paired: read with
/// `pending_confirmation_view`, decide via `resolve_confirmation`.
#[derive(Clone, serde::Serialize)]
pub struct ConfirmationView {
    pub id: u64,
    pub agent: String,
    pub input_preview: String,
    pub pane_id: u64,
    pub method: String,
    pub requested_at: String,
}

/// Lifecycle state for a single `session.suggest` suggestion. Stored
/// alongside the suggestion so `session.suggest_status` can report what
/// happened to a previously-posted suggestion (and so the suggest UI
/// can decide whether to render it).
#[derive(Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SuggestionState {
    Pending,
    Accepted { at: String, ran_immediately: bool },
    Dismissed { at: String },
    Expired { at: String },
    Cancelled { at: String },
}

/// Where a suggestion came from. All fields optional — anonymous agents
/// can still post suggestions, and `agent.identify` is the
/// recommended-but-not-required way to provide a recognizable label.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SuggestionSource {
    pub agent: Option<String>,
    pub session: Option<String>,
}

/// One MCP-driven suggestion. The text is **not** written to PTY at
/// post time — only when the user actively accepts it via the
/// suggest UI (or programmatically via `accept_suggestion`).
#[derive(Clone, serde::Serialize)]
pub struct Suggestion {
    pub id: String,
    pub pane_id: u64,
    pub text: String,
    pub rationale: Option<String>,
    pub source: SuggestionSource,
    pub created_at: String,
    pub ttl_ms: u64,
    pub state: SuggestionState,
    /// Connection ID of the agent that posted this suggestion. Used to
    /// stamp accept/dismiss audit entries with the originating agent
    /// (even after the connection has disconnected).
    pub posted_by_conn: u64,
    /// Cached agent label from `agent.identify`, so the suggest UI can
    /// render "from claude-code" even after the connection drops.
    pub posted_by_agent: String,
}

/// Global state for audit + policy + workspace
struct McpState {
    audit_log: Vec<AuditEntry>,
    policy: CommandPolicy,
    proxy: ProxySettings,
    /// Monotonically-increasing count of PTY writes that came from an
    /// MCP client (session.input / exec.send). Surfaced in the status
    /// bar so the user can see "AI just wrote N times" at a glance.
    input_event_count: u64,
    /// When the most recent MCP-origin PTY write happened. Status bar
    /// uses this for a short flash effect after each event.
    last_input_at: Option<std::time::Instant>,
    /// Per-connection agent identity. Connections that never called
    /// `agent.identify` are absent from the map and rendered as
    /// "anonymous" in audit views. Cleared when the connection drops.
    agents_by_connection: HashMap<u64, AgentIdentity>,
    /// First time we ever saw a given agent name (across all
    /// connections). Used by the (future, P0.3) "first-time per agent"
    /// confirmation flow to decide whether this agent is novel enough
    /// to interrupt the user.
    known_agents: HashMap<String, String>,
    /// Names of agents that have ever called `session.input` /
    /// `exec.send` since startup. Used to flag "first PTY write by
    /// this agent" in the audit log.
    agents_with_input_history: std::collections::HashSet<String>,
    /// pane_id → (agent name, when it last wrote there). Drives the
    /// left tab bar's "who is driving this pane" subtitle.
    pane_agents: HashMap<u64, (String, std::time::Instant)>,
    /// Agents the user has explicitly elected to skip future
    /// confirmation banners for (via the "Always allow this agent"
    /// affordance on a banner). Distinct from the static
    /// `mcp_trusted_agents` config — this list is session-only and
    /// only ever grows via user action.
    confirmed_agents: std::collections::HashSet<String>,
    /// Banners parked waiting for the user to allow/block. Each
    /// element carries a `SyncSender` the MCP worker is blocked on;
    /// the GUI thread fulfils it by sending a `ConfirmationDecision`.
    pending_confirmations: Vec<PendingConfirmation>,
    /// Monotonic id allocator for pending confirmations.
    confirmation_seq: u64,
    /// All suggestions posted via `session.suggest`, indexed by id.
    /// Both live (pending) and dead (accepted/dismissed/expired) are
    /// kept so `session.suggest_status` can return a lifecycle
    /// answer. Capped at SUGGEST_MAX entries — oldest dropped first.
    suggestions: HashMap<String, Suggestion>,
    /// Insertion order of suggestion ids, used for FIFO eviction when
    /// the map exceeds SUGGEST_MAX.
    suggestion_order: Vec<String>,
    /// Monotonically-increasing suffix appended to suggestion ids so
    /// they're unique even when the system clock has poor resolution.
    suggestion_seq: u64,
}

/// Snapshot of MCP input activity for UI surfaces. Returned by
/// `recent_mcp_input_activity()` so renderers don't need to know about
/// the global state Mutex.
pub struct McpInputActivity {
    pub count: u64,
    pub seconds_since_last: Option<f32>,
}

/// Read-only view of how often MCP clients have written to PTYs since
/// startup, plus how long ago the last write was. The status bar polls
/// this on every paint to decide whether to render the `⚡` flash.
pub fn recent_mcp_input_activity() -> McpInputActivity {
    let state = mcp_state().lock();
    McpInputActivity {
        count: state.input_event_count,
        seconds_since_last: state.last_input_at.map(|t| t.elapsed().as_secs_f32()),
    }
}

/// Oldest pending confirmation as a UI-friendly view. Returns `None`
/// when nothing is waiting. The GUI paints a banner whenever this is
/// `Some` and routes Enter/Esc/Ctrl-A to `resolve_confirmation`.
pub fn pending_confirmation_view() -> Option<ConfirmationView> {
    let state = mcp_state().lock();
    state
        .pending_confirmations
        .first()
        .map(|p| ConfirmationView {
            id: p.id,
            agent: p.agent.clone(),
            input_preview: p.input_preview.clone(),
            pane_id: p.pane_id,
            method: p.method.clone(),
            requested_at: p.requested_at.clone(),
        })
}

/// Number of pending confirmations. Cheaper than `pending_confirmation_view`
/// when the caller only needs to know "is the banner needed?".
pub fn pending_confirmation_count() -> usize {
    mcp_state().lock().pending_confirmations.len()
}

/// Fulfil a pending confirmation. Returns true if the id was found
/// and the worker thread was unblocked. `AlwaysAllow` additionally
/// remembers the agent so future calls by that agent name bypass the
/// banner for the rest of the session.
pub fn resolve_confirmation(id: u64, decision: ConfirmationDecision) -> bool {
    let mut state = mcp_state().lock();
    let Some(idx) = state.pending_confirmations.iter().position(|p| p.id == id) else {
        return false;
    };
    let pending = state.pending_confirmations.remove(idx);
    if matches!(decision, ConfirmationDecision::AlwaysAllow) {
        state.confirmed_agents.insert(pending.agent.clone());
        // Persist immediately so the choice survives a restart. The
        // snapshot is small (~few-KB JSON); the cost of writing on
        // every Alt+A is negligible compared to user surprise on
        // re-prompt after restart.
        let snapshot = state.confirmed_agents.clone();
        save_persisted_trusted(&snapshot);
    }
    // Drop the lock before sending so the waiting worker can
    // re-acquire it on its own audit/write path.
    drop(state);
    // `send` returns Err only if the receiver was dropped (the
    // worker timed out and gave up). That's fine — the audit will
    // already show the timeout entry, and the worker has returned
    // an error to its MCP client.
    let _ = pending.responder.send(decision);
    true
}

/// Aggregate counters surfaced in the Insights overlay. Single
/// snapshot under one Mutex acquire so renderers don't need to make
/// six separate calls.
pub struct InsightsMcpSnapshot {
    pub input_count: u64,
    pub seconds_since_last_input: Option<f32>,
    pub agents_seen: usize,
    pub pending_suggestions: usize,
    pub pending_confirmations: usize,
    pub recent_audit: Vec<String>,
}

/// Read all the MCP-side numbers the Insights overlay needs in one
/// lock acquisition. `recent_audit_limit` caps how many audit
/// entries (newest first) are rendered as one-line summaries.
pub fn insights_mcp_snapshot(recent_audit_limit: usize) -> InsightsMcpSnapshot {
    let state = mcp_state().lock();
    let recent_audit: Vec<String> = state
        .audit_log
        .iter()
        .rev()
        .take(recent_audit_limit)
        .map(|e| {
            // Time-of-day extracted from the rfc3339 timestamp; we
            // don't need full ISO precision in the overlay.
            let time = e
                .timestamp
                .split('T')
                .nth(1)
                .and_then(|tail| tail.split('.').next())
                .unwrap_or(&e.timestamp);
            format!(
                "{time}  {:<24} {}  agent={}",
                e.method,
                truncate(&e.detail, 60),
                e.agent
            )
        })
        .collect();
    let pending_suggestions = state
        .suggestions
        .values()
        .filter(|s| matches!(s.state, SuggestionState::Pending))
        .count();
    InsightsMcpSnapshot {
        input_count: state.input_event_count,
        seconds_since_last_input: state.last_input_at.map(|t| t.elapsed().as_secs_f32()),
        agents_seen: state.known_agents.len(),
        pending_suggestions,
        pending_confirmations: state.pending_confirmations.len(),
        recent_audit,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Snapshot the audit log as a pretty-printed JSON string. Used by
/// the status bar chip click (until P1.3's proper overlay lands) so
/// the user can paste the log into any text editor for review.
/// Returns at most `limit` most-recent entries, newest first.
/// Read-only snapshot of the trust state for the Web Settings panel.
/// `runtime` is the union of (loaded-from-disk) ∪ (Alt+A this session).
/// `static_config` is the user's `mcp_trusted_agents` Lua config.
/// `audit_counts` is a per-agent write count derived from `audit_log`
/// so the panel can show "claude-code: 47 writes" alongside the trust
/// toggle. Single lock acquire.
pub fn trust_snapshot() -> Value {
    let state = mcp_state().lock();
    let mut runtime: Vec<&String> = state.confirmed_agents.iter().collect();
    runtime.sort();
    let cfg = config::configuration();
    let mut static_config: Vec<&String> = cfg.mcp_trusted_agents.iter().collect();
    static_config.sort();
    let mut counts: HashMap<String, u64> = HashMap::new();
    for entry in &state.audit_log {
        // entry.agent is a plain String, defaulting to "anonymous"
        // when the connection hadn't called agent.identify yet.
        if !entry.agent.is_empty() {
            *counts.entry(entry.agent.clone()).or_insert(0) += 1;
        }
    }
    let mut count_list: Vec<(&String, &u64)> = counts.iter().collect();
    count_list.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    json!({
        "runtime": runtime,
        "static_config": static_config,
        "audit_counts": count_list
            .into_iter()
            .map(|(a, c)| json!({ "agent": a, "writes": c }))
            .collect::<Vec<_>>(),
    })
}

/// Revoke trust at runtime. Removes from `confirmed_agents` AND from
/// the persisted JSON so the next session also sees the agent as
/// "needs confirmation". Returns `true` if the name was present.
/// Note: this can't remove an entry from `mcp_trusted_agents` Lua
/// config — that's static and the user has to edit unterm.lua. The
/// Web Settings panel surfaces both lists so the user can see why
/// removal from runtime didn't actually un-trust.
pub fn revoke_trust(agent: &str) -> bool {
    let mut state = mcp_state().lock();
    let was = state.confirmed_agents.remove(agent);
    let snapshot = state.confirmed_agents.clone();
    drop(state);
    save_persisted_trusted(&snapshot);
    was
}

/// Promote an agent to trust without going through the banner. Used
/// by the Web Settings panel's "trust this agent now" button.
pub fn grant_trust(agent: &str) -> bool {
    let mut state = mcp_state().lock();
    let was_new = state.confirmed_agents.insert(agent.to_string());
    let snapshot = state.confirmed_agents.clone();
    drop(state);
    save_persisted_trusted(&snapshot);
    was_new
}

pub fn audit_log_snapshot_json(limit: usize) -> String {
    let state = mcp_state().lock();
    let recent: Vec<&AuditEntry> = state.audit_log.iter().rev().take(limit).collect();
    serde_json::to_string_pretty(&recent).unwrap_or_else(|_| "[]".to_string())
}

/// Pending suggestions for a specific pane, ordered oldest-first.
/// Called by the suggest UI on every paint to decide what to render.
/// Side-effect: lazily flips suggestions whose TTL has elapsed from
/// `Pending` to `Expired` — keeps the queue from showing stale text.
/// Name of the agent that most recently drove `pane_id` (PTY write via
/// MCP), if it was active within the last 15 minutes. Used by the left
/// tab bar subtitle.
pub fn agent_for_pane(pane_id: u64) -> Option<String> {
    let state = mcp_state().lock();
    state.pane_agents.get(&pane_id).and_then(|(name, at)| {
        if at.elapsed().as_secs() <= 15 * 60 {
            Some(name.clone())
        } else {
            None
        }
    })
}

/// Resolve the AI agent driving a pane. Tries MCP first (an external
/// agent calling session.input / exec.send registers itself there),
/// then falls back to inspecting the pane's foreground process tree
/// — for in-pane CLIs like `claude`, `codex`, `gemini`, `kimi`, `aider`,
/// `opencode`, `trae-cli`, `zcode`, `cursor-agent`. The MCP path only fires when an
/// external agent writes to a pane; an interactive in-tab CLI is
/// invisible to MCP, so without the process-tree check the chip
/// never lit up for the most common case.
pub fn detect_agent_for_pane(
    pane_id: u64,
    proc_info: Option<&procinfo::LocalProcessInfo>,
) -> Option<String> {
    if let Some(name) = agent_for_pane(pane_id) {
        return Some(name);
    }
    let info = proc_info?;
    fn match_name(name: &str) -> Option<&'static str> {
        let lower = name.to_ascii_lowercase();
        // Strip a leading path / common extensions so "claude.exe" /
        // "/usr/local/bin/claude" both hit.
        let bare = std::path::Path::new(&lower)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&lower);
        match bare {
            "claude" => Some("claude"),
            "codex" => Some("codex"),
            "gemini" => Some("gemini"),
            "kimi" | "kimi-code" => Some("kimi"),
            "aider" => Some("aider"),
            "opencode" => Some("opencode"),
            "trae" | "trae-cli" | "trae_agent" | "trae-agent" => Some("trae"),
            "zcode" | "z-code" | "z code" => Some("zcode"),
            "cursor-agent" | "cursoragent" => Some("cursor-agent"),
            _ => None,
        }
    }
    fn walk(p: &procinfo::LocalProcessInfo) -> Option<String> {
        if let Some(hit) = match_name(&p.name) {
            return Some(hit.to_string());
        }
        // Also peek the first argv element — some launchers exec a
        // wrapper script whose process name doesn't match the agent.
        if let Some(arg0) = p.argv.first() {
            if let Some(hit) = match_name(arg0) {
                return Some(hit.to_string());
            }
        }
        for child in p.children.values() {
            if let Some(hit) = walk(child) {
                return Some(hit);
            }
        }
        None
    }
    walk(info)
}

/// Cached `(agent, cwd-basename)` for a pane's status surfaces (vertical tab
/// rows and the top tab titles). Resolving it means a foreground-process
/// snapshot plus per-process PEB reads (cwd/argv) across the pane's subtree —
/// tens of milliseconds on Windows. Doing that for every tab on every
/// `update_title` (i.e. on every tab switch) was the dominant switch latency
/// once a window held several tabs. We instead serve the last known value
/// instantly and refresh it on a worker thread, mirroring the stats-bar
/// caches. ~2s staleness is invisible for an agent/cwd label.
#[derive(Clone, Default)]
struct PaneAgentCwd {
    agent: Option<String>,
    /// Foreground command running in the pane's shell (e.g. `npm run dev`,
    /// `git log`), or None when the shell is sitting idle at its prompt.
    /// Derived from the same foreground-process snapshot as `agent`, so it
    /// costs nothing extra and is refreshed on the same worker thread.
    foreground: Option<String>,
    cwd: Option<String>,
    /// Full cwd used to disambiguate projects that share the same basename.
    cwd_path: Option<String>,
    /// Stable repository/workspace root derived off the render thread.
    project: Option<String>,
    project_path: Option<String>,
}

/// Executable base names we treat as "the shell itself" — when the pane's
/// foreground process is one of these, no command is running and the row
/// should fall back to the shell/pane name rather than a command title.
fn is_shell_exe(bare: &str) -> bool {
    matches!(
        bare,
        "powershell"
            | "pwsh"
            | "cmd"
            | "bash"
            | "zsh"
            | "fish"
            | "sh"
            | "dash"
            | "ksh"
            | "tcsh"
            | "csh"
            | "nu"
            | "elvish"
            | "xonsh"
            | "wsl"
            | "conhost"
    )
}

/// Reduce a pane's foreground process to a short command title for the
/// sidebar. Returns None when the foreground process is just the shell (an
/// idle prompt). Otherwise the executable base name, optionally suffixed by a
/// bare first argument so `git log` / `cargo build` read as their subcommand.
fn foreground_command_title(info: &procinfo::LocalProcessInfo) -> Option<String> {
    let bare = info
        .executable
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if bare.is_empty() || is_shell_exe(&bare) {
        return None;
    }
    let mut title = bare.clone();
    // argv[0] is the executable; argv[1] is the first real argument. Append it
    // only when it looks like a subcommand (a bare word, not a flag or a path)
    // so common tools read naturally without pulling in noisy paths/flags.
    if let Some(arg) = info.argv.get(1).map(|a| a.trim()) {
        let looks_like_subcommand = !arg.is_empty()
            && !arg.starts_with('-')
            && !arg.contains('/')
            && !arg.contains('\\')
            && !arg.contains('.')
            && termwiz::cell::unicode_column_width(arg, None) <= 16;
        if looks_like_subcommand {
            title = format!("{bare} {arg}");
        }
    }
    Some(title)
}

const AGENT_CWD_TTL: std::time::Duration = std::time::Duration::from_millis(2000);
const AGENT_CWD_PRUNE_AFTER: std::time::Duration = std::time::Duration::from_secs(60);
const AGENT_CWD_PRUNE_MIN_SIZE: usize = 128;
const AGENT_CWD_MAX_INFLIGHT: usize = 16;

fn agent_cwd_cache() -> &'static Mutex<HashMap<u64, (std::time::Instant, PaneAgentCwd)>> {
    static C: std::sync::OnceLock<Mutex<HashMap<u64, (std::time::Instant, PaneAgentCwd)>>> =
        std::sync::OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn agent_cwd_inflight() -> &'static Mutex<std::collections::HashSet<u64>> {
    static S: std::sync::OnceLock<Mutex<std::collections::HashSet<u64>>> =
        std::sync::OnceLock::new();
    S.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

/// Non-blocking `(agent, cwd)` for status surfaces. Returns the cached value
/// immediately (empty until the first refresh lands) and kicks off an
/// off-thread refresh when the entry is missing or older than `AGENT_CWD_TTL`.
/// Safe to call from the render thread — it never touches the filesystem or
/// the process table itself.
pub fn agent_and_cwd_for_pane(pane_id: u64) -> (Option<String>, Option<String>) {
    let v = agent_fg_cwd_for_pane_inner(pane_id);
    (v.agent, v.cwd)
}

/// Non-blocking `(agent, foreground-command, cwd)` for status surfaces that
/// want to show the running command as the primary label (the left tab bar).
/// Same caching contract as `agent_and_cwd_for_pane`: instant cached read,
/// off-thread refresh, never touches the process table on the caller thread.
pub fn agent_fg_cwd_for_pane(
    pane_id: u64,
) -> (Option<String>, Option<String>, Option<String>) {
    let v = agent_fg_cwd_for_pane_inner(pane_id);
    (v.agent, v.foreground, v.cwd)
}

/// Non-blocking sidebar metadata including the full cwd.  The shorter helper
/// above is kept for existing status surfaces that only have room for a
/// basename.
pub fn agent_fg_cwd_path_for_pane(
    pane_id: u64,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let v = agent_fg_cwd_for_pane_inner(pane_id);
    (
        v.agent,
        v.foreground,
        v.cwd,
        v.cwd_path,
        v.project,
        v.project_path,
    )
}

fn project_root_for_path(path: &std::path::Path) -> std::path::PathBuf {
    for ancestor in path.ancestors() {
        if ancestor.join(".git").exists()
            || ancestor.join(".hg").exists()
            || ancestor.join(".svn").exists()
        {
            return ancestor.to_path_buf();
        }
    }
    path.to_path_buf()
}

fn agent_fg_cwd_for_pane_inner(pane_id: u64) -> PaneAgentCwd {
    let (cached, need_refresh) = {
        let mut cache = agent_cwd_cache().lock();
        if cache.len() > AGENT_CWD_PRUNE_MIN_SIZE {
            cache.retain(|_, (at, _)| at.elapsed() < AGENT_CWD_PRUNE_AFTER);
        }
        match cache.get(&pane_id) {
            Some((at, v)) if at.elapsed() < AGENT_CWD_TTL => {
                return v.clone();
            }
            Some((_, v)) => (v.clone(), true),
            None => (PaneAgentCwd::default(), true),
        }
    };
    let should_refresh = if need_refresh {
        let mut inflight = agent_cwd_inflight().lock();
        if inflight.len() < AGENT_CWD_MAX_INFLIGHT {
            inflight.insert(pane_id)
        } else {
            false
        }
    } else {
        false
    };
    if should_refresh {
        if std::thread::Builder::new()
            .name("agent-cwd-refresh".into())
            .spawn(move || {
                let fresh = compute_agent_cwd(pane_id);
                agent_cwd_cache()
                    .lock()
                    .insert(pane_id, (std::time::Instant::now(), fresh));
                agent_cwd_inflight().lock().remove(&pane_id);
            })
            .is_err()
        {
            agent_cwd_inflight().lock().remove(&pane_id);
        }
    }
    cached
}

/// The expensive part, run on a worker thread: snapshot the pane's foreground
/// process and derive the agent name + cwd basename.
fn compute_agent_cwd(pane_id: u64) -> PaneAgentCwd {
    let Some(mux) = Mux::try_get() else {
        return PaneAgentCwd::default();
    };
    let Some(pane) = mux.get_pane(pane_id as mux::pane::PaneId) else {
        return PaneAgentCwd::default();
    };
    // This function already runs off the render thread, so do the real work
    // here. Using AllowStale on a cold cache only queued another worker and
    // then stored an empty `(agent, cwd)` result for AGENT_CWD_TTL.
    let proc_info = pane.get_foreground_process_info(mux::pane::CachePolicy::FetchImmediate);
    let agent = detect_agent_for_pane(pane_id, proc_info.as_ref());
    // When an agent drives the pane its name already IS the title, so we skip
    // the command probe; otherwise reduce the foreground process to a short
    // command title (`None` while the shell is idle at its prompt).
    let foreground = if agent.is_some() {
        None
    } else {
        proc_info.as_ref().and_then(foreground_command_title)
    };
    let cwd_path_buf = pane
        .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
        .and_then(|url| url.to_file_path().ok());
    let cwd_path = cwd_path_buf
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());
    let cwd = cwd_path.as_ref().and_then(|path| {
        let p = std::path::Path::new(path);
        if dirs_next::home_dir().as_deref() == Some(p) {
            Some("~".to_string())
        } else {
            p.file_name().map(|n| n.to_string_lossy().to_string())
        }
    });
    let project_path_buf = cwd_path_buf.as_deref().map(project_root_for_path);
    let project = project_path_buf.as_ref().and_then(|path| {
        if dirs_next::home_dir().as_deref() == Some(path.as_path()) {
            Some("~".to_string())
        } else {
            path.file_name().map(|name| name.to_string_lossy().to_string())
        }
    });
    let project_path = project_path_buf.map(|path| path.to_string_lossy().to_string());
    PaneAgentCwd {
        agent,
        foreground,
        cwd,
        cwd_path,
        project,
        project_path,
    }
}

pub fn pending_suggestions_for_pane(pane_id: u64) -> Vec<Suggestion> {
    let mut state = mcp_state().lock();
    let now = chrono::Local::now();
    let ids_to_check: Vec<String> = state
        .suggestions
        .iter()
        .filter(|(_, s)| s.pane_id == pane_id && matches!(s.state, SuggestionState::Pending))
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids_to_check {
        let expired = {
            let s = &state.suggestions[&id];
            chrono::DateTime::parse_from_rfc3339(&s.created_at)
                .map(|created| {
                    let elapsed = now.signed_duration_since(created.with_timezone(&chrono::Local));
                    elapsed.num_milliseconds() as u64 > s.ttl_ms
                })
                .unwrap_or(false)
        };
        if expired {
            if let Some(s) = state.suggestions.get_mut(&id) {
                s.state = SuggestionState::Expired {
                    at: now.to_rfc3339(),
                };
            }
        }
    }
    let mut out: Vec<Suggestion> = state
        .suggestions
        .values()
        .filter(|s| s.pane_id == pane_id && matches!(s.state, SuggestionState::Pending))
        .cloned()
        .collect();
    out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    out
}

/// Mark a suggestion as accepted and return the text the caller (the
/// suggest UI) should write to the pane's PTY. `run_immediately = true`
/// means the user hit `Alt+Enter`, so the UI is expected to append
/// `\n` after writing the text. Audit is stamped here so the lifecycle
/// trail captures *who accepted* (the user — represented as
/// `agent="user"`), distinct from *who posted* (the agent label
/// frozen on the suggestion when it was created).
pub fn accept_suggestion(id: &str, run_immediately: bool) -> Result<String, String> {
    let mut state = mcp_state().lock();
    let suggestion = state
        .suggestions
        .get_mut(id)
        .ok_or_else(|| format!("unknown suggestion_id: {id}"))?;
    if !matches!(suggestion.state, SuggestionState::Pending) {
        return Err(format!("suggestion {id} is not pending"));
    }
    let text = suggestion.text.clone();
    let posted_by = suggestion.posted_by_agent.clone();
    let pane_id = suggestion.pane_id;
    suggestion.state = SuggestionState::Accepted {
        at: chrono::Local::now().to_rfc3339(),
        ran_immediately: run_immediately,
    };
    // Drop the lock before audit — `audit()` re-acquires it.
    drop(state);
    let entry = AuditEntry {
        timestamp: chrono::Local::now().to_rfc3339(),
        method: "session.suggest.accept".to_string(),
        session_id: Some(pane_id.to_string()),
        detail: format!(
            "id={} posted_by={} run={} {}",
            id,
            posted_by,
            run_immediately,
            input_preview(&text)
        ),
        allowed: true,
        agent: "user".to_string(),
    };
    let mut state = mcp_state().lock();
    state.audit_log.push(entry);
    Ok(text)
}

/// Mark a suggestion dismissed. Symmetric with `accept_suggestion` —
/// the UI calls this when the user hits Esc (or clicks the dismiss
/// affordance). Returns an error if the id is unknown or already
/// resolved.
pub fn dismiss_suggestion(id: &str) -> Result<(), String> {
    let mut state = mcp_state().lock();
    let suggestion = state
        .suggestions
        .get_mut(id)
        .ok_or_else(|| format!("unknown suggestion_id: {id}"))?;
    if !matches!(suggestion.state, SuggestionState::Pending) {
        return Err(format!("suggestion {id} is not pending"));
    }
    let pane_id = suggestion.pane_id;
    let posted_by = suggestion.posted_by_agent.clone();
    suggestion.state = SuggestionState::Dismissed {
        at: chrono::Local::now().to_rfc3339(),
    };
    drop(state);
    let entry = AuditEntry {
        timestamp: chrono::Local::now().to_rfc3339(),
        method: "session.suggest.dismiss".to_string(),
        session_id: Some(pane_id.to_string()),
        detail: format!("id={} posted_by={}", id, posted_by),
        allowed: true,
        agent: "user".to_string(),
    };
    mcp_state().lock().audit_log.push(entry);
    Ok(())
}

/// Render an `input` payload as a single-line audit-friendly summary.
/// Truncates to MAX_LEN graphemes, escapes ASCII control chars, and tags
/// whether the original contained any non-printable bytes — so log
/// readers can spot `\x03` (Ctrl-C) injection without us dumping every
/// raw keystroke (which could include passwords).
fn input_preview(input: &str) -> String {
    const MAX_LEN: usize = 80;
    let total_len = input.len();
    let mut has_ctrl = false;
    let mut rendered = String::with_capacity(input.len().min(MAX_LEN * 4) + 16);
    for c in input.chars() {
        if c == '\n' || c == '\r' || c == '\t' {
            // Keep whitespace visible but readable.
            match c {
                '\n' => rendered.push_str("\\n"),
                '\r' => rendered.push_str("\\r"),
                '\t' => rendered.push_str("\\t"),
                _ => {}
            }
        } else if c.is_control() {
            has_ctrl = true;
            rendered.push_str(&format!("\\x{:02x}", c as u32));
        } else {
            rendered.push(c);
        }
        if rendered.chars().count() >= MAX_LEN {
            rendered.push('…');
            break;
        }
    }
    format!(
        "len={}{} {}",
        total_len,
        if has_ctrl { " [ctrl]" } else { "" },
        rendered
    )
}

/// Path of the persistent trust list. Trust state survives Unterm
/// restarts so the user doesn't have to Alt+A their preferred agent
/// once per session.
fn trusted_agents_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("trusted_agents.json")
}

/// Load the persisted trust list. Schema:
///   { "agents": ["claude-code", "cursor", ...] }
/// Missing file / unreadable file / bad JSON → empty list. Silent
/// degradation is correct here: the worst case is the user has to
/// Alt+A again, which is one keystroke.
fn load_persisted_trusted() -> std::collections::HashSet<String> {
    let Ok(text) = std::fs::read_to_string(trusted_agents_path()) else {
        return std::collections::HashSet::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return std::collections::HashSet::new();
    };
    value
        .get("agents")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Atomic write of the trust list. Tempfile + rename so a concurrent
/// reader sees either the old set or the new set, never half-written.
/// Errors logged but not propagated: a missing rename succeeds for
/// the in-memory state, which is what matters for the current
/// session — the next restart just won't remember.
fn save_persisted_trusted(agents: &std::collections::HashSet<String>) {
    let path = trusted_agents_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::warn!("save trusted_agents.json: create_dir_all: {e:#}");
            return;
        }
    }
    let mut sorted: Vec<&String> = agents.iter().collect();
    sorted.sort();
    let body = json!({ "agents": sorted });
    let pretty = match serde_json::to_string_pretty(&body) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("save trusted_agents.json: serialize: {e:#}");
            return;
        }
    };
    let tmp = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp, pretty) {
        log::warn!("save trusted_agents.json: write temp: {e:#}");
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        log::warn!("save trusted_agents.json: rename: {e:#}");
    }
}

fn mcp_state() -> &'static Mutex<McpState> {
    static STATE: std::sync::OnceLock<Mutex<McpState>> = std::sync::OnceLock::new();
    STATE.get_or_init(|| {
        // Load persisted trust at startup so the user's choice from
        // last session survives. confirmed_agents is the merged set
        // (persisted + this-session-Alt+A); save_persisted_trusted
        // re-snapshots the whole thing on every mutation.
        let confirmed_agents = load_persisted_trusted();
        Mutex::new(McpState {
            audit_log: Vec::new(),
            policy: CommandPolicy::default(),
            proxy: load_proxy_settings(),
            input_event_count: 0,
            last_input_at: None,
            agents_by_connection: HashMap::new(),
            known_agents: HashMap::new(),
            pane_agents: HashMap::new(),
            agents_with_input_history: std::collections::HashSet::new(),
            confirmed_agents,
            pending_confirmations: Vec::new(),
            confirmation_seq: 0,
            suggestions: HashMap::new(),
            suggestion_order: Vec::new(),
            suggestion_seq: 0,
        })
    })
}

pub struct McpHandler;

impl McpHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle(&self, ctx: &ConnectionContext, method: &str, params: &Value) -> Result<Value> {
        CURRENT_CONN_ID.with(|cell| *cell.borrow_mut() = Some(ctx.conn_id));
        let _scope = ConnectionScope;
        match method {
            // Agent self-identification — call this right after
            // `auth.login` to tag your connection so audit entries
            // group by agent name instead of by connection ID.
            "agent.identify" => self.agent_identify(ctx, params),
            "agent.whoami" => self.agent_whoami(ctx),
            "agent.list_trusted" => Ok(crate::mcp::handler::trust_snapshot()),
            "agent.trust" => self.agent_trust(params),
            "agent.untrust" => self.agent_untrust(params),
            // Cockpit — agent state per pane, hook ingestion, inbox.
            "agent.status" => self.cockpit_agent_status(params),
            "agent.signal" => self.cockpit_agent_signal(ctx, params),
            "cockpit.inbox" => self.cockpit_inbox(),
            // Cockpit — fleets and review.
            "fleet.launch" => self.fleet_launch(params),
            "fleet.list" => Ok(json!({ "fleets": crate::cockpit::review::overview()["fleets"] })),
            "fleet.clean" => self.fleet_clean(params),
            "fleet.retry" => self.fleet_retry(params),
            "review.list" => Ok(crate::cockpit::verification::enrich_overview(
                crate::cockpit::observability::enrich_overview(crate::cockpit::review::overview()),
            )),
            "review.diff" => self.review_diff(params),
            "review.verify" => self.review_verify(params),
            "review.rollback" => self.review_rollback(params),
            "review.merge" => self.review_merge(params),
            "review.discard" => self.review_discard(params),
            "ghost.debug" => self.ghost_debug(params),
            // Suggest API — agents propose text; the user decides
            // whether it reaches the PTY (Tab/Esc in the suggest UI).
            "session.suggest" => self.session_suggest(ctx, params),
            "session.suggest_status" => self.session_suggest_status(params),
            "session.suggest_cancel" => self.session_suggest_cancel(params),
            "session.suggest_list" => self.session_suggest_list(params),
            // Session management
            "session.list" => self.session_list(),
            "session.get" | "session.status" => self.session_get(params),
            "session.create" => self.session_create(params),
            "session.split" => self.session_split(params),
            "session.focus" => self.session_focus(params),
            "session.input" => self.session_input(params),
            "session.resize" => self.session_resize(params),
            "session.destroy" => self.session_destroy(params),
            "session.idle" => self.session_idle(params),
            "session.cwd" => self.session_cwd(params),
            "session.env" => self.session_env(params),
            "session.set_env" => self.session_set_env(params),
            "session.history" => self.session_history(params),
            "session.audit_log" => self.session_audit_log(params),
            // Exec
            "exec.run" => self.exec_run(params),
            "exec.send" => self.session_input(params),
            "exec.run_wait" => self.exec_run_wait(params),
            "exec.status" => self.exec_status(params),
            "exec.cancel" => self.exec_cancel(params),
            // Screen
            "screen.read" => self.screen_read(params),
            "screen.text" => self.screen_text(params),
            // Full scrollback + viewport as text. AI-friendly alternative to a
            // rendered "long screenshot" — for long terminal output you want
            // to hand off to an LLM, this is strictly better than a PNG
            // (parses natively, no OCR, no font fidelity loss). Pass
            // `escapes: true` to preserve ANSI styling, otherwise plain text.
            "screen.scrollback_text" => self.screen_scrollback_text(params),
            "screen.cursor" => self.screen_cursor(params),
            "screen.scroll" => self.screen_scroll(params),
            "screen.search" => self.screen_search(params),
            "screen.detect_errors" => self.screen_detect_errors(params),
            // Signal
            "signal.send" => self.signal_send(params),
            // Orchestrate
            "orchestrate.launch" => self.orchestrate_launch(params),
            "orchestrate.broadcast" => self.orchestrate_broadcast(params),
            "orchestrate.wait" => self.orchestrate_wait(params),
            // Proxy
            "proxy.status" => self.proxy_status(),
            "proxy.nodes" => self.proxy_nodes(),
            "proxy.switch" => self.proxy_switch(params),
            "proxy.speedtest" => self.proxy_speedtest(params),
            "proxy.configure" => self.proxy_configure(params),
            "proxy.disable" => self.proxy_disable(),
            "proxy.env" => self.proxy_env(),
            "proxy.rotation" => self.proxy_rotation(params),
            "proxy.set_nodes" => self.proxy_set_nodes(params),
            "proxy.clash_status" => self.proxy_clash_status(),
            "proxy.clash_select" => self.proxy_clash_select(params),
            "proxy.clash_set_controller" => self.proxy_clash_set_controller(params),
            // Workspace
            "workspace.save" => self.workspace_save(params),
            "workspace.restore" => self.workspace_restore(params),
            "workspace.list" => self.workspace_list(),
            // Capture
            "capture.screen" => self.capture_screen(params),
            "capture.window" => self.capture_window(params),
            "capture.select" => self.capture_select(),
            "capture.clipboard" => self.capture_clipboard(),
            "capture.scrollback" => self.capture_scrollback(params),
            "capture.window_scroll" => self.capture_window_scroll(params),
            // Upload to user-configured object storage. Credentials live in
            // ~/.unterm/upload.json (OSS / COS / Qiniu) and never leave the
            // local machine. Pairs with `capture.*` so an AI agent can
            // screenshot → upload → embed the URL without dragging files.
            "upload.file" => crate::mcp::upload::upload(params),
            // Policy
            "policy.set" => self.policy_set(params),
            "policy.check" => self.policy_check(params),
            // System
            "system.info" => self.system_info(),
            "system.launch_admin" => self.system_launch_admin(params),
            "server.info" => self.server_info(),
            "server.health" => self.server_health(),
            "server.capabilities" => self.server_capabilities(),
            // Single-call discovery surface for AI agents and the Web Settings
            // "Reference" tab: MCP methods + CLI subcommands + live keybindings
            // in one round trip. See `meta.rs` for the source of truth list.
            "meta.surface" => crate::mcp::meta::surface(params),
            // Multi-instance discovery (one Unterm process = one instance,
            // each with a NATO-phonetic name like "alpha", "bravo", ...)
            "instance.list" => self.instance_list(),
            "instance.info" => self.instance_info(),
            "instance.set_title" => self.instance_set_title(params),
            "instance.focus" => self.instance_focus(params),
            // Identity profiles: read-only surface for agents. Writes
            // (create / set-secret / delete) and `profile.spawn` (which
            // would have to open a new GUI window) are intentionally
            // CLI-only — that keeps the agent-facing surface narrow
            // and means no plausible MCP call can write to keychain.
            "profile.list" => self.profile_list(),
            "profile.current" => self.profile_current(),
            "profile.audit" => self.profile_audit(),
            "selftest.run" => self.selftest_run(params),
            // Session recording
            "session.recording_start" => self.session_recording_start(params),
            "session.recording_stop" => self.session_recording_stop(params),
            "session.recording_status" => self.session_recording_status(params),
            "session.recording_list" => self.session_recording_list(params),
            "session.recording_read" => self.session_recording_read(params),
            "session.recording_attach_trace" => self.session_recording_attach_trace(params),
            "session.export_markdown" => self.session_export_markdown(params),
            _ => Err(anyhow!("Unknown method: {}", method)),
        }
    }

    fn get_mux(&self) -> Result<Arc<Mux>> {
        Mux::try_get().ok_or_else(|| anyhow!("Mux not available"))
    }

    fn get_pane(&self, params: &Value) -> Result<Arc<dyn Pane>> {
        let mux = self.get_mux()?;
        // Accept numeric "id", string "session_id", and the documented
        // standard "pane_id" (P_PANE_ID in mcp_meta) — the cockpit
        // methods pass the latter.
        let id_val = params
            .get("id")
            .or_else(|| params.get("session_id"))
            .or_else(|| params.get("pane_id"));
        let id = match id_val {
            Some(v) if v.is_u64() => v.as_u64().unwrap() as usize,
            Some(v) if v.is_string() => v
                .as_str()
                .unwrap()
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid session_id: {}", v))?,
            _ => return Err(anyhow!("Missing 'id' or 'session_id' parameter")),
        };

        mux.get_pane(id)
            .ok_or_else(|| anyhow!("Session {} not found", id))
    }

    fn detect_shell(pane: &Arc<dyn Pane>) -> Value {
        let process_name = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .unwrap_or_default();

        let shell_type = if process_name.is_empty() {
            "unknown"
        } else {
            let name = process_name
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&process_name)
                .to_lowercase();
            if name.contains("pwsh") || name.contains("powershell") {
                "powershell"
            } else if name.contains("cmd") {
                "cmd"
            } else if name.contains("bash") {
                "bash"
            } else if name.contains("zsh") {
                "zsh"
            } else if name.contains("fish") {
                "fish"
            } else if name.contains("nu") {
                "nushell"
            } else {
                "unknown"
            }
        };

        let cwd = pane
            .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
            .map(|u| u.to_string());

        json!({
            "shell_type": shell_type,
            "process_name": process_name,
            "cwd": cwd,
        })
    }

    fn server_info(&self) -> Result<Value> {
        Ok(json!({
            "name": "Unterm MCP Server",
            "version": "2.0.0",
            "engine": "Unterm (WezTerm)",
            "protocol": "json-rpc-2.0",
        }))
    }

    fn server_health(&self) -> Result<Value> {
        let mux_available = Mux::try_get().is_some();
        let pane_count = Mux::try_get()
            .map(|mux| mux.iter_panes().len())
            .unwrap_or_default();
        let config = config::configuration();

        Ok(json!({
            "status": if mux_available { "ok" } else { "degraded" },
            "engine": "Unterm (WezTerm)",
            "mcp": {
                "bind": "127.0.0.1",
                "port": 19876,
                "auth": "token",
            },
            "mux": {
                "available": mux_available,
                "pane_count": pane_count,
            },
            "terminal": {
                "initial_cols": config.initial_cols,
                "initial_rows": config.initial_rows,
                "color_scheme": config.color_scheme,
                "term": config.term,
            },
        }))
    }

    fn server_capabilities(&self) -> Result<Value> {
        // Derive the namespace → methods map from meta::MCP_METHODS so this
        // listing can never drift from what dispatch actually accepts.
        // Kept around for back-compat with agents that already learned to
        // call `server.capabilities`; new agents should prefer `meta.surface`.
        let mut grouped: std::collections::BTreeMap<&'static str, Vec<&'static str>> =
            std::collections::BTreeMap::new();
        for m in crate::mcp::meta::MCP_METHODS {
            grouped.entry(m.namespace).or_default().push(m.name);
        }
        Ok(serde_json::to_value(&grouped)?)
    }

    /// Enumerate every live Unterm instance on this machine. An "instance"
    /// is a single Unterm process; each owns a NATO-phonetic name (alpha,
    /// bravo, …) recorded in `~/.unterm/instances/<name>.json`. Stale
    /// files (PID dead) are filtered out by the storage layer.
    ///
    /// AI agents use this when driving multiple Unterm windows: list
    /// instances, pick one by cwd / title / start order, then connect
    /// to that instance's `mcp_port` with its `auth_token` directly.
    fn instance_list(&self) -> Result<Value> {
        let instances = crate::server_info::list_live_instances();
        let arr: Vec<Value> = instances
            .into_iter()
            .map(|i| {
                json!({
                    "id": i.id,
                    "pid": i.pid,
                    "started_at": i.started_at,
                    "mcp_port": i.mcp_port,
                    "http_port": i.http_port,
                    "title": i.title,
                    "cwd": i.cwd,
                    "version": i.version,
                    "platform": i.platform,
                })
            })
            .collect();
        Ok(json!({ "instances": arr }))
    }

    /// Return *this* instance's own metadata (id, ports, title, cwd).
    /// Helpful for an agent to confirm which instance it's actually
    /// connected to vs. what `instance.list` says.
    fn instance_info(&self) -> Result<Value> {
        let i = crate::server_info::read_current();
        Ok(json!({
            "id": i.id,
            "pid": i.pid,
            "started_at": i.started_at,
            "mcp_port": i.mcp_port,
            "http_port": i.http_port,
            "auth_token": i.auth_token,
            "title": i.title,
            "cwd": i.cwd,
            "version": i.version,
            "platform": i.platform,
        }))
    }

    /// Pin a custom display title for this instance — overrides the
    /// auto-derived `Unterm — <name> — <project>` window title, and
    /// shows up alongside the NATO id in `instance.list` so peers
    /// can route to the right window. Pass `null` (or omit) to
    /// clear the override and resume auto-titling.
    fn instance_set_title(&self, params: &Value) -> Result<Value> {
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
        crate::server_info::set_title(title.clone()).context("failed to write instance title")?;
        Ok(json!({ "ok": true, "title": title }))
    }

    /// Bring this instance's window to the foreground.
    ///
    /// **Cross-instance focus is intentionally NOT supported here** — to
    /// focus a peer, connect to that peer's MCP port directly and call
    /// `instance.focus` there. Keeps the auth model simple (each instance
    /// only ever acts on itself with its own token).
    ///
    /// Runs on the GUI thread because the front-end/window registry is
    /// thread-local there. We focus the first known OS window for this
    /// instance; cross-instance focus is still modeled by connecting to that
    /// peer with `--instance <id>` and calling `instance.focus` there.
    fn instance_focus(&self, _params: &Value) -> Result<Value> {
        let (tx, rx) = std::sync::mpsc::channel();
        promise::spawn::spawn_into_main_thread(async move {
            let result = crate::frontend::try_front_end()
                .and_then(|fe| fe.gui_windows().into_iter().next())
                .map(|win| {
                    win.window.focus();
                    json!({
                        "ok": true,
                        "mux_window_id": win.mux_window_id,
                    })
                })
                .ok_or_else(|| anyhow!("no GUI window is registered for this instance"));
            tx.send(result).ok();
        })
        .detach();

        rx.recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| anyhow!("Timeout waiting for instance.focus"))?
    }

    /// `profile.list` — every identity profile on disk plus a hint at
    /// which one is the registered default. We deliberately do NOT
    /// expose `secrets` values here — only counts and metadata, so an
    /// over-eager agent can't drain the keychain through one call.
    fn profile_list(&self) -> Result<Value> {
        let registry = unterm_profile::ProfileRegistry::load().context("load profile registry")?;
        let default = registry.default_id().map(str::to_string);
        let profiles: Vec<Value> = registry
            .iter_ordered()
            .into_iter()
            .map(|(id, p)| {
                json!({
                    "id": id,
                    "display_name": p.display_name,
                    "accent_color": p.accent_color,
                    "description": p.description,
                    "secret_count": p.secrets.len(),
                    "expiration_count": p.expiration.len(),
                    "is_default": default.as_deref() == Some(id),
                })
            })
            .collect();
        Ok(json!({
            "profiles": profiles,
            "default": default,
        }))
    }

    /// `profile.current` — which profile is bound to THIS Unterm
    /// window (the one running the MCP server the caller connected
    /// to). Agents use this to know "what identity will my next
    /// command run under?" before triggering destructive ops.
    fn profile_current(&self) -> Result<Value> {
        let info = crate::server_info::read_current();
        Ok(json!({
            "instance": info.id,
            "profile": info.profile,
        }))
    }

    /// `profile.audit` — list secrets expiring within 7 days plus a
    /// healthy count for the rest. Lets agents proactively warn users
    /// (or surface a "rotate your GitHub PAT" reminder) without the
    /// user having to remember to check.
    fn profile_audit(&self) -> Result<Value> {
        let registry = unterm_profile::ProfileRegistry::load().context("load profile registry")?;
        let today = chrono::Local::now().date_naive();
        let mut warnings = Vec::new();
        let mut healthy = 0usize;
        for (id, p) in registry.iter_ordered() {
            for (env_name, date_str) in &p.expiration {
                let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else {
                    continue;
                };
                let days = (date - today).num_days();
                if days <= 7 {
                    warnings.push(json!({
                        "profile": id,
                        "display_name": p.display_name,
                        "env_name": env_name,
                        "expires_on": date_str,
                        "days_remaining": days,
                    }));
                } else {
                    healthy += 1;
                }
            }
        }
        Ok(json!({
            "warnings": warnings,
            "healthy_count": healthy,
        }))
    }

    fn session_list(&self) -> Result<Value> {
        let mux = self.get_mux()?;
        // iter_panes walks a HashMap, so impose a stable order: clients
        // (unterm-cli among them) default to picking a pane from this
        // list, and an unstable order turns "no --id given" into writes
        // landing in a random pane.
        let mut panes = mux.iter_panes();
        panes.sort_by_key(|pane| pane.pane_id());

        // The pane the user is actually looking at, so clients can
        // default to it instead of guessing.
        let active_pane_id = mux
            .iter_windows()
            .into_iter()
            .find_map(|wid| mux.get_active_tab_for_window(wid))
            .and_then(|tab| tab.get_active_pane())
            .map(|pane| pane.pane_id());

        let sessions: Vec<Value> = panes
            .iter()
            .map(|pane| {
                let dims = pane.get_dimensions();
                let cursor = pane.get_cursor_position();
                let is_dead = pane.is_dead();
                let shell = Self::detect_shell(pane);

                json!({
                    "id": pane.pane_id(),
                    "title": pane.get_title(),
                    "cols": dims.cols,
                    "rows": dims.viewport_rows,
                    "cursor": {
                        "x": cursor.x,
                        "y": cursor.y,
                        "visible": cursor.visibility == termwiz::surface::CursorVisibility::Visible,
                    },
                    "is_dead": is_dead,
                    "is_active": Some(pane.pane_id()) == active_pane_id,
                    "domain_id": pane.domain_id(),
                    "shell": shell,
                })
            })
            .collect();

        Ok(json!({ "sessions": sessions }))
    }

    fn session_get(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();
        let cursor = pane.get_cursor_position();
        let shell = Self::detect_shell(&pane);

        Ok(json!({
            "id": pane.pane_id(),
            "title": pane.get_title(),
            "cols": dims.cols,
            "rows": dims.viewport_rows,
            "scrollback_rows": dims.scrollback_rows,
            "cursor": {
                "x": cursor.x,
                "y": cursor.y,
                "visible": cursor.visibility == termwiz::surface::CursorVisibility::Visible,
            },
            "is_dead": pane.is_dead(),
            "domain_id": pane.domain_id(),
            "shell": shell,
        }))
    }

    /// `session.split` — split an existing pane and return the
    /// newly-created pane's id. Pairs with `session.create` (new
    /// tab) and `session.input` (write to pane) to make the full
    /// "AI drives a side-by-side pane" loop available from MCP.
    ///
    /// Params:
    ///   - `id` or `session_id`  (required) — pane to split
    ///   - `direction`           "right" (default) | "left" | "down" | "up"
    ///   - `size_percent`        u8 (0..=100), defaults to 50
    ///   - `cwd`                 optional working dir for new pane
    ///
    /// Returns the same shape as `session.create`.
    fn session_split(&self, params: &Value) -> Result<Value> {
        use config::keyassignment::SpawnTabDomain;
        use mux::domain::SplitSource;
        use mux::tab::{SplitDirection, SplitRequest, SplitSize};

        // Source pane: accept the same id/session_id duality as get_pane
        // so callers don't have to remember which method takes which.
        let src_pane_id = params
            .get("id")
            .or_else(|| params.get("session_id"))
            .and_then(|v| {
                v.as_u64()
                    .map(|n| n as usize)
                    .or_else(|| v.as_str().and_then(|s| s.parse::<usize>().ok()))
            })
            .ok_or_else(|| anyhow!("Missing 'id' / 'session_id' (source pane to split)"))?;

        // Take an owned String here so the value can cross the async
        // closure boundary below — &str borrowed from `params` would
        // be tied to the request's lifetime which doesn't outlive the
        // spawned future.
        let dir_str: String = params
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("right")
            .to_string();
        let (direction, target_is_second) = match dir_str.as_str() {
            "right" => (SplitDirection::Horizontal, true),
            "left" => (SplitDirection::Horizontal, false),
            "down" | "bottom" => (SplitDirection::Vertical, true),
            "up" | "top" => (SplitDirection::Vertical, false),
            other => {
                return Err(anyhow!(
                    "invalid direction {other:?} (use right|left|down|up)"
                ))
            }
        };

        let size_percent = params
            .get("size_percent")
            .and_then(|v| v.as_u64())
            .map(|n| n.min(100) as u8)
            .unwrap_or(50);

        let request = SplitRequest {
            direction,
            target_is_second,
            top_level: false,
            size: SplitSize::Percent(size_percent),
        };

        let command_dir = params.get("cwd").and_then(|v| v.as_str()).map(String::from);

        // Same two-level spawn dance as session.create so we get the
        // async split_pane future back into this sync context. 10s cap
        // is generous; split should complete in <100ms once Mux is up.
        let (tx, rx) = std::sync::mpsc::channel();
        promise::spawn::spawn_into_main_thread(async move {
            promise::spawn::spawn(async move {
                let result = async {
                    let mux = Mux::get();
                    let (pane, _size) = mux
                        .split_pane(
                            src_pane_id,
                            request,
                            SplitSource::Spawn {
                                command: None,
                                command_dir,
                            },
                            SpawnTabDomain::DefaultDomain,
                        )
                        .await
                        .context("split_pane")?;
                    let dims = pane.get_dimensions();
                    let pid = pane.pane_id();
                    Ok::<Value, anyhow::Error>(json!({
                        "id": pid,
                        "session_id": pid.to_string(),
                        "title": pane.get_title(),
                        "cols": dims.cols,
                        "rows": dims.viewport_rows,
                        "direction": dir_str,
                        "src_pane_id": src_pane_id,
                        "size_percent": size_percent,
                    }))
                }
                .await;
                tx.send(result).ok();
            })
            .detach();
        })
        .detach();

        rx.recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|_| anyhow!("Timeout waiting for session.split"))?
    }

    /// `session.focus` — make this pane the active one in its tab
    /// (and bring its tab to the front of its window). Pairs with
    /// `session.split` so an agent can split-then-focus the new
    /// pane to make the side-by-side hand-off visible to the user
    /// immediately, rather than the new pane being a hidden split.
    ///
    /// Params: `id` or `session_id` (required).
    /// Returns: `{ ok: true, id: <focused-pane-id> }`.
    fn session_focus(&self, params: &Value) -> Result<Value> {
        let pane_id = params
            .get("id")
            .or_else(|| params.get("session_id"))
            .and_then(|v| {
                v.as_u64()
                    .map(|n| n as usize)
                    .or_else(|| v.as_str().and_then(|s| s.parse::<usize>().ok()))
            })
            .ok_or_else(|| anyhow!("Missing 'id' / 'session_id'"))?;

        // focus_pane_and_containing_tab does the whole work: walks up
        // to find the owning tab + window, sets the tab's active pane,
        // and activates the tab inside the window. No need for an
        // async hop — it's a synchronous Mux operation.
        let mux = self.get_mux()?;
        mux.focus_pane_and_containing_tab(pane_id)
            .with_context(|| format!("focus pane {pane_id}"))?;
        Ok(json!({ "ok": true, "id": pane_id }))
    }

    fn session_create(&self, params: &Value) -> Result<Value> {
        let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(120) as usize;
        let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
        let command_dir = params
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let profile = params
            .get("profile")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let mut cmd_builder = command.as_deref().map(shell_command_builder);
        if let Some(cwd) = command_dir.as_deref() {
            if let Some(builder) = cmd_builder.as_mut() {
                builder.cwd(cwd);
            }
        }
        let resolved_profile = if let Some(profile) = profile.as_deref() {
            Some(apply_profile_env_to_builder(&mut cmd_builder, profile)?)
        } else {
            None
        };

        let size = wezterm_term::TerminalSize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };

        // Use a channel to get the async result back to this sync context.
        // Two-level spawn pattern (same as wezterm-mux-server-impl) because
        // domain.spawn() returns non-Send futures.
        let (tx, rx) = std::sync::mpsc::channel();

        promise::spawn::spawn_into_main_thread(async move {
            promise::spawn::spawn(async move {
                let result = async {
                    let mux = Mux::get();
                    let window_id = mux
                        .iter_windows()
                        .into_iter()
                        .next()
                        .ok_or_else(|| anyhow!("No windows available"))?;

                    let (_tab, pane, _wid) = mux
                        .spawn_tab_or_window(
                            Some(window_id),
                            SpawnTabDomain::DefaultDomain,
                            cmd_builder,
                            command_dir,
                            size,
                            None,
                            String::new(),
                            None,
                        )
                        .await
                        .context("spawn_tab_or_window")?;

                    let dims = pane.get_dimensions();
                    let pid = pane.pane_id();
                    Ok::<Value, anyhow::Error>(json!({
                        "id": pid,
                        "session_id": pid.to_string(),
                        "title": pane.get_title(),
                        "cols": dims.cols,
                        "rows": dims.viewport_rows,
                        "profile": resolved_profile,
                        "command": command,
                    }))
                }
                .await;
                tx.send(result).ok();
            })
            .detach();
        })
        .detach();

        // Wait for the spawn to complete (up to 10 seconds)
        let result = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|_| anyhow!("Timeout waiting for session creation"))?;

        result
    }

    fn session_input(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let input = params
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'input' parameter"))?;

        // Gate the write on a user confirmation banner if policy
        // demands it. `Allow` continues to the audit + write below;
        // `Block` returns -32004 to the agent.
        match self.gate_pty_write("session.input", &pane, input)? {
            GateOutcome::Allow => {}
            GateOutcome::Block => {
                return Err(anyhow!("user denied"));
            }
        }

        // PTY 字节流一旦写下去就和用户手敲不可区分，必须留下审计痕迹。
        self.audit(
            "session.input",
            Some(&pane.pane_id().to_string()),
            &input_preview(input),
        );
        pane.writer().write_all(input.as_bytes())?;
        Ok(json!({"status": "ok"}))
    }

    /// Decide whether a PTY-writing MCP call should proceed and, when
    /// required, park the worker on a confirmation banner.
    ///
    /// Returns `GateOutcome::Allow` when the call may proceed (either
    /// because policy is `Never`, the agent is on the trusted list,
    /// the user already picked "always allow" this session, or
    /// because they just clicked Allow on the banner). Returns
    /// `GateOutcome::Block` when the user denied (or the banner
    /// timed out).
    fn gate_pty_write(
        &self,
        method: &str,
        pane: &Arc<dyn Pane>,
        input: &str,
    ) -> Result<GateOutcome> {
        let cfg = config::configuration();
        let agent = current_agent_label();
        let preview = input_preview(input);

        // 1) Configured `Never` → no banner ever.
        // 2) Agent name on the static trust list → skip.
        // 3) User previously chose AlwaysAllow this session → skip.
        let policy = cfg.mcp_input_confirmation;
        let trusted_static = cfg.mcp_trusted_agents.iter().any(|n| n == &agent);
        let already_confirmed = {
            let state = mcp_state().lock();
            state.confirmed_agents.contains(&agent)
        };
        let needs_banner = if trusted_static || already_confirmed {
            false
        } else {
            match policy {
                config::McpInputConfirmation::Never => false,
                config::McpInputConfirmation::Always => true,
                config::McpInputConfirmation::FirstTimePerAgent => {
                    // The first PTY write by this agent triggers a
                    // banner; once allowed, that decision sticks for
                    // the session via `confirmed_agents`. So this
                    // arm is only reached on the *very first* write.
                    let state = mcp_state().lock();
                    !state.agents_with_input_history.contains(&agent)
                }
            }
        };
        if !needs_banner {
            return Ok(GateOutcome::Allow);
        }

        // Park on a confirmation banner. Capacity is intentionally
        // small (1 slot) — the worker thread blocks until the GUI
        // resolves it.
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let id = {
            let mut state = mcp_state().lock();
            state.confirmation_seq = state.confirmation_seq.saturating_add(1);
            let id = state.confirmation_seq;
            state.pending_confirmations.push(PendingConfirmation {
                id,
                agent: agent.clone(),
                input_preview: preview.clone(),
                pane_id: pane.pane_id() as u64,
                method: method.to_string(),
                requested_at: chrono::Local::now().to_rfc3339(),
                responder: tx,
            });
            id
        };

        let timeout_ms = cfg.mcp_confirmation_timeout_ms.max(1000);
        let decision = rx.recv_timeout(std::time::Duration::from_millis(timeout_ms));
        match decision {
            Ok(ConfirmationDecision::Allow) => {
                self.audit(
                    "mcp.confirm.allow",
                    Some(&pane.pane_id().to_string()),
                    &format!("agent={} {}", agent, preview),
                );
                Ok(GateOutcome::Allow)
            }
            Ok(ConfirmationDecision::AlwaysAllow) => {
                self.audit(
                    "mcp.confirm.always_allow",
                    Some(&pane.pane_id().to_string()),
                    &format!("agent={} {}", agent, preview),
                );
                Ok(GateOutcome::Allow)
            }
            Ok(ConfirmationDecision::Block) => {
                self.audit(
                    "mcp.confirm.block",
                    Some(&pane.pane_id().to_string()),
                    &format!("agent={} {}", agent, preview),
                );
                Ok(GateOutcome::Block)
            }
            Err(_) => {
                // Timeout: clean up the still-queued banner (the
                // GUI may not have rendered it yet) and treat as
                // block. The receiver going out of scope makes
                // future GUI `send`s no-op, which is fine.
                {
                    let mut state = mcp_state().lock();
                    state.pending_confirmations.retain(|p| p.id != id);
                }
                self.audit(
                    "mcp.confirm.timeout",
                    Some(&pane.pane_id().to_string()),
                    &format!("agent={} {}", agent, preview),
                );
                Ok(GateOutcome::Block)
            }
        }
    }

    fn session_resize(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let cols = params
            .get("cols")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing 'cols'"))? as usize;
        let rows = params
            .get("rows")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing 'rows'"))? as usize;

        // A pane that is tiled inside a GUI window gets its geometry from
        // the window size and split layout; resizing only the PTY leaves
        // the model at one size and the visible grid at another (content
        // clips / wraps wrong until the next window resize resnaps it).
        // Reject instead of silently desyncing.
        let mux = self.get_mux()?;
        let pane_id = pane.pane_id();
        let in_gui_layout = mux.iter_windows().into_iter().any(|wid| {
            mux.get_window(wid)
                .map(|window| window.iter().any(|tab| tab.contains_pane(pane_id)))
                .unwrap_or(false)
        });
        if in_gui_layout {
            return Err(anyhow!(
                "Session {} is laid out by the GUI window; its size follows \
                 the window and splits. Resize the window or adjust the \
                 split instead.",
                pane_id
            ));
        }

        let size = wezterm_term::TerminalSize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };
        pane.resize(size)?;
        Ok(json!({"status": "ok"}))
    }

    fn session_destroy(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        self.audit(
            "session.destroy",
            Some(&pane.pane_id().to_string()),
            "destroy",
        );
        pane.kill();
        Ok(json!({"status": "ok", "destroyed": true}))
    }

    fn session_idle(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        // Heuristic: check if foreground process is the shell itself
        let fg = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .unwrap_or_default();
        let name = fg.rsplit(['/', '\\']).next().unwrap_or("").to_lowercase();
        let is_shell = name.contains("powershell")
            || name.contains("pwsh")
            || name.contains("cmd")
            || name.contains("bash")
            || name.contains("zsh")
            || name.contains("fish")
            || name.contains("nu");
        Ok(json!({"idle": is_shell, "foreground_process": fg}))
    }

    fn session_cwd(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let cwd = pane
            .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
            .map(|u| u.to_string())
            .unwrap_or_default();
        Ok(json!({"cwd": cwd}))
    }

    fn session_env(&self, _params: &Value) -> Result<Value> {
        // WezTerm doesn't expose per-pane env vars directly
        Ok(
            json!({"value": null, "message": "Environment variable reading not supported in WezTerm mode"}),
        )
    }

    fn session_set_env(&self, _params: &Value) -> Result<Value> {
        Ok(
            json!({"status": "ok", "message": "Environment variable setting not supported in WezTerm mode"}),
        )
    }

    fn session_history(&self, params: &Value) -> Result<Value> {
        // Return scrollback as "history"
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        let end = dims.physical_top;
        let start = (end - limit as isize).max(0);
        let (_first, lines) = pane.get_lines(start..end);

        let entries: Vec<Value> = lines
            .iter()
            .map(|line| {
                let text = line.as_str().trim_end().to_string();
                json!({"text": text})
            })
            .filter(|v| !v["text"].as_str().unwrap_or("").is_empty())
            .collect();

        Ok(json!({"entries": entries, "count": entries.len()}))
    }

    fn session_audit_log(&self, params: &Value) -> Result<Value> {
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let session_filter = params.get("session_id").and_then(|v| v.as_str());
        let state = mcp_state().lock();
        let entries: Vec<_> = state
            .audit_log
            .iter()
            .rev()
            .filter(|e| session_filter.map_or(true, |sid| e.session_id.as_deref() == Some(sid)))
            .take(limit)
            .cloned()
            .collect();
        Ok(json!(entries))
    }

    // --- Agent identity ---

    /// Record the calling connection's claimed identity. Self-asserted;
    /// we trust the agent label only to *group* activity, never to grant
    /// privileges — the auth token is still what gates access.
    fn agent_identify(&self, ctx: &ConnectionContext, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'name' parameter"))?
            .trim();
        if name.is_empty() {
            return Err(anyhow!("'name' must be non-empty"));
        }
        if name.len() > 64 {
            return Err(anyhow!("'name' must be ≤ 64 chars"));
        }
        let version = params
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let capabilities: Vec<String> = params
            .get("capabilities")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let now = chrono::Local::now().to_rfc3339();
        let identity = AgentIdentity {
            name: name.to_string(),
            version: version.clone(),
            capabilities: capabilities.clone(),
            peer_addr: ctx.peer_addr.clone(),
            identified_at: now.clone(),
        };

        let first_time = {
            let mut state = mcp_state().lock();
            state
                .agents_by_connection
                .insert(ctx.conn_id, identity.clone());
            // `entry().or_insert` so the timestamp records *first ever*
            // sighting, not the latest.
            let entry = state.known_agents.entry(name.to_string());
            let is_new = matches!(entry, std::collections::hash_map::Entry::Vacant(_));
            entry.or_insert_with(|| now.clone());
            is_new
        };

        // Audit the identification itself — useful for forensics
        // ("when did claude-code first connect?").
        self.audit(
            "agent.identify",
            None,
            &format!(
                "name={} version={} first_time={}",
                name,
                version.as_deref().unwrap_or("-"),
                first_time
            ),
        );

        Ok(json!({
            "status": "ok",
            "name": name,
            "first_time": first_time,
            "identified_at": now,
        }))
    }

    /// Echo back the connection's current identity (or "anonymous" if
    /// `agent.identify` was never called).
    fn agent_whoami(&self, ctx: &ConnectionContext) -> Result<Value> {
        let state = mcp_state().lock();
        match state.agents_by_connection.get(&ctx.conn_id) {
            Some(identity) => Ok(json!(identity)),
            None => Ok(json!({
                "name": "anonymous",
                "peer_addr": ctx.peer_addr,
            })),
        }
    }

    /// `agent.trust` — programmatically promote an agent to the
    /// persistent trust list. Equivalent to the user pressing Alt+A
    /// on a confirmation banner. Intended for the Web Settings UI
    /// (where the click happens server-side via HTTP) more than for
    /// random agents trusting themselves — but we don't ACL-gate it
    /// here because the MCP token is already an auth boundary;
    /// anyone with the token can write to a pane anyway, so trust
    /// management isn't a stronger capability.
    fn agent_trust(&self, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Missing or empty 'name'"))?;
        let added = grant_trust(name);
        Ok(json!({ "ok": true, "name": name, "added": added }))
    }

    /// `agent.untrust` — remove an agent from the persistent trust
    /// list. Subsequent writes from that agent will trigger the
    /// confirmation banner again. Does NOT touch the static lua
    /// `mcp_trusted_agents` config (the user has to edit the file
    /// for that — surfaced in the Web Settings panel UI).
    fn agent_untrust(&self, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Missing or empty 'name'"))?;
        let was = revoke_trust(name);
        Ok(json!({ "ok": true, "name": name, "removed": was }))
    }

    // --- Cockpit: agent state per pane ---

    fn cockpit_status_json(s: &crate::cockpit::PaneAgentStatus) -> Value {
        json!({
            "pane_id": s.pane_id,
            "agent": s.agent,
            "state": s.state.as_str(),
            "for_secs": s.since.elapsed().as_secs(),
            "task_hint": s.task_hint,
            "last_signal": s.last_signal,
            "fleet_id": s.fleet_id,
        })
    }

    /// `agent.status` — the cockpit's view of which agent runs in which
    /// pane and what it is doing. With `pane_id`, a single entry (or
    /// `null`); without, every tracked pane in Inbox order.
    fn cockpit_agent_status(&self, params: &Value) -> Result<Value> {
        if !config::configuration().cockpit_enabled {
            return Ok(json!({ "enabled": false, "agents": [] }));
        }
        let explicit_pane = params.get("pane_id").or_else(|| params.get("session_id"));
        if explicit_pane.is_some() {
            let pane = self.get_pane(params)?;
            let status = crate::cockpit::status_for_pane(pane.pane_id() as u64)
                .map(|s| Self::cockpit_status_json(&s));
            return Ok(json!({ "enabled": true, "agent": status }));
        }
        let agents: Vec<Value> = crate::cockpit::snapshot()
            .iter()
            .map(Self::cockpit_status_json)
            .collect();
        Ok(json!({ "enabled": true, "agents": agents }))
    }

    /// `agent.signal` — official hook ingestion (Claude Code hooks,
    /// Codex notify, Aider notifications-command). The strongest state
    /// signal; see cockpit::status for precedence rules.
    fn cockpit_agent_signal(&self, ctx: &ConnectionContext, params: &Value) -> Result<Value> {
        let event = params
            .get("event")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'event' (working|waiting|done|idle)"))?;
        // Hooks pass $WEZTERM_PANE; a bare CLI call falls back to the
        // pane the user is looking at.
        let pane = self.get_pane(params).or_else(|_| {
            let mux = self.get_mux()?;
            mux.iter_windows()
                .into_iter()
                .find_map(|wid| mux.get_active_tab_for_window(wid))
                .and_then(|tab| tab.get_active_pane())
                .ok_or_else(|| anyhow!("no active pane"))
        })?;
        let pane_id = pane.pane_id() as u64;
        let agent = params
            .get("agent")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                mcp_state()
                    .lock()
                    .agents_by_connection
                    .get(&ctx.conn_id)
                    .map(|a| a.name.clone())
            })
            .unwrap_or_else(|| "agent".to_string());
        if !crate::cockpit::on_hook_signal(pane_id, &agent, event) {
            anyhow::bail!("Invalid 'event' {event:?}: expected working|waiting|done|idle");
        }
        self.audit(
            "agent.signal",
            Some(&pane_id.to_string()),
            &format!("agent={agent} event={event}"),
        );
        Ok(json!({ "ok": true, "pane_id": pane_id, "agent": agent, "event": event }))
    }

    /// `cockpit.inbox` — every tracked agent joined with its tab/window
    /// location so a client can jump straight to it.
    fn cockpit_inbox(&self) -> Result<Value> {
        if !config::configuration().cockpit_enabled {
            return Ok(json!({ "enabled": false, "items": [] }));
        }
        let mux = self.get_mux()?;
        let items: Vec<Value> = crate::cockpit::snapshot()
            .iter()
            .map(|s| {
                let mut v = Self::cockpit_status_json(s);
                if let Some(pane) = mux.get_pane(s.pane_id as mux::pane::PaneId) {
                    v["pane_title"] = json!(pane.get_title());
                }
                if let Some((_domain, window_id, tab_id)) =
                    mux.resolve_pane_id(s.pane_id as mux::pane::PaneId)
                {
                    v["tab_id"] = json!(tab_id);
                    v["window_id"] = json!(window_id);
                }
                v
            })
            .collect();
        Ok(json!({ "enabled": true, "items": items }))
    }

    // --- Cockpit: fleet + review ---

    /// `fleet.launch` — one task × N agents × N git worktrees. Blocking
    /// (worktree creation + tab spawn), which is fine on the MCP thread.
    fn fleet_launch(&self, params: &Value) -> Result<Value> {
        let cwd = params
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(std::path::PathBuf::from)
            .or_else(|| {
                self.get_pane(&json!({})).ok().and_then(|p| {
                    p.get_current_working_dir(mux::pane::CachePolicy::AllowStale)
                        .and_then(|u| u.to_file_path().ok())
                })
            })
            .ok_or_else(|| anyhow!("Missing 'cwd' and no active pane to take it from"))?;
        let task = params
            .get("task")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow!("Missing 'task'"))?;
        let agents: Vec<String> = params
            .get("agents")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
            .ok_or_else(|| anyhow!("Missing 'agents' (e.g. [\"claude\",\"claude\"])"))?;
        let fleet = crate::cockpit::fleet::launch(&cwd, task, &agents)?;
        self.audit(
            "fleet.launch",
            None,
            &format!("id={} members={}", fleet.id, fleet.members.len()),
        );
        Ok(serde_json::to_value(&fleet)?)
    }

    fn fleet_clean(&self, params: &Value) -> Result<Value> {
        let id = params
            .get("id")
            .or_else(|| params.get("fleet_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'id'"))?;
        let force = params
            .get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        crate::cockpit::fleet::clean(id, force)?;
        self.audit("fleet.clean", None, &format!("id={id} force={force}"));
        Ok(json!({ "ok": true, "id": id }))
    }

    fn fleet_retry(&self, params: &Value) -> Result<Value> {
        let fleet_id = params
            .get("fleet_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'fleet_id'"))?;
        let member = params
            .get("member")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'member'"))?;
        let retried = crate::cockpit::fleet::retry_member(fleet_id, member)?;
        self.audit("fleet.retry", None, &format!("fleet={fleet_id} member={member}"));
        Ok(serde_json::to_value(retried)?)
    }

    fn review_verify(&self, params: &Value) -> Result<Value> {
        let fleet_id = params
            .get("fleet_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'fleet_id'"))?;
        let member = params
            .get("member")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'member'"))?;
        let command = params.get("command").and_then(|v| v.as_str());
        let timeout = params.get("timeout_secs").and_then(|v| v.as_u64());
        let record = crate::cockpit::verification::verify_member(
            fleet_id, member, command, timeout,
        )?;
        self.audit(
            "review.verify",
            None,
            &format!("fleet={fleet_id} member={member} command={}", record.command),
        );
        Ok(serde_json::to_value(record)?)
    }

    fn review_diff(&self, params: &Value) -> Result<Value> {
        // Either (fleet_id, member) or (repo, from).
        if let Some(fleet_id) = params.get("fleet_id").and_then(|v| v.as_str()) {
            let member = params
                .get("member")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'member'"))?;
            let fleet = crate::cockpit::fleet::get(fleet_id)
                .ok_or_else(|| anyhow!("no fleet {fleet_id:?}"))?;
            let m = crate::cockpit::fleet::resolve_member(&fleet, member)?;
            return crate::cockpit::review::diff(&m.worktree, &m.checkpoint);
        }
        let repo = params
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'repo' (or 'fleet_id'+'member')"))?;
        let from = params
            .get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'from' (checkpoint sha)"))?;
        crate::cockpit::review::diff(std::path::Path::new(repo), from)
    }

    fn review_rollback(&self, params: &Value) -> Result<Value> {
        let repo = params
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'repo'"))?;
        let sha = params
            .get("sha")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'sha'"))?;
        crate::cockpit::review::rollback(std::path::Path::new(repo), sha)?;
        self.audit("review.rollback", None, &format!("repo={repo} sha={sha}"));
        Ok(json!({ "ok": true, "repo": repo, "sha": sha }))
    }

    fn review_merge(&self, params: &Value) -> Result<Value> {
        let fleet_id = params
            .get("fleet_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'fleet_id'"))?;
        let member = params
            .get("member")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'member'"))?;
        let force = params.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
        let out = crate::cockpit::review::merge_member_with_policy(fleet_id, member, force)?;
        self.audit(
            "review.merge",
            None,
            &format!("fleet={fleet_id} member={member} force={force}"),
        );
        Ok(out)
    }

    fn review_discard(&self, params: &Value) -> Result<Value> {
        let fleet_id = params
            .get("fleet_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'fleet_id'"))?;
        let member = params
            .get("member")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'member'"))?;
        let out = crate::cockpit::review::discard_member(fleet_id, member)?;
        self.audit(
            "review.discard",
            None,
            &format!("fleet={fleet_id} member={member}"),
        );
        Ok(out)
    }

    /// Called by the TCP server when a client connection drops so
    /// `agents_by_connection` doesn't grow unboundedly. `known_agents`
    /// is intentionally kept — it's "have we *ever* seen this name?",
    /// which outlives any single connection.
    pub fn drop_connection(&self, conn_id: u64) {
        let mut state = mcp_state().lock();
        state.agents_by_connection.remove(&conn_id);
    }

    // --- Ghost text debug ---

    /// Read the ghost-text registry's current view of a pane.
    /// Read-only — never mutates state. Lets a remote debugger see
    /// whether the buffer is growing as the user types, whether
    /// commits are landing, and what (if anything) the predictor is
    /// proposing.
    fn ghost_debug(&self, params: &Value) -> Result<Value> {
        let pane_id = params
            .get("id")
            .or_else(|| params.get("pane_id"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing 'id' / 'pane_id' parameter"))?;
        match crate::ghost_text::debug_snapshot(pane_id) {
            Some(snap) => Ok(serde_json::to_value(snap)?),
            None => Ok(json!({"empty": true, "pane_id": pane_id})),
        }
    }

    // --- Suggest API ---

    /// Post a non-PTY-writing suggestion. The text is queued for the
    /// user to accept (Tab in the suggest UI) or dismiss (Esc); the
    /// MCP client never gets to inject keystrokes directly when it
    /// uses this method. Returns a suggestion id the caller can use
    /// for status / cancel.
    fn session_suggest(&self, ctx: &ConnectionContext, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let pane_id = pane.pane_id() as u64;

        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'text' parameter"))?;
        if text.is_empty() {
            return Err(anyhow!("'text' must be non-empty"));
        }
        if text.len() > 4096 {
            // A *suggestion* longer than 4KB is almost certainly a bug
            // — refuse it so a misbehaving agent can't OOM the queue.
            return Err(anyhow!("'text' too large ({} > 4096 bytes)", text.len()));
        }

        let rationale = params
            .get("rationale")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let ttl_ms = params
            .get("ttl_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| config::configuration().mcp_suggest_default_ttl_ms);
        let source: SuggestionSource = params
            .get("source")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let now = chrono::Local::now().to_rfc3339();
        let agent_label = current_agent_label();

        let id = {
            let mut state = mcp_state().lock();
            state.suggestion_seq = state.suggestion_seq.saturating_add(1);
            format!(
                "sg_{}_{}",
                chrono::Utc::now().timestamp_millis(),
                state.suggestion_seq
            )
        };

        let suggestion = Suggestion {
            id: id.clone(),
            pane_id,
            text: text.to_string(),
            rationale: rationale.clone(),
            source: source.clone(),
            created_at: now.clone(),
            ttl_ms,
            state: SuggestionState::Pending,
            posted_by_conn: ctx.conn_id,
            posted_by_agent: agent_label.clone(),
        };

        let suggest_max = config::configuration().mcp_suggest_queue_capacity.max(8);
        {
            let mut state = mcp_state().lock();
            state.suggestions.insert(id.clone(), suggestion);
            state.suggestion_order.push(id.clone());
            if state.suggestion_order.len() > suggest_max {
                let drop_n = (suggest_max / 8).max(1);
                for old_id in state.suggestion_order.drain(..drop_n).collect::<Vec<_>>() {
                    state.suggestions.remove(&old_id);
                }
            }
        }

        self.audit(
            "session.suggest",
            Some(&pane_id.to_string()),
            &format!("id={} {}", id, input_preview(text)),
        );

        Ok(json!({
            "suggestion_id": id,
            "status": "queued",
        }))
    }

    fn session_suggest_status(&self, params: &Value) -> Result<Value> {
        let id = params
            .get("suggestion_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'suggestion_id' parameter"))?;
        let state = mcp_state().lock();
        let suggestion = state
            .suggestions
            .get(id)
            .ok_or_else(|| anyhow!("Unknown suggestion_id: {}", id))?;
        Ok(json!(suggestion))
    }

    fn session_suggest_cancel(&self, params: &Value) -> Result<Value> {
        let id = params
            .get("suggestion_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'suggestion_id' parameter"))?
            .to_string();
        let mut state = mcp_state().lock();
        let suggestion = state
            .suggestions
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Unknown suggestion_id: {}", id))?;
        if matches!(suggestion.state, SuggestionState::Pending) {
            suggestion.state = SuggestionState::Cancelled {
                at: chrono::Local::now().to_rfc3339(),
            };
        }
        Ok(json!({"status": "ok"}))
    }

    fn session_suggest_list(&self, params: &Value) -> Result<Value> {
        let pane_filter = params.get("pane_id").and_then(|v| v.as_u64());
        let state = mcp_state().lock();
        let mut out: Vec<&Suggestion> = state
            .suggestions
            .values()
            .filter(|s| pane_filter.map_or(true, |p| s.pane_id == p))
            .filter(|s| matches!(s.state, SuggestionState::Pending))
            .collect();
        out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(json!(out))
    }

    fn audit(&self, method: &str, session_id: Option<&str>, detail: &str) {
        let entry = AuditEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            method: method.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            detail: detail.to_string(),
            allowed: true,
            agent: current_agent_label(),
        };
        let audit_max = config::configuration().mcp_audit_log_capacity.max(16);
        let mut state = mcp_state().lock();
        state.audit_log.push(entry);
        // Cap the in-memory log so a chatty agent can't OOM us. Drop the
        // oldest 10% in one shot so we amortize the shift cost instead of
        // re-shifting every push.
        if state.audit_log.len() > audit_max {
            let drop = audit_max / 10;
            state.audit_log.drain(..drop);
        }
        // Bump the activity counter so the status bar chip can surface
        // "AI just wrote to a pane" without scanning the whole audit
        // log. Only PTY-write methods qualify — read-only methods like
        // screen.read or session.list shouldn't make the chip flash.
        if matches!(
            method,
            "session.input" | "exec.send" | "exec.run" | "exec.run_wait"
        ) {
            // Attribute the pane to the writing agent for the left tab
            // bar's "agent · dir" subtitle.
            if let Some(pane_id) = session_id.and_then(|s| s.parse::<u64>().ok()) {
                let agent = entry_agent_from_last_audit(&state);
                state
                    .pane_agents
                    .insert(pane_id, (agent, std::time::Instant::now()));
            }
        }
        if method == "session.input" || method == "exec.send" {
            state.input_event_count = state.input_event_count.saturating_add(1);
            state.last_input_at = Some(std::time::Instant::now());
            // Stamp "first PTY write from this agent" — the soft
            // alternative to the (deferred) blocking confirmation
            // banner. A reader scanning the audit log sees a clear
            // signal when a new agent starts driving a terminal.
            let agent = entry_agent_from_last_audit(&state);
            if state.agents_with_input_history.insert(agent.clone()) {
                log::warn!(
                    "MCP: first PTY write by agent {agent:?} via {method} — review session.audit_log",
                );
                if let Some(last) = state.audit_log.last_mut() {
                    last.detail.push_str(" first_input=true");
                }
            }
        }
    }

    // --- Exec methods ---

    fn exec_run(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command'"))?;

        // Policy check
        if let Err(e) = self.check_policy_internal(command) {
            return Err(e);
        }

        self.audit("exec.run", Some(&pane.pane_id().to_string()), command);

        // Send command with newline
        let input = format!("{}\r", command);
        pane.writer().write_all(input.as_bytes())?;
        Ok(json!({"sent": true}))
    }

    fn exec_run_wait(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command'"))?;
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000);

        if let Err(e) = self.check_policy_internal(command) {
            return Err(e);
        }

        self.audit("exec.run_wait", Some(&pane.pane_id().to_string()), command);

        let marker = format!("__UNTERM_DONE_{}__", uuid::Uuid::new_v4().simple());
        let shell = Self::detect_shell(&pane);
        let shell_type = shell["shell_type"].as_str().unwrap_or("unknown");
        let wait_command = wait_wrapped_command(command, shell_type, &marker);

        // Capture screen before
        let before_text = self.read_pane_text(&pane);

        // Send command
        let input = format!("{}\r", wait_command);
        pane.writer().write_all(input.as_bytes())?;

        // Poll until the injected sentinel is rendered. This gives CLI/MCP
        // automation a deterministic completion condition across shells.
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            std::thread::sleep(std::time::Duration::from_millis(200));

            let current_text = self.read_pane_text(&pane);
            if current_text.contains(&marker) {
                std::thread::sleep(std::time::Duration::from_millis(200));
                let final_text = self.read_pane_text(&pane);
                let output = extract_wait_output(&before_text, &final_text, command, &marker);
                return Ok(json!({
                    "output": output,
                    "exit_status": "completed",
                    "timed_out": false,
                    "marker": marker,
                }));
            }

            if start.elapsed() > timeout {
                let current_text = self.read_pane_text(&pane);
                let output = extract_wait_output(&before_text, &current_text, command, &marker);
                return Ok(json!({
                    "output": output,
                    "exit_status": "timeout",
                    "timed_out": true,
                    "marker": marker,
                }));
            }
        }
    }

    fn exec_status(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let fg = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .unwrap_or_default();
        let name = fg.rsplit(['/', '\\']).next().unwrap_or("").to_lowercase();
        let is_shell = name.contains("powershell")
            || name.contains("pwsh")
            || name.contains("cmd")
            || name.contains("bash")
            || name.contains("zsh")
            || name.contains("fish");
        let status = if is_shell { "idle" } else { "running" };
        Ok(json!({"status": status, "foreground_process": fg}))
    }

    fn exec_cancel(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        self.audit("exec.cancel", Some(&pane.pane_id().to_string()), "Ctrl+C");
        // Send Ctrl+C
        pane.writer().write_all(b"\x03")?;
        Ok(json!({"cancelled": true}))
    }

    // --- Signal ---

    fn signal_send(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let signal = params
            .get("signal")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'signal'"))?;

        self.audit("signal.send", Some(&pane.pane_id().to_string()), signal);

        match signal.to_uppercase().as_str() {
            "SIGINT" | "INT" => pane.writer().write_all(b"\x03")?,
            "SIGTSTP" | "TSTP" => pane.writer().write_all(b"\x1a")?,
            "SIGQUIT" | "QUIT" => pane.writer().write_all(b"\x1c")?,
            "EOF" => pane.writer().write_all(b"\x04")?,
            _ => return Err(anyhow!("Unsupported signal: {}", signal)),
        }
        Ok(json!({"sent": true, "signal": signal}))
    }

    // --- Screen extensions ---

    fn screen_scroll(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as isize;
        let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(100) as isize;

        let start = offset;
        let end = offset + count;
        let (_first, lines) = pane.get_lines(start..end);

        let text_lines: Vec<String> = lines
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect();

        Ok(json!({"lines": text_lines, "offset": offset, "count": text_lines.len()}))
    }

    /// `screen.search` — find `pattern` (case-sensitive substring) in the
    /// pane's scrollback + viewport.
    ///
    /// Params:
    /// - `pattern` (string, required)
    /// - `max_results` (int, default 50)
    /// - `goto` (bool, default false): scroll the GUI viewport so the first
    ///   match is visible — "search and jump", not just "search".
    /// - `goto_match` (int): like `goto: true` but jump to the Nth match
    ///   (0-based, clamped). Lets an agent step through matches by calling
    ///   again with the next index.
    ///
    /// Each match carries the *stable* row index (the same coordinate space
    /// as `screen.scrollback_text`'s `first_row`/`start_line`), so results
    /// stay addressable even as new output scrolls in, plus the column of
    /// the first occurrence in that line.
    fn screen_search(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'pattern'"))?;
        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let dims = pane.get_dimensions();
        let start = dims.scrollback_top;
        let end = dims.physical_top + dims.viewport_rows as isize;
        let (first, lines) = pane.get_lines(start..end);

        let mut matches: Vec<Value> = Vec::new();
        let mut match_rows: Vec<isize> = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            let text = line.as_str().to_string();
            if let Some(byte_off) = text.find(pattern) {
                let row = first + idx as isize;
                matches.push(json!({
                    "row": row,
                    "col": text[..byte_off].chars().count(),
                    "text": text.trim_end(),
                }));
                match_rows.push(row);
                if matches.len() >= max_results {
                    break;
                }
            }
        }

        let goto_requested = params
            .get("goto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || params.get("goto_match").is_some();

        let mut scrolled_to = Value::Null;
        if goto_requested && !match_rows.is_empty() {
            let index = params
                .get("goto_match")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let index = index.min(match_rows.len() - 1);
            let target = match_rows[index];
            self.scroll_pane_viewport_to(pane.pane_id(), target)?;
            scrolled_to = json!({ "row": target, "match_index": index });
        }

        Ok(json!({
            "matches": matches,
            "total": matches.len(),
            "scrolled_to": scrolled_to,
        }))
    }

    /// Scroll the GUI viewport of `pane_id` so that stable row `target` is
    /// on screen, with ~1/4 of the viewport above it for context. The
    /// viewport is per-TermWindow GUI state, not Mux state, so this hops to
    /// the main thread and applies through the owning window's notify queue.
    fn scroll_pane_viewport_to(&self, pane_id: usize, target: isize) -> Result<()> {
        let mux = self.get_mux()?;
        let (_domain, mux_window_id, _tab) = mux
            .resolve_pane_id(pane_id)
            .ok_or_else(|| anyhow!("pane {pane_id} not found in any window"))?;

        let (tx, rx) = std::sync::mpsc::channel();
        promise::spawn::spawn_into_main_thread(async move {
            let result = (|| -> Result<()> {
                use ::window::WindowOps;
                let gui = crate::frontend::front_end()
                    .gui_window_for_mux_window(mux_window_id)
                    .ok_or_else(|| anyhow!("no GUI window for mux window {mux_window_id}"))?;
                gui.window
                    .notify(crate::termwindow::TermWindowNotif::Apply(Box::new(
                        move |term_window| {
                            if let Some(pane) = Mux::get().get_pane(pane_id) {
                                let dims = pane.get_dimensions();
                                let top = (target - dims.viewport_rows as isize / 4)
                                    .max(dims.scrollback_top);
                                // set_viewport itself clamps "past the bottom"
                                // back to live-follow mode.
                                term_window.set_viewport(pane_id, Some(top), dims);
                            }
                        },
                    )));
                Ok(())
            })();
            tx.send(result).ok();
        })
        .detach();

        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| anyhow!("timeout scrolling pane {pane_id} to row {target}"))?
    }

    // --- Orchestrate ---

    fn orchestrate_launch(&self, params: &Value) -> Result<Value> {
        // Create a new tab and run the command
        let result = self.session_create(params)?;
        let id = result.get("id").and_then(|v| v.as_u64());
        if let Some(pane_id) = id {
            if let Some(command) = params.get("command").and_then(|v| v.as_str()) {
                // Brief delay to let shell initialize
                std::thread::sleep(std::time::Duration::from_millis(500));
                if let Ok(mux) = self.get_mux() {
                    if let Some(pane) = mux.get_pane(pane_id as usize) {
                        let input = format!("{}\r", command);
                        let _ = pane.writer().write_all(input.as_bytes());
                    }
                }
            }
        }
        Ok(result)
    }

    fn orchestrate_broadcast(&self, params: &Value) -> Result<Value> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command'"))?;
        let sessions = params
            .get("sessions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing 'sessions'"))?;

        let mux = self.get_mux()?;
        let mut results = Vec::new();
        let input = format!("{}\r", command);

        for sid in sessions {
            let id_str = sid.as_str().unwrap_or("");
            if let Ok(id) = id_str.parse::<usize>() {
                if let Some(pane) = mux.get_pane(id) {
                    match pane.writer().write_all(input.as_bytes()) {
                        Ok(_) => results.push(json!({"session_id": id_str, "sent": true})),
                        Err(e) => {
                            results.push(json!({"session_id": id_str, "error": e.to_string()}))
                        }
                    }
                } else {
                    results.push(json!({"session_id": id_str, "error": "not found"}));
                }
            }
        }

        Ok(json!({"results": results}))
    }

    fn orchestrate_wait(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'pattern'"))?;
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(10000);

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            let text = self.read_pane_text(&pane);
            if text.contains(pattern) {
                return Ok(json!({"matched": true, "pattern": pattern}));
            }
            if start.elapsed() > timeout {
                return Ok(json!({"matched": false, "timed_out": true, "pattern": pattern}));
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    // --- Proxy ---

    /// Always reload from disk so external edits to `~/.unterm/proxy.json`
    /// (or the GUI proxy_settings overlay) are reflected immediately. The
    /// in-memory `mcp_state` proxy field is a write cache; reads should
    /// fetch fresh.
    fn refresh_proxy_state(&self) -> ProxySettings {
        let fresh = load_proxy_settings();
        mcp_state().lock().proxy = fresh.clone();
        fresh
    }

    fn proxy_status(&self) -> Result<Value> {
        let settings = self.refresh_proxy_state();
        let health = if settings.enabled {
            Some(probe_proxy_health(&settings))
        } else {
            None
        };
        // Live-probe each node so the Web Settings availability dots and the
        // rotation pool reflect reality right now, not a stale persisted flag.
        // Built explicitly (not via ProxyNodeConfig's Serialize) because the
        // transient probe fields are `skip_serializing` to keep proxy.json clean.
        let nodes: Vec<Value> = settings
            .nodes
            .iter()
            .map(|n| {
                let (available, latency_ms) = probe_node_latency(&n.url, 300);
                json!({
                    "name": n.name,
                    "url": n.url,
                    "available": available,
                    "latency_ms": latency_ms,
                })
            })
            .collect();
        Ok(json!({
            "enabled": settings.enabled,
            "mode": settings.mode,
            "http_proxy": settings.http_proxy,
            "socks_proxy": settings.socks_proxy,
            "no_proxy": settings.no_proxy,
            "current_node": settings.current_node,
            "node_count": nodes.len(),
            "nodes": nodes,
            "rotation": settings.rotation,
            "health": health,
        }))
    }

    fn proxy_nodes(&self) -> Result<Value> {
        let settings = self.refresh_proxy_state();
        Ok(json!({
            "current_node": settings.current_node,
            "nodes": settings.nodes,
        }))
    }

    fn proxy_configure(&self, params: &Value) -> Result<Value> {
        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();

        let enabled = params
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let mode = params
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("manual")
            .to_string();

        settings.enabled = enabled;
        settings.mode = if enabled { mode } else { "off".to_string() };

        if let Some(http) = params.get("http_proxy").and_then(|v| v.as_str()) {
            settings.http_proxy = Some(http.to_string());
        }
        if let Some(socks) = params.get("socks_proxy").and_then(|v| v.as_str()) {
            settings.socks_proxy = Some(socks.to_string());
        }
        if let Some(no_proxy) = params.get("no_proxy").and_then(|v| v.as_str()) {
            settings.no_proxy = no_proxy.to_string();
        }

        if let Some(nodes) = params.get("nodes").and_then(|v| v.as_array()) {
            settings.nodes = nodes
                .iter()
                .filter_map(|node| {
                    let name = node.get("name")?.as_str()?.to_string();
                    let url = node
                        .get("url")
                        .or_else(|| node.get("http_proxy"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if url.is_empty() {
                        return None;
                    }
                    Some(ProxyNodeConfig {
                        name,
                        url,
                        latency_ms: None,
                        available: true,
                    })
                })
                .collect();
        }

        if let Some(node_name) = params.get("current_node").and_then(|v| v.as_str()) {
            settings.current_node = Some(node_name.to_string());
            if let Some(node) = settings.nodes.iter().find(|node| node.name == node_name) {
                settings.http_proxy = Some(node.url.clone());
            }
        }

        save_proxy_settings(&settings)?;
        state.proxy = settings.clone();
        drop(state);

        Ok(json!({
            "configured": true,
            "status": self.proxy_status()?,
        }))
    }

    fn proxy_disable(&self) -> Result<Value> {
        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();
        settings.enabled = false;
        settings.mode = "off".to_string();
        save_proxy_settings(&settings)?;
        state.proxy = settings;
        Ok(json!({"disabled": true}))
    }

    /// Get or set endpoint-level auto-rotation. With no params, returns the
    /// current rotation config; otherwise updates `enabled` / `pool` /
    /// `interval_secs` and persists. The background monitor (started at GUI
    /// boot) picks up changes on its next tick. Software-agnostic — the pool is
    /// just node names that resolve to HTTP/SOCKS URLs.
    fn proxy_rotation(&self, params: &Value) -> Result<Value> {
        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();
        let mut changed = false;
        if let Some(enabled) = params.get("enabled").and_then(|v| v.as_bool()) {
            settings.rotation.enabled = enabled;
            // Turning rotation on implies the proxy is in use — enable injection
            // too so spawned shells actually route through it and the status bar
            // reads "proxy:on" instead of confusingly showing "off". (The
            // spawn-time liveness probe still guards against a dead endpoint.)
            // We never auto-disable the proxy when rotation goes off — the user
            // may still want the proxy on its own.
            if enabled && !settings.enabled {
                settings.enabled = true;
                if settings.mode.is_empty() || settings.mode == "off" {
                    settings.mode = "auto".to_string();
                }
            }
            changed = true;
        }
        if let Some(group) = params.get("group").and_then(|v| v.as_str()) {
            settings.rotation.group = group.to_string();
            changed = true;
        }
        if let Some(pool) = params.get("pool").and_then(|v| v.as_array()) {
            settings.rotation.pool = pool
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            changed = true;
        }
        if let Some(iv) = params.get("interval_secs").and_then(|v| v.as_u64()) {
            settings.rotation.interval_secs = iv.max(10);
            changed = true;
        }
        if changed {
            save_proxy_settings(&settings)?;
            state.proxy = settings.clone();
        }
        // Report which pool nodes are known vs missing so a caller can spot a
        // typo'd pool entry.
        let known: Vec<&String> = settings
            .rotation
            .pool
            .iter()
            .filter(|name| settings.nodes.iter().any(|n| &n.name == *name))
            .collect();
        Ok(json!({
            "enabled": settings.rotation.enabled,
            "group": settings.rotation.group,
            "pool": settings.rotation.pool,
            "interval_secs": settings.rotation.interval_secs,
            "pool_resolved": known.len(),
            "current_node": settings.current_node,
        }))
    }

    /// Clash/mihomo controller status + switchable groups and their nodes
    /// (with live alive/delay pulled from the proxies snapshot). This is what
    /// powers "read the nodes, you tick boxes" — no hand-typed URLs.
    fn proxy_clash_status(&self) -> Result<Value> {
        let settings = self.refresh_proxy_state();
        let Some(ep) = resolve_clash_endpoint(&settings) else {
            return Ok(json!({
                "connected": false,
                "controller": settings.clash_controller,
                "secret_set": !settings.clash_secret.is_empty(),
            }));
        };
        let version = crate::clash_api::version(&ep).unwrap_or_default();
        let proxies = match crate::clash_api::proxies(&ep) {
            Ok(p) => p,
            Err(e) => return Ok(json!({ "connected": false, "error": e.to_string() })),
        };
        let map = proxies.as_object().cloned().unwrap_or_default();
        let mut groups: Vec<Value> = Vec::new();
        for (name, v) in &map {
            if v.get("type").and_then(|t| t.as_str()) != Some("Selector") {
                continue;
            }
            let now = v
                .get("now")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let all = v
                .get("all")
                .and_then(|a| a.as_array())
                .cloned()
                .unwrap_or_default();
            let nodes: Vec<Value> = all
                .iter()
                .filter_map(|opt| {
                    let nm = opt.as_str()?;
                    let obj = map.get(nm);
                    // You rotate among *leaf* nodes — drop options that are
                    // themselves groups (rotating "to a group" is meaningless).
                    let ty = obj.and_then(|o| o.get("type")).and_then(|t| t.as_str());
                    if matches!(
                        ty,
                        Some("Selector") | Some("URLTest") | Some("Fallback") | Some("LoadBalance")
                    ) {
                        return None;
                    }
                    let (alive, delay) = obj.map(node_health).unwrap_or((false, None));
                    Some(json!({ "name": nm, "alive": alive, "delay": delay }))
                })
                .collect();
            groups.push(json!({ "name": name, "now": now, "nodes": nodes }));
        }
        groups.sort_by(|a, b| {
            a.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        });
        Ok(json!({
            "connected": true,
            "version": version,
            "controller": ep.label(),
            "manual": !settings.clash_controller.is_empty(),
            "groups": groups,
        }))
    }

    /// Point a Clash Selector group at a specific node (manual switch from the
    /// node checkboxes' "use now" affordance, and what rotation calls on
    /// failover).
    fn proxy_clash_select(&self, params: &Value) -> Result<Value> {
        let group = params
            .get("group")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("`group` required"))?;
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("`name` required"))?;
        let settings = mcp_state().lock().proxy.clone();
        let ep = resolve_clash_endpoint(&settings)
            .ok_or_else(|| anyhow::anyhow!("no Clash/mihomo controller found"))?;
        crate::clash_api::select(&ep, group, name)?;
        Ok(json!({ "group": group, "now": name }))
    }

    /// Set (or clear) the manual Clash controller override — the escape hatch
    /// for platforms/setups where auto-discovery can't find the controller
    /// (notably Windows, which has no Unix socket). Returns fresh clash status.
    fn proxy_clash_set_controller(&self, params: &Value) -> Result<Value> {
        let controller = params
            .get("controller")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let secret = params
            .get("secret")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        {
            let mut state = mcp_state().lock();
            let mut settings = state.proxy.clone();
            settings.clash_controller = controller;
            settings.clash_secret = secret;
            save_proxy_settings(&settings)?;
            state.proxy = settings;
        }
        self.proxy_clash_status()
    }

    /// Replace the configured node list (name + url pairs) from the GUI, so the
    /// rotation pool can be built by clicking instead of hand-editing
    /// proxy.json. Prunes any rotation-pool entry or current_node whose node
    /// was removed. Returns the freshly probed list so the UI dots are correct.
    fn proxy_set_nodes(&self, params: &Value) -> Result<Value> {
        let incoming = params
            .get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("`nodes` array required"))?;
        let mut nodes: Vec<ProxyNodeConfig> = Vec::new();
        for n in incoming {
            let name = n
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let url = n
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            // Skip blanks and duplicate names (first wins).
            if name.is_empty() || url.is_empty() || nodes.iter().any(|x| x.name == name) {
                continue;
            }
            nodes.push(ProxyNodeConfig {
                name,
                url,
                latency_ms: None,
                available: false,
            });
        }

        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();
        settings.nodes = nodes;
        // Drop pool entries / current_node that no longer name a real node.
        let node_names: std::collections::HashSet<String> =
            settings.nodes.iter().map(|n| n.name.clone()).collect();
        settings
            .rotation
            .pool
            .retain(|name| node_names.contains(name));
        if let Some(cur) = settings.current_node.clone() {
            if !node_names.contains(&cur) {
                settings.current_node = None;
            }
        }
        save_proxy_settings(&settings)?;
        state.proxy = settings.clone();
        drop(state); // release lock before network probes

        let probed: Vec<Value> = settings
            .nodes
            .iter()
            .map(|n| {
                let (available, latency_ms) = probe_node_latency(&n.url, 300);
                json!({ "name": n.name, "url": n.url, "available": available, "latency_ms": latency_ms })
            })
            .collect();
        Ok(json!({ "nodes": probed, "pool": settings.rotation.pool }))
    }

    fn proxy_switch(&self, params: &Value) -> Result<Value> {
        let node_name = params
            .get("node_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'node_name'"))?;

        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();
        let node = settings
            .nodes
            .iter()
            .find(|node| node.name == node_name)
            .cloned()
            .ok_or_else(|| anyhow!("Proxy node '{}' not found", node_name))?;

        settings.enabled = true;
        settings.mode = "manual".to_string();
        settings.current_node = Some(node.name.clone());
        settings.http_proxy = Some(node.url.clone());
        if settings.socks_proxy.is_none() && node.url.starts_with("socks") {
            settings.socks_proxy = Some(node.url.clone());
        }
        save_proxy_settings(&settings)?;
        state.proxy = settings;

        Ok(json!({
            "switched": true,
            "current_node": node.name,
            "http_proxy": node.url,
        }))
    }

    fn proxy_env(&self) -> Result<Value> {
        let settings = self.refresh_proxy_state();
        let mut env = serde_json::Map::new();
        if settings.enabled {
            // Prefer manual override URLs in proxy.json, otherwise auto-detect.
            let manual_http = settings.http_proxy.clone().filter(|s| !s.is_empty());
            let manual_socks = settings.socks_proxy.clone().filter(|s| !s.is_empty());
            let detected = if manual_http.is_none() && manual_socks.is_none() {
                crate::system_proxy::detect()
            } else {
                None
            };

            let http = manual_http.or_else(|| {
                detected
                    .as_ref()
                    .and_then(|d| d.primary_http().map(str::to_string))
            });
            let socks = manual_socks.or_else(|| detected.as_ref().and_then(|d| d.socks.clone()));
            let no_proxy = detected
                .as_ref()
                .and_then(|d| d.no_proxy.clone())
                .unwrap_or_else(|| settings.no_proxy.clone());

            if let Some(http) = &http {
                env.insert("HTTP_PROXY".to_string(), json!(http));
                env.insert("HTTPS_PROXY".to_string(), json!(http));
            }
            if let Some(socks) = &socks {
                env.insert("ALL_PROXY".to_string(), json!(socks));
            }
            env.insert("NO_PROXY".to_string(), json!(no_proxy));
        }
        Ok(json!({
            "enabled": settings.enabled,
            "env": env,
        }))
    }

    fn proxy_speedtest(&self, params: &Value) -> Result<Value> {
        let target_name = params.get("node_name").and_then(|v| v.as_str());
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(3000);

        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();
        let mut results = Vec::new();

        for node in &mut settings.nodes {
            if target_name.map_or(false, |name| node.name != name) {
                continue;
            }
            let start = std::time::Instant::now();
            let available = probe_proxy_endpoint(&node.url, timeout_ms);
            node.available = available;
            node.latency_ms = if available {
                Some(start.elapsed().as_millis() as u64)
            } else {
                None
            };
            results.push(json!({
                "name": node.name,
                "url": node.url,
                "available": node.available,
                "latency_ms": node.latency_ms,
            }));
        }

        if results.is_empty() && target_name.is_some() {
            return Err(anyhow!("Proxy node '{}' not found", target_name.unwrap()));
        }

        save_proxy_settings(&settings)?;
        state.proxy = settings;
        Ok(json!({"results": results}))
    }

    // --- Workspace ---

    fn workspace_save(&self, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'name'"))?;

        let mux = self.get_mux()?;
        let panes = mux.iter_panes();
        let sessions: Vec<Value> = panes
            .iter()
            .map(|pane| {
                let cwd = pane
                    .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
                    .and_then(|u| cwd_url_to_path(&u.to_string()));
                json!({
                    "id": pane.pane_id(),
                    "title": pane.get_title(),
                    "cwd": cwd,
                })
            })
            .collect();

        let workspace = json!({
            "name": name,
            "sessions": sessions,
            "saved_at": chrono::Local::now().to_rfc3339(),
        });

        // Save to ~/.unterm/workspaces/<name>.json
        let dir = dirs_next::home_dir()
            .unwrap_or_default()
            .join(".unterm")
            .join("workspaces");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", name));
        std::fs::write(&path, serde_json::to_string_pretty(&workspace)?)?;

        Ok(json!({"saved": true, "name": name, "sessions": sessions.len()}))
    }

    fn workspace_restore(&self, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'name'"))?;

        let path = dirs_next::home_dir()
            .unwrap_or_default()
            .join(".unterm")
            .join("workspaces")
            .join(format!("{}.json", name));

        if !path.exists() {
            return Err(anyhow!("Workspace '{}' not found", name));
        }

        let data = std::fs::read_to_string(&path)?;
        let workspace: Value = serde_json::from_str(&data)?;
        let dry_run = params
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let sessions = workspace
            .get("sessions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut planned = Vec::new();
        let mut created = Vec::new();
        let mut failed = Vec::new();

        for session in &sessions {
            let saved_id = session.get("id").cloned().unwrap_or(Value::Null);
            let title = session
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let cwd = session
                .get("cwd")
                .and_then(|v| v.as_str())
                .and_then(cwd_url_to_path);

            planned.push(json!({
                "saved_id": saved_id,
                "title": title,
                "cwd": cwd,
            }));

            if dry_run {
                continue;
            }

            let mut create_params = json!({});
            if let Some(cwd) = &cwd {
                create_params["cwd"] = json!(cwd);
            }
            match self.session_create(&create_params) {
                Ok(value) => {
                    created.push(json!({
                        "saved_id": saved_id,
                        "cwd": cwd,
                        "created": value,
                    }));
                }
                Err(err) => {
                    failed.push(json!({
                        "saved_id": saved_id,
                        "cwd": cwd,
                        "error": err.to_string(),
                    }));
                }
            }
        }

        Ok(json!({
            "restored": !dry_run && failed.is_empty(),
            "dry_run": dry_run,
            "name": name,
            "path": path,
            "workspace": workspace,
            "planned": planned,
            "created": created,
            "failed": failed,
        }))
    }

    fn workspace_list(&self) -> Result<Value> {
        let dir = dirs_next::home_dir()
            .unwrap_or_default()
            .join(".unterm")
            .join("workspaces");

        let mut workspaces = Vec::new();
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "json") {
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let mut item = json!({
                            "name": name,
                            "path": path,
                        });
                        match std::fs::read_to_string(&path)
                            .ok()
                            .and_then(|data| serde_json::from_str::<Value>(&data).ok())
                        {
                            Some(workspace) => {
                                item["saved_at"] =
                                    workspace.get("saved_at").cloned().unwrap_or(Value::Null);
                                item["session_count"] = json!(workspace
                                    .get("sessions")
                                    .and_then(|v| v.as_array())
                                    .map(|sessions| sessions.len())
                                    .unwrap_or(0));
                            }
                            None => {
                                item["saved_at"] = Value::Null;
                                item["session_count"] = json!(0);
                                item["error"] = json!("could not read workspace file");
                            }
                        }
                        workspaces.push(item);
                    }
                }
            }
        }
        workspaces.sort_by(|a, b| {
            a.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        });

        Ok(json!({"workspaces": workspaces}))
    }

    // --- Capture ---

    fn capture_screen(&self, params: &Value) -> Result<Value> {
        let mux = self.get_mux()?;
        let panes = mux.iter_panes();
        let mut captures = Vec::new();

        for pane in &panes {
            let text = self.read_pane_text(pane);
            captures.push(json!({
                "session_id": pane.pane_id().to_string(),
                "title": pane.get_title(),
                "screen": text,
                "type": "text",
            }));
        }

        let include_base64 = params
            .get("include_base64")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let image = capture_screen_image(include_base64)?;
        Ok(json!({
            "captures": captures,
            "image": image,
            "type": "image/png",
            "text_snapshot": true,
        }))
    }

    fn capture_window(&self, params: &Value) -> Result<Value> {
        let title_filter = params.get("title").and_then(|v| v.as_str());
        let pid_filter = params.get("pid").and_then(|v| v.as_u64()).map(|v| v as u32);
        let mux = self.get_mux()?;
        let panes = mux.iter_panes();

        let include_base64 = params
            .get("include_base64")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let image = capture_window_image(title_filter, pid_filter, include_base64)?;

        for pane in &panes {
            let pane_title = pane.get_title();
            let matches = title_filter.map_or(true, |t| {
                pane_title.contains(t) || pane.pane_id().to_string().contains(t)
            });
            if matches {
                let text = self.read_pane_text(pane);
                return Ok(json!({
                    "session_id": pane.pane_id().to_string(),
                    "title": pane_title,
                    "screen": text,
                    "image": image,
                    "type": "image/png",
                    "text_snapshot": true,
                }));
            }
        }

        Ok(json!({
            "image": image,
            "type": "image/png",
            "text_snapshot": false,
        }))
    }

    fn capture_select(&self) -> Result<Value> {
        let image = capture_screen_image(false)?;
        Ok(json!({
            "image": image,
            "type": "image/png",
            "mode": "screen_fallback",
            "message": "Interactive region selection is not available in headless MCP mode; captured the screen instead.",
        }))
    }

    fn capture_clipboard(&self) -> Result<Value> {
        clipboard_read_any()
    }

    /// Scrolling screenshot of the terminal itself: headlessly re-render the
    /// pane's entire scrollback into one tall PNG (no pixel capture, no
    /// occlusion constraints). `screen.scrollback_text` remains the
    /// AI-friendly text path; this is the human-shareable image path.
    fn capture_scrollback(&self, params: &Value) -> Result<Value> {
        let pane = match self.get_pane(params) {
            Ok(p) => p,
            Err(_) => {
                let mux = self.get_mux()?;
                mux.iter_windows()
                    .into_iter()
                    .find_map(|wid| mux.get_active_tab_for_window(wid))
                    .and_then(|tab| tab.get_active_pane())
                    .ok_or_else(|| anyhow!("no active pane available"))?
            }
        };
        let mut opts = crate::scrollshot::ScrollbackPngOptions::default();
        if let Some(n) = params.get("max_rows").and_then(|v| v.as_u64()) {
            opts.max_rows = (n as usize).max(1);
        }
        if let Some(n) = params.get("dpi").and_then(|v| v.as_u64()) {
            opts.dpi = (n as usize).clamp(48, 288);
        }
        let dir = capture_output_dir()?;
        let path = dir.join(format!(
            "scrollback_{}.png",
            chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
        ));
        let session = pane.pane_id().to_string();
        self.audit("capture.scrollback", Some(&session), "");
        let r = crate::scrollshot::render_scrollback_png(&pane, &path, &opts)?;
        Ok(json!({
            "path": r.path.display().to_string(),
            "width": r.width,
            "height": r.height,
            "rows": r.rows,
            "cols": r.cols,
            "truncated": r.truncated,
            "first_row": r.first_row,
            "session_id": session,
            "type": "image/png",
        }))
    }

    /// Scrolling screenshot of ANOTHER app's window (macOS): synthesize
    /// wheel events and stitch the frames by exact row-hash matching.
    fn capture_window_scroll(&self, params: &Value) -> Result<Value> {
        #[cfg(target_os = "macos")]
        {
            use crate::scrollshot::external;
            let pid = params.get("pid").and_then(|v| v.as_u64()).map(|v| v as u32);
            let app = params.get("app").and_then(|v| v.as_str());
            let title = params.get("title").and_then(|v| v.as_str());
            let under_cursor = params
                .get("under_cursor")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if pid.is_none() && app.is_none() && title.is_none() && !under_cursor {
                return Err(anyhow!(
                    "provide at least one of pid / app / title, or under_cursor=true"
                ));
            }
            let target = if under_cursor {
                external::window_under_cursor()?
            } else {
                external::find_target(pid, app, title)?
            };
            let mut opts = external::ScrollCaptureOptions::default();
            if let Some(n) = params.get("max_frames").and_then(|v| v.as_u64()) {
                opts.max_frames = (n as usize).clamp(2, 120);
            }
            if let Some(n) = params.get("settle_ms").and_then(|v| v.as_u64()) {
                opts.settle_ms = n.clamp(100, 2000);
            }
            if let Some(b) = params.get("activate").and_then(|v| v.as_bool()) {
                opts.activate = b;
            }
            if let Some(b) = params.get("restore_scroll").and_then(|v| v.as_bool()) {
                opts.restore_scroll = b;
            }
            let dir = capture_output_dir()?;
            let path = dir.join(format!(
                "windowscroll_{}.png",
                chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
            ));
            self.audit(
                "capture.window_scroll",
                None,
                &format!(
                    "app={} pid={} title={}",
                    target.app, target.pid, target.title
                ),
            );
            let r = external::scroll_capture_window(&target, &path, &opts)?;
            Ok(json!({
                "path": r.path.display().to_string(),
                "width": r.width,
                "height": r.height,
                "frames": r.frames,
                "window": {
                    "app": r.window.app,
                    "title": r.window.title,
                    "pid": r.window.pid,
                    "window_id": r.window.window_id,
                },
                "hint": r.hint,
                "type": "image/png",
            }))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = params;
            Err(anyhow!(
                "capture.window_scroll is currently macOS-only; \
                 use capture.window for a single-frame capture"
            ))
        }
    }

    // --- Policy ---

    fn policy_set(&self, params: &Value) -> Result<Value> {
        let policy: CommandPolicy =
            serde_json::from_value(params.clone()).map_err(|e| anyhow!("Invalid policy: {}", e))?;
        self.audit("policy.set", None, &format!("enabled={}", policy.enabled));
        mcp_state().lock().policy = policy;
        Ok(json!({"set": true}))
    }

    fn policy_check(&self, params: &Value) -> Result<Value> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command'"))?;

        let state = mcp_state().lock();
        if !state.policy.enabled {
            return Ok(json!({"allowed": true, "reason": "Policy disabled"}));
        }

        for pattern in &state.policy.blocked_patterns {
            if command.contains(pattern) {
                return Ok(json!({
                    "allowed": false,
                    "reason": format!("Blocked by pattern: {}", pattern),
                }));
            }
        }

        Ok(json!({"allowed": true}))
    }

    fn check_policy_internal(&self, command: &str) -> Result<()> {
        let state = mcp_state().lock();
        if !state.policy.enabled {
            return Ok(());
        }
        for pattern in &state.policy.blocked_patterns {
            if command.contains(pattern) {
                return Err(anyhow!("Command blocked by policy: {}", pattern));
            }
        }
        Ok(())
    }

    // --- System ---

    fn system_info(&self) -> Result<Value> {
        let mux = self.get_mux()?;
        let pane_count = mux.iter_panes().len();
        Ok(json!({
            "name": "Unterm",
            "version": "2.0.0",
            "engine": "Unterm (WezTerm)",
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "active_sessions": pane_count,
            "hostname": hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default(),
        }))
    }

    fn system_launch_admin(&self, params: &Value) -> Result<Value> {
        #[cfg(windows)]
        {
            let dry_run = params
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let shell = params
                .get("shell")
                .and_then(|v| v.as_str())
                .unwrap_or("pwsh");
            let args = elevated_unterm_command_args(shell)?;

            if !dry_run {
                std::process::Command::new(&args[0])
                    .args(&args[1..])
                    .spawn()
                    .context("launch elevated Unterm window via PowerShell RunAs")?;
            }

            Ok(json!({
                "status": if dry_run { "dry_run" } else { "launched" },
                "requires_uac": true,
                "command": args,
            }))
        }

        #[cfg(not(windows))]
        {
            let _ = params;
            Err(anyhow!("Administrator launch is only supported on Windows"))
        }
    }

    // --- Helpers ---

    fn read_pane_text(&self, pane: &Arc<dyn Pane>) -> String {
        let dims = pane.get_dimensions();
        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (_first, lines) = pane.get_lines(first_row..last_row);
        lines
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn screen_read(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();
        let cursor = pane.get_cursor_position();

        // Read visible lines
        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (first, lines) = pane.get_lines(first_row..last_row);

        let cells: Vec<Value> = lines
            .iter()
            .enumerate()
            .map(|(row_idx, line)| {
                let text = line.as_str().to_string();
                let text = text.trim_end().to_string();
                json!({
                    "row": first as i64 + row_idx as i64,
                    "text": text,
                })
            })
            .collect();

        Ok(json!({
            "cells": cells,
            "cursor": {
                "x": cursor.x,
                "y": cursor.y,
                "visible": cursor.visibility == termwiz::surface::CursorVisibility::Visible,
            },
            "cols": dims.cols,
            "rows": dims.viewport_rows,
            "scrollback_rows": dims.scrollback_rows,
        }))
    }

    fn screen_text(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();
        let cursor = pane.get_cursor_position();

        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (_first, lines) = pane.get_lines(first_row..last_row);

        let text_lines: Vec<String> = lines
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect();

        Ok(json!({
            "lines": text_lines,
            "cursor": { "x": cursor.x, "y": cursor.y },
            "cols": dims.cols,
            "rows": dims.viewport_rows,
        }))
    }

    /// Full scrollback + viewport as a single string, optionally with ANSI
    /// styling preserved. For long terminal output you want to feed to an
    /// LLM, this is strictly better than a rendered "long screenshot": no
    /// OCR step, no font fidelity loss, no encoding back to text on the
    /// other side. Pairs with `capture.*` for the human-consumption path.
    ///
    /// Params:
    /// - `pane_id` / `session_id` (optional): standard pane resolution.
    /// - `escapes` (bool, default false): if true, returns text with
    ///   embedded ANSI color/style escapes. If false, plain text only.
    /// - `start_line` (int, optional): clamp the start. Default: scrollback_top.
    /// - `end_line` (int, optional): clamp the end (exclusive). Default: bottom of viewport.
    ///   Both are absolute StableRowIndex values (negatives are allowed; the
    ///   server clamps to the actual scrollback range).
    /// - `tail_lines` (int, optional): keep only the last N rows within the
    ///   selected range. Useful for LLM hand-offs that need recent output
    ///   without fetching the entire scrollback.
    fn screen_scrollback_text(&self, params: &Value) -> Result<Value> {
        // Unlike the other screen.* methods we let callers omit pane_id and
        // fall back to the active pane of the first window — the typical
        // agent intent is "dump *this* terminal," not "dump some specific
        // session id I don't know yet."
        let pane = match self.get_pane(params) {
            Ok(p) => p,
            Err(_) => {
                let mux = self.get_mux()?;
                mux.iter_windows()
                    .into_iter()
                    .find_map(|wid| mux.get_active_tab_for_window(wid))
                    .and_then(|tab| tab.get_active_pane())
                    .ok_or_else(|| anyhow!("no active pane available"))?
            }
        };
        let dims = pane.get_dimensions();
        let want_escapes = params
            .get("escapes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let viewport_bottom = dims.physical_top + dims.viewport_rows as isize;
        let mut start = params
            .get("start_line")
            .and_then(|v| v.as_i64())
            .map(|n| n as isize)
            .unwrap_or(dims.scrollback_top)
            .max(dims.scrollback_top);
        let end = params
            .get("end_line")
            .and_then(|v| v.as_i64())
            .map(|n| n as isize)
            .unwrap_or(viewport_bottom)
            .min(viewport_bottom);
        if let Some(tail) = params.get("tail_lines").and_then(|v| v.as_i64()) {
            if tail > 0 {
                start = start.max(end.saturating_sub(tail as isize));
            }
        }

        if end <= start {
            return Ok(json!({
                "text": "",
                "lines": Vec::<String>::new(),
                "first_row": start,
                "row_count": 0,
                "cols": dims.cols,
                "escapes": want_escapes,
                "scrollback_top": dims.scrollback_top,
                "physical_top": dims.physical_top,
                "viewport_rows": dims.viewport_rows,
            }));
        }

        let (first, lines) = pane.get_lines(start..end);

        if want_escapes {
            let text = termwiz_funcs::lines_to_escapes(lines).map_err(|e| anyhow!(e))?;
            Ok(json!({
                "text": text,
                "first_row": first,
                "row_count": (end - start) as i64,
                "cols": dims.cols,
                "escapes": true,
                "scrollback_top": dims.scrollback_top,
                "physical_top": dims.physical_top,
                "viewport_rows": dims.viewport_rows,
            }))
        } else {
            let text_lines: Vec<String> = lines
                .iter()
                .map(|line| line.as_str().trim_end().to_string())
                .collect();
            let text = text_lines.join("\n");
            Ok(json!({
                "text": text,
                "lines": text_lines,
                "first_row": first,
                "row_count": (end - start) as i64,
                "cols": dims.cols,
                "escapes": false,
                "scrollback_top": dims.scrollback_top,
                "physical_top": dims.physical_top,
                "viewport_rows": dims.viewport_rows,
            }))
        }
    }

    fn screen_cursor(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let cursor = pane.get_cursor_position();

        Ok(json!({
            "x": cursor.x,
            "y": cursor.y,
            "visible": cursor.visibility == termwiz::surface::CursorVisibility::Visible,
            "shape": format!("{:?}", cursor.shape),
        }))
    }

    fn screen_detect_errors(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();

        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (_first, lines) = pane.get_lines(first_row..last_row);

        let error_patterns = [
            "error:",
            "Error:",
            "ERROR:",
            "error[",
            "fatal:",
            "Fatal:",
            "FATAL:",
            "panic:",
            "PANIC:",
            "not found",
            "command not found",
            "Permission denied",
            "permission denied",
            "No such file or directory",
            "failed",
            "FAILED",
            "traceback",
            "Traceback",
            "Exception",
            "exception:",
            "segfault",
            "Segmentation fault",
        ];

        let mut errors: Vec<Value> = Vec::new();

        for (row_idx, line) in lines.iter().enumerate() {
            let text = line.as_str().to_string();
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            for pattern in &error_patterns {
                if trimmed.contains(pattern) {
                    errors.push(json!({
                        "row": first_row as i64 + row_idx as i64,
                        "text": trimmed,
                        "pattern": pattern,
                    }));
                    break;
                }
            }
        }

        Ok(json!({
            "has_errors": !errors.is_empty(),
            "errors": errors,
        }))
    }

    fn selftest_run(&self, params: &Value) -> Result<Value> {
        let mut checks: Vec<Value> = Vec::new();

        let mux_available = Mux::try_get().is_some();
        checks.push(json!({
            "name": "mux.available",
            "ok": mux_available,
            "detail": if mux_available { "Mux is available" } else { "Mux is not initialized" },
        }));

        let health = self.server_health()?;
        checks.push(json!({
            "name": "server.health",
            "ok": health["status"] == "ok",
            "detail": health,
        }));

        let caps = self.server_capabilities()?;
        let has_screen = caps
            .get("screen")
            .and_then(|v| v.as_array())
            .is_some_and(|v| !v.is_empty());
        checks.push(json!({
            "name": "server.capabilities",
            "ok": has_screen,
            "detail": {
                "has_screen": has_screen,
            },
        }));

        let policy = self.policy_check(&json!({"command": "echo unterm-selftest"}));
        checks.push(json!({
            "name": "policy.check",
            "ok": policy.is_ok(),
            "detail": match policy {
                Ok(value) => value,
                Err(err) => json!({"error": err.to_string()}),
            },
        }));

        let admin = self.system_launch_admin(&json!({"dry_run": true, "shell": "pwsh"}));
        // `system.launch_admin` is Windows-only (UAC elevation). Off Windows it
        // returns Err by design, so treating that as a failed check made
        // selftest.run always report ok:false on macOS/Linux. Score the Err as
        // "skipped" (n/a) on non-Windows so the suite reflects real health.
        let admin_ok = admin.is_ok() || cfg!(not(windows));
        checks.push(json!({
            "name": "system.launch_admin",
            "ok": admin_ok,
            "detail": match admin {
                Ok(value) => value,
                Err(err) => json!({"skipped": true, "reason": err.to_string()}),
            },
        }));

        let proxy = self.proxy_status();
        checks.push(json!({
            "name": "proxy.status",
            "ok": proxy.is_ok(),
            "detail": match proxy {
                Ok(value) => value,
                Err(err) => json!({"error": err.to_string()}),
            },
        }));

        let capture = self.capture_window(&json!({"pid": std::process::id()}));
        checks.push(json!({
            "name": "capture.window",
            "ok": capture
                .as_ref()
                .ok()
                .and_then(|value| value.pointer("/image/path"))
                .and_then(|value| value.as_str())
                .map(|path| std::path::Path::new(path).exists())
                .unwrap_or(false),
            "detail": match capture {
                Ok(value) => value,
                Err(err) => json!({"error": err.to_string()}),
            },
        }));

        if let Some(session_id) = params.get("session_id").and_then(|v| v.as_str()) {
            let session_params = json!({ "session_id": session_id });

            let session = self.session_get(&session_params);
            checks.push(json!({
                "name": "session.status",
                "ok": session.is_ok(),
                "detail": match session {
                    Ok(value) => value,
                    Err(err) => json!({"error": err.to_string()}),
                },
            }));

            let screen = self.screen_text(&session_params);
            checks.push(json!({
                "name": "screen.text",
                "ok": screen.is_ok(),
                "detail": match screen {
                    Ok(value) => value,
                    Err(err) => json!({"error": err.to_string()}),
                },
            }));

            let detect = self.screen_detect_errors(&session_params);
            checks.push(json!({
                "name": "screen.detect_errors",
                "ok": detect.is_ok(),
                "detail": match detect {
                    Ok(value) => value,
                    Err(err) => json!({"error": err.to_string()}),
                },
            }));
        }

        // Non-mutating recording self-check against pane 0 (or the
        // first available pane). We never start a recording here.
        let probe_id =
            Mux::try_get().and_then(|mux| mux.iter_panes().first().map(|p| p.pane_id() as u64));
        let rec_status = self.session_recording_status(&json!({"id": probe_id.unwrap_or(0)}));
        checks.push(json!({
            "name": "session.recording_status",
            "ok": rec_status.is_ok(),
            "detail": match rec_status {
                Ok(value) => value,
                Err(err) => json!({"error": err.to_string()}),
            },
        }));

        let ok = checks
            .iter()
            .all(|check| check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

        Ok(json!({
            "ok": ok,
            "checks": checks,
        }))
    }

    // ----------------------------------------------------------------
    // Session recording
    // ----------------------------------------------------------------

    fn session_recording_start(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let pane_id = pane.pane_id();
        self.audit(
            "session.recording_start",
            Some(&pane_id.to_string()),
            "start",
        );
        let r = crate::recording::start_recording(pane_id)?;
        Ok(json!({
            "session_id": r.session_id,
            "log_path": r.log_path,
            "md_path_when_done": r.md_path,
        }))
    }

    fn session_recording_stop(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let pane_id = pane.pane_id();
        self.audit("session.recording_stop", Some(&pane_id.to_string()), "stop");
        let r = crate::recording::stop_recording(pane_id)?;
        Ok(json!({
            "session_id": r.session_id,
            "ended_at": r.ended_at,
            "block_count": r.block_count,
            "exit_reason": r.exit_reason,
            "md_path": r.md_path,
        }))
    }

    fn session_recording_status(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        Ok(crate::recording::recording_status(pane.pane_id()))
    }

    fn session_recording_list(&self, params: &Value) -> Result<Value> {
        let project = params.get("project").and_then(|v| v.as_str());
        let entries = crate::recording::list_sessions(project)?;
        let entries_json: Vec<Value> = entries
            .into_iter()
            .map(|e| {
                json!({
                    "unterm_session_id": e.unterm_session_id,
                    "tab_id": e.tab_id,
                    "project_path": e.project_path,
                    "project_slug": e.project_slug,
                    "started_at": e.started_at,
                    "ended_at": e.ended_at,
                    "block_count": e.block_count,
                    "bytes_raw": e.bytes_raw,
                    "log_path": e.log_path,
                    "md_path": e.md_path,
                })
            })
            .collect();
        Ok(json!(entries_json))
    }

    fn session_recording_read(&self, params: &Value) -> Result<Value> {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'session_id'"))?;
        let md = crate::recording::read_session_markdown(session_id)?;
        Ok(json!({"markdown": md}))
    }

    fn session_recording_attach_trace(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let trace_id = params
            .get("trace_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'trace_id'"))?
            .to_string();
        let traces = crate::recording::attach_trace(pane.pane_id(), trace_id)?;
        Ok(json!({"trace_ids": traces}))
    }

    fn session_export_markdown(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .map(std::path::PathBuf::from);
        let (dest, out) = crate::recording::export_pane_markdown(pane.pane_id(), path)?;
        Ok(json!({
            "session_id": uuid::Uuid::new_v4().to_string(),
            "path": dest.display().to_string(),
            "bytes": out.markdown.len(),
            "block_count": out.block_count,
        }))
    }
}

fn proxy_config_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("proxy.json")
}

fn load_proxy_settings() -> ProxySettings {
    let path = proxy_config_path();
    let mut settings: ProxySettings = match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => ProxySettings::default(),
    };

    // Auto-refresh: when the user is in "auto" mode (or hasn't picked a
    // mode yet), re-run system_proxy::detect() at every load and overlay
    // its URLs over whatever stale values were on disk. This is what
    // makes "I changed Clash from 7890 to 7897" Just Work without the
    // user having to delete proxy.json by hand. Manual mode preserves
    // the user's explicit URLs untouched.
    //
    // We don't persist the refreshed URLs to disk — they're overlay-only.
    // That keeps the on-disk file as a record of *intent* (auto vs
    // manual + manual URLs if applicable), while in-memory state always
    // reflects what's actually reachable right now.
    let is_auto = settings.mode != "manual";
    if is_auto {
        if let Some(found) = crate::system_proxy::detect() {
            settings.http_proxy = found.primary_http().map(|s| s.to_string());
            settings.socks_proxy = found.socks.clone();
            if !settings.no_proxy.is_empty() && found.no_proxy.as_deref().unwrap_or("").is_empty() {
                // Keep user's no_proxy if set; otherwise take detected.
            } else if let Some(np) = found.no_proxy.clone() {
                settings.no_proxy = np;
            }
        }
    }

    settings
}

/// Result of probing whether the configured (or auto-detected) proxy is
/// reachable. Returned in the `health` field of `proxy.status`.
fn probe_proxy_health(settings: &ProxySettings) -> Value {
    // 1. If the user supplied an explicit URL, probe that.
    if let Some(url) = settings
        .http_proxy
        .as_ref()
        .filter(|s| !s.is_empty())
        .or(settings.socks_proxy.as_ref().filter(|s| !s.is_empty()))
    {
        let alive = probe_proxy_endpoint(url, 200);
        return json!({
            "source": "manual",
            "url": url,
            "reachable": alive,
            "hint": if alive { "" } else { "configured proxy is not responding — start your proxy software or disable the toggle" },
        });
    }
    // 2. Otherwise rely on the auto-detect path.
    match crate::system_proxy::detect() {
        Some(found) => json!({
            "source": found.source,
            "url": found.primary_http(),
            "socks": found.socks,
            "no_proxy": found.no_proxy,
            "reachable": true,
        }),
        None => json!({
            "source": "auto",
            "reachable": false,
            "hint": "no system proxy detected and no responsive proxy on common local ports (7897, 7890, 1087, …); start your Clash/V2Ray or set a system proxy in Settings",
        }),
    }
}

fn save_proxy_settings(settings: &ProxySettings) -> Result<()> {
    let path = proxy_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

fn probe_proxy_endpoint(url: &str, timeout_ms: u64) -> bool {
    let Some(rest) = url.split("://").nth(1) else {
        return false;
    };
    let host_port = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .rsplit('@')
        .next()
        .unwrap_or(rest);
    let mut parts = host_port.rsplitn(2, ':');
    let Some(port) = parts.next().and_then(|p| p.parse::<u16>().ok()) else {
        return false;
    };
    let host = parts.next().unwrap_or("127.0.0.1");
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    let timeout = std::time::Duration::from_millis(timeout_ms);
    addrs
        .into_iter()
        .any(|addr| std::net::TcpStream::connect_timeout(&addr, timeout).is_ok())
}

/// Probe a node URL, returning `(reachable, latency_ms)`. Latency is the TCP
/// connect time in milliseconds, or `None` when unreachable. Used to fill the
/// live availability/latency the Web Settings rotation pool shows.
fn probe_node_latency(url: &str, timeout_ms: u64) -> (bool, Option<u64>) {
    let Some(rest) = url.split("://").nth(1) else {
        return (false, None);
    };
    let host_port = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .rsplit('@')
        .next()
        .unwrap_or(rest);
    let mut parts = host_port.rsplitn(2, ':');
    let Some(port) = parts.next().and_then(|p| p.parse::<u16>().ok()) else {
        return (false, None);
    };
    let host = parts.next().unwrap_or("127.0.0.1");
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return (false, None);
    };
    let timeout = std::time::Duration::from_millis(timeout_ms);
    for addr in addrs {
        let start = std::time::Instant::now();
        if std::net::TcpStream::connect_timeout(&addr, timeout).is_ok() {
            return (true, Some(start.elapsed().as_millis() as u64));
        }
    }
    (false, None)
}

/// Pick the fastest reachable node named in `pool` from `nodes`. `probe`
/// returns `Some(latency_ms)` if a node's URL is reachable, `None` if it's
/// down. Pure selection so it's unit-testable without touching the network.
fn select_fastest_node<'a>(
    nodes: &'a [ProxyNodeConfig],
    pool: &[String],
    probe: impl Fn(&str) -> Option<u64>,
) -> Option<&'a ProxyNodeConfig> {
    let mut best: Option<(&ProxyNodeConfig, u64)> = None;
    for name in pool {
        if let Some(node) = nodes.iter().find(|n| &n.name == name) {
            if let Some(latency) = probe(&node.url) {
                if best.as_ref().map_or(true, |(_, b)| latency < *b) {
                    best = Some((node, latency));
                }
            }
        }
    }
    best.map(|(n, _)| n)
}

#[cfg(test)]
mod proxy_rotation_tests {
    use super::*;

    fn node(name: &str, url: &str) -> ProxyNodeConfig {
        ProxyNodeConfig {
            name: name.into(),
            url: url.into(),
            latency_ms: None,
            available: false,
        }
    }

    #[test]
    fn picks_fastest_reachable_skips_down_and_unknown() {
        let nodes = vec![
            node("a", "http://a:1"),
            node("b", "http://b:2"),
            node("c", "http://c:3"),
        ];
        // pool includes a missing name; "a" is down, "b" slow, "c" fastest.
        let pool = vec!["a".into(), "b".into(), "missing".into(), "c".into()];
        let pick = select_fastest_node(&nodes, &pool, |url| match url {
            "http://a:1" => None,     // unreachable → skipped
            "http://b:2" => Some(80), // reachable, slower
            "http://c:3" => Some(20), // reachable, fastest → winner
            _ => None,
        });
        assert_eq!(pick.map(|n| n.name.as_str()), Some("c"));
    }

    #[test]
    fn none_when_pool_all_down_or_empty() {
        let nodes = vec![node("a", "http://a:1")];
        assert!(select_fastest_node(&nodes, &["a".into()], |_| None).is_none());
        assert!(select_fastest_node(&nodes, &[], |_| Some(5)).is_none());
        // a pool name that isn't in nodes resolves to nothing.
        assert!(select_fastest_node(&nodes, &["ghost".into()], |_| Some(5)).is_none());
    }

    /// The core of the "rotation keeps dropping my network" fix: a single missed
    /// health check must NOT cross the failover threshold; only repeated misses
    /// on the same node do, and any success / node change resets the count.
    #[test]
    fn failover_debounces_until_consecutive_failures() {
        // The fix only makes sense if more than one miss is required.
        assert!(
            ROTATION_FAILS_BEFORE_SWITCH >= 2,
            "a single jittery probe must never trigger a switch"
        );
        rotation_reset_failure();

        // One miss: recorded, but below the switch threshold → hold, don't switch.
        let first = rotation_note_failure("node-A");
        assert_eq!(first, 1);
        assert!(first < ROTATION_FAILS_BEFORE_SWITCH);

        // Consecutive misses on the same node accumulate up to the threshold.
        let mut count = first;
        while count < ROTATION_FAILS_BEFORE_SWITCH {
            count = rotation_note_failure("node-A");
        }
        assert!(
            count >= ROTATION_FAILS_BEFORE_SWITCH,
            "repeated misses must fail over"
        );

        // A healthy probe (or a completed switch) clears the strike count.
        rotation_reset_failure();
        assert_eq!(
            rotation_note_failure("node-A"),
            1,
            "reset must wipe accumulated failures"
        );

        // A different active node starts fresh — it inherits no strikes from the
        // node we just left, so it can't be condemned on its first miss.
        assert_eq!(
            rotation_note_failure("node-B"),
            1,
            "a new active node starts with a clean slate"
        );
        rotation_reset_failure();
    }
}

/// Endpoint-level proxy failover (one check). If auto-rotation is on and the
/// active node is unreachable, probe the pool and switch to the fastest live
/// node. Software-agnostic: a "node" is just an HTTP/SOCKS URL, probed by TCP —
/// works with any proxy app (Clash/Mihomo, V2Ray, sing-box, remote SOCKS, …).
/// Reuses the same probe + persistence as the manual proxy methods so a rotated
/// switch is indistinguishable from a hand switch. Returns the node switched to.
/// Extract `(alive, last_delay_ms)` from a Clash proxy node object. A node with
/// a last-history delay of 0 (mihomo's "timed out" sentinel) reads as down.
fn node_health(obj: &Value) -> (bool, Option<u64>) {
    let delay = obj
        .get("history")
        .and_then(|h| h.as_array())
        .and_then(|arr| arr.last())
        .and_then(|e| e.get("delay"))
        .and_then(|d| d.as_u64())
        .filter(|d| *d > 0);
    let alive = obj
        .get("alive")
        .and_then(|a| a.as_bool())
        .unwrap_or(delay.is_some());
    (alive, delay)
}

/// Resolve the Clash/mihomo controller to talk to: the user's manual override
/// (if set and reachable) wins, otherwise fall back to auto-discovery. The
/// manual override is the escape hatch for Windows / non-standard setups.
fn resolve_clash_endpoint(settings: &ProxySettings) -> Option<crate::clash_api::ClashEndpoint> {
    if !settings.clash_controller.trim().is_empty() {
        let ep =
            crate::clash_api::manual_endpoint(&settings.clash_controller, &settings.clash_secret);
        if crate::clash_api::version(&ep).is_ok() {
            return Some(ep);
        }
    }
    crate::clash_api::discover_cached()
}

/// Consecutive failed health-check ticks for the rotation's *current* node.
/// Reset to 0 the instant a probe succeeds or we switch. This is the guard that
/// keeps a single slow/jittery probe from yanking the proxy exit out from under
/// live connections — the root cause of "rotation keeps dropping my network."
/// Only the single rotation-monitor thread touches it, so a plain Mutex is fine.
static ROTATION_FAILS: std::sync::Mutex<(String, u32)> = std::sync::Mutex::new((String::new(), 0));

/// How many consecutive ticks (each already retried within the tick) the active
/// node must miss before we fail over. With the default 30s interval that's a
/// ~1 min grace window — long enough to ride out latency spikes, short enough
/// to recover from a genuinely dead node.
const ROTATION_FAILS_BEFORE_SWITCH: u32 = 2;

/// Note a failed health-check tick for `node`; returns the new consecutive
/// count. Resets first when the active node changed since the last tick (so a
/// new node starts with a clean slate). Fails open (returns the switch
/// threshold) if the lock is poisoned, so a poisoned mutex can't wedge failover.
fn rotation_note_failure(node: &str) -> u32 {
    let Ok(mut g) = ROTATION_FAILS.lock() else {
        return ROTATION_FAILS_BEFORE_SWITCH;
    };
    if g.0 != node {
        g.0 = node.to_string();
        g.1 = 0;
    }
    g.1 += 1;
    g.1
}

/// Clear the failure counter — the active node is healthy again, or we just
/// switched to a fresh one.
fn rotation_reset_failure() {
    if let Ok(mut g) = ROTATION_FAILS.lock() {
        g.0.clear();
        g.1 = 0;
    }
}

/// Delay-test a clash node, retrying once so a single jittery probe doesn't read
/// as "dead." Alive if any attempt returns a positive delay. A genuinely dead
/// node fails both attempts within the same tick; a merely slow one usually
/// passes on the retry.
fn clash_node_alive(
    ep: &crate::clash_api::ClashEndpoint,
    name: &str,
    url: &str,
    timeout_ms: u64,
) -> bool {
    (0..2).any(|_| matches!(crate::clash_api::delay(ep, name, url, timeout_ms), Ok(d) if d > 0))
}

/// One clash-mode failover cycle: if the group's current node is unreachable,
/// delay-test the pool and switch the group to the fastest live node.
fn clash_rotation_tick(settings: &ProxySettings) -> Option<String> {
    const PROBE_TIMEOUT_MS: u64 = 5000;
    let group = &settings.rotation.group;
    let pool = &settings.rotation.pool;
    if pool.is_empty() {
        return None;
    }
    let ep = resolve_clash_endpoint(settings)?;
    let url = crate::clash_api::DELAY_TEST_URL;

    // Current selection of the group.
    let proxies = crate::clash_api::proxies(&ep).ok()?;
    let now = proxies
        .get(group)
        .and_then(|g| g.get("now"))
        .and_then(|n| n.as_str())
        .map(str::to_string);

    // If the current node is in the pool, only fail over once it has missed its
    // health check on REPEATED ticks. Switching a selector changes the exit for
    // every connection routed through it, so a single slow probe must never
    // trigger a switch — that is precisely the "rotation dropped my network"
    // symptom. Retry within the tick (clash_node_alive) to absorb jitter, then
    // require ROTATION_FAILS_BEFORE_SWITCH consecutive failing ticks.
    if let Some(cur) = &now {
        if pool.contains(cur) {
            if clash_node_alive(&ep, cur, url, PROBE_TIMEOUT_MS) {
                rotation_reset_failure();
                return None; // healthy — stay put
            }
            let fails = rotation_note_failure(cur);
            if fails < ROTATION_FAILS_BEFORE_SWITCH {
                log::debug!(
                    "proxy rotation (clash): '{cur}' missed health check {fails}/{ROTATION_FAILS_BEFORE_SWITCH}, holding"
                );
                return None; // give it another tick before yanking the exit
            }
            log::info!(
                "proxy rotation (clash): '{cur}' failed {fails} consecutive checks → failing over"
            );
        }
    }

    // Pick the fastest live node in the pool.
    let mut best: Option<(String, u64)> = None;
    for name in pool {
        if let Ok(d) = crate::clash_api::delay(&ep, name, url, PROBE_TIMEOUT_MS) {
            if d > 0 && best.as_ref().map_or(true, |(_, b)| d < *b) {
                best = Some((name.clone(), d));
            }
        }
    }
    let (pick, ms) = best?;
    if now.as_deref() == Some(pick.as_str()) {
        rotation_reset_failure(); // current is still the fastest live node
        return None;
    }
    crate::clash_api::select(&ep, group, &pick).ok()?;
    rotation_reset_failure();
    log::info!("proxy auto-rotation (clash): group '{group}' → '{pick}' ({ms}ms)");
    Some(pick)
}

pub fn proxy_rotation_tick() -> Option<String> {
    const PROBE_TIMEOUT_MS: u64 = 5000;
    let mut state = mcp_state().lock();
    let mut settings = state.proxy.clone();
    if !settings.rotation.enabled {
        return None;
    }
    // Clash mode: rotation is driven through the proxy software's controller.
    if !settings.rotation.group.is_empty() {
        drop(state); // network I/O; don't hold the state lock
        return clash_rotation_tick(&settings);
    }
    if settings.rotation.pool.is_empty() {
        return None;
    }

    // Active node still reachable? Retry once to ride out transient jitter, and
    // require ROTATION_FAILS_BEFORE_SWITCH consecutive failing ticks before
    // swapping the injected proxy — a single missed probe must not flip the
    // exit out from under the user.
    if let Some((cur_name, cur_url)) = settings.current_node.as_ref().and_then(|cn| {
        settings
            .nodes
            .iter()
            .find(|n| &n.name == cn)
            .map(|n| (cn.clone(), n.url.clone()))
    }) {
        if (0..2).any(|_| probe_proxy_endpoint(&cur_url, PROBE_TIMEOUT_MS)) {
            rotation_reset_failure();
            return None; // healthy — nothing to do
        }
        let fails = rotation_note_failure(&cur_name);
        if fails < ROTATION_FAILS_BEFORE_SWITCH {
            log::debug!(
                "proxy rotation: '{cur_name}' missed health check {fails}/{ROTATION_FAILS_BEFORE_SWITCH}, holding"
            );
            return None;
        }
        log::info!("proxy rotation: '{cur_name}' failed {fails} consecutive checks → failing over");
    }

    // Active node down (or unset): probe the pool, pick the fastest reachable.
    let target = select_fastest_node(&settings.nodes, &settings.rotation.pool, |url| {
        let start = std::time::Instant::now();
        probe_proxy_endpoint(url, PROBE_TIMEOUT_MS).then(|| start.elapsed().as_millis() as u64)
    });
    let (name, url) = match target {
        Some(node) => (node.name.clone(), node.url.clone()),
        None => return None, // nothing in the pool is reachable — stay put
    };

    settings.enabled = true;
    settings.mode = "auto".to_string();
    settings.current_node = Some(name.clone());
    settings.http_proxy = Some(url.clone());
    if url.starts_with("socks") {
        settings.socks_proxy = Some(url.clone());
    }
    if save_proxy_settings(&settings).is_ok() {
        state.proxy = settings;
        rotation_reset_failure();
        log::info!("proxy auto-rotation: active node unreachable → switched to '{name}' ({url})");
        Some(name)
    } else {
        None
    }
}

/// Spawn the background auto-rotation monitor. Re-reads the interval each cycle
/// so live config changes take effect; cheap when disabled (a lock + two field
/// reads). Min 5s to avoid hammering. Started once at GUI startup.
pub fn start_proxy_rotation_monitor() {
    std::thread::Builder::new()
        .name("proxy-rotation".into())
        .spawn(|| loop {
            let interval = mcp_state().lock().proxy.rotation.interval_secs.max(10);
            std::thread::sleep(std::time::Duration::from_secs(interval));
            proxy_rotation_tick();
        })
        .ok();
}

#[cfg(windows)]
fn elevated_unterm_command_args(shell: &str) -> Result<Vec<String>> {
    let gui_exe = std::env::current_exe().context("resolve current Unterm GUI executable")?;
    let gui_exe = admin_launcher_exe(&gui_exe);
    let shell_args: Vec<String> = match shell.to_ascii_lowercase().as_str() {
        "powershell" | "windows-powershell" | "windows_powershell" => {
            vec!["powershell.exe".to_string(), "-NoLogo".to_string()]
        }
        "pwsh" | "powershell7" | "powershell-7" | "powershell_7" => {
            let pwsh = "C:\\Program Files\\PowerShell\\7\\pwsh.exe";
            if std::path::Path::new(pwsh).exists() {
                vec![pwsh.to_string(), "-NoLogo".to_string()]
            } else {
                vec!["powershell.exe".to_string(), "-NoLogo".to_string()]
            }
        }
        other => return Err(anyhow!("Unsupported elevated shell: {other}")),
    };

    let script = r#"
$exe = $args[0]
$argv = @()
if ($args.Length -gt 1) {
  $argv = $args[1..($args.Length - 1)]
}
Start-Process -Verb RunAs -FilePath $exe -ArgumentList $argv
"#;

    let mut args = vec![
        "powershell.exe".to_string(),
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-Command".to_string(),
        script.to_string(),
        gui_exe.display().to_string(),
        "start".to_string(),
        "--always-new-process".to_string(),
        "--".to_string(),
    ];
    args.extend(shell_args);
    Ok(args)
}

#[cfg(windows)]
fn admin_launcher_exe(gui_exe: &std::path::Path) -> std::path::PathBuf {
    let Some(dir) = gui_exe.parent() else {
        return gui_exe.to_path_buf();
    };
    let launcher = dir.join("Unterm.exe");
    let should_copy = match (std::fs::metadata(gui_exe), std::fs::metadata(&launcher)) {
        (Ok(src), Ok(dst)) => src.len() != dst.len() || src.modified().ok() != dst.modified().ok(),
        (Ok(_), Err(_)) => true,
        _ => false,
    };

    if should_copy {
        if let Err(err) = std::fs::copy(gui_exe, &launcher) {
            log::warn!(
                "failed to prepare Unterm.exe admin launcher at {}: {err:#}",
                launcher.display()
            );
        }
    }

    if launcher.exists() {
        launcher
    } else {
        gui_exe.to_path_buf()
    }
}

/// Extract new output by comparing before/after screen text
fn diff_output(before: &str, after: &str) -> String {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();

    // Find where they diverge
    let common_prefix = before_lines
        .iter()
        .zip(after_lines.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // New content is everything after the common prefix, minus the last line (prompt)
    let new_lines: Vec<&str> = after_lines[common_prefix..].to_vec();
    if new_lines.is_empty() {
        return String::new();
    }

    // Skip the command echo (first new line) and the new prompt (last line)
    let output_lines = if new_lines.len() > 2 {
        &new_lines[1..new_lines.len() - 1]
    } else if new_lines.len() > 1 {
        &new_lines[1..]
    } else {
        &new_lines[..]
    };

    output_lines.join("\n")
}

fn wait_wrapped_command(command: &str, shell_type: &str, marker: &str) -> String {
    match shell_type {
        "powershell" => format!("{}; Write-Output '{}'", command, marker),
        "cmd" => format!("{} & echo {}", command, marker),
        _ => format!("{}; echo {}", command, marker),
    }
}

fn extract_wait_output(before: &str, after: &str, command: &str, marker: &str) -> String {
    let diff = diff_output(before, after);
    let mut lines = Vec::new();

    for line in diff.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.contains(marker) {
            continue;
        }
        if trimmed.contains(command) {
            continue;
        }
        lines.push(trimmed.to_string());
    }

    if !lines.is_empty() {
        return lines.join("\n");
    }

    after
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.contains(marker))
        .filter(|line| !line.contains(command))
        .filter(|line| !before.contains(line))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Capture helpers — cross-platform plumbing
// ---------------------------------------------------------------------------

fn capture_output_dir() -> Result<std::path::PathBuf> {
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("screenshots");
    std::fs::create_dir_all(&dir).context("create screenshots output dir")?;
    Ok(dir)
}

fn append_base64_if_requested(mut value: Value, include_base64: bool) -> Result<Value> {
    if include_base64 {
        if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
            let bytes = std::fs::read(path)?;
            value["base64"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
        }
    }
    Ok(value)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn unix_command_exists(name: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_var) {
        if dir.join(name).is_file() {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Cross-platform clipboard.read entry point
// ---------------------------------------------------------------------------

fn clipboard_read_any() -> Result<Value> {
    #[cfg(windows)]
    {
        return clipboard_read_win32();
    }
    #[cfg(target_os = "macos")]
    {
        return clipboard_read_macos();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return clipboard_read_linux();
    }
    #[allow(unreachable_code)]
    Err(anyhow!(
        "Clipboard reading is not supported on this platform"
    ))
}

// ---------------------------------------------------------------------------
// Windows implementations (PowerShell + Win32 API)
// ---------------------------------------------------------------------------

/// Read clipboard content using Win32 API.
/// Supports both text (CF_UNICODETEXT) and image (CF_DIB) formats.
/// IMPORTANT: Do NOT use PowerShell for clipboard access — it steals window focus.
#[cfg(windows)]
fn clipboard_read_win32() -> Result<Value> {
    use std::ptr;
    use winapi::shared::minwindef::HGLOBAL;
    use winapi::um::winbase::{GlobalLock, GlobalSize, GlobalUnlock};
    use winapi::um::wingdi::BITMAPINFOHEADER;
    use winapi::um::winuser::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, CF_DIB,
        CF_UNICODETEXT,
    };

    let has_image = unsafe { IsClipboardFormatAvailable(CF_DIB as u32) != 0 };
    let has_text = unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT as u32) != 0 };

    if !has_image && !has_text {
        return Err(anyhow!("Clipboard is empty or contains unsupported format"));
    }

    let opened = unsafe { OpenClipboard(ptr::null_mut()) };
    if opened == 0 {
        return Err(anyhow!(
            "Failed to open clipboard (it may be locked by another application)"
        ));
    }

    struct ClipboardGuard;
    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
    let _guard = ClipboardGuard;

    if has_image {
        let handle: HGLOBAL = unsafe { GetClipboardData(CF_DIB as u32) as HGLOBAL };
        if !handle.is_null() {
            let ptr = unsafe { GlobalLock(handle) };
            if ptr.is_null() {
                return Err(anyhow!("GlobalLock failed on clipboard DIB data"));
            }

            let data_size = unsafe { GlobalSize(handle) };
            if data_size < std::mem::size_of::<BITMAPINFOHEADER>() {
                unsafe {
                    GlobalUnlock(handle);
                }
                return Err(anyhow!("Clipboard DIB data too small"));
            }

            let bih = unsafe { &*(ptr as *const BITMAPINFOHEADER) };
            let width = bih.biWidth as u32;
            let height_signed = bih.biHeight;
            let height = height_signed.unsigned_abs();
            let bit_count = bih.biBitCount;
            let compression = bih.biCompression;

            if compression != 0 {
                unsafe {
                    GlobalUnlock(handle);
                }
                return Err(anyhow!(
                    "Unsupported DIB compression: {}. Only uncompressed (BI_RGB) is supported.",
                    compression
                ));
            }

            if bit_count != 24 && bit_count != 32 {
                unsafe {
                    GlobalUnlock(handle);
                }
                return Err(anyhow!(
                    "Unsupported DIB bit depth: {}. Only 24-bit and 32-bit are supported.",
                    bit_count
                ));
            }

            let bytes_per_pixel = (bit_count / 8) as usize;
            let row_stride = ((width as usize * bytes_per_pixel + 3) / 4) * 4;
            let header_size = bih.biSize as usize;
            let pixel_offset = header_size;
            let total_pixel_bytes = row_stride * height as usize;

            if pixel_offset + total_pixel_bytes > data_size {
                unsafe {
                    GlobalUnlock(handle);
                }
                return Err(anyhow!("DIB pixel data exceeds clipboard buffer size"));
            }

            let pixel_data = unsafe {
                std::slice::from_raw_parts((ptr as *const u8).add(pixel_offset), total_pixel_bytes)
            };

            let mut rgba_buf = vec![0u8; (width * height * 4) as usize];
            let bottom_up = height_signed > 0;

            for y in 0..height as usize {
                let src_y = if bottom_up {
                    height as usize - 1 - y
                } else {
                    y
                };
                let src_row = &pixel_data
                    [src_y * row_stride..src_y * row_stride + width as usize * bytes_per_pixel];
                let dst_offset = y * width as usize * 4;

                for x in 0..width as usize {
                    let si = x * bytes_per_pixel;
                    let di = dst_offset + x * 4;
                    rgba_buf[di] = src_row[si + 2];
                    rgba_buf[di + 1] = src_row[si + 1];
                    rgba_buf[di + 2] = src_row[si];
                    rgba_buf[di + 3] = if bytes_per_pixel == 4 {
                        src_row[si + 3]
                    } else {
                        255
                    };
                }
            }

            unsafe {
                GlobalUnlock(handle);
            }

            let img = image::RgbaImage::from_raw(width, height, rgba_buf)
                .ok_or_else(|| anyhow!("Failed to create image buffer from DIB data"))?;

            let clipboard_dir = dirs_next::home_dir()
                .unwrap_or_default()
                .join(".unterm")
                .join("clipboard");
            std::fs::create_dir_all(&clipboard_dir)
                .context("Failed to create clipboard output directory")?;

            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
            let filename = format!("clipboard_{}.png", timestamp);
            let file_path = clipboard_dir.join(&filename);

            img.save(&file_path)
                .context("Failed to save clipboard image as PNG")?;

            let path_str = file_path.to_string_lossy().to_string();
            let png_bytes = std::fs::read(&file_path)
                .context("Failed to read saved clipboard PNG for base64 encoding")?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

            return Ok(json!({
                "type": "image",
                "format": "png",
                "image_path": path_str,
                "width": width,
                "height": height,
                "bit_depth": bit_count,
                "size_bytes": png_bytes.len(),
                "base64": b64,
            }));
        }
    }

    if has_text {
        let handle: HGLOBAL = unsafe { GetClipboardData(CF_UNICODETEXT as u32) as HGLOBAL };
        if handle.is_null() {
            return Err(anyhow!("GetClipboardData(CF_UNICODETEXT) returned NULL"));
        }

        let ptr = unsafe { GlobalLock(handle) };
        if ptr.is_null() {
            return Err(anyhow!("GlobalLock failed on clipboard text data"));
        }

        let wchar_ptr = ptr as *const u16;
        let mut len = 0usize;
        unsafe {
            while *wchar_ptr.add(len) != 0 {
                len += 1;
            }
        }
        let wstr = unsafe { std::slice::from_raw_parts(wchar_ptr, len) };
        let text = String::from_utf16_lossy(wstr);

        unsafe {
            GlobalUnlock(handle);
        }

        return Ok(json!({"type": "text", "content": text}));
    }

    Err(anyhow!("Clipboard is empty"))
}

#[cfg(windows)]
fn ps_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(windows)]
fn run_powershell_json(script: &str) -> Result<Value> {
    let script = format!(
        "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)\n$OutputEncoding = [Console]::OutputEncoding\n{}",
        script
    );
    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut command = std::process::Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &encoded,
    ]);
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().context("run PowerShell capture helper")?;
    if !output.status.success() {
        return Err(anyhow!(
            "PowerShell helper failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value =
        serde_json::from_str(stdout.trim()).context("parse PowerShell helper JSON output")?;
    Ok(value)
}

#[cfg(windows)]
fn capture_screen_image(include_base64: bool) -> Result<Value> {
    let path = capture_output_dir()?.join(format!(
        "screen_{}.png",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));
    let path = path.display().to_string();
    let qpath = ps_single_quote(&path);
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
$bmp.Save({qpath}, [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()
[pscustomobject]@{{
  path = {qpath}
  width = $bounds.Width
  height = $bounds.Height
  left = $bounds.Left
  top = $bounds.Top
}} | ConvertTo-Json -Compress
"#
    );
    append_base64_if_requested(run_powershell_json(&script)?, include_base64)
}

#[cfg(windows)]
fn capture_window_image(
    title_filter: Option<&str>,
    pid_filter: Option<u32>,
    include_base64: bool,
) -> Result<Value> {
    let path = capture_output_dir()?.join(format!(
        "window_{}.png",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));
    let path = path.display().to_string();
    let qpath = ps_single_quote(&path);
    let title = title_filter
        .map(ps_single_quote)
        .unwrap_or_else(|| "$null".to_string());
    let pid = pid_filter.unwrap_or_else(std::process::id);
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class UntermCapture {{
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
}}
public struct RECT {{ public int Left; public int Top; public int Right; public int Bottom; }}
"@
$pidFilter = {pid}
$titleFilter = {title}
if ($titleFilter -ne $null) {{
  $proc = Get-Process | Where-Object {{ $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -like "*$titleFilter*" }} | Select-Object -First 1
}} else {{
  $proc = Get-Process -Id $pidFilter -ErrorAction Stop
}}
if ($null -eq $proc -or $proc.MainWindowHandle -eq 0) {{ throw "No matching window found" }}
[UntermCapture]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 150
$rect = New-Object RECT
[UntermCapture]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
if ($width -le 0 -or $height -le 0) {{ throw "Invalid window bounds" }}
$bmp = New-Object System.Drawing.Bitmap $width, $height
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$bmp.Save({qpath}, [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()
[pscustomobject]@{{
  path = {qpath}
  width = $width
  height = $height
  left = $rect.Left
  top = $rect.Top
  pid = $proc.Id
  title = $proc.MainWindowTitle
}} | ConvertTo-Json -Compress
"#
    );
    append_base64_if_requested(run_powershell_json(&script)?, include_base64)
}

// ---------------------------------------------------------------------------
// macOS implementations (screencapture + osascript)
// ---------------------------------------------------------------------------

/// Read PNG dimensions cheaply (no full decode) for capture metadata.
/// Falls back to 0x0 if anything goes wrong — purely informational.
#[cfg(unix)]
fn png_dimensions(path: &std::path::Path) -> (u32, u32) {
    use std::io::Read;
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (0, 0),
    };
    let mut header = [0u8; 24];
    if file.read_exact(&mut header).is_err() {
        return (0, 0);
    }
    // PNG: 8-byte signature + 4-byte length + 4-byte "IHDR" + 4-byte width + 4-byte height
    if &header[0..8] != b"\x89PNG\r\n\x1a\n" || &header[12..16] != b"IHDR" {
        return (0, 0);
    }
    let w = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let h = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    (w, h)
}

#[cfg(target_os = "macos")]
fn capture_screen_image(include_base64: bool) -> Result<Value> {
    let dir = capture_output_dir()?;
    let path = dir.join(format!(
        "screen_{}.png",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));

    // -x = no shutter sound, -t png = explicit format
    let status = std::process::Command::new("/usr/sbin/screencapture")
        .args(["-x", "-t", "png"])
        .arg(&path)
        .status()
        .context("invoke /usr/sbin/screencapture")?;
    if !status.success() {
        return Err(anyhow!("screencapture exited with {status}"));
    }
    if !path.exists() {
        return Err(anyhow!("screencapture did not produce {}", path.display()));
    }

    let (width, height) = png_dimensions(&path);
    let value = json!({
        "path": path.display().to_string(),
        "width": width,
        "height": height,
        "left": 0,
        "top": 0,
    });
    append_base64_if_requested(value, include_base64)
}

#[cfg(target_os = "macos")]
fn capture_window_image(
    title_filter: Option<&str>,
    pid_filter: Option<u32>,
    include_base64: bool,
) -> Result<Value> {
    // macOS `screencapture -l <CGWindowID>` captures a specific window without
    // any UI. We just have to translate (pid, title) → CGWindowID via
    // CGWindowListCopyWindowInfo. If neither filter is supplied, fall back
    // to the calling process's own pid (i.e. capture this Unterm window).
    let target_pid = pid_filter.unwrap_or_else(std::process::id);
    let is_self = target_pid == std::process::id();
    // Self-capture race: right after window creation, the NSWindow exists
    // but CGWindowList may not yet flag it onScreen, so find returns None
    // and the caller silently gets a full-screen fallback instead of their
    // own chrome. For self-targets we briefly retry — five 120 ms ticks
    // (~600 ms ceiling) covers every startup we've measured. External-pid
    // captures still single-shot to avoid blocking on a wrong pid.
    let attempts = if is_self { 5 } else { 1 };
    let mut window_id_opt = None;
    let mut last_err = None;
    for attempt in 0..attempts {
        match find_cg_window_id(target_pid, title_filter) {
            Ok(Some(id)) => {
                window_id_opt = Some(id);
                break;
            }
            Ok(None) => {
                if attempt + 1 < attempts {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                }
            }
            Err(err) => {
                last_err = Some(err);
                break;
            }
        }
    }
    if let Some(err) = last_err {
        return Err(err);
    }
    let window_id = match window_id_opt {
        Some(id) => id,
        None => {
            // No matching on-screen window — degrade to full-screen capture so
            // the caller still gets pixels rather than an error.
            let mut value = capture_screen_image(false)?;
            if let Some(obj) = value.as_object_mut() {
                obj.insert("mode".into(), json!("screen_fallback"));
                obj.insert(
                    "note".into(),
                    json!(format!(
                        "no on-screen window matched pid={} title={:?}; returned full screen",
                        target_pid, title_filter
                    )),
                );
            }
            return append_base64_if_requested(value, include_base64);
        }
    };

    let dir = capture_output_dir()?;
    let path = dir.join(format!(
        "window_{}.png",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));
    let status = std::process::Command::new("/usr/sbin/screencapture")
        .args(["-x", "-o", "-t", "png", "-l", &window_id.to_string()])
        .arg(&path)
        .status()
        .context("invoke /usr/sbin/screencapture -l")?;
    if !status.success() || !path.exists() {
        return Err(anyhow!(
            "screencapture -l {} failed (status {:?}, path exists: {})",
            window_id,
            status.code(),
            path.exists()
        ));
    }

    let (width, height) = png_dimensions(&path);
    let value = json!({
        "path": path.display().to_string(),
        "width": width,
        "height": height,
        "pid": target_pid,
        "window_id": window_id,
    });
    append_base64_if_requested(value, include_base64)
}

/// Return the first on-screen CGWindowID belonging to `pid` (and optionally
/// containing `title_substr`), or None if no match.
#[cfg(target_os = "macos")]
fn find_cg_window_id(pid: u32, title_substr: Option<&str>) -> Result<Option<u32>> {
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::display::{
        kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
        CGWindowListCopyWindowInfo,
    };

    let info: CFArray<CFDictionary<CFString, CFType>> = unsafe {
        let raw = CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
            kCGNullWindowID,
        );
        if raw.is_null() {
            return Ok(None);
        }
        CFArray::wrap_under_create_rule(raw)
    };

    let pid_key = CFString::from_static_string("kCGWindowOwnerPID");
    let id_key = CFString::from_static_string("kCGWindowNumber");
    let name_key = CFString::from_static_string("kCGWindowName");

    for entry in info.iter() {
        let owner_pid: i64 = entry
            .find(&pid_key)
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64())
            .unwrap_or(-1);
        if owner_pid as u32 != pid {
            continue;
        }
        if let Some(needle) = title_substr {
            let window_title = entry
                .find(&name_key)
                .and_then(|v| v.downcast::<CFString>())
                .map(|s| s.to_string())
                .unwrap_or_default();
            if !window_title.contains(needle) {
                continue;
            }
        }
        let window_id: i64 = entry
            .find(&id_key)
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64())
            .unwrap_or(0);
        if window_id > 0 {
            return Ok(Some(window_id as u32));
        }
    }
    Ok(None)
}

#[cfg(target_os = "macos")]
fn clipboard_read_macos() -> Result<Value> {
    // 1) If pasteboard contains an image, write it to a PNG and return.
    //    osascript exits non-zero when the cast to PNGf fails (i.e. no image).
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("clipboard");
    std::fs::create_dir_all(&dir).context("create clipboard output dir")?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
    let png_path = dir.join(format!("clipboard_{}.png", timestamp));

    let script = format!(
        "try\n  set theData to the clipboard as «class PNGf»\n  set fp to open for access POSIX file \"{}\" with write permission\n  set eof of fp to 0\n  write theData to fp\n  close access fp\n  return \"image\"\non error\n  try\n    close access fp\n  end try\n  return \"none\"\nend try",
        png_path.display()
    );
    let out = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("invoke osascript for clipboard image probe")?;
    let kind = String::from_utf8_lossy(&out.stdout).trim().to_string();

    if kind == "image" && png_path.exists() {
        let png_bytes = std::fs::read(&png_path)?;
        let (width, height) = png_dimensions(&png_path);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        return Ok(json!({
            "type": "image",
            "format": "png",
            "image_path": png_path.display().to_string(),
            "width": width,
            "height": height,
            "size_bytes": png_bytes.len(),
            "base64": b64,
        }));
    }

    // Cleanup empty file if osascript wrote a zero-byte file before failing
    if png_path.exists() {
        let _ = std::fs::remove_file(&png_path);
    }

    // 2) Fall back to text via pbpaste.
    let out = std::process::Command::new("pbpaste")
        .output()
        .context("invoke pbpaste")?;
    if !out.status.success() {
        return Err(anyhow!(
            "pbpaste failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.is_empty() {
        return Err(anyhow!("Clipboard is empty"));
    }
    Ok(json!({"type": "text", "content": text}))
}

// ---------------------------------------------------------------------------
// Linux implementations (probe grim/gnome-screenshot/spectacle/scrot/maim)
// ---------------------------------------------------------------------------

#[cfg(all(unix, not(target_os = "macos")))]
fn capture_screen_image(include_base64: bool) -> Result<Value> {
    let dir = capture_output_dir()?;
    let path = dir.join(format!(
        "screen_{}.png",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));
    let path_str = path.display().to_string();

    // Probe each tool in order; the first that exists is used. No interactive
    // selection — we want a full-screen PNG.
    let candidates: &[(&str, &[&str])] = &[
        ("grim", &[]),                       // grim <file>
        ("gnome-screenshot", &["-f"]),       // gnome-screenshot -f <file>
        ("spectacle", &["-bn", "-f", "-o"]), // spectacle -bn -f -o <file>
        ("scrot", &[]),                      // scrot <file>
        ("maim", &[]),                       // maim <file>
    ];

    let mut last_err: Option<String> = None;
    for (tool, args) in candidates {
        if !unix_command_exists(tool) {
            continue;
        }
        let mut cmd = std::process::Command::new(tool);
        cmd.args(*args);
        cmd.arg(&path_str);
        match cmd.status() {
            Ok(s) if s.success() && path.exists() => {
                let (width, height) = png_dimensions(&path);
                let value = json!({
                    "path": path_str,
                    "width": width,
                    "height": height,
                    "left": 0,
                    "top": 0,
                });
                return append_base64_if_requested(value, include_base64);
            }
            Ok(s) => last_err = Some(format!("{tool} exited with {s}")),
            Err(e) => last_err = Some(format!("failed to run {tool}: {e}")),
        }
    }

    Err(anyhow!(
        "{}",
        last_err.unwrap_or_else(|| "No screenshot tool found. Install one of: grim, gnome-screenshot, spectacle, scrot, or maim".into())
    ))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn capture_window_image(
    _title_filter: Option<&str>,
    _pid_filter: Option<u32>,
    include_base64: bool,
) -> Result<Value> {
    // We don't have a robust cross-WM way to locate a window by pid/title
    // without xdotool/wmctrl + a lot of glue. For headless MCP we fall back
    // to a full-screen capture so the caller still gets pixels.
    let mut value = capture_screen_image(false)?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert("mode".into(), json!("screen_fallback"));
        obj.insert(
            "note".into(),
            json!("Linux MCP capture currently falls back to full screen; install xdotool/wmctrl to wire window-pick up if needed"),
        );
    }
    append_base64_if_requested(value, include_base64)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn clipboard_read_linux() -> Result<Value> {
    // 1) Try image via wl-paste / xclip first.
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("clipboard");
    std::fs::create_dir_all(&dir).context("create clipboard output dir")?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
    let png_path = dir.join(format!("clipboard_{}.png", timestamp));

    let mut got_image = false;

    if unix_command_exists("wl-paste") {
        // Check available types; if image/png is offered, save it.
        let types = std::process::Command::new("wl-paste")
            .args(["--list-types"])
            .output();
        if let Ok(out) = types {
            let listed = String::from_utf8_lossy(&out.stdout);
            if listed.lines().any(|t| t.trim() == "image/png") {
                let f = std::fs::File::create(&png_path)?;
                let status = std::process::Command::new("wl-paste")
                    .args(["--type", "image/png"])
                    .stdout(f)
                    .status()?;
                if status.success() && png_path.exists() && std::fs::metadata(&png_path)?.len() > 0
                {
                    got_image = true;
                }
            }
        }
    }

    if !got_image && unix_command_exists("xclip") {
        // xclip -selection clipboard -t TARGETS -o -> list of mime types
        let targets = std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-t", "TARGETS", "-o"])
            .output();
        if let Ok(out) = targets {
            let listed = String::from_utf8_lossy(&out.stdout);
            if listed.lines().any(|t| t.trim() == "image/png") {
                let f = std::fs::File::create(&png_path)?;
                let status = std::process::Command::new("xclip")
                    .args(["-selection", "clipboard", "-t", "image/png", "-o"])
                    .stdout(f)
                    .status()?;
                if status.success() && png_path.exists() && std::fs::metadata(&png_path)?.len() > 0
                {
                    got_image = true;
                }
            }
        }
    }

    if got_image {
        let png_bytes = std::fs::read(&png_path)?;
        let (width, height) = png_dimensions(&png_path);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        return Ok(json!({
            "type": "image",
            "format": "png",
            "image_path": png_path.display().to_string(),
            "width": width,
            "height": height,
            "size_bytes": png_bytes.len(),
            "base64": b64,
        }));
    }

    if png_path.exists() {
        let _ = std::fs::remove_file(&png_path);
    }

    // 2) Text fallback.
    let text_cmds: &[(&str, &[&str])] = &[
        ("wl-paste", &["--no-newline"]),
        ("xclip", &["-selection", "clipboard", "-o"]),
        ("xsel", &["--clipboard", "--output"]),
    ];
    for (cmd, args) in text_cmds {
        if !unix_command_exists(cmd) {
            continue;
        }
        let out = std::process::Command::new(cmd).args(*args).output();
        if let Ok(out) = out {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                if !text.is_empty() {
                    return Ok(json!({"type": "text", "content": text}));
                }
            }
        }
    }

    Err(anyhow!(
        "Clipboard is empty or no clipboard tool available (install wl-clipboard, xclip, or xsel)"
    ))
}
