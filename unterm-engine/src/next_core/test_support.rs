use super::{
    activity::SessionIoActivity, recording_output, runtime, CellAttributes, NextCoreScreen,
};
use anyhow::Result;
use std::{sync::atomic::Ordering, sync::Arc, time::Instant};

pub(super) fn reset_state_for_test() {
    runtime::test_facade::reset();
}

pub(super) fn set_output_for_test(pane_id: usize, text: &str) -> Result<()> {
    let handles = runtime::test_facade::session_handles(pane_id)?;
    let started_at = Instant::now();
    *handles.output.lock() = text.to_string();
    let mut screen = handles.screen.lock();
    let revision = screen.revision();
    *screen = NextCoreScreen::new(handles.cols, handles.rows);
    screen.render_state.set_revision(revision);
    screen.feed(text);
    let recorded = if let Some(recording) = handles.recording.lock().as_mut() {
        recording_output::append_now(recording, text);
        true
    } else {
        false
    };
    handles
        .activity
        .lock()
        .mark_output(text.len(), 0, recorded, started_at.elapsed());
    Ok(())
}

pub(super) fn mark_dead_for_test(pane_id: usize) -> Result<()> {
    let handles = runtime::test_facade::session_handles(pane_id)?;

    *handles.dead_reason.lock() = Some("test_dead_marker".to_string());
    handles.dead.store(true, Ordering::Release);
    Ok(())
}

fn make_activity_stale_for_test(pane_id: usize) -> Result<()> {
    let handles = runtime::test_facade::session_handles(pane_id)?;

    handles.activity.lock().mark_stale_for_test();
    Ok(())
}

pub(super) fn reset_activity_for_test(pane_id: usize) -> Result<()> {
    let handles = runtime::test_facade::session_handles(pane_id)?;

    *handles.activity.lock() = SessionIoActivity::new();
    make_activity_stale_for_test(pane_id)
}

pub(super) fn viewport_attrs_for_test(pane_id: usize) -> Result<Vec<Vec<CellAttributes>>> {
    let handles = runtime::test_facade::session_handles(pane_id)?;

    let attrs = handles.screen.lock().attrs_for_viewport();
    Ok(attrs)
}

pub(super) fn screen_for_test(pane_id: usize) -> Result<Arc<parking_lot::Mutex<NextCoreScreen>>> {
    runtime::test_facade::session_handles(pane_id).map(|handles| handles.screen)
}
