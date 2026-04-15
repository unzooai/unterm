//! unterm-core: Unterm daemon 进程
//!
//! 职责：
//! - PTY 进程池管理
//! - Session 生命周期（创建、销毁、持久化）
//! - MCP Server（JSON-RPC over IPC）
//! - 内置代理引擎（clash-rs）
//! - 操作审计日志
//! - 安全策略执行

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

    info!("unterm-core starting...");

    // TODO: 加载配置
    // TODO: 启动内置代理
    // TODO: 启动 MCP Server (IPC)
    // TODO: 等待连接

    info!("unterm-core ready");

    // 保持 daemon 运行
    tokio::signal::ctrl_c().await?;
    info!("unterm-core shutting down...");

    Ok(())
}
