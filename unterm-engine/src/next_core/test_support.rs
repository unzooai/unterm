use super::{
    activity::SessionIoActivity, recording_output, state, CellAttributes, NextCoreScreen,
    NextCoreState,
};
use anyhow::{bail, Result};
use std::{sync::atomic::Ordering, sync::Arc, time::Instant};

pub(super) fn reset_state_for_test() {
    *state().write() = NextCoreState::default();
}

pub(super) fn set_output_for_test(pane_id: usize, text: &str) -> Result<()> {
    let (output, screen, recording, activity, cols, rows) = {
        let state = state().read();
        let Some(session) = state
            .sessions
            .iter()
            .find(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };
        (
            Arc::clone(&session.output),
            Arc::clone(&session.screen),
            Arc::clone(&session.recording),
            Arc::clone(&session.activity),
            session.snapshot.cols,
            session.snapshot.rows,
        )
    };
    let started_at = Instant::now();
    *output.lock() = text.to_string();
    let mut screen = screen.lock();
    let revision = screen.revision();
    *screen = NextCoreScreen::new(cols, rows);
    screen.render_state.set_revision(revision);
    screen.feed(text);
    let recorded = if let Some(recording) = recording.lock().as_mut() {
        recording_output::append_now(recording, text);
        true
    } else {
        false
    };
    activity
        .lock()
        .mark_output(text.len(), 0, recorded, started_at.elapsed());
    Ok(())
}

pub(super) fn mark_dead_for_test(pane_id: usize) -> Result<()> {
    let state = state().read();
    let Some(session) = state
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
    else {
        bail!("next-core session {pane_id} not found");
    };

    *session.dead_reason.lock() = Some("test_dead_marker".to_string());
    session.dead.store(true, Ordering::Release);
    Ok(())
}

fn make_activity_stale_for_test(pane_id: usize) -> Result<()> {
    let state = state().read();
    let Some(session) = state
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
    else {
        bail!("next-core session {pane_id} not found");
    };

    session.activity.lock().mark_stale_for_test();
    Ok(())
}

pub(super) fn reset_activity_for_test(pane_id: usize) -> Result<()> {
    let state = state().read();
    let Some(session) = state
        .sessions
        .iter()
        .find(|session| session.snapshot.id == pane_id)
    else {
        bail!("next-core session {pane_id} not found");
    };

    *session.activity.lock() = SessionIoActivity::new();
    make_activity_stale_for_test(pane_id)
}

pub(super) fn viewport_attrs_for_test(pane_id: usize) -> Result<Vec<Vec<CellAttributes>>> {
    let screen = {
        let state = state().read();
        let Some(session) = state
            .sessions
            .iter()
            .find(|session| session.snapshot.id == pane_id)
        else {
            bail!("next-core session {pane_id} not found");
        };
        Arc::clone(&session.screen)
    };

    let attrs = screen.lock().attrs_for_viewport();
    Ok(attrs)
}
