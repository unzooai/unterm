//! unterm: CLI 工具
//!
//! MCP client 的薄封装，将子命令映射到 MCP tool 调用。
//!
//! 用法：
//!   unterm session list
//!   unterm session create --name "dev" --cwd "/path/to/project"
//!   unterm exec -s <session_id> "npm test"
//!   unterm screen read -s <session_id>
//!   unterm proxy status
//!   unterm proxy switch hk-01
//!   unterm workspace save my-project
//!   unterm workspace restore my-project

use anyhow::Result;

fn main() -> Result<()> {
    // TODO: 解析命令行参数
    // TODO: 连接 unterm-core daemon (IPC)
    // TODO: 将子命令转换为 MCP JSON-RPC 请求
    // TODO: 发送请求，接收响应
    // TODO: 格式化输出

    println!("unterm CLI - Unzoo Terminal");
    println!("usage: unterm <command> [options]");

    Ok(())
}
