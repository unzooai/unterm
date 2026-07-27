use super::{NextCoreSession, NextCoreState};
use anyhow::{bail, Result};
use portable_pty::PtySize;

pub(super) fn pty_size(cols: usize, rows: usize) -> PtySize {
    PtySize {
        rows: rows.clamp(1, u16::MAX as usize) as u16,
        cols: cols.clamp(1, u16::MAX as usize) as u16,
        pixel_width: 0,
        pixel_height: 0,
    }
}

pub(super) fn resize(
    state: &mut NextCoreState,
    pane_id: usize,
    cols: usize,
    rows: usize,
) -> Result<()> {
    let Some(session) = state
        .sessions
        .iter_mut()
        .find(|session| session.snapshot.id == pane_id)
    else {
        bail!("next-core session {pane_id} not found");
    };

    resize_session(session, cols, rows)
}

fn resize_session(session: &mut NextCoreSession, cols: usize, rows: usize) -> Result<()> {
    session.master.lock().resize(pty_size(cols, rows))?;
    session.snapshot.cols = cols;
    session.snapshot.rows = rows;
    session.screen.lock().resize(cols, rows);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_size_clamps_to_conpty_safe_range() {
        let size = pty_size(0, usize::MAX);

        assert_eq!(size.cols, 1);
        assert_eq!(size.rows, u16::MAX);
        assert_eq!(size.pixel_width, 0);
        assert_eq!(size.pixel_height, 0);
    }

    #[test]
    fn resize_reports_missing_session() {
        let mut state = NextCoreState::default();
        let err = resize(&mut state, 42, 80, 24).expect_err("missing session should fail");

        assert!(err.to_string().contains("next-core session 42 not found"));
    }
}
