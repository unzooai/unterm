//! Directory jump palette (v0.40 "B"): Warp-grade go-to-directory.
//!
//! Summoned from the ⌄ menu, Ctrl+Shift+O, or the top-bar ⌸ button. Type to
//! fuzzy-match across recent/project directories and the live subdirectories
//! of the current browse root (initially the active pane's cwd). Enter cds
//! the pane (shell-aware quoting via cd_command_for_pane), Cmd/Ctrl+Enter
//! opens the directory in a new tab, Tab descends into the selection,
//! Backspace on an empty query ascends to the parent, Cmd+O falls back to
//! the system folder picker. Fully mouse-operable: hover moves the
//! selection (one shared cursor with the keyboard), click jumps. Layout
//! follows the same Warp-measured point spec as popup_menu.rs.

use crate::overlay::selector::{matcher_pattern, matcher_score};
use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::{DimensionContext, TermWindow, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::keyassignment::{KeyAssignment, SpawnCommand};
use config::Dimension;
use mux::pane::PaneId;
use std::cell::{Ref, RefCell};
use std::path::{Path, PathBuf};
use wezterm_term::{KeyCode, KeyModifiers, MouseEvent};
use window::color::LinearRgba;
use window::WindowOps;

/// Ellipsize from the LEFT so the tail of a path stays readable
/// (`…termwindow/render`). Width is measured in display cells (CJK = 2).
/// Long labels otherwise lay out wider than the palette card — the text
/// and the selected-row highlight both spill past the card's right edge.
fn ellipsize_path_left(text: &str, max_cols: usize) -> String {
    use termwiz::cell::unicode_column_width;
    if max_cols == 0 {
        return String::new();
    }
    if unicode_column_width(text, None) <= max_cols {
        return text.to_string();
    }
    let mut tail: Vec<char> = vec![];
    let mut used = 0usize;
    for c in text.chars().rev() {
        let w = unicode_column_width(&c.to_string(), None);
        if used + w + 1 > max_cols {
            break;
        }
        tail.push(c);
        used += w;
    }
    let mut out = String::from("…");
    out.extend(tail.iter().rev());
    out
}

#[derive(Copy, Clone, PartialEq)]
enum Section {
    Recent,
    /// Mounted volumes (macOS /Volumes, Linux /mnt + /media/$USER) — the
    /// Finder-sidebar "Locations". Without these, a user whose work lives
    /// on a second disk (/Volumes/Dev/…) can never reach it by name: typing
    /// `dev` completes to the system /dev instead.
    Location,
    SubDir,
    /// Nested directory found by the bounded recursive scan; matched and
    /// displayed by its path relative to the browse root.
    Deep,
    /// Filesystem path completion (query starts with `/` or `~`).
    PathComp,
}

struct DirItem {
    name: String,
    path: PathBuf,
    section: Section,
    /// Deep: path relative to the browse root (main label + match haystack).
    /// PathComp: the parent directory, for the dim right-hand hint.
    rel: Option<String>,
}

pub struct DirJump {
    pane_id: PaneId,
    action: DirJumpAction,
    base: RefCell<PathBuf>,
    input: RefCell<String>,
    items: RefCell<Vec<DirItem>>,
    /// Bounded recursive scan below `base`, built lazily on the first
    /// non-empty query and reused until the base changes. This is what lets
    /// a query like `term ren` land on …/termwindow/render in one shot
    /// instead of Tab-descending level by level.
    deep: RefCell<Option<Vec<DirItem>>>,
    /// Display order → index into `items`, after fuzzy filtering.
    all_visible: RefCell<Vec<usize>>,
    /// First index from `all_visible` rendered in the current page.
    page_start: RefCell<usize>,
    /// Current page's display order → index into `items`.
    visible: RefCell<Vec<usize>>,
    /// Index into `visible`.
    selected: RefCell<usize>,
    /// How many matches were cut off by MAX_VISIBLE — rendered as a dim
    /// "… +N" footer so a truncated list never masquerades as complete.
    overflow: RefCell<usize>,
    element: RefCell<Option<Vec<ComputedElement>>>,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum DirJumpAction {
    ChangeCwd,
    NewTab,
    SplitRight,
}

const MAX_VISIBLE: usize = 14;
const DEEP_MAX_DEPTH: usize = 6;
const DEEP_MAX_ENTRIES: usize = 3000;
const SCROLLBAR_W: f32 = 8.0;

/// Directories that are pure noise in a "jump to directory" flow — huge,
/// machine-managed, and never a place you cd to on purpose from a picker.
const DEEP_SKIP: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".cache",
    ".venv",
    "venv",
    "__pycache__",
    ".Trash",
    ".npm",
    ".cargo",
];

/// Bounded BFS below `base`: depth ≤ DEEP_MAX_DEPTH, ≤ DEEP_MAX_ENTRIES
/// total, hidden + noise dirs skipped. Returns (path, path-relative-to-base).
fn deep_scan(base: &Path) -> Vec<(PathBuf, String)> {
    let mut out = vec![];
    let mut queue: std::collections::VecDeque<(PathBuf, usize)> = Default::default();
    queue.push_back((base.to_path_buf(), 0));
    while let Some((dir, depth)) = queue.pop_front() {
        if depth >= DEEP_MAX_DEPTH || out.len() >= DEEP_MAX_ENTRIES {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.filter_map(|e| e.ok()) {
            if out.len() >= DEEP_MAX_ENTRIES {
                break;
            }
            if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || DEEP_SKIP.contains(&name.as_str()) {
                continue;
            }
            let path = e.path();
            if let Ok(rel) = path.strip_prefix(base) {
                // Normalize to `/`: the depth indent (rel.matches('/'))
                // and the query matching both assume forward slashes,
                // but Path::display yields `\` on Windows.
                let rel = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                // depth 0 children are already in the SubDir section
                if depth > 0 {
                    out.push((path.clone(), rel));
                }
            }
            queue.push_back((path, depth + 1));
        }
    }
    out
}

/// Mounted volumes / drive locations, Finder-sidebar style.
fn locations() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    #[cfg(target_os = "macos")]
    {
        if let Ok(rd) = std::fs::read_dir("/Volumes") {
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                // Skip the startup-disk symlink (it just aliases /).
                if let Ok(target) = std::fs::read_link(&p) {
                    if target == Path::new("/") {
                        continue;
                    }
                }
                if p.is_dir() {
                    out.push(p);
                }
            }
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for base in ["/mnt".to_string(), {
            let user = std::env::var("USER").unwrap_or_default();
            format!("/media/{user}")
        }] {
            if let Ok(rd) = std::fs::read_dir(&base) {
                for e in rd.filter_map(|e| e.ok()) {
                    let p = e.path();
                    if p.is_dir() && !e.file_name().to_string_lossy().starts_with('.') {
                        out.push(p);
                    }
                }
            }
        }
    }
    #[cfg(windows)]
    {
        // Drive letters are separate roots on Windows: from C:\ you can
        // never ascend to D:\, so without these a second drive is simply
        // unreachable from the palette.
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            if Path::new(&drive).is_dir() {
                out.push(PathBuf::from(drive));
            }
        }
    }
    out.sort();
    out
}

/// Is this query an explicit filesystem path (vs a fuzzy fragment)?
/// Unix absolute (`/…`), home (`~…`), Windows drive (`D:` / `D:\…` / `D:/…`)
/// and UNC (`\\server\…`) all count.
fn is_path_query(s: &str) -> bool {
    if s.starts_with('/') || s.starts_with('~') {
        return true;
    }
    let b = s.as_bytes();
    if b.len() >= 2 && b[1] == b':' && (b[0] as char).is_ascii_alphabetic() {
        return true;
    }
    s.starts_with("\\\\")
}

/// Split a path-mode query into (parent-to-list, fragment-to-filter),
/// platform-independently so the Windows rules are unit-testable anywhere.
/// Windows: backslashes normalized (incl. what Tab writes back), bare "D:"
/// promoted to the drive root ("D:" alone is cwd-relative in Win32
/// semantics, never what a picker means), and a parent that collapses to
/// "D:" gets its slash back so read_dir hits the root.
fn split_path_query(expanded: &str, windows: bool) -> Option<(String, String)> {
    let mut expanded = expanded.to_string();
    if windows {
        expanded = expanded.replace('\\', "/");
        let b = expanded.as_bytes();
        if b.len() == 2 && b[1] == b':' {
            expanded.push('/');
        }
    }
    let (parent, frag) = match expanded.rfind('/') {
        Some(0) => ("/".to_string(), expanded[1..].to_string()),
        Some(i) => (expanded[..i].to_string(), expanded[i + 1..].to_string()),
        None => return None,
    };
    let parent = if windows && parent.len() == 2 && parent.ends_with(':') {
        format!("{parent}/")
    } else {
        parent
    };
    Some((parent, frag))
}

/// Expand a leading `~` to the home directory.
fn expand_tilde(input: &str) -> String {
    if let Some(rest) = input.strip_prefix('~') {
        if let Some(home) = dirs_next::home_dir() {
            return format!("{}{}", home.display(), rest);
        }
    }
    input.to_string()
}

fn load_recents() -> Vec<PathBuf> {
    let path = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("projects.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return vec![];
    };
    let mut out = vec![];
    if let Some(arr) = value.as_array() {
        for v in arr {
            if let Some(s) = v.as_str() {
                let p = PathBuf::from(s);
                if p.is_dir() {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn subdirs_of(base: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(base)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .take(400)
                .collect()
        })
        .unwrap_or_default();
    dirs.sort_by(|a, b| {
        // dotdirs sort after normal dirs, then lexicographic
        let ah = a
            .file_name()
            .map_or(false, |n| n.to_string_lossy().starts_with('.'));
        let bh = b
            .file_name()
            .map_or(false, |n| n.to_string_lossy().starts_with('.'));
        ah.cmp(&bh).then_with(|| a.cmp(b))
    });
    dirs
}

fn display_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}

fn tilde_path(p: &Path) -> String {
    let s = p.display().to_string();
    if let Some(home) = dirs_next::home_dir() {
        let h = home.display().to_string();
        if let Some(rest) = s.strip_prefix(&h) {
            return format!("~{rest}");
        }
    }
    s
}

fn page_start_for_thumb_top(
    thumb_top: usize,
    track_top: usize,
    track_height: usize,
    thumb_height: usize,
    total: usize,
) -> usize {
    if total <= MAX_VISIBLE {
        return 0;
    }
    let max_start = total.saturating_sub(MAX_VISIBLE);
    let travel = track_height.saturating_sub(thumb_height).max(1);
    let offset = thumb_top.saturating_sub(track_top).min(travel);
    ((offset as f32 / travel as f32) * max_start as f32).round() as usize
}

impl DirJump {
    pub(crate) fn with_action(pane_id: PaneId, base: PathBuf, action: DirJumpAction) -> Self {
        let me = Self {
            pane_id,
            action,
            base: RefCell::new(base),
            input: RefCell::new(String::new()),
            items: RefCell::new(vec![]),
            deep: RefCell::new(None),
            all_visible: RefCell::new(vec![]),
            page_start: RefCell::new(0),
            visible: RefCell::new(vec![]),
            selected: RefCell::new(0),
            overflow: RefCell::new(0),
            element: RefCell::new(None),
        };
        me.reload();
        me
    }

    fn reload(&self) {
        let base = self.base.borrow().clone();
        let mut items = vec![];
        for p in load_recents() {
            items.push(DirItem {
                name: display_name(&p),
                path: p,
                section: Section::Recent,
                rel: None,
            });
        }
        for p in locations() {
            items.push(DirItem {
                name: display_name(&p),
                path: p,
                section: Section::Location,
                rel: None,
            });
        }
        for p in subdirs_of(&base) {
            items.push(DirItem {
                name: display_name(&p),
                path: p,
                section: Section::SubDir,
                rel: None,
            });
        }
        *self.items.borrow_mut() = items;
        self.deep.borrow_mut().take();
        self.refilter();
    }

    /// Rebuild `items` for path-completion mode: the query is an absolute
    /// (or ~-relative) path; list the directories under its parent and
    /// filter on the final fragment.
    fn path_completion_items(&self, input: &str) -> Vec<DirItem> {
        let Some((parent, frag)) = split_path_query(&expand_tilde(input), cfg!(windows)) else {
            return vec![];
        };
        let parent_path = PathBuf::from(&parent);
        let pattern = (!frag.is_empty()).then(|| matcher_pattern(&frag));
        // With a bare `dir/` (no fragment yet), offer the typed directory
        // itself as the first row so Enter goes exactly where the input
        // says — completion shouldn't force you one level deeper.
        let mut head: Vec<DirItem> = vec![];
        if frag.is_empty() && parent_path.is_dir() {
            head.push(DirItem {
                name: crate::i18n::t("dirjump.here"),
                rel: Some(tilde_path(&parent_path)),
                path: parent_path.clone(),
                section: Section::PathComp,
            });
        }
        let mut scored: Vec<(u32, DirItem)> = subdirs_of(&parent_path)
            .into_iter()
            .filter_map(|p| {
                let name = display_name(&p);
                let score = match &pattern {
                    Some(pat) => matcher_score(pat, &name)?,
                    None => 0,
                };
                Some((
                    score,
                    DirItem {
                        name,
                        rel: Some(tilde_path(&parent_path)),
                        path: p,
                        section: Section::PathComp,
                    },
                ))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
        head.extend(scored.into_iter().map(|(_, it)| it));
        head
    }

    fn refilter(&self) {
        let input = self.input.borrow().clone();
        *self.page_start.borrow_mut() = 0;

        // Path-completion mode: typing an absolute or ~ path navigates the
        // real filesystem instead of fuzzy-matching the browse root.
        if is_path_query(&input) {
            let comp = self.path_completion_items(&input);
            let all_visible: Vec<usize> = (0..comp.len()).collect();
            *self.items.borrow_mut() = comp;
            self.set_all_visible(all_visible);
            *self.selected.borrow_mut() = 0;
            self.element.borrow_mut().take();
            return;
        }

        // (Re)attach the standard item set if a previous keystroke was in
        // path mode (items were swapped out wholesale).
        if self
            .items
            .borrow()
            .iter()
            .any(|it| it.section == Section::PathComp)
        {
            // reload() rebuilds recents+subdirs and calls refilter() again.
            let base_changed_input = self.input.borrow().clone();
            self.reload();
            *self.input.borrow_mut() = base_changed_input;
        }

        let mut all_visible: Vec<usize> = vec![];
        if input.is_empty() {
            let items = self.items.borrow();
            // Recents, then locations (volumes), then subdirs, natural order.
            for section in [Section::Recent, Section::Location, Section::SubDir] {
                for (i, _) in items
                    .iter()
                    .enumerate()
                    .filter(|(_, it)| it.section == section)
                {
                    all_visible.push(i);
                }
            }
        } else {
            // Lazily build the deep scan and splice it into `items` once per
            // browse root (first filtering keystroke pays the ~tens of ms).
            if self.deep.borrow().is_none() {
                let base = self.base.borrow().clone();
                let scanned: Vec<DirItem> = deep_scan(&base)
                    .into_iter()
                    .map(|(path, rel)| DirItem {
                        name: display_name(&path),
                        path,
                        section: Section::Deep,
                        rel: Some(rel),
                    })
                    .collect();
                self.deep.borrow_mut().replace(scanned);
                let mut items = self.items.borrow_mut();
                items.retain(|it| it.section != Section::Deep);
                items.extend(
                    self.deep
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|it| DirItem {
                            name: it.name.clone(),
                            path: it.path.clone(),
                            section: it.section,
                            rel: it.rel.clone(),
                        }),
                );
            }
            let items = self.items.borrow();
            let pattern = matcher_pattern(&input);
            let mut scored: Vec<(u32, usize)> = items
                .iter()
                .enumerate()
                .filter_map(|(i, it)| {
                    let (hay, depth_penalty) = match it.section {
                        Section::Recent | Section::Location => {
                            (format!("{} {}", it.name, it.path.display()), 0u32)
                        }
                        Section::SubDir => (it.name.clone(), 0),
                        Section::Deep => {
                            let rel = it.rel.clone().unwrap_or_else(|| it.name.clone());
                            // Mild shallow-first bias so `src` prefers
                            // ./src over ./a/b/c/src on equal match quality.
                            let depth = rel.matches('/').count() as u32;
                            (rel, depth * 2)
                        }
                        Section::PathComp => (it.name.clone(), 0),
                    };
                    matcher_score(&pattern, &hay).map(|s| (s.saturating_sub(depth_penalty), i))
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            all_visible = scored.into_iter().map(|(_, i)| i).collect();
        }
        self.set_all_visible(all_visible);
        *self.selected.borrow_mut() = 0;
        self.element.borrow_mut().take();
    }

    fn set_all_visible(&self, all_visible: Vec<usize>) {
        *self.all_visible.borrow_mut() = all_visible;
        self.refresh_visible_page();
    }

    fn refresh_visible_page(&self) {
        let all_visible = self.all_visible.borrow();
        let max_start = all_visible.len().saturating_sub(MAX_VISIBLE);
        {
            let mut page_start = self.page_start.borrow_mut();
            *page_start = (*page_start).min(max_start);
        }
        let page_start = *self.page_start.borrow();
        let visible: Vec<usize> = all_visible
            .iter()
            .skip(page_start)
            .take(MAX_VISIBLE)
            .copied()
            .collect();
        *self.overflow.borrow_mut() = all_visible.len().saturating_sub(page_start + visible.len());
        *self.visible.borrow_mut() = visible;
        let len = self.visible.borrow().len();
        if len == 0 {
            *self.selected.borrow_mut() = 0;
        } else {
            let mut selected = self.selected.borrow_mut();
            *selected = (*selected).min(len - 1);
        }
    }

    fn invalidate(&self, term_window: &TermWindow) {
        self.element.borrow_mut().take();
        if let Some(window) = term_window.window.as_ref() {
            window.invalidate();
        }
    }

    fn selected_path(&self, display_idx: usize) -> Option<PathBuf> {
        let visible = self.visible.borrow();
        let items = self.items.borrow();
        visible
            .get(display_idx)
            .and_then(|&i| items.get(i))
            .map(|it| it.path.clone())
    }

    /// Apply the selected directory and close. Cmd/Ctrl+Enter always opens
    /// a new tab as an escape hatch; menu-launched palettes can make Enter
    /// mean new-tab or split directly.
    /// Mouse hover moves the keyboard selection so there's exactly one
    /// cursor: the old renderer-native hover highlight could sit on one
    /// row while the keyboard selection marked another, which read as
    /// two overlapping cursors.
    pub fn hover_select(&self, display_idx: usize) {
        if display_idx >= self.visible.borrow().len() {
            return;
        }
        if *self.selected.borrow() != display_idx {
            *self.selected.borrow_mut() = display_idx;
            self.element.borrow_mut().take();
        }
    }

    pub fn activate(&self, display_idx: usize, term_window: &mut TermWindow, force_new_tab: bool) {
        let Some(path) = self.selected_path(display_idx) else {
            return;
        };
        term_window.cancel_modal();
        let action = if force_new_tab {
            DirJumpAction::NewTab
        } else {
            self.action
        };
        match action {
            DirJumpAction::NewTab => {
                let spawn = SpawnCommand {
                    cwd: Some(path),
                    ..Default::default()
                };
                if let Some(pane) = term_window.get_active_pane_or_overlay() {
                    let _ = term_window
                        .perform_key_assignment(&pane, &KeyAssignment::SpawnCommandInNewTab(spawn));
                }
            }
            DirJumpAction::SplitRight => {
                let spawn = SpawnCommand {
                    cwd: Some(path),
                    ..Default::default()
                };
                if let Some(pane) = term_window.get_active_pane_or_overlay() {
                    let _ = term_window
                        .perform_key_assignment(&pane, &KeyAssignment::SplitHorizontal(spawn));
                }
            }
            DirJumpAction::ChangeCwd => {
                let Some(pane) = term_window.get_active_pane_no_overlay() else {
                    return;
                };
                if pane.pane_id() != self.pane_id {
                    // The pane we were summoned for is gone; cd the active one.
                }
                let cmd = super::cd_command_for_pane(&pane, &path);
                {
                    let mut writer = pane.writer();
                    if let Err(err) = writer.write_all(cmd.as_bytes()) {
                        log::warn!("dir jump: could not inject cd: {err:#}");
                    }
                }
            }
        }
    }

    fn open_system_picker(&self, term_window: &mut TermWindow) {
        let pane_id = self.pane_id;
        let action = self.action;
        term_window.cancel_modal();
        match action {
            DirJumpAction::ChangeCwd => {
                term_window.change_working_directory_for_pane_system_picker(pane_id)
            }
            DirJumpAction::NewTab => term_window.open_project_directory_from_system_picker(),
            DirJumpAction::SplitRight => term_window.open_folder_in_split_system_picker(pane_id),
        }
    }

    /// Tab: shell-style completion. The selected directory's path is written
    /// into the input field with a trailing `/`, which flips the palette
    /// into path-completion mode listing its children — exactly the
    /// `cd dir<Tab>` muscle memory. The input field always shows where you
    /// are, and Backspace edits it like text (no hidden browse-root jumps).
    fn complete_selected(&self, term_window: &TermWindow) {
        let sel = *self.selected.borrow();
        if let Some(path) = self.selected_path(sel) {
            let mut completed = tilde_path(&path);
            if !completed.ends_with('/') && !completed.ends_with('\\') {
                completed.push('/');
            }
            *self.input.borrow_mut() = completed;
            self.refilter();
            self.invalidate(term_window);
        }
    }

    /// Backspace on empty input: ascend to parent.
    fn ascend(&self, term_window: &TermWindow) {
        let parent = self.base.borrow().parent().map(|p| p.to_path_buf());
        if let Some(parent) = parent {
            *self.base.borrow_mut() = parent;
            self.reload();
            self.invalidate(term_window);
        }
    }

    fn move_selection(&self, delta: isize, term_window: &TermWindow) {
        let len = self.visible.borrow().len();
        if len == 0 {
            return;
        }
        let cur = *self.selected.borrow() as isize;
        let next = cur + delta;
        if next < 0 {
            let mut page_start = self.page_start.borrow_mut();
            if *page_start > 0 {
                *page_start = page_start.saturating_sub(1);
                drop(page_start);
                self.refresh_visible_page();
            } else {
                *self.selected.borrow_mut() = len - 1;
            }
            self.invalidate(term_window);
            return;
        }
        if next >= len as isize {
            let mut page_start = self.page_start.borrow_mut();
            let all_len = self.all_visible.borrow().len();
            if *page_start + len < all_len {
                *page_start += 1;
                drop(page_start);
                self.refresh_visible_page();
            } else {
                *self.selected.borrow_mut() = 0;
            }
            self.invalidate(term_window);
            return;
        }
        let next = next as usize;
        *self.selected.borrow_mut() = next;
        self.invalidate(term_window);
    }

    pub fn scroll_results(&self, delta: isize, term_window: &TermWindow) {
        if delta == 0 || self.all_visible.borrow().len() <= MAX_VISIBLE {
            return;
        }
        let max_start = self.all_visible.borrow().len().saturating_sub(MAX_VISIBLE);
        let cur = *self.page_start.borrow() as isize;
        let next = (cur + delta).clamp(0, max_start as isize) as usize;
        if next == *self.page_start.borrow() {
            return;
        }
        *self.page_start.borrow_mut() = next;
        *self.selected.borrow_mut() = 0;
        self.refresh_visible_page();
        self.invalidate(term_window);
    }

    pub fn scroll_to_thumb_top(
        &self,
        thumb_top: usize,
        track_top: usize,
        track_height: usize,
        thumb_height: usize,
        term_window: &TermWindow,
    ) {
        let all_len = self.all_visible.borrow().len();
        if all_len <= MAX_VISIBLE {
            return;
        }
        let max_start = all_len.saturating_sub(MAX_VISIBLE);
        let next =
            page_start_for_thumb_top(thumb_top, track_top, track_height, thumb_height, all_len);
        if next == *self.page_start.borrow() {
            return;
        }
        *self.page_start.borrow_mut() = next.min(max_start);
        *self.selected.borrow_mut() = 0;
        self.refresh_visible_page();
        self.invalidate(term_window);
    }

    fn compute(&self, term_window: &mut TermWindow) -> anyhow::Result<Vec<ComputedElement>> {
        let font = term_window
            .fonts
            .title_font()
            .expect("to resolve title font");
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let pt = term_window.dimensions.dpi as f32 / 72.0;

        let bg = LinearRgba::with_srgba(0x2a, 0x2a, 0x2a, 0xff);
        let fg = LinearRgba::with_srgba(0xf2, 0xf2, 0xf0, 0xff);
        let dim = LinearRgba::with_srgba(0xac, 0xac, 0xa8, 0xff);
        let teal = LinearRgba::with_srgba(0x6f, 0xcc, 0xb8, 0xff);
        let hover_bg = LinearRgba::with_srgba(0x3d, 0x3d, 0x3d, 0xff);

        let input = self.input.borrow().clone();
        let base = tilde_path(&self.base.borrow());
        let selected = *self.selected.borrow();

        // Row text budget in display cells: the card's width minus its
        // border, the row's left border + padding, and the scrollbar
        // reserve. Labels/hints longer than this must be ellipsized —
        // the box model does not clip text, so an over-long path lays
        // out past the card's right edge (and the selected-row
        // highlight along with it).
        let card_width = (480. * pt)
            .min(term_window.dimensions.pixel_width as f32 - 32. * pt)
            .round();
        let row_cols = (((card_width - (2. + (2. + 12. + 14. + 8. + 8. + SCROLLBAR_W) * pt))
            / metrics.cell_size.width as f32)
            .floor()
            .max(8.)) as usize;

        let mut children: Vec<Element> = vec![];

        // Header: a real input field — inset darker box, teal caret, hint
        // when empty. The browse root moves down into the subdir caption.
        let field_bg = LinearRgba::with_srgba(0x1c, 0x1c, 0x1c, 0xff);
        let shown_input = if input.is_empty() {
            crate::i18n::t("dirjump.placeholder")
        } else {
            input.clone()
        };
        let input_color = if input.is_empty() { dim } else { fg };
        let field = Element::new(
            &font,
            ElementContent::Children(vec![
                Element::new(&font, ElementContent::Text("⌕ ".to_string())).colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: teal.into(),
                }),
                Element::new(&font, ElementContent::Text(shown_input)).colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: input_color.into(),
                }),
                Element::new(&font, ElementContent::Text("▏".to_string())).colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: teal.into(),
                }),
            ]),
        )
        .colors(ElementColors {
            border: BorderColor::new(fg.mul_alpha(0.14)),
            bg: field_bg.into(),
            text: fg.into(),
        })
        .border(BoxDimension::new(Dimension::Pixels(1.)))
        .padding(BoxDimension {
            left: Dimension::Pixels(10. * pt),
            right: Dimension::Pixels(10. * pt),
            top: Dimension::Pixels(6. * pt),
            bottom: Dimension::Pixels(6. * pt),
        })
        .min_width(Some(Dimension::Percent(1.)))
        .display(DisplayType::Block);
        children.push(
            Element::new(&font, ElementContent::Children(vec![field]))
                .padding(BoxDimension {
                    left: Dimension::Pixels(10. * pt),
                    right: Dimension::Pixels(10. * pt),
                    top: Dimension::Pixels(8. * pt),
                    bottom: Dimension::Pixels(6. * pt),
                })
                .min_width(Some(Dimension::Percent(1.)))
                .display(DisplayType::Block),
        );

        let visible = self.visible.borrow();
        let items = self.items.borrow();
        let total_visible = self.all_visible.borrow().len();
        // The result rows live in their own container so the floated
        // scrollbar can sit beside them. The scrollbar must NOT be a leading
        // block sibling of the rows: box_model adds a float's height to the
        // current block-row height, so a tall float placed before the rows
        // shoves every following block down by the scrollbar's height (the
        // big empty gap users saw). Instead we build it here and append it
        // AFTER the row container, not marked `display: Block`, so it floats
        // at the list's top-right and nothing flows below it.
        let mut list_kids: Vec<Element> = vec![];
        let mut scrollbar_el: Option<Element> = None;
        if total_visible > MAX_VISIBLE {
            let track_height = (MAX_VISIBLE as f32 * 24. * pt).round().max(64.);
            let thumb_height = ((MAX_VISIBLE as f32 / total_visible as f32) * track_height)
                .round()
                .max(24.)
                .min(track_height);
            let page_start = *self.page_start.borrow();
            let max_start = total_visible.saturating_sub(MAX_VISIBLE).max(1);
            let thumb_top =
                ((page_start as f32 / max_start as f32) * (track_height - thumb_height)).round();
            let top_spacer = Element::new(&font, ElementContent::Text(String::new()))
                .item_type(UIItemType::DirJumpScrollTrack {
                    thumb_height: thumb_height as usize,
                })
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Pixels(SCROLLBAR_W * pt)))
                .min_height(Some(Dimension::Pixels(thumb_top)))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(0.10).into(),
                    text: fg.into(),
                });
            let thumb = Element::new(&font, ElementContent::Text(String::new()))
                .item_type(UIItemType::DirJumpScrollThumb {
                    track_top: 0,
                    track_height: track_height as usize,
                })
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Pixels(SCROLLBAR_W * pt)))
                .min_height(Some(Dimension::Pixels(thumb_height)))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: teal.mul_alpha(0.78).into(),
                    text: fg.into(),
                });
            let bottom_spacer = Element::new(&font, ElementContent::Text(String::new()))
                .item_type(UIItemType::DirJumpScrollTrack {
                    thumb_height: thumb_height as usize,
                })
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Pixels(SCROLLBAR_W * pt)))
                .min_height(Some(Dimension::Pixels(
                    (track_height - thumb_top - thumb_height).max(0.),
                )))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(0.10).into(),
                    text: fg.into(),
                });
            scrollbar_el = Some(
                Element::new(
                    &font,
                    ElementContent::Children(vec![top_spacer, thumb, bottom_spacer]),
                )
                .item_type(UIItemType::DirJumpScrollTrack {
                    thumb_height: thumb_height as usize,
                })
                .float(Float::Right)
                .margin(BoxDimension {
                    left: Dimension::Pixels(8. * pt),
                    right: Dimension::Pixels(8. * pt),
                    top: Dimension::Pixels(6. * pt),
                    bottom: Dimension::Pixels(0.),
                })
                .min_width(Some(Dimension::Pixels(SCROLLBAR_W * pt)))
                .min_height(Some(Dimension::Pixels(track_height)))
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(0.10).into(),
                    text: fg.into(),
                })
                .zindex(20),
            );
        }
        let mut last_section: Option<Section> = None;
        for (display_idx, &item_idx) in visible.iter().enumerate() {
            let item = &items[item_idx];
            // Section caption when the section changes (only for empty query).
            if input.is_empty() && last_section != Some(item.section) {
                last_section = Some(item.section);
                let caption = match item.section {
                    Section::Recent => crate::i18n::t("dirjump.recent"),
                    Section::Location => crate::i18n::t("dirjump.locations"),
                    Section::SubDir => format!(
                        "{} — {}",
                        crate::i18n::t("dirjump.subdirs"),
                        ellipsize_path_left(&base, row_cols.saturating_sub(12))
                    ),
                    // Only present while filtering, where captions are off.
                    Section::Deep | Section::PathComp => continue,
                };
                list_kids.push(
                    Element::new(&font, ElementContent::Text(caption))
                        .display(DisplayType::Block)
                        .padding(BoxDimension {
                            left: Dimension::Pixels(14. * pt),
                            right: Dimension::Pixels(14. * pt),
                            top: Dimension::Pixels(6. * pt),
                            bottom: Dimension::Pixels(2. * pt),
                        })
                        .colors(ElementColors {
                            border: BorderColor::default(),
                            bg: LinearRgba::TRANSPARENT.into(),
                            text: dim.into(),
                        }),
                );
            }

            let (row_bg, row_fg): (InheritableColor, InheritableColor) = if display_idx == selected
            {
                (hover_bg.into(), fg.into())
            } else {
                (LinearRgba::TRANSPARENT.into(), fg.into())
            };
            // Deep rows are labeled by their path relative to the browse
            // root — that's both what was matched and what tells the user
            // where they're jumping. Everything else shows the plain name.
            let label = if item.section == Section::Deep {
                item.rel.clone().unwrap_or_else(|| item.name.clone())
            } else {
                item.name.clone()
            };
            let label = ellipsize_path_left(&label, row_cols);
            // Greedy-subsequence match highlight: matched chars in teal.
            let mut row: Vec<Element> = vec![];
            if input.is_empty() {
                row.push(Element::new(&font, ElementContent::Text(label.clone())));
            } else {
                let lower_input: Vec<char> = input.to_lowercase().chars().collect();
                let mut ii = 0usize;
                let mut seg = String::new();
                let mut seg_hit = false;
                for ch in label.chars() {
                    let hit =
                        ii < lower_input.len() && ch.to_lowercase().next() == Some(lower_input[ii]);
                    if hit {
                        ii += 1;
                    }
                    if seg.is_empty() || hit == seg_hit {
                        seg_hit = hit;
                        seg.push(ch);
                    } else {
                        let color = if seg_hit { teal } else { fg };
                        row.push(
                            Element::new(&font, ElementContent::Text(seg.clone())).colors(
                                ElementColors {
                                    border: BorderColor::default(),
                                    bg: LinearRgba::TRANSPARENT.into(),
                                    text: color.into(),
                                },
                            ),
                        );
                        seg.clear();
                        seg.push(ch);
                        seg_hit = hit;
                    }
                }
                if !seg.is_empty() {
                    let color = if seg_hit { teal } else { fg };
                    row.push(Element::new(&font, ElementContent::Text(seg)).colors(
                        ElementColors {
                            border: BorderColor::default(),
                            bg: LinearRgba::TRANSPARENT.into(),
                            text: color.into(),
                        },
                    ));
                }
            }
            // Dim path hint: recents show where they live; path completion
            // shows the parent being completed in. (SubDir/Deep labels
            // already carry their location.)
            if matches!(
                item.section,
                Section::Recent | Section::Location | Section::PathComp
            ) {
                let hint = match item.section {
                    Section::PathComp => item.rel.clone().unwrap_or_else(|| tilde_path(&item.path)),
                    _ => tilde_path(&item.path),
                };
                // The hint floats right of the label on the same row;
                // its budget is what the label left over (4 cols gap).
                let hint = {
                    use termwiz::cell::unicode_column_width;
                    let used = unicode_column_width(&label, None);
                    ellipsize_path_left(&hint, row_cols.saturating_sub(used + 4))
                };
                row.push(
                    Element::new(&font, ElementContent::Text(hint))
                        .float(Float::Right)
                        .padding(BoxDimension {
                            left: Dimension::Pixels(24. * pt),
                            right: Dimension::Pixels(0.),
                            top: Dimension::Pixels(0.),
                            bottom: Dimension::Pixels(0.),
                        })
                        .zindex(10)
                        .colors(ElementColors {
                            border: BorderColor::default(),
                            bg: row_bg.clone(),
                            text: dim.into(),
                        }),
                );
            }
            let mut row_border = BorderColor::default();
            if display_idx == selected {
                row_border.left = teal;
            }
            // No hover_colors: hover moves the selection itself (see
            // hover_select), so the selected style IS the hover style —
            // a second renderer-native highlight would bring back the
            // two-cursors-at-once artifact.
            list_kids.push(
                Element::new(&font, ElementContent::Children(row))
                    .item_type(UIItemType::DirJumpRow(display_idx))
                    .colors(ElementColors {
                        border: row_border,
                        bg: row_bg,
                        text: row_fg,
                    })
                    .border(BoxDimension {
                        left: Dimension::Pixels(2. * pt),
                        right: Dimension::Pixels(0.),
                        top: Dimension::Pixels(0.),
                        bottom: Dimension::Pixels(0.),
                    })
                    .padding(BoxDimension {
                        left: Dimension::Pixels(12. * pt),
                        right: Dimension::Pixels(14. * pt),
                        top: Dimension::Pixels(5. * pt),
                        bottom: Dimension::Pixels(5. * pt),
                    })
                    .min_width(Some(Dimension::Percent(1.)))
                    .display(DisplayType::Block),
            );
        }

        let overflow = *self.overflow.borrow();
        if total_visible > MAX_VISIBLE {
            let page_start = *self.page_start.borrow();
            let from = page_start + 1;
            let to = page_start + visible.len();
            list_kids.push(
                Element::new(
                    &font,
                    ElementContent::Text(crate::i18n::t_args(
                        "dirjump.page",
                        &[
                            ("from", &from.to_string()),
                            ("to", &to.to_string()),
                            ("total", &total_visible.to_string()),
                            ("more", &overflow.to_string()),
                        ],
                    )),
                )
                .display(DisplayType::Block)
                .padding(BoxDimension {
                    left: Dimension::Pixels(14. * pt),
                    right: Dimension::Pixels(14. * pt),
                    top: Dimension::Pixels(4. * pt),
                    bottom: Dimension::Pixels(2. * pt),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                }),
            );
        }

        if visible.is_empty() {
            list_kids.push(
                Element::new(&font, ElementContent::Text(crate::i18n::t("dirjump.empty")))
                    .display(DisplayType::Block)
                    .padding(BoxDimension {
                        left: Dimension::Pixels(14. * pt),
                        right: Dimension::Pixels(14. * pt),
                        top: Dimension::Pixels(8. * pt),
                        bottom: Dimension::Pixels(8. * pt),
                    })
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: dim.into(),
                    }),
            );
        }

        // Scrollable list area: the rows in a block container, with the
        // scrollbar floated beside them. Appending the float AFTER the row
        // container (instead of before, as a block sibling) keeps the box
        // model from pushing the rows — and the footer — down by the
        // scrollbar's height.
        let mut list_area_kids: Vec<Element> = vec![Element::new(
            &font,
            ElementContent::Children(std::mem::take(&mut list_kids)),
        )
        .display(DisplayType::Block)
        .min_width(Some(Dimension::Percent(1.)))];
        if let Some(sb) = scrollbar_el.take() {
            list_area_kids.push(sb);
        }
        children.push(
            Element::new(&font, ElementContent::Children(list_area_kids))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.))),
        );

        // Footer hints.
        children.push(
            Element::new(&font, ElementContent::Text(String::new()))
                .display(DisplayType::Block)
                .min_width(Some(Dimension::Percent(1.)))
                .line_height(Some(0.08))
                .margin(BoxDimension {
                    left: Dimension::Pixels(0.),
                    right: Dimension::Pixels(0.),
                    top: Dimension::Pixels(4. * pt),
                    bottom: Dimension::Pixels(0.),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: fg.mul_alpha(0.12).into(),
                    text: fg.into(),
                }),
        );
        let hints_key = if cfg!(target_os = "macos") {
            "dirjump.hints_macos"
        } else {
            "dirjump.hints"
        };
        children.push(
            Element::new(&font, ElementContent::Text(crate::i18n::t(hints_key)))
                .display(DisplayType::Block)
                .padding(BoxDimension {
                    left: Dimension::Pixels(14. * pt),
                    right: Dimension::Pixels(14. * pt),
                    top: Dimension::Pixels(6. * pt),
                    bottom: Dimension::Pixels(2. * pt),
                })
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: dim.into(),
                }),
        );

        let card = Element::new(&font, ElementContent::Children(children))
            .item_type(UIItemType::PopupMenuCard)
            .colors(ElementColors {
                border: BorderColor::new(LinearRgba::with_srgba(0x00, 0x00, 0x00, 0xb0)),
                bg: bg.into(),
                text: fg.into(),
            })
            .padding(BoxDimension {
                left: Dimension::Pixels(0.),
                right: Dimension::Pixels(0.),
                top: Dimension::Pixels(6. * pt),
                bottom: Dimension::Pixels(6. * pt),
            })
            .border(BoxDimension::new(Dimension::Pixels(1.)))
            .border_corners(Some(Corners {
                top_left: SizedPoly {
                    width: Dimension::Pixels(6. * pt),
                    height: Dimension::Pixels(6. * pt),
                    poly: TOP_LEFT_ROUNDED_CORNER,
                },
                top_right: SizedPoly {
                    width: Dimension::Pixels(6. * pt),
                    height: Dimension::Pixels(6. * pt),
                    poly: TOP_RIGHT_ROUNDED_CORNER,
                },
                bottom_left: SizedPoly {
                    width: Dimension::Pixels(6. * pt),
                    height: Dimension::Pixels(6. * pt),
                    poly: BOTTOM_LEFT_ROUNDED_CORNER,
                },
                bottom_right: SizedPoly {
                    width: Dimension::Pixels(6. * pt),
                    height: Dimension::Pixels(6. * pt),
                    poly: BOTTOM_RIGHT_ROUNDED_CORNER,
                },
            }));

        let dimensions = term_window.dimensions;
        let border = term_window.get_os_border();
        let top_bar_height = if term_window.show_tab_bar && !term_window.config.tab_bar_at_bottom {
            term_window.tab_bar_pixel_height().unwrap_or(0.)
        } else {
            0.
        };
        let width = card_width;
        let x = ((dimensions.pixel_width as f32 - width) / 2.).round();
        let y = (top_bar_height + border.top.get() as f32 + 28. * pt).round();

        let computed = term_window.compute_element(
            &LayoutContext {
                height: DimensionContext {
                    dpi: dimensions.dpi as f32,
                    pixel_max: dimensions.pixel_height as f32,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: DimensionContext {
                    dpi: dimensions.dpi as f32,
                    pixel_max: dimensions.pixel_width as f32,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(x, y, width, dimensions.pixel_height as f32 - y),
                metrics: &metrics,
                gl_state: term_window.render_state.as_ref().unwrap(),
                zindex: 100,
            },
            &card,
        )?;
        Ok(vec![computed])
    }
}

impl Modal for DirJump {
    fn mouse_event(&self, _event: MouseEvent, _term_window: &mut TermWindow) -> anyhow::Result<()> {
        Ok(())
    }

    fn key_down(
        &self,
        key: KeyCode,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool> {
        match (key, mods) {
            (KeyCode::Escape, KeyModifiers::NONE) => term_window.cancel_modal(),
            (KeyCode::UpArrow, KeyModifiers::NONE) => self.move_selection(-1, term_window),
            (KeyCode::DownArrow, KeyModifiers::NONE) => self.move_selection(1, term_window),
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                self.scroll_results(-(MAX_VISIBLE as isize), term_window)
            }
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                self.scroll_results(MAX_VISIBLE as isize, term_window)
            }
            (KeyCode::Tab, KeyModifiers::NONE) => self.complete_selected(term_window),
            (KeyCode::Enter, KeyModifiers::NONE) => {
                let sel = *self.selected.borrow();
                self.activate(sel, term_window, false);
            }
            (KeyCode::Enter, KeyModifiers::SUPER) | (KeyCode::Enter, KeyModifiers::CTRL) => {
                let sel = *self.selected.borrow();
                self.activate(sel, term_window, true);
            }
            (KeyCode::Char('o'), KeyModifiers::SUPER) => {
                self.open_system_picker(term_window);
            }
            (KeyCode::Char('u'), KeyModifiers::CTRL) => {
                self.input.borrow_mut().clear();
                self.refilter();
                self.invalidate(term_window);
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                let emptied = {
                    let mut input = self.input.borrow_mut();
                    if input.pop().is_none() {
                        true
                    } else {
                        false
                    }
                };
                if emptied {
                    self.ascend(term_window);
                } else {
                    self.refilter();
                    self.invalidate(term_window);
                }
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.input.borrow_mut().push(c);
                self.refilter();
                self.invalidate(term_window);
            }
            _ => {}
        }
        Ok(true)
    }

    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<Ref<'_, [ComputedElement]>> {
        if self.element.borrow().is_none() {
            let element = self.compute(term_window)?;
            self.element.borrow_mut().replace(element);
        }
        Ok(Ref::map(self.element.borrow(), |v| {
            v.as_ref().unwrap().as_slice()
        }))
    }

    fn reconfigure(&self, _term_window: &mut TermWindow) {
        self.element.borrow_mut().take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_scan_finds_nested_and_skips_noise() {
        let tmp = std::env::temp_dir().join(format!("djtest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        for d in [
            "a/b/c",
            "src/termwindow/render",
            "node_modules/x",
            ".git/objects",
            "a/target/debug",
        ] {
            std::fs::create_dir_all(tmp.join(d)).unwrap();
        }
        let rels: Vec<String> = deep_scan(&tmp).into_iter().map(|(_, r)| r).collect();
        // nested dirs found by relative path
        assert!(rels.contains(&"a/b".to_string()), "{:?}", rels);
        assert!(rels.contains(&"a/b/c".to_string()));
        assert!(rels.contains(&"src/termwindow".to_string()));
        assert!(rels.contains(&"src/termwindow/render".to_string()));
        // depth-0 children excluded (SubDir section already lists them)
        assert!(!rels.contains(&"a".to_string()));
        assert!(!rels.contains(&"src".to_string()));
        // noise pruned
        assert!(!rels.iter().any(|r| r.contains("node_modules")));
        assert!(!rels.iter().any(|r| r.contains(".git")));
        assert!(!rels.iter().any(|r| r.contains("target")));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn ellipsize_keeps_path_tail() {
        assert_eq!(ellipsize_path_left("abc", 10), "abc");
        assert_eq!(
            ellipsize_path_left("src/termwindow/render", 10),
            "…ow/render"
        );
        assert_eq!(ellipsize_path_left("x", 0), "");
    }

    #[test]
    fn tilde_expansion() {
        if let Some(home) = dirs_next::home_dir() {
            assert_eq!(expand_tilde("~/x"), format!("{}/x", home.display()));
        }
        assert_eq!(expand_tilde("/abs/x"), "/abs/x");
    }

    #[test]
    fn thumb_position_maps_to_result_page() {
        assert_eq!(page_start_for_thumb_top(0, 0, 140, 40, 34), 0);
        assert_eq!(page_start_for_thumb_top(100, 0, 140, 40, 34), 20);
        assert_eq!(page_start_for_thumb_top(240, 40, 180, 40, 54), 40);
        assert_eq!(page_start_for_thumb_top(0, 0, 140, 40, MAX_VISIBLE), 0);
    }
}

#[cfg(test)]
mod win_tests {
    use super::*;

    #[test]
    fn windows_path_query_splitting() {
        // bare drive promotes to root
        assert_eq!(
            split_path_query("D:", true),
            Some(("D:/".into(), "".into()))
        );
        // backslashes normalize; parent keeps the drive-root slash
        assert_eq!(
            split_path_query("D:\\code", true),
            Some(("D:/".into(), "code".into()))
        );
        assert_eq!(
            split_path_query("D:/code/un", true),
            Some(("D:/code".into(), "un".into()))
        );
        // unix splitting unchanged
        assert_eq!(
            split_path_query("/usr/lo", false),
            Some(("/usr".into(), "lo".into()))
        );
        assert_eq!(split_path_query("/", false), Some(("/".into(), "".into())));
        assert_eq!(split_path_query("plain", false), None);
    }
    #[test]
    fn path_query_detection() {
        assert!(is_path_query("/usr"));
        assert!(is_path_query("~/code"));
        assert!(is_path_query("D:"));
        assert!(is_path_query("d:\\code"));
        assert!(is_path_query("D:/code"));
        assert!(is_path_query("\\\\server\\share"));
        assert!(!is_path_query("dev"));
        assert!(!is_path_query("term ren"));
    }
}
