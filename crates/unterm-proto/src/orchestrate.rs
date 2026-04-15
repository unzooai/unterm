use serde::{Deserialize, Serialize};

use crate::session::SessionId;

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub command: String,
    pub name: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchResponse {
    pub session_id: SessionId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastRequest {
    pub session_ids: Vec<SessionId>,
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitRequest {
    pub session_id: SessionId,
    /// 正则表达式模式
    pub pattern: String,
    /// 超时时间（毫秒）
    pub timeout: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitResponse {
    /// 匹配到的文本
    pub matched: String,
    /// 是否超时
    pub timed_out: bool,
}
