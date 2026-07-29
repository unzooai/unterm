//! What git says about the directory a pane is in.
//!
//! Read-only, deliberately. A terminal that stages and commits for you is a
//! git client with a shell attached, and the shell is right there -- what is
//! missing when you are looking at a terminal is not the ability to run `git
//! add`, it is knowing whether you are on the branch you think you are and
//! what is dirty before you run something.
//!
//! Parsing is separated from running so it can be checked without a
//! repository. Porcelain v1 is the format to parse: it is the one git
//! promises not to change, and every line is fixed-width where it matters.

/// What a repository looks like right now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub branch: String,
    /// Commits this branch is ahead of its upstream, and behind it.
    pub ahead: usize,
    pub behind: usize,
    pub entries: Vec<Entry>,
}

/// One changed path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The two-letter code git prints: index state, then worktree state.
    pub code: String,
    pub path: String,
}

impl Status {
    /// A one-line summary, for a heading.
    pub fn summary(&self) -> String {
        let mut parts = vec![if self.branch.is_empty() {
            "(detached)".to_string()
        } else {
            self.branch.clone()
        }];
        if self.ahead > 0 {
            parts.push(format!("↑{}", self.ahead));
        }
        if self.behind > 0 {
            parts.push(format!("↓{}", self.behind));
        }
        parts.push(match self.entries.len() {
            0 => "clean".to_string(),
            1 => "1 change".to_string(),
            count => format!("{count} changes"),
        });
        parts.join("  ")
    }
}

/// Parse `git status --porcelain=v1 --branch`.
///
/// Unrecognised lines are skipped rather than guessed at: a status panel that
/// invents an entry is worse than one that shows fewer.
pub fn parse(output: &str) -> Status {
    let mut status = Status::default();
    for line in output.lines() {
        if let Some(header) = line.strip_prefix("## ") {
            parse_branch(header, &mut status);
        } else if line.len() > 3 {
            // Two code characters, a space, then the path. Renames read
            // `R  old -> new`; the new name is the one that exists now.
            let (code, rest) = line.split_at(2);
            let path = rest.trim_start();
            let path = path.rsplit(" -> ").next().unwrap_or(path);
            status.entries.push(Entry {
                code: code.trim().to_string(),
                path: path.trim_matches('"').to_string(),
            });
        }
    }
    status
}

/// The `## branch...upstream [ahead 1, behind 2]` header.
fn parse_branch(header: &str, status: &mut Status) {
    let (names, tracking) = match header.split_once(" [") {
        Some((names, tracking)) => (names, tracking.trim_end_matches(']')),
        None => (header, ""),
    };
    // `main...origin/main` -- the local name is the part before the dots.
    status.branch = names.split("...").next().unwrap_or(names).trim().to_string();
    if status.branch == "HEAD (no branch)" {
        status.branch.clear();
    }

    for part in tracking.split(", ") {
        if let Some(count) = part.strip_prefix("ahead ") {
            status.ahead = count.trim().parse().unwrap_or(0);
        } else if let Some(count) = part.strip_prefix("behind ") {
            status.behind = count.trim().parse().unwrap_or(0);
        }
    }
}

/// What the panel has to show.
///
/// Three answers, not two. "No git installed" and "not a repository" look the
/// same from a failed command and mean completely different things: one is
/// "this folder is not tracked", the other is "this terminal cannot tell you".
/// Reporting the first when the second is true sends someone looking for a
/// `.git` directory that is right there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Panel {
    Status(Status),
    NotARepository,
    NoGit,
}

impl Panel {
    /// The heading this answer deserves.
    pub fn heading(&self) -> String {
        match self {
            Panel::Status(status) => format!("Git  {}", status.summary()),
            Panel::NotARepository => "Git  (not a repository)".to_string(),
            Panel::NoGit => "Git  (git is not on PATH)".to_string(),
        }
    }

    pub fn entries(&self) -> &[Entry] {
        match self {
            Panel::Status(status) => &status.entries,
            _ => &[],
        }
    }
}

/// Ask git about `directory`.
pub fn read(directory: &std::path::Path) -> Panel {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain=v1", "--branch"])
        .current_dir(directory)
        .output();
    match output {
        // Git ran and said no: this is not a repository.
        Ok(output) if !output.status.success() => Panel::NotARepository,
        Ok(output) => Panel::Status(parse(&String::from_utf8_lossy(&output.stdout))),
        // Git did not run at all.
        Err(_) => Panel::NoGit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "No git" and "not a repository" are different answers. Showing the
    /// second when the first is true sends someone looking for a `.git`
    /// directory that is sitting right there.
    #[test]
    fn a_missing_git_does_not_claim_the_folder_is_untracked() {
        assert!(Panel::NoGit.heading().contains("PATH"));
        assert!(Panel::NotARepository.heading().contains("not a repository"));
        assert_ne!(Panel::NoGit.heading(), Panel::NotARepository.heading());
    }

    #[test]
    fn a_status_heading_names_the_branch() {
        let panel = Panel::Status(parse(
            "## main...origin/main [ahead 1]
 M a.rs
",
        ));
        let heading = panel.heading();
        assert!(heading.contains("main"), "{heading}");
        assert!(heading.contains("1 change"), "{heading}");
        assert_eq!(panel.entries().len(), 1);
    }

    #[test]
    fn the_answers_that_are_not_a_status_have_no_entries() {
        assert!(Panel::NoGit.entries().is_empty());
        assert!(Panel::NotARepository.entries().is_empty());
    }

    #[test]
    fn a_clean_repository_reads_as_clean() {
        let status = parse("## main...origin/main\n");
        assert_eq!(status.branch, "main");
        assert_eq!(status.entries.len(), 0);
        assert_eq!(status.summary(), "main  clean");
    }

    #[test]
    fn ahead_and_behind_are_both_read() {
        let status = parse("## main...origin/main [ahead 2, behind 3]\n");
        assert_eq!((status.ahead, status.behind), (2, 3));
        assert!(status.summary().contains("↑2"), "{}", status.summary());
        assert!(status.summary().contains("↓3"), "{}", status.summary());
    }

    #[test]
    fn ahead_alone_does_not_invent_a_behind() {
        let status = parse("## main...origin/main [ahead 1]\n");
        assert_eq!((status.ahead, status.behind), (1, 0));
        assert!(!status.summary().contains('↓'), "{}", status.summary());
    }

    /// A branch with no upstream has no counts, and must not be mistaken for
    /// one that is level with its upstream.
    #[test]
    fn a_branch_with_no_upstream_still_names_itself() {
        let status = parse("## experiment\n");
        assert_eq!(status.branch, "experiment");
        assert_eq!((status.ahead, status.behind), (0, 0));
    }

    /// Detached HEAD is a state people get into by accident, and the panel
    /// exists to tell them.
    #[test]
    fn a_detached_head_says_so_rather_than_naming_a_branch() {
        let status = parse("## HEAD (no branch)\n");
        assert!(status.branch.is_empty());
        assert!(status.summary().starts_with("(detached)"), "{}", status.summary());
    }

    #[test]
    fn changed_paths_keep_their_codes() {
        let status = parse("## main\n M src/lib.rs\n?? notes.txt\nA  added.rs\n");
        assert_eq!(status.entries.len(), 3);
        assert_eq!(status.entries[0].code, "M");
        assert_eq!(status.entries[0].path, "src/lib.rs");
        assert_eq!(status.entries[1].code, "??");
        assert_eq!(status.entries[2].code, "A");
    }

    /// A rename prints both names. The one that exists now is the one to
    /// show: pointing at a path that is gone is a panel telling a lie.
    #[test]
    fn a_rename_shows_where_the_file_is_now() {
        let status = parse("## main\nR  old/name.rs -> new/name.rs\n");
        assert_eq!(status.entries[0].path, "new/name.rs");
    }

    /// Paths with spaces come back quoted; the quotes are git's, not the
    /// file's.
    #[test]
    fn a_quoted_path_loses_its_quotes() {
        let status = parse("## main\n M \"a file.txt\"\n");
        assert_eq!(status.entries[0].path, "a file.txt");
    }

    #[test]
    fn one_change_is_singular_and_two_are_not() {
        let one = parse("## main\n M a.rs\n");
        assert!(one.summary().ends_with("1 change"), "{}", one.summary());
        let two = parse("## main\n M a.rs\n M b.rs\n");
        assert!(two.summary().ends_with("2 changes"), "{}", two.summary());
    }

    /// Nothing at all is a repository with no commits yet, not a crash.
    #[test]
    fn empty_output_is_a_status_rather_than_a_panic() {
        let status = parse("");
        assert_eq!(status, Status::default());
        assert_eq!(status.summary(), "(detached)  clean");
    }

    /// Junk in the middle is skipped, not guessed at: a panel that invents an
    /// entry is worse than one that shows fewer.
    #[test]
    fn a_line_too_short_to_be_an_entry_is_ignored() {
        let status = parse("## main\nx\n M real.rs\n");
        assert_eq!(status.entries.len(), 1);
        assert_eq!(status.entries[0].path, "real.rs");
    }
}
