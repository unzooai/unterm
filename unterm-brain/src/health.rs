//! Whether a brain is worth dispatching to.
//!
//! A fleet that keeps handing work to a model whose provider is down turns
//! one outage into a queue of identical failures. This keeps a short account
//! of how each (adapter, model) pair has been behaving so a launcher can skip
//! the ones that are clearly sick.
//!
//! Two decisions worth stating, because both are easy to get wrong in the
//! obvious direction:
//!
//! **Only failures count as failures.** A turn the user interrupted, or one
//! that hit a context cap, says nothing about the model's health — treating
//! either as a fault would make a busy user look like an outage and take
//! their model away from them.
//!
//! **Nothing is persisted.** A restarted Unterm starts optimistic. Pessimism
//! that survives a restart is pessimism nobody can clear: the provider
//! recovers, the file still says it is down, and the user has no idea why
//! their model is being skipped.

use crate::StopReason;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How many failures in a row before a brain is considered degraded. Three,
/// because one is noise and two is a coincidence.
pub const FAILURES_BEFORE_DEGRADED: u32 = 3;

/// How long a degraded brain is left alone before it is worth another try.
pub const COOLDOWN: Duration = Duration::from_secs(60);

/// What is known about one (adapter, model) pair.
#[derive(Clone, Debug, Default)]
pub struct Health {
    pub successes: u64,
    pub failures: u64,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    degraded_since: Option<Instant>,
}

impl Health {
    /// Degraded and still inside its cooldown.
    pub fn is_degraded(&self) -> bool {
        match self.degraded_since {
            Some(since) => since.elapsed() < COOLDOWN,
            None => false,
        }
    }
}

/// The health of every brain this process has run.
#[derive(Default)]
pub struct Registry {
    entries: Mutex<HashMap<String, Health>>,
}

fn key(adapter: &str, model: Option<&str>) -> String {
    format!("{adapter}/{}", model.unwrap_or("default"))
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record how a run ended.
    ///
    /// `error` is the diagnosis when there is one; it is kept so a caller can
    /// tell the user *why* a model is being skipped rather than only that it
    /// is.
    pub fn record(
        &self,
        adapter: &str,
        model: Option<&str>,
        stop: Option<StopReason>,
        error: Option<String>,
    ) {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.entry(key(adapter, model)).or_default();
        match stop {
            Some(StopReason::Error) => {
                entry.failures += 1;
                entry.consecutive_failures += 1;
                entry.last_error = error;
                if entry.consecutive_failures >= FAILURES_BEFORE_DEGRADED
                    && entry.degraded_since.is_none()
                {
                    entry.degraded_since = Some(Instant::now());
                }
            }
            Some(StopReason::Interrupted) | Some(StopReason::Limit) => {
                // Neither is the model's fault, and neither is evidence that
                // it is working either. Left as it was.
            }
            _ => {
                entry.successes += 1;
                entry.consecutive_failures = 0;
                entry.degraded_since = None;
                entry.last_error = None;
            }
        }
    }

    pub fn health(&self, adapter: &str, model: Option<&str>) -> Health {
        self.entries
            .lock()
            .unwrap()
            .get(&key(adapter, model))
            .cloned()
            .unwrap_or_default()
    }

    /// Whether to hold off on this one for now.
    pub fn is_degraded(&self, adapter: &str, model: Option<&str>) -> bool {
        self.health(adapter, model).is_degraded()
    }

    /// Everything currently considered sick, with the reason.
    pub fn degraded(&self) -> Vec<(String, Option<String>)> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, health)| health.is_degraded())
            .map(|(name, health)| (name.clone(), health.last_error.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_failure_is_noise() {
        let registry = Registry::new();
        registry.record("codex", Some("gpt-5"), Some(StopReason::Error), None);
        assert!(!registry.is_degraded("codex", Some("gpt-5")));
    }

    #[test]
    fn a_run_of_failures_takes_it_out_of_rotation() {
        let registry = Registry::new();
        for _ in 0..FAILURES_BEFORE_DEGRADED {
            registry.record(
                "codex",
                Some("gpt-5"),
                Some(StopReason::Error),
                Some("connection refused".into()),
            );
        }
        assert!(registry.is_degraded("codex", Some("gpt-5")));
        assert_eq!(
            registry.degraded(),
            vec![("codex/gpt-5".to_string(), Some("connection refused".into()))],
            "a skipped model must be able to say why it is being skipped"
        );
    }

    #[test]
    fn one_success_clears_it() {
        let registry = Registry::new();
        for _ in 0..FAILURES_BEFORE_DEGRADED {
            registry.record("codex", Some("gpt-5"), Some(StopReason::Error), None);
        }
        registry.record("codex", Some("gpt-5"), Some(StopReason::Completed), None);
        assert!(!registry.is_degraded("codex", Some("gpt-5")));
        assert_eq!(registry.health("codex", Some("gpt-5")).last_error, None);
    }

    #[test]
    fn a_user_who_keeps_interrupting_is_not_an_outage() {
        // The failure this prevents: someone changes their mind three times
        // and their model disappears from the fleet.
        let registry = Registry::new();
        for _ in 0..10 {
            registry.record("claude", Some("opus"), Some(StopReason::Interrupted), None);
        }
        assert!(!registry.is_degraded("claude", Some("opus")));
    }

    #[test]
    fn hitting_a_cap_is_not_a_fault_either() {
        let registry = Registry::new();
        for _ in 0..10 {
            registry.record("claude", Some("opus"), Some(StopReason::Limit), None);
        }
        assert!(!registry.is_degraded("claude", Some("opus")));
    }

    #[test]
    fn models_are_judged_apart() {
        // One model being down says nothing about the other, and a registry
        // that conflated them would idle a whole fleet over one outage.
        let registry = Registry::new();
        for _ in 0..FAILURES_BEFORE_DEGRADED {
            registry.record("codex", Some("gpt-5"), Some(StopReason::Error), None);
        }
        assert!(registry.is_degraded("codex", Some("gpt-5")));
        assert!(!registry.is_degraded("codex", Some("gpt-5-mini")));
        assert!(!registry.is_degraded("claude", Some("gpt-5")));
    }
}
