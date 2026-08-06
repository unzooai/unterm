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

/// Ask the operating system for a directory without holding the UI thread.
///
/// Windows' `FolderBrowserDialog` is hosted by a short-lived STA PowerShell
/// process.  Keeping that work here makes the directory palette and the
/// system picker share one validated `PathBuf` result instead of each action
/// growing its own quoting and cancellation rules.
#[cfg(windows)]
pub fn pick_directory(
    start_at: Option<&std::path::Path>,
    title: &str,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = $env:UNTERM_FOLDER_PICKER_TITLE
$dialog.ShowNewFolderButton = $true
if ($env:UNTERM_FOLDER_PICKER_START -and (Test-Path -LiteralPath $env:UNTERM_FOLDER_PICKER_START -PathType Container)) {
  $dialog.SelectedPath = $env:UNTERM_FOLDER_PICKER_START
}
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  [Console]::Out.Write($dialog.SelectedPath)
}
"#;

    let mut command = std::process::Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-STA",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        SCRIPT,
    ]);
    command.env("UNTERM_FOLDER_PICKER_TITLE", title);
    if let Some(start) = start_at.filter(|path| path.is_dir()) {
        command.env("UNTERM_FOLDER_PICKER_START", start);
    }
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000);

    let output = command.output()?;
    if !output.status.success() {
        anyhow::bail!("system folder picker returned {}", output.status);
    }
    let selected = String::from_utf8(output.stdout)?.trim().to_string();
    if selected.is_empty() {
        return Ok(None);
    }
    let path = std::path::PathBuf::from(selected);
    if !path.is_dir() {
        anyhow::bail!("selected path is not a directory: {}", path.display());
    }
    Ok(Some(path))
}

/// Ask macOS for a directory through `osascript`'s `choose folder`.
///
/// The dialog lives in a short-lived helper process for the same reason the
/// Windows one lives in PowerShell: the UI thread never waits on a modal it
/// does not own.  The title and starting directory travel as argv into the
/// script's `run` handler rather than being spliced into its source, so a
/// path with a quote in it is a path, not AppleScript.
#[cfg(target_os = "macos")]
pub fn pick_directory(
    start_at: Option<&std::path::Path>,
    title: &str,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    // `choose folder` reports a dismissed dialog as error -128, and only that
    // error becomes the empty answer that means "cancelled".  Anything else
    // -- a scripting restriction, a vanished start directory -- re-raises,
    // fails the process, and reaches the caller as the failure it is.
    const SCRIPT: &str = r#"on run argv
  try
    if (count of argv) is 2 then
      try
        return POSIX path of (choose folder with prompt (item 1 of argv) default location (POSIX file (item 2 of argv)))
      on error errText number errNum
        if errNum is -128 then return ""
        -- A start directory the dialog cannot open (unmounted volume, a
        -- cloud placeholder, a path that vanished since the check) is the
        -- start directory's problem, not the dialog's: ask again without it
        -- rather than failing the whole picker over a default.
        return POSIX path of (choose folder with prompt (item 1 of argv))
      end try
    else
      return POSIX path of (choose folder with prompt (item 1 of argv))
    end if
  on error errText number errNum
    if errNum is -128 then return ""
    error errText number errNum
  end try
end run"#;

    let mut command = std::process::Command::new("/usr/bin/osascript");
    command.arg("-e").arg(SCRIPT).arg(title);
    if let Some(start) = start_at.filter(|path| path.is_dir()) {
        command.arg(start);
    }

    let output = command.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "system folder picker returned {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    selected_directory(&output.stdout)
}

/// Ask a Linux desktop for a directory with whichever dialog tool it has.
///
/// There is no one system picker across desktops: GNOME ships zenity, KDE
/// ships kdialog, and yad turns up where the other two do not.  The
/// candidates run in that order and the first one that exists answers.  The
/// title and start directory are argv to the tool, never a shell string.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn pick_directory(
    start_at: Option<&std::path::Path>,
    title: &str,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    let start = start_at
        .filter(|path| path.is_dir())
        .map(|path| path.display().to_string());

    // zenity opens inside `--filename=<dir>/` (the trailing slash means "in
    // it", not "at it"); kdialog takes the start directory as a positional
    // argument; yad mirrors zenity's flags.
    let zenity_start = start.as_ref().map(|dir| format!("--filename={dir}/"));
    let title_flag = format!("--title={title}");

    let mut zenity_args = vec![
        "--file-selection".to_string(),
        "--directory".to_string(),
        title_flag.clone(),
    ];
    if let Some(flag) = &zenity_start {
        zenity_args.push(flag.clone());
    }
    let kdialog_args = vec![
        "--title".to_string(),
        title.to_string(),
        "--getexistingdirectory".to_string(),
        start.clone().unwrap_or_else(|| ".".to_string()),
    ];
    let mut yad_args = vec!["--file".to_string(), "--directory".to_string(), title_flag];
    if let Some(flag) = &zenity_start {
        yad_args.push(flag.clone());
    }

    for (bin, args) in [
        ("zenity", zenity_args),
        ("kdialog", kdialog_args),
        ("yad", yad_args),
    ] {
        let output = match std::process::Command::new(bin).args(&args).output() {
            Ok(output) => output,
            // Not installed: the next candidate's turn.
            Err(_) => continue,
        };
        match linux_picker_verdict(output.status.code()) {
            LinuxVerdict::Answered => return selected_directory(&output.stdout),
            LinuxVerdict::Cancelled => return Ok(None),
            LinuxVerdict::TryNext => continue,
        }
    }
    anyhow::bail!("no usable folder picker; install one of zenity, kdialog or yad")
}

/// What one Linux picker's exit code decided.
#[cfg(all(unix, not(target_os = "macos")))]
#[derive(Debug, PartialEq, Eq)]
enum LinuxVerdict {
    /// The dialog closed with OK; stdout holds the answer.
    Answered,
    /// The dialog was dismissed.
    Cancelled,
    /// The tool failed before a dialog could mean anything.
    TryNext,
}

/// zenity, kdialog and yad all exit 1 for a dismissed dialog, and once a
/// dialog has been on screen the question is answered: falling through to the
/// next candidate would pop a second picker at someone who just closed one.
/// Any other failure is the tool's own, so the next candidate gets its turn.
#[cfg(all(unix, not(target_os = "macos")))]
fn linux_picker_verdict(code: Option<i32>) -> LinuxVerdict {
    match code {
        Some(0) => LinuxVerdict::Answered,
        Some(1) => LinuxVerdict::Cancelled,
        _ => LinuxVerdict::TryNext,
    }
}

/// One validated result out of whatever a picker process printed.
///
/// Every picker speaks the same protocol on the way out: a path on stdout is
/// a choice, nothing is a dialog closed without one.  Empty is `None` rather
/// than an error because cancelling is not a failure, and the path is checked
/// here so every platform shares one rule for what counts as a directory.
#[cfg(not(windows))]
fn selected_directory(stdout: &[u8]) -> anyhow::Result<Option<std::path::PathBuf>> {
    let selected = String::from_utf8(stdout.to_vec())?.trim().to_string();
    if selected.is_empty() {
        return Ok(None);
    }
    let path = std::path::PathBuf::from(selected);
    if !path.is_dir() {
        anyhow::bail!("selected path is not a directory: {}", path.display());
    }
    Ok(Some(path))
}

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
        assert!(
            matches!(rows[1].command, Command::NewTabIn { .. }),
            "{:?}",
            rows[1]
        );
        let alpha = rows.iter().find(|row| row.label == "Alpha").unwrap();
        assert!(
            matches!(
                alpha.command,
                Command::Browse {
                    then: BrowseThen::NewTab,
                    ..
                }
            ),
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
        assert_eq!(
            labels(&rows),
            vec!["..".to_string(), "Use this folder".to_string()]
        );
    }

    /// A picker that printed nothing was cancelled, not broken: the empty
    /// answer is how both the AppleScript and the dialog tools say "the
    /// dialog closed without a choice", and it must stay quiet.
    #[cfg(not(windows))]
    #[test]
    fn a_picker_that_printed_nothing_was_cancelled() {
        assert!(matches!(selected_directory(b""), Ok(None)));
        assert!(matches!(selected_directory(b"  \n"), Ok(None)));
    }

    /// The pickers print the path with a trailing newline, and osascript adds
    /// a trailing slash of its own; neither is part of the folder's name.
    #[cfg(not(windows))]
    #[test]
    fn a_pickers_decorations_are_not_part_of_the_path() {
        const NAME: &str = "a_pickers_decorations_are_not_part_of_the_path";
        let root = scratch(NAME);
        let newline = format!("{}\n", root.display());
        assert_eq!(
            selected_directory(newline.as_bytes()).unwrap(),
            Some(root.clone())
        );
        let slash = format!("{}/\n", root.display());
        assert_eq!(selected_directory(slash.as_bytes()).unwrap(), Some(root));
    }

    /// A path that is not a directory is a failure, not a choice: the two
    /// quick actions behind the picker both `cd`, and handing them a file
    /// would fail later, further from the cause.
    #[cfg(not(windows))]
    #[test]
    fn a_picked_file_is_an_error_not_a_choice() {
        const NAME: &str = "a_picked_file_is_an_error_not_a_choice";
        let file = scratch(NAME).join("notes.txt");
        assert!(selected_directory(file.display().to_string().as_bytes()).is_err());
    }

    /// Bytes that are not text cannot name a folder anybody chose.
    #[cfg(not(windows))]
    #[test]
    fn a_picker_that_printed_garbage_is_an_error() {
        assert!(selected_directory(&[0xff, 0xfe]).is_err());
    }

    /// Exit 1 is the dismissed dialog, and only the dismissed dialog: probing
    /// the next tool after a cancel would pop a second picker at someone who
    /// just closed one.
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_dismissed_dialog_stops_the_probing() {
        assert_eq!(linux_picker_verdict(Some(0)), LinuxVerdict::Answered);
        assert_eq!(linux_picker_verdict(Some(1)), LinuxVerdict::Cancelled);
        assert_eq!(linux_picker_verdict(Some(255)), LinuxVerdict::TryNext);
        assert_eq!(linux_picker_verdict(None), LinuxVerdict::TryNext);
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
