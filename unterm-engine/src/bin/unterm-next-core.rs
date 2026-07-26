use anyhow::{bail, Result};
use portable_pty::CommandBuilder;
use std::time::Duration;
use unterm_engine::{next_core, CreateSessionRequest, InputEngine, ScreenEngine, SessionEngine};

struct Args {
    cols: usize,
    rows: usize,
    wait_ms: u64,
    write: Option<String>,
    cwd: Option<String>,
    command: Option<Vec<String>>,
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
                    "Usage: unterm-next-core [--cols N] [--rows N] [--wait-ms N] [--cwd PATH] [--write TEXT] [-- COMMAND [ARG...]]"
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

fn main() -> Result<()> {
    let args = parse_args()?;
    let engine = next_core();
    let session = engine.create_session(CreateSessionRequest {
        cols: args.cols,
        rows: args.rows,
        command_dir: args.cwd,
        command: command_builder(args.command),
    })?;

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
