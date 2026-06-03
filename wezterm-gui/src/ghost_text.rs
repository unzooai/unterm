//! Fish-style "ghost text" — predicts what the user is about to type
//! based on recent command-line activity, and renders the prediction
//! in dim grey to the right of the cursor. Pressing the
//! `AcceptGhostText` action (default Right Arrow) writes the
//! prediction to the PTY; any other key dismisses or refines it.
//!
//! ## Data model
//!
//! The module keeps a per-pane "current input buffer" — the
//! characters the user has typed since the most recent Enter /
//! Ctrl-C / Ctrl-U / etc. It is a *best-effort heuristic*:
//! we never read shell state directly, so a `cd ..` invoked via a
//! key macro or paste won't shift the buffer. Acceptable trade-off
//! for now; shell-integration (OSC 133) can replace this later.
//!
//! ## Candidate pool
//!
//! Lines from the active pane's scrollback. We treat each non-empty
//! line as a possible command and match by prefix against the
//! current buffer. This isn't perfect (scrollback contains output,
//! not just commands), but it's local, free, and improves
//! immediately as the user works.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// What the user did with a key. Drives state transitions in the
/// per-pane input buffer. We accept the slight imprecision
/// (no exact mapping to terminal semantics) in exchange for not
/// needing to parse the shell.
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    /// A printable character the user typed.
    Char(char),
    /// Enter — buffer commits, becomes a candidate, then resets.
    Enter,
    /// Backspace — last char removed from buffer.
    Backspace,
    /// Ctrl-C, Ctrl-G, Esc — drop the buffer without committing.
    Cancel,
    /// Ctrl-U / Ctrl-W — clear the line wholesale.
    ClearLine,
}

#[derive(Debug, Default, Clone)]
struct PaneGhostState {
    /// Characters the user has typed since the last commit / cancel.
    input: String,
    /// Cached candidate matching `input` as a prefix. Re-computed
    /// every time `input` changes; cleared when no candidate fits.
    ghost: Option<String>,
    /// Commits recorded from this pane (oldest → newest), capped to
    /// MAX_COMMITS so a long-running pane doesn't grow forever.
    commits: Vec<String>,
}

const MAX_COMMITS: usize = 256;
const MAX_GLOBAL_COMMITS: usize = 512;
const MAX_INPUT_LEN: usize = 1024;
const MAX_CANDIDATES_SCANNED: usize = 4096;

/// Process-wide registry. One entry per pane id. Survives across
/// pane resizes and rendering passes — only goes away when the pane
/// closes (currently never cleaned up; the leak is bounded because
/// pane ids don't recycle within a process lifetime in practice).
fn registry() -> &'static Mutex<HashMap<u64, PaneGhostState>> {
    static REG: OnceLock<Mutex<HashMap<u64, PaneGhostState>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Cross-pane command history. Every Enter-commit from any pane is
/// appended here in addition to the pane's own pool, giving fresh
/// panes a non-empty prediction source the moment they open. Capped
/// at MAX_GLOBAL_COMMITS — oldest dropped first.
///
/// Lock order: always acquire `registry()` BEFORE `global_commits()`
/// to avoid deadlock with concurrent observe / debug callers.
fn global_commits() -> &'static Mutex<Vec<String>> {
    static G: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    G.get_or_init(|| Mutex::new(Vec::new()))
}

fn push_global_commit(cmd: &str) {
    let mut g = global_commits().lock();
    if g.last().map_or(true, |last| last != cmd) {
        g.push(cmd.to_string());
        if g.len() > MAX_GLOBAL_COMMITS {
            let drop = MAX_GLOBAL_COMMITS / 8;
            g.drain(..drop);
        }
    }
}

fn snapshot_global_commits() -> Vec<String> {
    global_commits().lock().clone()
}

/// Record a keyboard event on `pane_id`. `external_candidates` is
/// merged with the pane's own commit history when recomputing the
/// ghost — caller typically passes recent scrollback lines so the
/// pool reflects what's on screen, but it's optional.
///
/// In real wezterm dispatch, `Key::Code` and `Key::Composed` are
/// mutually exclusive — a physical key fires exactly one of them —
/// so observing in both branches still results in exactly one call
/// per keystroke. (Test harnesses using `PostMessage(WM_KEYDOWN)`
/// can produce duplicated dispatches; that's a test artifact, not
/// a real-user concern.)
pub fn observe(pane_id: u64, event: InputEvent, external_candidates: &[String]) {
    let mut reg = registry().lock();
    let state = reg.entry(pane_id).or_default();
    match event {
        InputEvent::Char(c) => {
            if state.input.len() < MAX_INPUT_LEN {
                state.input.push(c);
            }
        }
        InputEvent::Backspace => {
            state.input.pop();
        }
        InputEvent::Cancel | InputEvent::ClearLine => {
            state.input.clear();
        }
        InputEvent::Enter => {
            let committed = std::mem::take(&mut state.input);
            let trimmed = committed.trim();
            if !trimmed.is_empty() {
                if state.commits.last().map_or(true, |last| last != trimmed) {
                    state.commits.push(trimmed.to_string());
                    if state.commits.len() > MAX_COMMITS {
                        let drop = MAX_COMMITS / 8;
                        state.commits.drain(..drop);
                    }
                }
                // Share commits cross-pane so newly-opened panes have
                // a non-empty prediction source from day one. Honours
                // the documented lock order (registry already held).
                push_global_commit(trimmed);
            }
        }
    }
    recompute_ghost(state, external_candidates);
}

/// Drop the current input buffer without committing it. Called from
/// the key-event path when the user presses Up/Down to navigate
/// shell history — shell PSReadLine rewrites the visible line, but
/// we don't see what it wrote, so the safest move is to start fresh.
/// Idempotent; safe to call on a pane that has no recorded state.
pub fn cancel_input(pane_id: u64) {
    let mut reg = registry().lock();
    let Some(state) = reg.get_mut(&pane_id) else {
        return;
    };
    state.input.clear();
    state.ghost = None;
}

/// Best ghost continuation for `pane_id` — the substring to render
/// in dim grey to the right of the cursor. Returns `None` when the
/// buffer is empty, no candidate matches, or the buffer already
/// equals a full candidate.
pub fn current_ghost(pane_id: u64) -> Option<(String, String)> {
    let reg = registry().lock();
    let state = reg.get(&pane_id)?;
    let ghost = state.ghost.as_ref()?;
    Some((state.input.clone(), ghost.clone()))
}

/// True when the active ghost should respond to Right Arrow / End.
/// Used by the key dispatcher: if there's no ghost the action
/// returns `Unhandled` so the arrow key still moves the cursor.
pub fn has_pending_ghost(pane_id: u64) -> bool {
    let reg = registry().lock();
    reg.get(&pane_id)
        .map(|s| s.ghost.as_deref().map_or(false, |g| !g.is_empty()))
        .unwrap_or(false)
}

/// Take the ghost and the corresponding input, returning the bytes
/// that should be written to the PTY (just the ghost continuation —
/// the user already typed the prefix). After accepting, the buffer
/// is updated as if the user had typed the rest themselves; if the
/// next key is Enter the full command will commit normally.
pub fn accept(pane_id: u64) -> Option<String> {
    let mut reg = registry().lock();
    let state = reg.get_mut(&pane_id)?;
    let ghost = state.ghost.take()?;
    if ghost.is_empty() {
        return None;
    }
    state.input.push_str(&ghost);
    Some(ghost)
}

/// Refresh the cached ghost against a freshly-provided candidate
/// pool. Useful when the caller knows the scrollback changed (e.g.
/// a `screen.read` returned new lines) but no key event happened.
pub fn refresh_candidates(pane_id: u64, external_candidates: &[String]) {
    let mut reg = registry().lock();
    let state = reg.entry(pane_id).or_default();
    recompute_ghost(state, external_candidates);
}

/// Most-frequently-committed commands across all panes since
/// startup. Returns up to `limit` entries sorted by descending
/// count. Used by the Insights overlay to show "you keep typing
/// these — maybe set up an alias".
pub fn commit_frequency(limit: usize) -> Vec<(String, u32)> {
    use std::collections::HashMap;
    let g = global_commits().lock();
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for cmd in g.iter() {
        *counts.entry(cmd.as_str()).or_insert(0) += 1;
    }
    let mut ranked: Vec<(String, u32)> = counts
        .into_iter()
        .filter(|(_, c)| *c >= 2)
        .map(|(s, c)| (s.to_string(), c))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(limit);
    ranked
}

/// Most-recent commits across all panes. Returns up to `limit`
/// entries, newest first. Pane-scoped recent commits are visible in
/// `debug_snapshot`; this is the cross-pane view used by Insights.
pub fn recent_global_commits(limit: usize) -> Vec<String> {
    let g = global_commits().lock();
    g.iter().rev().take(limit).cloned().collect()
}

/// Diagnostic snapshot of the ghost-text state for a pane. Returned
/// by the `ghost.debug` MCP method so a remote debugger can verify
/// "is the buffer growing as I type?", "are commits landing in the
/// pool?", "why isn't a ghost showing up?".
#[derive(serde::Serialize)]
pub struct DebugSnapshot {
    pub input_buffer: String,
    pub input_buffer_len: usize,
    pub ghost: Option<String>,
    pub commit_count: usize,
    pub recent_commits: Vec<String>,
    pub global_commit_count: usize,
    pub recent_global_commits: Vec<String>,
}

/// Take a snapshot of the named pane's ghost state. Returns `None`
/// when the pane has never been seen by the observer (no key events
/// recorded for it). Also includes the global cross-pane commit
/// pool, since that's a candidate source the predictor checks
/// alongside the pane-local pool.
pub fn debug_snapshot(pane_id: u64) -> Option<DebugSnapshot> {
    let reg = registry().lock();
    let state = reg.get(&pane_id)?;
    let recent: Vec<String> = state
        .commits
        .iter()
        .rev()
        .take(10)
        .cloned()
        .collect();
    let global = global_commits().lock();
    let global_recent: Vec<String> = global.iter().rev().take(10).cloned().collect();
    let global_count = global.len();
    drop(global);
    Some(DebugSnapshot {
        input_buffer: state.input.clone(),
        input_buffer_len: state.input.chars().count(),
        ghost: state.ghost.clone(),
        commit_count: state.commits.len(),
        recent_commits: recent,
        global_commit_count: global_count,
        recent_global_commits: global_recent,
    })
}

fn recompute_ghost(state: &mut PaneGhostState, external: &[String]) {
    if state.input.is_empty() {
        state.ghost = None;
        return;
    }
    let prefix = &state.input;
    // Search newest-first: a freshly-typed command is the strongest
    // signal of what the user is about to retype.
    let mut best: Option<String> = None;
    // Priority: pane-local > cross-pane global > caller-supplied
    // external pool. Pane-local wins because the user has just been
    // working in this pane; their last commands are the strongest
    // signal of what they're about to retype.
    //
    // We snapshot the global pool to a local Vec to avoid holding
    // its lock across the loop (registry lock is already held).
    let global_snapshot = snapshot_global_commits();
    let candidates = state
        .commits
        .iter()
        .rev()
        .chain(global_snapshot.iter().rev())
        .chain(external.iter().rev())
        .take(MAX_CANDIDATES_SCANNED);
    for candidate in candidates {
        if candidate.len() <= prefix.len() {
            continue;
        }
        if !candidate.starts_with(prefix.as_str()) {
            continue;
        }
        let rest = &candidate[prefix.len()..];
        if rest.is_empty() {
            continue;
        }
        best = Some(rest.to_string());
        break;
    }
    // Fallback: if shell history didn't predict anything and the user is
    // typing a known AI coding-CLI, offer a flag completion from that CLI's
    // manifest flag_catalog (e.g. `gemini --sk` → `ip-trust`). History wins
    // when present so a command you actually ran still takes priority.
    if best.is_none() {
        best = agent_flag_ghost(prefix);
    }
    // Last fallback: complete the AI agent's *name* itself from the manifest,
    // so `cla` → `claude` even on a fresh shell you've never run it in. Without
    // this, command completion only kicked in for agents already in history,
    // which is why it felt like it "didn't work for everything".
    if best.is_none() {
        best = agent_exec_ghost(prefix);
    }
    state.ghost = best;
}

/// Map of agent exec name → flag completion tokens, built lazily from the
/// offline manifest set (baked / on-disk cache — never hits the network).
/// `OnceLock` so the disk read happens at most once per process; flag
/// catalogs don't change mid-session in practice.
fn agent_flag_tokens() -> &'static HashMap<String, Vec<String>> {
    static MAP: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        // 1) Whatever the signed manifest set offers (may be a small subset).
        if let Ok(set) = unterm_agents::fetch_manifests_offline() {
            for manifest in set.for_current_platform() {
                let toks: Vec<String> = manifest
                    .launch
                    .flag_catalog
                    .iter()
                    .map(|f| flag_completion_token(&f.arg))
                    .collect();
                if !toks.is_empty() {
                    m.entry(manifest.launch.exec.clone())
                        .or_default()
                        .extend(toks);
                }
            }
        }
        // 2) Merge a built-in, comprehensive flag set per known agent so
        // argument completion is full — the manifest's flag_catalog only ever
        // carried a handful, which is why completion felt partial. Independent
        // of the signed manifest (no re-signing needed).
        for (exec, args) in BUILTIN_AGENT_FLAGS {
            let bucket = m.entry((*exec).to_string()).or_default();
            for arg in *args {
                let tok = flag_completion_token(arg);
                if !bucket.contains(&tok) {
                    bucket.push(tok);
                }
            }
        }
        m
    })
}

/// Built-in flag templates per AI coding-CLI, kept current with the tools'
/// documented options. `{value}` marks a flag that takes an argument (the
/// completion stops at the space, ready for the value). Order matters only as
/// a tie-break for which single ghost we surface for a bare `--` prefix.
const BUILTIN_AGENT_FLAGS: &[(&str, &[&str])] = &[
    (
        "claude",
        &[
            "--continue",
            "--resume {value}",
            "--print",
            "--model {value}",
            "--permission-mode {value}",
            "--dangerously-skip-permissions",
            "--add-dir {value}",
            "--allowed-tools {value}",
            "--disallowed-tools {value}",
            "--mcp-config {value}",
            "--strict-mcp-config",
            "--append-system-prompt {value}",
            "--output-format {value}",
            "--input-format {value}",
            "--settings {value}",
            "--session-id {value}",
            "--fork-session",
            "--agents {value}",
            "--ide",
            "--verbose",
            "--debug",
            "--help",
            "--version",
        ],
    ),
    (
        "codex",
        &[
            "--model {value}",
            "--ask-for-approval {value}",
            "--sandbox {value}",
            "--config {value}",
            "--cd {value}",
            "--profile {value}",
            "--image {value}",
            "--full-auto",
            "--dangerously-bypass-approvals-and-sandbox",
            "--search",
            "--oss",
            "--help",
            "--version",
        ],
    ),
    (
        "gemini",
        &[
            "--model {value}",
            "--prompt {value}",
            "--sandbox",
            "--yolo",
            "--all-files",
            "--approval-mode {value}",
            "--include-directories {value}",
            "--extensions {value}",
            "--checkpointing",
            "--proxy {value}",
            "--debug",
            "--help",
            "--version",
        ],
    ),
    (
        "aider",
        &[
            "--model {value}",
            "--message {value}",
            "--architect",
            "--edit-format {value}",
            "--yes-always",
            "--no-auto-commits",
            "--auto-commits",
            "--read {value}",
            "--file {value}",
            "--no-stream",
            "--map-tokens {value}",
            "--cache-prompts",
            "--dry-run",
            "--commit",
            "--help",
        ],
    ),
    (
        "opencode",
        &[
            "--model {value}",
            "--continue",
            "--prompt {value}",
            "--mode {value}",
            "--help",
            "--version",
        ],
    ),
];

/// The completion token for a flag arg template:
/// `"--model {value}"` → `"--model "` (ready for the value),
/// `"--skip-trust"` → `"--skip-trust"`.
fn flag_completion_token(arg: &str) -> String {
    match arg.split_once("{value}") {
        Some((before, _)) => format!("{} ", before.trim_end()),
        None => arg.trim().to_string(),
    }
}

/// Ghost continuation that completes a known AI agent's exec name from the
/// manifest set, while the user is still typing the first token (no space yet).
/// `cla` → `ude` (claude), `cod` → `ex` (codex), `gem` → `ini` (gemini). Picks
/// the alphabetically-first match for determinism when several agents share a
/// prefix. This is what makes "type an AI command, it completes" work even
/// before that command is in history.
fn agent_exec_ghost(input: &str) -> Option<String> {
    if input.is_empty() || input.contains(' ') {
        return None;
    }
    let map = agent_flag_tokens();
    let mut best: Option<&str> = None;
    for exec in map.keys() {
        if exec.len() > input.len() && exec.starts_with(input) {
            if best.map_or(true, |b| exec.as_str() < b) {
                best = Some(exec.as_str());
            }
        }
    }
    best.map(|e| e[input.len()..].to_string())
}

/// Ghost continuation from the agent flag catalog. Fires only when the input's
/// first token is a known AI CLI exec and there's a space after it (so we
/// never interrupt typing the binary name itself). Matches the token currently
/// being typed (the substring after the last space) against the catalog.
fn agent_flag_ghost(input: &str) -> Option<String> {
    let exec = input.split(' ').next()?;
    if exec.is_empty() {
        return None;
    }
    let map = agent_flag_tokens();
    let flags = map.get(exec)?;
    let last_space = input.rfind(' ')?;
    let current = &input[last_space + 1..];
    for tok in flags {
        if tok.len() > current.len() && tok.starts_with(current) {
            return Some(tok[current.len()..].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commits_drive_future_predictions() {
        let pane = 1u64;
        for c in "git status".chars() {
            observe(pane, InputEvent::Char(c), &[]);
        }
        observe(pane, InputEvent::Enter, &[]);
        for c in "git ".chars() {
            observe(pane, InputEvent::Char(c), &[]);
        }
        let (typed, ghost) = current_ghost(pane).expect("ghost should appear");
        assert_eq!(typed, "git ");
        assert_eq!(ghost, "status");
    }

    #[test]
    fn accept_writes_only_the_continuation() {
        let pane = 2u64;
        for c in "ls -la".chars() {
            observe(pane, InputEvent::Char(c), &[]);
        }
        observe(pane, InputEvent::Enter, &[]);
        observe(pane, InputEvent::Char('l'), &[]);
        let accepted = accept(pane).expect("ghost should be accept-able");
        assert_eq!(accepted, "s -la");
    }

    #[test]
    fn cancel_clears_buffer() {
        let pane = 3u64;
        observe(pane, InputEvent::Char('x'), &[]);
        observe(pane, InputEvent::Cancel, &[]);
        assert!(current_ghost(pane).is_none());
    }

    #[test]
    fn commits_propagate_across_panes() {
        // Pane A commits a distinctive command; pane B opens fresh
        // (no local history) and should still see it as a prediction.
        let a = 1001u64;
        let b = 1002u64;
        for c in "git fetch --prune-tags".chars() {
            observe(a, InputEvent::Char(c), &[]);
        }
        observe(a, InputEvent::Enter, &[]);

        // Pane B types `git fe` — pane-local pool is empty, so the
        // prediction has to come from the global pool.
        for c in "git fe".chars() {
            observe(b, InputEvent::Char(c), &[]);
        }
        let (typed, ghost) = current_ghost(b).expect("cross-pane prediction should land");
        assert_eq!(typed, "git fe");
        assert_eq!(ghost, "tch --prune-tags");
    }

    #[test]
    fn agent_exec_ghost_completes_agent_names() {
        // From the baked manifest set: claude / codex / gemini / aider / opencode.
        assert_eq!(agent_exec_ghost("cla").as_deref(), Some("ude"));
        assert_eq!(agent_exec_ghost("gem").as_deref(), Some("ini"));
        // Don't fire once a space is typed (that's flag territory).
        assert_eq!(agent_exec_ghost("claude "), None);
        // Unknown prefix → nothing.
        assert_eq!(agent_exec_ghost("zzz"), None);
    }

    #[test]
    fn agent_flag_ghost_completes_full_flag_set() {
        // Built-in catalog: claude --mod → "el " (--model ).
        assert_eq!(agent_flag_ghost("claude --mod").as_deref(), Some("el "));
        // codex --sand → "box " (--sandbox ).
        assert_eq!(agent_flag_ghost("codex --sand").as_deref(), Some("box "));
        // gemini --yo → "lo" (--yolo, no value).
        assert_eq!(agent_flag_ghost("gemini --yo").as_deref(), Some("lo"));
        // Only fires for a known agent exec.
        assert_eq!(agent_flag_ghost("ls --mod"), None);
    }

    #[test]
    fn flag_completion_token_strips_value_placeholder() {
        assert_eq!(flag_completion_token("--model {value}"), "--model ");
        assert_eq!(flag_completion_token("--skip-trust"), "--skip-trust");
        assert_eq!(flag_completion_token("--approval-mode {value}"), "--approval-mode ");
    }

    #[test]
    fn cancel_input_drops_buffer_and_ghost() {
        let pane = 4u64;
        for c in "echo".chars() {
            observe(pane, InputEvent::Char(c), &[]);
        }
        observe(pane, InputEvent::Enter, &[]);
        for c in "ec".chars() {
            observe(pane, InputEvent::Char(c), &[]);
        }
        assert!(current_ghost(pane).is_some());
        cancel_input(pane);
        assert!(current_ghost(pane).is_none());
    }
}
