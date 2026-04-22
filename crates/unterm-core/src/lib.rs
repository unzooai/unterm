pub mod grid;
pub mod term;
pub mod pty;
pub mod session;
pub mod mcp;
pub mod screen;
pub mod orchestrate;
pub mod proxy;

use std::sync::Arc;
use tracing::info;

/// 启动内嵌 MCP Server（在 tokio 任务中运行）
/// 返回 (JoinHandle, auth_token)
pub fn spawn_mcp_server() -> (tokio::task::JoinHandle<()>, String) {
    let session_manager = Arc::new(session::SessionManager::new());
    let router = Arc::new(mcp::McpRouter::new(session_manager.clone()));
    let server = mcp::IpcServer::new(router);
    let token = server.auth_token().to_string();

    info!("内嵌 MCP Server 启动中...");

    let handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            tracing::error!("内嵌 MCP Server 错误: {}", e);
        }
    });

    (handle, token)
}
