//! What the agents are doing, on screen.
//!
//! This is the product's reason to exist: several AI agents running in
//! several panes, and a person who needs to know which of them is waiting on
//! them without visiting each one. The data layer already tracks it --
//! `unterm_services::cockpit` watches screens, titles and hook signals -- so
//! what is here is the part a window owns: what the rows say, what order they
//! come in, and which one needs the person now.
//!
//! Sorting is the whole feature. An inbox that lists panes in pane order is a
//! list of panes; one that puts "waiting for you, for the longest" at the top
//! is an inbox.

use unterm_services::cockpit::status::{AgentState, PaneAgentStatus};

/// A row of the inbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub pane_id: u64,
    /// What the row says: agent, state, and how long it has been that way.
    pub label: String,
    /// The task, if the agent said what it was doing.
    pub hint: String,
    /// True when this pane wants the person.
    pub needs_you: bool,
}

/// Whether a state is one the person has to answer.
pub fn needs_attention(state: AgentState) -> bool {
    matches!(state, AgentState::WaitingForUser | AgentState::Done)
}

/// How a duration reads in an inbox.
///
/// Coarse on purpose: "4m" is the answer to "has this been sitting a while",
/// and a ticking seconds counter redraws the row every second to say nothing.
pub fn describe_age(seconds: u64) -> String {
    match seconds {
        0..=9 => "now".to_string(),
        10..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m", seconds / 60),
        _ => format!("{}h", seconds / 3600),
    }
}

/// What a state is called in a row.
pub fn describe_state(state: AgentState) -> &'static str {
    match state {
        AgentState::WaitingForUser => "waiting for you",
        AgentState::Working => "working",
        AgentState::Done => "done",
        AgentState::Idle => "idle",
    }
}

/// Build the inbox's rows from the tracked statuses.
///
/// `age_of` gives each pane's seconds-in-state, which the caller measures --
/// keeping the clock out of here is what lets the ordering be tested.
pub fn rows(
    statuses: &[PaneAgentStatus],
    mut age_of: impl FnMut(&PaneAgentStatus) -> u64,
) -> Vec<Row> {
    let mut rows: Vec<(u8, u64, Row)> = statuses
        .iter()
        .map(|status| {
            let age = age_of(status);
            let row = Row {
                pane_id: status.pane_id,
                label: format!(
                    "{}  {}  {}",
                    status.agent,
                    describe_state(status.state),
                    describe_age(age)
                ),
                hint: status.task_hint.clone().unwrap_or_default(),
                needs_you: needs_attention(status.state),
            };
            (rank(status.state), age, row)
        })
        .collect();

    // Waiting first, longest-waiting at the top of that: the person should be
    // able to answer the oldest question without reading the list.
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    rows.into_iter().map(|(_, _, row)| row).collect()
}

fn rank(state: AgentState) -> u8 {
    match state {
        AgentState::WaitingForUser => 0,
        AgentState::Done => 1,
        AgentState::Working => 2,
        AgentState::Idle => 3,
    }
}

/// How many panes want the person.
///
/// For the tab bar, where a number is all there is room for.
pub fn attention_count(statuses: &[PaneAgentStatus]) -> usize {
    statuses
        .iter()
        .filter(|status| needs_attention(status.state))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn status(pane_id: u64, agent: &str, state: AgentState) -> PaneAgentStatus {
        PaneAgentStatus {
            pane_id,
            agent: agent.to_string(),
            state,
            since: Instant::now(),
            task_hint: None,
            last_signal: "test",
            fleet_id: None,
        }
    }

    #[test]
    fn waiting_panes_come_first() {
        let statuses = [
            status(1, "claude", AgentState::Working),
            status(2, "codex", AgentState::WaitingForUser),
            status(3, "gemini", AgentState::Idle),
        ];
        let rows = rows(&statuses, |_| 0);
        assert_eq!(rows[0].pane_id, 2, "the one wanting an answer goes first");
        assert!(rows[0].needs_you);
    }

    #[test]
    fn the_longest_wait_is_at_the_top() {
        // The person should be able to answer the oldest question without
        // reading the list.
        let statuses = [
            status(1, "claude", AgentState::WaitingForUser),
            status(2, "codex", AgentState::WaitingForUser),
        ];
        let rows = rows(&statuses, |status| if status.pane_id == 2 { 300 } else { 5 });
        assert_eq!(rows[0].pane_id, 2);
    }

    #[test]
    fn done_outranks_working_but_not_waiting() {
        let statuses = [
            status(1, "a", AgentState::Working),
            status(2, "b", AgentState::Done),
            status(3, "c", AgentState::WaitingForUser),
        ];
        let order: Vec<u64> = rows(&statuses, |_| 0).iter().map(|r| r.pane_id).collect();
        assert_eq!(order, [3, 2, 1]);
    }

    #[test]
    fn a_row_says_which_agent_what_it_is_doing_and_for_how_long() {
        let statuses = [status(7, "claude", AgentState::WaitingForUser)];
        let rows = rows(&statuses, |_| 125);
        assert!(rows[0].label.contains("claude"));
        assert!(rows[0].label.contains("waiting for you"));
        assert!(rows[0].label.contains("2m"));
    }

    #[test]
    fn ages_are_coarse_rather_than_ticking() {
        // A seconds counter redraws the row every second to say nothing.
        assert_eq!(describe_age(3), "now");
        assert_eq!(describe_age(45), "45s");
        assert_eq!(describe_age(90), "1m");
        assert_eq!(describe_age(7200), "2h");
    }

    #[test]
    fn only_waiting_and_done_want_the_person() {
        assert!(needs_attention(AgentState::WaitingForUser));
        assert!(needs_attention(AgentState::Done));
        assert!(!needs_attention(AgentState::Working));
        assert!(!needs_attention(AgentState::Idle));
    }

    #[test]
    fn the_count_is_what_the_tab_bar_has_room_for() {
        let statuses = [
            status(1, "a", AgentState::WaitingForUser),
            status(2, "b", AgentState::Working),
            status(3, "c", AgentState::Done),
        ];
        assert_eq!(attention_count(&statuses), 2);
    }

    #[test]
    fn an_empty_cockpit_has_no_rows_and_wants_nothing() {
        assert!(rows(&[], |_| 0).is_empty());
        assert_eq!(attention_count(&[]), 0);
    }

    #[test]
    fn a_task_hint_is_carried_through_when_the_agent_gave_one() {
        let mut status = status(1, "claude", AgentState::Working);
        status.task_hint = Some("refactoring the parser".to_string());
        let rows = rows(&[status], |_| 0);
        assert_eq!(rows[0].hint, "refactoring the parser");
    }
}
