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
    write_discovery(&endpoint.to_string(), &token)?;
    eprintln!(
        "unterm-core ready endpoint={} pid={}",
        endpoint,
        std::process::id()
    );
    let result = server.run();
    let _ = clear_discovery();
    result
}
