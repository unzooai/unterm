//! The Unzoo binding against the real service.
//!
//! Ignored by default: it needs Unzoo Browser to be installed and running,
//! which a build machine has no business assuming. Run it on a machine that
//! has one:
//!
//! ```text
//! cargo test -p unterm-providers --test unzoo_live -- --ignored --nocapture
//! ```
//!
//! What it proves that the fake cannot: that discovery reads a real port
//! file, that a real MCP handshake settles on a version this build speaks,
//! that the tool surface actually sorts into the three capabilities, and that
//! a lease-covered call reaches a real browser and comes back with evidence.

use std::sync::Arc;
use unterm_providers::registry::{Acquire, Registry};
use unterm_providers::{discovery, unzoo, Call, Capability};
use unterm_tasks::{NewLease, Presented, TaskStore};

fn state_dir() -> tempfile::TempDir {
    // Never the user's own pins: this test binds and unbinds.
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("UNTERM_STATE_DIR", dir.path());
    dir
}

#[test]
#[ignore = "needs Unzoo Browser running on this machine"]
fn unzoo_is_discovered_bound_and_usable() {
    let _dir = state_dir();
    let manifest = discovery::find(unzoo::ID)
        .expect("Unzoo was not discovered — is the browser running?");
    assert_eq!(manifest.source, "unzoo:rest-port");
    println!("discovered at {:?} via {}", manifest.endpoint, manifest.source);

    let store = Arc::new(TaskStore::in_memory().unwrap());
    let registry = Registry::new(Arc::clone(&store));
    let provider = unzoo::provider(&manifest).expect("an HTTP endpoint");
    registry.register(manifest.clone(), Arc::new(provider));

    // --- handshake ----------------------------------------------------
    let handshake = registry.bind(unzoo::ID).expect("the handshake failed");
    println!(
        "bound to {} {} over {}",
        handshake.identity.name, handshake.identity.version, handshake.protocol
    );
    assert!(
        discovery::PROTOCOLS.contains(&handshake.protocol.as_str()),
        "settled on a protocol this build does not speak: {}",
        handshake.protocol
    );
    assert!(
        handshake.capabilities.contains(&Capability::Browser),
        "a browser that offers no browser capability: {:?}",
        handshake.capabilities
    );
    // The real surface has all three families; if it ever loses one, the
    // capability model needs revisiting rather than quietly narrowing.
    for capability in Capability::ALL {
        assert!(
            handshake.capabilities.contains(&capability),
            "the live service no longer offers {}",
            capability.as_str()
        );
    }

    // --- identity pinning ---------------------------------------------
    assert_eq!(
        discovery::pinned(unzoo::ID).as_ref(),
        Some(&handshake.identity),
        "binding did not pin who answered"
    );
    // Binding again must agree with itself.
    let again = registry.bind(unzoo::ID).expect("the second handshake failed");
    assert_eq!(again.identity, handshake.identity);

    // --- a real call, under a real lease ------------------------------
    let Acquire::Ready(lease) = registry
        .acquire(
            Capability::Browser,
            NewLease {
                actor: Some("unzoo-live-test".into()),
                ttl_seconds: 120,
                ..NewLease::default()
            },
        )
        .unwrap()
    else {
        panic!("a bound provider issued no lease");
    };
    let broker = registry.broker(unzoo::ID).unwrap();

    // Listing tabs reads; it opens nothing and closes nothing, which is what
    // a test is allowed to do to somebody's browser.
    let outcome = broker
        .invoke(
            &Call::new(Capability::Browser, "tab_list", serde_json::json!({})),
            &Presented {
                lease_id: lease.id.clone(),
                epoch: lease.epoch,
                seq: 1,
            },
        )
        .expect("the call failed");
    assert_eq!(outcome.evidence.request_sha256.len(), 64);
    assert_eq!(outcome.evidence.response_sha256.len(), 64);
    assert!(!outcome.replayed_from_record);
    println!("tab_list evidence: {:?}", outcome.evidence);

    // --- the boundary that matters ------------------------------------
    // A browser lease must not reach the cookie jar, even though the same
    // provider offers both.
    let refused = broker.invoke(
        &Call::new(Capability::Browser, "cookie_get_all", serde_json::json!({})),
        &Presented {
            lease_id: lease.id.clone(),
            epoch: lease.epoch,
            seq: 2,
        },
    );
    assert!(
        matches!(
            refused,
            Err(unterm_providers::Failure::Unsupported(Capability::Profile))
        ),
        "a browser lease reached the profile capability: {refused:?}"
    );

    // --- replay -------------------------------------------------------
    let replayed = broker.invoke(
        &Call::new(Capability::Browser, "tab_list", serde_json::json!({})),
        &Presented {
            lease_id: lease.id.clone(),
            epoch: lease.epoch,
            seq: 1,
        },
    );
    assert!(
        matches!(
            replayed,
            Err(unterm_providers::Failure::Lease(
                unterm_tasks::Refusal::Replay
            ))
        ),
        "a recorded exchange was accepted again: {replayed:?}"
    );

    // --- unbind -------------------------------------------------------
    registry.unbind(unzoo::ID).unwrap();
    assert!(store.lease(&lease.id).unwrap().unwrap().revoked_at.is_some());
    assert_eq!(
        discovery::pinned(unzoo::ID),
        None,
        "unbinding left the pin behind"
    );
}

#[test]
#[ignore = "needs Unzoo Browser running on this machine"]
fn the_live_service_passes_the_contract_suite() {
    let _dir = state_dir();
    let manifest = discovery::find(unzoo::ID).expect("Unzoo was not discovered");
    let store = Arc::new(TaskStore::in_memory().unwrap());
    let registry = Registry::new(Arc::clone(&store));
    registry.register(
        manifest.clone(),
        Arc::new(unzoo::provider(&manifest).unwrap()),
    );

    let checks = unterm_providers::contract::run(
        &registry,
        &store,
        unzoo::ID,
        Capability::Browser,
        "tab_list",
        serde_json::json!({}),
    );
    for check in &checks {
        println!(
            "{} {}: {}",
            if check.passed { "ok  " } else { "FAIL" },
            check.name,
            check.detail
        );
    }
    let failures: Vec<&unterm_providers::contract::Check> =
        checks.iter().filter(|check| !check.passed).collect();
    assert!(failures.is_empty(), "{failures:#?}");
}
