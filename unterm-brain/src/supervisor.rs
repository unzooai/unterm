//! Running a brain *for* a task, so that killing either one loses nothing.
//!
//! A brain by itself is a process that prints things. This ties it to a step
//! in the durable store: the step is claimed before the process starts,
//! renewed while it lives, and closed with a verdict when it stops. What the
//! brain did and cost is written onto the step, so a restarted Unterm can
//! still say what happened.
//!
//! The recovery story rests on an absence rather than a handler. Nothing runs
//! at the moment of a crash — no atexit, no signal handler, no flush. The
//! lease simply stops being renewed, and the next `reconcile()` finds a claim
//! nobody is holding and returns the step as `Interrupted`. That is why a
//! `kill -9` of the whole of Unterm loses no task: there was never a step
//! that had to execute for the truth to be recorded.

use crate::runtime::{self, Running, Snapshot, Spec};
use crate::{BrainAdapter, BrainEvent, StopReason};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;
use unterm_tasks::{Claim, RunId, State, StepId, TaskStore};

/// How long a claim is good for, and how often it is renewed.
///
/// The renewal interval is a third of the lease so that one missed beat — a
/// busy machine, a slow disk — does not hand the step to somebody else while
/// the brain is still working on it.
pub const LEASE_SECONDS: i64 = 30;

fn renew_every(lease_seconds: i64) -> Duration {
    Duration::from_secs((lease_seconds.max(1) as u64 / 3).max(1))
}

/// Keeps a claim alive while the work is happening.
///
/// Stops on drop, which is the useful property: every way of leaving the
/// function — return, `?`, panic — stops the renewals, and a claim that stops
/// being renewed is a claim reconciliation can safely take back.
pub struct Lease {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Lease {
    pub fn hold(store: Arc<TaskStore>, step: StepId, worker: String, lease_seconds: i64) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            let interval = renew_every(lease_seconds);
            while !flag.load(Ordering::SeqCst) {
                // Wait against a deadline rather than by counting slices: two
                // hundred sleeps of 50ms overshoot by enough to miss a beat,
                // and a missed beat is a claim somebody else can take.
                let next = std::time::Instant::now() + interval;
                while std::time::Instant::now() < next {
                    if flag.load(Ordering::SeqCst) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                if flag.load(Ordering::SeqCst) {
                    return;
                }
                if store
                    .heartbeat_step(&step, &worker, lease_seconds)
                    .unwrap_or(false)
                    .eq(&false)
                {
                    // The claim is gone — reconciled away, or cancelled. Stop
                    // renewing something that is no longer ours rather than
                    // fighting whoever took it.
                    return;
                }
            }
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// A brain running against a claimed step.
pub struct Supervised {
    pub step: StepId,
    pub running: Running,
    lease: Lease,
    store: Arc<TaskStore>,
    version: i64,
}

/// Claim a step and start a brain on it.
///
/// The claim comes first. Starting the process and then claiming would leave
/// a window where a model is spending tokens on work the store thinks nobody
/// is doing — and if the claim were then denied, no way to take it back.
pub fn start(
    store: Arc<TaskStore>,
    run: &RunId,
    kind: &str,
    idempotency_key: Option<&str>,
    worker: &str,
    spec: &Spec,
    adapter: Box<dyn BrainAdapter>,
) -> Result<Supervised> {
    start_with_lease(
        store,
        run,
        kind,
        idempotency_key,
        worker,
        spec,
        adapter,
        LEASE_SECONDS,
    )
}

/// As [`start`], with a lease length of your own.
///
/// Shorter leases return abandoned work sooner and cost more renewals; the
/// default suits an agent turn. Callers running something much shorter than a
/// turn — or a test — are better served choosing.
#[allow(clippy::too_many_arguments)]
pub fn start_with_lease(
    store: Arc<TaskStore>,
    run: &RunId,
    kind: &str,
    idempotency_key: Option<&str>,
    worker: &str,
    spec: &Spec,
    adapter: Box<dyn BrainAdapter>,
    lease_seconds: i64,
) -> Result<Supervised> {
    let request = store.request_step_with_detail(
        run,
        kind,
        idempotency_key,
        serde_json::json!({ "adapter": adapter.id() }),
    )?;
    let step = request.step().clone();
    if !request.is_new() {
        // An idempotency key that has been used before means this work has
        // already been started once. Refusing is the whole point of the key:
        // a retried request must not spend a second set of tokens.
        anyhow::bail!(
            "step {} already exists for this key, in state {}",
            step.id,
            step.state.as_str()
        );
    }

    let claimed = match store.claim_step(&step.id, worker, lease_seconds)? {
        Claim::Granted(step) => step,
        Claim::Denied { held_by, state } => {
            anyhow::bail!(
                "step {} is not ours to run: held by {:?}, state {}",
                step.id,
                held_by,
                state.as_str()
            )
        }
    };

    let lease = Lease::hold(
        Arc::clone(&store),
        claimed.id.clone(),
        worker.to_string(),
        lease_seconds,
    );
    let running = match runtime::spawn(spec, adapter) {
        Ok(running) => running,
        Err(error) => {
            // The brain never started. Close the step rather than leaving a
            // claim to lapse: a failure we know about should not have to wait
            // out a lease before anyone can see it.
            drop(lease);
            let _ = store.set_step_detail(
                &claimed.id,
                serde_json::json!({ "error": error.to_string() }),
                State::Failed,
            );
            return Err(error);
        }
    };

    Ok(Supervised {
        step: claimed.id,
        running,
        lease,
        store,
        version: claimed.version,
    })
}

impl Supervised {
    /// The event stream. Taken once.
    pub fn events(&mut self) -> Receiver<BrainEvent> {
        self.running.events()
    }

    /// Stop the brain, and say the step was stopped rather than that it failed.
    pub fn interrupt(&self, grace: Duration) -> Result<()> {
        self.running.interrupt(grace)
    }

    /// Wait for the brain, close the step, and write down what it did.
    ///
    /// The verdict distinguishes three endings a reader has to be able to
    /// tell apart: it finished, somebody stopped it, or it broke. Only the
    /// last is a fault; a run that was interrupted can be resumed, and one
    /// that hit a cap can be continued.
    pub fn finish(self) -> Result<(Snapshot, State)> {
        let Supervised {
            step,
            running,
            lease,
            store,
            version,
        } = self;
        let snapshot = running.wait()?;
        // Renewals stop before the verdict is written, so nothing can extend
        // a claim on a step that is already closed.
        drop(lease);

        let state = match snapshot.last_stop {
            Some(StopReason::Error) => State::Failed,
            Some(StopReason::Interrupted) => State::Interrupted,
            _ if snapshot.exit_code.unwrap_or(0) != 0 => State::Failed,
            _ => State::Succeeded,
        };

        // The account is written before the verdict. A crash between the two
        // leaves a step that reconciliation will reopen with its snapshot
        // intact; the other order would lose the account of a step already
        // marked done.
        let mut detail = serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null);
        if let (Some(map), Some(reason)) = (
            detail.as_object_mut(),
            Running::failure_reason(&snapshot),
        ) {
            map.insert("failure".into(), serde_json::Value::String(reason));
        }
        let _ = store.set_step_detail(&step, detail, State::Running);
        // Re-read rather than reusing the version from the claim: writing the
        // account bumped it, and a compare-and-swap against a version this
        // process itself invalidated would fail for no reason.
        let version = store
            .step(&step)?
            .map(|step| step.version)
            .unwrap_or(version);

        let closed = store.finish_step(&step, state, version)?;
        Ok((snapshot, closed.state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::CodexAdapter;

    fn store() -> Arc<TaskStore> {
        Arc::new(TaskStore::in_memory().unwrap())
    }

    fn brain(script: &str) -> Spec {
        Spec::new("sh").arg("-c").arg(script)
    }

    fn a_run(store: &TaskStore) -> RunId {
        let task = store.create_task("brain", "do a thing").unwrap();
        store.start_run(&task.id).unwrap().id
    }

    const CLEAN_TURN: &str = r#"
printf '%s\n' '{"type":"turn.started","model":"gpt-5","session_id":"sess_1"}'
printf '%s\n' '{"type":"agent_message","text":"done"}'
printf '%s\n' '{"type":"turn.completed"}'
"#;

    #[test]
    fn a_finished_brain_closes_its_step_with_what_it_did() {
        let store = store();
        let run = a_run(&store);
        let mut supervised = start(
            Arc::clone(&store),
            &run,
            "turn",
            None,
            "worker-1",
            &brain(CLEAN_TURN),
            Box::new(CodexAdapter::new()),
        )
        .unwrap();
        supervised.events().into_iter().for_each(drop);
        let (snapshot, state) = supervised.finish().unwrap();

        assert_eq!(state, State::Succeeded);
        assert_eq!(snapshot.external_id.as_deref(), Some("sess_1"));

        let steps = store.steps(&run).unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].state, State::Succeeded);
        // The account survives on the step, which is what a restarted Unterm
        // reads to say what a brain that is no longer running had done.
        assert_eq!(steps[0].detail["model"], "gpt-5");
        assert_eq!(steps[0].detail["external_id"], "sess_1");
    }

    #[test]
    fn a_broken_brain_fails_its_step_and_says_why() {
        let store = store();
        let run = a_run(&store);
        let mut supervised = start(
            Arc::clone(&store),
            &run,
            "turn",
            None,
            "worker-1",
            &brain("echo 'no credentials' >&2; exit 3"),
            Box::new(CodexAdapter::new()),
        )
        .unwrap();
        supervised.events().into_iter().for_each(drop);
        let (_, state) = supervised.finish().unwrap();

        assert_eq!(state, State::Failed);
        let steps = store.steps(&run).unwrap();
        assert_eq!(steps[0].detail["failure"], "no credentials");
    }

    #[test]
    #[cfg(unix)]
    fn an_interrupted_brain_is_not_recorded_as_a_failure() {
        // The distinction matters downstream: a failure is retried or
        // escalated, whereas work somebody stopped on purpose is not.
        let store = store();
        let run = a_run(&store);
        let mut supervised = start(
            Arc::clone(&store),
            &run,
            "turn",
            None,
            "worker-1",
            &brain("printf '%s\\n' '{\"type\":\"turn.started\",\"model\":\"m\"}'; sleep 30"),
            Box::new(CodexAdapter::new()),
        )
        .unwrap();
        let _events = supervised.events();
        std::thread::sleep(Duration::from_millis(100));
        supervised.interrupt(Duration::from_millis(200)).unwrap();
        let (snapshot, state) = supervised.finish().unwrap();

        assert!(snapshot.interrupted);
        assert_eq!(state, State::Interrupted);
    }

    #[test]
    fn the_same_key_does_not_start_a_second_brain() {
        // A retried request must not spend a second set of tokens.
        let store = store();
        let run = a_run(&store);
        let mut first = start(
            Arc::clone(&store),
            &run,
            "turn",
            Some("idem-1"),
            "worker-1",
            &brain(CLEAN_TURN),
            Box::new(CodexAdapter::new()),
        )
        .unwrap();
        first.events().into_iter().for_each(drop);
        first.finish().unwrap();

        let again = start(
            Arc::clone(&store),
            &run,
            "turn",
            Some("idem-1"),
            "worker-1",
            &brain(CLEAN_TURN),
            Box::new(CodexAdapter::new()),
        );
        assert!(again.is_err(), "the same key started a second brain");
        assert_eq!(store.steps(&run).unwrap().len(), 1);
    }

    #[test]
    fn a_brain_that_never_starts_closes_its_step_immediately() {
        // Rather than leaving a claim on a step nobody is working, to be
        // discovered a lease later.
        let store = store();
        let run = a_run(&store);
        let failed = start(
            Arc::clone(&store),
            &run,
            "turn",
            None,
            "worker-1",
            &Spec::new("/nonexistent/brain"),
            Box::new(CodexAdapter::new()),
        );
        assert!(failed.is_err());
        let steps = store.steps(&run).unwrap();
        assert_eq!(steps[0].state, State::Failed);
        assert!(steps[0].detail["error"].is_string());
    }

    #[test]
    fn the_lease_is_renewed_while_the_brain_works() {
        // Without this a long turn would have its step taken away mid-flight
        // by reconciliation.
        let store = store();
        let run = a_run(&store);
        // A three-second lease renews every second, so this asks a real
        // question in a second rather than in half a minute.
        let mut supervised = start_with_lease(
            Arc::clone(&store),
            &run,
            "turn",
            None,
            "worker-1",
            &brain("sleep 12"),
            Box::new(CodexAdapter::new()),
            3,
        )
        .unwrap();
        let before = store.step(&supervised.step).unwrap().unwrap();
        let first = before.lease_expires_at.clone();
        std::thread::sleep(renew_every(3) + Duration::from_millis(600));
        let after = store.step(&supervised.step).unwrap().unwrap();
        assert!(
            after.lease_expires_at > first,
            "the lease was not renewed: {first:?} then {:?}",
            after.lease_expires_at
        );
        assert!(store.reconcile().unwrap().is_empty(), "a live step was reclaimed");

        supervised.interrupt(Duration::from_millis(200)).unwrap();
        supervised.events().into_iter().for_each(drop);
        supervised.finish().unwrap();
    }

    #[test]
    fn killing_the_whole_runtime_does_not_lose_the_task() {
        // M4's gate. Nothing runs at the moment of death: the lease is simply
        // not renewed, and reconciliation turns the abandoned claim into a
        // verdict a reader can act on.
        let store = store();
        let run = a_run(&store);
        let step = store
            .request_step(&run, "turn", None)
            .unwrap()
            .step()
            .clone();
        let Claim::Granted(claimed) = store.claim_step(&step.id, "worker-1", 0).unwrap() else {
            panic!("the claim was denied");
        };
        assert_eq!(claimed.state, State::Running);
        // No lease is held — this is the process having been killed.
        std::thread::sleep(Duration::from_millis(1100));

        let reclaimed = store.reconcile().unwrap();
        assert_eq!(reclaimed.len(), 1);
        assert_eq!(reclaimed[0].id, claimed.id);
        assert_eq!(
            reclaimed[0].state,
            State::Interrupted,
            "a dead brain's step must end as interrupted, not sit at running forever"
        );
        // And the task itself is still there to be picked up again.
        assert_eq!(store.tasks().unwrap().len(), 1);
        assert_eq!(store.steps(&run).unwrap().len(), 1);
    }
}
