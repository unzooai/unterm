use anyhow::Result;
use unterm_core::{clear_discovery, try_acquire_instance_lock, write_discovery, CoreServer};

fn main() -> Result<()> {
    let args = CoreArgs::parse(std::env::args_os().skip(1))?;
    if args.version {
        println!("unterm-core {}", unterm_protocol::PRODUCT_VERSION);
        return Ok(());
    }
    if args.help {
        print_help();
        return Ok(());
    }

    // Single-instance gate: concurrent GUI/CLI launches may race to
    // spawn a core; only the lock holder may bind and publish
    // discovery. Losers exit quietly and their parent keeps polling
    // the winner's discovery record.
    let Some(_lock) = try_acquire_instance_lock()? else {
        eprintln!("unterm-core already running for this user; exiting");
        return Ok(());
    };
    let token =
        std::env::var("UNTERM_CORE_TOKEN").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
    let server = CoreServer::bind(("127.0.0.1", 0), &token)?;
    let endpoint = server.endpoint()?;

    // The agent surface enforces the user's policy — trusted agents,
    // confirmation mode, scrollback size. A Core that skipped the
    // config would judge every write by defaults instead of by what
    // the user chose in it.
    let (config, config_errors) = unterm_services::settings::load(None);
    for error in &config_errors {
        eprintln!("unterm-core: config line {}: {}", error.line, error.message);
    }
    unterm_services::settings::set_current(&config);
    unterm_engine::next_core::NextCoreEngine::set_new_session_scrollback_lines(
        unterm_services::settings::scrollback_lines(&config),
    );

    // The agent surface lives here, not in any GUI: sessions belong to
    // this process, so the 103-method MCP server drives the local
    // engine directly. Discovery via our own record; a GUI's MCP
    // server (transitional) keeps server.json to itself.
    unterm_engine::install_next_core_provider();
    // Register the window-facing half of the surface before the MCP
    // server opens. It answers "is there a window?" with the truth at
    // the moment it is asked -- no window attached and it degrades
    // exactly as a headless surface always did, so this is safe to
    // install whether or not a GUI ever shows up.
    unterm_engine::set_mcp_host(&unterm_core::RemoteMcpHost);
    let mcp_port = match unterm_mcp::start_headless_mcp_server(&token) {
        Ok(port) => Some(port),
        Err(err) => {
            eprintln!("unterm-core: MCP surface unavailable: {err:#}");
            None
        }
    };
    match unterm_services::bridge_registry::request_incompatible_drains() {
        Ok(0) | Err(_) => {}
        Ok(count) => eprintln!("unterm-core: requested drain for {count} incompatible bridge(s)"),
    }

    // Two installs share one state directory: whichever starts first owns the
    // sessions, and two versions can migrate the same database differently.
    // Said at startup, once, because it explains a class of failure nobody
    // would otherwise connect to having installed Unterm twice.
    let installs = unterm_services::install::survey();
    for conflict in unterm_services::install::conflicts(&installs) {
        eprintln!("unterm-core: {} — {}", conflict.reason, conflict.advice);
    }

    write_discovery(&endpoint.to_string(), &token, mcp_port, server.started_at())?;
    eprintln!(
        "unterm-core ready endpoint={} mcp_port={:?} pid={}",
        endpoint,
        mcp_port,
        std::process::id()
    );
    // The system will tell us before it takes this process away — SIGTERM on
    // macOS and Linux, a console control event on Windows. The Core is where
    // that has to be heard: it owns the sessions and the task store, so it is
    // the process whose sudden death costs something.
    unterm_services::power::install();
    std::thread::Builder::new()
        .name("core-power-watch".into())
        .spawn(|| loop {
            if let Some(reason) = unterm_services::power::should_stop() {
                // Seconds, not minutes. Say why we are going, take the
                // discovery record with us so nobody connects to a corpse,
                // and leave. Finishing work here is how a process gets killed
                // halfway through finishing it.
                eprintln!("unterm-core stopping: {reason}");
                let _ = clear_discovery();
                std::process::exit(0);
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        })
        .ok();

    let result = server.run();
    let _ = clear_discovery();
    result
}

struct CoreArgs {
    help: bool,
    version: bool,
}

impl CoreArgs {
    fn parse(arguments: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self> {
        let mut parsed = Self {
            help: false,
            version: false,
        };
        for argument in arguments {
            match argument.to_string_lossy().as_ref() {
                "--help" | "-h" => parsed.help = true,
                "--version" | "-V" => parsed.version = true,
                "--headless" => {}
                other => anyhow::bail!("unknown unterm-core argument {other:?}"),
            }
        }
        Ok(parsed)
    }
}

fn print_help() {
    println!(
        "unterm-core {}\n\nUSAGE:\n    unterm-core [--headless]\n\nOPTIONS:\n    --headless    Run without creating any GUI window (the default for unterm-core)\n    -V, --version Print version and exit before initialization\n    -h, --help    Print help and exit before initialization",
        unterm_protocol::PRODUCT_VERSION
    );
}
