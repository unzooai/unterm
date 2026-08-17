//! A provider that does nothing, on purpose.
//!
//! It exists so the contract suite can be run against something whose
//! behaviour is chosen rather than observed: a provider that is offline, one
//! that hangs until cancelled, one whose identity changes underneath a
//! binding. None of those are things a real browser will do on demand.
//!
//! It is also the reference implementation. When a real provider disagrees
//! with the suite, this is what "correct" looked like.

use crate::{Call, Capability, Failure, Handshake, Identity, Provider};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A provider under the test's control.
pub struct FakeProvider {
    id: String,
    identity: Mutex<Identity>,
    capabilities: Vec<Capability>,
    offline: AtomicBool,
    /// Calls that should block until cancelled, by method name.
    slow: Mutex<HashSet<String>>,
    cancelled: Mutex<HashSet<String>>,
    /// Every call actually performed. The suite counts these: "ran once" is a
    /// claim about the provider, not about the store's bookkeeping.
    performed: Mutex<Vec<String>>,
    failures: AtomicUsize,
}

impl FakeProvider {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            identity: Mutex::new(Identity {
                name: "fake".into(),
                version: "1.0.0".into(),
            }),
            capabilities: Capability::ALL.to_vec(),
            offline: AtomicBool::new(false),
            slow: Mutex::new(HashSet::new()),
            cancelled: Mutex::new(HashSet::new()),
            performed: Mutex::new(Vec::new()),
            failures: AtomicUsize::new(0),
        }
    }

    pub fn offering(mut self, capabilities: Vec<Capability>) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Pull the plug.
    pub fn go_offline(&self) {
        self.offline.store(true, Ordering::SeqCst);
    }

    pub fn come_back(&self) {
        self.offline.store(false, Ordering::SeqCst);
    }

    /// Become a different build, the way a provider that was upgraded — or
    /// replaced by something else on the same endpoint — would.
    pub fn become_identity(&self, identity: Identity) {
        *self.identity.lock().unwrap() = identity;
    }

    /// Make a method block until it is cancelled.
    pub fn make_slow(&self, method: &str) {
        self.slow.lock().unwrap().insert(method.to_string());
    }

    /// Methods actually performed, in order.
    pub fn performed(&self) -> Vec<String> {
        self.performed.lock().unwrap().clone()
    }

    pub fn times_called(&self, method: &str) -> usize {
        self.performed
            .lock()
            .unwrap()
            .iter()
            .filter(|name| name.as_str() == method)
            .count()
    }

    pub fn failure_count(&self) -> usize {
        self.failures.load(Ordering::SeqCst)
    }
}

impl Provider for FakeProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn handshake(&self) -> Result<Handshake, Failure> {
        if self.offline.load(Ordering::SeqCst) {
            return Err(Failure::Offline("the fake provider is switched off".into()));
        }
        Ok(Handshake {
            identity: self.identity.lock().unwrap().clone(),
            protocol: crate::discovery::PROTOCOLS[0].to_string(),
            capabilities: self.capabilities.clone(),
        })
    }

    fn call(&self, call: &Call) -> Result<Value, Failure> {
        if self.offline.load(Ordering::SeqCst) {
            self.failures.fetch_add(1, Ordering::SeqCst);
            return Err(Failure::Offline("the fake provider is switched off".into()));
        }
        if !self.capabilities.contains(&call.capability) {
            return Err(Failure::Unsupported(call.capability));
        }

        if self.slow.lock().unwrap().contains(&call.method) {
            // Block until somebody cancels this call, or give up. A test that
            // waits forever is worse than one that fails.
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                if self.cancelled.lock().unwrap().contains(&call.id) {
                    return Err(Failure::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            return Err(Failure::Provider("the slow call was never cancelled".into()));
        }

        self.performed.lock().unwrap().push(call.method.clone());
        Ok(json!({
            "method": call.method,
            "params": call.params,
            "by": self.id,
        }))
    }

    fn cancel(&self, call_id: &str) -> Result<(), Failure> {
        if self.offline.load(Ordering::SeqCst) {
            // Honest: the far side never heard. A cancel that reports success
            // while the provider is unreachable is how a user ends up
            // believing something stopped that did not.
            return Err(Failure::Offline(
                "cannot reach the provider to cancel".into(),
            ));
        }
        self.cancelled.lock().unwrap().insert(call_id.to_string());
        Ok(())
    }
}

/// Register a fake with a registry, with a manifest that looks discovered.
pub fn registered(
    registry: &crate::registry::Registry,
    provider: Arc<FakeProvider>,
) -> crate::ProviderManifest {
    let manifest = crate::ProviderManifest {
        id: provider.id().to_string(),
        name: format!("Fake {}", provider.id()),
        endpoint: crate::Endpoint::Stdio {
            program: "fake".into(),
            args: Vec::new(),
        },
        protocols: crate::discovery::PROTOCOLS
            .iter()
            .map(|version| version.to_string())
            .collect(),
        capabilities: provider.capabilities.clone(),
        families: crate::unzoo::families(),
        source: "test".into(),
        pinned: None,
    };
    registry.register(manifest.clone(), provider);
    manifest
}
