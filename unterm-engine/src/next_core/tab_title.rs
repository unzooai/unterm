//! Tab titles from a template instead of a callback.
//!
//! Naming a tab was the one thing configs reliably reached for code to do:
//! take the pane's title, fall back to the running program, tidy it up. The
//! rules are always the same handful, so they are settings here and the title
//! is computed without running anything the user wrote.
//!
//! What a title has to get right, and why:
//!
//! - A shell that reports nothing must still name its tab. An empty tab is
//!   worse than a generic one, so there is always a fallback.
//! - `pwsh.exe` should read as `Pwsh`. The extension is noise on the one
//!   platform that has it, and a bare lowercase name looks like a mistake.
//! - The program comes from a path, and only its last component is a name.

/// How a tab's title is built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabTitleRules {
    /// Template, with `{title}` and `{index}` substituted.
    pub format: String,
    /// Used when nothing else yields a name.
    pub fallback: String,
    /// Drop a trailing `.exe`, which is noise in a tab.
    pub strip_extension: bool,
    /// Upper-case the first character.
    pub capitalize: bool,
    /// Titles treated as "the shell said nothing useful".
    pub ignored_titles: Vec<String>,
}

impl Default for TabTitleRules {
    fn default() -> Self {
        Self {
            format: "  {title}  ".to_string(),
            fallback: "Terminal".to_string(),
            strip_extension: true,
            capitalize: true,
            ignored_titles: vec!["default".to_string()],
        }
    }
}

/// What the engine knows about a tab when it needs a title.
#[derive(Clone, Copy, Debug, Default)]
pub struct TabContext<'a> {
    pub pane_title: &'a str,
    /// Path of the foreground program, which may be empty.
    pub process_path: &'a str,
    /// 1-based, as it is shown.
    pub index: usize,
}

/// The placeholders a format string may use.
pub const PLACEHOLDERS: &[&str] = &["title", "index"];

/// Placeholders in `format` that are not recognised.
///
/// A template silently rendering `{tittle}` as literal text is the same
/// failure as a config key that does nothing, so callers reject these.
pub fn unknown_placeholders(format: &str) -> Vec<String> {
    let mut unknown = Vec::new();
    let mut rest = format;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            break;
        };
        let name = &after[..end];
        if !PLACEHOLDERS.contains(&name) && !unknown.iter().any(|seen| seen == name) {
            unknown.push(name.to_string());
        }
        rest = &after[end + 1..];
    }
    unknown
}

pub fn render(rules: &TabTitleRules, context: TabContext) -> String {
    let mut title = context.pane_title.trim().to_string();

    if title.is_empty() || rules.ignored_titles.iter().any(|ignored| *ignored == title) {
        title = program_name(context.process_path);
    }

    if rules.strip_extension {
        title = strip_executable_extension(&title);
    }
    if rules.capitalize {
        title = capitalize_first(&title);
    }
    if title.is_empty() {
        // An unnamed tab is worse than a generically named one.
        title = rules.fallback.clone();
    }

    rules
        .format
        .replace("{title}", &title)
        .replace("{index}", &context.index.to_string())
}

/// The last component of a path, whichever separator it uses.
///
/// The path can come from either platform -- a remote shell reports POSIX
/// paths on Windows -- so both separators are honoured everywhere.
fn program_name(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Drop a trailing `.exe`, case-insensitively: Windows does not care about the
/// case and neither should the tab.
fn strip_executable_extension(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.strip_suffix(".exe") {
        Some(stem) => name[..stem.len()].to_string(),
        None => name.to_string(),
    }
}

fn capitalize_first(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        // Upper-casing can yield more than one character, and languages with
        // no case leave it unchanged, which is the right answer for both.
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(pane_title: &'a str, process_path: &'a str) -> TabContext<'a> {
        TabContext {
            pane_title,
            process_path,
            index: 1,
        }
    }

    #[test]
    fn a_reported_title_is_used_as_is() {
        let rendered = render(
            &TabTitleRules::default(),
            context("Building project", "/usr/bin/cargo"),
        );

        assert_eq!(rendered, "  Building project  ");
    }

    #[test]
    fn an_empty_title_falls_back_to_the_running_program() {
        let rendered = render(&TabTitleRules::default(), context("", "/usr/bin/zsh"));

        assert_eq!(rendered, "  Zsh  ");
    }

    #[test]
    fn a_placeholder_title_is_treated_as_no_title() {
        // A shell that reports "default" has told us nothing.
        let rendered = render(&TabTitleRules::default(), context("default", "/bin/bash"));

        assert_eq!(rendered, "  Bash  ");
    }

    #[test]
    fn a_windows_program_loses_its_extension() {
        let rendered = render(
            &TabTitleRules::default(),
            context("", r"C:\Program Files\PowerShell\7\pwsh.exe"),
        );

        assert_eq!(rendered, "  Pwsh  ");
    }

    #[test]
    fn the_extension_is_stripped_whatever_its_case() {
        let rendered = render(&TabTitleRules::default(), context("", r"C:\bin\TOOL.EXE"));

        // Windows does not care about the case, so neither should the tab.
        assert_eq!(rendered, "  TOOL  ");
    }

    #[test]
    fn both_path_separators_are_understood_on_every_platform() {
        // A remote shell reports POSIX paths even on Windows.
        assert_eq!(
            render(&TabTitleRules::default(), context("", "/usr/local/bin/vim")),
            "  Vim  "
        );
        assert_eq!(
            render(&TabTitleRules::default(), context("", r"C:\tools\vim")),
            "  Vim  "
        );
    }

    #[test]
    fn a_trailing_separator_does_not_produce_an_empty_name() {
        let rendered = render(&TabTitleRules::default(), context("", "/usr/bin/"));

        assert_eq!(rendered, "  Bin  ");
    }

    #[test]
    fn nothing_at_all_still_names_the_tab() {
        let rendered = render(&TabTitleRules::default(), context("", ""));

        // An unnamed tab is worse than a generically named one.
        assert_eq!(rendered, "  Terminal  ");
    }

    #[test]
    fn the_index_can_be_shown() {
        let rules = TabTitleRules {
            format: "{index}: {title}".to_string(),
            ..TabTitleRules::default()
        };

        assert_eq!(
            render(
                &rules,
                TabContext {
                    pane_title: "logs",
                    process_path: "",
                    index: 3,
                }
            ),
            "3: Logs"
        );
    }

    #[test]
    fn capitalizing_leaves_scripts_without_case_alone() {
        let rendered = render(&TabTitleRules::default(), context("终端", ""));

        assert_eq!(rendered, "  终端  ");
    }

    #[test]
    fn tidying_can_be_turned_off() {
        let rules = TabTitleRules {
            strip_extension: false,
            capitalize: false,
            ..TabTitleRules::default()
        };

        assert_eq!(
            render(&rules, context("", r"C:\bin\pwsh.exe")),
            "  pwsh.exe  "
        );
    }

    #[test]
    fn an_unknown_placeholder_is_reported_rather_than_rendered_literally() {
        // Same failure as a config key that does nothing: the user edits, the
        // tab does not change, and nothing says why.
        assert_eq!(unknown_placeholders("{tittle}"), vec!["tittle".to_string()]);
        assert!(unknown_placeholders("  {title} {index}  ").is_empty());
    }

    #[test]
    fn an_unclosed_placeholder_does_not_hang_the_scan() {
        assert!(unknown_placeholders("{title").is_empty());
    }
}
