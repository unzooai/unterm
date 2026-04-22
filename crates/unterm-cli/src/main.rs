//! unterm: CLI 工具
//! MCP client 的薄封装，将子命令映射到 MCP tool 调用。

mod client;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;

rust_i18n::i18n!("locales", fallback = "en");

#[derive(Parser)]
#[command(name = "unterm", version, about = "Unterm — AI-native super workstation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 管理终端会话
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// 在会话中执行命令
    Exec {
        /// Session ID
        session_id: String,
        /// 要执行的命令
        command: String,
        /// 超时时间（毫秒）
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// 向会话发送原始输入
    Send {
        session_id: String,
        input: String,
    },
    /// 向会话发送信号
    Signal {
        session_id: String,
        /// 信号名（SIGINT, SIGTERM, SIGKILL）
        signal: String,
    },
    /// 读取终端屏幕
    Screen {
        #[command(subcommand)]
        action: ScreenAction,
    },
    /// AI agent 编排调度
    Orchestrate {
        #[command(subcommand)]
        action: OrchestrateAction,
    },
    /// 代理管理
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },
    /// 工作区快照
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    /// 截图与剪贴板
    Capture {
        #[command(subcommand)]
        action: CaptureAction,
    },
    /// 执行命令并等待结果（AI 模式）
    Run {
        session_id: String,
        command: String,
        #[arg(long, default_value = "30000")]
        timeout: u64,
    },
    /// 取消正在运行的命令
    Cancel {
        session_id: String,
    },
    /// 系统信息
    System,
    /// 命令策略管理
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
    },
    /// 审计日志
    Audit {
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u32,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// 列出所有活跃会话
    List,
    /// 创建新会话
    Create {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long)]
        shell: Option<String>,
    },
    /// 销毁会话
    Destroy { session_id: String },
    /// 调整会话尺寸
    Resize {
        session_id: String,
        #[arg(long)]
        cols: u16,
        #[arg(long)]
        rows: u16,
    },
    /// 查看会话状态
    Status { session_id: String },
    /// 查看会话历史
    History {
        session_id: String,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// 检查会话是否空闲
    Idle { session_id: String },
    /// 获取当前工作目录
    Cwd { session_id: String },
    /// 读取环境变量
    Env { session_id: String, name: String },
    /// 设置环境变量
    SetEnv { session_id: String, name: String, value: String },
}

#[derive(Subcommand)]
enum ScreenAction {
    /// 读取屏幕内容
    Read {
        session_id: String,
        #[arg(long)]
        lines: Option<u32>,
    },
    /// 获取光标位置
    Cursor { session_id: String },
    /// 读取滚动缓冲区
    Scroll {
        session_id: String,
        #[arg(long)]
        offset: u32,
        #[arg(long)]
        count: u32,
    },
    /// 读取屏幕纯文本
    Text { session_id: String },
    /// 搜索屏幕内容
    Search {
        session_id: String,
        pattern: String,
        #[arg(long, default_value = "50")]
        max_results: u64,
    },
}

#[derive(Subcommand)]
enum OrchestrateAction {
    /// 启动新 AI agent
    Launch {
        command: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
    },
    /// 向多个会话广播命令
    Broadcast {
        command: String,
        #[arg(long, value_delimiter = ',')]
        sessions: Vec<String>,
    },
    /// 等待输出匹配模式
    Wait {
        session_id: String,
        pattern: String,
        #[arg(long)]
        timeout: Option<u64>,
    },
}

#[derive(Subcommand)]
enum ProxyAction {
    /// 查看代理状态
    Status,
    /// 列出所有节点
    Nodes,
    /// 切换节点
    Switch { node_name: String },
    /// 测速
    Speedtest { node_name: Option<String> },
}

#[derive(Subcommand)]
enum WorkspaceAction {
    /// 保存工作区快照
    Save { name: String },
    /// 恢复工作区快照
    Restore { name: String },
    /// 列出已保存的工作区
    List,
}

#[derive(Subcommand)]
enum CaptureAction {
    /// 截取整个屏幕
    Screen,
    /// 截取指定窗口
    Window {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        pid: Option<u32>,
    },
    /// 交互式框选截图
    Select,
    /// 读取剪贴板
    Clipboard,
}

#[derive(Subcommand)]
enum PolicyAction {
    /// 检查命令是否被允许
    Check { command: String },
    /// 设置策略（JSON）
    Set {
        /// JSON 格式: {"enabled":true,"blocked_patterns":["rm -rf"],"allowed_patterns":[]}
        json: String,
    },
}

fn detect_locale() {
    let locale = std::env::var("UNTERM_LOCALE")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    if locale.starts_with("zh") {
        rust_i18n::set_locale("zh-CN");
    } else {
        rust_i18n::set_locale("en");
    }
}

fn main() -> Result<()> {
    // Windows 控制台设为 UTF-8，避免 GBK 乱码
    #[cfg(target_os = "windows")]
    unsafe {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
            fn SetConsoleCP(wCodePageID: u32) -> i32;
        }
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }

    detect_locale();
    let cli = Cli::parse();

    match cli.command {
        Commands::Session { action } => match action {
            SessionAction::List => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("session.list", json!({}))?;
                if result.is_null() || result.as_array().is_some_and(|a| a.is_empty()) {
                    println!("{}", rust_i18n::t!("messages.no_sessions"));
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            SessionAction::Create { name, cwd, shell } => {
                let mut client = client::McpClient::connect()?;
                let mut params = json!({});
                if let Some(name) = name {
                    params["name"] = json!(name);
                }
                if let Some(cwd) = cwd {
                    params["cwd"] = json!(cwd);
                }
                if let Some(shell) = shell {
                    params["shell"] = json!(shell);
                }
                let result = client.call("session.create", params)?;
                let id = result
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                println!("{}", rust_i18n::t!("messages.session_created", id = id));
            }
            SessionAction::Destroy { session_id } => {
                let mut client = client::McpClient::connect()?;
                client.call("session.destroy", json!({ "session_id": session_id }))?;
                println!(
                    "{}",
                    rust_i18n::t!("messages.session_destroyed", id = &session_id)
                );
            }
            SessionAction::Resize {
                session_id,
                cols,
                rows,
            } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call(
                    "session.resize",
                    json!({ "session_id": session_id, "cols": cols, "rows": rows }),
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionAction::Status { session_id } => {
                let mut client = client::McpClient::connect()?;
                let result =
                    client.call("session.status", json!({ "session_id": session_id }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionAction::History {
                session_id,
                since,
                limit,
            } => {
                let mut client = client::McpClient::connect()?;
                let mut params = json!({ "session_id": session_id });
                if let Some(since) = since {
                    params["since"] = json!(since);
                }
                if let Some(limit) = limit {
                    params["limit"] = json!(limit);
                }
                let result = client.call("session.history", params)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionAction::Idle { session_id } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("session.idle", json!({ "session_id": session_id }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionAction::Cwd { session_id } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("session.cwd", json!({ "session_id": session_id }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionAction::Env { session_id, name } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("session.env", json!({ "session_id": session_id, "name": name }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionAction::SetEnv { session_id, name, value } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("session.set_env", json!({ "session_id": session_id, "name": name, "value": value }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Commands::Exec {
            session_id,
            command,
            timeout,
        } => {
            let mut client = client::McpClient::connect()?;
            let mut params = json!({ "session_id": session_id, "command": command });
            if let Some(timeout) = timeout {
                params["timeout_ms"] = json!(timeout);
            }
            let result = client.call("exec.run", params)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Send { session_id, input } => {
            let mut client = client::McpClient::connect()?;
            client.call("exec.send", json!({ "session_id": session_id, "input": input }))?;
        }
        Commands::Signal {
            session_id,
            signal,
        } => {
            let mut client = client::McpClient::connect()?;
            let result = client.call(
                "signal.send",
                json!({ "session_id": session_id, "signal": signal }),
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Screen { action } => match action {
            ScreenAction::Read { session_id, lines } => {
                let mut client = client::McpClient::connect()?;
                let mut params = json!({ "session_id": session_id });
                if let Some(lines) = lines {
                    params["lines"] = json!(lines);
                }
                let result = client.call("screen.read", params)?;
                // 直接输出屏幕文本内容
                if let Some(text) = result.as_str() {
                    println!("{}", text);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            ScreenAction::Cursor { session_id } => {
                let mut client = client::McpClient::connect()?;
                let result =
                    client.call("screen.cursor", json!({ "session_id": session_id }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScreenAction::Scroll {
                session_id,
                offset,
                count,
            } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call(
                    "screen.scroll",
                    json!({ "session_id": session_id, "offset": offset, "count": count }),
                )?;
                if let Some(text) = result.as_str() {
                    println!("{}", text);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            ScreenAction::Text { session_id } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("screen.text", json!({ "session_id": session_id }))?;
                // 输出纯文本行
                if let Some(lines) = result.get("lines").and_then(|v| v.as_array()) {
                    for line in lines {
                        println!("{}", line.as_str().unwrap_or(""));
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            ScreenAction::Search { session_id, pattern, max_results } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call(
                    "screen.search",
                    json!({ "session_id": session_id, "pattern": pattern, "max_results": max_results }),
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Commands::Orchestrate { action } => match action {
            OrchestrateAction::Launch { command, name, cwd } => {
                let mut client = client::McpClient::connect()?;
                let mut params = json!({ "command": command });
                if let Some(name) = name {
                    params["name"] = json!(name);
                }
                if let Some(cwd) = cwd {
                    params["cwd"] = json!(cwd);
                }
                let result = client.call("orchestrate.launch", params)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            OrchestrateAction::Broadcast {
                command,
                sessions,
            } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call(
                    "orchestrate.broadcast",
                    json!({ "command": command, "sessions": sessions }),
                )?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            OrchestrateAction::Wait {
                session_id,
                pattern,
                timeout,
            } => {
                let mut client = client::McpClient::connect()?;
                let mut params = json!({ "session_id": session_id, "pattern": pattern });
                if let Some(timeout) = timeout {
                    params["timeout_ms"] = json!(timeout);
                }
                let result = client.call("orchestrate.wait", params)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Commands::Proxy { action } => match action {
            ProxyAction::Status => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("proxy.status", json!({}))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ProxyAction::Nodes => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("proxy.nodes", json!({}))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ProxyAction::Switch { node_name } => {
                let mut client = client::McpClient::connect()?;
                let result =
                    client.call("proxy.switch", json!({ "node_name": node_name }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ProxyAction::Speedtest { node_name } => {
                let mut client = client::McpClient::connect()?;
                let mut params = json!({});
                if let Some(node_name) = node_name {
                    params["node_name"] = json!(node_name);
                }
                let result = client.call("proxy.speedtest", params)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Commands::Workspace { action } => match action {
            WorkspaceAction::Save { name } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("workspace.save", json!({ "name": name }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            WorkspaceAction::Restore { name } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("workspace.restore", json!({ "name": name }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            WorkspaceAction::List => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("workspace.list", json!({}))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Commands::Capture { action } => match action {
            CaptureAction::Screen => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("capture.screen", json!({}))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            CaptureAction::Window { title, pid } => {
                let mut client = client::McpClient::connect()?;
                let mut params = json!({});
                if let Some(title) = title {
                    params["title"] = json!(title);
                }
                if let Some(pid) = pid {
                    params["pid"] = json!(pid);
                }
                let result = client.call("capture.window", params)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            CaptureAction::Select => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("capture.select", json!({}))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            CaptureAction::Clipboard => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("capture.clipboard", json!({}))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Commands::Run { session_id, command, timeout } => {
            let mut client = client::McpClient::connect()?;
            let result = client.call(
                "exec.run_wait",
                json!({ "session_id": session_id, "command": command, "timeout_ms": timeout }),
            )?;
            // 直接输出命令结果
            if let Some(output) = result.get("output").and_then(|v| v.as_str()) {
                println!("{}", output);
            } else {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Commands::Cancel { session_id } => {
            let mut client = client::McpClient::connect()?;
            let result = client.call("exec.cancel", json!({ "session_id": session_id }))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::System => {
            let mut client = client::McpClient::connect()?;
            let result = client.call("system.info", json!({}))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Policy { action } => match action {
            PolicyAction::Check { command } => {
                let mut client = client::McpClient::connect()?;
                let result = client.call("policy.check", json!({ "command": command }))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PolicyAction::Set { json: policy_json } => {
                let mut client = client::McpClient::connect()?;
                let params: serde_json::Value = serde_json::from_str(&policy_json)?;
                let result = client.call("policy.set", params)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Commands::Audit { session_id, limit } => {
            let mut client = client::McpClient::connect()?;
            let mut params = json!({ "limit": limit });
            if let Some(sid) = session_id {
                params["session_id"] = json!(sid);
            }
            let result = client.call("session.audit_log", params)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
