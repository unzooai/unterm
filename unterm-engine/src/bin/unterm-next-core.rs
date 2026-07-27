use anyhow::{bail, Context, Result};
use portable_pty::CommandBuilder;
use serde_json::json;
use std::time::{Duration, Instant};
use unterm_engine::{
    next_core, CreateSessionRequest, HealthEngine, InputEngine, LaunchEnvBinding, LaunchEnvSource,
    LaunchPolicySnapshot, ScreenEngine, SessionEngine,
};

struct Args {
    cols: usize,
    rows: usize,
    wait_ms: u64,
    write: Option<String>,
    paste: Option<String>,
    cwd: Option<String>,
    env: Vec<(String, String)>,
    command: Option<Vec<String>>,
    bench_input_writes: Option<usize>,
    bench_key_to_screen: Option<usize>,
    bench_input_burst: Option<usize>,
    bench_echo: Option<usize>,
    bench_flood_lines: Option<usize>,
    bench_scrollback_lines: Option<usize>,
    bench_viewport_scrolls: Option<usize>,
    bench_viewport_page_cycle_lines: Option<usize>,
    bench_viewport_scroll_flood: Option<usize>,
    bench_paste_kb: Option<usize>,
    bench_paste_under_flood_kb: Option<usize>,
    bench_dual_agent_lines: Option<usize>,
    bench_agent_startup_lines: Option<usize>,
    bench_screen_read_lines: Option<usize>,
    bench_render_frames: Option<usize>,
    bench_render_plans: Option<usize>,
    bench_render_geometry_plans: Option<usize>,
    bench_render_submission_plans: Option<usize>,
    bench_render_commit_plans: Option<usize>,
    bench_render_cursor_moves: Option<usize>,
    bench_render_application_cursor_moves: Option<usize>,
    bench_focus_switches: Option<usize>,
    bench_session_create: Option<usize>,
    bench_session_ready: Option<usize>,
    poll_ms: u64,
    timeout_ms: u64,
    json: bool,
}

fn parse_args() -> Result<Args> {
    let mut args = std::env::args().skip(1);
    let mut parsed = Args {
        cols: 100,
        rows: 30,
        wait_ms: 250,
        write: None,
        paste: None,
        cwd: None,
        env: Vec::new(),
        command: None,
        bench_input_writes: None,
        bench_key_to_screen: None,
        bench_input_burst: None,
        bench_echo: None,
        bench_flood_lines: None,
        bench_scrollback_lines: None,
        bench_viewport_scrolls: None,
        bench_viewport_page_cycle_lines: None,
        bench_viewport_scroll_flood: None,
        bench_paste_kb: None,
        bench_paste_under_flood_kb: None,
        bench_dual_agent_lines: None,
        bench_agent_startup_lines: None,
        bench_screen_read_lines: None,
        bench_render_frames: None,
        bench_render_plans: None,
        bench_render_geometry_plans: None,
        bench_render_submission_plans: None,
        bench_render_commit_plans: None,
        bench_render_cursor_moves: None,
        bench_render_application_cursor_moves: None,
        bench_focus_switches: None,
        bench_session_create: None,
        bench_session_ready: None,
        poll_ms: 5,
        timeout_ms: 5000,
        json: false,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => {
                let command: Vec<String> = args.collect();
                if command.is_empty() {
                    bail!("-- requires a command");
                }
                parsed.command = Some(command);
                break;
            }
            "--cols" => {
                parsed.cols = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--cols requires a value"))?
                    .parse()?;
            }
            "--rows" => {
                parsed.rows = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--rows requires a value"))?
                    .parse()?;
            }
            "--wait-ms" => {
                parsed.wait_ms = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--wait-ms requires a value"))?
                    .parse()?;
            }
            "--poll-ms" => {
                parsed.poll_ms = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--poll-ms requires a value"))?
                    .parse()?;
            }
            "--timeout-ms" => {
                parsed.timeout_ms = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--timeout-ms requires a value"))?
                    .parse()?;
            }
            "--bench-echo" => {
                parsed.bench_echo = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-echo requires a value"))?
                        .parse()?,
                );
            }
            "--bench-input-writes" => {
                parsed.bench_input_writes = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-input-writes requires a value"))?
                        .parse()?,
                );
            }
            "--bench-key-to-screen" => {
                parsed.bench_key_to_screen = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-key-to-screen requires a value"))?
                        .parse()?,
                );
            }
            "--bench-input-burst" => {
                parsed.bench_input_burst = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-input-burst requires a value"))?
                        .parse()?,
                );
            }
            "--bench-flood-lines" => {
                parsed.bench_flood_lines = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-flood-lines requires a value"))?
                        .parse()?,
                );
            }
            "--bench-scrollback-lines" => {
                parsed.bench_scrollback_lines = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-scrollback-lines requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-viewport-scrolls" => {
                parsed.bench_viewport_scrolls = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-viewport-scrolls requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-viewport-page-cycle-lines" => {
                parsed.bench_viewport_page_cycle_lines = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-viewport-page-cycle-lines requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-viewport-scroll-flood" => {
                parsed.bench_viewport_scroll_flood = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-viewport-scroll-flood requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-paste-kb" => {
                parsed.bench_paste_kb = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-paste-kb requires a value"))?
                        .parse()?,
                );
            }
            "--bench-paste-under-flood-kb" => {
                parsed.bench_paste_under_flood_kb = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-paste-under-flood-kb requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-dual-agent-lines" => {
                parsed.bench_dual_agent_lines = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-dual-agent-lines requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-agent-startup-lines" => {
                parsed.bench_agent_startup_lines = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-agent-startup-lines requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-screen-read-lines" => {
                parsed.bench_screen_read_lines = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-screen-read-lines requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-render-frames" => {
                parsed.bench_render_frames = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-render-frames requires a value"))?
                        .parse()?,
                );
            }
            "--bench-render-plans" => {
                parsed.bench_render_plans = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-render-plans requires a value"))?
                        .parse()?,
                );
            }
            "--bench-render-geometry-plans" => {
                parsed.bench_render_geometry_plans = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-render-geometry-plans requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-render-submission-plans" => {
                parsed.bench_render_submission_plans = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-render-submission-plans requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-render-commit-plans" => {
                parsed.bench_render_commit_plans = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-render-commit-plans requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-render-cursor-moves" => {
                parsed.bench_render_cursor_moves = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-render-cursor-moves requires a value")
                        })?
                        .parse()?,
                );
            }
            "--bench-render-application-cursor-moves" => {
                parsed.bench_render_application_cursor_moves = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "--bench-render-application-cursor-moves requires a value"
                            )
                        })?
                        .parse()?,
                );
            }
            "--bench-focus-switches" => {
                parsed.bench_focus_switches = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-focus-switches requires a value"))?
                        .parse()?,
                );
            }
            "--bench-session-create" => {
                parsed.bench_session_create = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-session-create requires a value"))?
                        .parse()?,
                );
            }
            "--bench-session-ready" => {
                parsed.bench_session_ready = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-session-ready requires a value"))?
                        .parse()?,
                );
            }
            "--write" => {
                parsed.write = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--write requires a value"))?,
                );
            }
            "--paste" => {
                parsed.paste = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--paste requires a value"))?,
                );
            }
            "--cwd" => {
                parsed.cwd = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--cwd requires a value"))?,
                );
            }
            "--env" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--env requires KEY=VALUE"))?;
                let Some((key, env_value)) = value.split_once('=') else {
                    bail!("--env requires KEY=VALUE");
                };
                if key.trim().is_empty() {
                    bail!("--env requires a non-empty key");
                }
                parsed.env.push((key.to_string(), env_value.to_string()));
            }
            "--json" => {
                parsed.json = true;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: unterm-next-core [--cols N] [--rows N] [--wait-ms N] [--poll-ms N] [--timeout-ms N] [--bench-input-writes N] [--bench-key-to-screen N] [--bench-input-burst N] [--bench-echo N] [--bench-flood-lines N] [--bench-scrollback-lines N] [--bench-viewport-scrolls N] [--bench-viewport-page-cycle-lines N] [--bench-viewport-scroll-flood N] [--bench-paste-kb N] [--bench-dual-agent-lines N] [--bench-agent-startup-lines N] [--bench-screen-read-lines N] [--bench-render-frames N] [--bench-render-plans N] [--bench-render-cursor-moves N] [--bench-render-application-cursor-moves N] [--bench-focus-switches N] [--bench-session-create N] [--bench-session-ready N] [--cwd PATH] [--env KEY=VALUE] [--write TEXT] [--paste TEXT] [--json] [-- COMMAND [ARG...]]"
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }

    Ok(parsed)
}

fn command_builder(argv: Option<Vec<String>>) -> Option<CommandBuilder> {
    let mut argv = argv?.into_iter();
    let program = argv.next()?;
    let mut command = CommandBuilder::new(program);
    for arg in argv {
        command.arg(arg);
    }
    Some(command)
}

fn shell_quote_cmd_arg(text: &str) -> String {
    text.replace('"', "\"\"")
}

fn percentile(sorted: &[u128], percentile: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }

    let rank = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

struct FloodRun {
    marker: String,
    before_raw_len: usize,
    started_at: Instant,
}

fn start_flood_stream(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    lines: usize,
) -> Result<FloodRun> {
    let marker = format!("UNTERM_NEXT_CORE_FLOOD_DONE_{lines}_{pane_id}");
    let before_raw_len = engine.debug_output(pane_id)?.len();
    let command = format!("for /L %i in (1,1,{lines}) do @echo UNTERM_NEXT_CORE_FLOOD_%i\r");
    let started_at = Instant::now();
    engine.write_input(pane_id, command.as_str())?;
    engine.write_input(
        pane_id,
        format!("echo {}\r", shell_quote_cmd_arg(marker.as_str())).as_str(),
    )?;
    Ok(FloodRun {
        marker,
        before_raw_len,
        started_at,
    })
}

fn wait_for_marker(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    run: &FloodRun,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<(Duration, usize)> {
    loop {
        let raw = engine.debug_output(pane_id)?;
        if raw[run.before_raw_len.min(raw.len())..].contains(run.marker.as_str()) {
            return Ok((
                run.started_at.elapsed(),
                raw.len().saturating_sub(run.before_raw_len),
            ));
        }
        if run.started_at.elapsed() >= timeout {
            bail!("timed out waiting for marker {}", run.marker);
        }
        std::thread::sleep(poll_interval);
    }
}

fn wait_for_stable_screen_revision(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<u64> {
    let started = Instant::now();
    let poll_interval = poll_interval.max(Duration::from_millis(5));
    let mut previous = engine.read_screen(pane_id)?.revision;
    loop {
        std::thread::sleep(poll_interval);
        let current = engine.read_screen(pane_id)?.revision;
        if current == previous {
            return Ok(current);
        }
        if started.elapsed() >= timeout {
            bail!("timed out waiting for stable screen revision");
        }
        previous = current;
    }
}

fn run_flood_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    lines: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if lines == 0 {
        bail!("--bench-flood-lines must be greater than 0");
    }

    let run = start_flood_stream(engine, pane_id, lines)?;
    let (elapsed, bytes) = wait_for_marker(engine, pane_id, &run, poll_interval, timeout)?;
    let seconds = elapsed.as_secs_f64().max(0.000_001);
    println!(
        "bench_flood lines={} bytes={} elapsed_ms={} lines_per_sec={:.1} bytes_per_sec={:.1}",
        lines,
        bytes,
        elapsed.as_millis(),
        lines as f64 / seconds,
        bytes as f64 / seconds
    );
    Ok(())
}

fn run_scrollback_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    lines: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if lines == 0 {
        bail!("--bench-scrollback-lines must be greater than 0");
    }

    run_flood_benchmark(engine, pane_id, lines, poll_interval, timeout)?;

    let screen = engine.read_screen(pane_id)?;
    let page_rows = screen.rows.max(1);
    let total_rows = screen.scrollback_rows + screen.rows;
    let mut pages = Vec::new();
    let mut start = total_rows.saturating_sub(page_rows);
    loop {
        pages.push(start as i64);
        if start == 0 {
            break;
        }
        start = start.saturating_sub(page_rows);
    }

    let before = Instant::now();
    let mut latencies_us = Vec::with_capacity(pages.len());
    let mut rows_read = 0usize;
    for start in pages.iter().copied() {
        let page_before = Instant::now();
        rows_read += engine.read_lines(pane_id, start, page_rows)?.len();
        latencies_us.push(page_before.elapsed().as_micros());
    }

    latencies_us.sort_unstable();
    let min = latencies_us[0];
    let p50 = percentile(&latencies_us, 0.50);
    let p95 = percentile(&latencies_us, 0.95);
    let max = *latencies_us.last().unwrap_or(&0);
    println!(
        "bench_scrollback lines={} pages={} rows_read={} total_ms={} min_us={} p50_us={} p95_us={} max_us={}",
        lines,
        pages.len(),
        rows_read,
        before.elapsed().as_millis(),
        min,
        p50,
        p95,
        max
    );
    Ok(())
}

fn run_viewport_scroll_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    lines: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if lines == 0 {
        bail!("--bench-viewport-scrolls must be greater than 0");
    }

    run_flood_benchmark(engine, pane_id, lines, poll_interval, timeout)?;

    let screen = engine.read_screen(pane_id)?;
    let page_rows = screen.rows.max(1);
    let total_rows = screen.scrollback_rows + screen.rows;
    let mut targets = Vec::new();
    let mut start = total_rows.saturating_sub(page_rows);
    loop {
        targets.push(start as isize);
        if start == 0 {
            break;
        }
        start = start.saturating_sub(page_rows);
    }

    let before = Instant::now();
    let mut latencies_us = Vec::with_capacity(targets.len());
    let mut rows_read = 0usize;
    for target in targets.iter().copied() {
        let page_before = Instant::now();
        engine.scroll_viewport_to(pane_id, target)?;
        rows_read += engine.read_screen(pane_id)?.lines.len();
        latencies_us.push(page_before.elapsed().as_micros());
    }

    latencies_us.sort_unstable();
    println!(
        "bench_viewport_scroll lines={} pages={} rows_read={} total_ms={} min_us={} p50_us={} p95_us={} max_us={}",
        lines,
        targets.len(),
        rows_read,
        before.elapsed().as_millis(),
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn screen_first_row(screen: &unterm_engine::ScreenSnapshot) -> i64 {
    screen.cells.first().map(|line| line.row).unwrap_or(0)
}

fn run_viewport_page_cycle_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    lines: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if lines == 0 {
        bail!("--bench-viewport-page-cycle-lines must be greater than 0");
    }

    run_flood_benchmark(engine, pane_id, lines, poll_interval, timeout)?;

    let initial = engine.read_screen(pane_id)?;
    let page_rows = initial.rows.max(1) as isize;
    let total_rows = initial.scrollback_rows + initial.rows;
    let max_top = total_rows.saturating_sub(initial.rows) as isize;
    if max_top <= 0 {
        bail!("viewport page-cycle benchmark did not produce scrollback");
    }

    let mut latencies_us = Vec::new();
    let mut pages = 0usize;
    let mut rows_read = 0usize;
    let mut missed_pages = 0usize;
    let mut direction_up = true;
    let mut reached_top = false;
    let before = Instant::now();

    loop {
        let before_screen = engine.read_screen(pane_id)?;
        let before_row = screen_first_row(&before_screen);
        if direction_up && before_row <= 0 {
            reached_top = true;
            direction_up = false;
            continue;
        }
        if !direction_up && before_row >= max_top as i64 {
            break;
        }

        let target = if direction_up {
            before_row as isize - page_rows
        } else {
            before_row as isize + page_rows
        };
        let page_before = Instant::now();
        engine.scroll_viewport_to(pane_id, target)?;
        let after_screen = engine.read_screen(pane_id)?;
        latencies_us.push(page_before.elapsed().as_micros());
        rows_read += after_screen.lines.len();
        pages += 1;

        let after_row = screen_first_row(&after_screen);
        if (direction_up && after_row >= before_row) || (!direction_up && after_row <= before_row) {
            missed_pages += 1;
        }

        if direction_up && after_row <= 0 {
            reached_top = true;
            direction_up = false;
        } else if !direction_up && after_row >= max_top as i64 {
            break;
        }

        if before.elapsed() >= timeout {
            bail!("timed out during viewport page-cycle benchmark");
        }
    }

    if latencies_us.is_empty() {
        bail!("viewport page-cycle benchmark did not perform any page moves");
    }
    latencies_us.sort_unstable();
    let reached_bottom = true;
    let live_tail = reached_bottom && engine.read_screen(pane_id)?.lines.len() == initial.rows;
    let boundary_misses =
        usize::from(!reached_top) + usize::from(!reached_bottom) + usize::from(!live_tail);
    println!(
        "bench_viewport_page_cycle lines={} pages={} rows_read={} reached_top={} reached_bottom={} live_tail={} boundary_misses={} missed_pages={} total_ms={} min_us={} p50_us={} p95_us={} max_us={}",
        lines,
        pages,
        rows_read,
        reached_top,
        reached_bottom,
        live_tail,
        boundary_misses,
        missed_pages,
        before.elapsed().as_millis(),
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_viewport_scroll_during_flood_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    lines: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if lines == 0 {
        bail!("--bench-viewport-scroll-flood must be greater than 0");
    }

    let run = start_flood_stream(engine, pane_id, lines)?;
    let mut latencies_us = Vec::new();
    let mut scrolls = 0usize;
    let mut rows_read = 0usize;
    let mut target = 0isize;
    loop {
        let read_before = Instant::now();
        let screen = engine.read_screen(pane_id)?;
        let total_rows = screen.scrollback_rows + screen.rows;
        let max_top = total_rows.saturating_sub(screen.rows) as isize;
        if max_top > 0 {
            target = if target <= 0 { max_top } else { target - 1 };
        }
        engine.scroll_viewport_to(pane_id, target)?;
        rows_read += engine.read_screen(pane_id)?.lines.len();
        latencies_us.push(read_before.elapsed().as_micros());
        scrolls += 1;

        let raw = engine.debug_output(pane_id)?;
        if raw[run.before_raw_len.min(raw.len())..].contains(run.marker.as_str()) {
            let elapsed = run.started_at.elapsed();
            latencies_us.sort_unstable();
            println!(
                "bench_viewport_scroll_flood lines={} scrolls={} rows_read={} total_ms={} min_us={} p50_us={} p95_us={} max_us={}",
                lines,
                scrolls,
                rows_read,
                elapsed.as_millis(),
                latencies_us[0],
                percentile(&latencies_us, 0.50),
                percentile(&latencies_us, 0.95),
                *latencies_us.last().unwrap_or(&0)
            );
            return Ok(());
        }
        if run.started_at.elapsed() >= timeout {
            bail!(
                "timed out waiting for viewport-scroll flood marker {}",
                run.marker
            );
        }
        std::thread::sleep(poll_interval);
    }
}

fn run_screen_read_during_flood_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    lines: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if lines == 0 {
        bail!("--bench-screen-read-lines must be greater than 0");
    }

    let run = start_flood_stream(engine, pane_id, lines)?;
    let mut latencies_us = Vec::new();
    let mut reads = 0usize;
    let mut total_text_bytes = 0usize;
    loop {
        let read_before = Instant::now();
        let screen = engine.read_screen(pane_id)?;
        latencies_us.push(read_before.elapsed().as_micros());
        reads += 1;
        total_text_bytes += screen.lines.iter().map(String::len).sum::<usize>();

        let raw = engine.debug_output(pane_id)?;
        if raw[run.before_raw_len.min(raw.len())..].contains(run.marker.as_str()) {
            let elapsed = run.started_at.elapsed();
            latencies_us.sort_unstable();
            println!(
                "bench_screen_read_flood lines={} reads={} total_ms={} min_us={} p50_us={} p95_us={} max_us={} text_bytes={}",
                lines,
                reads,
                elapsed.as_millis(),
                latencies_us[0],
                percentile(&latencies_us, 0.50),
                percentile(&latencies_us, 0.95),
                *latencies_us.last().unwrap_or(&0),
                total_text_bytes
            );
            return Ok(());
        }
        if run.started_at.elapsed() >= timeout {
            bail!(
                "timed out waiting for screen-read flood marker {}",
                run.marker
            );
        }
        std::thread::sleep(poll_interval);
    }
}

fn run_render_frame_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-render-frames must be greater than 0");
    }

    let ready_marker = "RENDER_FRAME_BENCH_READY";
    let ready_run = FloodRun {
        marker: ready_marker.to_string(),
        before_raw_len: engine.debug_output(pane_id)?.len(),
        started_at: Instant::now(),
    };
    engine.write_input(
        pane_id,
        "for /L %i in (1,1,30) do @echo RENDER_FRAME_BENCH_%i\r",
    )?;
    engine.write_input(pane_id, format!("echo {ready_marker}\r").as_str())?;
    wait_for_marker(engine, pane_id, &ready_run, poll_interval, timeout)?;

    let full_before = Instant::now();
    let full = engine.read_render_frame(pane_id, None)?;
    let full_us = full_before.elapsed().as_micros();
    if !full.full || full.lines.is_empty() {
        bail!("render frame full snapshot was empty");
    }

    let mut latencies_us = Vec::with_capacity(rounds);
    let mut empty_delta_count = 0usize;
    for _ in 0..rounds {
        let before = Instant::now();
        let frame = engine.read_render_frame(pane_id, Some(full.revision))?;
        latencies_us.push(before.elapsed().as_micros());
        if !frame.full && frame.lines.is_empty() {
            empty_delta_count += 1;
        }
    }

    let dirty_rounds = rounds.min(50);
    let mut dirty_latencies_us = Vec::with_capacity(dirty_rounds);
    let mut dirty_lines = 0usize;
    for idx in 0..dirty_rounds {
        let dirty_baseline = engine.read_render_frame(pane_id, None)?;
        if !dirty_baseline.full || dirty_baseline.lines.is_empty() {
            bail!("render frame dirty baseline was empty");
        }

        let marker = format!("RENDER_FRAME_DIRTY_{idx:04}");
        let wait_started = Instant::now();
        engine.write_input(pane_id, format!("echo {marker}\r").as_str())?;
        loop {
            let screen = engine.read_screen(pane_id)?;
            if screen
                .lines
                .iter()
                .any(|line| line.contains(marker.as_str()))
            {
                break;
            }
            if wait_started.elapsed() >= timeout {
                bail!("timed out waiting for render-frame dirty marker {marker}");
            }
            std::thread::sleep(poll_interval);
        }

        let before = Instant::now();
        let frame = engine.read_render_frame(pane_id, Some(dirty_baseline.revision))?;
        dirty_latencies_us.push(before.elapsed().as_micros());
        if frame.revision <= dirty_baseline.revision
            || frame.dirty_rows.is_none()
            || frame.lines.is_empty()
        {
            bail!(
                "render frame dirty snapshot did not include changed dirty lines: previous_revision={} revision={} full={} dirty_rows={:?} lines={}",
                dirty_baseline.revision,
                frame.revision,
                frame.full,
                frame.dirty_rows,
                frame.lines.len()
            );
        }
        dirty_lines += frame.lines.len();
    }

    latencies_us.sort_unstable();
    dirty_latencies_us.sort_unstable();
    println!(
        "bench_render_frame rounds={} full_us={} full_lines={} empty_deltas={} min_us={} p50_us={} p95_us={} max_us={} dirty_rounds={} dirty_lines={} dirty_min_us={} dirty_p50_us={} dirty_p95_us={} dirty_max_us={}",
        rounds,
        full_us,
        full.lines.len(),
        empty_delta_count,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0),
        dirty_rounds,
        dirty_lines,
        dirty_latencies_us[0],
        percentile(&dirty_latencies_us, 0.50),
        percentile(&dirty_latencies_us, 0.95),
        *dirty_latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_render_plan_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-render-plans must be greater than 0");
    }

    let ready_marker = "RENDER_PLAN_BENCH_READY";
    let ready_run = FloodRun {
        marker: ready_marker.to_string(),
        before_raw_len: engine.debug_output(pane_id)?.len(),
        started_at: Instant::now(),
    };
    engine.write_input(
        pane_id,
        "for /L %i in (1,1,30) do @echo RENDER_PLAN_BENCH_%i abcdefghijklmnopqrstuvwxyz\r",
    )?;
    engine.write_input(pane_id, format!("echo {ready_marker}\r").as_str())?;
    wait_for_marker(engine, pane_id, &ready_run, poll_interval, timeout)?;

    let api_plan = engine.read_render_draw_plan(pane_id, None)?;
    if api_plan.cols == 0 || api_plan.rows == 0 {
        bail!("render draw-plan API returned empty dimensions");
    }

    let frame = engine.read_render_frame(pane_id, None)?;
    if !frame.full || frame.lines.is_empty() {
        bail!("render plan benchmark frame was empty");
    }

    let mut latencies_us = Vec::with_capacity(rounds);
    let mut glyph_runs = 0usize;
    let mut cell_runs = 0usize;
    for _ in 0..rounds {
        let before = Instant::now();
        let plan = frame.to_draw_plan();
        latencies_us.push(before.elapsed().as_micros());
        glyph_runs = plan.glyph_runs.len();
        cell_runs = plan.cell_runs.len();
        if plan.cols != frame.cols || plan.rows != frame.rows {
            bail!(
                "render plan dimensions changed: frame={}x{} plan={}x{}",
                frame.cols,
                frame.rows,
                plan.cols,
                plan.rows
            );
        }
    }

    latencies_us.sort_unstable();
    println!(
        "bench_render_plan rounds={} glyph_runs={} cell_runs={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        glyph_runs,
        cell_runs,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_render_geometry_plan_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-render-geometry-plans must be greater than 0");
    }

    let ready_marker = "RENDER_GEOMETRY_PLAN_BENCH_READY";
    let ready_run = FloodRun {
        marker: ready_marker.to_string(),
        before_raw_len: engine.debug_output(pane_id)?.len(),
        started_at: Instant::now(),
    };
    engine.write_input(
        pane_id,
        "for /L %i in (1,1,30) do @echo RENDER_GEOMETRY_PLAN_BENCH_%i abcdefghijklmnopqrstuvwxyz\r",
    )?;
    engine.write_input(pane_id, format!("echo {ready_marker}\r").as_str())?;
    wait_for_marker(engine, pane_id, &ready_run, poll_interval, timeout)?;

    let frame = engine.read_render_frame(pane_id, None)?;
    if !frame.full || frame.lines.is_empty() {
        bail!("render geometry plan benchmark frame was empty");
    }
    let draw_plan = frame.to_draw_plan();
    if draw_plan.glyph_runs.is_empty() || draw_plan.cell_runs.is_empty() {
        bail!("render geometry plan benchmark draw plan had no runs");
    }

    let metrics = unterm_engine::RenderCellMetrics {
        cell_width_px: 8,
        cell_height_px: 16,
    };
    let mut latencies_us = Vec::with_capacity(rounds);
    let mut glyph_runs = 0usize;
    let mut cell_runs = 0usize;
    let mut viewport_width = 0usize;
    let mut viewport_height = 0usize;
    for _ in 0..rounds {
        let before = Instant::now();
        let geometry = draw_plan.to_geometry_plan(metrics);
        latencies_us.push(before.elapsed().as_micros());
        glyph_runs = geometry.glyph_runs.len();
        cell_runs = geometry.cell_runs.len();
        viewport_width = geometry.viewport.width;
        viewport_height = geometry.viewport.height;
        if geometry.viewport.width != draw_plan.cols * metrics.cell_width_px
            || geometry.viewport.height != draw_plan.rows * metrics.cell_height_px
        {
            bail!(
                "render geometry viewport changed: plan={}x{} viewport={}x{}",
                draw_plan.cols,
                draw_plan.rows,
                geometry.viewport.width,
                geometry.viewport.height
            );
        }
    }

    latencies_us.sort_unstable();
    println!(
        "bench_render_geometry_plan rounds={} glyph_runs={} cell_runs={} viewport={}x{} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        glyph_runs,
        cell_runs,
        viewport_width,
        viewport_height,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_render_submission_plan_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-render-submission-plans must be greater than 0");
    }

    let ready_marker = "RENDER_SUBMISSION_PLAN_BENCH_READY";
    let ready_run = FloodRun {
        marker: ready_marker.to_string(),
        before_raw_len: engine.debug_output(pane_id)?.len(),
        started_at: Instant::now(),
    };
    engine.write_input(
        pane_id,
        "for /L %i in (1,1,30) do @echo RENDER_SUBMISSION_PLAN_BENCH_%i abcdefghijklmnopqrstuvwxyz\r",
    )?;
    engine.write_input(pane_id, format!("echo {ready_marker}\r").as_str())?;
    wait_for_marker(engine, pane_id, &ready_run, poll_interval, timeout)?;

    let geometry = engine
        .read_render_frame(pane_id, None)?
        .to_draw_plan()
        .to_geometry_plan(unterm_engine::RenderCellMetrics {
            cell_width_px: 8,
            cell_height_px: 16,
        });
    if geometry.glyph_runs.is_empty() || geometry.cell_runs.is_empty() {
        bail!("render submission plan benchmark geometry had no runs");
    }

    let mut latencies_us = Vec::with_capacity(rounds);
    let mut damage_rects = 0usize;
    let mut background_quads = 0usize;
    let mut text_runs = 0usize;
    let mut has_cursor = false;
    for _ in 0..rounds {
        let before = Instant::now();
        let submission = geometry.to_submission_plan();
        latencies_us.push(before.elapsed().as_micros());
        damage_rects = submission.damage_rects.len();
        background_quads = submission.background_quads.len();
        text_runs = submission.text_runs.len();
        has_cursor = submission.cursor.is_some();
        if submission.viewport != geometry.viewport || submission.revision != geometry.revision {
            bail!("render submission plan did not preserve geometry metadata");
        }
    }
    if damage_rects == 0 || background_quads == 0 || text_runs == 0 || !has_cursor {
        bail!(
            "render submission plan missing commands: damage={} background={} text={} cursor={}",
            damage_rects,
            background_quads,
            text_runs,
            has_cursor
        );
    }

    latencies_us.sort_unstable();
    println!(
        "bench_render_submission_plan rounds={} damage_rects={} background_quads={} text_runs={} cursor={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        damage_rects,
        background_quads,
        text_runs,
        has_cursor,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_render_commit_plan_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-render-commit-plans must be greater than 0");
    }

    let ready_marker = "RENDER_COMMIT_PLAN_BENCH_READY";
    let ready_run = FloodRun {
        marker: ready_marker.to_string(),
        before_raw_len: engine.debug_output(pane_id)?.len(),
        started_at: Instant::now(),
    };
    engine.write_input(
        pane_id,
        "for /L %i in (1,1,30) do @echo RENDER_COMMIT_PLAN_BENCH_%i abcdefghijklmnopqrstuvwxyz\r",
    )?;
    engine.write_input(pane_id, format!("echo {ready_marker}\r").as_str())?;
    wait_for_marker(engine, pane_id, &ready_run, poll_interval, timeout)?;
    let _ = wait_for_stable_screen_revision(engine, pane_id, poll_interval, timeout)?;

    let metrics = unterm_engine::RenderCellMetrics {
        cell_width_px: 8,
        cell_height_px: 16,
    };
    let mut full_latencies_us = Vec::with_capacity(rounds);
    let mut skip_latencies_us = Vec::with_capacity(rounds);
    let mut damage_rects = 0usize;
    let mut text_runs = 0usize;
    for _ in 0..rounds {
        let mut consumer = unterm_engine::RenderConsumerState::new();
        let full_before = Instant::now();
        let full = engine.read_render_commit_plan(pane_id, metrics, &mut consumer)?;
        full_latencies_us.push(full_before.elapsed().as_micros());
        if !full.submit || !full.requires_full_repaint {
            bail!("render commit plan first read did not submit full repaint");
        }
        let Some(submission) = full.submission else {
            bail!("render commit plan first read did not include submission");
        };
        damage_rects = submission.damage_rects.len();
        text_runs = submission.text_runs.len();
        if damage_rects == 0 || text_runs == 0 {
            bail!(
                "render commit plan first read missing commands: damage={} text={}",
                damage_rects,
                text_runs
            );
        }

        let skip_before = Instant::now();
        let skipped = engine.read_render_commit_plan(pane_id, metrics, &mut consumer)?;
        skip_latencies_us.push(skip_before.elapsed().as_micros());
        if skipped.submit || skipped.submission.is_some() {
            bail!("render commit plan repeated revision did not skip submission");
        }
    }

    full_latencies_us.sort_unstable();
    skip_latencies_us.sort_unstable();
    println!(
        "bench_render_commit_plan rounds={} damage_rects={} text_runs={} full_min_us={} full_p50_us={} full_p95_us={} full_max_us={} skip_min_us={} skip_p50_us={} skip_p95_us={} skip_max_us={}",
        rounds,
        damage_rects,
        text_runs,
        full_latencies_us[0],
        percentile(&full_latencies_us, 0.50),
        percentile(&full_latencies_us, 0.95),
        *full_latencies_us.last().unwrap_or(&0),
        skip_latencies_us[0],
        percentile(&skip_latencies_us, 0.50),
        percentile(&skip_latencies_us, 0.95),
        *skip_latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_render_cursor_move_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
    application_cursor_sequences: bool,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-render-cursor-moves must be greater than 0");
    }

    let marker = "UNTERM_CURSOR_MOVE_BENCHMARK";
    engine.write_input(pane_id, marker)?;
    let started = Instant::now();
    let mut screen = loop {
        let screen = engine.read_screen(pane_id)?;
        if screen.lines.iter().any(|line| line.contains(marker)) {
            break screen;
        }
        if started.elapsed() >= timeout {
            bail!("timed out waiting for cursor-move benchmark marker");
        }
        std::thread::sleep(poll_interval);
    };

    let mut baseline = engine.read_render_frame(pane_id, None)?;
    if !baseline.full || baseline.lines.is_empty() {
        bail!("render cursor-move baseline was empty");
    }

    let mut latencies_us = Vec::with_capacity(rounds);
    let mut dirty_lines = 0usize;
    let mut full_frames = 0usize;
    let mut snapshots = 0usize;
    let mut left_moves = 0usize;
    let mut right_moves = 0usize;
    for idx in 0..rounds {
        let moving_left = idx % 2 == 0;
        let before_cursor = screen.cursor;
        let expected_x = if moving_left {
            before_cursor.x.saturating_sub(1)
        } else {
            (before_cursor.x + 1).min(screen.cols.saturating_sub(1))
        };
        let input = match (application_cursor_sequences, moving_left) {
            (true, true) => "\x1bOD",
            (true, false) => "\x1bOC",
            (false, true) => "\x1b[D",
            (false, false) => "\x1b[C",
        };

        engine.write_input(pane_id, input)?;
        let wait_started = Instant::now();
        loop {
            screen = engine.read_screen(pane_id)?;
            snapshots += 1;
            if screen.cursor.y == before_cursor.y && screen.cursor.x == expected_x {
                break;
            }
            if wait_started.elapsed() >= timeout {
                bail!(
                    "timed out waiting for cursor move: before=({}, {}) expected_x={} actual=({}, {})",
                    before_cursor.x,
                    before_cursor.y,
                    expected_x,
                    screen.cursor.x,
                    screen.cursor.y
                );
            }
            std::thread::sleep(poll_interval);
        }
        if moving_left {
            left_moves += 1;
        } else {
            right_moves += 1;
        }

        let before = Instant::now();
        let frame = engine.read_render_frame(pane_id, Some(baseline.revision))?;
        latencies_us.push(before.elapsed().as_micros());
        if frame.full {
            full_frames += 1;
        }
        let Some(dirty_rows) = frame.dirty_rows else {
            bail!("render cursor-move delta did not include dirty rows");
        };
        if screen.cursor.y < 0 {
            bail!("cursor row was negative: {}", screen.cursor.y);
        }
        let cursor_y = screen.cursor.y as usize;
        if frame.lines.is_empty() || cursor_y < dirty_rows.start || cursor_y > dirty_rows.end {
            bail!(
                "render cursor-move dirty rows did not cover cursor row: cursor_y={} dirty_rows={:?} lines={}",
                cursor_y,
                dirty_rows,
                frame.lines.len()
            );
        }
        if frame
            .lines
            .iter()
            .any(|line| line.cells.len() != screen.cols)
        {
            bail!(
                "render cursor-move delta returned an unpadded line: cols={} line_cell_counts={:?}",
                screen.cols,
                frame
                    .lines
                    .iter()
                    .map(|line| line.cells.len())
                    .collect::<Vec<_>>()
            );
        }
        dirty_lines += frame.lines.len();
        baseline = frame;
    }

    engine.write_input(pane_id, "\x03")?;

    latencies_us.sort_unstable();
    let missed_moves = rounds.saturating_sub(left_moves + right_moves);
    let prefix = if application_cursor_sequences {
        "bench_render_application_cursor_move"
    } else {
        "bench_render_cursor_move"
    };
    println!(
        "{prefix} rounds={} snapshots={} dirty_lines={} full_frames={} left_moves={} right_moves={} missed_moves={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        snapshots,
        dirty_lines,
        full_frames,
        left_moves,
        right_moves,
        missed_moves,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn make_paste_payload(bytes: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut payload = String::with_capacity(bytes);
    for idx in 0..bytes {
        payload.push(ALPHABET[idx % ALPHABET.len()] as char);
    }
    payload
}

fn run_paste_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    kb: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if kb == 0 {
        bail!("--bench-paste-kb must be greater than 0");
    }

    let bytes = kb
        .checked_mul(1024)
        .ok_or_else(|| anyhow::anyhow!("--bench-paste-kb is too large"))?;
    let marker = format!("UNTERM_NEXT_CORE_PASTE_DONE_{bytes}");
    let payload = make_paste_payload(bytes);
    let command = format!(
        "set /p UNTERM_NEXT_CORE_PASTE_INPUT=&echo {}\r",
        shell_quote_cmd_arg(marker.as_str())
    );
    engine.write_input(pane_id, command.as_str())?;
    std::thread::sleep(poll_interval);

    let before_raw_len = engine.debug_output(pane_id)?.len();
    let before = Instant::now();
    engine.paste_input(pane_id, format!("{payload}\r").as_str())?;

    loop {
        let raw = engine.debug_output(pane_id)?;
        if raw[before_raw_len.min(raw.len())..].contains(marker.as_str()) {
            let elapsed = before.elapsed();
            let seconds = elapsed.as_secs_f64().max(0.000_001);
            println!(
                "bench_paste bytes={} elapsed_ms={} bytes_per_sec={:.1}",
                bytes,
                elapsed.as_millis(),
                bytes as f64 / seconds
            );
            return Ok(());
        }
        if before.elapsed() >= timeout {
            bail!("timed out waiting for paste marker {marker}");
        }
        std::thread::sleep(poll_interval);
    }
}

fn run_paste_under_flood_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    cols: usize,
    rows: usize,
    kb: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if kb == 0 {
        bail!("--bench-paste-under-flood-kb must be greater than 0");
    }
    let bytes = kb
        .checked_mul(1024)
        .ok_or_else(|| anyhow::anyhow!("--bench-paste-under-flood-kb is too large"))?;
    let flood_lines = kb.saturating_mul(500).max(5000);
    let agent = engine.create_session(cmd_session(cols, rows))?;
    let flood = start_flood_stream(engine, agent.id, flood_lines)?;

    let result = (|| -> Result<(u128, u128, u8)> {
        let marker = format!("UNTERM_NEXT_CORE_PASTE_FLOOD_DONE_{bytes}");
        engine.write_input(
            pane_id,
            format!(
                "set /p UNTERM_NEXT_CORE_PASTE_FLOOD_INPUT=&echo {}\r",
                shell_quote_cmd_arg(marker.as_str())
            )
            .as_str(),
        )?;
        std::thread::sleep(poll_interval);
        let before_raw_len = engine.debug_output(pane_id)?.len();
        let before = Instant::now();
        engine.paste_input(pane_id, format!("{}\r", make_paste_payload(bytes)).as_str())?;
        let write_ms = before.elapsed().as_millis();
        loop {
            let raw = engine.debug_output(pane_id)?;
            if raw[before_raw_len.min(raw.len())..].contains(marker.as_str()) {
                return Ok((before.elapsed().as_millis(), write_ms, 0));
            }
            if before.elapsed() >= timeout {
                return Ok((before.elapsed().as_millis(), write_ms, 1));
            }
            std::thread::sleep(poll_interval);
        }
    })();

    let flood_wait = wait_for_marker(engine, agent.id, &flood, poll_interval, timeout);
    engine.destroy_session(agent.id)?;
    let (elapsed_ms, write_ms, marker_misses) = result?;
    let (flood_elapsed, flood_bytes) = flood_wait?;
    println!(
        "bench_paste_under_flood bytes={} flood_lines={} flood_bytes={} elapsed_ms={} write_ms={} marker_misses={} background_elapsed_ms={}",
        bytes,
        flood_lines,
        flood_bytes,
        elapsed_ms,
        write_ms,
        marker_misses,
        flood_elapsed.as_millis()
    );
    Ok(())
}

fn run_echo_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-echo must be greater than 0");
    }

    let sorted = collect_echo_latencies(engine, pane_id, rounds, poll_interval, timeout)?;
    print_echo_summary("bench_echo", rounds, &sorted);
    Ok(())
}

fn collect_echo_latencies(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<Vec<u128>> {
    let mut latencies_us = Vec::with_capacity(rounds);
    let mut seen_len = engine.debug_output(pane_id)?.len();
    for idx in 0..rounds {
        let marker = format!("UNTERM_NEXT_CORE_BENCH_{idx:04}");
        let before = Instant::now();
        engine.write_input(pane_id, format!("echo {marker}\r").as_str())?;

        loop {
            let raw = engine.debug_output(pane_id)?;
            if raw.len() >= seen_len && raw[seen_len..].contains(marker.as_str()) {
                seen_len = raw.len();
                latencies_us.push(before.elapsed().as_micros());
                break;
            }
            if before.elapsed() >= timeout {
                bail!("timed out waiting for echo marker {marker}");
            }
            std::thread::sleep(poll_interval);
        }
    }

    let mut sorted = latencies_us;
    sorted.sort_unstable();
    Ok(sorted)
}

fn print_echo_summary(label: &str, rounds: usize, sorted: &[u128]) {
    let min = sorted[0];
    let p50 = percentile(&sorted, 0.50);
    let p95 = percentile(&sorted, 0.95);
    let max = *sorted.last().unwrap_or(&0);
    println!(
        "{} rounds={} min_us={} p50_us={} p95_us={} max_us={}",
        label, rounds, min, p50, p95, max
    );
}

fn run_input_write_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-input-writes must be greater than 0");
    }

    let mut latencies_us = Vec::with_capacity(rounds);
    let mut bytes = 0usize;
    for _ in 0..rounds {
        let before = Instant::now();
        engine.write_input(pane_id, "\x1b[C")?;
        latencies_us.push(before.elapsed().as_micros());
        bytes += 3;
    }

    latencies_us.sort_unstable();
    let seconds = latencies_us.iter().sum::<u128>() as f64 / 1_000_000.0;
    let bytes_per_sec = if seconds > 0.0 {
        bytes as f64 / seconds
    } else {
        bytes as f64
    };
    println!(
        "bench_input_write rounds={} bytes={} min_us={} p50_us={} p95_us={} max_us={} bytes_per_sec={:.1}",
        rounds,
        bytes,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0),
        bytes_per_sec
    );
    Ok(())
}

fn run_key_to_screen_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    pane_id: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-key-to-screen must be greater than 0");
    }

    let mut latencies_us = Vec::with_capacity(rounds);
    let mut snapshots = 0usize;
    for idx in 0..rounds {
        let marker = format!("KTS{idx:04}");
        let before = Instant::now();
        engine.write_input(pane_id, format!("echo {marker}\r").as_str())?;
        loop {
            let screen = engine.read_screen(pane_id)?;
            snapshots += 1;
            if screen
                .lines
                .iter()
                .any(|line| line.contains(marker.as_str()))
            {
                latencies_us.push(before.elapsed().as_micros());
                break;
            }
            if before.elapsed() >= timeout {
                bail!("timed out waiting for key-to-screen marker {marker}");
            }
            std::thread::sleep(poll_interval);
        }
    }

    latencies_us.sort_unstable();
    println!(
        "bench_key_to_screen rounds={} snapshots={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        snapshots,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_input_burst_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    interactive_pane_id: usize,
    cols: usize,
    rows: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-input-burst must be greater than 0");
    }

    let flood_lines = rounds.saturating_mul(20).max(1000);
    let first_agent = engine.create_session(cmd_session(cols, rows))?;
    let second_agent = engine.create_session(cmd_session(cols, rows))?;
    let first_run = start_flood_stream(engine, first_agent.id, flood_lines)?;
    let second_run = start_flood_stream(engine, second_agent.id, flood_lines)?;

    let result = (|| -> Result<Vec<u128>> {
        let mut latencies_us = Vec::with_capacity(rounds);
        for _ in 0..rounds {
            let before = Instant::now();
            engine.write_input(interactive_pane_id, "\x1b[C")?;
            latencies_us.push(before.elapsed().as_micros());
        }
        latencies_us.sort_unstable();
        Ok(latencies_us)
    })();

    let first_wait = wait_for_marker(engine, first_agent.id, &first_run, poll_interval, timeout);
    let second_wait = wait_for_marker(engine, second_agent.id, &second_run, poll_interval, timeout);
    engine.destroy_session(first_agent.id)?;
    engine.destroy_session(second_agent.id)?;

    let latencies_us = result?;
    let (first_elapsed, first_bytes) = first_wait?;
    let (second_elapsed, second_bytes) = second_wait?;
    println!(
        "bench_input_burst rounds={} background_sessions=2 background_lines_per_session={} background_bytes={} background_elapsed_ms={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        flood_lines,
        first_bytes + second_bytes,
        first_elapsed.max(second_elapsed).as_millis(),
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn cmd_session(cols: usize, rows: usize) -> CreateSessionRequest {
    CreateSessionRequest {
        cols,
        rows,
        command_dir: None,
        command: Some(CommandBuilder::new("cmd.exe")),
        env: Vec::new(),
        launch_policy: Default::default(),
    }
}

fn run_dual_agent_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    interactive_pane_id: usize,
    cols: usize,
    rows: usize,
    lines: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if lines == 0 {
        bail!("--bench-dual-agent-lines must be greater than 0");
    }

    let first_agent = engine.create_session(cmd_session(cols, rows))?;
    let second_agent = engine.create_session(cmd_session(cols, rows))?;
    let first_run = start_flood_stream(engine, first_agent.id, lines)?;
    let second_run = start_flood_stream(engine, second_agent.id, lines)?;

    let echo_rounds = 20;
    let echo_latencies = collect_echo_latencies(
        engine,
        interactive_pane_id,
        echo_rounds,
        poll_interval,
        timeout,
    )?;
    print_echo_summary("bench_dual_agents_echo", echo_rounds, &echo_latencies);

    let (first_elapsed, first_bytes) =
        wait_for_marker(engine, first_agent.id, &first_run, poll_interval, timeout)?;
    let (second_elapsed, second_bytes) =
        wait_for_marker(engine, second_agent.id, &second_run, poll_interval, timeout)?;
    let combined_seconds = first_elapsed
        .max(second_elapsed)
        .as_secs_f64()
        .max(0.000_001);
    println!(
        "bench_dual_agents lines_per_agent={} total_bytes={} elapsed_ms={} combined_lines_per_sec={:.1} combined_bytes_per_sec={:.1}",
        lines,
        first_bytes + second_bytes,
        first_elapsed.max(second_elapsed).as_millis(),
        (lines * 2) as f64 / combined_seconds,
        (first_bytes + second_bytes) as f64 / combined_seconds
    );
    engine.destroy_session(first_agent.id)?;
    engine.destroy_session(second_agent.id)?;
    Ok(())
}

fn run_agent_startup_stall_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    interactive_pane_id: usize,
    cols: usize,
    rows: usize,
    lines: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if lines == 0 {
        bail!("--bench-agent-startup-lines must be greater than 0");
    }

    let agent = engine.create_session(cmd_session(cols, rows))?;
    let run = start_flood_stream(engine, agent.id, lines)?;
    let result = (|| -> Result<(Vec<u128>, Vec<u128>, usize, Duration, usize)> {
        let mut input_latencies_us = Vec::new();
        let mut screen_latencies_us = Vec::new();
        let mut screen_reads = 0usize;
        loop {
            let input_before = Instant::now();
            engine.write_input(interactive_pane_id, "\x1b[C")?;
            input_latencies_us.push(input_before.elapsed().as_micros());

            let screen_before = Instant::now();
            let _ = engine.read_screen(interactive_pane_id)?;
            screen_latencies_us.push(screen_before.elapsed().as_micros());
            screen_reads += 1;

            let raw = engine.debug_output(agent.id)?;
            if raw[run.before_raw_len.min(raw.len())..].contains(run.marker.as_str()) {
                let elapsed = run.started_at.elapsed();
                let bytes = raw.len().saturating_sub(run.before_raw_len);
                input_latencies_us.sort_unstable();
                screen_latencies_us.sort_unstable();
                return Ok((
                    input_latencies_us,
                    screen_latencies_us,
                    screen_reads,
                    elapsed,
                    bytes,
                ));
            }
            if run.started_at.elapsed() >= timeout {
                bail!("timed out waiting for agent-startup marker {}", run.marker);
            }
            std::thread::sleep(poll_interval);
        }
    })();

    engine.destroy_session(agent.id)?;

    let (input_latencies_us, screen_latencies_us, screen_reads, elapsed, bytes) = result?;
    println!(
        "bench_agent_startup_stall lines={} bytes={} input_writes={} screen_reads={} elapsed_ms={} input_min_us={} input_p50_us={} input_p95_us={} input_max_us={} screen_read_min_us={} screen_read_p50_us={} screen_read_p95_us={} screen_read_max_us={}",
        lines,
        bytes,
        input_latencies_us.len(),
        screen_reads,
        elapsed.as_millis(),
        input_latencies_us[0],
        percentile(&input_latencies_us, 0.50),
        percentile(&input_latencies_us, 0.95),
        *input_latencies_us.last().unwrap_or(&0),
        screen_latencies_us[0],
        percentile(&screen_latencies_us, 0.50),
        percentile(&screen_latencies_us, 0.95),
        *screen_latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_focus_switch_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    initial_pane_id: usize,
    cols: usize,
    rows: usize,
    rounds: usize,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-focus-switches must be greater than 0");
    }

    let mut pane_ids = vec![initial_pane_id];
    for _ in 0..3 {
        let session = engine.create_session(cmd_session(cols, rows))?;
        pane_ids.push(session.id);
    }

    let result = (|| -> Result<Vec<u128>> {
        let mut latencies_us = Vec::with_capacity(rounds);
        for idx in 0..rounds {
            let target = pane_ids[idx % pane_ids.len()];
            let before = Instant::now();
            engine.focus_session(target)?;
            let focused = engine.get_session(target)?;
            if !focused.is_active {
                bail!("focused session {target} was not marked active");
            }
            latencies_us.push(before.elapsed().as_micros());
        }

        latencies_us.sort_unstable();
        Ok(latencies_us)
    })();

    for pane_id in pane_ids.iter().copied().skip(1) {
        engine.destroy_session(pane_id)?;
    }

    let latencies_us = result?;
    println!(
        "bench_focus_switch rounds={} sessions={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        pane_ids.len(),
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn run_session_create_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    cols: usize,
    rows: usize,
    rounds: usize,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-session-create must be greater than 0");
    }

    let mut latencies_us = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let before = Instant::now();
        let session = engine.create_session(cmd_session(cols, rows))?;
        latencies_us.push(before.elapsed().as_micros());
        engine.destroy_session(session.id)?;
    }

    latencies_us.sort_unstable();
    println!(
        "bench_session_create rounds={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn ready_session(marker: &str, cols: usize, rows: usize) -> CreateSessionRequest {
    let mut command = CommandBuilder::new("cmd.exe");
    command.arg("/C");
    command.arg(format!("echo {marker}"));
    CreateSessionRequest {
        cols,
        rows,
        command_dir: None,
        command: Some(command),
        env: Vec::new(),
        launch_policy: Default::default(),
    }
}

fn run_session_ready_benchmark(
    engine: &unterm_engine::next_core::NextCoreEngine,
    cols: usize,
    rows: usize,
    rounds: usize,
    poll_interval: Duration,
    timeout: Duration,
) -> Result<()> {
    if rounds == 0 {
        bail!("--bench-session-ready must be greater than 0");
    }

    let mut latencies_us = Vec::with_capacity(rounds);
    for idx in 0..rounds {
        let marker = format!("UNTERM_NEXT_CORE_READY_{idx:04}");
        let before = Instant::now();
        let session = engine.create_session(ready_session(marker.as_str(), cols, rows))?;
        let result = loop {
            let raw = engine.debug_output(session.id)?;
            if raw.contains(marker.as_str()) {
                break Ok(before.elapsed().as_micros());
            }
            if before.elapsed() >= timeout {
                break Err(anyhow::anyhow!(
                    "timed out waiting for ready marker {marker}"
                ));
            }
            std::thread::sleep(poll_interval);
        };
        engine.destroy_session(session.id)?;
        latencies_us.push(result?);
    }

    latencies_us.sort_unstable();
    println!(
        "bench_session_ready rounds={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds,
        latencies_us[0],
        percentile(&latencies_us, 0.50),
        percentile(&latencies_us, 0.95),
        *latencies_us.last().unwrap_or(&0)
    );
    Ok(())
}

fn explicit_launch_policy(env: &[(String, String)]) -> LaunchPolicySnapshot {
    LaunchPolicySnapshot {
        profile: env
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("UNTERM_PROFILE"))
            .map(|(_, value)| value.clone())
            .filter(|value| !value.trim().is_empty()),
        proxy_env_keys: env
            .iter()
            .filter_map(|(key, _)| {
                let upper = key.to_ascii_uppercase();
                matches!(
                    upper.as_str(),
                    "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY" | "NO_PROXY"
                )
                .then(|| key.clone())
            })
            .collect(),
        env: env
            .iter()
            .map(|(key, _)| LaunchEnvBinding {
                key: key.clone(),
                source: LaunchEnvSource::Explicit,
            })
            .collect(),
        ..Default::default()
    }
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let engine = next_core();
    let launch_policy = explicit_launch_policy(&args.env);
    let session = engine.create_session(CreateSessionRequest {
        cols: args.cols,
        rows: args.rows,
        command_dir: args.cwd,
        command: command_builder(args.command),
        env: args.env,
        launch_policy,
    })?;

    if let Some(rounds) = args.bench_input_writes {
        run_input_write_benchmark(&engine, session.id, rounds)
            .with_context(|| format!("bench_input_write failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_key_to_screen {
        run_key_to_screen_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_key_to_screen failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_input_burst {
        run_input_burst_benchmark(
            &engine,
            session.id,
            args.cols,
            args.rows,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_input_burst failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_echo {
        run_echo_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_echo failed for session {}", session.id))?;
    }

    if let Some(lines) = args.bench_flood_lines {
        run_flood_benchmark(
            &engine,
            session.id,
            lines,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_flood failed for session {}", session.id))?;
    }

    if let Some(lines) = args.bench_scrollback_lines {
        run_scrollback_benchmark(
            &engine,
            session.id,
            lines,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_scrollback failed for session {}", session.id))?;
    }

    if let Some(lines) = args.bench_viewport_scrolls {
        run_viewport_scroll_benchmark(
            &engine,
            session.id,
            lines,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_viewport_scroll failed for session {}", session.id))?;
    }

    if let Some(lines) = args.bench_viewport_page_cycle_lines {
        run_viewport_page_cycle_benchmark(
            &engine,
            session.id,
            lines,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| {
            format!(
                "bench_viewport_page_cycle failed for session {}",
                session.id
            )
        })?;
    }

    if let Some(lines) = args.bench_viewport_scroll_flood {
        run_viewport_scroll_during_flood_benchmark(
            &engine,
            session.id,
            lines,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| {
            format!(
                "bench_viewport_scroll_flood failed for session {}",
                session.id
            )
        })?;
    }

    if let Some(kb) = args.bench_paste_kb {
        run_paste_benchmark(
            &engine,
            session.id,
            kb,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_paste failed for session {}", session.id))?;
    }

    if let Some(kb) = args.bench_paste_under_flood_kb {
        run_paste_under_flood_benchmark(
            &engine,
            session.id,
            args.cols,
            args.rows,
            kb,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_paste_under_flood failed for session {}", session.id))?;
    }

    if let Some(lines) = args.bench_dual_agent_lines {
        run_dual_agent_benchmark(
            &engine,
            session.id,
            args.cols,
            args.rows,
            lines,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_dual_agents failed for session {}", session.id))?;
    }

    if let Some(lines) = args.bench_agent_startup_lines {
        run_agent_startup_stall_benchmark(
            &engine,
            session.id,
            args.cols,
            args.rows,
            lines,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| {
            format!(
                "bench_agent_startup_stall failed for session {}",
                session.id
            )
        })?;
    }

    if let Some(lines) = args.bench_screen_read_lines {
        run_screen_read_during_flood_benchmark(
            &engine,
            session.id,
            lines,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_screen_read failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_render_frames {
        run_render_frame_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_render_frame failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_render_plans {
        run_render_plan_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_render_plan failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_render_geometry_plans {
        run_render_geometry_plan_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| {
            format!(
                "bench_render_geometry_plan failed for session {}",
                session.id
            )
        })?;
    }

    if let Some(rounds) = args.bench_render_submission_plans {
        run_render_submission_plan_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| {
            format!(
                "bench_render_submission_plan failed for session {}",
                session.id
            )
        })?;
    }

    if let Some(rounds) = args.bench_render_commit_plans {
        run_render_commit_plan_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_render_commit_plan failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_render_cursor_moves {
        run_render_cursor_move_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
            false,
        )
        .with_context(|| format!("bench_render_cursor_move failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_render_application_cursor_moves {
        run_render_cursor_move_benchmark(
            &engine,
            session.id,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
            true,
        )
        .with_context(|| {
            format!(
                "bench_render_application_cursor_move failed for session {}",
                session.id
            )
        })?;
    }

    if let Some(rounds) = args.bench_focus_switches {
        run_focus_switch_benchmark(&engine, session.id, args.cols, args.rows, rounds)
            .with_context(|| format!("bench_focus_switch failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_session_create {
        run_session_create_benchmark(&engine, args.cols, args.rows, rounds)
            .with_context(|| format!("bench_session_create failed for session {}", session.id))?;
    }

    if let Some(rounds) = args.bench_session_ready {
        run_session_ready_benchmark(
            &engine,
            args.cols,
            args.rows,
            rounds,
            Duration::from_millis(args.poll_ms),
            Duration::from_millis(args.timeout_ms),
        )
        .with_context(|| format!("bench_session_ready failed for session {}", session.id))?;
    }

    if let Some(input) = args.write {
        engine.write_input(session.id, input.as_str())?;
    }

    if let Some(input) = args.paste {
        engine.paste_input(session.id, input.as_str())?;
    }

    std::thread::sleep(Duration::from_millis(args.wait_ms));

    let session = engine.get_session(session.id)?;
    let screen = engine.read_screen(session.id)?;
    let render_frame = engine.read_render_frame(session.id, None)?;
    let render_delta = engine.read_render_frame(session.id, Some(render_frame.revision))?;
    let render_draw_plan = engine.read_render_draw_plan(session.id, None)?;
    let render_draw_delta =
        engine.read_render_draw_plan(session.id, Some(render_frame.revision))?;
    let render_geometry_plan =
        render_draw_plan.to_geometry_plan(unterm_engine::RenderCellMetrics {
            cell_width_px: 8,
            cell_height_px: 16,
        });
    let render_submission_plan = render_geometry_plan.to_submission_plan();
    let mut render_consumer_state = unterm_engine::RenderConsumerState::new();
    let render_commit_plan = engine.read_render_commit_plan(
        session.id,
        unterm_engine::RenderCellMetrics {
            cell_width_px: 8,
            cell_height_px: 16,
        },
        &mut render_consumer_state,
    )?;
    let activity = engine.activity(session.id)?;
    let health = engine.health()?;
    let raw_bytes = engine.debug_output(session.id)?.len();
    let visible_text = engine.read_visible_text(session.id)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "session": session,
                "screen": screen,
                "render_frame": render_frame,
                "render_delta": render_delta,
                "render_draw_plan": render_draw_plan,
                "render_draw_delta": render_draw_delta,
                "render_geometry_plan": render_geometry_plan,
                "render_submission_plan": render_submission_plan,
                "render_commit_plan": render_commit_plan,
                "activity": activity,
                "health": health,
                "raw_bytes": raw_bytes,
                "visible_text": visible_text,
            }))?
        );
    } else {
        let dead_reason = session.dead_reason.as_deref().unwrap_or("none");
        println!(
            "session id={} cols={} rows={} dead={} dead_reason={} cursor=({}, {}) raw_bytes={}",
            session.id,
            screen.cols,
            screen.rows,
            session.is_dead,
            dead_reason,
            screen.cursor.x,
            screen.cursor.y,
            raw_bytes
        );
        println!(
            "render_frame revision={} full={} dirty_rows={:?} lines={} render_delta_lines={}",
            render_frame.revision,
            render_frame.full,
            render_frame.dirty_rows,
            render_frame.lines.len(),
            render_delta.lines.len()
        );
        if let Some(process) = activity.process.as_ref() {
            let root_pid = process
                .root_pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "none".to_string());
            let foreground_pid = process
                .foreground_pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "none".to_string());
            let detected_agent = process.detected_agent.as_deref().unwrap_or("none");
            let foreground_cwd = process.foreground_cwd.as_deref().unwrap_or("none");
            let root_cwd = process.root_cwd.as_deref().unwrap_or("none");
            println!(
                "activity_process foreground={} foreground_pid={} foreground_cwd={} root={} root_pid={} root_cwd={} child_count={} detected_agent={}",
                process.foreground_process,
                foreground_pid,
                foreground_cwd,
                process.root_process,
                root_pid,
                root_cwd,
                process.child_count,
                detected_agent
            );
        } else {
            println!(
                "activity_process foreground={} foreground_pid=none foreground_cwd=none root=none root_pid=none root_cwd=none child_count=0 detected_agent=none",
                activity.foreground_process
            );
        }
        if let Some(io) = health.io.as_ref() {
            println!(
                "health_io input_writes={} input_bytes={} output_chunks={} output_bytes={} paste_count={} paste_text_bytes={} screen_reads={} viewport_scrolls={}",
                io.input_writes,
                io.input_bytes,
                io.output_chunks,
                io.output_bytes,
                io.paste_count,
                io.paste_text_bytes,
                io.screen_reads,
                io.viewport_scrolls
            );
        }
        if let Some(lifecycle) = health.lifecycle.as_ref() {
            let last_dead_reason = lifecycle.last_dead_reason.as_deref().unwrap_or("none");
            println!(
                "health_lifecycle live_sessions={} dead_sessions={} total_created={} total_destroyed={} total_marked_dead={} last_dead_reason={}",
                lifecycle.live_sessions,
                lifecycle.dead_sessions,
                lifecycle.total_created,
                lifecycle.total_destroyed,
                lifecycle.total_marked_dead,
                last_dead_reason
            );
        }
        if let Some(pump) = health.runtime_pump.as_ref() {
            println!(
                "health_runtime_pump drain_calls={} dispatched_commands={} dispatched_lifecycle={} dispatched_input={} dispatched_render={} dispatched_screen={} dispatched_background={} waited_for_response={} completed_without_wait={} total_dispatch_us={} max_dispatch_us={} total_drain_us={} max_drain_us={}",
                pump.drain_calls,
                pump.dispatched_commands,
                pump.dispatched_lifecycle_commands,
                pump.dispatched_input_commands,
                pump.dispatched_render_commands,
                pump.dispatched_screen_commands,
                pump.dispatched_background_commands,
                pump.waited_for_response,
                pump.completed_without_wait,
                pump.total_dispatch_elapsed_micros,
                pump.max_dispatch_elapsed_micros,
                pump.total_drain_elapsed_micros,
                pump.max_drain_elapsed_micros
            );
        }
        println!("{visible_text}");
    }
    engine.destroy_session(session.id)?;
    Ok(())
}
