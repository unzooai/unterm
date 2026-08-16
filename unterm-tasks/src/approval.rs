//! Standing permissions, and the questions that create them.
//!
//! Two different things live here and the difference matters. An **approval**
//! is one question about one action, waiting for an answer. A **grant** is the
//! answer generalised — "stop asking me about this" — and it is the dangerous
//! one, because it is the thing that will still be saying yes in a week when
//! nobody remembers agreeing to it.
//!
//! Both are rows, so both survive a restart. An approval that evaporated when
//! the process died would mean an agent's request silently becoming a refusal;
//! a grant that evaporated would mean the user being asked the same question
//! forever, which is how people learn to click yes without reading.

use crate::model::State;
use serde::{Deserialize, Serialize};

/// What a permission was given *about*.
///
/// Deliberately about intent rather than duration: "just this once" and "for
/// this task" are different promises even when they happen to expire at the
/// same moment, and collapsing them loses the user's actual answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Covers exactly one action and is spent.
    Once,
    /// Covers work done for one task.
    Task,
    /// Covers actions on one resource — a path, a branch, a pane.
    Resource,
    /// Covers matching actions until it expires or is revoked.
    Always,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Once => "once",
            Scope::Task => "task",
            Scope::Resource => "resource",
            Scope::Always => "always",
        }
    }

    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        Ok(match raw {
            "once" => Scope::Once,
            "task" => Scope::Task,
            "resource" => Scope::Resource,
            "always" => Scope::Always,
            other => anyhow::bail!("unknown grant scope {other:?}"),
        })
    }
}

/// What the user agreed to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grant {
    pub id: String,
    pub scope: Scope,
    /// `None` means any method — the broadest thing a user can give, which is
    /// why the UI that offers it has to say so.
    pub method: Option<String>,
    pub actor: Option<String>,
    pub task_id: Option<String>,
    pub resource: Option<String>,
    /// The most dangerous action this covers. A permission given for a local
    /// mutation must never quietly start covering destruction.
    pub max_risk: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub consumed_at: Option<String>,
}

impl Grant {
    /// Whether it can still say yes to anything.
    pub fn is_live(&self, now: &str) -> bool {
        if self.revoked_at.is_some() || self.consumed_at.is_some() {
            return false;
        }
        match &self.expires_at {
            // RFC3339 in UTC sorts lexicographically, which is why every
            // timestamp in this crate is written that way.
            Some(expiry) => expiry.as_str() > now,
            None => true,
        }
    }
}

/// What to create a grant with.
#[derive(Clone, Debug, Default)]
pub struct NewGrant {
    pub scope_or_once: Option<Scope>,
    pub method: Option<String>,
    pub actor: Option<String>,
    pub task_id: Option<String>,
    pub resource: Option<String>,
    pub max_risk: Option<String>,
    /// Seconds from now. `None` never expires, which the caller should offer
    /// reluctantly.
    pub ttl_seconds: Option<i64>,
}

/// One question about one action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Approval {
    pub id: String,
    pub method: String,
    pub actor: Option<String>,
    pub task_id: Option<String>,
    pub resource: Option<String>,
    pub command: Option<String>,
    pub risk: String,
    pub state: ApprovalState,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub decided_at: Option<String>,
    pub decided_by: Option<String>,
    pub grant_id: Option<String>,
}

/// Where a question got to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Allowed,
    Denied,
    /// Nobody answered in time. Distinct from `Denied`: nobody said no, they
    /// just were not there — and an action that proceeds on that basis is the
    /// failure this whole subsystem exists to prevent.
    Expired,
    /// The grant that would have answered it was taken away first.
    Revoked,
}

impl ApprovalState {
    pub fn as_str(self) -> &'static str {
        match self {
            ApprovalState::Pending => "pending",
            ApprovalState::Allowed => "allowed",
            ApprovalState::Denied => "denied",
            ApprovalState::Expired => "expired",
            ApprovalState::Revoked => "revoked",
        }
    }

    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        Ok(match raw {
            "pending" => ApprovalState::Pending,
            "allowed" => ApprovalState::Allowed,
            "denied" => ApprovalState::Denied,
            "expired" => ApprovalState::Expired,
            "revoked" => ApprovalState::Revoked,
            other => anyhow::bail!("unknown approval state {other:?}"),
        })
    }

    pub fn is_settled(self) -> bool {
        !matches!(self, ApprovalState::Pending)
    }
}

/// What the gateway asks a grant about.
///
/// Mirrors the gateway's `ActionContext` without depending on it: this crate
/// sits below the gateway and must not know its types.
#[derive(Clone, Debug, Default)]
pub struct Ask {
    pub method: String,
    pub actor: Option<String>,
    pub task_id: Option<String>,
    pub resource: Option<String>,
    /// Ordered so a comparison against a grant's ceiling means something.
    pub risk_rank: u8,
}

/// Rank a risk name so grants can carry a ceiling.
///
/// Unknown names rank at the top rather than the bottom: a risk this build
/// does not recognise is one it cannot reason about, and the safe reading of
/// "I do not know how dangerous this is" is not "probably fine".
pub fn risk_rank(risk: &str) -> u8 {
    match risk {
        "readonly" => 0,
        "local_mutation" => 1,
        "destructive" => 2,
        _ => u8::MAX,
    }
}

pub(crate) fn matches(grant: &Grant, ask: &Ask, now: &str) -> bool {
    if !grant.is_live(now) {
        return false;
    }
    if risk_rank(&grant.max_risk) < ask.risk_rank {
        return false;
    }
    if let Some(method) = &grant.method {
        if method != &ask.method {
            return false;
        }
    }
    if let Some(actor) = &grant.actor {
        if Some(actor) != ask.actor.as_ref() {
            return false;
        }
    }
    match grant.scope {
        // A task-scoped grant that is not about this task is not about this
        // action, however well everything else lines up.
        Scope::Task => grant.task_id.is_some() && grant.task_id == ask.task_id,
        Scope::Resource => grant.resource.is_some() && grant.resource == ask.resource,
        Scope::Once | Scope::Always => true,
    }
}

/// A step's state, for callers that mirror an approval onto one.
pub fn state_for(approval: ApprovalState) -> State {
    match approval {
        ApprovalState::Pending => State::Pending,
        ApprovalState::Allowed => State::Running,
        ApprovalState::Denied | ApprovalState::Revoked => State::Cancelled,
        ApprovalState::Expired => State::Interrupted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(scope: Scope) -> Grant {
        Grant {
            id: "g1".to_string(),
            scope,
            method: None,
            actor: None,
            task_id: None,
            resource: None,
            max_risk: "destructive".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: None,
            revoked_at: None,
            consumed_at: None,
        }
    }

    fn ask(method: &str) -> Ask {
        Ask {
            method: method.to_string(),
            risk_rank: risk_rank("destructive"),
            ..Ask::default()
        }
    }

    #[test]
    fn a_revoked_or_spent_grant_says_nothing() {
        let now = "2026-06-01T00:00:00Z";
        let mut revoked = grant(Scope::Always);
        revoked.revoked_at = Some(now.to_string());
        assert!(!matches(&revoked, &ask("session.destroy"), now));

        let mut spent = grant(Scope::Once);
        spent.consumed_at = Some(now.to_string());
        assert!(!matches(&spent, &ask("session.destroy"), now));
    }

    #[test]
    fn a_grant_stops_covering_things_once_it_expires() {
        let mut g = grant(Scope::Always);
        g.expires_at = Some("2026-06-01T00:00:00Z".to_string());
        assert!(matches(&g, &ask("session.destroy"), "2026-05-31T23:59:59Z"));
        assert!(!matches(&g, &ask("session.destroy"), "2026-06-01T00:00:01Z"));
    }

    #[test]
    fn a_grant_never_covers_something_more_dangerous_than_it_was_given_for() {
        let mut g = grant(Scope::Always);
        g.max_risk = "local_mutation".to_string();
        let now = "2026-06-01T00:00:00Z";

        let mut mutation = ask("session.input");
        mutation.risk_rank = risk_rank("local_mutation");
        assert!(matches(&g, &mutation, now));

        let destructive = ask("session.destroy");
        assert!(
            !matches(&g, &destructive, now),
            "permission for a mutation must not quietly cover destruction"
        );
    }

    #[test]
    fn an_unrecognised_risk_is_treated_as_the_worst_case() {
        // A build that does not know what a risk means cannot reason about
        // it, and "I do not know how dangerous this is" must not read as
        // "probably fine".
        assert_eq!(risk_rank("something-from-the-future"), u8::MAX);
        let g = grant(Scope::Always);
        let mut unknown = ask("whatever");
        unknown.risk_rank = risk_rank("something-from-the-future");
        assert!(!matches(&g, &unknown, "2026-06-01T00:00:00Z"));
    }

    #[test]
    fn scopes_only_cover_what_they_name() {
        let now = "2026-06-01T00:00:00Z";

        let mut task_scoped = grant(Scope::Task);
        task_scoped.task_id = Some("tsk_1".to_string());
        let mut same_task = ask("session.destroy");
        same_task.task_id = Some("tsk_1".to_string());
        let mut other_task = ask("session.destroy");
        other_task.task_id = Some("tsk_2".to_string());
        assert!(matches(&task_scoped, &same_task, now));
        assert!(!matches(&task_scoped, &other_task, now));
        // And a task-scoped grant with no task named covers nothing, rather
        // than everything.
        let mut nameless = grant(Scope::Task);
        nameless.task_id = None;
        assert!(!matches(&nameless, &same_task, now));

        let mut resource_scoped = grant(Scope::Resource);
        resource_scoped.resource = Some("/repo/src".to_string());
        let mut same_resource = ask("session.destroy");
        same_resource.resource = Some("/repo/src".to_string());
        let mut elsewhere = ask("session.destroy");
        elsewhere.resource = Some("/etc".to_string());
        assert!(matches(&resource_scoped, &same_resource, now));
        assert!(!matches(&resource_scoped, &elsewhere, now));
    }

    #[test]
    fn a_grant_that_names_a_method_or_an_actor_is_held_to_it() {
        let now = "2026-06-01T00:00:00Z";
        let mut g = grant(Scope::Always);
        g.method = Some("exec.run".to_string());
        g.actor = Some("claude".to_string());

        let mut right = ask("exec.run");
        right.actor = Some("claude".to_string());
        assert!(matches(&g, &right, now));

        let mut wrong_method = ask("session.destroy");
        wrong_method.actor = Some("claude".to_string());
        assert!(!matches(&g, &wrong_method, now));

        let mut wrong_actor = ask("exec.run");
        wrong_actor.actor = Some("someone-else".to_string());
        assert!(!matches(&g, &wrong_actor, now));

        // An action with no actor cannot borrow a grant given to one.
        assert!(!matches(&g, &ask("exec.run"), now));
    }

    #[test]
    fn an_unanswered_question_is_not_a_refusal() {
        // The states have to stay distinguishable: "nobody was there" and
        // "somebody said no" call for different behaviour from the caller,
        // and only one of them should be retried.
        assert_ne!(ApprovalState::Expired, ApprovalState::Denied);
        assert!(ApprovalState::Expired.is_settled());
        assert!(!ApprovalState::Pending.is_settled());
        assert_eq!(state_for(ApprovalState::Expired), State::Interrupted);
        assert_eq!(state_for(ApprovalState::Denied), State::Cancelled);
        assert_eq!(state_for(ApprovalState::Allowed), State::Running);
    }
}
