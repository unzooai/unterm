use super::{session_registry, with_current, with_current_mut, with_session, NextCoreRuntime};
use anyhow::Result;
use parking_lot::Mutex;
use std::sync::{atomic::AtomicBool, Arc};

pub(in crate::next_core) fn reset() {
    with_current_mut(|state| *state = NextCoreRuntime::default());
}

pub(in crate::next_core) struct TestSessionHandles {
    pub(in crate::next_core) output: Arc<Mutex<String>>,
    pub(in crate::next_core) screen: Arc<Mutex<super::super::NextCoreScreen>>,
    pub(in crate::next_core) recording: Arc<Mutex<Option<super::super::NextCoreRecording>>>,
    pub(in crate::next_core) activity: Arc<Mutex<super::super::activity::SessionIoActivity>>,
    pub(in crate::next_core) dead: Arc<AtomicBool>,
    pub(in crate::next_core) dead_reason: Arc<Mutex<Option<String>>>,
    pub(in crate::next_core) cols: usize,
    pub(in crate::next_core) rows: usize,
}

pub(in crate::next_core) fn session_handles(pane_id: usize) -> Result<TestSessionHandles> {
    with_session(pane_id, |session| {
        Ok(TestSessionHandles {
            output: Arc::clone(&session.output),
            screen: Arc::clone(&session.screen),
            recording: Arc::clone(&session.recording),
            activity: Arc::clone(&session.activity),
            dead: Arc::clone(&session.dead),
            dead_reason: Arc::clone(&session.dead_reason),
            cols: session.snapshot.cols,
            rows: session.snapshot.rows,
        })
    })
}

pub(in crate::next_core) fn pane_count() -> usize {
    with_current(session_registry::pane_count)
}

pub(in crate::next_core) fn next_session_id() -> usize {
    with_current_mut(session_registry::next_session_id)
}
