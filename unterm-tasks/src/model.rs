//! What a task is, and the only shapes it is allowed to take.
//!
//! This is freeze point F2: task identity, the state machine, the event
//! envelope and the cursor. Everything downstream — the Cockpit, the Fleet
//! page, the Brain runtime, any provider — reads projections of these, so a
//! change here is a change to every reader at once. Add states reluctantly;
//! removing one is a migration.

use serde::{Deserialize, Serialize};

/// A typed identifier.
///
/// Task, run and step ids are all opaque strings, which is exactly how they
/// get swapped for each other at a call site that compiles fine and then
/// looks up nothing. The prefix makes the mistake visible in a log line and
/// the newtype makes it a compile error.
macro_rules! id_type {
    ($name:ident, $prefix:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// A fresh identifier.
            pub fn new() -> Self {
                Self(format!("{}{}", $prefix, uuid::Uuid::new_v4().simple()))
            }

            /// Adopt an identifier that already exists, rejecting one that
            /// belongs to a different kind of thing.
            pub fn parse(raw: &str) -> anyhow::Result<Self> {
                if !raw.starts_with($prefix) {
                    anyhow::bail!(
                        "{raw:?} is not a {} identifier (expected the {:?} prefix)",
                        stringify!($name),
                        $prefix
                    );
                }
                Ok(Self(raw.to_string()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

id_type!(TaskId, "tsk_", "Identifies one thing the user or an agent asked for.");
id_type!(RunId, "run_", "Identifies one attempt at a task.");
id_type!(StepId, "stp_", "Identifies one unit of work inside a run.");

/// Where a task, run or step is in its life.
///
/// One enum for all three on purpose: they answer the same question, readers
/// render them the same way, and three near-identical enums is three places
/// to forget a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Created, nobody has started it.
    Pending,
    /// Someone holds it and is working.
    Running,
    /// Finished, did what it was for.
    Succeeded,
    /// Finished, did not.
    Failed,
    /// Stopped because someone asked it to stop.
    Cancelled,
    /// Stopped because whoever was running it disappeared. Distinct from
    /// `Failed`: the work did not report a verdict, so nobody knows how far
    /// it got, and that is what a reader has to be told.
    Interrupted,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Pending => "pending",
            State::Running => "running",
            State::Succeeded => "succeeded",
            State::Failed => "failed",
            State::Cancelled => "cancelled",
            State::Interrupted => "interrupted",
        }
    }

    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        Ok(match raw {
            "pending" => State::Pending,
            "running" => State::Running,
            "succeeded" => State::Succeeded,
            "failed" => State::Failed,
            "cancelled" => State::Cancelled,
            "interrupted" => State::Interrupted,
            other => anyhow::bail!("unknown state {other:?}"),
        })
    }

    /// Whether nothing more can happen to it.
    ///
    /// A terminal state is not a resting place a retry moves out of: retrying
    /// makes a *new run*, so the record of what the failed attempt did stays
    /// exactly as it was left.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            State::Succeeded | State::Failed | State::Cancelled | State::Interrupted
        )
    }

    /// Whether this state may follow that one.
    pub fn may_follow(self, previous: State) -> bool {
        match previous {
            State::Pending => matches!(self, State::Running | State::Cancelled),
            // A running thing can end four ways, and `Interrupted` is the one
            // nobody chooses: it is what reconciliation writes when the worker
            // holding it stopped answering.
            State::Running => self.is_terminal(),
            _ => false,
        }
    }
}

/// One thing that was asked for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    /// What kind of work this is, for readers that group or filter. Free-form
    /// on purpose: the store has no opinion about what a task can be.
    pub kind: String,
    pub title: String,
    pub state: State,
    /// Increments on every write. A reader that saw version N and wants to
    /// change the row says so, and a stale writer loses instead of silently
    /// overwriting a decision it never saw.
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    /// The caller's own data, carried and never interpreted. Fleet keeps a
    /// worktree and a branch here; the engine keeps its hands off.
    pub detail: serde_json::Value,
}

/// One attempt at a task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    pub task_id: TaskId,
    /// 1 for the first attempt. Retries add runs rather than resetting one.
    pub attempt: i64,
    pub state: State,
    pub version: i64,
    pub started_at: String,
    pub updated_at: String,
    pub ended_at: Option<String>,
}

/// One unit of work inside a run — typically a tool call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub id: StepId,
    pub run_id: RunId,
    /// Position within the run, so a reader can show them in the order they
    /// were asked for rather than the order they finished.
    pub ordinal: i64,
    pub kind: String,
    pub state: State,
    pub version: i64,
    /// The caller's promise that two steps carrying the same key are the same
    /// request. The store enforces it: the second insert returns the first
    /// step instead of creating a second one, so a retried message cannot
    /// perform its side effect twice.
    pub idempotency_key: Option<String>,
    /// Which worker holds it, while one does.
    pub claimed_by: Option<String>,
    /// When that claim stops being believed. A worker that dies stops
    /// renewing, the lease lapses, and reconciliation can tell the difference
    /// between "still working" and "gone".
    pub lease_expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// As on [`Task`]: the caller's data, opaque to the engine.
    pub detail: serde_json::Value,
}

/// Something that happened, in the order it happened.
///
/// `seq` is the cursor the whole system reads by: a subscriber remembers the
/// last one it saw and asks for what came after. It is assigned by the
/// database, so two writers cannot invent the same position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub seq: i64,
    pub at: String,
    pub task_id: Option<TaskId>,
    pub run_id: Option<RunId>,
    pub step_id: Option<StepId>,
    pub kind: String,
    pub payload: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_do_not_impersonate_each_other() {
        let task = TaskId::new();
        assert!(task.as_str().starts_with("tsk_"));
        assert!(TaskId::parse(task.as_str()).is_ok());
        // The whole point of the prefix: a run id offered where a task id
        // belongs is refused instead of looking up nothing.
        let run = RunId::new();
        assert!(TaskId::parse(run.as_str()).is_err());
        assert!(StepId::parse(run.as_str()).is_err());
    }

    #[test]
    fn identifiers_are_unique() {
        let a = TaskId::new();
        let b = TaskId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn the_state_machine_has_exactly_the_edges_it_should() {
        use State::*;
        assert!(Running.may_follow(Pending));
        assert!(Cancelled.may_follow(Pending));
        // Work cannot succeed without having run: a store that allows it will
        // eventually be told a task finished that never started.
        assert!(!Succeeded.may_follow(Pending));
        assert!(!Failed.may_follow(Pending));
        assert!(!Interrupted.may_follow(Pending));

        for end in [Succeeded, Failed, Cancelled, Interrupted] {
            assert!(end.may_follow(Running), "{end:?} must be able to end a run");
        }
        assert!(!Running.may_follow(Running));
        assert!(!Pending.may_follow(Running));

        // Terminal means terminal. A retry is a new run, so nothing leaves.
        for terminal in [Succeeded, Failed, Cancelled, Interrupted] {
            assert!(terminal.is_terminal());
            for next in [Pending, Running, Succeeded, Failed, Cancelled, Interrupted] {
                assert!(
                    !next.may_follow(terminal),
                    "{next:?} must not follow the terminal {terminal:?}"
                );
            }
        }
        assert!(!Pending.is_terminal());
        assert!(!Running.is_terminal());
    }

    #[test]
    fn states_round_trip_through_their_wire_names() {
        for state in [
            State::Pending,
            State::Running,
            State::Succeeded,
            State::Failed,
            State::Cancelled,
            State::Interrupted,
        ] {
            assert_eq!(State::parse(state.as_str()).unwrap(), state);
            // And through serde, which is what a reader on the far side of
            // the event stream actually parses.
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, format!("\"{}\"", state.as_str()));
        }
        assert!(State::parse("half-done").is_err());
    }
}
