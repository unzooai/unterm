use crate::{
    InputActivitySnapshot, OutputActivitySnapshot, PasteActivitySnapshot, ScreenActivitySnapshot,
};
use std::time::{Duration, Instant};

const IDLE_AFTER: Duration = Duration::from_secs(2);

#[derive(Clone, Debug)]
pub(super) struct SessionIoActivity {
    created_at: Instant,
    last_input_at: Option<Instant>,
    last_output_at: Option<Instant>,
    pub(super) input: Option<InputActivitySnapshot>,
    pub(super) output: Option<OutputActivitySnapshot>,
    pub(super) paste: Option<PasteActivitySnapshot>,
    pub(super) screen: Option<ScreenActivitySnapshot>,
}

impl SessionIoActivity {
    pub(super) fn new() -> Self {
        Self {
            created_at: Instant::now(),
            last_input_at: None,
            last_output_at: None,
            input: None,
            output: None,
            paste: None,
            screen: None,
        }
    }

    pub(super) fn mark_input(&mut self, bytes: usize, duration: Duration) {
        self.last_input_at = Some(Instant::now());
        let mut snapshot = self.input.clone().unwrap_or(InputActivitySnapshot {
            total_writes: 0,
            total_bytes: 0,
            last_bytes: 0,
            last_duration_ms: 0,
        });
        snapshot.total_writes = snapshot.total_writes.saturating_add(1);
        snapshot.total_bytes = snapshot.total_bytes.saturating_add(bytes as u64);
        snapshot.last_bytes = bytes;
        snapshot.last_duration_ms = duration.as_millis().min(u64::MAX as u128) as u64;
        self.input = Some(snapshot);
    }

    pub(super) fn mark_output(
        &mut self,
        bytes: usize,
        terminal_response_bytes: usize,
        recorded: bool,
        duration: Duration,
    ) {
        self.last_output_at = Some(Instant::now());
        let mut snapshot = self.output.clone().unwrap_or(OutputActivitySnapshot {
            total_chunks: 0,
            total_bytes: 0,
            total_terminal_response_bytes: 0,
            recorded_chunks: 0,
            last_bytes: 0,
            last_terminal_response_bytes: 0,
            last_recorded: false,
            last_duration_ms: 0,
        });
        snapshot.total_chunks = snapshot.total_chunks.saturating_add(1);
        snapshot.total_bytes = snapshot.total_bytes.saturating_add(bytes as u64);
        snapshot.total_terminal_response_bytes = snapshot
            .total_terminal_response_bytes
            .saturating_add(terminal_response_bytes as u64);
        if recorded {
            snapshot.recorded_chunks = snapshot.recorded_chunks.saturating_add(1);
        }
        snapshot.last_bytes = bytes;
        snapshot.last_terminal_response_bytes = terminal_response_bytes;
        snapshot.last_recorded = recorded;
        snapshot.last_duration_ms = duration.as_millis().min(u64::MAX as u128) as u64;
        self.output = Some(snapshot);
    }

    pub(super) fn mark_paste(
        &mut self,
        text_bytes: usize,
        wire_bytes: usize,
        chunk_count: usize,
        bracketed: bool,
        duration: Duration,
    ) {
        let mut snapshot = self.paste.clone().unwrap_or(PasteActivitySnapshot {
            total_pastes: 0,
            total_text_bytes: 0,
            total_chunks: 0,
            last_text_bytes: 0,
            last_wire_bytes: 0,
            last_chunk_count: 0,
            last_bracketed: false,
            last_duration_ms: 0,
        });
        snapshot.total_pastes = snapshot.total_pastes.saturating_add(1);
        snapshot.total_text_bytes = snapshot.total_text_bytes.saturating_add(text_bytes as u64);
        snapshot.total_chunks = snapshot.total_chunks.saturating_add(chunk_count as u64);
        snapshot.last_text_bytes = text_bytes;
        snapshot.last_wire_bytes = wire_bytes;
        snapshot.last_chunk_count = chunk_count;
        snapshot.last_bracketed = bracketed;
        snapshot.last_duration_ms = duration.as_millis().min(u64::MAX as u128) as u64;
        self.paste = Some(snapshot);
    }

    pub(super) fn mark_screen_read(&mut self, duration: Duration) {
        let mut snapshot = self.screen.clone().unwrap_or(ScreenActivitySnapshot {
            total_reads: 0,
            total_viewport_scrolls: 0,
            last_read_duration_ms: 0,
            last_scroll_duration_ms: 0,
        });
        snapshot.total_reads = snapshot.total_reads.saturating_add(1);
        snapshot.last_read_duration_ms = duration.as_millis().min(u64::MAX as u128) as u64;
        self.screen = Some(snapshot);
    }

    pub(super) fn mark_viewport_scroll(&mut self, duration: Duration) {
        let mut snapshot = self.screen.clone().unwrap_or(ScreenActivitySnapshot {
            total_reads: 0,
            total_viewport_scrolls: 0,
            last_read_duration_ms: 0,
            last_scroll_duration_ms: 0,
        });
        snapshot.total_viewport_scrolls = snapshot.total_viewport_scrolls.saturating_add(1);
        snapshot.last_scroll_duration_ms = duration.as_millis().min(u64::MAX as u128) as u64;
        self.screen = Some(snapshot);
    }

    pub(super) fn is_idle(&self, now: Instant) -> bool {
        self.last_io_at()
            .map(|last_io| now.duration_since(last_io) >= IDLE_AFTER)
            .unwrap_or_else(|| now.duration_since(self.created_at) >= IDLE_AFTER)
    }

    fn last_io_at(&self) -> Option<Instant> {
        match (self.last_input_at, self.last_output_at) {
            (Some(input), Some(output)) => Some(input.max(output)),
            (Some(input), None) => Some(input),
            (None, Some(output)) => Some(output),
            (None, None) => None,
        }
    }

    #[cfg(test)]
    pub(super) fn mark_stale_for_test(&mut self) {
        let stale_at = Instant::now() - IDLE_AFTER - Duration::from_millis(1);
        self.created_at = stale_at;
        self.last_input_at = None;
        self.last_output_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_idle_after_threshold_without_io() {
        let mut activity = SessionIoActivity::new();
        assert!(!activity.is_idle(Instant::now()));
        activity.mark_stale_for_test();
        assert!(activity.is_idle(Instant::now()));
    }

    #[test]
    fn input_and_output_refresh_idle_state_and_metrics() {
        let mut activity = SessionIoActivity::new();
        activity.mark_stale_for_test();
        activity.mark_input(5, Duration::from_millis(7));
        assert!(!activity.is_idle(Instant::now()));
        let input = activity.input.as_ref().expect("input snapshot");
        assert_eq!(input.total_writes, 1);
        assert_eq!(input.total_bytes, 5);
        assert_eq!(input.last_duration_ms, 7);

        activity.mark_output(9, 4, true, Duration::from_millis(11));
        let output = activity.output.as_ref().expect("output snapshot");
        assert_eq!(output.total_chunks, 1);
        assert_eq!(output.total_bytes, 9);
        assert_eq!(output.total_terminal_response_bytes, 4);
        assert_eq!(output.recorded_chunks, 1);
        assert_eq!(output.last_bytes, 9);
        assert_eq!(output.last_terminal_response_bytes, 4);
        assert!(output.last_recorded);
        assert_eq!(output.last_duration_ms, 11);
    }

    #[test]
    fn paste_and_screen_metrics_accumulate() {
        let mut activity = SessionIoActivity::new();
        activity.mark_paste(3, 9, 2, true, Duration::from_millis(4));
        activity.mark_screen_read(Duration::from_millis(6));
        activity.mark_viewport_scroll(Duration::from_millis(8));

        let paste = activity.paste.as_ref().expect("paste snapshot");
        assert_eq!(paste.total_pastes, 1);
        assert_eq!(paste.total_text_bytes, 3);
        assert_eq!(paste.total_chunks, 2);
        assert_eq!(paste.last_wire_bytes, 9);
        assert!(paste.last_bracketed);

        let screen = activity.screen.as_ref().expect("screen snapshot");
        assert_eq!(screen.total_reads, 1);
        assert_eq!(screen.total_viewport_scrolls, 1);
        assert_eq!(screen.last_read_duration_ms, 6);
        assert_eq!(screen.last_scroll_duration_ms, 8);
    }
}
