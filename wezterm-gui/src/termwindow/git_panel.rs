//! Right-docked source-control panel (read-only MVP).
//!
//! A vertical panel docked at the window's RIGHT edge, mirroring the LEFT
//! directory-tree sidebar's plumbing: the panel's pixel width is injected
//! into `window_padding.right` at every terminal-size evaluation site, so the
//! panes reflow to leave a reserved gutter that this panel paints into.
//!
//! It shows, for the active pane's working directory:
//!   - the current branch
//!   - ahead/behind vs. upstream
//!   - the changed files with their staged / unstaged / untracked status
//! and a small "not a git repository" placeholder when the cwd isn't a repo.
//!
//! Freshness: git is never run on the render thread. Status is resolved from a
//! process-global cache keyed by cwd (2 s TTL); a stale/missing entry kicks a
//! background `git status` and returns the previous value immediately. The
//! panel adopts the active pane's cwd on every paint, so a `cd` refreshes the
//! view automatically once the new directory's status lands.

use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Build a `Command` that won't flash a console window on Windows. Unterm's
/// GUI is a `windows_subsystem = "windows"` binary with no console, so
/// launching `git` would otherwise briefly pop a cmd-like window.
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

/// How a changed file relates to the index / worktree. Drives grouping and
/// color in the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// Change staged in the index (X column set).
    Staged,
    /// Change only in the worktree (Y column set), or an unmerged conflict.
    Unstaged,
    /// Not tracked by git.
    Untracked,
}

/// One changed file in the working tree.
#[derive(Clone, Debug)]
pub struct GitFile {
    /// Two-character porcelain-v2 XY status code (`.` rendered as space);
    /// `??` for untracked.
    pub code: String,
    /// Path relative to the repo root, as git reports it.
    pub path: String,
    pub kind: ChangeKind,
}

/// A snapshot of a directory's source-control state.
#[derive(Clone, Debug)]
pub struct GitDetail {
    /// False when `cwd` is not inside a git work tree (or git is missing).
    pub is_repo: bool,
    /// Short branch name, or `(detached)` on a raw commit.
    pub branch: String,
    /// Upstream ref (`origin/main`, ...) when the branch tracks one.
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    /// Changed files, grouped staged → unstaged → untracked. Capped at
    /// `MAX_FILES`; `truncated` records whether more were dropped.
    pub files: Vec<GitFile>,
    pub truncated: bool,
}

impl GitDetail {
    fn not_repo() -> Self {
        Self {
            is_repo: false,
            branch: String::new(),
            upstream: None,
            ahead: 0,
            behind: 0,
            files: vec![],
            truncated: false,
        }
    }
}

const CACHE_TTL: Duration = Duration::from_millis(2000);
const CACHE_PRUNE_AFTER: Duration = Duration::from_secs(60);
const CACHE_PRUNE_MIN_SIZE: usize = 64;
const MAX_INFLIGHT: usize = 8;
const MAX_FILES: usize = 400;

#[derive(Default)]
struct DetailCache {
    by_cwd: HashMap<PathBuf, (Instant, GitDetail)>,
}

fn detail_cache() -> &'static Mutex<DetailCache> {
    static CACHE: OnceLock<Mutex<DetailCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(DetailCache::default()))
}

fn inflight() -> &'static Mutex<HashSet<PathBuf>> {
    static S: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}

fn prune_stale<K: Eq + Hash>(map: &mut HashMap<K, (Instant, GitDetail)>) {
    if map.len() > CACHE_PRUNE_MIN_SIZE {
        map.retain(|_, (at, _)| at.elapsed() < CACHE_PRUNE_AFTER);
    }
}

/// Resolve source-control detail for `cwd`. Returns the cached snapshot
/// immediately (cloned) and kicks a background refresh when the cache is
/// stale or empty; the render thread never blocks on `git`. Returns `None`
/// only until the first background scan for a directory completes.
pub fn git_detail_for(cwd: &Path) -> Option<GitDetail> {
    let need_refresh;
    let cached;
    {
        let mut cache = detail_cache().lock();
        prune_stale(&mut cache.by_cwd);
        match cache.by_cwd.get(cwd) {
            Some((at, detail)) if at.elapsed() < CACHE_TTL => {
                return Some(detail.clone());
            }
            Some((_, detail)) => {
                cached = Some(detail.clone());
                need_refresh = true;
            }
            None => {
                cached = None;
                need_refresh = true;
            }
        }
    }
    if need_refresh {
        let mut guard = inflight().lock();
        if guard.len() < MAX_INFLIGHT && guard.insert(cwd.to_path_buf()) {
            let cwd_owned = cwd.to_path_buf();
            if std::thread::Builder::new()
                .name("git-panel".into())
                .spawn(move || {
                    let fresh = compute_git_detail(&cwd_owned);
                    detail_cache()
                        .lock()
                        .by_cwd
                        .insert(cwd_owned.clone(), (Instant::now(), fresh));
                    inflight().lock().remove(&cwd_owned);
                })
                .is_err()
            {
                guard.remove(cwd);
            }
        }
    }
    cached
}

fn compute_git_detail(cwd: &Path) -> GitDetail {
    let out = hidden_command("git")
        .arg("-C")
        .arg(cwd)
        .arg("status")
        .arg("--porcelain=v2")
        .arg("--branch")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .output();
    let out = match out {
        Ok(o) if o.status.success() => o,
        // Not a repo, or git missing — both treated the same.
        _ => return GitDetail::not_repo(),
    };
    parse_porcelain_v2(&String::from_utf8_lossy(&out.stdout))
}

fn parse_porcelain_v2(text: &str) -> GitDetail {
    let mut branch = "(detached)".to_string();
    let mut upstream = None;
    let mut ahead = 0usize;
    let mut behind = 0usize;
    let mut files: Vec<GitFile> = vec![];
    let mut truncated = false;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            let name = rest.trim();
            // git prints "(detached)" here already when on a raw commit.
            branch = name.to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            upstream = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // Format: "+<ahead> -<behind>"
            let mut parts = rest.split_whitespace();
            if let Some(a) = parts.next() {
                ahead = a.trim_start_matches('+').parse().unwrap_or(0);
            }
            if let Some(b) = parts.next() {
                behind = b.trim_start_matches('-').parse().unwrap_or(0);
            }
        } else if line.starts_with('#') {
            // Other header lines (branch.oid, etc.) — ignore.
        } else if files.len() >= MAX_FILES {
            truncated = true;
        } else if let Some(file) = parse_change_line(line) {
            files.push(file);
        }
    }

    // Group staged → unstaged → untracked (stable within a group).
    files.sort_by_key(|f| match f.kind {
        ChangeKind::Staged => 0,
        ChangeKind::Unstaged => 1,
        ChangeKind::Untracked => 2,
    });

    GitDetail {
        is_repo: true,
        branch,
        upstream,
        ahead,
        behind,
        files,
        truncated,
    }
}

/// Parse a single porcelain-v2 entry line into a `GitFile`, or `None` for
/// lines we don't surface (e.g. ignored `!`).
fn parse_change_line(line: &str) -> Option<GitFile> {
    let kind_byte = line.as_bytes().first().copied()?;
    match kind_byte {
        // Untracked: "? <path>"
        b'?' => {
            let path = line.get(2..)?.to_string();
            Some(GitFile {
                code: "??".to_string(),
                path,
                kind: ChangeKind::Untracked,
            })
        }
        // Ordinary changed entry: "1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>"
        b'1' => {
            let mut it = line.splitn(9, ' ');
            let _tag = it.next()?;
            let xy = it.next()?;
            let path = it.nth(6)?.to_string();
            Some(classify(xy, path))
        }
        // Renamed/copied: "2 <XY> ... <Xscore> <path><tab><origPath>"
        b'2' => {
            let mut it = line.splitn(10, ' ');
            let _tag = it.next()?;
            let xy = it.next()?;
            let rest = it.nth(7)?;
            let path = rest.split('\t').next().unwrap_or(rest).to_string();
            Some(classify(xy, path))
        }
        // Unmerged: "u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>"
        b'u' => {
            let mut it = line.splitn(11, ' ');
            let _tag = it.next()?;
            let xy = it.next()?;
            let path = it.nth(8)?.to_string();
            Some(GitFile {
                code: normalize_code(xy),
                path,
                kind: ChangeKind::Unstaged,
            })
        }
        _ => None,
    }
}

/// Turn a porcelain XY code into a `GitFile`, choosing the change kind:
/// a worktree change (Y set) reads as Unstaged, otherwise a set index (X)
/// column reads as Staged. Files that are both keep the full `MM`-style code.
fn classify(xy: &str, path: String) -> GitFile {
    let mut chars = xy.chars();
    let x = chars.next().unwrap_or('.');
    let y = chars.next().unwrap_or('.');
    let kind = if y != '.' {
        ChangeKind::Unstaged
    } else if x != '.' {
        ChangeKind::Staged
    } else {
        ChangeKind::Unstaged
    };
    GitFile {
        code: normalize_code(xy),
        path,
        kind,
    }
}

/// Render `.` placeholders as spaces so the code column reads as `M `, ` M`,
/// `MM`, etc.
fn normalize_code(xy: &str) -> String {
    let mut chars = xy.chars();
    let x = chars.next().unwrap_or('.');
    let y = chars.next().unwrap_or('.');
    let f = |c: char| if c == '.' { ' ' } else { c };
    format!("{}{}", f(x), f(y))
}

/// Per-window view state for the git panel. The heavy git result lives in the
/// process-global cache (`git_detail_for`); this only tracks which directory
/// the panel is anchored to plus lightweight layout bookkeeping.
pub struct GitPanel {
    /// Directory the panel currently reflects — adopted from the active pane
    /// on every paint so a `cd` refreshes the view.
    pub cwd: PathBuf,
    pub visible_rows: usize,
}

impl GitPanel {
    pub fn new(cwd: PathBuf) -> Self {
        // Warm the cache so the first paint has something to show quickly.
        let _ = git_detail_for(&cwd);
        Self {
            cwd,
            visible_rows: 1,
        }
    }

    /// Point the panel at a new working directory (e.g. the active pane
    /// changed, or the user `cd`'d). Cheap: the actual scan happens off-thread
    /// via the shared cache.
    pub fn set_cwd(&mut self, cwd: PathBuf) {
        if cwd != self.cwd {
            self.cwd = cwd;
        }
        let _ = git_detail_for(&self.cwd);
    }

    /// Current cached detail for the panel's cwd (kicks a background refresh
    /// when stale). `None` only briefly, before the first scan lands.
    pub fn detail(&self) -> Option<GitDetail> {
        git_detail_for(&self.cwd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_ahead_behind() {
        let text = "\
# branch.oid abc123
# branch.head feature/x
# branch.upstream origin/feature/x
# branch.ab +2 -1
1 M. N... 100644 100644 100644 aa bb src/lib.rs
1 .M N... 100644 100644 100644 cc dd README.md
? notes.txt
";
        let d = parse_porcelain_v2(text);
        assert!(d.is_repo);
        assert_eq!(d.branch, "feature/x");
        assert_eq!(d.upstream.as_deref(), Some("origin/feature/x"));
        assert_eq!(d.ahead, 2);
        assert_eq!(d.behind, 1);
        assert_eq!(d.files.len(), 3);
        // Grouped staged first, then unstaged, then untracked.
        assert_eq!(d.files[0].kind, ChangeKind::Staged);
        assert_eq!(d.files[0].path, "src/lib.rs");
        assert_eq!(d.files[0].code, "M ");
        assert_eq!(d.files[1].kind, ChangeKind::Unstaged);
        assert_eq!(d.files[1].code, " M");
        assert_eq!(d.files[2].kind, ChangeKind::Untracked);
        assert_eq!(d.files[2].path, "notes.txt");
    }

    #[test]
    fn parses_rename_line() {
        let text = "2 R. N... 100644 100644 100644 aa bb R100 new/name.rs\told/name.rs\n";
        let d = parse_porcelain_v2(text);
        assert_eq!(d.files.len(), 1);
        assert_eq!(d.files[0].path, "new/name.rs");
        assert_eq!(d.files[0].kind, ChangeKind::Staged);
    }

    #[test]
    fn handles_paths_with_spaces() {
        let text = "1 .M N... 100644 100644 100644 aa bb my file.txt\n";
        let d = parse_porcelain_v2(text);
        assert_eq!(d.files.len(), 1);
        assert_eq!(d.files[0].path, "my file.txt");
    }
}
