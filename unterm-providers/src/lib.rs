//! Capabilities that live in another process.
//!
//! A brain can think and a terminal can run commands, but a great deal of
//! what an agent is asked to do happens somewhere else — in a browser, in a
//! profile, on a desktop. Those live behind their own processes with their
//! own lifecycles, and the interesting problems are all consequences of that:
//! they can be missing, they can be a different build than the one bound
//! yesterday, they can be told to stop and not stop, and something that
//! recorded a legitimate exchange can try it again.
//!
//! The shape of the answer:
//!
//! * A **manifest** says who a provider is, what it speaks, and what it
//!   offers. It is discovered, never assumed — in particular an endpoint is
//!   read from whatever the provider advertised, so nothing here contains a
//!   port number.
//! * A **lease** ([`unterm_tasks::Lease`]) is permission to use one
//!   capability until a moment, carrying an epoch and a sequence number so a
//!   replayed exchange is refused before it is performed.
//! * A **registry** knows which providers are ready. When the one a task
//!   needs is not, the task waits on `waiting_provider` rather than failing:
//!   a browser that is closed right now is not an error, it is a browser that
//!   is closed right now.
//! * A **contract suite** every provider must pass, real or fake. It is the
//!   only way to know that the fake used in tests and the real thing behave
//!   alike where it matters.

#[cfg(test)]
pub(crate) mod testing {
    //! One lock for every test that changes the environment.
    //!
    //! `HOME` and `UNTERM_STATE_DIR` are process-wide, and tests run in
    //! parallel by default: without this, one test's temporary directory
    //! becomes another's, and the failure looks like a bug in the code under
    //! test rather than in the harness. Serialising them is cheaper than
    //! threading a root path through every discovery call, and it does not
    //! rely on the runner being asked for one thread.

    use std::sync::{Mutex, MutexGuard, OnceLock};

    pub fn env_guard() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        // A poisoned lock means some other test panicked while holding it;
        // that is not a reason to fail every test after it.
        match LOCK.get_or_init(|| Mutex::new(())).lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

pub mod contract;
pub mod discovery;
pub mod fake;
pub mod mcp_http;
pub mod negotiate;
pub mod registry;
pub mod unzoo;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Who a provider says it is.
///
/// Pinned on first bind and compared on every handshake afterwards. Loopback
/// tells you a process on this machine answered; it does not tell you it is
/// the same process you bound to. When this changes, the binding is
/// interrupted and the user is asked again — silently trusting a new identity
/// is how "bound to my browser" quietly becomes "bound to whatever answered".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub version: String,
}

/// A family of things a provider can do.
///
/// Coarse on purpose. A lease names one of these, and a user deciding whether
/// to let an agent drive their browser is answering a question about a
/// family, not about four hundred methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Navigating, reading and clicking pages.
    Browser,
    /// Identities: cookies, logins, profiles. The capability that decides
    /// *whose* browser this is, which is why it is separate from driving it.
    Profile,
    /// The machine outside the browser: input, files, downloads.
    Computer,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Capability::Browser => "browser",
            Capability::Profile => "profile",
            Capability::Computer => "computer",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "browser" => Some(Capability::Browser),
            "profile" => Some(Capability::Profile),
            "computer" => Some(Capability::Computer),
            _ => None,
        }
    }

    pub const ALL: [Capability; 3] = [
        Capability::Browser,
        Capability::Profile,
        Capability::Computer,
    ];
}

/// Where a provider can be reached.
///
/// Every variant carries what it needs; none of them defaults to a port.
/// Discovery fills this in from what the provider itself advertised, which is
/// the whole of "no fixed ports": a provider that moved is found, and one
/// that is not running is not silently mistaken for another process that
/// happens to be on the port it used to have.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Endpoint {
    /// An HTTP endpoint speaking MCP.
    Http { url: String },
    /// A program to run, speaking MCP over its pipes.
    Stdio { program: String, args: Vec<String> },
    /// A unix socket.
    Unix { path: String },
}

/// Everything known about a provider before it is contacted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderManifest {
    pub id: String,
    pub name: String,
    pub endpoint: Endpoint,
    /// The protocol versions this side accepts, newest first.
    pub protocols: Vec<String>,
    pub capabilities: Vec<Capability>,
    /// Which of the provider's tools belong to which capability, keyed by the
    /// part of the tool name before the first underscore. Required: a tool
    /// that maps to nothing can be covered by no lease, which is what keeps a
    /// provider update from silently widening what a permission means.
    #[serde(default)]
    pub families: std::collections::BTreeMap<String, Capability>,
    /// How this manifest was found, for diagnosis — an operator asking "why
    /// is it pointing there" is asking about this field.
    pub source: String,
    /// The identity pinned when the user bound this provider, if they have.
    pub pinned: Option<Identity>,
}

impl ProviderManifest {
    pub fn offers(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}

/// What a provider said when it was contacted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Handshake {
    pub identity: Identity,
    /// The version both sides settled on.
    pub protocol: String,
    pub capabilities: Vec<Capability>,
}

/// One request to a provider.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// This call's own id, used to cancel it.
    pub id: String,
    pub capability: Capability,
    pub method: String,
    pub params: Value,
    /// Repeating a call with the same key must not repeat its effect.
    pub idempotency_key: Option<String>,
}

impl Call {
    pub fn new(capability: Capability, method: impl Into<String>, params: Value) -> Self {
        Self {
            id: format!("cal_{}", uuid::Uuid::new_v4().simple()),
            capability,
            method: method.into(),
            params,
            idempotency_key: None,
        }
    }

    pub fn idempotent(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// What a call produced, and the proof of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    pub value: Value,
    pub evidence: Evidence,
    /// True when this answer came from the record of an earlier identical
    /// call rather than from the provider. The caller usually does not care;
    /// somebody reading an audit trail very much does.
    pub replayed_from_record: bool,
}

/// Enough to prove afterwards what was asked and what came back.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub call_id: String,
    pub provider: String,
    pub capability: Capability,
    pub method: String,
    pub lease_id: Option<String>,
    pub request_sha256: String,
    pub response_sha256: String,
    pub at: String,
}

/// Why a provider could not do something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// It is not running, or not reachable.
    Offline(String),
    /// It answered, but as somebody else.
    IdentityChanged { expected: Identity, found: Identity },
    /// No version in common.
    Incompatible(String),
    /// This provider does not offer that.
    Unsupported(Capability),
    /// The lease presented was refused. Carries the store's own word for why,
    /// because "denied" without a reason is the least actionable message a
    /// user can be given.
    Lease(unterm_tasks::Refusal),
    /// Somebody cancelled it.
    Cancelled,
    /// It failed for its own reasons.
    Provider(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Offline(why) => write!(f, "the provider is not reachable: {why}"),
            Failure::IdentityChanged { expected, found } => write!(
                f,
                "expected {} {} but {} {} answered; the binding must be confirmed again",
                expected.name, expected.version, found.name, found.version
            ),
            Failure::Incompatible(why) => write!(f, "no protocol in common: {why}"),
            Failure::Unsupported(capability) => {
                write!(f, "this provider does not offer {}", capability.as_str())
            }
            Failure::Lease(refusal) => write!(f, "the lease was refused: {}", refusal.as_str()),
            Failure::Cancelled => write!(f, "cancelled"),
            Failure::Provider(why) => write!(f, "the provider failed: {why}"),
        }
    }
}

impl std::error::Error for Failure {}

/// Something that can do work on Unterm's behalf.
///
/// Deliberately small. Everything about permission, deduplication and
/// evidence happens in [`registry::Broker`] around this trait, so a new
/// provider is a matter of speaking to something — not of re-implementing the
/// rules, which is how two providers end up enforcing different ones.
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;

    /// Contact it and find out who answered.
    fn handshake(&self) -> Result<Handshake, Failure>;

    /// Do one thing. Implementations perform no permission checks: by the
    /// time this is called the lease has been presented and accepted.
    fn call(&self, call: &Call) -> Result<Value, Failure>;

    /// Stop a call that is in flight.
    ///
    /// Best-effort by nature — the far side may already have finished — but
    /// it must reach the provider. A cancel that only marks a local record is
    /// the failure this exists to prevent: the browser keeps loading the page
    /// and the user is told it stopped.
    fn cancel(&self, call_id: &str) -> Result<(), Failure>;
}
