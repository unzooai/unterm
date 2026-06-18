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
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Build a `Command` that won't flash a console window on Windows.
///
/// Unterm's GUI is a `windows_subsystem = "windows"` binary with no
/// console, so launching a console program (git / powershell) makes
/// Windows briefly pop a cmd-like window. The stats bar refreshes these
/// on every tab switch, so the flash was very visible. `CREATE_NO_WINDOW`
/// suppresses it. No-op on non-Windows platforms.
fn hidden_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Set of cache keys currently being refreshed off the render thread.
/// Stops a paint storm from spawning N threads for the same query.
fn inflight_git() -> &'static Mutex<HashSet<PathBuf>> {
    static S: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}
fn inflight_proc() -> &'static Mutex<HashSet<u32>> {
    static S: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}

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

/// Resolve git status for `cwd`. Returns whatever's in the cache
/// immediately (cloned), and kicks a background refresh if the cache
/// is stale or empty — the render thread never blocks on `git`. The
/// first paint after a cwd change shows None ("—") for a few ms while
/// the worker fetches, then subsequent paints pick up the fresh value.
pub fn git_status_for(cwd: &Path) -> Option<GitStatus> {
    let need_refresh;
    let cached;
    {
        let cache = git_cache().lock();
        match cache.by_cwd.get(cwd) {
            Some((at, status)) if at.elapsed() < GIT_CACHE_TTL => {
                return status.clone();
            }
            Some((_, status)) => {
                cached = status.clone();
                need_refresh = true;
            }
            None => {
                cached = None;
                need_refresh = true;
            }
        }
    }
    if need_refresh {
        let mut inflight = inflight_git().lock();
        if inflight.insert(cwd.to_path_buf()) {
            let cwd_owned = cwd.to_path_buf();
            std::thread::spawn(move || {
                let fresh = compute_git_status(&cwd_owned);
                let mut cache = git_cache().lock();
                cache
                    .by_cwd
                    .insert(cwd_owned.clone(), (Instant::now(), fresh));
                inflight_git().lock().remove(&cwd_owned);
            });
        }
    }
    cached
}

fn compute_git_status(cwd: &Path) -> Option<GitStatus> {
    let out = hidden_command("git")
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
/// The leading icon is the Powerline branch glyph (U+E0A0). It lives in
/// the bundled `SymbolsNerdFontMono` fallback and shares baseline + cell
/// width with JetBrains Mono, so a mono-rendered stats string lays out
/// without the per-glyph baseline drift that the old `⎇` (Apple
/// Option-key symbol) caused — `⎇` is missing from JBM, so the renderer
/// fell back to a proportional system font with a different x-height
/// and the chrome read as misaligned ("ʆ master 3.7% CPU · 2M").
///
///   `\u{e0a0} main`            — clean repo
///   `\u{e0a0} main *3`         — clean branch with 3 dirty files
///   `\u{e0a0} main +2`         — 2 ahead of upstream
///   `\u{e0a0} main *3 +2 -1`   — all three
///
/// Dirty / ahead / behind markers stay ASCII (`*` / `+` / `-`) on
/// purpose — the `●` / `↑` / `↓` Unicode set also triggered fallback
/// in some shipped JBM variants. ASCII keeps a single baseline.
pub fn render_git_segment(status: &Option<GitStatus>) -> String {
    let Some(s) = status else {
        return String::new();
    };
    let mut out = format!("\u{e0a0} {}", s.branch);
    if s.dirty > 0 {
        out.push_str(&format!(" *{}", s.dirty));
    }
    if s.ahead > 0 {
        out.push_str(&format!(" +{}", s.ahead));
    }
    if s.behind > 0 {
        out.push_str(&format!(" -{}", s.behind));
    }
    out
}

/// One snapshot of an active pane's foreground process: CPU %, RSS,
/// elapsed wall time. All three come from `ps -p <pid> -o ...`.
#[derive(Clone, Debug)]
pub struct ProcStatus {
    /// CPU percent (0-100 per core; ps reports total).
    pub cpu_pct: f32,
    /// Resident memory in bytes.
    pub rss_bytes: u64,
    /// Wall-clock seconds since the process started.
    pub uptime_secs: u64,
    /// COMM name — shown so the user knows whose CPU it is.
    pub name: String,
}

#[derive(Default)]
struct ProcCache {
    by_pid: HashMap<u32, (Instant, Option<ProcStatus>)>,
}

const PROC_CACHE_TTL: Duration = Duration::from_millis(2000);

fn proc_cache() -> &'static Mutex<ProcCache> {
    static CACHE: OnceLock<Mutex<ProcCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ProcCache::default()))
}

/// Resolve process status for `pid`. Cached for 2 s; stale lookups
/// kick a background refresh and return the stale value, so the
/// render thread never blocks on `ps`.
pub fn proc_status_for(pid: u32) -> Option<ProcStatus> {
    let need_refresh;
    let cached;
    {
        let cache = proc_cache().lock();
        match cache.by_pid.get(&pid) {
            Some((at, status)) if at.elapsed() < PROC_CACHE_TTL => {
                return status.clone();
            }
            Some((_, status)) => {
                cached = status.clone();
                need_refresh = true;
            }
            None => {
                cached = None;
                need_refresh = true;
            }
        }
    }
    if need_refresh {
        let mut inflight = inflight_proc().lock();
        if inflight.insert(pid) {
            std::thread::spawn(move || {
                let fresh = compute_proc_status(pid);
                let mut cache = proc_cache().lock();
                cache.by_pid.insert(pid, (Instant::now(), fresh));
                inflight_proc().lock().remove(&pid);
            });
        }
    }
    cached
}

#[cfg(unix)]
fn compute_proc_status(pid: u32) -> Option<ProcStatus> {
    // POSIX ps with empty `=` headers prints values only — single
    // space-separated line. Works the same on macOS, Linux, *BSD.
    let out = hidden_command("ps")
        .args(["-p", &pid.to_string(), "-o", "pcpu=,rss=,etime=,comm="])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    // ps -o … prints fields left-padded with spaces (`  0.0   1712 …`),
    // so `splitn(4, whitespace_char)` returned empty fragments for the
    // runs of consecutive spaces and the rss parse failed silently.
    // `split_whitespace()` collapses any run of whitespace into one
    // separator, which is what we want.
    let line = String::from_utf8_lossy(&out.stdout);
    let mut parts = line.split_whitespace();
    let pcpu: f32 = parts.next()?.parse().ok()?;
    let rss_kb: u64 = parts.next()?.parse().ok()?;
    let etime_str = parts.next()?;
    // The comm field can contain spaces (e.g. login shells start with
    // `-` and may have spaces); rejoin the rest of the tokens.
    let name: String = parts.collect::<Vec<_>>().join(" ");
    let name = if name.is_empty() {
        "?".to_string()
    } else {
        name
    };
    // etime formats: "MM:SS", "HH:MM:SS", or "DD-HH:MM:SS"
    let uptime_secs = parse_etime(etime_str);
    Some(ProcStatus {
        cpu_pct: pcpu,
        rss_bytes: rss_kb * 1024,
        uptime_secs,
        name,
    })
}

#[cfg(windows)]
fn compute_proc_status(pid: u32) -> Option<ProcStatus> {
    // Windows ps shim — no native `ps`. Use PowerShell's Get-Process
    // for WS (working set / RSS) + StartTime + ProcessName. CPU%
    // would need a second sample to compute a delta, which is more
    // bookkeeping than the column is worth; leave it at 0.0 for now
    // so the rest of the columns still light up.
    let script = format!(
        "$p = Get-Process -Id {pid} -ErrorAction Stop; \
         $secs = [int](([DateTime]::Now) - $p.StartTime).TotalSeconds; \
         \"{{0}}|{{1}}|{{2}}\" -f $p.WS, $secs, $p.ProcessName"
    );
    let out = hidden_command("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let line = line.trim();
    let mut parts = line.split('|');
    let rss_bytes: u64 = parts.next()?.trim().parse().ok()?;
    let uptime_secs: u64 = parts.next()?.trim().parse().ok()?;
    let name = parts.next().unwrap_or("?").trim().to_string();
    Some(ProcStatus {
        cpu_pct: 0.0,
        rss_bytes,
        uptime_secs,
        name,
    })
}

fn parse_etime(s: &str) -> u64 {
    let (days, rest) = match s.split_once('-') {
        Some((d, r)) => (d.parse::<u64>().unwrap_or(0), r),
        None => (0, s),
    };
    let parts: Vec<u64> = rest.split(':').filter_map(|p| p.parse().ok()).collect();
    let (h, m, sec) = match parts.as_slice() {
        [h, m, s] => (*h, *m, *s),
        [m, s] => (0u64, *m, *s),
        _ => (0, 0, 0),
    };
    days * 86_400 + h * 3_600 + m * 60 + sec
}

fn format_bytes(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if b >= GB {
        format!("{:.1}G", b as f64 / GB as f64)
    } else if b >= MB {
        format!("{}M", b / MB)
    } else if b >= KB {
        format!("{}K", b / KB)
    } else {
        format!("{}B", b)
    }
}

fn format_etime(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h{:02}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d{:02}h", secs / 86_400, (secs % 86_400) / 3600)
    }
}

/// `8% cpu  1.4G  4m` — compact process column. Empty when no data.
///
/// Separator is two spaces (not `·`). The middle-dot rendered fine in
/// SF Pro but didn't share a baseline with the digits when the stats
/// text moved to JBM, and the two-space gap reads cleaner against the
/// numeric run anyway. "cpu" is lowercased to keep one all-caps run
/// out of the bar, which otherwise drew the eye to a single segment.
pub fn render_proc_segment(status: &Option<ProcStatus>) -> String {
    let Some(s) = status else {
        return String::new();
    };
    format!(
        "{:.1}% cpu  {}  {}",
        s.cpu_pct,
        format_bytes(s.rss_bytes),
        format_etime(s.uptime_secs)
    )
}
