//! Which providers are ready, and what may be done with them.
//!
//! Two things live here, and they are separate on purpose.
//!
//! The **registry** knows the state of each provider and issues leases. Its
//! most important answer is the one it gives when nothing is ready: a task
//! that needs a browser and has no browser is *waiting*, not failed. A user
//! who closed their browser has not made a mistake, and a system that turns
//! that into a failed task teaches them to distrust it.
//!
//! The **broker** is the only path from a lease to an actual call. Permission,
//! deduplication and evidence are all here rather than in the providers,
//! because rules implemented once per provider are rules that differ per
//! provider.

use crate::{
    Call, Capability, Evidence, Failure, Handshake, Identity, Outcome, Provider, ProviderManifest,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use unterm_tasks::{CallSlot, Lease, NewLease, Presented, TaskStore};

/// The word a task waits on when the thing it needs is not there.
///
/// One string, used everywhere, because a caller matching on the reason must
/// not have to guess whether this build says "offline" or "not_ready".
pub const WAITING_PROVIDER: &str = "waiting_provider";

/// Where a provider stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Found, never contacted.
    Discovered,
    /// Contacted, identity matches, protocol agreed.
    Ready {
        protocol: String,
        identity: Identity,
    },
    /// Reachable but not usable — usually an identity that changed, which
    /// needs a person rather than a retry.
    Degraded { reason: String },
    /// Not reachable.
    Offline { reason: String },
    /// The user switched it off. Distinct from offline: nothing is wrong, and
    /// nothing should retry, and the difference is what the user is told.
    Paused,
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::Discovered => "discovered",
            State::Ready { .. } => "ready",
            State::Degraded { .. } => "degraded",
            State::Offline { .. } => "offline",
            State::Paused => "paused",
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, State::Ready { .. })
    }

    /// What to tell somebody about this state, when there is anything to add.
    pub fn detail(&self) -> Option<String> {
        match self {
            State::Degraded { reason } | State::Offline { reason } => Some(reason.clone()),
            State::Ready { protocol, identity } => Some(format!(
                "{} {} over {protocol}",
                identity.name, identity.version
            )),
            _ => None,
        }
    }
}

/// A provider as the outside sees it.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Status {
    pub id: String,
    pub name: String,
    pub state: String,
    pub detail: Option<String>,
    pub capabilities: Vec<String>,
    /// How it was found — the answer to "why is it pointing there".
    pub source: String,
    pub endpoint: String,
    pub pinned: Option<String>,
    pub live_leases: usize,
}

/// What asking for a capability got you.
#[derive(Clone, Debug, PartialEq)]
pub enum Acquire {
    /// Go ahead, under this lease.
    Ready(Lease),
    /// Nothing can do this right now. `reason` is [`WAITING_PROVIDER`] when
    /// the provider simply is not there, which a task can wait on.
    Waiting {
        provider: Option<String>,
        reason: String,
        detail: Option<String>,
    },
    /// No, and waiting will not change it. Kept apart from `Waiting` because
    /// a caller that retries a refusal forever is a caller nobody can stop.
    Denied {
        reason: String,
        detail: Option<String>,
    },
}

struct Entry {
    manifest: ProviderManifest,
    provider: Arc<dyn Provider>,
    state: State,
}

/// Everything Unterm can reach, and what it is allowed to do with it.
pub struct Registry {
    store: Arc<TaskStore>,
    entries: Mutex<HashMap<String, Entry>>,
}

/// How long a freshly issued lease lasts.
///
/// Short enough that a forgotten one stops mattering, long enough that a
/// browsing task is not renewing every other step.
pub const DEFAULT_LEASE_SECONDS: i64 = 900;

impl Registry {
    pub fn new(store: Arc<TaskStore>) -> Self {
        Self {
            store,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Put a provider in the registry. Does not contact it.
    pub fn register(&self, manifest: ProviderManifest, provider: Arc<dyn Provider>) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(
            manifest.id.clone(),
            Entry {
                manifest,
                provider,
                state: State::Discovered,
            },
        );
    }

    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entries.lock().unwrap().keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn state(&self, id: &str) -> Option<State> {
        self.entries
            .lock()
            .unwrap()
            .get(id)
            .map(|entry| entry.state.clone())
    }

    fn provider(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.entries
            .lock()
            .unwrap()
            .get(id)
            .map(|entry| Arc::clone(&entry.provider))
    }

    /// Contact a provider and decide whether it may be used.
    ///
    /// The first successful handshake pins who answered. Every one after that
    /// compares — a different name or version leaves the provider `Degraded`
    /// and needing a person, because "something on this port answered" is not
    /// the same as "my browser answered", and the whole value of binding is
    /// the difference.
    pub fn bind(&self, id: &str) -> Result<Handshake, Failure> {
        let provider = self
            .provider(id)
            .ok_or_else(|| Failure::Offline(format!("{id} is not registered")))?;

        let result = provider.handshake();
        let mut entries = self.entries.lock().unwrap();
        let Some(entry) = entries.get_mut(id) else {
            return Err(Failure::Offline(format!("{id} is not registered")));
        };

        match result {
            Ok(handshake) => {
                let pinned = entry
                    .manifest
                    .pinned
                    .clone()
                    .or_else(|| crate::discovery::pinned(id));
                if let Some(expected) = pinned {
                    if expected != handshake.identity {
                        let failure = Failure::IdentityChanged {
                            expected,
                            found: handshake.identity.clone(),
                        };
                        entry.state = State::Degraded {
                            reason: failure.to_string(),
                        };
                        return Err(failure);
                    }
                } else {
                    // First contact. Trust it and write it down; from here on
                    // a change is visible.
                    let _ = crate::discovery::pin(id, &handshake.identity);
                    entry.manifest.pinned = Some(handshake.identity.clone());
                }
                entry.manifest.capabilities = handshake.capabilities.clone();
                entry.state = State::Ready {
                    protocol: handshake.protocol.clone(),
                    identity: handshake.identity.clone(),
                };
                Ok(handshake)
            }
            Err(failure) => {
                entry.state = match &failure {
                    Failure::Offline(reason) => State::Offline {
                        reason: reason.clone(),
                    },
                    other => State::Degraded {
                        reason: other.to_string(),
                    },
                };
                Err(failure)
            }
        }
    }

    /// Stop using a provider, without forgetting it.
    ///
    /// Leases go too. Pausing while leaving keys outstanding would mean the
    /// provider is off and still usable, which is the sort of half-state
    /// nobody can reason about afterwards.
    pub fn pause(&self, id: &str) -> anyhow::Result<usize> {
        let revoked = self.store.revoke_provider_leases(id)?;
        if let Some(entry) = self.entries.lock().unwrap().get_mut(id) {
            entry.state = State::Paused;
        }
        Ok(revoked)
    }

    /// Undo a pause. The provider still has to be bound again — a pause is
    /// not a promise that it is still there.
    pub fn resume(&self, id: &str) -> Result<Handshake, Failure> {
        if let Some(entry) = self.entries.lock().unwrap().get_mut(id) {
            if entry.state == State::Paused {
                entry.state = State::Discovered;
            }
        }
        self.bind(id)
    }

    /// Forget a binding entirely: keys back, identity forgotten.
    ///
    /// The next bind starts from nothing known, rather than comparing against
    /// an identity the user has just rejected.
    pub fn unbind(&self, id: &str) -> anyhow::Result<usize> {
        let revoked = self.store.revoke_provider_leases(id)?;
        let _ = crate::discovery::unpin(id);
        if let Some(entry) = self.entries.lock().unwrap().get_mut(id) {
            entry.manifest.pinned = None;
            entry.state = State::Discovered;
        }
        Ok(revoked)
    }

    /// Ask for permission to use a capability.
    pub fn acquire(&self, capability: Capability, spec: NewLease) -> anyhow::Result<Acquire> {
        let candidate = {
            let entries = self.entries.lock().unwrap();
            let mut ready: Vec<(&String, &Entry)> = entries
                .iter()
                .filter(|(_, entry)| {
                    entry.state.is_ready() && entry.manifest.offers(capability)
                })
                .collect();
            ready.sort_by(|a, b| a.0.cmp(b.0));
            ready.first().map(|(id, _)| (*id).clone())
        };

        let Some(id) = candidate else {
            // Why there is nothing, in as much detail as we have — a user
            // told only "waiting" cannot tell a closed browser from a broken
            // binding.
            let entries = self.entries.lock().unwrap();
            let offering: Vec<&Entry> = entries
                .values()
                .filter(|entry| entry.manifest.offers(capability))
                .collect();
            let (provider, detail) = match offering.first() {
                Some(entry) => (
                    Some(entry.manifest.id.clone()),
                    entry
                        .state
                        .detail()
                        .or_else(|| Some(entry.state.as_str().to_string())),
                ),
                None => (
                    None,
                    Some(format!(
                        "nothing registered offers {}",
                        capability.as_str()
                    )),
                ),
            };
            return Ok(Acquire::Waiting {
                provider,
                reason: WAITING_PROVIDER.to_string(),
                detail,
            });
        };

        let lease = self.store.issue_lease(NewLease {
            provider: id,
            capability: capability.as_str().to_string(),
            ttl_seconds: if spec.ttl_seconds > 0 {
                spec.ttl_seconds
            } else {
                DEFAULT_LEASE_SECONDS
            },
            ..spec
        })?;
        Ok(Acquire::Ready(lease))
    }

    /// A broker for one provider, if it is registered.
    pub fn broker(&self, id: &str) -> Option<Broker> {
        self.provider(id).map(|provider| Broker {
            store: Arc::clone(&self.store),
            provider,
        })
    }

    /// Stop everything being done for a task, everywhere.
    ///
    /// Cancelling a task has to reach the far side. A cancel that only
    /// updates rows leaves the browser still loading the page while the user
    /// is told it stopped — and leaves a lease that still works.
    pub fn cancel_task(&self, task_id: &str) -> anyhow::Result<usize> {
        let mut stopped = 0;
        for lease in self.store.leases()? {
            if lease.task_id.as_deref() != Some(task_id) || lease.revoked_at.is_some() {
                continue;
            }
            for call in self.store.calls_under_lease(&lease.id)? {
                if call.state != "pending" {
                    continue;
                }
                if let Some(broker) = self.broker(&lease.provider) {
                    let _ = broker.cancel(&call.id);
                }
                stopped += 1;
            }
            self.store.revoke_lease(&lease.id)?;
        }
        Ok(stopped)
    }

    /// What to show a person who asks how things stand.
    pub fn statuses(&self) -> anyhow::Result<Vec<Status>> {
        let leases = self.store.leases()?;
        let stamp = chrono::Utc::now().to_rfc3339();
        let entries = self.entries.lock().unwrap();
        let mut statuses: Vec<Status> = entries
            .values()
            .map(|entry| Status {
                id: entry.manifest.id.clone(),
                name: entry.manifest.name.clone(),
                state: entry.state.as_str().to_string(),
                detail: entry.state.detail(),
                capabilities: entry
                    .manifest
                    .capabilities
                    .iter()
                    .map(|capability| capability.as_str().to_string())
                    .collect(),
                source: entry.manifest.source.clone(),
                endpoint: match &entry.manifest.endpoint {
                    crate::Endpoint::Http { url } => url.clone(),
                    crate::Endpoint::Stdio { program, .. } => program.clone(),
                    crate::Endpoint::Unix { path } => path.clone(),
                },
                pinned: entry
                    .manifest
                    .pinned
                    .as_ref()
                    .map(|identity| format!("{} {}", identity.name, identity.version)),
                live_leases: leases
                    .iter()
                    .filter(|lease| lease.provider == entry.manifest.id && lease.is_live(&stamp))
                    .count(),
            })
            .collect();
        statuses.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(statuses)
    }
}

/// The only path from a lease to a call.
pub struct Broker {
    store: Arc<TaskStore>,
    provider: Arc<dyn Provider>,
}

impl Broker {
    /// Do one thing, if the lease allows it.
    ///
    /// The order matters and is the same every time: check the lease, then
    /// deduplicate, then perform, then record. Performing before checking
    /// would make the check a report; recording before performing would
    /// invent evidence for something that had not happened.
    pub fn invoke(&self, call: &Call, presented: &Presented) -> Result<Outcome, Failure> {
        let lease = match self
            .store
            .use_lease(presented)
            .map_err(|error| Failure::Provider(error.to_string()))?
        {
            Ok(lease) => lease,
            Err(refusal) => return Err(Failure::Lease(refusal)),
        };
        if lease.capability != call.capability.as_str() {
            // A browser lease is not permission to read the user's cookies.
            return Err(Failure::Unsupported(call.capability));
        }

        let slot = self
            .store
            .begin_call(
                call.idempotency_key.as_deref(),
                &lease.provider,
                call.capability.as_str(),
                &call.method,
                Some(&lease.id),
                &call.params,
            )
            .map_err(|error| Failure::Provider(error.to_string()))?;

        let record = match slot {
            CallSlot::Settled(record) => {
                // Already done. Handing back the recorded answer is the whole
                // point of the key: a retry after a dropped connection must
                // not click the button twice.
                let value = record
                    .response
                    .as_deref()
                    .and_then(|text| serde_json::from_str(text).ok())
                    .unwrap_or(serde_json::Value::Null);
                return Ok(Outcome {
                    evidence: Evidence {
                        call_id: record.id.clone(),
                        provider: record.provider.clone(),
                        capability: call.capability,
                        method: record.method.clone(),
                        lease_id: record.lease_id.clone(),
                        request_sha256: record.request_sha256.clone(),
                        response_sha256: record.response_sha256.clone().unwrap_or_default(),
                        at: record.finished_at.clone().unwrap_or(record.created_at),
                    },
                    value,
                    replayed_from_record: true,
                });
            }
            CallSlot::InFlight(record) => {
                return Err(Failure::Provider(format!(
                    "an identical call ({}) is already in flight",
                    record.id
                )))
            }
            CallSlot::Fresh(record) => record,
        };

        // The provider sees a call carrying the record's id, so cancelling by
        // that id reaches the right thing on the far side.
        let mut outgoing = call.clone();
        outgoing.id = record.id.clone();

        match self.provider.call(&outgoing) {
            Ok(value) => {
                let finished = self
                    .store
                    .finish_call(&record.id, "succeeded", Some(&value), None)
                    .map_err(|error| Failure::Provider(error.to_string()))?
                    .unwrap_or(record);
                Ok(Outcome {
                    evidence: Evidence {
                        call_id: finished.id,
                        provider: finished.provider,
                        capability: call.capability,
                        method: finished.method,
                        lease_id: finished.lease_id,
                        request_sha256: finished.request_sha256,
                        response_sha256: finished.response_sha256.unwrap_or_default(),
                        at: finished.finished_at.unwrap_or(finished.created_at),
                    },
                    value,
                    replayed_from_record: false,
                })
            }
            Err(failure) => {
                let state = if failure == Failure::Cancelled {
                    "cancelled"
                } else {
                    "failed"
                };
                let _ = self
                    .store
                    .finish_call(&record.id, state, None, Some(&failure.to_string()));
                Err(failure)
            }
        }
    }

    /// Stop a call, on the far side and in the record.
    ///
    /// The provider is told first. If that fails the record is still closed —
    /// but the error comes back, because a caller who is told "cancelled"
    /// when the far side never heard is being told something false.
    pub fn cancel(&self, call_id: &str) -> Result<(), Failure> {
        let reached = self.provider.cancel(call_id);
        let _ = self
            .store
            .finish_call(call_id, "cancelled", None, Some("cancelled"));
        reached
    }
}
