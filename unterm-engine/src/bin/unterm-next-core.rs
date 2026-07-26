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
                    "Usage: unterm-next-core [--cols N] [--rows N] [--wait-ms N] [--poll-ms N] [--timeout-ms N] [--bench-echo N] [--cwd PATH] [--write TEXT] [-- COMMAND [ARG...]]"
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

fn percentile(sorted: &[u128], percentile: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }

    let rank = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
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
    let min = sorted[0];
    let p50 = percentile(&sorted, 0.50);
    let p95 = percentile(&sorted, 0.95);
    let max = *sorted.last().unwrap_or(&0);
    println!(
        "bench_echo rounds={} min_us={} p50_us={} p95_us={} max_us={}",
        rounds, min, p50, p95, max
    );
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
