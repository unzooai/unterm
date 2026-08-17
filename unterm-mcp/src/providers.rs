//! `provider.*` — driving the things that live outside this process.
//!
//! Everything here is a thin translation onto
//! [`unterm_services::providers`]; the rules live there and in the registry,
//! so the MCP door and the settings page cannot drift apart by being written
//! twice.
//!
//! The one judgement call in this file is which methods exist at all.
//! Managing a binding — list, bind, pause, diagnose — is plainly Unterm's
//! business. Acquiring a lease and making a call is what an *agent* needs,
//! and it is here rather than left to agents talking to providers directly:
//! a browser driven around Unterm is one with no lease, no audit trail and
//! nothing to revoke.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use unterm_providers::registry::Acquire;
use unterm_providers::{Call, Capability};
use unterm_services::providers as services;
use unterm_tasks::{NewLease, Presented};

/// The `provider.*` methods, in one place so the dispatcher stays a table.
pub const METHODS: &[&str] = &[
    "provider.list",
    "provider.bind",
    "provider.pause",
    "provider.resume",
    "provider.unbind",
    "provider.diagnose",
    "provider.leases",
    "provider.acquire",
    "provider.call",
    "provider.revoke_lease",
    "provider.chain",
];

pub fn handles(method: &str) -> bool {
    METHODS.contains(&method)
}

fn id_of(params: &Value) -> Result<String> {
    params
        .get("provider")
        .or_else(|| params.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Missing 'provider'"))
}

fn capability_of(params: &Value) -> Result<Capability> {
    let raw = params
        .get("capability")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing 'capability'"))?;
    Capability::parse(raw).ok_or_else(|| {
        anyhow!(
            "{raw:?} is not a capability; expected browser, profile or computer"
        )
    })
}

pub fn dispatch(method: &str, params: &Value) -> Result<Value> {
    match method {
        "provider.list" => {
            if params
                .get("rediscover")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                services::rediscover();
            }
            Ok(json!({"providers": services::statuses()}))
        }

        "provider.bind" => {
            let id = id_of(params)?;
            let handshake = services::bind(&id)?;
            Ok(json!({
                "provider": id,
                "identity": handshake.identity,
                "protocol": handshake.protocol,
                "capabilities": handshake
                    .capabilities
                    .iter()
                    .map(|capability| capability.as_str())
                    .collect::<Vec<_>>(),
            }))
        }

        "provider.pause" => {
            let id = id_of(params)?;
            let revoked = services::pause(&id)?;
            // Saying how many keys were taken back is the difference between
            // "paused" and "paused, and three things stopped".
            Ok(json!({"provider": id, "paused": true, "leases_revoked": revoked}))
        }

        "provider.resume" => {
            let id = id_of(params)?;
            let handshake = services::resume(&id)?;
            Ok(json!({
                "provider": id,
                "identity": handshake.identity,
                "protocol": handshake.protocol,
            }))
        }

        "provider.unbind" => {
            let id = id_of(params)?;
            let revoked = services::unbind(&id)?;
            Ok(json!({"provider": id, "unbound": true, "leases_revoked": revoked}))
        }

        "provider.diagnose" => {
            let id = id_of(params)?;
            services::diagnose(&id, params.get("method").and_then(Value::as_str))
        }

        "provider.leases" => {
            let live_only = params
                .get("live_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let stamp = chrono::Utc::now().to_rfc3339();
            let leases: Vec<_> = services::leases()
                .into_iter()
                .filter(|lease| !live_only || lease.is_live(&stamp))
                .collect();
            Ok(json!({"leases": leases}))
        }

        "provider.acquire" => {
            let capability = capability_of(params)?;
            let spec = NewLease {
                capability: capability.as_str().to_string(),
                actor: params
                    .get("actor")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                task_id: params
                    .get("task_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                grant_id: params
                    .get("grant_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                ttl_seconds: params
                    .get("ttl_seconds")
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                ..NewLease::default()
            };
            match services::acquire(capability, spec)? {
                Acquire::Ready(lease) => Ok(json!({"state": "ready", "lease": lease})),
                // Not an error. A caller who asked for a browser and has none
                // is waiting, and the shape of the answer says so plainly
                // enough that a client can poll rather than give up.
                Acquire::Waiting {
                    provider,
                    reason,
                    detail,
                } => Ok(json!({
                    "state": "waiting",
                    "reason": reason,
                    "provider": provider,
                    // When the wait is for a person, this is the question's
                    // id: a caller can watch it in `approval.list` instead of
                    // retrying blind.
                    "detail": detail,
                })),
                // A refusal, said once. A caller that cannot tell this from
                // waiting will retry it forever.
                Acquire::Denied { reason, detail } => Ok(json!({
                    "state": "denied",
                    "reason": reason,
                    "detail": detail,
                })),
            }
        }

        "provider.call" => {
            let capability = capability_of(params)?;
            let lease_id = params
                .get("lease")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Missing 'lease'"))?;
            let seq = params
                .get("seq")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow!("Missing 'seq' — every use of a lease needs its own number, or a recorded exchange could be repeated"))?;
            let epoch = params.get("epoch").and_then(Value::as_i64);
            let name = params
                .get("method")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Missing 'method'"))?;

            let store = unterm_services::cockpit::fleet_store::tasks()
                .ok_or_else(|| anyhow!("there is no task store"))?;
            let lease = store
                .lease(lease_id)?
                .ok_or_else(|| anyhow!("no such lease: {lease_id}"))?;
            let registry = services::registry().ok_or_else(|| anyhow!("no provider registry"))?;
            let broker = registry
                .broker(&lease.provider)
                .ok_or_else(|| anyhow!("{} is not registered", lease.provider))?;

            let mut call = Call::new(
                capability,
                name,
                params.get("params").cloned().unwrap_or(json!({})),
            );
            if let Some(key) = params.get("idempotency_key").and_then(Value::as_str) {
                call = call.idempotent(key);
            }
            let outcome = broker
                .invoke(
                    &call,
                    &Presented {
                        lease_id: lease_id.to_string(),
                        // The lease's own epoch unless the caller pins one.
                        // A client that tracks epochs catches a renewal it
                        // did not make; one that does not still works.
                        epoch: epoch.unwrap_or(lease.epoch),
                        seq,
                    },
                )
                .map_err(|failure| anyhow!(failure))?;
            Ok(json!({
                "value": outcome.value,
                "evidence": outcome.evidence,
                "replayed_from_record": outcome.replayed_from_record,
            }))
        }

        "provider.revoke_lease" => {
            let id = params
                .get("lease")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Missing 'lease'"))?;
            Ok(json!({"lease": id, "revoked": services::revoke_lease(id)?}))
        }

        "provider.chain" => {
            let id = params
                .get("lease")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Missing 'lease'"))?;
            match services::authorisation_chain(id)? {
                Some(chain) => Ok(chain),
                None => Err(anyhow!("no such lease: {id}")),
            }
        }

        other => Err(anyhow!("provider dispatch reached {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_method_is_dispatched() {
        // The table and the match must agree, or a published method answers
        // "unknown method" in the field.
        for method in METHODS {
            assert!(handles(method), "{method} is not claimed by this module");
            let answer = dispatch(method, &json!({}));
            // With empty params most of these fail — on a *missing argument*,
            // never on "provider dispatch reached", which would mean the arm
            // does not exist.
            if let Err(error) = answer {
                assert!(
                    !error.to_string().contains("provider dispatch reached"),
                    "{method} has no arm"
                );
            }
        }
    }

    #[test]
    fn a_capability_nobody_defined_is_refused_by_name() {
        let error = dispatch("provider.acquire", &json!({"capability": "everything"}))
            .unwrap_err()
            .to_string();
        assert!(error.contains("everything"), "{error}");
        assert!(error.contains("browser"), "{error}");
    }

    #[test]
    fn using_a_lease_without_a_sequence_number_is_refused() {
        // Making `seq` optional would be the friendly choice and would quietly
        // remove replay protection for every caller who omitted it.
        let error = dispatch(
            "provider.call",
            &json!({"capability": "browser", "lease": "lse_x", "method": "tab_list"}),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("seq"), "{error}");
    }
}
