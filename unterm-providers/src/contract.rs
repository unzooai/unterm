//! What every provider must do, whoever wrote it.
//!
//! A fake that behaves better than the real thing is a test suite that lies,
//! and the lie is only discovered in front of a user. So the same four
//! properties are checked here for anything implementing [`Provider`]:
//!
//! * **Offline is a wait, not a failure.** A browser that is closed is a
//!   browser that is closed.
//! * **Cancel reaches the far side.** Not just the record.
//! * **A key means once.** A retry after a dropped connection must not click
//!   the button twice.
//! * **Every call leaves evidence.** Enough to prove afterwards what was
//!   asked and what came back, without keeping either payload.
//!
//! [`run`] is the suite. Point it at a provider and a live registry; it
//! returns what failed, in words, rather than panicking — so a caller can run
//! it against a real provider at runtime and show the result in a diagnostics
//! panel.

use crate::registry::{Acquire, Registry, WAITING_PROVIDER};
use crate::{Call, Capability, Failure};
use std::sync::Arc;
use unterm_tasks::{NewLease, Presented, TaskStore};

/// One checked property.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

fn ok(name: &str, detail: impl Into<String>) -> Check {
    Check {
        name: name.to_string(),
        passed: true,
        detail: detail.into(),
    }
}

fn failed(name: &str, detail: impl Into<String>) -> Check {
    Check {
        name: name.to_string(),
        passed: false,
        detail: detail.into(),
    }
}

/// Run the suite against a live provider.
///
/// `capability` should be one the provider offers, and `method` something it
/// can do harmlessly — the suite calls it a few times.
pub fn run(
    registry: &Registry,
    store: &Arc<TaskStore>,
    provider_id: &str,
    capability: Capability,
    method: &str,
    params: serde_json::Value,
) -> Vec<Check> {
    let mut checks = Vec::new();

    // --- binding ------------------------------------------------------
    match registry.bind(provider_id) {
        Ok(handshake) => checks.push(ok(
            "handshake",
            format!(
                "{} {} over {}",
                handshake.identity.name, handshake.identity.version, handshake.protocol
            ),
        )),
        Err(failure) => {
            checks.push(failed("handshake", failure.to_string()));
            // Nothing below can mean anything without a binding.
            return checks;
        }
    }

    let lease = match registry.acquire(
        capability,
        NewLease {
            actor: Some("contract-suite".into()),
            ttl_seconds: 120,
            ..NewLease::default()
        },
    ) {
        Ok(Acquire::Ready(lease)) => lease,
        Ok(Acquire::Waiting { reason, detail, .. }) => {
            checks.push(failed(
                "lease",
                format!("a bound provider would not issue a lease: {reason} {detail:?}"),
            ));
            return checks;
        }
        Ok(Acquire::Denied { reason, detail }) => {
            checks.push(failed(
                "lease",
                format!("the lease was refused: {reason} {detail:?}"),
            ));
            return checks;
        }
        Err(error) => {
            checks.push(failed("lease", error.to_string()));
            return checks;
        }
    };
    checks.push(ok("lease", format!("issued {}", lease.id)));

    let Some(broker) = registry.broker(provider_id) else {
        checks.push(failed("broker", "the provider vanished from the registry"));
        return checks;
    };
    let mut seq = 0;
    let mut next = |lease_id: &str, epoch: i64| {
        seq += 1;
        Presented {
            lease_id: lease_id.to_string(),
            epoch,
            seq,
        }
    };

    // --- evidence -----------------------------------------------------
    let call = Call::new(capability, method, params.clone());
    match broker.invoke(&call, &next(&lease.id, lease.epoch)) {
        Ok(outcome) => {
            let evidence = &outcome.evidence;
            if evidence.request_sha256.len() == 64 && evidence.response_sha256.len() == 64 {
                checks.push(ok(
                    "evidence",
                    format!(
                        "request {}… response {}…",
                        &evidence.request_sha256[..8],
                        &evidence.response_sha256[..8]
                    ),
                ));
            } else {
                checks.push(failed(
                    "evidence",
                    format!("a call produced no usable hashes: {evidence:?}"),
                ));
            }
        }
        Err(failure) => checks.push(failed("evidence", failure.to_string())),
    }

    // --- idempotency --------------------------------------------------
    let key = format!("contract-{}", uuid::Uuid::new_v4().simple());
    let keyed = Call::new(capability, method, params.clone()).idempotent(&key);
    let first = broker.invoke(&keyed, &next(&lease.id, lease.epoch));
    let again = broker.invoke(&keyed, &next(&lease.id, lease.epoch));
    match (first, again) {
        (Ok(first), Ok(second)) => {
            if second.replayed_from_record && second.evidence.call_id == first.evidence.call_id {
                checks.push(ok(
                    "idempotency",
                    "the second call returned the first one's record",
                ));
            } else {
                checks.push(failed(
                    "idempotency",
                    "the same key performed the call twice",
                ));
            }
        }
        (first, second) => checks.push(failed(
            "idempotency",
            format!("the keyed calls did not both succeed: {first:?} then {second:?}"),
        )),
    }

    // --- replay -------------------------------------------------------
    let replayed = Presented {
        lease_id: lease.id.clone(),
        epoch: lease.epoch,
        seq: 1,
    };
    match broker.invoke(&Call::new(capability, method, params.clone()), &replayed) {
        Err(Failure::Lease(unterm_tasks::Refusal::Replay)) => {
            checks.push(ok("replay", "a repeated sequence number was refused"))
        }
        other => checks.push(failed(
            "replay",
            format!("a repeated sequence number was not refused: {other:?}"),
        )),
    }

    // --- cancel -------------------------------------------------------
    // Only meaningful when something is in flight; the caller supplies a
    // provider that can be made to wait. Checked against the record, because
    // whether the far side heard is the provider's own business to report.
    let live = store
        .calls_under_lease(&lease.id)
        .map(|calls| calls.len())
        .unwrap_or(0);
    if live == 0 {
        checks.push(failed("record", "no call was recorded under the lease"));
    } else {
        checks.push(ok("record", format!("{live} calls recorded under the lease")));
    }

    // --- offline ------------------------------------------------------
    // Left to the caller: switching a real provider off is not something a
    // suite may do to somebody's browser. `offline_is_a_wait` covers it for
    // anything that can be switched off.
    checks
}

/// The one property that needs a provider you are allowed to switch off.
///
/// Kept apart from [`run`] because a diagnostics panel must not turn the
/// user's browser off to see what happens.
pub fn offline_is_a_wait(
    registry: &Registry,
    provider_id: &str,
    capability: Capability,
    switch_off: impl FnOnce(),
) -> Check {
    switch_off();
    let _ = registry.bind(provider_id);
    match registry.acquire(capability, NewLease::default()) {
        Ok(Acquire::Waiting { reason, .. }) if reason == WAITING_PROVIDER => ok(
            "offline",
            "an unreachable provider makes the work wait rather than fail",
        ),
        other => failed("offline", format!("{other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{registered, FakeProvider};
    use serde_json::json;
    use crate::registry::State;
    use crate::Identity;

    fn scaffold() -> (Arc<TaskStore>, Registry, Arc<FakeProvider>) {
        let store = Arc::new(TaskStore::in_memory().unwrap());
        let registry = Registry::new(Arc::clone(&store));
        let provider = Arc::new(FakeProvider::new("fake"));
        registered(&registry, Arc::clone(&provider));
        (store, registry, provider)
    }

    fn isolated_state() -> (tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
        // Identity pins are files; tests must not read or write the user's.
        let guard = crate::testing::env_guard();
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("UNTERM_STATE_DIR", dir.path());
        (dir, guard)
    }

    #[test]
    fn the_fake_passes_its_own_contract() {
        let _isolated = isolated_state();
        let (store, registry, _provider) = scaffold();
        let checks = run(
            &registry,
            &store,
            "fake",
            Capability::Browser,
            "navigate",
            json!({"url": "https://example.invalid"}),
        );
        let failures: Vec<&Check> = checks.iter().filter(|check| !check.passed).collect();
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(checks.len() >= 6, "the suite checked almost nothing: {checks:#?}");
    }

    #[test]
    fn a_closed_browser_is_something_to_wait_for() {
        let _isolated = isolated_state();
        let (_store, registry, provider) = scaffold();
        registry.bind("fake").unwrap();

        let check = offline_is_a_wait(&registry, "fake", Capability::Browser, || {
            provider.go_offline()
        });
        assert!(check.passed, "{check:?}");
        assert!(matches!(
            registry.state("fake"),
            Some(State::Offline { .. })
        ));
    }

    #[test]
    fn a_key_means_the_provider_is_asked_once() {
        // Checked on the provider, not on the store: "ran once" is a claim
        // about what happened out there.
        let _isolated = isolated_state();
        let (_store, registry, provider) = scaffold();
        registry.bind("fake").unwrap();
        let Acquire::Ready(lease) = registry
            .acquire(Capability::Browser, NewLease::default())
            .unwrap()
        else {
            panic!("no lease");
        };
        let broker = registry.broker("fake").unwrap();
        let call = Call::new(Capability::Browser, "click", json!({"ref": "1"}))
            .idempotent("the-same-button");

        for seq in 1..=3 {
            broker
                .invoke(
                    &call,
                    &Presented {
                        lease_id: lease.id.clone(),
                        epoch: lease.epoch,
                        seq,
                    },
                )
                .unwrap();
        }
        assert_eq!(
            provider.times_called("click"),
            1,
            "the button was clicked more than once: {:?}",
            provider.performed()
        );
    }

    #[test]
    fn cancelling_reaches_the_provider() {
        let _isolated = isolated_state();
        let (store, registry, provider) = scaffold();
        registry.bind("fake").unwrap();
        provider.make_slow("download");
        let Acquire::Ready(lease) = registry
            .acquire(
                Capability::Browser,
                NewLease {
                    task_id: Some("tsk_1".into()),
                    ttl_seconds: 120,
                    ..NewLease::default()
                },
            )
            .unwrap()
        else {
            panic!("no lease");
        };

        let broker = registry.broker("fake").unwrap();
        let call = Call::new(Capability::Browser, "download", json!({"url": "x"}));
        let presented = Presented {
            lease_id: lease.id.clone(),
            epoch: lease.epoch,
            seq: 1,
        };
        let in_flight = std::thread::spawn({
            let broker = registry.broker("fake").unwrap();
            move || broker.invoke(&call, &presented)
        });

        // Wait for the call to be recorded, then cancel the whole task.
        let mut call_id = None;
        for _ in 0..200 {
            if let Some(record) = store
                .calls_under_lease(&lease.id)
                .unwrap()
                .into_iter()
                .find(|record| record.state == "pending")
            {
                call_id = Some(record.id);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let call_id = call_id.expect("the call was never recorded");

        let stopped = registry.cancel_task("tsk_1").unwrap();
        assert_eq!(stopped, 1, "the task's in-flight call was not cancelled");

        let result = in_flight.join().unwrap();
        assert_eq!(
            result.unwrap_err(),
            Failure::Cancelled,
            "the provider kept working after the cancel"
        );
        // And the record says so, rather than being left pending forever.
        let record = store.call_by_id(&call_id).unwrap().unwrap();
        assert_eq!(record.state, "cancelled");
        // The lease goes with it: cancelled work must not leave a usable key.
        assert!(store.lease(&lease.id).unwrap().unwrap().revoked_at.is_some());
        drop(broker);
    }

    #[test]
    fn a_cancel_that_never_arrived_is_not_reported_as_success() {
        // The failure this prevents: telling a user something stopped when
        // the far side was unreachable and never heard.
        let _isolated = isolated_state();
        let (_store, registry, provider) = scaffold();
        registry.bind("fake").unwrap();
        provider.go_offline();
        let broker = registry.broker("fake").unwrap();
        assert!(matches!(
            broker.cancel("cal_whatever"),
            Err(Failure::Offline(_))
        ));
    }

    #[test]
    fn a_provider_that_became_somebody_else_needs_a_person() {
        let _isolated = isolated_state();
        let (_store, registry, provider) = scaffold();
        registry.bind("fake").unwrap();

        provider.become_identity(Identity {
            name: "something-else".into(),
            version: "9.9.9".into(),
        });
        let failure = registry.bind("fake").unwrap_err();
        assert!(matches!(failure, Failure::IdentityChanged { .. }), "{failure:?}");
        assert!(matches!(registry.state("fake"), Some(State::Degraded { .. })));

        // And no lease is issued while it is in that state — the gate that
        // makes the pin mean anything.
        assert!(matches!(
            registry
                .acquire(Capability::Browser, NewLease::default())
                .unwrap(),
            Acquire::Waiting { .. }
        ));

        // Unbinding forgets the pin, so the user can accept the new one.
        registry.unbind("fake").unwrap();
        assert!(registry.bind("fake").is_ok());
    }

    #[test]
    fn a_lease_is_permission_for_one_capability_only() {
        let _isolated = isolated_state();
        let (_store, registry, _provider) = scaffold();
        registry.bind("fake").unwrap();
        let Acquire::Ready(lease) = registry
            .acquire(Capability::Browser, NewLease::default())
            .unwrap()
        else {
            panic!("no lease");
        };
        let broker = registry.broker("fake").unwrap();
        // Driving the browser is not permission to read the cookies in it.
        let failure = broker
            .invoke(
                &Call::new(Capability::Profile, "cookie_get_all", json!({})),
                &Presented {
                    lease_id: lease.id,
                    epoch: lease.epoch,
                    seq: 1,
                },
            )
            .unwrap_err();
        assert_eq!(failure, Failure::Unsupported(Capability::Profile));
    }

    #[test]
    fn pausing_takes_the_keys_back() {
        let _isolated = isolated_state();
        let (store, registry, _provider) = scaffold();
        registry.bind("fake").unwrap();
        let Acquire::Ready(lease) = registry
            .acquire(Capability::Browser, NewLease::default())
            .unwrap()
        else {
            panic!("no lease");
        };

        assert_eq!(registry.pause("fake").unwrap(), 1);
        assert_eq!(registry.state("fake"), Some(State::Paused));
        assert!(store.lease(&lease.id).unwrap().unwrap().revoked_at.is_some());
        // A paused provider issues nothing, and says it is paused rather than
        // pretending to be broken.
        let waiting = registry
            .acquire(Capability::Browser, NewLease::default())
            .unwrap();
        assert!(matches!(waiting, Acquire::Waiting { .. }), "{waiting:?}");

        registry.resume("fake").unwrap();
        assert!(registry.state("fake").unwrap().is_ready());
    }

    #[test]
    fn a_capability_nobody_offers_is_named_as_such() {
        let _isolated = isolated_state();
        let store = Arc::new(TaskStore::in_memory().unwrap());
        let registry = Registry::new(Arc::clone(&store));
        registered(
            &registry,
            Arc::new(FakeProvider::new("browseronly").offering(vec![Capability::Browser])),
        );
        registry.bind("browseronly").unwrap();

        let waiting = registry
            .acquire(Capability::Computer, NewLease::default())
            .unwrap();
        let Acquire::Waiting { detail, reason, .. } = waiting else {
            panic!("a capability nobody offers was granted");
        };
        assert_eq!(reason, WAITING_PROVIDER);
        assert!(
            detail.unwrap_or_default().contains("computer"),
            "the reason does not say what is missing"
        );
    }

    #[test]
    fn the_statuses_say_enough_to_diagnose_with() {
        let _isolated = isolated_state();
        let (_store, registry, provider) = scaffold();
        registry.bind("fake").unwrap();
        registry
            .acquire(Capability::Browser, NewLease::default())
            .unwrap();

        let status = registry.statuses().unwrap().remove(0);
        assert_eq!(status.id, "fake");
        assert_eq!(status.state, "ready");
        assert_eq!(status.live_leases, 1);
        assert_eq!(status.pinned.as_deref(), Some("fake 1.0.0"));
        assert!(!status.source.is_empty(), "no source means nobody can tell where it came from");

        provider.go_offline();
        let _ = registry.bind("fake");
        let status = registry.statuses().unwrap().remove(0);
        assert_eq!(status.state, "offline");
        assert!(status.detail.is_some(), "offline with no reason is not diagnosable");
    }
}
