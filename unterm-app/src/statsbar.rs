//! The line of facts about the pane in front, shown in the top bar.
//!
//! Four things, in the order they answer questions people actually ask of a
//! terminal: which agent is bound to this pane, which branch it is on and how
//! dirty, what its process is costing, and what it is running.
//!
//!     ⚡ claude    <branch> main *3 +1    8.0% cpu  1.4G  4m    ▶ cargo test
//!
//! Composed here rather than at the paint site because every rule in it is a
//! judgement -- when it appears, what is dropped first, what an empty value
//! looks like -- and none of those are checkable in a screenshot.

/// Below this the bar has no room for facts, and dropping them is better than
/// pushing the tabs off the edge.
///
/// Measured in logical pixels, so it does not move when the display's scale
/// does: the question is how much room the reader sees, not how many device
/// pixels the panel has.
pub const MIN_WIDTH: f32 = 900.0;

/// The gap between segments: four spaces.
///
/// Not a separator character. A run of segments that are already differently
/// shaped -- an icon, a name, a run of digits -- reads as a list without one,
/// and a dot between them puts a hard vertical edge where the eye wants to
/// skip.
const GAP: &str = "    ";

/// What is bound to this pane, if anything.
pub fn agent_segment(name: Option<&str>) -> String {
    match name {
        Some(name) if !name.trim().is_empty() => format!("\u{26A1} {}", name.trim()),
        _ => String::new(),
    }
}

/// What the pane is running, if it is running something worth naming.
///
/// The pane's own shell is not. A pane sitting at a prompt has its shell in
/// front, and saying "pwsh" in the bar above a pane that obviously contains a
/// shell is noise.
///
/// Which is decided by comparing the process in front with the one the pane
/// was started with, rather than by matching names against a list of shells.
/// The list gets this wrong in exactly the case the segment exists for: a pane
/// running `powershell` from a `cmd` prompt is running something, and a list
/// with "powershell" in it hides it.
pub fn title_segment(foreground: &str, shell: &str) -> String {
    let name = program_name(foreground);
    if name.is_empty() || name.eq_ignore_ascii_case(&program_name(shell)) {
        return String::new();
    }
    format!("\u{25B6} {name}")
}

/// A program's name with neither its path nor its extension.
///
/// `C:\Windows\System32\cmd.exe` and `cmd` are the same program, and the two
/// spellings turn up in the same comparison.
fn program_name(program: &str) -> String {
    program
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_string()
}

/// Join what there is to say, dropping what there is not.
///
/// Nothing at all when nothing is known, rather than a placeholder: an empty
/// slot in the bar is invisible, and a row of dashes is a thing to read.
pub fn compose(segments: &[String]) -> String {
    segments
        .iter()
        .filter(|segment| !segment.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(GAP)
}

/// Drop whole segments until the line fits `columns`, in the order given.
///
/// Whole segments rather than characters. Half a branch name and half a memory
/// figure are both worse than not showing them: the reader cannot tell a
/// truncated `main-experiment` from a branch called `main-exp`, and a number
/// that has lost its unit is a wrong number.
///
/// `give_up` names the indices to sacrifice, first to go first. It is separate
/// from the order they are shown in because those are different questions: the
/// numbers read best last and matter least, so a bar that drops from the end
/// loses what is running to make room for a memory figure.
pub fn fit(segments: &[String], give_up: &[usize], columns: usize) -> String {
    let mut kept: Vec<Option<String>> = segments
        .iter()
        .map(|segment| {
            Some(segment.clone()).filter(|segment| !segment.trim().is_empty())
        })
        .collect();
    let line_of = |kept: &[Option<String>]| {
        compose(&kept.iter().flatten().cloned().collect::<Vec<_>>())
    };

    // Anything the caller did not name is given up afterwards, so a short list
    // still ends at an empty line rather than at one that does not fit.
    for index in give_up.iter().copied().chain(0..segments.len()) {
        let line = line_of(&kept);
        if width_of(&line) <= columns {
            return line;
        }
        if let Some(slot) = kept.get_mut(index) {
            *slot = None;
        }
    }
    String::new()
}

/// How many cells a string takes, which is not how many characters it has.
pub fn width_of(text: &str) -> usize {
    text.chars().map(crate::terminal::column_width).sum()
}

/// The four segments for one pane, before they are fitted to a width.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    pub agent: String,
    pub git: String,
    pub process: String,
    pub title: String,
}

impl Facts {
    /// In the order they are shown, which is the order the questions are
    /// asked: whose pane is this, where is it, what is it costing, what is it
    /// doing.
    pub fn segments(&self) -> [String; 4] {
        [
            self.agent.clone(),
            self.git.clone(),
            self.process.clone(),
            self.title.clone(),
        ]
    }

    /// And the order they are given up in, which is not the same.
    ///
    /// The numbers go first: they read best at the end of the line and they
    /// are the least likely reason anyone looked. Then the branch, which is
    /// the longest and the slowest to change. What is running survives both,
    /// because it is the one fact here that is news. The agent survives
    /// everything: it says whose pane this is.
    pub const GIVE_UP: [usize; 4] = [2, 1, 3, 0];
}

/// How long a set of facts stays good.
///
/// A quarter of a second. Reading them means walking the machine's process
/// table, which costs tens of milliseconds -- and a frame is sixteen, so doing
/// it while painting would drop every frame that asked.
const FACTS_TTL: std::time::Duration = std::time::Duration::from_millis(250);

type FactsCache = std::collections::HashMap<usize, (std::time::Instant, Facts)>;

fn facts_cache() -> &'static parking_lot::Mutex<FactsCache> {
    static CACHE: std::sync::OnceLock<parking_lot::Mutex<FactsCache>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn refreshing() -> &'static parking_lot::Mutex<std::collections::HashSet<usize>> {
    static RUNNING: std::sync::OnceLock<parking_lot::Mutex<std::collections::HashSet<usize>>> =
        std::sync::OnceLock::new();
    RUNNING.get_or_init(Default::default)
}

/// What is known about a pane, refreshing behind the caller's back.
///
/// Returns whatever was known last, immediately -- including nothing, the
/// first time a pane is asked about. Everything underneath it (the process
/// table, git, the per-process sampler) is cached the same way, so the bar
/// stays a bar rather than becoming a reason the window is slow.
pub fn facts_for(pane_id: usize) -> Facts {
    let previous;
    {
        let cache = facts_cache().lock();
        match cache.get(&pane_id) {
            Some((at, facts)) if at.elapsed() < FACTS_TTL => return facts.clone(),
            Some((_, facts)) => previous = facts.clone(),
            None => previous = Facts::default(),
        }
    }

    let mut running = refreshing().lock();
    if running.insert(pane_id) {
        let spawned = std::thread::Builder::new()
            .name("stats-facts".into())
            .spawn(move || {
                let facts = read_facts(pane_id);
                facts_cache()
                    .lock()
                    .insert(pane_id, (std::time::Instant::now(), facts));
                refreshing().lock().remove(&pane_id);
            });
        if spawned.is_err() {
            running.remove(&pane_id);
        }
    }
    previous
}

/// Drop what is known about a pane that is gone, so a long-lived window does
/// not accumulate one entry per tab it has ever opened.
pub fn forget(pane_id: usize) {
    facts_cache().lock().remove(&pane_id);
}

/// Ask the machine. Only ever called from the refresh thread.
fn read_facts(pane_id: usize) -> Facts {
    // The engine handle carries no state of its own, so the refresh thread can
    // make its own rather than borrowing the window's.
    let engine = unterm_engine::next_core::NextCoreEngine;
    let activity = unterm_engine::SessionEngine::activity(&engine, pane_id).ok();
    let process = activity.as_ref().and_then(|activity| activity.process.clone());

    let directory = process
        .as_ref()
        .and_then(|process| {
            process
                .foreground_cwd
                .clone()
                .or_else(|| process.root_cwd.clone())
        })
        .or_else(|| {
            unterm_engine::SessionEngine::shell(&engine, pane_id)
                .ok()
                .and_then(|shell| shell.cwd)
        });

    Facts {
        agent: agent_segment(
            process
                .as_ref()
                .and_then(|process| process.detected_agent.as_deref()),
        ),
        git: directory
            .as_deref()
            // Blocking is fine here: this is already the refresh thread, and
            // waiting for git is what it is for.
            .map(|cwd| crate::git::read_cached(std::path::Path::new(cwd)))
            .map(|panel| crate::git::render_segment(&panel))
            .unwrap_or_default(),
        process: unterm_services::process_stats::render_segment(
            &process
                .as_ref()
                .and_then(|process| process.foreground_pid.or(process.root_pid))
                .and_then(unterm_services::process_stats::sample_now),
        ),
        title: title_segment(
            activity
                .as_ref()
                .map(|activity| activity.foreground_process.as_str())
                .unwrap_or_default(),
            process
                .as_ref()
                .map(|process| process.root_process.as_str())
                .unwrap_or_default(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bound_agent_is_named() {
        assert_eq!(agent_segment(Some("claude")), "\u{26A1} claude");
    }

    /// No agent is no segment -- not a lightning bolt on its own, which reads
    /// as an agent whose name failed to load.
    #[test]
    fn no_agent_is_no_segment() {
        assert_eq!(agent_segment(None), "");
        assert_eq!(agent_segment(Some("  ")), "");
    }

    /// A shell at its prompt is what a terminal contains. Saying so twice, in
    /// the pane and in the bar above it, is noise.
    #[test]
    fn a_shell_sitting_at_its_prompt_is_not_worth_naming() {
        for shell in ["pwsh", "bash", "zsh", "PowerShell.exe", "C:\\Windows\\cmd.exe"] {
            assert_eq!(title_segment(shell, shell), "", "{shell}");
        }
    }

    /// The path and the extension are spelling, not identity: the process in
    /// front comes back as `C:\Windows\System32\cmd.exe` while the pane was
    /// started with `cmd`, and those are the same shell.
    #[test]
    fn a_shell_is_recognised_however_it_is_spelled() {
        assert_eq!(title_segment("C:\\Windows\\System32\\cmd.exe", "cmd"), "");
        assert_eq!(title_segment("PWSH.EXE", "pwsh"), "");
        assert_eq!(title_segment("/usr/bin/zsh", "zsh"), "");
    }

    /// And the case the segment exists for: a shell running another shell. A
    /// list of shell names hides exactly this, which is why there is no list --
    /// the pane already knows which shell is its own.
    #[test]
    fn a_shell_running_another_shell_is_worth_naming() {
        assert_eq!(
            title_segment("powershell.exe", "cmd.exe"),
            "\u{25B6} powershell"
        );
        assert_eq!(title_segment("bash", "zsh"), "\u{25B6} bash");
    }

    #[test]
    fn anything_else_running_is_named() {
        assert_eq!(title_segment("cargo.exe", "cmd"), "\u{25B6} cargo");
        assert_eq!(title_segment("  vim ", "bash"), "\u{25B6} vim");
        assert_eq!(title_segment("", "bash"), "");
    }

    /// Knowing nothing about the pane's own shell is not a reason to stay
    /// quiet: naming what is in front is still right, and the only thing lost
    /// is the chance to recognise it as the shell.
    #[test]
    fn a_pane_with_no_shell_recorded_still_names_what_is_in_front() {
        assert_eq!(title_segment("cargo", ""), "\u{25B6} cargo");
        assert_eq!(title_segment("", ""), "");
    }

    /// Four spaces, so segments that are already differently shaped read as a
    /// list without a character drawing a line between them.
    #[test]
    fn segments_are_separated_by_a_gap_rather_than_a_mark() {
        let line = compose(&["a".into(), "b".into()]);
        assert_eq!(line, "a    b");
        assert!(!line.contains('\u{00B7}'), "a separator crept in");
    }

    #[test]
    fn empty_segments_leave_no_gap_behind_them() {
        assert_eq!(
            compose(&["a".into(), String::new(), "  ".into(), "b".into()]),
            "a    b"
        );
    }

    /// Nothing known shows nothing, not a placeholder: an empty slot in a bar
    /// is invisible, and a row of dashes is a thing the eye stops on.
    #[test]
    fn nothing_known_shows_nothing() {
        assert_eq!(compose(&[]), "");
        assert_eq!(compose(&[String::new(), "   ".into()]), "");
    }

    /// What is given up first is not what is shown last. The numbers go before
    /// the branch, and both go before what is running.
    #[test]
    fn a_narrow_bar_gives_up_the_numbers_before_anything_else() {
        let facts = Facts {
            agent: "A".into(),
            git: "GGGG".into(),
            process: "PPPP".into(),
            title: "TT".into(),
        };
        let fit_to = |columns| fit(&facts.segments(), &Facts::GIVE_UP, columns);
        assert_eq!(fit_to(100), "A    GGGG    PPPP    TT");
        // The numbers first.
        assert_eq!(fit_to(22), "A    GGGG    TT");
        // Then the branch.
        assert_eq!(fit_to(14), "A    TT");
        // Then what is running. The agent is last: it says whose pane this is.
        assert_eq!(fit_to(6), "A");
        assert_eq!(fit_to(0), "");
    }

    /// Whole segments, never half of one. Half a branch name reads as a branch
    /// with that name, and a memory figure without its unit is a wrong number.
    #[test]
    fn segments_are_dropped_whole() {
        let segments = vec!["main-experiment".to_string(), "1.4G".to_string()];
        let fitted = fit(&segments, &[1, 0], 17);
        assert!(
            fitted == "main-experiment" || fitted.is_empty(),
            "a segment was cut in half: {fitted:?}"
        );
    }

    /// An order that names fewer segments than there are still ends at an
    /// empty line rather than at one that does not fit.
    #[test]
    fn an_incomplete_order_still_gives_everything_up() {
        let segments = vec!["aaaa".to_string(), "bbbb".to_string()];
        assert_eq!(fit(&segments, &[1], 3), "");
    }

    /// Width is measured in cells. A CJK title takes two columns per character,
    /// and measuring it in characters puts the line through the tabs.
    #[test]
    fn width_is_counted_in_cells_rather_than_characters() {
        assert_eq!(width_of("abc"), 3);
        assert_eq!(width_of("\u{4E2D}\u{6587}"), 4);
        let wide = vec!["\u{4E2D}\u{6587}\u{4E2D}\u{6587}".to_string()];
        assert_eq!(fit(&wide, &[0], 7), "");
        assert_eq!(fit(&wide, &[0], 8), "\u{4E2D}\u{6587}\u{4E2D}\u{6587}");
    }

    /// The whole line, as it appears in a window wide enough for all of it.
    #[test]
    fn the_whole_line_reads_in_the_order_the_questions_are_asked() {
        let line = compose(&[
            agent_segment(Some("claude")),
            "\u{E0A0} main *3".to_string(),
            "8.0% cpu  1.4G  4m".to_string(),
            title_segment("cargo.exe", "pwsh"),
        ]);
        assert_eq!(
            line,
            "\u{26A1} claude    \u{E0A0} main *3    8.0% cpu  1.4G  4m    \u{25B6} cargo"
        );
    }
}
