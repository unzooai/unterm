//! Jump to a directory by typing part of its name.
//!
//! Ported from the previous front end's `dir_jump.rs`. The picker in
//! `directory` walks one level at a time, which is right for "choose a
//! folder" and wrong for "I know roughly where it is": this one matches at
//! any depth below the current directory, lists recent projects and the
//! machine's drives, and takes a path directly when one is typed.
//!
//! The parts that came across verbatim are the ones with reasons that are not
//! visible from the code: the scan's limits, the list of directories it
//! refuses to walk into, and the Windows rules for a typed path -- where a
//! bare `D:` means "wherever the process happens to be on drive D", which is
//! never what somebody typing into a picker means.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// How deep below the current directory the scan goes.
const MAX_DEPTH: usize = 6;
/// And how much of it, so a scan inside a home directory cannot run away.
const MAX_ENTRIES: usize = 3000;

/// Directories that are pure noise in a jump-to-directory flow: huge,
/// machine-managed, and never somewhere anybody navigates to on purpose.
const SKIP: &[&str] = &[
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

/// Which part of the list a row belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Locations,
    Recent,
    Subdirectories,
}

impl Section {
    /// The heading, from the catalogue the product already ships.
    pub fn heading(self) -> String {
        match self {
            Section::Locations => unterm_services::i18n::t("dirjump.locations"),
            Section::Recent => unterm_services::i18n::t("dirjump.recent"),
            Section::Subdirectories => unterm_services::i18n::t("dirjump.subdirs"),
        }
    }
}

/// One directory the picker is offering.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    /// What the row says: a name, or a path relative to where we started.
    pub label: String,
    pub section: Section,
}

/// The machine's other roots: drives, mounted volumes, removable media.
///
/// Windows needs these more than the others do. Drive letters are separate
/// roots -- from `C:\` there is no ascending to `D:\` -- so without this a
/// second drive is simply unreachable from the picker.
pub fn locations() -> Vec<Entry> {
    let mut found: Vec<PathBuf> = Vec::new();

    #[cfg(windows)]
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if Path::new(&drive).is_dir() {
            found.push(PathBuf::from(drive));
        }
    }
    #[cfg(target_os = "macos")]
    if let Ok(reading) = std::fs::read_dir("/Volumes") {
        for entry in reading.flatten() {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            // The startup disk appears here as a symlink to `/`, which is
            // already reachable and would list everything twice.
            if std::fs::read_link(&path).map(|target| target == Path::new("/")).unwrap_or(false) {
                continue;
            }
            if path.is_dir() {
                found.push(path);
            }
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    for base in [
        "/mnt".to_string(),
        format!("/media/{}", std::env::var("USER").unwrap_or_default()),
    ] {
        if let Ok(reading) = std::fs::read_dir(&base) {
            for entry in reading.flatten() {
                let path = entry.path();
                if path.is_dir() && !entry.file_name().to_string_lossy().starts_with('.') {
                    found.push(path);
                }
            }
        }
    }

    found.sort();
    found
        .into_iter()
        .map(|path| Entry {
            label: path.display().to_string(),
            path,
            section: Section::Locations,
        })
        .collect()
}

/// Whether what has been typed is a path rather than a name to match.
///
/// `/` and `~` are paths anywhere. On Windows so is `D:` -- and so is a UNC
/// share, which is the one people forget.
pub fn is_path_query(query: &str) -> bool {
    if query.starts_with('/') || query.starts_with('~') {
        return true;
    }
    let bytes = query.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && (bytes[0] as char).is_ascii_alphabetic() {
        return true;
    }
    query.starts_with("\\\\")
}

/// Split a typed path into the directory to list and the fragment to filter.
///
/// `windows` rather than `cfg!` so the Windows rules can be tested anywhere.
/// Two of them are not obvious: a bare `D:` is promoted to `D:/`, because in
/// Win32 `D:` means "wherever this process last was on drive D" and a picker
/// always means the root; and a parent that collapses back to `D:` gets its
/// slash returned, or listing it reads the same wrong place.
pub fn split_path_query(expanded: &str, windows: bool) -> Option<(String, String)> {
    let mut expanded = expanded.to_string();
    if windows {
        expanded = expanded.replace('\\', "/");
        let bytes = expanded.as_bytes();
        if bytes.len() == 2 && bytes[1] == b':' {
            expanded.push('/');
        }
    }
    let (parent, fragment) = match expanded.rfind('/') {
        Some(0) => ("/".to_string(), expanded[1..].to_string()),
        Some(index) => (expanded[..index].to_string(), expanded[index + 1..].to_string()),
        None => return None,
    };
    let parent = if windows && parent.len() == 2 && parent.ends_with(':') {
        format!("{parent}/")
    } else {
        parent
    };
    Some((parent, fragment))
}

/// Expand a leading `~`.
pub fn expand_tilde(input: &str) -> String {
    if let Some(rest) = input.strip_prefix('~') {
        if let Some(home) = dirs_next::home_dir() {
            return format!("{}{}", home.display(), rest);
        }
    }
    input.to_string()
}

/// Every directory below `base`, to a bounded depth.
///
/// Breadth-first, so the near ones arrive first and the cap cuts the far ones
/// rather than a random branch. Depth-0 children are left out because they are
/// already the subdirectory list.
pub fn deep_scan(base: &Path) -> Vec<Entry> {
    let mut found = Vec::new();
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((base.to_path_buf(), 0));

    while let Some((directory, depth)) = queue.pop_front() {
        if depth >= MAX_DEPTH || found.len() >= MAX_ENTRIES {
            break;
        }
        let Ok(reading) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in reading.flatten() {
            if found.len() >= MAX_ENTRIES {
                break;
            }
            if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || SKIP.contains(&name.as_str()) {
                continue;
            }
            let path = entry.path();
            if depth > 0 {
                if let Ok(relative) = path.strip_prefix(base) {
                    // Forward slashes whatever the platform: the matching and
                    // the indent both count them.
                    let label = relative
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    found.push(Entry {
                        path: path.clone(),
                        label,
                        section: Section::Subdirectories,
                    });
                }
            }
            queue.push_back((path, depth + 1));
        }
    }
    found
}

/// The directories directly under `base`.
pub fn subdirectories(base: &Path) -> Vec<Entry> {
    let Ok(reading) = std::fs::read_dir(base) else {
        return Vec::new();
    };
    let mut found: Vec<Entry> = reading
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .map(|entry| Entry {
            label: entry.file_name().to_string_lossy().to_string(),
            path: entry.path(),
            section: Section::Subdirectories,
        })
        .collect();
    found.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    found
}

/// The projects opened before, most recent first.
///
/// From `~/.unterm/projects.json`, which is where the product has always kept
/// them -- so a list built up by the previous version is still there.
pub fn recents() -> Vec<Entry> {
    let Some(path) = dirs_next::home_dir().map(|home| home.join(".unterm").join("projects.json"))
    else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let list = value
        .get("projects")
        .and_then(|projects| projects.as_array())
        .cloned()
        .or_else(|| value.as_array().cloned())
        .unwrap_or_default();

    list.iter()
        .filter_map(|item| {
            let path = item
                .as_str()
                .map(str::to_string)
                .or_else(|| item.get("path")?.as_str().map(str::to_string))?;
            let path = PathBuf::from(path);
            path.is_dir().then(|| Entry {
                label: display_name(&path),
                path,
                section: Section::Recent,
            })
        })
        .collect()
}

/// What a path is called, for a row that has room for a name and not a path.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// Whether an entry matches what has been typed.
///
/// Every character of the query in order, anywhere in the label -- the same
/// loose matching a command palette uses, which is what makes "usr/lib" find
/// `usr/local/lib` without anyone spelling it out.
pub fn matches(label: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let label = label.to_lowercase();
    let mut haystack = label.chars();
    query
        .to_lowercase()
        .chars()
        .all(|wanted| haystack.any(|found| found == wanted))
}

/// Filter and order the entries for a query.
///
/// Ordered by section, and within a section by how early the match starts: a
/// directory whose name begins with what was typed is what was meant far more
/// often than one that merely contains it.
pub fn filter(entries: &[Entry], query: &str) -> Vec<Entry> {
    let mut matched: Vec<Entry> = entries
        .iter()
        .filter(|entry| matches(&entry.label, query))
        .cloned()
        .collect();
    if query.is_empty() {
        return matched;
    }
    let lowered = query.to_lowercase();
    matched.sort_by_key(|entry| {
        let label = entry.label.to_lowercase();
        let position = label.find(&lowered).unwrap_or(usize::MAX - 1);
        (section_order(entry.section), position, entry.label.len())
    });
    matched
}

fn section_order(section: Section) -> usize {
    match section {
        Section::Subdirectories => 0,
        Section::Recent => 1,
        Section::Locations => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows drives are separate roots: from `C:\` there is no ascending to
    /// `D:\`, so without these a second drive cannot be reached at all.
    #[test]
    fn the_machines_roots_are_offered() {
        let found = locations();
        if cfg!(windows) {
            assert!(!found.is_empty(), "at least the system drive");
            assert!(found.iter().all(|entry| entry.section == Section::Locations));
            assert!(
                found.iter().any(|entry| entry.path.is_dir()),
                "and they exist"
            );
        }
    }

    #[test]
    fn a_leading_slash_or_tilde_is_a_path() {
        assert!(is_path_query("/usr/local"));
        assert!(is_path_query("~/code"));
        assert!(!is_path_query("code"));
        assert!(!is_path_query(""));
    }

    /// A drive letter is a path on Windows, and so is a UNC share -- which is
    /// the one that gets forgotten.
    #[test]
    fn a_drive_or_a_share_is_a_path_too() {
        assert!(is_path_query("D:"));
        assert!(is_path_query("d:/code"));
        assert!(is_path_query("\\\\server\\share"));
        assert!(!is_path_query("D"), "a bare letter is a name");
    }

    #[test]
    fn a_typed_path_splits_into_a_parent_and_a_fragment() {
        assert_eq!(
            split_path_query("/usr/lo", false),
            Some(("/usr".to_string(), "lo".to_string()))
        );
        assert_eq!(
            split_path_query("/us", false),
            Some(("/".to_string(), "us".to_string())),
            "a fragment at the root still lists the root"
        );
        assert_eq!(split_path_query("usr", false), None, "not a path at all");
    }

    /// `D:` alone means "wherever this process last was on drive D" in Win32,
    /// which is never what somebody typing into a picker means.
    #[test]
    fn a_bare_drive_is_promoted_to_its_root() {
        assert_eq!(
            split_path_query("D:", true),
            Some(("D:/".to_string(), "".to_string()))
        );
    }

    /// And a parent that collapses back to `D:` gets its slash returned, or
    /// listing it reads the same wrong place.
    #[test]
    fn a_parent_that_is_a_drive_keeps_its_slash() {
        assert_eq!(
            split_path_query("D:/co", true),
            Some(("D:/".to_string(), "co".to_string()))
        );
    }

    /// Backslashes are what Windows users type and what completion writes
    /// back, so both have to split the same way.
    #[test]
    fn backslashes_split_like_slashes() {
        assert_eq!(
            split_path_query("D:\\code\\un", true),
            Some(("D:/code".to_string(), "un".to_string()))
        );
    }

    /// Loose matching: every character in order, anywhere. This is what makes
    /// a few letters find a directory several levels down.
    #[test]
    fn matching_finds_characters_in_order_anywhere() {
        assert!(matches("usr/local/lib", "usrlib"));
        assert!(matches("usr/local/lib", "ulib"));
        assert!(!matches("usr/local/lib", "libusr"), "order still counts");
        assert!(matches("anything", ""), "an empty query matches all");
    }

    #[test]
    fn matching_ignores_case() {
        assert!(matches("Documents", "doc"));
        assert!(matches("documents", "DOC"));
    }

    /// A name that begins with what was typed is what was meant, far more
    /// often than one that merely contains it.
    #[test]
    fn an_earlier_match_is_offered_first() {
        let entries = vec![
            Entry {
                path: PathBuf::from("/a/my-code"),
                label: "my-code".to_string(),
                section: Section::Subdirectories,
            },
            Entry {
                path: PathBuf::from("/a/code"),
                label: "code".to_string(),
                section: Section::Subdirectories,
            },
        ];
        let filtered = filter(&entries, "code");
        assert_eq!(filtered[0].label, "code");
    }

    /// Subdirectories of where you are come before anywhere else: the picker
    /// was opened from somewhere, and that somewhere is the likeliest answer.
    #[test]
    fn the_current_directorys_children_come_first() {
        let entries = vec![
            Entry {
                path: PathBuf::from("/recent/app"),
                label: "app".to_string(),
                section: Section::Recent,
            },
            Entry {
                path: PathBuf::from("./app"),
                label: "app".to_string(),
                section: Section::Subdirectories,
            },
        ];
        let filtered = filter(&entries, "app");
        assert_eq!(filtered[0].section, Section::Subdirectories);
    }

    /// The noise list is what keeps a scan of a home directory from being
    /// thirty thousand rows of `node_modules`.
    #[test]
    fn the_directories_nobody_navigates_to_are_skipped() {
        for noise in ["node_modules", "target", ".git", "__pycache__"] {
            assert!(SKIP.contains(&noise), "{noise} should be skipped");
        }
    }

    /// A scan of somewhere enormous has to stop. Both limits exist because a
    /// picker that hangs on a home directory is a picker nobody opens twice.
    #[test]
    fn the_scan_is_bounded_in_both_directions() {
        assert!(MAX_DEPTH > 0 && MAX_DEPTH <= 8);
        assert!(MAX_ENTRIES >= 500 && MAX_ENTRIES <= 10_000);
    }

    #[test]
    fn a_directory_that_cannot_be_read_lists_nothing_rather_than_failing() {
        let missing = std::env::temp_dir().join("unterm-dir-jump-missing");
        let _ = std::fs::remove_dir_all(&missing);
        assert!(subdirectories(&missing).is_empty());
        assert!(deep_scan(&missing).is_empty());
    }

    /// Every heading comes from the catalogue, so the picker is translated
    /// like everything else.
    #[test]
    fn the_headings_are_translated() {
        for section in [Section::Locations, Section::Recent, Section::Subdirectories] {
            let heading = section.heading();
            assert!(!heading.is_empty());
            assert!(!heading.starts_with("dirjump."), "{heading} is a raw key");
        }
    }
}
