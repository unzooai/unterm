use anyhow::Result;
use unterm_core::{clear_discovery, try_acquire_instance_lock, write_discovery, CoreServer};

fn main() -> Result<()> {
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

    // The agent surface lives here, not in any GUI: sessions belong to
    // this process, so the 103-method MCP server drives the local
    // engine directly. Discovery via our own record; a GUI's MCP
    // server (transitional) keeps server.json to itself.
    unterm_engine::install_next_core_provider();
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

    write_discovery(&endpoint.to_string(), &token, mcp_port)?;
    eprintln!(
        "unterm-core ready endpoint={} mcp_port={:?} pid={}",
        endpoint,
        mcp_port,
        std::process::id()
    );
    let result = server.run();
    let _ = clear_discovery();
    result
}
