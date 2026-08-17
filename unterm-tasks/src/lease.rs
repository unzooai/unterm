//! Permission to use somebody else's capability, for a while.
//!
//! A grant records what the user agreed to. A lease is narrower and more
//! physical: *this* provider's *this* capability, until *this* moment,
//! for the work named on it. It exists because the thing on the other end is
//! a separate process that Unterm does not control — it can be restarted,
//! impersonated, or handed a stale token by something that recorded an
//! earlier exchange.
//!
//! Two fields carry the whole anti-replay story:
//!
//! * **`epoch`** goes up on every renewal. A holder presenting an old epoch
//!   is holding a lease that has since been renewed, which is not the lease
//!   they think it is.
//! * **`last_seq`** is the highest sequence number the lease has been used
//!   with. A use at or below it is a replay and is refused *before* anything
//!   is performed, because a replay that is detected afterwards has already
//!   done whatever it was replaying.

use serde::{Deserialize, Serialize};

/// A capability lease.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lease {
    pub id: String,
    pub provider: String,
    pub capability: String,
    pub actor: Option<String>,
    pub task_id: Option<String>,
    pub step_id: Option<String>,
    /// The standing permission that authorised issuing this.
    pub grant_id: Option<String>,
    /// The question a human answered, when one was asked.
    pub approval_id: Option<String>,
    pub issued_at: String,
    pub expires_at: String,
    pub renewed_at: Option<String>,
    pub revoked_at: Option<String>,
    pub epoch: i64,
    pub last_seq: i64,
}

impl Lease {
    /// Whether this lease is usable at `now`.
    pub fn is_live(&self, now: &str) -> bool {
        self.revoked_at.is_none() && self.expires_at.as_str() > now
    }
}

/// What issuing a lease needs to know.
#[derive(Clone, Debug, Default)]
pub struct NewLease {
    pub provider: String,
    pub capability: String,
    pub actor: Option<String>,
    pub task_id: Option<String>,
    pub step_id: Option<String>,
    pub grant_id: Option<String>,
    pub approval_id: Option<String>,
    /// How long it is good for. There is no "forever": a lease that never
    /// expires is a key that has to be found and revoked by hand, and the
    /// hand belongs to somebody who has forgotten it exists.
    pub ttl_seconds: i64,
}

/// Why a lease could not be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No such lease.
    Unknown,
    /// Revoked by somebody.
    Revoked,
    /// Its time ran out.
    Expired,
    /// The epoch presented is not the current one.
    StaleEpoch,
    /// This sequence number has been seen. Somebody is repeating an exchange
    /// they recorded, or a client is retrying without a fresh number — and
    /// from here those two look identical, so both are refused.
    Replay,
}

impl Refusal {
    pub fn as_str(self) -> &'static str {
        match self {
            Refusal::Unknown => "unknown_lease",
            Refusal::Revoked => "revoked",
            Refusal::Expired => "expired",
            Refusal::StaleEpoch => "stale_epoch",
            Refusal::Replay => "replay",
        }
    }
}

/// Presenting a lease: which one, at what epoch, with what sequence number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presented {
    pub lease_id: String,
    pub epoch: i64,
    pub seq: i64,
}

/// Everything that authorised one action, from the lease back to the human.
///
/// This is what "an action can be traced to its authorisation" means in
/// practice: not a log line saying it was allowed, but the actual chain of
/// records, each of which can be revoked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chain {
    pub lease: Lease,
    pub grant: Option<crate::approval::Grant>,
    pub approval: Option<crate::approval::Approval>,
    pub task: Option<crate::model::Task>,
}

/// What was asked of a provider, and what came back.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallRecord {
    pub id: String,
    pub idempotency_key: Option<String>,
    pub provider: String,
    pub capability: String,
    pub method: String,
    pub lease_id: Option<String>,
    /// pending | succeeded | failed | cancelled
    pub state: String,
    pub request_sha256: String,
    pub response_sha256: Option<String>,
    pub response: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

impl CallRecord {
    pub fn is_finished(&self) -> bool {
        self.state != "pending"
    }
}

/// What starting a call found.
#[derive(Clone, Debug, PartialEq)]
pub enum CallSlot {
    /// Nobody has asked this before. Go ahead.
    Fresh(CallRecord),
    /// This exact request has already been answered. Here is the answer; do
    /// not perform it again.
    Settled(CallRecord),
    /// Somebody is performing it right now. Neither repeating it nor
    /// pretending it is done would be true, so the caller is told which.
    InFlight(CallRecord),
}

impl CallSlot {
    pub fn record(&self) -> &CallRecord {
        match self {
            CallSlot::Fresh(record) | CallSlot::Settled(record) | CallSlot::InFlight(record) => {
                record
            }
        }
    }
}

/// How much of a provider's answer is worth keeping in the database.
///
/// Beyond this only the hash is kept: the row still proves what came back,
/// and the payload — which can be a page of somebody's mail — does not sit in
/// a file that gets copied around.
pub const RESPONSE_KEPT_BYTES: usize = 16 * 1024;

/// The hash that makes a call's evidence checkable later.
pub fn digest(value: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    // Canonical form, so the same request hashes the same whichever order the
    // caller's map happened to iterate in.
    hasher.update(canonical(value).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn canonical(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .into_iter()
                .map(|key| format!("{}:{}", serde_json::to_string(key).unwrap_or_default(), canonical(&map[key])))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        serde_json::Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", inner.join(","))
        }
        other => other.to_string(),
    }
}
