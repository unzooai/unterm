mod pty;
mod session;
mod mcp;
mod screen;
mod orchestrate;
mod proxy;

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("unterm_core=debug")
        .init();

    info!("unterm-core 正在启动...");

    let _session_manager = session::SessionManager::new();

    info!("unterm-core 已就绪");

    tokio::signal::ctrl_c().await?;
    info!("unterm-core 正在关闭...");

    Ok(())
}
