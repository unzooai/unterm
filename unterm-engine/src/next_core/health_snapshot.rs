use super::{activity::SessionIoActivity, lifecycle, runtime::NextCoreRuntime, session_registry};
use crate::{
    EngineHealthSnapshot, EngineIoHealthSnapshot, EngineLifecycleHealthSnapshot,
    EngineRuntimePumpHealthSnapshot, EngineRuntimeQueueHealthSnapshot,
};

pub(super) fn snapshot(state: &mut NextCoreRuntime) -> EngineHealthSnapshot {
    let pane_count = session_registry::pane_count(state);
    let mut io = EngineIoHealthSnapshot {
        input_writes: 0,
        input_bytes: 0,
        output_chunks: 0,
        output_bytes: 0,
        paste_count: 0,
        paste_text_bytes: 0,
        screen_reads: 0,
        viewport_scrolls: 0,
    };
    let mut dead_reasons = Vec::new();
    let mut dead_sessions = 0u64;
    session_registry::for_each_session_mut(state, |session| {
        if let Some(reason) = lifecycle::refresh_liveness(session) {
            dead_reasons.push(reason);
        }
        if session.snapshot.is_dead {
            dead_sessions = dead_sessions.saturating_add(1);
        }
        add_activity_io(&mut io, &session.activity.lock());
    });
    for reason in dead_reasons {
        lifecycle::record_dead_reason(state, reason);
    }
    let stats = session_registry::stats(state);
    let queue_stats = state.command_queue.stats();
    let pump_stats = state.pump_stats;
    let lifecycle = EngineLifecycleHealthSnapshot {
        live_sessions: pane_count.saturating_sub(dead_sessions as usize) as u64,
        dead_sessions,
        total_created: stats.total_created,
        total_destroyed: stats.total_destroyed,
        total_marked_dead: stats.total_marked_dead,
        last_dead_reason: stats.last_dead_reason,
    };
    EngineHealthSnapshot {
        engine: "next-core".to_string(),
        ready: true,
        status: "ok".to_string(),
        detail: "next-core session registry is available".to_string(),
        pane_count: Some(pane_count),
        io: Some(io),
        lifecycle: Some(lifecycle),
        runtime_queue: Some(EngineRuntimeQueueHealthSnapshot {
            pending_commands: queue_stats.pending_commands,
            pending_input_bytes: queue_stats.pending_input_bytes,
            pending_lifecycle_commands: queue_stats.pending_lanes.lifecycle,
            pending_input_commands: queue_stats.pending_lanes.input,
            pending_render_commands: queue_stats.pending_lanes.render,
            pending_screen_commands: queue_stats.pending_lanes.screen,
            pending_background_commands: queue_stats.pending_lanes.background,
            rejected_commands: queue_stats.rejected_commands,
            rejected_input_bytes: queue_stats.rejected_input_bytes,
        }),
        runtime_pump: Some(EngineRuntimePumpHealthSnapshot {
            drain_calls: pump_stats.drain_calls,
            dispatched_commands: pump_stats.dispatched_commands,
            dispatched_lifecycle_commands: pump_stats.dispatched_lifecycle_commands,
            dispatched_input_commands: pump_stats.dispatched_input_commands,
            dispatched_render_commands: pump_stats.dispatched_render_commands,
            dispatched_screen_commands: pump_stats.dispatched_screen_commands,
            dispatched_background_commands: pump_stats.dispatched_background_commands,
            waited_for_response: pump_stats.waited_for_response,
            completed_without_wait: pump_stats.completed_without_wait,
            total_dispatch_elapsed_micros: pump_stats.total_dispatch_elapsed_micros,
            max_dispatch_elapsed_micros: pump_stats.max_dispatch_elapsed_micros,
            total_drain_elapsed_micros: pump_stats.total_drain_elapsed_micros,
            max_drain_elapsed_micros: pump_stats.max_drain_elapsed_micros,
        }),
    }
}

fn add_activity_io(io: &mut EngineIoHealthSnapshot, activity: &SessionIoActivity) {
    if let Some(input) = &activity.input {
        io.input_writes = io.input_writes.saturating_add(input.total_writes);
        io.input_bytes = io.input_bytes.saturating_add(input.total_bytes);
    }
    if let Some(output) = &activity.output {
        io.output_chunks = io.output_chunks.saturating_add(output.total_chunks);
        io.output_bytes = io.output_bytes.saturating_add(output.total_bytes);
    }
    if let Some(paste) = &activity.paste {
        io.paste_count = io.paste_count.saturating_add(paste.total_pastes);
        io.paste_text_bytes = io.paste_text_bytes.saturating_add(paste.total_text_bytes);
    }
    if let Some(screen) = &activity.screen {
        io.screen_reads = io.screen_reads.saturating_add(screen.total_reads);
        io.viewport_scrolls = io
            .viewport_scrolls
            .saturating_add(screen.total_viewport_scrolls);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn empty_state_reports_ready_health() {
        let mut state = NextCoreRuntime::default();

        let health = snapshot(&mut state);

        assert_eq!(health.engine, "next-core");
        assert!(health.ready);
        assert_eq!(health.pane_count, Some(0));
        assert_eq!(health.lifecycle.expect("lifecycle").live_sessions, 0);
        let queue = health.runtime_queue.expect("runtime queue");
        assert_eq!(queue.pending_commands, 0);
        assert_eq!(queue.pending_input_commands, 0);
        assert_eq!(queue.pending_render_commands, 0);
        assert_eq!(queue.pending_screen_commands, 0);
        assert_eq!(queue.rejected_input_bytes, 0);
        let pump = health.runtime_pump.expect("runtime pump");
        assert_eq!(pump.drain_calls, 0);
        assert_eq!(pump.dispatched_commands, 0);
        assert_eq!(pump.dispatched_lifecycle_commands, 0);
        assert_eq!(pump.dispatched_input_commands, 0);
        assert_eq!(pump.dispatched_render_commands, 0);
        assert_eq!(pump.dispatched_screen_commands, 0);
        assert_eq!(pump.dispatched_background_commands, 0);
        assert_eq!(pump.waited_for_response, 0);
        assert_eq!(pump.completed_without_wait, 0);
        assert_eq!(pump.total_dispatch_elapsed_micros, 0);
        assert_eq!(pump.max_dispatch_elapsed_micros, 0);
        assert_eq!(pump.total_drain_elapsed_micros, 0);
        assert_eq!(pump.max_drain_elapsed_micros, 0);
    }

    #[test]
    fn runtime_pump_health_reports_accumulated_stats() {
        let mut state = NextCoreRuntime::default();
        state.pump_stats.drain_calls = 3;
        state.pump_stats.dispatched_commands = 7;
        state.pump_stats.dispatched_lifecycle_commands = 1;
        state.pump_stats.dispatched_input_commands = 2;
        state.pump_stats.dispatched_render_commands = 3;
        state.pump_stats.dispatched_screen_commands = 4;
        state.pump_stats.dispatched_background_commands = 5;
        state.pump_stats.waited_for_response = 1;
        state.pump_stats.completed_without_wait = 2;
        state.pump_stats.total_dispatch_elapsed_micros = 11;
        state.pump_stats.max_dispatch_elapsed_micros = 7;
        state.pump_stats.total_drain_elapsed_micros = 13;
        state.pump_stats.max_drain_elapsed_micros = 9;

        let health = snapshot(&mut state);

        let pump = health.runtime_pump.expect("runtime pump");
        assert_eq!(pump.drain_calls, 3);
        assert_eq!(pump.dispatched_commands, 7);
        assert_eq!(pump.dispatched_lifecycle_commands, 1);
        assert_eq!(pump.dispatched_input_commands, 2);
        assert_eq!(pump.dispatched_render_commands, 3);
        assert_eq!(pump.dispatched_screen_commands, 4);
        assert_eq!(pump.dispatched_background_commands, 5);
        assert_eq!(pump.waited_for_response, 1);
        assert_eq!(pump.completed_without_wait, 2);
        assert_eq!(pump.total_dispatch_elapsed_micros, 11);
        assert_eq!(pump.max_dispatch_elapsed_micros, 7);
        assert_eq!(pump.total_drain_elapsed_micros, 13);
        assert_eq!(pump.max_drain_elapsed_micros, 9);
    }

    #[test]
    fn activity_io_accumulates_all_channels() {
        let mut io = EngineIoHealthSnapshot {
            input_writes: 0,
            input_bytes: 0,
            output_chunks: 0,
            output_bytes: 0,
            paste_count: 0,
            paste_text_bytes: 0,
            screen_reads: 0,
            viewport_scrolls: 0,
        };
        let mut activity = SessionIoActivity::new();
        activity.mark_input(3, Duration::from_millis(1));
        activity.mark_output(5, 0, false, Duration::from_millis(2));
        activity.mark_paste(7, 11, 2, true, Duration::from_millis(3));
        activity.mark_screen_read(Duration::from_millis(4));
        activity.mark_viewport_scroll(Duration::from_millis(5));

        add_activity_io(&mut io, &activity);

        assert_eq!(io.input_writes, 1);
        assert_eq!(io.input_bytes, 3);
        assert_eq!(io.output_chunks, 1);
        assert_eq!(io.output_bytes, 5);
        assert_eq!(io.paste_count, 1);
        assert_eq!(io.paste_text_bytes, 7);
        assert_eq!(io.screen_reads, 1);
        assert_eq!(io.viewport_scrolls, 1);
    }
}
