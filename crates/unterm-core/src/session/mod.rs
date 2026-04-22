//! Session 生命周期管理
//! 每个 session 对应一个 PTY 进程 + VT 解析器。

use std::collections::HashMap;
use std::io::Read;
use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use tracing::info;
use serde::{Serialize, Deserialize};

use unterm_proto::session::*;
use crate::pty::{PtyConfig, PtyHandle, PtyManager};
use crate::term::Terminal;

/// raw_output 缓冲区最大大小 (16MB)
const RAW_OUTPUT_MAX_SIZE: usize = 16 * 1024 * 1024;

/// 正则表达式最大长度
const MAX_REGEX_LEN: usize = 1024;

/// 安全编译正则，限制长度
fn safe_regex(pattern: &str) -> Result<regex::Regex> {
    if pattern.len() > MAX_REGEX_LEN {
        anyhow::bail!("正则表达式过长（最大 {} 字符）", MAX_REGEX_LEN);
    }
    regex::Regex::new(pattern)
        .map_err(|e| anyhow::anyhow!("无效的正则表达式: {}", e))
}

/// 用户粘贴的图片
#[derive(Clone)]
pub struct PastedImage {
    pub id: String,
    pub data_base64: String,
    pub mime_type: String,
    pub timestamp: String,
}

/// I/O 历史条目
#[derive(Clone, Serialize, Deserialize)]
pub struct IoHistoryEntry {
    pub timestamp: String,
    pub direction: String, // "input" | "output"
    pub content: String,
}

/// 工作区快照
#[derive(Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub name: String,
    pub created_at: String,
    pub sessions: Vec<WorkspaceSessionInfo>,
}

/// 工作区中保存的 session 信息
#[derive(Clone, Serialize, Deserialize)]
pub struct WorkspaceSessionInfo {
    pub name: Option<String>,
    pub shell: String,
    pub cwd: String,
}

/// 审计日志条目
#[derive(Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub method: String,
    pub session_id: Option<String>,
    pub detail: String,
    pub success: bool,
}

/// 命令策略
#[derive(Clone, Serialize, Deserialize)]
pub struct CommandPolicy {
    /// 黑名单正则（匹配则拒绝）
    pub blocked_patterns: Vec<String>,
    /// 白名单正则（非空时只允许匹配的命令）
    pub allowed_patterns: Vec<String>,
    /// 是否启用策略
    pub enabled: bool,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        Self {
            blocked_patterns: Vec::new(),
            allowed_patterns: Vec::new(),
            enabled: false,
        }
    }
}

/// Shell 类型
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ShellType {
    Pwsh,
    PowerShell,
    Cmd,
    Bash,
    Zsh,
    Sh,
    Fish,
    Unknown,
}

impl ShellType {
    pub fn detect(shell_path: &str) -> Self {
        let lower = shell_path.to_lowercase();
        if lower.contains("pwsh") { Self::Pwsh }
        else if lower.contains("powershell") { Self::PowerShell }
        else if lower.contains("cmd") { Self::Cmd }
        else if lower.contains("bash") { Self::Bash }
        else if lower.contains("zsh") { Self::Zsh }
        else if lower.contains("fish") { Self::Fish }
        else if lower.ends_with("/sh") || lower == "sh" { Self::Sh }
        else { Self::Unknown }
    }

    /// 是否为 PowerShell 系
    pub fn is_powershell(&self) -> bool {
        matches!(self, Self::Pwsh | Self::PowerShell)
    }

    /// 提示符检测正则
    pub fn prompt_pattern(&self) -> &str {
        match self {
            Self::Pwsh | Self::PowerShell => r"^PS [A-Z]:\\.*>\s*$",
            Self::Cmd => r"^[A-Z]:\\.*>$",
            Self::Bash | Self::Sh => r"[$#]\s*$",
            Self::Zsh => r"[%$#]\s*$",
            Self::Fish => r"[>❯]\s*$",
            Self::Unknown => r"[$#%>]\s*$",
        }
    }
}

/// 单个 Session
pub struct Session {
    pub info: SessionInfo,
    pub pty_handle: Mutex<PtyHandle>,
    /// PTY writer，创建时通过 take_writer 获取（与 reader 线程共享用于 DSR 响应）
    pub writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    /// 终端仿真器（Grid + VTE Parser）
    pub terminal: Arc<RwLock<Terminal>>,
    /// PTY 原始输出缓冲（供 xterm.js 等前端直接消费）
    pub raw_output: Arc<Mutex<Vec<u8>>>,
    /// 用户粘贴的图片（AI 可通过 MCP 读取）
    pub images: Mutex<Vec<PastedImage>>,
    /// I/O 历史记录
    pub history: Arc<Mutex<Vec<IoHistoryEntry>>>,
    /// Shell 类型
    pub shell_type: ShellType,
    /// 最后一次发送输入的时间
    pub last_input_time: Arc<Mutex<std::time::Instant>>,
    /// 最后一次收到输出的时间
    pub last_output_time: Arc<Mutex<std::time::Instant>>,
}

/// Session 管理器
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    pty_manager: Mutex<PtyManager>,
    /// 工作区快照
    workspaces: Mutex<Vec<WorkspaceSnapshot>>,
    /// 审计日志
    audit_log: Mutex<Vec<AuditEntry>>,
    /// 命令策略
    policy: Mutex<CommandPolicy>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            pty_manager: Mutex::new(PtyManager::new()),
            workspaces: Mutex::new(Vec::new()),
            audit_log: Mutex::new(Vec::new()),
            policy: Mutex::new(CommandPolicy::default()),
        }
    }

    /// 记录审计日志
    pub fn audit(&self, method: &str, session_id: Option<&str>, detail: &str, success: bool) {
        let mut log = self.audit_log.lock();
        log.push(AuditEntry {
            timestamp: Utc::now().to_rfc3339(),
            method: method.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            detail: detail.to_string(),
            success,
        });
        // 最多保留 5000 条
        if log.len() > 5000 {
            let excess = log.len() - 5000;
            log.drain(..excess);
        }
    }

    /// 创建新 session
    pub fn create_session(&self, req: CreateSessionRequest) -> Result<SessionInfo> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let shell = req.shell.clone().unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                // 优先 pwsh (PowerShell 7+)，回退 powershell
                std::env::var_os("PATH")
                    .and_then(|paths| {
                        if std::env::split_paths(&paths).any(|d| d.join("pwsh.exe").exists()) {
                            Some("pwsh.exe".into())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "powershell.exe".into())
            } else {
                "/bin/zsh".into()
            }
        });
        let cwd = req.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".into())
        });

        let config = PtyConfig {
            shell: Some(shell.clone()),
            cwd: Some(cwd.clone()),
            env: req.env.clone(),
            cols: 80,
            rows: 24,
            proxy_env: None,
        };

        let pty_handle = self.pty_manager.lock().create_pty(config)?;

        let shell_type = ShellType::detect(&shell);

        let info = SessionInfo {
            id: id.clone(),
            name: req.name.clone(),
            status: SessionStatus::Running,
            policy: SessionPolicy::Full,
            shell,
            cwd,
            created_at: now.clone(),
            last_activity: now,
            cols: 80,
            rows: 24,
        };

        let terminal = Arc::new(RwLock::new(Terminal::new(80, 24)));

        // 提前获取 writer（take_writer 只能调用一次）
        let writer = pty_handle.master.take_writer()
            .map_err(|e| anyhow::anyhow!("无法获取 PTY writer: {}", e))?;
        let writer = Arc::new(Mutex::new(writer));
        let raw_output: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let history: Arc<Mutex<Vec<IoHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let last_input_time = Arc::new(Mutex::new(std::time::Instant::now()));
        let last_output_time = Arc::new(Mutex::new(std::time::Instant::now()));

        // 启动 PTY 输出读取线程
        let terminal_clone = terminal.clone();
        let writer_clone = writer.clone();
        let raw_output_clone = raw_output.clone();
        let history_clone = history.clone();
        let last_output_clone = last_output_time.clone();
        let mut reader = pty_handle.master.try_clone_reader()
            .map_err(|e| anyhow::anyhow!("无法克隆 PTY reader: {}", e))?;

        let session_id_clone = id.clone();
        std::thread::spawn(move || {
            use std::io::Write;
            info!("PTY reader 线程已启动: {}", session_id_clone);
            let mut buf = [0u8; 4096];
            let mut total_bytes = 0usize;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        info!("PTY reader EOF: {} (共读取 {} 字节)", session_id_clone, total_bytes);
                        break;
                    }
                    Ok(n) => {
                        total_bytes += n;
                        if total_bytes <= n {
                            info!("PTY 首次输出 ({} 字节): {:?}", n, String::from_utf8_lossy(&buf[..n.min(200)]));
                        }
                        // 保存原始输出供 xterm.js 消费（限制缓冲区大小）
                        {
                            let mut raw = raw_output_clone.lock();
                            if raw.len() < RAW_OUTPUT_MAX_SIZE {
                                raw.extend_from_slice(&buf[..n]);
                            }
                        }
                        *last_output_clone.lock() = std::time::Instant::now();
                        // 记录输出历史（截断避免内存膨胀）
                        {
                            let content = String::from_utf8_lossy(&buf[..n]);
                            if !content.is_empty() {
                                let mut hist = history_clone.lock();
                                hist.push(IoHistoryEntry {
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    direction: "output".into(),
                                    content: if content.len() > 4096 {
                                        content[..4096].to_string()
                                    } else {
                                        content.to_string()
                                    },
                                });
                                // 最多保留 1000 条
                                if hist.len() > 1000 {
                                    let excess = hist.len() - 1000;
                                    hist.drain(..excess);
                                }
                            }
                        }
                        let mut term = terminal_clone.write();
                        term.process(&buf[..n]);
                        // 写回 DSR 等响应
                        let responses = term.take_pending_responses();
                        drop(term);
                        if !responses.is_empty() {
                            info!("写回 PTY 响应 ({} 字节): {:?}", responses.len(), String::from_utf8_lossy(&responses));
                            let mut w = writer_clone.lock();
                            let _ = w.write_all(&responses);
                            let _ = w.flush();
                        }
                    }
                    Err(e) => {
                        info!("PTY reader 错误: {} (共读取 {} 字节): {}", session_id_clone, total_bytes, e);
                        break;
                    }
                }
            }
        });

        let session = Session {
            info: info.clone(),
            pty_handle: Mutex::new(pty_handle),
            writer,
            terminal,
            raw_output,
            images: Mutex::new(Vec::new()),
            history,
            shell_type,
            last_input_time,
            last_output_time,
        };

        self.sessions.write().insert(id, session);
        info!("Session 已创建: {}", &info.id);

        Ok(info)
    }

    /// 列出所有活跃 session
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions.read().values().map(|s| s.info.clone()).collect()
    }

    /// 获取指定 session 信息
    pub fn get_session(&self, id: &str) -> Option<SessionInfo> {
        self.sessions.read().get(id).map(|s| s.info.clone())
    }

    /// 销毁 session
    pub fn destroy_session(&self, id: &str) -> Result<()> {
        if let Some(session) = self.sessions.write().remove(id) {
            session.pty_handle.lock().child.kill()?;
            info!("Session 已销毁: {}", id);
            Ok(())
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 调整 session 尺寸
    pub fn resize_session(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(id) {
            session.pty_handle.lock().resize(cols, rows)?;
            session.terminal.write().resize(cols, rows);
            session.info.cols = cols;
            session.info.rows = rows;
            Ok(())
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 向 session 发送输入
    pub fn send_input(&self, id: &str, input: &str) -> Result<()> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            use std::io::Write;
            let mut writer = session.writer.lock();
            writer.write_all(input.as_bytes())?;
            writer.flush()?;
            *session.last_input_time.lock() = std::time::Instant::now();
            // 记录输入历史
            let mut hist = session.history.lock();
            let content = if input.len() > 4096 {
                input[..4096].to_string()
            } else {
                input.to_string()
            };
            hist.push(IoHistoryEntry {
                timestamp: Utc::now().to_rfc3339(),
                direction: "input".into(),
                content,
            });
            if hist.len() > 1000 {
                let excess = hist.len() - 1000;
                                    hist.drain(..excess);
            }
            Ok(())
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 读取 session 屏幕内容（结构化 JSON）
    pub fn read_screen(&self, id: &str) -> Result<serde_json::Value> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let term = session.terminal.read();
            let grid = term.grid();
            // 只传最近 500 行 scrollback，避免传输过大
            let sb = grid.scrollback();
            let sb_start = sb.len().saturating_sub(500);
            let recent_scrollback: Vec<&Vec<crate::grid::Cell>> = sb.iter().skip(sb_start).collect();
            Ok(serde_json::json!({
                "cells": grid.visible_rows(),
                "cursor": grid.cursor,
                "cols": grid.cols(),
                "rows": grid.rows(),
                "scrollback": recent_scrollback,
                "scrollback_len": grid.scrollback_len(),
            }))
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 存储图片到 session（用户粘贴时调用）
    pub fn store_image(&self, session_id: &str, data_base64: String, mime_type: String) -> Result<String> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(session_id) {
            let image_id = Uuid::new_v4().to_string();
            let image = PastedImage {
                id: image_id.clone(),
                data_base64,
                mime_type,
                timestamp: Utc::now().to_rfc3339(),
            };
            let mut images = session.images.lock();
            images.push(image);
            // 最多保留 20 张
            if images.len() > 20 {
                images.remove(0);
            }
            info!("图片已存储: session={}, image_id={}", session_id, image_id);
            Ok(image_id)
        } else {
            anyhow::bail!("Session 未找到: {}", session_id)
        }
    }

    /// 获取 session 中的所有图片（元数据，不含 base64 数据）
    pub fn list_images(&self, session_id: &str) -> Result<Vec<serde_json::Value>> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(session_id) {
            let images = session.images.lock();
            let list: Vec<serde_json::Value> = images.iter().map(|img| {
                serde_json::json!({
                    "id": img.id,
                    "mime_type": img.mime_type,
                    "timestamp": img.timestamp,
                    "size": img.data_base64.len() * 3 / 4, // 估算原始大小
                })
            }).collect();
            Ok(list)
        } else {
            anyhow::bail!("Session 未找到: {}", session_id)
        }
    }

    /// 获取指定图片的 base64 数据（AI 读取用）
    pub fn get_image(&self, session_id: &str, image_id: &str) -> Result<Option<PastedImage>> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(session_id) {
            let images = session.images.lock();
            Ok(images.iter().find(|img| img.id == image_id).cloned())
        } else {
            anyhow::bail!("Session 未找到: {}", session_id)
        }
    }

    /// 读取并清空原始 PTY 输出（供 xterm.js 直接消费）
    pub fn read_raw_output(&self, id: &str) -> Result<Vec<u8>> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let mut buf = session.raw_output.lock();
            let data = std::mem::take(&mut *buf);
            Ok(data)
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 获取 session I/O 历史
    pub fn get_history(&self, id: &str, since: Option<&str>, limit: Option<u32>) -> Result<Vec<IoHistoryEntry>> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let hist = session.history.lock();
            let mut entries: Vec<IoHistoryEntry> = hist.iter().cloned().collect();
            // 按 since 过滤
            if let Some(since) = since {
                entries.retain(|e| e.timestamp.as_str() >= since);
            }
            // 限制条数
            if let Some(limit) = limit {
                let limit = limit as usize;
                if entries.len() > limit {
                    entries = entries[entries.len() - limit..].to_vec();
                }
            }
            Ok(entries)
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 获取光标位置
    pub fn read_cursor(&self, id: &str) -> Result<serde_json::Value> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let term = session.terminal.read();
            let cursor = &term.grid().cursor;
            Ok(serde_json::json!({
                "row": cursor.row,
                "col": cursor.col,
                "visible": cursor.visible,
            }))
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 读取滚动缓冲区指定范围
    pub fn read_scrollback(&self, id: &str, offset: u32, count: u32) -> Result<serde_json::Value> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let term = session.terminal.read();
            let grid = term.grid();
            let sb = grid.scrollback();
            let offset = offset as usize;
            let count = count as usize;
            let start = offset.min(sb.len());
            let end = (offset + count).min(sb.len());
            // 将 Cell 行转为纯文本行
            let lines: Vec<String> = sb.iter().skip(start).take(end - start).map(|row| {
                row.iter()
                    .filter(|c| !c.is_wide_continuation)
                    .map(|c| c.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            }).collect();
            Ok(serde_json::json!({
                "lines": lines,
                "offset": start,
                "count": lines.len(),
                "total": sb.len(),
            }))
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 向 session 发送信号（Windows: 仅支持 kill / Ctrl+C）
    pub fn send_signal(&self, id: &str, signal: &str) -> Result<()> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            match signal.to_uppercase().as_str() {
                "SIGINT" | "INT" | "2" => {
                    // 发送 Ctrl+C
                    use std::io::Write;
                    let mut writer = session.writer.lock();
                    writer.write_all(&[0x03])?; // ETX = Ctrl+C
                    writer.flush()?;
                }
                "SIGTERM" | "TERM" | "15" | "SIGKILL" | "KILL" | "9" => {
                    // Windows 上 SIGTERM/SIGKILL 都用 kill
                    session.pty_handle.lock().child.kill()?;
                }
                "SIGEOF" | "EOF" => {
                    // 发送 Ctrl+D
                    use std::io::Write;
                    let mut writer = session.writer.lock();
                    writer.write_all(&[0x04])?; // EOT = Ctrl+D
                    writer.flush()?;
                }
                "SIGTSTP" | "TSTP" | "20" => {
                    // 发送 Ctrl+Z
                    use std::io::Write;
                    let mut writer = session.writer.lock();
                    writer.write_all(&[0x1A])?; // SUB = Ctrl+Z
                    writer.flush()?;
                }
                _ => anyhow::bail!("不支持的信号: {}", signal),
            }
            Ok(())
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// orchestrate.launch: 创建新 session 并执行命令
    pub fn launch(&self, command: &str, name: Option<String>, cwd: Option<String>) -> Result<serde_json::Value> {
        let req = CreateSessionRequest {
            shell: None,
            cwd,
            env: None,
            name,
        };
        let info = self.create_session(req)?;
        let session_id = info.id.clone();
        // 发送命令
        self.send_input(&session_id, &format!("{}\n", command))?;
        Ok(serde_json::json!({
            "session_id": session_id,
            "command": command,
            "status": "launched",
        }))
    }

    /// orchestrate.broadcast: 向多个 session 广播命令
    pub fn broadcast(&self, command: &str, session_ids: &[String]) -> Result<serde_json::Value> {
        let mut results = Vec::new();
        for sid in session_ids {
            let ok = self.send_input(sid, &format!("{}\n", command)).is_ok();
            results.push(serde_json::json!({
                "session_id": sid,
                "sent": ok,
            }));
        }
        Ok(serde_json::json!({
            "command": command,
            "results": results,
        }))
    }

    /// orchestrate.wait: 等待 session 屏幕输出匹配指定模式
    pub fn wait_for_pattern(&self, id: &str, pattern: &str, timeout_ms: u64) -> Result<serde_json::Value> {
        let regex = safe_regex(pattern)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        loop {
            // 读取当前屏幕文本
            {
                let sessions = self.sessions.read();
                if let Some(session) = sessions.get(id) {
                    let term = session.terminal.read();
                    let grid = term.grid();
                    let rows = grid.visible_rows();
                    for row in rows {
                        let line: String = row.iter()
                            .filter(|c| !c.is_wide_continuation)
                            .map(|c| c.ch)
                            .collect();
                        if regex.is_match(&line) {
                            return Ok(serde_json::json!({
                                "matched": true,
                                "line": line.trim_end(),
                                "pattern": pattern,
                            }));
                        }
                    }
                } else {
                    anyhow::bail!("Session 未找到: {}", id);
                }
            }
            if std::time::Instant::now() >= deadline {
                return Ok(serde_json::json!({
                    "matched": false,
                    "pattern": pattern,
                    "timeout": true,
                }));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// workspace.save: 保存当前所有 session 信息为快照
    pub fn workspace_save(&self, name: &str) -> Result<serde_json::Value> {
        let sessions = self.sessions.read();
        let session_infos: Vec<WorkspaceSessionInfo> = sessions.values().map(|s| {
            WorkspaceSessionInfo {
                name: s.info.name.clone(),
                shell: s.info.shell.clone(),
                cwd: s.info.cwd.clone(),
            }
        }).collect();
        drop(sessions);

        let snapshot = WorkspaceSnapshot {
            name: name.to_string(),
            created_at: Utc::now().to_rfc3339(),
            sessions: session_infos,
        };

        let mut workspaces = self.workspaces.lock();
        // 覆盖同名快照
        workspaces.retain(|w| w.name != name);
        workspaces.push(snapshot);

        info!("工作区已保存: {}", name);
        Ok(serde_json::json!({
            "saved": true,
            "name": name,
        }))
    }

    /// workspace.restore: 恢复工作区快照（重新创建 sessions）
    pub fn workspace_restore(&self, name: &str) -> Result<serde_json::Value> {
        let snapshot = {
            let workspaces = self.workspaces.lock();
            workspaces.iter().find(|w| w.name == name).cloned()
        };
        let snapshot = match snapshot {
            Some(s) => s,
            None => anyhow::bail!("工作区未找到: {}", name),
        };

        let mut created = Vec::new();
        for s in &snapshot.sessions {
            let req = CreateSessionRequest {
                shell: Some(s.shell.clone()),
                cwd: Some(s.cwd.clone()),
                env: None,
                name: s.name.clone(),
            };
            match self.create_session(req) {
                Ok(info) => created.push(info.id),
                Err(e) => info!("恢复 session 失败: {}", e),
            }
        }

        info!("工作区已恢复: {}, 创建 {} 个 session", name, created.len());
        Ok(serde_json::json!({
            "restored": true,
            "name": name,
            "sessions": created,
        }))
    }

    /// workspace.list: 列出所有已保存的工作区
    pub fn workspace_list(&self) -> serde_json::Value {
        let workspaces = self.workspaces.lock();
        let list: Vec<serde_json::Value> = workspaces.iter().map(|w| {
            serde_json::json!({
                "name": w.name,
                "created_at": w.created_at,
                "session_count": w.sessions.len(),
            })
        }).collect();
        serde_json::json!(list)
    }

    // ════════════════════════════════════════════════════════════
    //  新增 13 个 AI 控制方法
    // ════════════════════════════════════════════════════════════

    /// 辅助：读取屏幕可见行的纯文本
    fn visible_text(&self, id: &str) -> Result<Vec<String>> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let term = session.terminal.read();
            let grid = term.grid();
            let lines: Vec<String> = grid.visible_rows().iter().map(|row| {
                row.iter()
                    .filter(|c| !c.is_wide_continuation)
                    .map(|c| c.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            }).collect();
            Ok(lines)
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// screen.text: 读取屏幕纯文本
    pub fn read_text(&self, id: &str) -> Result<serde_json::Value> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let term = session.terminal.read();
            let grid = term.grid();
            let lines: Vec<String> = grid.visible_rows().iter().map(|row| {
                row.iter()
                    .filter(|c| !c.is_wide_continuation)
                    .map(|c| c.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            }).collect();
            Ok(serde_json::json!({
                "lines": lines,
                "cursor": {
                    "row": grid.cursor.row,
                    "col": grid.cursor.col,
                },
                "cols": grid.cols(),
                "rows": grid.rows(),
            }))
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// exec.run_wait: 执行命令并等待完成，返回纯文本输出
    pub fn run_wait(&self, id: &str, command: &str, timeout_ms: u64) -> Result<serde_json::Value> {
        // 策略检查
        self.check_policy_internal(command)?;

        let shell_type = {
            let sessions = self.sessions.read();
            match sessions.get(id) {
                Some(s) => s.shell_type.clone(),
                None => anyhow::bail!("Session 未找到: {}", id),
            }
        };

        // 生成唯一标记
        let marker = format!("UNTERM_DONE_{}", Uuid::new_v4().to_string().replace('-', ""));

        // 构造带标记的命令
        let full_cmd = if shell_type.is_powershell() {
            format!("{}; Write-Host '{}'\n", command, marker)
        } else {
            format!("{}; echo '{}'\n", command, marker)
        };

        let start = std::time::Instant::now();

        // 发送命令
        self.send_input(id, &full_cmd)?;

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        // 轮询等待标记出现
        loop {
            let lines = self.visible_text(id)?;
            // 查找标记行
            let marker_idx = lines.iter().position(|l| l.contains(&marker));
            if let Some(idx) = marker_idx {
                // 找到命令回显行（包含原始命令的行）
                let cmd_idx = lines.iter().position(|l| l.contains(command)).unwrap_or(0);
                // 输出 = 命令行之后 到 标记行之前
                let output_start = (cmd_idx + 1).min(idx);
                let output_lines: Vec<&str> = lines[output_start..idx]
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                let output = output_lines.join("\n");
                let elapsed = start.elapsed().as_millis() as u64;
                return Ok(serde_json::json!({
                    "output": output,
                    "command": command,
                    "elapsed_ms": elapsed,
                    "completed": true,
                }));
            }

            if std::time::Instant::now() >= deadline {
                // 超时，返回当前屏幕内容
                let elapsed = start.elapsed().as_millis() as u64;
                let output = lines.join("\n");
                return Ok(serde_json::json!({
                    "output": output,
                    "command": command,
                    "elapsed_ms": elapsed,
                    "completed": false,
                    "timeout": true,
                }));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// session.idle: 检查 Shell 是否空闲
    pub fn is_idle(&self, id: &str) -> Result<serde_json::Value> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let last_input = *session.last_input_time.lock();
            let last_output = *session.last_output_time.lock();
            let since_output = last_output.elapsed();
            let output_after_input = last_output > last_input;

            // Shell 空闲条件：最后输出在最后输入之后，且 500ms 没有新输出
            let idle_by_timing = output_after_input && since_output > std::time::Duration::from_millis(500);

            // 额外检查：最后一行是否匹配提示符模式
            let prompt_pattern = session.shell_type.prompt_pattern();
            let prompt_regex = regex::Regex::new(prompt_pattern).ok();
            let lines = {
                let term = session.terminal.read();
                let grid = term.grid();
                // 从光标行开始，找最后一行非空文本
                let cursor_row = grid.cursor.row as usize;
                let row = &grid.visible_rows()[cursor_row.min(grid.rows() as usize - 1)];
                row.iter()
                    .filter(|c| !c.is_wide_continuation)
                    .map(|c| c.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            };
            let prompt_match = prompt_regex
                .as_ref()
                .map_or(false, |re| re.is_match(&lines));

            let idle = idle_by_timing && prompt_match;

            Ok(serde_json::json!({
                "idle": idle,
                "idle_by_timing": idle_by_timing,
                "prompt_detected": prompt_match,
                "cursor_line": lines,
                "since_last_output_ms": since_output.as_millis() as u64,
                "shell_type": session.shell_type,
            }))
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// session.cwd: 获取当前工作目录
    pub fn get_cwd(&self, id: &str) -> Result<serde_json::Value> {
        let shell_type = {
            let sessions = self.sessions.read();
            match sessions.get(id) {
                Some(s) => s.shell_type.clone(),
                None => anyhow::bail!("Session 未找到: {}", id),
            }
        };

        // 通过提示符行提取路径
        let lines = self.visible_text(id)?;
        let cursor_row = {
            let sessions = self.sessions.read();
            let session = sessions.get(id).unwrap();
            let term = session.terminal.read();
            term.grid().cursor.row as usize
        };

        let prompt_line = &lines[cursor_row.min(lines.len() - 1)];

        // PowerShell: "PS E:\code\unterm> "
        if shell_type.is_powershell() {
            if let Some(start) = prompt_line.find("PS ") {
                if let Some(end) = prompt_line[start + 3..].find('>') {
                    let cwd = &prompt_line[start + 3..start + 3 + end];
                    return Ok(serde_json::json!({
                        "cwd": cwd.trim(),
                        "source": "prompt",
                    }));
                }
            }
        }

        // bash/zsh: 尝试从提示符提取，或者回退到 session info
        let sessions = self.sessions.read();
        let session = sessions.get(id).unwrap();
        Ok(serde_json::json!({
            "cwd": session.info.cwd,
            "source": "session_info",
        }))
    }

    /// session.env: 读取环境变量
    pub fn get_env(&self, id: &str, var_name: &str) -> Result<serde_json::Value> {
        let shell_type = {
            let sessions = self.sessions.read();
            match sessions.get(id) {
                Some(s) => s.shell_type.clone(),
                None => anyhow::bail!("Session 未找到: {}", id),
            }
        };

        // 通过 exec.run_wait 读取环境变量
        let cmd = if shell_type.is_powershell() {
            format!("$env:{}", var_name)
        } else {
            format!("echo ${}", var_name)
        };

        self.run_wait(id, &cmd, 5000)
    }

    /// session.set_env: 设置环境变量
    pub fn set_env(&self, id: &str, var_name: &str, value: &str) -> Result<serde_json::Value> {
        let shell_type = {
            let sessions = self.sessions.read();
            match sessions.get(id) {
                Some(s) => s.shell_type.clone(),
                None => anyhow::bail!("Session 未找到: {}", id),
            }
        };

        let cmd = if shell_type.is_powershell() {
            format!("$env:{}='{}'", var_name, value.replace('\'', "''"))
        } else {
            format!("export {}='{}'", var_name, value.replace('\'', "'\\''"))
        };

        self.run_wait(id, &cmd, 5000)?;
        Ok(serde_json::json!({
            "set": true,
            "name": var_name,
            "value": value,
        }))
    }

    /// system.info: 获取系统信息
    pub fn system_info(&self) -> serde_json::Value {
        serde_json::json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "family": std::env::consts::FAMILY,
            "hostname": hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".into()),
            "user": std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .unwrap_or_else(|_| "unknown".into()),
            "home": std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| "unknown".into()),
            "pid": std::process::id(),
        })
    }

    /// screen.search: 在屏幕 + scrollback 中搜索文本
    pub fn search_screen(&self, id: &str, pattern: &str, max_results: usize) -> Result<serde_json::Value> {
        let regex = safe_regex(pattern)?;

        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            let term = session.terminal.read();
            let grid = term.grid();
            let mut matches = Vec::new();

            // 搜索 scrollback
            let sb = grid.scrollback();
            for (i, row) in sb.iter().enumerate() {
                let line: String = row.iter()
                    .filter(|c| !c.is_wide_continuation)
                    .map(|c| c.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string();
                if regex.is_match(&line) {
                    matches.push(serde_json::json!({
                        "line_number": -(sb.len() as i64 - i as i64),
                        "region": "scrollback",
                        "text": line,
                    }));
                    if matches.len() >= max_results { break; }
                }
            }

            // 搜索可见区域
            if matches.len() < max_results {
                for (i, row) in grid.visible_rows().iter().enumerate() {
                    let line: String = row.iter()
                        .filter(|c| !c.is_wide_continuation)
                        .map(|c| c.ch)
                        .collect::<String>()
                        .trim_end()
                        .to_string();
                    if regex.is_match(&line) {
                        matches.push(serde_json::json!({
                            "line_number": i,
                            "region": "visible",
                            "text": line,
                        }));
                        if matches.len() >= max_results { break; }
                    }
                }
            }

            Ok(serde_json::json!({
                "pattern": pattern,
                "matches": matches,
                "total_found": matches.len(),
            }))
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// exec.status: 查询当前是否有命令在运行
    pub fn exec_status(&self, id: &str) -> Result<serde_json::Value> {
        // 复用 is_idle 的逻辑
        let idle_result = self.is_idle(id)?;
        let idle = idle_result.get("idle").and_then(|v| v.as_bool()).unwrap_or(false);
        Ok(serde_json::json!({
            "running": !idle,
            "idle": idle,
            "detail": idle_result,
        }))
    }

    /// exec.cancel: 取消正在运行的命令
    pub fn cancel_command(&self, id: &str) -> Result<serde_json::Value> {
        // 发送 Ctrl+C
        self.send_signal(id, "SIGINT")?;

        // 等待回到提示符（最多 3 秒）
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let idle_result = self.is_idle(id)?;
            let idle = idle_result.get("idle").and_then(|v| v.as_bool()).unwrap_or(false);
            if idle {
                return Ok(serde_json::json!({
                    "cancelled": true,
                    "prompt_restored": true,
                }));
            }
            if std::time::Instant::now() >= deadline {
                return Ok(serde_json::json!({
                    "cancelled": true,
                    "prompt_restored": false,
                    "message": "Ctrl+C 已发送，但提示符未恢复",
                }));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// session.audit_log: 获取审计日志
    pub fn get_audit_log(&self, limit: Option<u32>, session_id: Option<&str>) -> Vec<AuditEntry> {
        let log = self.audit_log.lock();
        let mut entries: Vec<AuditEntry> = log.iter().cloned().collect();
        if let Some(sid) = session_id {
            entries.retain(|e| e.session_id.as_deref() == Some(sid));
        }
        if let Some(limit) = limit {
            let limit = limit as usize;
            if entries.len() > limit {
                entries = entries[entries.len() - limit..].to_vec();
            }
        }
        entries
    }

    /// policy.set: 设置命令策略
    pub fn set_policy(&self, new_policy: CommandPolicy) {
        *self.policy.lock() = new_policy;
    }

    /// policy.get: 获取当前策略
    pub fn get_policy(&self) -> CommandPolicy {
        self.policy.lock().clone()
    }

    /// policy.check: 检查命令是否被允许
    pub fn check_policy(&self, command: &str) -> serde_json::Value {
        match self.check_policy_internal(command) {
            Ok(()) => serde_json::json!({
                "allowed": true,
                "command": command,
            }),
            Err(e) => serde_json::json!({
                "allowed": false,
                "command": command,
                "reason": e.to_string(),
            }),
        }
    }

    /// 内部策略检查
    fn check_policy_internal(&self, command: &str) -> Result<()> {
        let policy = self.policy.lock();
        if !policy.enabled {
            return Ok(());
        }

        // 黑名单检查
        for pattern in &policy.blocked_patterns {
            if let Ok(re) = safe_regex(pattern) {
                if re.is_match(command) {
                    anyhow::bail!("命令被策略拒绝（匹配黑名单: {}）: {}", pattern, command);
                }
            }
        }

        // 白名单检查（非空时只允许匹配的命令）
        if !policy.allowed_patterns.is_empty() {
            let allowed = policy.allowed_patterns.iter().any(|pattern| {
                safe_regex(pattern)
                    .map(|re| re.is_match(command))
                    .unwrap_or(false)
            });
            if !allowed {
                anyhow::bail!("命令不在白名单中: {}", command);
            }
        }

        Ok(())
    }
}
