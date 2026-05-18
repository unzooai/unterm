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
    /// Wall-clock of the most recent observe() call, used to dedup
    /// duplicate dispatches of the same physical keystroke (see
    /// `observe()` for context).
    last_observed_at: Option<std::time::Instant>,
    /// A coarse identity for the most recent observed event. Same
    /// shape as a (kind, payload) tuple but flattened to a tiny
    /// stack-allocated enum to keep the hot path branch-free.
    last_observed_key: Option<EventDedupKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EventDedupKey {
    Char(char),
    Enter,
    Backspace,
    Cancel,
    ClearLine,
}

fn event_dedup_key(event: &InputEvent) -> EventDedupKey {
    match event {
        InputEvent::Char(c) => EventDedupKey::Char(*c),
        InputEvent::Enter => EventDedupKey::Enter,
        InputEvent::Backspace => EventDedupKey::Backspace,
        InputEvent::Cancel => EventDedupKey::Cancel,
        InputEvent::ClearLine => EventDedupKey::ClearLine,
    }
}

const MAX_COMMITS: usize = 256;
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

/// Record a keyboard event on `pane_id`. `external_candidates` is
/// merged with the pane's own commit history when recomputing the
/// ghost — caller typically passes recent scrollback lines so the
/// pool reflects what's on screen, but it's optional.
///
/// Built-in dedup: the same `(pane_id, event)` arriving within
/// `DEDUP_WINDOW_MS` of the previous one is silently dropped. This
/// matters because wezterm dispatches certain keystrokes through
/// both the `Key::Code` and `Key::Composed` branches (notably on
/// Windows with an IME loaded), and we observe on both. Without
/// dedup each character would land in the buffer twice and pollute
/// the commit pool with `wwhhooaammii`-style garbage.
pub fn observe(pane_id: u64, event: InputEvent, external_candidates: &[String]) {
    const DEDUP_WINDOW_MS: u128 = 8;
    let mut reg = registry().lock();
    let state = reg.entry(pane_id).or_default();
    let now = std::time::Instant::now();
    let event_key = event_dedup_key(&event);
    if let (Some(prev_at), Some(prev_key)) = (state.last_observed_at, &state.last_observed_key) {
        if prev_key == &event_key
            && now.duration_since(prev_at).as_millis() < DEDUP_WINDOW_MS
        {
            return;
        }
    }
    state.last_observed_at = Some(now);
    state.last_observed_key = Some(event_key);
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
            if !trimmed.is_empty() && state.commits.last().map_or(true, |last| last != trimmed) {
                state.commits.push(trimmed.to_string());
                if state.commits.len() > MAX_COMMITS {
                    let drop = MAX_COMMITS / 8;
                    state.commits.drain(..drop);
                }
            }
        }
    }
    recompute_ghost(state, external_candidates);
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
}

/// Take a snapshot of the named pane's ghost state. Returns `None`
/// when the pane has never been seen by the observer (no key events
/// recorded for it).
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
    Some(DebugSnapshot {
        input_buffer: state.input.clone(),
        input_buffer_len: state.input.chars().count(),
        ghost: state.ghost.clone(),
        commit_count: state.commits.len(),
        recent_commits: recent,
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
    // Pane-local commits first (most relevant), then external pool.
    let candidates = state
        .commits
        .iter()
        .rev()
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
    state.ghost = best;
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
}
