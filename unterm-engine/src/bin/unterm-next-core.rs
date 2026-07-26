use anyhow::{bail, Context, Result};
use portable_pty::CommandBuilder;
use std::time::{Duration, Instant};
use unterm_engine::{next_core, CreateSessionRequest, InputEngine, ScreenEngine, SessionEngine};

struct Args {
    cols: usize,
    rows: usize,
    wait_ms: u64,
    write: Option<String>,
    cwd: Option<String>,
    command: Option<Vec<String>>,
    bench_echo: Option<usize>,
    bench_flood_lines: Option<usize>,
    bench_scrollback_lines: Option<usize>,
    bench_paste_kb: Option<usize>,
    bench_dual_agent_lines: Option<usize>,
    bench_screen_read_lines: Option<usize>,
    poll_ms: u64,
    timeout_ms: u64,
}

fn parse_args() -> Result<Args> {
    let mut args = std::env::args().skip(1);
    let mut parsed = Args {
        cols: 100,
        rows: 30,
        wait_ms: 250,
        write: None,
        cwd: None,
        command: None,
        bench_echo: None,
        bench_flood_lines: None,
        bench_scrollback_lines: None,
        bench_paste_kb: None,
        bench_dual_agent_lines: None,
        bench_screen_read_lines: None,
        poll_ms: 5,
        timeout_ms: 5000,
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
            "--bench-paste-kb" => {
                parsed.bench_paste_kb = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--bench-paste-kb requires a value"))?
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
            "--bench-screen-read-lines" => {
                parsed.bench_screen_read_lines = Some(
                    args.next()
                        .ok_or_else(|| {
                            anyhow::anyhow!("--bench-screen-read-lines requires a value")
                        })?
                        .parse()?,
                );
            }
            "--write" => {
                parsed.write = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--write requires a value"))?,
                );
            }
            "--cwd" => {
                parsed.cwd = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--cwd requires a value"))?,
                );
            }
            "--help" | "-h" => {
                println!(
                    "Usage: unterm-next-core [--cols N] [--rows N] [--wait-ms N] [--poll-ms N] [--timeout-ms N] [--bench-echo N] [--bench-flood-lines N] [--bench-scrollback-lines N] [--bench-paste-kb N] [--bench-dual-agent-lines N] [--bench-screen-read-lines N] [--cwd PATH] [--write TEXT] [-- COMMAND [ARG...]]"
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

fn cmd_session(cols: usize, rows: usize) -> CreateSessionRequest {
    CreateSessionRequest {
        cols,
        rows,
        command_dir: None,
        command: Some(CommandBuilder::new("cmd.exe")),
        env: Vec::new(),
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

fn main() -> Result<()> {
    let args = parse_args()?;
    let engine = next_core();
    let session = engine.create_session(CreateSessionRequest {
        cols: args.cols,
        rows: args.rows,
        command_dir: args.cwd,
        command: command_builder(args.command),
        env: Vec::new(),
    })?;

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

    if let Some(input) = args.write {
        engine.write_input(session.id, input.as_str())?;
    }

    std::thread::sleep(Duration::from_millis(args.wait_ms));

    let session = engine.get_session(session.id)?;
    let screen = engine.read_screen(session.id)?;
    println!(
        "session id={} cols={} rows={} dead={} cursor=({}, {}) raw_bytes={}",
        session.id,
        screen.cols,
        screen.rows,
        session.is_dead,
        screen.cursor.x,
        screen.cursor.y,
        engine.debug_output(session.id)?.len()
    );
    println!("{}", engine.read_visible_text(session.id)?);
    engine.destroy_session(session.id)?;
    Ok(())
}
