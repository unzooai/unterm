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

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

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
}

const RESCAN_AFTER_MS: u128 = 2500;
const MAX_ENTRIES_PER_DIR: usize = 500;

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
        };
        me.rebuild();
        me
    }

    pub fn toggle_dir(&mut self, path: &Path) {
        if self.expanded.contains(path) {
            self.expanded.remove(path);
        } else {
            self.expanded.insert(path.to_path_buf());
        }
        self.rebuild();
    }

    /// Rescan if the cache is stale; returns true when rows changed.
    pub fn ensure_fresh(&mut self) -> bool {
        if self.last_scan.elapsed().as_millis() < RESCAN_AFTER_MS {
            return false;
        }
        let before: Vec<PathBuf> = self.rows.iter().map(|r| r.path.clone()).collect();
        self.rebuild();
        let after: Vec<PathBuf> = self.rows.iter().map(|r| r.path.clone()).collect();
        before != after
    }

    pub fn rebuild(&mut self) {
        let mut rows = vec![];
        if let Some(parent) = self.root.parent() {
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
            if drive != self.root {
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
        let root = self.root.clone();
        self.walk(&root, 0, &mut rows);
        self.rows = rows;
        self.last_scan = Instant::now();
        let max_top = self.rows.len().saturating_sub(1);
        if self.scroll_top > max_top {
            self.scroll_top = max_top;
        }
    }

    /// Move the tree's anchor one level up. No-op at filesystem root.
    /// Used by clicks on the synthetic "↑ .." row.
    pub fn navigate_to_parent(&mut self) {
        if let Some(parent) = self.root.parent() {
            self.root = parent.to_path_buf();
            self.scroll_top = 0;
            self.rebuild();
        }
    }

    pub fn navigate_to_root(&mut self, root: PathBuf) {
        self.root = root;
        self.scroll_top = 0;
        self.rebuild();
    }

    fn walk(&self, dir: &Path, depth: usize, rows: &mut Vec<TreeRow>) {
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
            let expanded = is_dir && self.expanded.contains(&path);
            rows.push(TreeRow {
                path: path.clone(),
                depth,
                is_dir,
                expanded,
                is_hidden,
                is_drive: false,
                is_parent: false,
            });
            if expanded {
                self.walk(&path, depth + 1, rows);
            }
        }
    }

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
