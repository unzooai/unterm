//! Left directory-tree sidebar (v0.40 "D").
//!
//! A persistent, collapsible file tree docked at the window's left edge,
//! rooted at the active pane's project directory. The terminal content
//! reflows around it: the sidebar width is injected at every
//! window_padding.left evaluation site, so panes, splits, mouse mapping and
//! terminal cols all shift together. Rendering is box-model (same pipeline
//! as the tab bar), so it is cross-platform and theme-aware.
//!
//! Interactions (wired in mouseevent.rs):
//!   single-click dir    → expand/collapse
//!   double-click dir    → cd the active pane there
//!   single-click file   → insert the quoted path at the prompt
//!   double-click file   → open with the system handler
//!   right-click         → popup context menu (copy path / reveal / new tab)
//!   wheel               → scroll
//!
//! Freshness: the tree rescans lazily (on expand/collapse/toggle and at most
//! every ~2.5s during paints) rather than running a filesystem watcher; a
//! notify-based watcher is a follow-up.

use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use termwiz::cell::unicode_column_width;

pub struct TreeRow {
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    pub is_hidden: bool,
    /// Synthetic Windows drive root row (`C:\`, `D:\`, ...).
    pub is_drive: bool,
    /// Synthetic "↑ .." row prepended to every non-root tree — clicking
    /// it re-anchors the sidebar at `root.parent()`. Same shape as a
    /// real row so we don't fork the click pipeline.
    pub is_parent: bool,
}

pub struct TreeSidebar {
    pub root: PathBuf,
    expanded: HashSet<PathBuf>,
    pub rows: Vec<TreeRow>,
    pub scroll_top: usize,
    pub visible_rows: usize,
    /// User-resized width in points; None uses the shared UI token.
    pub width_pts: Option<f32>,
    last_scan: Instant,
    /// Result slot for an in-flight background rescan, tagged with the
    /// `scan_epoch` it was started under so a result computed before a
    /// navigation (expand/collapse/cd) is discarded instead of clobbering
    /// the fresh tree. `None` until a scan finishes.
    pending: Arc<Mutex<Option<(u64, Vec<TreeRow>)>>>,
    /// True while a background rescan thread is running, so paints don't
    /// spawn a second one for the same window.
    scanning: Arc<AtomicBool>,
    /// Bumped on every synchronous `rebuild` (navigation). A background
    /// scan whose epoch no longer matches is dropped on arrival.
    scan_epoch: Arc<AtomicU64>,
    /// Path of the row hit by the *first* click of a potential double-click.
    /// The first click of a double already expands/collapses the row and
    /// repaints, which shifts every row below it — so the second physical
    /// click can hit-test a different row. The double-click handler uses this
    /// captured path instead of re-resolving the (now shifted) hit, so a
    /// double-click always acts on the row the user aimed at. See
    /// `mouse_event_tree_sidebar`.
    pub pending_double_path: Option<PathBuf>,
}

const RESCAN_AFTER_MS: u128 = 2500;
const MAX_ENTRIES_PER_DIR: usize = 500;

pub(crate) fn fit_tree_header_text(
    root_name: &str,
    row_count: usize,
    visible_rows: usize,
    scroll_top: usize,
    max_cols: usize,
) -> String {
    let prefix = "▦  ";
    let suffix = if row_count > visible_rows {
        let from = scroll_top.saturating_add(1).min(row_count);
        let to = scroll_top.saturating_add(visible_rows).min(row_count);
        format!("  ↕ {from}-{to}/{row_count}")
    } else {
        String::new()
    };
    let fixed_cols = unicode_column_width(prefix, None) + unicode_column_width(&suffix, None);
    let root = crate::termwindow::sidebar_text::ellipsize_middle(
        root_name,
        max_cols.saturating_sub(fixed_cols),
    );
    let text = format!("{prefix}{root}{suffix}");
    crate::termwindow::sidebar_text::ellipsize_middle(&text, max_cols)
}

pub(crate) fn fit_tree_row_text(glyph: &str, name: &str, max_cols: usize) -> String {
    let glyph_cols = unicode_column_width(glyph, None);
    let name = crate::termwindow::sidebar_text::ellipsize_middle(
        name,
        max_cols.saturating_sub(glyph_cols),
    );
    let text = format!("{glyph}{name}");
    crate::termwindow::sidebar_text::ellipsize_middle(&text, max_cols)
}

fn list_dir(path: &Path) -> Vec<(PathBuf, bool)> {
    let mut out: Vec<(PathBuf, bool)> = std::fs::read_dir(path)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| {
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    (e.path(), is_dir)
                })
                .take(MAX_ENTRIES_PER_DIR)
                .collect()
        })
        .unwrap_or_default();
    // dirs first, dotfiles last within each group, then lexicographic
    out.sort_by(|a, b| {
        let ha =
            a.0.file_name()
                .map_or(false, |n| n.to_string_lossy().starts_with('.'));
        let hb =
            b.0.file_name()
                .map_or(false, |n| n.to_string_lossy().starts_with('.'));
        b.1.cmp(&a.1).then(ha.cmp(&hb)).then_with(|| a.0.cmp(&b.0))
    });
    out
}

impl TreeSidebar {
    pub fn new(root: PathBuf) -> Self {
        let mut me = Self {
            root,
            expanded: HashSet::new(),
            rows: vec![],
            scroll_top: 0,
            visible_rows: 1,
            width_pts: None,
            last_scan: Instant::now(),
            pending: Arc::new(Mutex::new(None)),
            scanning: Arc::new(AtomicBool::new(false)),
            scan_epoch: Arc::new(AtomicU64::new(0)),
            pending_double_path: None,
        };
        me.queue_rebuild();
        me
    }

    pub fn toggle_dir(&mut self, path: &Path) {
        if self.expanded.contains(path) {
            self.expanded.remove(path);
        } else {
            self.expanded.insert(path.to_path_buf());
        }
        self.queue_rebuild();
    }

    /// Called from the paint path. Never touches the filesystem itself:
    /// it adopts a ready background-scan result (if any) and otherwise, when
    /// the tree is stale, spawns the rescan on a worker thread. This keeps
    /// `read_dir` of every expanded directory OFF the render thread, which
    /// previously produced a periodic hitch every `RESCAN_AFTER_MS` while
    /// the sidebar was open (visible as stutter during terminal output and
    /// tab switches). Returns true when rows changed this call.
    pub fn ensure_fresh(&mut self) -> bool {
        // 1. Adopt a finished background scan, if its epoch is still current.
        let ready = self.pending.lock().take();
        if let Some((epoch, rows)) = ready {
            self.scanning.store(false, Ordering::Release);
            if epoch == self.scan_epoch.load(Ordering::Acquire) {
                self.rows = rows;
                self.last_scan = Instant::now();
                let max_top = self.rows.len().saturating_sub(1);
                if self.scroll_top > max_top {
                    self.scroll_top = max_top;
                }
                return true;
            }
            // Stale (a navigation happened mid-scan); discard and let the
            // staleness check below kick off a fresh scan.
        }

        // 2. Kick off a rescan if due and none is already running.
        if self.last_scan.elapsed().as_millis() >= RESCAN_AFTER_MS
            && !self.scanning.swap(true, Ordering::AcqRel)
        {
            // Stamp last_scan now so we don't re-spawn on every paint while
            // the worker runs; the adopt branch refreshes it on completion.
            self.last_scan = Instant::now();
            let epoch = self.scan_epoch.load(Ordering::Acquire);
            let root = self.root.clone();
            let expanded = self.expanded.clone();
            let pending = Arc::clone(&self.pending);
            if !spawn_scan(epoch, root, expanded, pending) {
                self.scanning.store(false, Ordering::Release);
            }
        }
        false
    }

    /// Rebuild for navigation (expand/collapse/cd/anchor change). Keep only
    /// cheap chrome rows immediately and let the paint path adopt the full
    /// filesystem scan when a worker finishes.
    pub fn rebuild(&mut self) {
        self.queue_rebuild();
    }

    fn queue_rebuild(&mut self) {
        let epoch = self.scan_epoch.fetch_add(1, Ordering::AcqRel) + 1;
        self.rows = build_chrome_rows(&self.root);
        self.last_scan = Instant::now();
        self.scroll_top = 0;
        let max_top = self.rows.len().saturating_sub(1);
        if self.scroll_top > max_top {
            self.scroll_top = max_top;
        }
        self.scanning.store(true, Ordering::Release);
        let root = self.root.clone();
        let expanded = self.expanded.clone();
        let pending = Arc::clone(&self.pending);
        if !spawn_scan(epoch, root, expanded, pending) {
            self.scanning.store(false, Ordering::Release);
        }
    }

    /// Move the tree's anchor one level up. No-op at filesystem root.
    /// Used by clicks on the synthetic "↑ .." row.
    pub fn navigate_to_parent(&mut self) {
        if let Some(parent) = self.root.parent() {
            self.root = parent.to_path_buf();
            self.scroll_top = 0;
            self.queue_rebuild();
        }
    }

    pub fn navigate_to_root(&mut self, root: PathBuf) {
        self.root = root;
        self.scroll_top = 0;
        self.queue_rebuild();
    }

    // Tree construction lives in the free functions `build_rows` /
    // `walk_into` below so a background thread (which has no `&self`) can
    // run the identical scan.
}

#[cfg(test)]
mod layout_tests {
    use super::{fit_tree_header_text, fit_tree_row_text};
    use termwiz::cell::unicode_column_width;

    #[test]
    fn long_selected_folder_stays_inside_header_budget() {
        let text = fit_tree_header_text(
            "workspace/特别长的文件夹名称-with-a-specific-tail",
            240,
            18,
            80,
            24,
        );
        assert!(unicode_column_width(&text, None) <= 24, "{}", text);
        assert!(text.contains('…'));
    }

    #[test]
    fn deeply_nested_row_stays_inside_remaining_budget() {
        let text = fit_tree_row_text("▾ ", "a-very-long-nested-folder-name", 12);
        assert!(unicode_column_width(&text, None) <= 12, "{}", text);
        assert!(text.starts_with("▾ "));
    }

    #[test]
    fn tiny_file_sidebar_budget_never_overflows() {
        for budget in 0..5 {
            let text = fit_tree_header_text("long-folder", 1, 8, 0, budget);
            assert!(
                unicode_column_width(&text, None) <= budget,
                "{}: {}",
                budget,
                text
            );
        }
    }
}

/// Build the full row list for a tree rooted at `root` with the given set
/// of `expanded` directories. Pure filesystem work — safe to run on any
/// thread. This is what the background rescan calls.
fn build_rows(root: &Path, expanded: &HashSet<PathBuf>) -> Vec<TreeRow> {
    let mut rows = build_chrome_rows(root);
    walk_into(expanded, root, 0, &mut rows);
    rows
}

fn build_chrome_rows(root: &Path) -> Vec<TreeRow> {
    let mut rows = vec![];
    if let Some(parent) = root.parent() {
        rows.push(TreeRow {
            path: parent.to_path_buf(),
            depth: 0,
            is_dir: true,
            expanded: false,
            is_hidden: false,
            is_drive: false,
            is_parent: true,
        });
    }
    #[cfg(windows)]
    for drive in windows_drive_roots() {
        if drive != *root {
            rows.push(TreeRow {
                path: drive,
                depth: 0,
                is_dir: true,
                expanded: false,
                is_hidden: false,
                is_drive: true,
                is_parent: false,
            });
        }
    }
    rows
}

fn spawn_scan(
    epoch: u64,
    root: PathBuf,
    expanded: HashSet<PathBuf>,
    pending: Arc<Mutex<Option<(u64, Vec<TreeRow>)>>>,
) -> bool {
    std::thread::Builder::new()
        .name("tree-sidebar-scan".into())
        .spawn(move || {
            let rows = build_rows(&root, &expanded);
            let mut slot = pending.lock();
            if slot
                .as_ref()
                .map(|(existing_epoch, _)| *existing_epoch <= epoch)
                .unwrap_or(true)
            {
                *slot = Some((epoch, rows));
            }
        })
        .is_ok()
}

fn walk_into(expanded: &HashSet<PathBuf>, dir: &Path, depth: usize, rows: &mut Vec<TreeRow>) {
    let at_fs_root = dir.parent().is_none();
    for (path, is_dir) in list_dir(dir) {
        let basename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        // Dotfiles are dimmed at any level; macOS system mounts
        // (`/dev`, `/proc`, `/sys`, `/private`, `/cores`, `/.vol`)
        // are dimmed only when they sit at filesystem root, so a
        // user's `/Volumes/Dev` workspace stays full-strength.
        let is_dotfile = basename.starts_with('.');
        let is_system_root = at_fs_root
            && matches!(
                basename.as_str(),
                "dev"
                    | "proc"
                    | "sys"
                    | "private"
                    | "cores"
                    | "bin"
                    | "sbin"
                    | "usr"
                    | "var"
                    | "etc"
                    | "tmp"
                    | "opt"
                    | "lost+found"
            );
        let is_hidden = is_dotfile || is_system_root;
        let is_expanded = is_dir && expanded.contains(&path);
        rows.push(TreeRow {
            path: path.clone(),
            depth,
            is_dir,
            expanded: is_expanded,
            is_hidden,
            is_drive: false,
            is_parent: false,
        });
        if is_expanded {
            walk_into(expanded, &path, depth + 1, rows);
        }
    }
}

impl TreeSidebar {
    pub fn scroll_by(&mut self, delta: isize, visible_rows: usize) {
        let len = self.rows.len();
        let max_top = len.saturating_sub(visible_rows.max(1));
        let next = (self.scroll_top as isize + delta).clamp(0, max_top as isize);
        self.scroll_top = next as usize;
    }

    pub fn scroll_to_thumb_top(
        &mut self,
        thumb_top: usize,
        track_top: usize,
        track_height: usize,
        thumb_height: usize,
        row_count: usize,
        visible_rows: usize,
    ) {
        let max_top = row_count.saturating_sub(visible_rows.max(1));
        if max_top == 0 {
            self.scroll_top = 0;
            return;
        }

        let max_thumb_top = track_height.saturating_sub(thumb_height);
        if max_thumb_top == 0 {
            self.scroll_top = 0;
            return;
        }

        let thumb_top = thumb_top.saturating_sub(track_top).min(max_thumb_top);
        self.scroll_top =
            ((thumb_top as f32 / max_thumb_top as f32) * max_top as f32).round() as usize;
    }
}

#[cfg(windows)]
fn windows_drive_roots() -> Vec<PathBuf> {
    let mask = unsafe { winapi::um::fileapi::GetLogicalDrives() };
    if mask == 0 {
        return vec![];
    }

    ('A'..='Z')
        .enumerate()
        .filter_map(|(idx, drive)| {
            if (mask & (1 << idx)) == 0 {
                None
            } else {
                Some(PathBuf::from(format!("{drive}:\\")))
            }
        })
        .collect()
}
