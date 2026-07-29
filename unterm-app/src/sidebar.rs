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

/// How wide the strip is, in columns.
///
/// Wide enough for a project name and a command beside it, narrow enough that
/// turning it on does not cost half the terminal.
pub const COLUMNS: usize = 22;

/// The strip's width in pixels, or nothing when it is closed.
pub fn width(open: bool, metrics: unterm_render::quads::CellMetrics) -> f32 {
    if open {
        COLUMNS as f32 * metrics.width
    } else {
        0.0
    }
}

/// What a row says, fitted to the strip.
pub fn text_for(row: &Row, columns: usize) -> String {
    let text = match row {
        Row::Group { label, hint, count, .. } => match hint {
            Some(hint) => format!("{hint}/{label}  {count}"),
            None => format!("{label}  {count}"),
        },
        Row::Tab { index, label, detail, .. } => match detail {
            Some(detail) => format!(" {}  {label}  {detail}", index + 1),
            None => format!(" {}  {label}", index + 1),
        },
    };
    // Cut from the end: a row's beginning is its number and its name, which
    // are what it is identified by.
    if text.chars().count() <= columns {
        text
    } else {
        let kept: String = text.chars().take(columns.saturating_sub(1)).collect();
        format!("{kept}\u{2026}")
    }
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
    },
    Tab {
        index: usize,
        label: String,
        /// What is running in it, if anything is.
        detail: Option<String>,
        active: bool,
    },
}

/// One tab, as the strip needs to know it.
#[derive(Clone, Debug, PartialEq)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
    /// The full working directory, which is the project's identity: two
    /// folders called `app` in different places are two projects.
    pub cwd: Option<String>,
    /// The command running in front of the shell, if one is.
    pub foreground: Option<String>,
    pub active: bool,
}

/// Build the strip's lines.
pub fn rows(tabs: &[TabInfo]) -> Vec<Row> {
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
                        key: key.clone(),
                    });
                }
            }
        }
        rows.push(Row::Tab {
            index: tab.index,
            label: label_for(tab),
            detail: tab.foreground.clone(),
            active: tab.active,
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
    path.replace('\\', "/")
        .trim_end_matches('/')
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .to_string()
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

    fn tab(index: usize, title: &str, cwd: Option<&str>) -> TabInfo {
        TabInfo {
            index,
            title: title.to_string(),
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
        let rows = rows(&[
            TabInfo {
                index: 0,
                title: "a very long shell name that goes on".to_string(),
                cwd: Some("/work/some/deeply/nested/project".to_string()),
                foreground: Some("npm run dev --workspace=everything".to_string()),
                active: true,
            },
            tab(1, "pwsh", Some("/elsewhere/project")),
        ]);
        for row in &rows {
            let text = text_for(row, COLUMNS);
            assert!(
                text.chars().count() <= COLUMNS,
                "{text:?} is {} wide",
                text.chars().count()
            );
        }
    }

    /// A tab's line leads with its number, so it can be found by the key that
    /// selects it.
    #[test]
    fn a_tabs_line_starts_with_its_number() {
        let rows = rows(&[tab(0, "pwsh", Some("/work/app"))]);
        let text = text_for(&rows[0], COLUMNS);
        assert!(text.trim_start().starts_with('1'), "{text:?}");
    }

    /// What is running comes after the name: three shells in one project are
    /// told apart by what they are doing.
    #[test]
    fn a_running_command_is_shown_beside_the_name() {
        let rows = rows(&[TabInfo {
            index: 0,
            title: "pwsh".to_string(),
            cwd: Some("/work/app".to_string()),
            foreground: Some("cargo test".to_string()),
            active: true,
        }]);
        assert!(text_for(&rows[0], 40).contains("cargo test"));
    }

    #[test]
    fn a_closed_strip_takes_no_width() {
        let metrics = unterm_render::quads::CellMetrics {
            width: 10.0,
            height: 20.0,
            baseline: 16.0,
        };
        assert_eq!(width(false, metrics), 0.0);
        assert!(width(true, metrics) > 0.0);
    }

    /// One project needs no headers: a header above every tab says nothing.
    #[test]
    fn a_single_project_gets_no_group_headers() {
        let rows = rows(&[
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
        let rows = rows(&[
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
        let rows = rows(&[
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
        let rows = rows(&[
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
        let rows = rows(&[
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
        let rows = rows(&[tab(0, "pwsh", None), tab(1, "pwsh", Some("/work/app"))]);
        let tabs = rows.iter().filter(|row| matches!(row, Row::Tab { .. })).count();
        assert_eq!(tabs, 2, "{:?}", labels(&rows));
    }

    #[test]
    fn a_group_counts_its_own_tabs() {
        let rows = rows(&[
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
