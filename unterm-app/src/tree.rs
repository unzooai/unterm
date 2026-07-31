//! A file tree down the left edge.
//!
//! Rooted at the pane's directory, collapsible, and scrolling. The terminal
//! makes room for it rather than being covered by it, so a shell never
//! believes in columns that are behind a panel.
//!
//! Ported from the previous front end. The parts that came across unchanged
//! are the ones whose reasons are not visible from the code:
//!
//! - **Order**: directories first, dotfiles last within each group, then
//!   alphabetically. Sorting purely by name buries `src` under a dozen dot
//!   directories, which is the opposite of what a project tree is for.
//! - **Dimmed rows**: a dotfile is dimmed at any depth, but the system mounts
//!   are dimmed *only at the filesystem root*. Dimming `dev` anywhere would
//!   dim somebody's `~/work/dev`, which is a directory they chose.
//! - **A cap per directory**: a folder with fifty thousand files in it must
//!   not stall the scan, and nobody reads past five hundred rows anyway.
//! - **A parent row**: every tree that has a parent offers `..`, so the root
//!   can be moved up without a separate control.
//!
//! Scanning happens off the drawing thread and carries the number of the
//! navigation it was started for, so a scan that finishes after you have
//! collapsed something cannot put the old tree back.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// How wide the strip is, in cells.
///
/// Wider than the tab strip, because a file name is longer than a tab number
/// and a tree of `...` is not a tree.
pub const COLUMNS: usize = 28;

/// How wide the strip is in pixels, or nothing when it is closed.
pub fn width(open: bool, metrics: unterm_render::quads::CellMetrics) -> f32 {
    if open {
        COLUMNS as f32 * metrics.width
    } else {
        0.0
    }
}

/// How many entries are read from one directory.
const MAX_ENTRIES: usize = 500;
/// How stale a tree may be before a paint asks for a fresh one.
const RESCAN_AFTER: std::time::Duration = std::time::Duration::from_millis(2500);

/// One line of the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    /// Drawn dimmed: a dotfile, or one of the machine's own directories.
    pub is_hidden: bool,
    /// The `..` row. Pressing it moves the tree's root up one.
    pub is_parent: bool,
    /// A drive root, offered because on Windows there is no ascending from
    /// `C:\` to `D:\` -- they are separate roots, and without this a second
    /// drive cannot be reached from the tree at all.
    pub is_drive: bool,
}

impl Row {
    /// What the row says: an arrow for a directory, and its name.
    pub fn text(&self, columns: usize) -> String {
        let name = if self.is_parent {
            "..".to_string()
        } else if self.is_drive {
            self.path.display().to_string()
        } else {
            self.path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| self.path.display().to_string())
        };
        let mark = if self.is_parent {
            "\u{2191} "
        } else if !self.is_dir {
            "  "
        } else if self.expanded {
            "\u{25BE} "
        } else {
            "\u{25B8} "
        };
        let indent = "  ".repeat(self.depth);
        fit(&format!("{indent}{mark}{name}"), columns)
    }
}

/// Shorten to `columns`, cutting the middle.
///
/// The middle, because a file name's ending is where the extension is and its
/// beginning is where the subject is: `report-2026-final.tar.gz` cut from the
/// end is a file of unknown kind.
pub fn fit(text: &str, columns: usize) -> String {
    if columns == 0 {
        return String::new();
    }
    if width_of(text) <= columns {
        return text.to_string();
    }
    const ELLIPSIS: char = '\u{2026}';
    if columns <= 1 {
        return ELLIPSIS.to_string();
    }
    let available = columns - 1;
    let head_budget = available / 2;
    let tail_budget = available - head_budget;

    let characters: Vec<char> = text.chars().collect();
    let mut head = String::new();
    let mut used = 0;
    let mut taken = 0;
    for ch in &characters {
        let wide = crate::terminal::column_width(*ch);
        if used + wide > head_budget {
            break;
        }
        head.push(*ch);
        used += wide;
        taken += 1;
    }
    let mut tail: Vec<char> = Vec::new();
    let mut used = 0;
    for ch in characters[taken..].iter().rev() {
        let wide = crate::terminal::column_width(*ch);
        if used + wide > tail_budget {
            break;
        }
        tail.push(*ch);
        used += wide;
    }
    tail.reverse();
    format!("{head}{ELLIPSIS}{}", tail.into_iter().collect::<String>())
}

fn width_of(text: &str) -> usize {
    text.chars().map(crate::terminal::column_width).sum()
}

/// Read one directory, in the order a project tree wants.
pub fn entries(directory: &Path) -> Vec<(PathBuf, bool)> {
    let Ok(reading) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<(PathBuf, bool)> = reading
        .flatten()
        .map(|entry| {
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            (entry.path(), is_dir)
        })
        .take(MAX_ENTRIES)
        .collect();

    // Directories first, dotfiles last within each group, then by name.
    found.sort_by(|a, b| {
        let dotted = |path: &PathBuf| {
            path.file_name()
                .map(|name| name.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
        };
        b.1.cmp(&a.1)
            .then(dotted(&a.0).cmp(&dotted(&b.0)))
            .then_with(|| a.0.cmp(&b.0))
    });
    found
}

/// The machine's own directories, which are noise in a project tree.
///
/// Only at the filesystem root. `~/work/dev` is a directory somebody chose and
/// belongs at full strength; `/dev` is the kernel's.
const SYSTEM_AT_ROOT: &[&str] = &[
    "dev",
    "proc",
    "sys",
    "private",
    "cores",
    "bin",
    "sbin",
    "usr",
    "var",
    "etc",
    "tmp",
    "opt",
    "lost+found",
];

/// Whether a row is drawn dimmed.
pub fn is_hidden(path: &Path, at_filesystem_root: bool) -> bool {
    let Some(name) = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return false;
    };
    name.starts_with('.') || (at_filesystem_root && SYSTEM_AT_ROOT.contains(&name.as_str()))
}

/// Every visible row of the tree under `root`.
pub fn rows(root: &Path, expanded: &HashSet<PathBuf>) -> Vec<Row> {
    let mut rows = Vec::new();
    if let Some(parent) = root.parent() {
        rows.push(Row {
            path: parent.to_path_buf(),
            depth: 0,
            is_dir: true,
            expanded: false,
            is_hidden: false,
            is_parent: true,
            is_drive: false,
        });
    }
    for drive in drive_roots() {
        if drive != root {
            rows.push(Row {
                path: drive,
                depth: 0,
                is_dir: true,
                expanded: false,
                is_hidden: false,
                is_parent: false,
                is_drive: true,
            });
        }
    }
    walk(root, 0, expanded, &mut rows);
    rows
}

fn walk(directory: &Path, depth: usize, expanded: &HashSet<PathBuf>, rows: &mut Vec<Row>) {
    let at_root = directory.parent().is_none();
    for (path, is_dir) in entries(directory) {
        let open = is_dir && expanded.contains(&path);
        rows.push(Row {
            is_hidden: is_hidden(&path, at_root),
            depth,
            is_dir,
            expanded: open,
            is_parent: false,
            is_drive: false,
            path: path.clone(),
        });
        if open {
            walk(&path, depth + 1, expanded, rows);
        }
    }
}

/// The other roots this machine has.
fn drive_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        (b'A'..=b'Z')
            .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
            .filter(|drive| drive.is_dir())
            .collect()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// An open tree.
pub struct Tree {
    pub root: PathBuf,
    expanded: HashSet<PathBuf>,
    pub rows: Vec<Row>,
    pub scroll: usize,
    scanned_at: std::time::Instant,
    /// The navigation this tree is showing. A scan started before a collapse
    /// carries an older number and is discarded rather than putting the old
    /// tree back.
    epoch: u64,
    pending: std::sync::Arc<parking_lot::Mutex<Option<(u64, Vec<Row>)>>>,
    scanning: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Tree {
    pub fn new(root: PathBuf) -> Self {
        let expanded = HashSet::new();
        let rows = rows(&root, &expanded);
        Self {
            root,
            expanded,
            rows,
            scroll: 0,
            scanned_at: std::time::Instant::now(),
            epoch: 0,
            pending: Default::default(),
            scanning: Default::default(),
        }
    }

    /// Open or close a directory, or move the root when it is the `..` row.
    pub fn press(&mut self, row: usize) -> Option<PathBuf> {
        let row = self.rows.get(row)?.clone();
        if row.is_parent || row.is_drive {
            self.root = row.path.clone();
            self.expanded.clear();
            self.scroll = 0;
            self.rescan();
            return None;
        }
        if !row.is_dir {
            return Some(row.path);
        }
        if !self.expanded.remove(&row.path) {
            self.expanded.insert(row.path);
        }
        self.rescan();
        None
    }

    /// Rebuild now, because something the reader did changed the shape.
    fn rescan(&mut self) {
        self.epoch += 1;
        self.rows = rows(&self.root, &self.expanded);
        self.scanned_at = std::time::Instant::now();
    }

    /// Move the root, keeping nothing: a different project is a different tree.
    pub fn go_to(&mut self, root: PathBuf) {
        if root == self.root {
            return;
        }
        self.root = root;
        self.expanded.clear();
        self.scroll = 0;
        self.rescan();
    }

    /// Take whatever a background scan finished, and start one if the tree has
    /// gone stale.
    ///
    /// Called while painting, so it never waits: reading a directory takes
    /// milliseconds and a frame is sixteen.
    pub fn refresh(&mut self) {
        if let Some((epoch, rows)) = self.pending.lock().take() {
            if epoch == self.epoch {
                self.rows = rows;
                self.scanned_at = std::time::Instant::now();
            }
        }
        if self.scanned_at.elapsed() < RESCAN_AFTER {
            return;
        }
        use std::sync::atomic::Ordering;
        if self.scanning.swap(true, Ordering::AcqRel) {
            return;
        }
        let (root, expanded, epoch) = (self.root.clone(), self.expanded.clone(), self.epoch);
        let (pending, scanning) = (self.pending.clone(), self.scanning.clone());
        let spawned = std::thread::Builder::new()
            .name("tree-scan".into())
            .spawn(move || {
                let fresh = rows(&root, &expanded);
                *pending.lock() = Some((epoch, fresh));
                scanning.store(false, Ordering::Release);
            });
        if spawned.is_err() {
            self.scanning.store(false, Ordering::Release);
        }
    }

    /// Scroll, without running off either end.
    pub fn scroll_by(&mut self, delta: isize, visible: usize) {
        let last = self.rows.len().saturating_sub(visible.max(1));
        let next = self.scroll as isize + delta;
        self.scroll = next.clamp(0, last as isize) as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("a temporary directory")
    }

    /// Directories first, dotfiles last within each group, then by name.
    /// Sorting purely by name buries `src` under a dozen dot directories.
    #[test]
    fn directories_come_first_and_dotfiles_come_last() {
        let root = tempdir();
        for name in [".cache", "src", ".config", "assets"] {
            std::fs::create_dir(root.path().join(name)).expect("a directory");
        }
        for name in [".gitignore", "README.md", "Cargo.toml"] {
            std::fs::write(root.path().join(name), b"").expect("a file");
        }
        let names: Vec<String> = entries(root.path())
            .into_iter()
            .map(|(path, _)| path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "assets",
                "src",
                ".cache",
                ".config",
                "Cargo.toml",
                "README.md",
                ".gitignore"
            ]
        );
    }

    /// A dotfile is dimmed wherever it is.
    #[test]
    fn a_dotfile_is_dimmed_at_any_depth() {
        assert!(is_hidden(Path::new("/home/me/project/.git"), false));
        assert!(is_hidden(Path::new("/.hidden"), true));
        assert!(!is_hidden(Path::new("/home/me/project/src"), false));
    }

    /// The machine's own directories are dimmed only at the filesystem root.
    /// `~/work/dev` is a directory somebody chose; `/dev` is the kernel's.
    #[test]
    fn the_machines_own_directories_are_dimmed_only_at_the_root() {
        assert!(is_hidden(Path::new("/usr"), true));
        assert!(is_hidden(Path::new("/var"), true));
        assert!(!is_hidden(Path::new("/home/me/work/dev"), false));
        assert!(!is_hidden(Path::new("/home/me/tmp"), false));
    }

    /// Every tree that has a parent offers a way up, so the root can be moved
    /// without a separate control.
    #[test]
    fn a_tree_offers_a_way_up() {
        let root = tempdir();
        let inner = root.path().join("project");
        std::fs::create_dir(&inner).expect("a directory");
        let rows = rows(&inner, &HashSet::new());
        assert!(rows.first().map(|row| row.is_parent).unwrap_or(false));
        assert_eq!(rows[0].path, root.path());
    }

    /// Pressing a directory opens it, and its children appear indented under
    /// it. Pressing again closes it.
    #[test]
    fn pressing_a_directory_opens_and_closes_it() {
        let root = tempdir();
        let inner = root.path().join("src");
        std::fs::create_dir(&inner).expect("a directory");
        std::fs::write(inner.join("main.rs"), b"").expect("a file");

        let mut tree = Tree::new(root.path().to_path_buf());
        let at = tree
            .rows
            .iter()
            .position(|row| row.path == inner)
            .expect("the directory is listed");
        assert!(!tree.rows[at].expanded);

        assert_eq!(tree.press(at), None);
        let child = tree
            .rows
            .iter()
            .find(|row| row.path.ends_with("main.rs"))
            .expect("the child appeared");
        assert_eq!(child.depth, 1, "the child is not indented under its parent");

        assert_eq!(tree.press(at), None);
        assert!(!tree.rows.iter().any(|row| row.path.ends_with("main.rs")));
    }

    /// Pressing a file hands its path back rather than doing something to it:
    /// what happens to a file is the window's decision, not the tree's.
    #[test]
    fn pressing_a_file_hands_back_its_path() {
        let root = tempdir();
        let file = root.path().join("notes.txt");
        std::fs::write(&file, b"").expect("a file");
        let mut tree = Tree::new(root.path().to_path_buf());
        let at = tree
            .rows
            .iter()
            .position(|row| row.path == file)
            .expect("the file is listed");
        assert_eq!(tree.press(at), Some(file));
    }

    /// Pressing the way up moves the root there and forgets what was open:
    /// the paths that were expanded are no longer where they were.
    #[test]
    fn pressing_the_way_up_moves_the_root() {
        let root = tempdir();
        let inner = root.path().join("project");
        std::fs::create_dir(&inner).expect("a directory");
        let mut tree = Tree::new(inner.clone());
        assert_eq!(tree.press(0), None);
        assert_eq!(tree.root, root.path());
    }

    /// Scrolling stops at both ends. A tree scrolled past its last row is a
    /// blank strip that looks broken.
    #[test]
    fn scrolling_stops_at_both_ends() {
        let root = tempdir();
        for index in 0..30 {
            std::fs::write(root.path().join(format!("file{index:02}")), b"").expect("a file");
        }
        let mut tree = Tree::new(root.path().to_path_buf());
        tree.scroll_by(-5, 10);
        assert_eq!(tree.scroll, 0);
        tree.scroll_by(1000, 10);
        assert_eq!(tree.scroll, tree.rows.len() - 10);
    }

    /// A name too long to fit keeps both ends: the beginning says what it is
    /// about and the end says what kind of file it is.
    #[test]
    fn a_long_name_keeps_both_of_its_ends() {
        let short = fit("report-2026-final.tar.gz", 16);
        assert_eq!(width_of(&short), 16);
        assert!(short.starts_with("report"), "{short}");
        assert!(short.ends_with(".gz"), "{short}");
    }

    #[test]
    fn a_name_that_fits_is_left_alone() {
        assert_eq!(fit("src", 20), "src");
        assert_eq!(fit("", 20), "");
        assert_eq!(fit("anything", 0), "");
    }

    /// A row says what it is without being opened: an arrow for a closed
    /// directory, a different one for an open one, nothing for a file.
    #[test]
    fn a_row_says_whether_it_is_open() {
        let closed = Row {
            path: PathBuf::from("/a/src"),
            depth: 0,
            is_dir: true,
            expanded: false,
            is_hidden: false,
            is_parent: false,
            is_drive: false,
        };
        let open = Row {
            expanded: true,
            ..closed.clone()
        };
        let file = Row {
            is_dir: false,
            ..closed.clone()
        };
        assert!(closed.text(20).contains('\u{25B8}'));
        assert!(open.text(20).contains('\u{25BE}'));
        assert!(!file.text(20).contains('\u{25B8}'));
        assert!(closed.text(20).contains("src"));
    }

    /// And a row's depth is drawn, or a tree is a list.
    #[test]
    fn depth_is_drawn_as_indentation() {
        let deep = Row {
            path: PathBuf::from("/a/b/c/d"),
            depth: 2,
            is_dir: false,
            expanded: false,
            is_hidden: false,
            is_parent: false,
            is_drive: false,
        };
        assert!(deep.text(30).starts_with("    "), "{:?}", deep.text(30));
    }

    /// Every row fits the strip it is drawn in, whatever its depth or name.
    #[test]
    fn every_row_fits_the_width_it_is_given() {
        let root = tempdir();
        std::fs::write(
            root.path().join("a-very-long-file-name-indeed-yes.txt"),
            b"",
        )
        .expect("a file");
        let tree = Tree::new(root.path().to_path_buf());
        for columns in [4, 10, 22, 40] {
            for row in &tree.rows {
                assert!(
                    width_of(&row.text(columns)) <= columns,
                    "{:?} overflows {columns} columns",
                    row.text(columns)
                );
            }
        }
    }
}
