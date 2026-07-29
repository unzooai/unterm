//! The strip along the bottom that says where you are and what agents are up
//! to.
//!
//! Two halves. On the left, the shell and the directory it is in -- the two
//! things people look down for. On the right, chips: `mcp:N` for how many
//! times an agent has written to a pane, and the proxy in force if there is
//! one. An agent driving this terminal is invisible otherwise, and a terminal
//! that hides that is the wrong terminal for the job.
//!
//! Laid out here rather than in the event handler because the interesting part
//! is what happens when there is not enough room. Chips are dropped before the
//! directory is, and the directory is shortened from the left, so the end of
//! the path -- the part that says which project -- survives. A previous
//! version of this hid the chips behind a 208-column window, wider than a
//! laptop screen can reach, so they were effectively never drawn; the tiers
//! here are pinned by tests naming the sizes real screens have.

/// How tall the bar is, in cells.
pub const ROWS: usize = 1;

/// Below this width there is no room for anything but the directory.
const CHIPS_FROM_COLUMNS: usize = 60;

/// A program's name without the path that found it.
///
/// The engine reports what it launched, which on Windows is
/// `C:\WINDOWS\system32\cmd.exe`. Half the bar spent saying where cmd lives
/// is half a bar not saying where *you* are.
pub fn short_name(program: &str) -> String {
    program
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(program)
        .to_string()
}

/// A proxy as host and port, without the scheme.
///
/// `http://127.0.0.1:7897` is a quarter of a narrow bar, and the scheme is the
/// part nobody is checking.
pub fn short_proxy(url: &str) -> String {
    let without_scheme = url.rsplit("://").next().unwrap_or(url);
    without_scheme.trim_end_matches('/').to_string()
}

/// What the bar has to say.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    pub shell: String,
    pub directory: String,
    /// How many times an agent has written to a pane this session.
    pub agent_writes: u64,
    /// How many agent writes are waiting on the user to allow them.
    pub pending: usize,
    /// The proxy in force, if any.
    pub proxy: Option<String>,
}

/// One piece of the bar, already placed.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub column: usize,
    pub text: String,
    /// Chips are drawn dimmer than the left-hand text, which is what makes
    /// the directory readable at a glance rather than one of five things.
    pub dim: bool,
}

/// Lay the bar out for a window `columns` wide.
///
/// Empty when there is no room for even a shortened directory: a bar with one
/// letter in it is a row of output spent on nothing.
pub fn segments(status: &Status, columns: usize) -> Vec<Segment> {
    if columns < 12 {
        return Vec::new();
    }

    let chips = if columns >= CHIPS_FROM_COLUMNS {
        chips_for(status)
    } else {
        Vec::new()
    };
    let chip_text = chips.join("  ");
    // A gap eitherhand of the chips, so they do not touch the path or the edge.
    let reserved = if chip_text.is_empty() {
        0
    } else {
        chip_text.chars().count() + 2
    };

    let mut segments = Vec::new();
    let left_room = columns.saturating_sub(reserved);
    let left = left_text(status, left_room);
    if !left.is_empty() {
        segments.push(Segment {
            column: 0,
            text: left,
            dim: false,
        });
    }
    if !chip_text.is_empty() {
        segments.push(Segment {
            column: columns - chip_text.chars().count(),
            text: chip_text,
            dim: true,
        });
    }
    segments
}

/// The shell and directory, shortened to fit.
fn left_text(status: &Status, room: usize) -> String {
    if room == 0 {
        return String::new();
    }
    let shell = status.shell.trim();
    let directory = status.directory.trim();
    if directory.is_empty() {
        return truncate_start(shell, room);
    }
    let prefix = if shell.is_empty() {
        String::new()
    } else {
        format!("{shell}  ")
    };
    let prefix_width = prefix.chars().count();
    if prefix_width + 8 <= room {
        format!("{prefix}{}", truncate_start(directory, room - prefix_width))
    } else {
        // Not enough room for both: the directory is the one worth keeping.
        truncate_start(directory, room)
    }
}

/// Shorten from the left, so the end of a path survives.
///
/// The end is the part that says which project this is; the beginning is
/// `C:\Users\somebody\code` on every pane at once.
fn truncate_start(text: &str, room: usize) -> String {
    let width = text.chars().count();
    if width <= room {
        return text.to_string();
    }
    if room <= 1 {
        return "…".repeat(room);
    }
    let kept: String = text.chars().skip(width - (room - 1)).collect();
    format!("…{kept}")
}

/// The right-hand chips, in the order they are dropped when room runs out --
/// which is to say, least important last.
fn chips_for(status: &Status) -> Vec<String> {
    let mut chips = Vec::new();
    if status.pending > 0 {
        // Something is waiting on the user. First, and worded as a question
        // rather than a count, because a number here is easy to read past.
        chips.push(format!("{} waiting on you", status.pending));
    }
    if status.agent_writes > 0 {
        chips.push(format!("mcp:{}", status.agent_writes));
    }
    if let Some(proxy) = &status.proxy {
        chips.push(format!("proxy:{proxy}"));
    }
    chips
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> Status {
        Status {
            shell: "pwsh".to_string(),
            directory: r"D:\code\unterm".to_string(),
            agent_writes: 7,
            pending: 0,
            proxy: None,
        }
    }

    fn rendered(status: &Status, columns: usize) -> String {
        let mut line = vec![' '; columns];
        for segment in segments(status, columns) {
            for (offset, ch) in segment.text.chars().enumerate() {
                if let Some(slot) = line.get_mut(segment.column + offset) {
                    *slot = ch;
                }
            }
        }
        line.into_iter().collect::<String>().trim_end().to_string()
    }

    #[test]
    fn a_shell_is_named_not_located() {
        assert_eq!(short_name(r"C:\WINDOWS\system32\cmd.exe"), "cmd.exe");
        assert_eq!(short_name("/usr/bin/zsh"), "zsh");
        assert_eq!(short_name("pwsh"), "pwsh");
        assert_eq!(short_name(""), "");
    }

    #[test]
    fn a_proxy_is_shown_as_host_and_port() {
        assert_eq!(short_proxy("http://127.0.0.1:7897"), "127.0.0.1:7897");
        assert_eq!(short_proxy("socks5://proxy.internal:1080/"), "proxy.internal:1080");
        assert_eq!(short_proxy("127.0.0.1:7897"), "127.0.0.1:7897");
    }

    #[test]
    fn the_shell_and_the_directory_are_on_the_left() {
        let line = rendered(&status(), 100);
        assert!(line.starts_with("pwsh"), "{line:?}");
        assert!(line.contains(r"D:\code\unterm"), "{line:?}");
    }

    #[test]
    fn the_chips_are_on_the_right() {
        let segments = segments(&status(), 100);
        let chip = segments.last().unwrap();
        assert_eq!(chip.text, "mcp:7");
        assert_eq!(chip.column + chip.text.len(), 100, "flush to the edge");
    }

    /// A laptop at full screen is about 150 columns, and a split pane half
    /// that. The chips have to survive both: gating them behind a window
    /// wider than the screen is the same as not drawing them.
    #[test]
    fn the_chips_survive_a_laptop_sized_window() {
        for columns in [80, 100, 128, 150, 180] {
            let line = rendered(&status(), columns);
            assert!(line.contains("mcp:7"), "{columns} columns: {line:?}");
        }
    }

    #[test]
    fn a_narrow_pane_keeps_the_directory_and_drops_the_chips() {
        let line = rendered(&status(), 40);
        assert!(!line.contains("mcp:"), "{line:?}");
        assert!(line.contains("unterm"), "{line:?}");
    }

    /// Shortened from the left: the end of a path is the part that says which
    /// project this is. Cutting the other end leaves every pane looking the
    /// same.
    #[test]
    fn a_long_path_keeps_its_end() {
        let mut status = status();
        status.directory = r"C:\Users\somebody\code\projects\unterm\unterm-app".to_string();
        let line = rendered(&status, 40);
        assert!(line.ends_with("unterm-app"), "{line:?}");
        assert!(line.contains('…'), "the cut should be visible: {line:?}");
    }

    #[test]
    fn nothing_is_drawn_when_there_is_no_room_for_anything() {
        assert!(segments(&status(), 4).is_empty());
        assert!(segments(&status(), 0).is_empty());
    }

    /// Nothing must ever run past the edge: a segment that does is drawn over
    /// the pane beside it.
    #[test]
    fn no_segment_runs_off_the_end() {
        let mut status = status();
        status.pending = 2;
        status.proxy = Some("clash".to_string());
        status.directory = r"C:\Users\somebody\code\projects\unterm".to_string();
        for columns in 12..200 {
            for segment in segments(&status, columns) {
                assert!(
                    segment.column + segment.text.chars().count() <= columns,
                    "{columns} columns: {segment:?}"
                );
            }
        }
    }

    /// The left text must not run into the chips either.
    #[test]
    fn the_left_text_stops_before_the_chips() {
        let mut status = status();
        status.directory = r"C:\Users\somebody\code\projects\unterm\unterm-app\src".to_string();
        status.proxy = Some("clash".to_string());
        let segments = segments(&status, 70);
        assert_eq!(segments.len(), 2);
        let left = &segments[0];
        assert!(
            left.column + left.text.chars().count() <= segments[1].column,
            "{segments:?}"
        );
    }

    /// Something waiting on the user goes first and says so in words.
    #[test]
    fn a_pending_confirmation_leads_the_chips() {
        let mut status = status();
        status.pending = 1;
        let segments = segments(&status, 120);
        assert!(
            segments.last().unwrap().text.starts_with("1 waiting on you"),
            "{segments:?}"
        );
    }

    #[test]
    fn a_quiet_session_shows_no_chips_at_all() {
        let quiet = Status {
            shell: "pwsh".to_string(),
            directory: r"D:\code".to_string(),
            ..Default::default()
        };
        assert_eq!(segments(&quiet, 120).len(), 1, "just the directory");
    }
}
