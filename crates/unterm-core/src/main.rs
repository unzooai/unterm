use std::sync::Arc;
use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("unterm_core=debug")
        .init();

    info!("unterm-core 正在启动...");

    // 初始化 Session 管理器
    let session_manager = Arc::new(unterm_core::session::SessionManager::new());

    // 初始化代理管理器
    let proxy_config = unterm_core::proxy::ProxyConfig::default();
    let _proxy_manager = unterm_core::proxy::ProxyManager::new(proxy_config);

    // 启动 MCP Server
    let router = Arc::new(unterm_core::mcp::McpRouter::new(session_manager.clone()));
    let server = unterm_core::mcp::IpcServer::new(router);

    info!("unterm-core 已就绪");

    // MCP Server 在后台运行，Ctrl+C 退出
    tokio::select! {
        result = server.start() => {
            if let Err(e) = result {
                tracing::error!("MCP Server 错误: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("unterm-core 正在关闭...");
        }
    }

    Ok(())
}
