//! Warp-style top stats bar.
//!
//! A slim row painted between the integrated tab bar and the pane
//! content. Carries per-pane context that's useful at a glance but
//! doesn't belong inside the prompt:
//!
//!   - Git: branch · dirty count · ahead↑/behind↓
//!   - Process: foreground process name · CPU% · MEM · uptime  (Phase 2)
//!   - Agent tokens: cumulative input/output tokens for the bound AI  (Phase 3)
//!   - Last command: name + elapsed                                   (Phase 4)
//!
//! All four columns reflect the *active* pane and switch when the user
//! moves focus. Phase 1 (this commit) wires up the framework and the
//! Git column; subsequent commits fill in the rest.
//!
//! Refresh model: each cell decides its own staleness. Git status
//! runs `git -C <cwd>` subprocesses; results are cached per-cwd for
//! ~2 s so painting many panes in quick succession (or repainting at
//! 60fps) doesn't fork-bomb. Detection runs synchronously on the
//! render thread for now — git is fast on local SSDs and our cache
//! window dominates the cost.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// One snapshot of a repo's git status, plus when it was taken.
#[derive(Clone, Debug)]
pub struct GitStatus {
    /// Current branch name (`HEAD` short ref); `(detached)` when on a
    /// raw commit.
    pub branch: String,
    /// Number of tracked files with worktree or index changes (M / A /
    /// D / R / U / ?). Untracked dotfiles count too — matches what
    /// `git status` shows.
    pub dirty: usize,
    /// Commits the local branch is ahead of its upstream.
    pub ahead: usize,
    /// Commits the local branch is behind its upstream.
    pub behind: usize,
}

#[derive(Default)]
struct GitCache {
    by_cwd: HashMap<PathBuf, (Instant, Option<GitStatus>)>,
}

const GIT_CACHE_TTL: Duration = Duration::from_millis(2000);

fn git_cache() -> &'static Mutex<GitCache> {
    static CACHE: OnceLock<Mutex<GitCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(GitCache::default()))
}

/// Resolve git status for `cwd`. Returns None when the directory is
/// outside any repo (so the bar can hide the column instead of
/// showing an empty placeholder).
pub fn git_status_for(cwd: &Path) -> Option<GitStatus> {
    {
        let cache = git_cache().lock();
        if let Some((at, status)) = cache.by_cwd.get(cwd) {
            if at.elapsed() < GIT_CACHE_TTL {
                return status.clone();
            }
        }
    }
    let fresh = compute_git_status(cwd);
    let mut cache = git_cache().lock();
    cache.by_cwd.insert(cwd.to_path_buf(), (Instant::now(), fresh.clone()));
    fresh
}

fn compute_git_status(cwd: &Path) -> Option<GitStatus> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .arg("status")
        .arg("--porcelain=v2")
        .arg("--branch")
        .arg("--no-renames")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .output()
        .ok()?;
    if !out.status.success() {
        // Not a repo, or git missing — both treated the same (no
        // column). git prints the exit code to stderr which we
        // discarded above.
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut branch = "(detached)".to_string();
    let mut ahead = 0usize;
    let mut behind = 0usize;
    let mut dirty = 0usize;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // Format: "+<ahead> -<behind>"
            let mut parts = rest.split_whitespace();
            if let Some(a) = parts.next() {
                ahead = a.trim_start_matches('+').parse().unwrap_or(0);
            }
            if let Some(b) = parts.next() {
                behind = b.trim_start_matches('-').parse().unwrap_or(0);
            }
        } else if line.starts_with('1')
            || line.starts_with('2')
            || line.starts_with('u')
            || line.starts_with('?')
        {
            // 1/2/u = tracked changes (XY codes); ? = untracked. We
            // count each line; v2 emits one line per file.
            dirty += 1;
        }
    }
    Some(GitStatus {
        branch,
        dirty,
        ahead,
        behind,
    })
}

/// Format the git column compactly. Empty string when not in a repo.
///
///   `⎇ main`            — clean repo
///   `⎇ main ●3`         — clean branch with 3 dirty files
///   `⎇ main ↑2`         — 2 ahead of upstream
///   `⎇ main ●3 ↑2 ↓1`   — all three
pub fn render_git_segment(status: &Option<GitStatus>) -> String {
    let Some(s) = status else { return String::new() };
    let mut out = format!("⎇ {}", s.branch);
    if s.dirty > 0 {
        out.push_str(&format!(" ●{}", s.dirty));
    }
    if s.ahead > 0 {
        out.push_str(&format!(" ↑{}", s.ahead));
    }
    if s.behind > 0 {
        out.push_str(&format!(" ↓{}", s.behind));
    }
    out
}
