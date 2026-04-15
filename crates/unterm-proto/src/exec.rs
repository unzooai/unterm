use serde::{Deserialize, Serialize};

use crate::session::SessionId;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecRunRequest {
    pub session_id: SessionId,
    pub command: String,
    /// 超时时间（毫秒），None 表示不限
    pub timeout: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecRunResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecSendRequest {
    pub session_id: SessionId,
    /// 原始输入内容（支持控制字符）
    pub input: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecSignalRequest {
    pub session_id: SessionId,
    pub signal: Signal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Signal {
    Sigint,
    Sigterm,
    Sigkill,
    Sigtstp,
    Sigcont,
}
