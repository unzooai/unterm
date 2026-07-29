//! Picking a folder without a folder dialog.
//!
//! The two quick actions that need one -- change the working directory, open a
//! folder in a new tab -- would otherwise want a native picker, and there is
//! no such thing that is the same on macOS, Linux and Windows without dragging
//! in a dependency per platform. Cross-platform parity here is a correctness
//! property, not a nice-to-have, so the picker is the palette: the folders
//! under a path, filtered by typing, with the parent first.
//!
//! It is also faster. A modal dialog is a mouse trip; this is a few letters
//! and Enter, and it stays open as you descend.

use crate::palette::{BrowseThen, Command, Entry};

/// The rows for picking inside `path`: the parent, then the folders under it.
///
/// Unreadable directories give an empty list rather than an error: the palette
/// showing nothing says "there is nothing here" clearly enough, and a terminal
/// that pops an error because a folder is not readable is a terminal that
/// interrupts you over something you did not ask about.
pub fn entries(path: &std::path::Path, then: BrowseThen) -> Vec<Entry> {
    let mut rows = Vec::new();

    if let Some(parent) = path.parent() {
        rows.push(Entry {
            label: "..".to_string(),
            hint: parent.display().to_string(),
            command: Command::Browse {
                path: parent.display().to_string(),
                then,
            },
        });
    }

    // This directory itself, so descending is not the only way out: whoever
    // opened the picker meant to end up somewhere, and they may already be
    // there.
    rows.push(Entry {
        label: "Use this folder".to_string(),
        hint: path.display().to_string(),
        command: match then {
            BrowseThen::ChangeDirectory => Command::ChangeDirectory {
                path: path.display().to_string(),
            },
            BrowseThen::NewTab => Command::NewTabIn {
                path: path.display().to_string(),
            },
        },
    });

    let Ok(reading) = std::fs::read_dir(path) else {
        return rows;
    };
    let mut folders: Vec<(String, String)> = reading
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| !is_hidden(&entry.file_name().to_string_lossy()))
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().to_string(),
                entry.path().display().to_string(),
            )
        })
        .collect();
    // Alphabetical, because any other order means hunting for a name you can
    // already see in the shell behind the palette.
    folders.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    rows.extend(folders.into_iter().map(|(name, full)| Entry {
        label: name,
        hint: full.clone(),
        command: Command::Browse { path: full, then },
    }));
    rows
}

/// Dot-directories are noise here: `.git` and `.cache` are not where anybody
/// means to `cd`, and they crowd out the folders that are.
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One directory per test. Sharing one and clearing it at the start
    /// means whichever test runs second finds the fixture the first one was
    /// still using -- which is how these first failed, differently each run.
    fn scratch(test: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("unterm-directory-{test}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("beta")).unwrap();
        std::fs::create_dir_all(root.join("Alpha")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("notes.txt"), b"not a folder").unwrap();
        root
    }

    fn labels(rows: &[Entry]) -> Vec<String> {
        rows.iter().map(|row| row.label.clone()).collect()
    }

    #[test]
    fn the_folders_are_listed_and_the_files_are_not() {
        const NAME: &str = "the_folders_are_listed_and_the_files_are_not";
        let rows = entries(&scratch(NAME), BrowseThen::ChangeDirectory);
        let labels = labels(&rows);
        assert!(labels.contains(&"Alpha".to_string()), "{labels:?}");
        assert!(labels.contains(&"beta".to_string()), "{labels:?}");
        assert!(!labels.contains(&"notes.txt".to_string()), "{labels:?}");
    }

    /// `.git` and `.cache` are not where anybody means to `cd`, and they push
    /// the folders that are off a short list.
    #[test]
    fn dot_directories_are_left_out() {
        const NAME: &str = "dot_directories_are_left_out";
        let labels = labels(&entries(&scratch(NAME), BrowseThen::ChangeDirectory));
        assert!(!labels.contains(&".git".to_string()), "{labels:?}");
    }

    /// Alphabetical regardless of case, so a capitalised folder does not sort
    /// into a block of its own away from its neighbours.
    #[test]
    fn folders_are_alphabetical_ignoring_case() {
        const NAME: &str = "folders_are_alphabetical_ignoring_case";
        let rows = entries(&scratch(NAME), BrowseThen::ChangeDirectory);
        let folders: Vec<String> = labels(&rows)
            .into_iter()
            .filter(|label| label != ".." && label != "Use this folder")
            .collect();
        assert_eq!(folders, vec!["Alpha".to_string(), "beta".to_string()]);
    }

    /// The way out is first, and the way to stop is second: descending is the
    /// common case but it must not be the only one.
    #[test]
    fn the_parent_and_this_folder_come_first() {
        const NAME: &str = "the_parent_and_this_folder_come_first";
        let rows = entries(&scratch(NAME), BrowseThen::ChangeDirectory);
        assert_eq!(rows[0].label, "..");
        assert_eq!(rows[1].label, "Use this folder");
        assert!(matches!(rows[1].command, Command::ChangeDirectory { .. }));
    }

    /// A folder row descends rather than choosing, so picking one three deep
    /// is three keystrokes instead of three trips through the menu.
    #[test]
    fn a_folder_row_descends_into_it() {
        const NAME: &str = "a_folder_row_descends_into_it";
        let rows = entries(&scratch(NAME), BrowseThen::ChangeDirectory);
        let alpha = rows.iter().find(|row| row.label == "Alpha").unwrap();
        match &alpha.command {
            Command::Browse { path, .. } => assert!(path.ends_with("Alpha"), "{path}"),
            other => panic!("expected to descend, got {other:?}"),
        }
    }

    /// The picker remembers what opened it. Both quick actions browse the
    /// same folders; only the last step differs, and a picker that forgets
    /// which one it was for can only ever do one of them.
    #[test]
    fn a_picker_opened_for_a_new_tab_ends_in_a_new_tab() {
        const NAME: &str = "a_picker_opened_for_a_new_tab_ends_in_a_new_tab";
        let rows = entries(&scratch(NAME), BrowseThen::NewTab);
        assert!(matches!(rows[1].command, Command::NewTabIn { .. }), "{:?}", rows[1]);
        let alpha = rows.iter().find(|row| row.label == "Alpha").unwrap();
        assert!(
            matches!(alpha.command, Command::Browse { then: BrowseThen::NewTab, .. }),
            "descending must carry the intent: {:?}",
            alpha.command
        );
    }

    /// A directory that cannot be read is an empty list, not an error: a
    /// terminal that interrupts you over an unreadable folder is interrupting
    /// you over something you did not ask about.
    #[test]
    fn an_unreadable_directory_still_offers_a_way_out() {
        let missing = std::env::temp_dir().join("unterm-directory-tests-missing");
        let _ = std::fs::remove_dir_all(&missing);
        let rows = entries(&missing, BrowseThen::ChangeDirectory);
        assert_eq!(labels(&rows), vec!["..".to_string(), "Use this folder".to_string()]);
    }

    /// The root of a drive has no parent, and asking for one must not panic.
    #[test]
    fn a_root_has_no_parent_row() {
        let root = if cfg!(windows) {
            std::path::PathBuf::from("C:\\")
        } else {
            std::path::PathBuf::from("/")
        };
        let rows = entries(&root, BrowseThen::ChangeDirectory);
        assert_ne!(rows.first().map(|row| row.label.as_str()), Some(".."));
    }
}
