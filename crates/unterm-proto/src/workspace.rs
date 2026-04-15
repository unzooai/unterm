use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub name: String,
    pub created_at: String,
    pub sessions: Vec<WorkspaceSession>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceSession {
    pub name: Option<String>,
    pub shell: String,
    pub cwd: String,
    pub env: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveWorkspaceRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreWorkspaceRequest {
    pub name: String,
}
