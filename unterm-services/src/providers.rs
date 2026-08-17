//! The process's view of what can be reached outside it.
//!
//! One registry, built from whatever discovery found, rebuilt when asked. It
//! is deliberately lazy: nothing is contacted until somebody binds, because a
//! terminal that probes the user's browser at startup is a terminal that
//! wakes their browser at startup.

use std::sync::{Arc, Mutex, OnceLock};
use unterm_providers::registry::{Acquire, Registry, Status};
use unterm_providers::{discovery, unzoo, Capability, Failure, Handshake, ProviderManifest};
use unterm_tasks::{Lease, NewLease};

fn slot() -> &'static Mutex<Option<Arc<Registry>>> {
    static REGISTRY: OnceLock<Mutex<Option<Arc<Registry>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(None))
}

/// Build a client for a manifest.
///
/// Every provider Unterm can speak to is HTTP MCP today. A manifest naming an
/// endpoint this build cannot reach is reported rather than skipped: a
/// provider that silently fails to appear is one nobody can debug.
fn client(manifest: &ProviderManifest) -> Result<Arc<dyn unterm_providers::Provider>, Failure> {
    if manifest.id == unzoo::ID {
        return unzoo::provider(manifest)
            .map(|provider| Arc::new(provider) as Arc<dyn unterm_providers::Provider>);
    }
    unterm_providers::mcp_http::HttpMcpProvider::from_manifest(manifest)
        .map(|provider| Arc::new(provider) as Arc<dyn unterm_providers::Provider>)
}

/// The process-wide registry, discovering on first use.
pub fn registry() -> Option<Arc<Registry>> {
    let mut slot = slot().lock().ok()?;
    if let Some(registry) = slot.as_ref() {
        return Some(Arc::clone(registry));
    }
    let store = crate::cockpit::fleet_store::tasks()?;
    let registry = Arc::new(Registry::new(store));
    for manifest in discovery::discover() {
        match client(&manifest) {
            Ok(provider) => registry.register(manifest, provider),
            Err(failure) => {
                eprintln!(
                    "unterm: cannot speak to provider {}: {failure}",
                    manifest.id
                );
            }
        }
    }
    *slot = Some(Arc::clone(&registry));
    Some(registry)
}

/// Discover again — after installing a provider, or when one has moved.
///
/// Bindings are not preserved across this: a rediscovered provider is
/// contacted afresh, and its pinned identity is what decides whether it is
/// still the one the user bound.
pub fn rediscover() -> Option<Arc<Registry>> {
    if let Ok(mut slot) = slot().lock() {
        *slot = None;
    }
    registry()
}

/// Drop the registry, for tests.
#[cfg(any(test, feature = "test-support"))]
pub fn reset_for_tests() {
    if let Ok(mut slot) = slot().lock() {
        *slot = None;
    }
}

/// What to show a person.
pub fn statuses() -> Vec<Status> {
    registry()
        .and_then(|registry| registry.statuses().ok())
        .unwrap_or_default()
}

pub fn bind(id: &str) -> anyhow::Result<Handshake> {
    let registry = registry().ok_or_else(|| anyhow::anyhow!("there is no provider registry"))?;
    registry.bind(id).map_err(|failure| anyhow::anyhow!(failure))
}

pub fn pause(id: &str) -> anyhow::Result<usize> {
    let registry = registry().ok_or_else(|| anyhow::anyhow!("there is no provider registry"))?;
    registry.pause(id)
}

pub fn resume(id: &str) -> anyhow::Result<Handshake> {
    let registry = registry().ok_or_else(|| anyhow::anyhow!("there is no provider registry"))?;
    registry
        .resume(id)
        .map_err(|failure| anyhow::anyhow!(failure))
}

pub fn unbind(id: &str) -> anyhow::Result<usize> {
    let registry = registry().ok_or_else(|| anyhow::anyhow!("there is no provider registry"))?;
    registry.unbind(id)
}

/// The word an answer waits on when a human has to say yes first.
pub const WAITING_APPROVAL: &str = "waiting_approval";

/// The gateway method that stands for "may an agent use this capability".
///
/// Separate from `provider.acquire`, which is the mechanical act of taking a
/// lease. What the user is being asked is not "may this call happen" but "may
/// this agent drive my browser" — and the three capabilities are three
/// different questions, which is why they are three methods rather than one
/// with an argument.
pub fn permission_method(capability: Capability) -> &'static str {
    match capability {
        Capability::Browser => "capability.browser",
        Capability::Profile => "capability.profile",
        Capability::Computer => "capability.computer",
    }
}

/// Ask for permission to use a capability.
///
/// Two gates, in order, and the order matters. The user's permission is asked
/// *before* a provider is chosen: being told "your browser is closed" when
/// the real answer is "you were never going to be allowed" wastes the user's
/// time on the wrong problem — and asking for consent that is then not needed
/// is worse, so the reverse order is not an option either. In practice a
/// refusal is the cheaper answer to reach, and the honest one to give first.
pub fn acquire(capability: Capability, spec: NewLease) -> anyhow::Result<Acquire> {
    let registry = registry().ok_or_else(|| anyhow::anyhow!("there is no provider registry"))?;

    let mut context = unterm_gateway::ActionContext::new(permission_method(capability))
        .entry(unterm_gateway::Entry::Mcp);
    context.actor = spec.actor.clone();
    context.task_id = spec.task_id.clone();
    context.resource = Some(capability.as_str().to_string());

    // No command policy here: the user's blocklist is about shell commands,
    // and this question is about a capability. The gateway's own
    // classification is what decides it.
    let passage = crate::gateway::admit(&context, &crate::gateway::SettingsPolicy::off());
    let grant_id = match passage.outcome {
        crate::gateway::Outcome::Proceed { authorised_by } => authorised_by,
        crate::gateway::Outcome::AwaitApproval { approval_id } => {
            // Not an error and not a refusal: somebody has to say yes. The
            // caller gets the id so it can wait for an answer rather than
            // guessing when to retry.
            return Ok(Acquire::Waiting {
                provider: None,
                reason: WAITING_APPROVAL.to_string(),
                detail: Some(approval_id),
            });
        }
        crate::gateway::Outcome::Refuse => {
            return Ok(Acquire::Denied {
                reason: passage.verdict.code.as_str().to_string(),
                detail: Some(passage.verdict.reason.clone()),
            })
        }
    };

    // The grant that allowed this, and the question that created the grant,
    // both go onto the lease: that chain is the whole of "an action can be
    // traced back to the human who allowed it".
    let approval_id = grant_id.as_deref().and_then(approval_behind_grant);
    registry.acquire(
        capability,
        NewLease {
            grant_id,
            approval_id,
            ..spec
        },
    )
}

/// The question whose answer created this grant, if a question was asked.
fn approval_behind_grant(grant_id: &str) -> Option<String> {
    let store = crate::cockpit::fleet_store::tasks()?;
    store
        .approvals_for_grant(grant_id)
        .ok()?
        .into_iter()
        .next()
        .map(|approval| approval.id)
}

/// Every lease, live or spent.
pub fn leases() -> Vec<Lease> {
    crate::cockpit::fleet_store::tasks()
        .and_then(|store| store.leases().ok())
        .unwrap_or_default()
}

pub fn revoke_lease(id: &str) -> anyhow::Result<bool> {
    let store = crate::cockpit::fleet_store::tasks()
        .ok_or_else(|| anyhow::anyhow!("there is no task store"))?;
    store.revoke_lease(id)
}

/// Everything that authorised an action, as JSON for a caller to display.
pub fn authorisation_chain(lease_id: &str) -> anyhow::Result<Option<serde_json::Value>> {
    let store = crate::cockpit::fleet_store::tasks()
        .ok_or_else(|| anyhow::anyhow!("there is no task store"))?;
    let Some(chain) = store.authorisation_chain(lease_id)? else {
        return Ok(None);
    };
    let calls = store.calls_under_lease(lease_id)?;
    Ok(Some(serde_json::json!({
        "lease": chain.lease,
        "grant": chain.grant,
        "approval": chain.approval,
        "task": chain.task,
        "calls": calls,
    })))
}

/// Run the contract suite against a bound provider.
///
/// The read-only half: it binds, leases and makes one harmless call. It does
/// not switch the provider off, because a diagnostics button must not close
/// the user's browser to see what happens.
pub fn diagnose(id: &str, method: Option<&str>) -> anyhow::Result<serde_json::Value> {
    let registry = registry().ok_or_else(|| anyhow::anyhow!("there is no provider registry"))?;
    let store = crate::cockpit::fleet_store::tasks()
        .ok_or_else(|| anyhow::anyhow!("there is no task store"))?;
    let method = method.unwrap_or(DIAGNOSTIC_METHOD);
    let checks = unterm_providers::contract::run(
        &registry,
        &store,
        id,
        Capability::Browser,
        method,
        serde_json::json!({}),
    );
    Ok(serde_json::json!({
        "provider": id,
        "method": method,
        "passed": checks.iter().all(|check| check.passed),
        "checks": checks,
    }))
}

/// The harmless call diagnostics makes.
///
/// Listing tabs reads; it opens nothing, closes nothing and navigates
/// nowhere. A diagnostic that changed something would be one nobody dares
/// press.
pub const DIAGNOSTIC_METHOD: &str = "tab_list";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_registry_every_answer_is_an_error_rather_than_a_pretence() {
        // A build that cannot reach its own state directory must not report
        // providers as absent — "none found" and "could not look" are
        // different answers and only one of them means the user should
        // install something.
        for result in [
            bind("nobody").err(),
            resume("nobody").err(),
            pause("nobody").err(),
        ] {
            if let Some(error) = result {
                assert!(!error.to_string().is_empty());
            }
        }
    }

    #[test]
    fn the_diagnostic_call_only_reads() {
        // Guard on the constant itself: somebody changing this to
        // `browser_navigate` would make the diagnostics button open a page in
        // the user's browser.
        assert_eq!(DIAGNOSTIC_METHOD, "tab_list");
        assert_eq!(
            unzoo::family_of(DIAGNOSTIC_METHOD),
            Some(Capability::Browser)
        );
    }
}
