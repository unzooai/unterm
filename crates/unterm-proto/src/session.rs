use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Session 唯一标识
pub type SessionId = String;

/// Session 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// 正在运行
    Running,
    /// 等待用户输入
    WaitingForInput,
    /// 已完成（进程退出）
    Exited { code: i32 },
    /// 出错
    Error { message: String },
}

/// Session 权限等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionPolicy {
    /// 只读：只能读取屏幕，不能执行命令
    Readonly,
    /// 受限：可执行命令，但高危命令需审批
    Restricted,
    /// 完全控制
    Full,
}

/// Session 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub name: Option<String>,
    pub status: SessionStatus,
    pub policy: SessionPolicy,
    pub shell: String,
    pub cwd: String,
    pub created_at: String,
    pub last_activity: String,
    pub cols: u16,
    pub rows: u16,
}

// -- MCP Tool 请求/响应类型 --

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub shell: Option<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeSessionRequest {
    pub session_id: SessionId,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionHistoryRequest {
    pub session_id: SessionId,
    pub since: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub direction: HistoryDirection,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryDirection {
    Input,
    Output,
}
