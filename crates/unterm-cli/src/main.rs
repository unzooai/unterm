//! unterm: CLI 工具
//! MCP client 的薄封装，将子命令映射到 MCP tool 调用。

use clap::{Parser, Subcommand};

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

fn main() {
    detect_locale();
    let cli = Cli::parse();

    match cli.command {
        Commands::Session { action } => match action {
            SessionAction::List => println!("{}", rust_i18n::t!("messages.not_implemented")),
            SessionAction::Create { name, cwd, shell } => {
                println!("session create: name={:?}, cwd={:?}, shell={:?}", name, cwd, shell);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            SessionAction::Destroy { session_id } => {
                println!("session destroy: {}", session_id);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            SessionAction::Resize { session_id, cols, rows } => {
                println!("session resize: {} {}x{}", session_id, cols, rows);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            SessionAction::Status { session_id } => {
                println!("session status: {}", session_id);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            SessionAction::History { session_id, since, limit } => {
                println!("session history: {} since={:?} limit={:?}", session_id, since, limit);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
        },
        Commands::Exec { session_id, command, timeout } => {
            println!("exec: session={} cmd={} timeout={:?}", session_id, command, timeout);
            println!("{}", rust_i18n::t!("messages.not_implemented"));
        }
        Commands::Send { session_id, input } => {
            println!("send: session={} input={}", session_id, input);
            println!("{}", rust_i18n::t!("messages.not_implemented"));
        }
        Commands::Signal { session_id, signal } => {
            println!("signal: session={} signal={}", session_id, signal);
            println!("{}", rust_i18n::t!("messages.not_implemented"));
        }
        Commands::Screen { action } => match action {
            ScreenAction::Read { session_id, lines } => {
                println!("screen read: {} lines={:?}", session_id, lines);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            ScreenAction::Cursor { session_id } => {
                println!("screen cursor: {}", session_id);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            ScreenAction::Scroll { session_id, offset, count } => {
                println!("screen scroll: {} offset={} count={}", session_id, offset, count);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
        },
        Commands::Orchestrate { action } => match action {
            OrchestrateAction::Launch { command, name, cwd } => {
                println!("orchestrate launch: {} name={:?} cwd={:?}", command, name, cwd);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            OrchestrateAction::Broadcast { command, sessions } => {
                println!("orchestrate broadcast: {} sessions={:?}", command, sessions);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            OrchestrateAction::Wait { session_id, pattern, timeout } => {
                println!("orchestrate wait: {} pattern={} timeout={:?}", session_id, pattern, timeout);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
        },
        Commands::Proxy { action } => match action {
            ProxyAction::Status => println!("{}", rust_i18n::t!("messages.not_implemented")),
            ProxyAction::Nodes => println!("{}", rust_i18n::t!("messages.not_implemented")),
            ProxyAction::Switch { node_name } => {
                println!("proxy switch: {}", node_name);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            ProxyAction::Speedtest { node_name } => {
                println!("proxy speedtest: {:?}", node_name);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
        },
        Commands::Workspace { action } => match action {
            WorkspaceAction::Save { name } => {
                println!("workspace save: {}", name);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            WorkspaceAction::Restore { name } => {
                println!("workspace restore: {}", name);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            WorkspaceAction::List => println!("{}", rust_i18n::t!("messages.not_implemented")),
        },
        Commands::Capture { action } => match action {
            CaptureAction::Screen => println!("{}", rust_i18n::t!("messages.not_implemented")),
            CaptureAction::Window { title, pid } => {
                println!("capture window: title={:?} pid={:?}", title, pid);
                println!("{}", rust_i18n::t!("messages.not_implemented"));
            }
            CaptureAction::Select => println!("{}", rust_i18n::t!("messages.not_implemented")),
            CaptureAction::Clipboard => println!("{}", rust_i18n::t!("messages.not_implemented")),
        },
    }
}
