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
//! Refresh model: each cell decides its own staleness. Git/process stats
//! return cached values immediately and refresh off the render thread when
//! stale, with in-flight limits so painting many panes in quick succession
//! (or repainting at 60fps) doesn't fork-bomb.

use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
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
const GIT_CACHE_PRUNE_AFTER: Duration = Duration::from_secs(60);
const GIT_CACHE_PRUNE_MIN_SIZE: usize = 128;
const GIT_MAX_INFLIGHT: usize = 8;
const PROC_CACHE_PRUNE_AFTER: Duration = Duration::from_secs(60);
const PROC_CACHE_PRUNE_MIN_SIZE: usize = 256;
const PROC_MAX_INFLIGHT: usize = 16;

fn git_cache() -> &'static Mutex<GitCache> {
    static CACHE: OnceLock<Mutex<GitCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(GitCache::default()))
}

fn prune_stale_cache<K, V>(
    map: &mut HashMap<K, (Instant, V)>,
    prune_min_size: usize,
    prune_after: Duration,
) where
    K: Eq + Hash,
{
    if map.len() > prune_min_size {
        map.retain(|_, (at, _)| at.elapsed() < prune_after);
    }
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
        let mut cache = git_cache().lock();
        prune_stale_cache(
            &mut cache.by_cwd,
            GIT_CACHE_PRUNE_MIN_SIZE,
            GIT_CACHE_PRUNE_AFTER,
        );
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
        if inflight.len() < GIT_MAX_INFLIGHT && inflight.insert(cwd.to_path_buf()) {
            let cwd_owned = cwd.to_path_buf();
            if std::thread::Builder::new()
                .name("top-stats-git".into())
                .spawn(move || {
                    let fresh = compute_git_status(&cwd_owned);
                    let mut cache = git_cache().lock();
                    cache
                        .by_cwd
                        .insert(cwd_owned.clone(), (Instant::now(), fresh));
                    inflight_git().lock().remove(&cwd_owned);
                })
                .is_err()
            {
                inflight.remove(cwd);
            }
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
        let mut cache = proc_cache().lock();
        prune_stale_cache(
            &mut cache.by_pid,
            PROC_CACHE_PRUNE_MIN_SIZE,
            PROC_CACHE_PRUNE_AFTER,
        );
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
        if inflight.len() < PROC_MAX_INFLIGHT && inflight.insert(pid) {
            if std::thread::Builder::new()
                .name("top-stats-proc".into())
                .spawn(move || {
                    let fresh = compute_proc_status(pid);
                    let mut cache = proc_cache().lock();
                    cache.by_pid.insert(pid, (Instant::now(), fresh));
                    inflight_proc().lock().remove(&pid);
                })
                .is_err()
            {
                inflight.remove(&pid);
            }
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

/// Last (wall clock, cumulative CPU time) sample per pid, both in
/// FILETIME 100 ns units. CPU% is the delta between two refreshes;
/// keyed the same way as the proc cache so it prunes alongside it.
#[cfg(windows)]
struct WinCpuSample {
    wall_100ns: u64,
    cpu_100ns: u64,
}

#[cfg(windows)]
fn win_cpu_samples() -> &'static Mutex<HashMap<u32, (Instant, WinCpuSample)>> {
    static S: OnceLock<Mutex<HashMap<u32, (Instant, WinCpuSample)>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(windows)]
fn compute_proc_status(pid: u32) -> Option<ProcStatus> {
    // Native Win32 sampling — no `ps` on Windows, and the previous
    // PowerShell Get-Process shim cost a hidden ~100 ms process spawn
    // per refresh and couldn't report CPU% at all (a percent needs two
    // samples). GetProcessTimes gives cumulative CPU time; we keep the
    // previous sample per pid and report the delta between refreshes.
    // The first sighting of a pid has no window yet, so it falls back
    // to the average over the process's whole lifetime.
    use winapi::shared::minwindef::FILETIME;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{GetProcessTimes, OpenProcess};
    use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;
    use winapi::um::winbase::QueryFullProcessImageNameW;
    use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;

    fn filetime_100ns(ft: &FILETIME) -> u64 {
        ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
    }

    struct ProcessHandle(winapi::um::winnt::HANDLE);
    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let handle = ProcessHandle(handle);

    let (creation, kernel, user) = unsafe {
        let mut creation: FILETIME = std::mem::zeroed();
        let mut exit: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        if GetProcessTimes(handle.0, &mut creation, &mut exit, &mut kernel, &mut user) == 0 {
            return None;
        }
        (creation, kernel, user)
    };
    let now_100ns = unsafe {
        let mut now: FILETIME = std::mem::zeroed();
        GetSystemTimeAsFileTime(&mut now);
        filetime_100ns(&now)
    };
    let created_100ns = filetime_100ns(&creation);
    let cpu_100ns = filetime_100ns(&kernel) + filetime_100ns(&user);
    let uptime_secs = now_100ns.saturating_sub(created_100ns) / 10_000_000;

    let cpu_pct = {
        let mut samples = win_cpu_samples().lock();
        prune_stale_cache(
            &mut samples,
            PROC_CACHE_PRUNE_MIN_SIZE,
            PROC_CACHE_PRUNE_AFTER,
        );
        let prev = samples.insert(
            pid,
            (
                Instant::now(),
                WinCpuSample {
                    wall_100ns: now_100ns,
                    cpu_100ns,
                },
            ),
        );
        let (wall_delta, cpu_delta) = match prev {
            // Guard both subtractions: the wall clock can step backwards
            // (NTP), and a recycled pid would make cpu appear to shrink.
            Some((_, prev)) if now_100ns > prev.wall_100ns && cpu_100ns >= prev.cpu_100ns => {
                (now_100ns - prev.wall_100ns, cpu_100ns - prev.cpu_100ns)
            }
            _ => (now_100ns.saturating_sub(created_100ns), cpu_100ns),
        };
        if wall_delta == 0 {
            0.0
        } else {
            (cpu_delta as f64 / wall_delta as f64 * 100.0) as f32
        }
    };

    let rss_bytes = unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if GetProcessMemoryInfo(handle.0, &mut pmc, pmc.cb) != 0 {
            pmc.WorkingSetSize as u64
        } else {
            0
        }
    };

    // Match the old shim's ProcessName semantics: image name without
    // the .exe extension.
    let name = unsafe {
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        if QueryFullProcessImageNameW(handle.0, 0, buf.as_mut_ptr(), &mut len) != 0 {
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            std::path::Path::new(&full)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "?".to_string())
        } else {
            "?".to_string()
        }
    };

    Some(ProcStatus {
        cpu_pct,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn native_proc_status_reports_own_process() {
        let pid = std::process::id();
        let first = compute_proc_status(pid).expect("own process must resolve");
        assert!(first.rss_bytes > 0);
        assert!(!first.name.is_empty() && first.name != "?");

        // Burn CPU so the second sample's delta window is clearly hot;
        // the first call above seeded the per-pid sample.
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_millis(150) {
            std::hint::black_box(t0.elapsed());
        }
        let second = compute_proc_status(pid).expect("own process must resolve");
        assert!(
            second.cpu_pct > 5.0,
            "busy loop should register, got {}",
            second.cpu_pct
        );
    }

    #[test]
    fn parse_etime_handles_ps_formats() {
        assert_eq!(parse_etime("03:04"), 184);
        assert_eq!(parse_etime("02:03:04"), 7_384);
        assert_eq!(parse_etime("1-02:03:04"), 93_784);
        assert_eq!(parse_etime("not-time"), 0);
    }

    #[test]
    fn prune_stale_cache_waits_until_threshold() {
        let mut map = HashMap::new();
        let old = Instant::now() - Duration::from_secs(120);
        map.insert(1u32, (old, "old"));

        prune_stale_cache(&mut map, 1, Duration::from_secs(60));

        assert!(map.contains_key(&1));
    }

    #[test]
    fn prune_stale_cache_removes_old_entries_when_large() {
        let mut map = HashMap::new();
        let old = Instant::now() - Duration::from_secs(120);
        let fresh = Instant::now();
        map.insert(1u32, (old, "old"));
        map.insert(2u32, (fresh, "fresh"));

        prune_stale_cache(&mut map, 1, Duration::from_secs(60));

        assert!(!map.contains_key(&1));
        assert!(map.contains_key(&2));
    }
}
