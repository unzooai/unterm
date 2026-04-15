use serde::{Deserialize, Serialize};

use crate::session::{SessionId, SessionPolicy};

#[derive(Debug, Serialize, Deserialize)]
pub struct SetPolicyRequest {
    pub session_id: SessionId,
    pub policy: SessionPolicy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityRules {
    /// 命令黑名单（正则匹配）
    pub blacklist: Vec<String>,
    /// 命令白名单（正则匹配），设置后只允许白名单内的命令
    pub whitelist: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub session_id: SessionId,
    pub request_id: String,
}

/// 被拦截的高危命令
#[derive(Debug, Serialize, Deserialize)]
pub struct PendingApproval {
    pub request_id: String,
    pub session_id: SessionId,
    pub command: String,
    pub reason: String,
    pub timestamp: String,
}
