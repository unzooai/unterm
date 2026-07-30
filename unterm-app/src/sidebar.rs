//! The list of tabs down the left-hand side.
//!
//! Ported from the previous front end's `left_tab_bar.rs`. A vertical strip
//! reads a tab list better than a horizontal one does: a tab's label is a
//! path and a command, which fit along a row and not across one, and there is
//! room for the state a top strip has to leave out.
//!
//! Tabs are grouped by the directory their pane is in, and the grouping is
//! derived rather than configured -- the point being that a person running
//! three projects gets three groups without setting anything up. Two projects
//! whose folders share a name are told apart by the shortest parent path that
//! distinguishes them, which is the fiddly part and the reason that algorithm
//! came across verbatim rather than being re-derived.

use std::collections::HashMap;

/// The strip's width in pixels, or nothing when it is closed.
///
/// Measured in points against the display rather than in terminal cells: the
/// strip is chrome, and a panel that changes width when somebody zooms the
/// terminal is a panel that moves for no reason.
pub fn width(open: bool, requested_points: Option<f32>, window_width: f32, scale: f32) -> f32 {
    if !open {
        return 0.0;
    }
    let pt = crate::chrome_font::point(scale);
    let window_points = window_width / pt.max(0.001);
    let widest = (window_points * crate::ui_tokens::LEFT_TAB_BAR_MAX_RATIO)
        .max(crate::ui_tokens::LEFT_TAB_BAR_MIN_WIDTH);
    let points = requested_points
        .unwrap_or(crate::ui_tokens::LEFT_TAB_BAR_WIDTH)
        .clamp(crate::ui_tokens::LEFT_TAB_BAR_MIN_WIDTH, widest);
    (points * pt).round()
}

/// The icon a tab row leads with.
///
/// Ported unchanged from the previous front end, code points included. They
/// come from the bundled symbols face -- a robot for an AI agent's pane, the
/// platform's own terminal mark for each shell -- and they are what makes a row
/// identifiable before its text is read.
pub fn shell_icon(title: &str) -> char {
    let lower = title.to_lowercase();
    let has = |name: &str| lower.contains(name);
    if [
        "claude",
        "codex",
        "gemini",
        "kimi",
        "aider",
        "opencode",
        "trae",
        "zcode",
        "cursor agent",
    ]
    .iter()
    .any(|name| has(name))
    {
        return ROBOT;
    }
    if has("pwsh") || has("powershell") {
        '\u{ebc7}'
    } else if has("cmd.exe") || has("command prompt") || has("cmd") {
        '\u{ebc4}'
    } else if has("bash") {
        '\u{ebca}'
    } else if has("ssh") {
        '\u{eb3a}'
    } else {
        // A generic terminal beats an empty leading slot: a row with no icon
        // reads as a row whose icon failed to load.
        '\u{ea85}'
    }
}

/// An AI agent's pane.
pub const ROBOT: char = '\u{f06a9}';
/// The folder a project row leads with.
pub const FOLDER: char = '\u{f07b}';
/// The disclosure arrows, closed and open.
pub const CLOSED: char = '\u{25B8}';
pub const OPEN: char = '\u{25BE}';

/// Shorten a label to `columns`, keeping its start.
///
/// A row's beginning is its name, which is what it is identified by; a project
/// called `unterm-experiments` cut from the front is unrecognisable.
pub fn fit(text: &str, columns: usize) -> String {
    if columns == 0 {
        return String::new();
    }
    let width: usize = text.chars().map(crate::terminal::column_width).sum();
    if width <= columns {
        return text.to_string();
    }
    let mut kept = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let wide = crate::terminal::column_width(ch);
        if used + wide > columns.saturating_sub(1) {
            break;
        }
        kept.push(ch);
        used += wide;
    }
    format!("{kept}\u{2026}")
}

/// What one line of the strip is.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// The project a run of tabs belongs to. Only when there is more than one.
    Group {
        key: String,
        label: String,
        /// The parent path that tells this project from another of the same
        /// name, when one is needed.
        hint: Option<String>,
        count: usize,
        /// Whether its tabs are folded away. Window state, not disk state: it
        /// survives repaints and resizes without touching the filesystem.
        collapsed: bool,
        /// Whether the tab in front belongs to it.
        active: bool,
    },
    Tab {
        index: usize,
        label: String,
        /// What is running in it, if anything is.
        detail: Option<String>,
        active: bool,
        /// The icon it leads with.
        icon: char,
        /// Whether it sits under a project header, and is therefore indented.
        grouped: bool,
    },
}

/// One tab, as the strip needs to know it.
#[derive(Clone, Debug, PartialEq)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
    /// The AI agent bound to the pane, if one is. The agent's name *is* the
    /// row's title when there is one -- the pane title just repeats it.
    pub agent: Option<String>,
    /// The full working directory, which is the project's identity: two
    /// folders called `app` in different places are two projects.
    pub cwd: Option<String>,
    /// The command running in front of the shell, if one is.
    pub foreground: Option<String>,
    pub active: bool,
}

/// Build the strip's lines.
pub fn rows(tabs: &[TabInfo], collapsed: &std::collections::HashSet<String>) -> Vec<Row> {
    let projects: Vec<(String, String)> = {
        let mut seen = Vec::new();
        for tab in tabs {
            let Some(cwd) = tab.cwd.as_deref() else { continue };
            let key = project_key(cwd);
            if !seen.iter().any(|(existing, _)| *existing == key) {
                seen.push((key, cwd.to_string()));
            }
        }
        seen
    };
    let hints = shortest_unique_parent_hints(&projects);

    // One project needs no headers: a header above every tab is a header that
    // says nothing.
    let grouped = projects.len() > 1;
    let mut rows = Vec::new();
    let mut done: Vec<String> = Vec::new();

    for tab in tabs {
        let key = tab.cwd.as_deref().map(project_key);
        if grouped {
            if let Some(key) = &key {
                if !done.contains(key) {
                    done.push(key.clone());
                    rows.push(Row::Group {
                        label: leaf(key),
                        hint: hints.get(key).cloned(),
                        count: tabs
                            .iter()
                            .filter(|other| {
                                other.cwd.as_deref().map(project_key).as_ref() == Some(key)
                            })
                            .count(),
                        collapsed: collapsed.contains(key),
                        active: tabs.iter().any(|other| {
                            other.active
                                && other.cwd.as_deref().map(project_key).as_ref() == Some(key)
                        }),
                        key: key.clone(),
                    });
                }
            }
        }
        // A collapsed project shows its header and nothing under it -- except
        // the tab in front, which must never disappear: a window with no
        // visible selection reads as a window that lost track of itself.
        let folded = grouped
            && key
                .as_ref()
                .map(|key| collapsed.contains(key))
                .unwrap_or(false);
        if folded && !tab.active {
            continue;
        }
        rows.push(Row::Tab {
            index: tab.index,
            label: label_for(tab),
            detail: tab.foreground.clone(),
            active: tab.active,
            icon: shell_icon(tab.agent.as_deref().unwrap_or(&tab.title)),
            grouped,
        });
    }
    rows
}

/// What a tab's line says.
///
/// The command in front of the shell first: three shells in one project are
/// told apart by what they are doing, not by all being called the same thing.
fn label_for(tab: &TabInfo) -> String {
    use unterm_engine::next_core::tab_title::{resolve_name, TabContext, TabTitleRules};

    // An agent's name is the title. The pane title just repeats it, so showing
    // both is showing the same word twice on a row with no room for it.
    if let Some(agent) = tab.agent.as_deref().filter(|name| !name.trim().is_empty()) {
        return agent.to_string();
    }

    let rules = TabTitleRules {
        capitalize: false,
        ..TabTitleRules::default()
    };
    let resolved = resolve_name(
        &rules,
        TabContext {
            pane_title: &tab.title,
            process_path: "",
            index: tab.index,
        },
    );
    if resolved.trim().is_empty() {
        format!("{}", tab.index + 1)
    } else {
        resolved
    }
}

/// A project's identity: its path, compared the way the platform compares
/// paths.
fn project_key(cwd: &str) -> String {
    let normalised = cwd.replace('\\', "/");
    let trimmed = normalised.trim_end_matches('/');
    if cfg!(windows) {
        trimmed.to_lowercase()
    } else {
        trimmed.to_string()
    }
}

/// The last component of a path -- what a project is called.
fn leaf(path: &str) -> String {
    // Home is "Home", not whatever the account is called. It is the one project
    // everybody has and the one whose folder name says nothing about it: a
    // header reading `lixd2` names the machine's owner rather than the project.
    if is_home(path) {
        return "Home".to_string();
    }
    path.replace('\\', "/")
        .trim_end_matches('/')
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .to_string()
}

/// Whether a path *is* the home directory, rather than something inside it.
fn is_home(path: &str) -> bool {
    let Some(home) = dirs_next::home_dir() else {
        return false;
    };
    let tidy = |text: &str| text.replace('\\', "/").trim_end_matches('/').to_lowercase();
    tidy(path) == tidy(&home.display().to_string())
}

/// The shortest parent path that tells same-named projects apart.
///
/// Ported whole. Two folders called `app` need something in front of them or
/// the list has two identical headers; the shortest suffix that is unique is
/// the least noise that does the job. When no suffix is unique -- `/acme/app`
/// against `/work/acme/app`, where one path's components are a suffix of the
/// other's -- the immediate parent is used, so the pair still reads
/// differently even though neither is strictly unique.
fn shortest_unique_parent_hints(projects: &[(String, String)]) -> HashMap<String, String> {
    let components = |path: &str| {
        path.replace('\\', "/")
            .trim_end_matches('/')
            .split('/')
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let comparable = |value: &str| {
        if cfg!(windows) {
            value.to_lowercase()
        } else {
            value.to_string()
        }
    };
    let mut result = HashMap::new();

    for (key, path) in projects {
        let parts = components(path);
        let Some(leaf) = parts.last() else { continue };
        let peers: Vec<Vec<String>> = projects
            .iter()
            .filter_map(|(_, peer_path)| {
                let peer = components(peer_path);
                peer.last()
                    .is_some_and(|name| comparable(name) == comparable(leaf))
                    .then_some(peer)
            })
            .collect();
        if peers.len() < 2 || parts.len() < 2 {
            continue;
        }

        let mut disambiguated = false;
        for suffix_len in 2..=parts.len() {
            let suffix = parts[parts.len() - suffix_len..].join("/");
            let matches = peers
                .iter()
                .filter(|peer| {
                    peer.len() >= suffix_len
                        && comparable(&peer[peer.len() - suffix_len..].join("/"))
                            == comparable(&suffix)
                })
                .count();
            if matches == 1 {
                result.insert(
                    key.clone(),
                    parts[parts.len() - suffix_len..parts.len() - 1].join("/"),
                );
                disambiguated = true;
                break;
            }
        }
        if !disambiguated {
            result.insert(key.clone(), parts[parts.len() - 2].clone());
        }
    }
    result
}

/// How many of the strip's lines fit, and which one is at the top.
///
/// Clamped so a list that shrinks -- a tab closed, a group collapsed -- cannot
/// leave the strip scrolled past its own end showing nothing.
pub fn clamp_scroll(scroll_top: usize, rows: usize, visible: usize) -> usize {
    scroll_top.min(rows.saturating_sub(visible))
}

/// Scroll far enough to bring `row` into view, moving as little as possible.
pub fn scroll_to_show(scroll_top: usize, row: usize, visible: usize) -> usize {
    if visible == 0 {
        return scroll_top;
    }
    if row < scroll_top {
        row
    } else if row >= scroll_top + visible {
        row + 1 - visible
    } else {
        scroll_top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rows with nothing folded away, which is what every test here wants.
    fn rows_of(tabs: &[TabInfo]) -> Vec<Row> {
        rows(tabs, &std::collections::HashSet::new())
    }

    /// What a row says, for the tests that assert on its text. The window
    /// draws the pieces separately -- the count has to float right -- so this
    /// is the tests' own joining rather than something the painter uses.
    fn text_of(row: &Row) -> String {
        match row {
            Row::Group { label, hint, count, .. } => match hint {
                Some(hint) => format!("{hint}/{label}  {count}"),
                None => format!("{label}  {count}"),
            },
            Row::Tab { index, label, detail, .. } => match detail {
                Some(detail) => format!(" {}  {label}  {detail}", index + 1),
                None => format!(" {}  {label}", index + 1),
            },
        }
    }

    fn tab(index: usize, title: &str, cwd: Option<&str>) -> TabInfo {
        TabInfo {
            index,
            title: title.to_string(),
            agent: None,
            cwd: cwd.map(str::to_string),
            foreground: None,
            active: index == 0,
        }
    }

    fn labels(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Group { label, hint, .. } => match hint {
                    Some(hint) => format!("[{hint}/{label}]"),
                    None => format!("[{label}]"),
                },
                Row::Tab { label, .. } => label.clone(),
            })
            .collect()
    }

    /// Every row fits the strip: one that runs past its width is drawn over
    /// the terminal beside it.
    #[test]
    fn no_row_is_wider_than_the_strip() {
        let rows = rows_of(&[
            TabInfo {
                index: 0,
                title: "a very long shell name that goes on".to_string(),
                agent: None,
                cwd: Some("/work/some/deeply/nested/project".to_string()),
                foreground: Some("npm run dev --workspace=everything".to_string()),
                active: true,
            },
            tab(1, "pwsh", Some("/elsewhere/project")),
        ]);
        // Twenty-two columns is about what the default strip holds.
        const ROOM: usize = 22;
        for row in &rows {
            let fitted = fit(&text_of(row), ROOM);
            let wide: usize = fitted.chars().map(crate::terminal::column_width).sum();
            assert!(wide <= ROOM, "{fitted:?} is {wide} wide");
        }
    }

    /// A tab's line leads with its number, so it can be found by the key that
    /// selects it.
    #[test]
    fn a_tabs_line_starts_with_its_number() {
        let rows = rows_of(&[tab(0, "pwsh", Some("/work/app"))]);
        let text = text_of(&rows[0]);
        assert!(text.trim_start().starts_with('1'), "{text:?}");
    }

    /// What is running comes after the name: three shells in one project are
    /// told apart by what they are doing.
    #[test]
    fn a_running_command_is_shown_beside_the_name() {
        let rows = rows_of(&[TabInfo {
            index: 0,
            title: "pwsh".to_string(),
            agent: None,
            cwd: Some("/work/app".to_string()),
            foreground: Some("cargo test".to_string()),
            active: true,
        }]);
        assert!(text_of(&rows[0]).contains("cargo test"));
    }

    /// A closed strip takes nothing, and an open one takes the default width
    /// -- in points against the display, not in terminal cells: the strip is
    /// chrome, and it must not move when the terminal is zoomed.
    #[test]
    fn a_closed_strip_takes_no_width() {
        assert_eq!(width(false, None, 1600.0, 1.0), 0.0);
        let open = width(true, None, 1600.0, 1.0);
        assert!(open > 0.0);
        // The same window at a bigger scale gives a proportionally wider strip.
        let scaled = width(true, None, 2400.0, 1.5);
        assert!(
            (scaled / open - 1.5).abs() < 0.05,
            "{open} then {scaled}"
        );
    }

    /// It never takes more of the window than the budget allows, however wide
    /// somebody drags it: a strip that is most of the window is not a strip.
    #[test]
    fn the_strip_never_takes_more_than_its_share() {
        for window in [400.0, 800.0, 1600.0, 3840.0] {
            let widest = width(true, Some(10_000.0), window, 1.0);
            assert!(
                widest <= window * crate::ui_tokens::LEFT_TAB_BAR_MAX_RATIO + 1.0
                    || widest <= crate::ui_tokens::LEFT_TAB_BAR_MIN_WIDTH
                        * crate::chrome_font::point(1.0)
                        + 1.0,
                "a {window}px window gave a {widest}px strip"
            );
        }
    }

    /// And never so narrow that a project name cannot be read.
    #[test]
    fn the_strip_never_collapses_to_nothing_while_open() {
        for window in [200.0, 600.0, 1600.0] {
            assert!(width(true, Some(0.0), window, 1.0) > 0.0);
        }
    }

    /// One project needs no headers: a header above every tab says nothing.
    #[test]
    fn a_single_project_gets_no_group_headers() {
        let rows = rows_of(&[
            tab(0, "pwsh", Some("/home/me/app")),
            tab(1, "pwsh", Some("/home/me/app")),
        ]);
        assert!(
            !rows.iter().any(|row| matches!(row, Row::Group { .. })),
            "{:?}",
            labels(&rows)
        );
        assert_eq!(rows.len(), 2);
    }

    /// Two projects get one header each, above their own tabs.
    #[test]
    fn two_projects_are_grouped() {
        let rows = rows_of(&[
            tab(0, "pwsh", Some("/home/me/alpha")),
            tab(1, "pwsh", Some("/home/me/beta")),
        ]);
        let labels = labels(&rows);
        assert_eq!(labels.len(), 4, "{labels:?}");
        assert!(labels[0].starts_with('['), "{labels:?}");
        assert!(labels[2].starts_with('['), "{labels:?}");
    }

    /// Two folders with the same name are told apart by the shortest parent
    /// that distinguishes them -- not by the whole path, which is noise.
    #[test]
    fn same_named_projects_are_told_apart_by_the_shortest_parent() {
        let rows = rows_of(&[
            tab(0, "pwsh", Some("/work/acme/app")),
            tab(1, "pwsh", Some("/work/globex/app")),
        ]);
        let labels = labels(&rows);
        assert!(labels.contains(&"[acme/app]".to_string()), "{labels:?}");
        assert!(labels.contains(&"[globex/app]".to_string()), "{labels:?}");
    }

    /// And a project whose path is a suffix of another's still reads
    /// differently, even though no suffix of it is unique.
    #[test]
    fn a_path_that_is_a_suffix_of_another_still_gets_a_hint() {
        let rows = rows_of(&[
            tab(0, "pwsh", Some("/acme/app")),
            tab(1, "pwsh", Some("/work/acme/app")),
        ]);
        let hints: Vec<String> = rows
            .iter()
            .filter_map(|row| match row {
                Row::Group { hint, .. } => hint.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(hints.len(), 2, "both need one: {hints:?}");
        assert_ne!(hints[0], hints[1], "and they have to differ: {hints:?}");
    }

    /// Differently-named projects need no hint at all.
    #[test]
    fn distinct_names_are_left_alone() {
        let rows = rows_of(&[
            tab(0, "pwsh", Some("/work/alpha")),
            tab(1, "pwsh", Some("/work/beta")),
        ]);
        for row in &rows {
            if let Row::Group { hint, label, .. } = row {
                assert!(hint.is_none(), "{label} did not need a hint");
            }
        }
    }

    /// A tab with no directory still gets a line: a pane whose shell has not
    /// reported one yet is not a pane to hide.
    #[test]
    fn a_tab_with_no_directory_is_still_listed() {
        let rows = rows_of(&[tab(0, "pwsh", None), tab(1, "pwsh", Some("/work/app"))]);
        let tabs = rows.iter().filter(|row| matches!(row, Row::Tab { .. })).count();
        assert_eq!(tabs, 2, "{:?}", labels(&rows));
    }

    #[test]
    fn a_group_counts_its_own_tabs() {
        let rows = rows_of(&[
            tab(0, "pwsh", Some("/work/alpha")),
            tab(1, "pwsh", Some("/work/alpha")),
            tab(2, "pwsh", Some("/work/beta")),
        ]);
        let counts: Vec<usize> = rows
            .iter()
            .filter_map(|row| match row {
                Row::Group { count, .. } => Some(*count),
                _ => None,
            })
            .collect();
        assert_eq!(counts, vec![2, 1]);
    }

    /// A list that shrinks must not leave the strip scrolled past its end
    /// showing nothing.
    #[test]
    fn scrolling_cannot_run_off_the_end() {
        assert_eq!(clamp_scroll(20, 5, 10), 0);
        assert_eq!(clamp_scroll(3, 20, 10), 3);
        assert_eq!(clamp_scroll(15, 20, 10), 10);
    }

    /// Bringing a row into view moves as little as it can.
    #[test]
    fn showing_a_row_moves_the_least_it_can() {
        assert_eq!(scroll_to_show(5, 7, 10), 5, "already visible");
        assert_eq!(scroll_to_show(5, 2, 10), 2, "above: scroll up to it");
        assert_eq!(scroll_to_show(0, 12, 10), 3, "below: just far enough");
    }

    #[test]
    fn a_strip_with_no_room_does_not_scroll() {
        assert_eq!(scroll_to_show(4, 99, 0), 4);
    }
}

/// Whether two names are the same program spelled differently.
///
/// `cmd` and `cmd.exe`, `pwsh` and `pwsh.exe`, `bash` and `/usr/bin/bash`. A
/// row that shows both says one thing twice, on a row with no room for it.
pub fn same_program(a: &str, b: &str) -> bool {
    let bare = |name: &str| {
        name.trim()
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .trim_end_matches(".exe")
            .trim_end_matches(".EXE")
            .to_lowercase()
    };
    let (a, b) = (bare(a), bare(b));
    !a.is_empty() && a == b
}

#[cfg(test)]
mod naming_tests {
    use super::*;

    /// `cmd` and `cmd.exe` are one program, and a row that shows both says one
    /// thing twice.
    #[test]
    fn a_program_is_recognised_however_it_is_spelled() {
        assert!(same_program("cmd.exe", "cmd"));
        assert!(same_program("PWSH.EXE", "pwsh"));
        assert!(same_program("/usr/bin/bash", "bash"));
        assert!(same_program("C:\\Windows\\System32\\cmd.exe", "cmd"));
    }

    /// And two different programs are two different programs, which is the
    /// case the second half of the row exists for.
    #[test]
    fn two_programs_stay_two() {
        assert!(!same_program("cargo", "pwsh"));
        assert!(!same_program("npm run dev", "pwsh"));
        assert!(!same_program("", "pwsh"));
    }

    /// The home directory is "Home". A header reading the account name names
    /// the machine's owner rather than the project.
    #[test]
    fn the_home_directory_is_called_home() {
        let Some(home) = dirs_next::home_dir() else {
            return;
        };
        assert_eq!(leaf(&home.display().to_string()), "Home");
        // And something *inside* home keeps its own name.
        let inside = home.join("projects").join("unterm");
        assert_eq!(leaf(&inside.display().to_string()), "unterm");
    }
}
