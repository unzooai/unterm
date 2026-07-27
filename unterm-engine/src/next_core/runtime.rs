use super::session_registry;
use parking_lot::RwLock;
use std::sync::OnceLock;

#[derive(Default)]
pub(super) struct NextCoreRuntime {
    pub(super) registry: session_registry::SessionRegistry,
}

pub(super) fn current() -> &'static RwLock<NextCoreRuntime> {
    static RUNTIME: OnceLock<RwLock<NextCoreRuntime>> = OnceLock::new();
    RUNTIME.get_or_init(|| RwLock::new(NextCoreRuntime::default()))
}
