use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub enabled: bool,
    pub mode: ProxyMode,
    pub current_node: Option<String>,
    pub latency_ms: Option<u64>,
    pub upload_bytes: u64,
    pub download_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMode {
    Builtin,
    External,
    Off,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyNode {
    pub name: String,
    pub node_type: String,
    pub latency_ms: Option<u64>,
    pub available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchNodeRequest {
    pub node_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoSwitchConfig {
    pub enabled: bool,
    /// 延迟阈值（毫秒），超过此值触发自动切换
    pub threshold_ms: Option<u64>,
    /// 检测间隔（秒）
    pub interval_secs: Option<u32>,
    /// 自动切换节点列表，严格按顺序切换
    pub fallback_nodes: Option<Vec<String>>,
    /// 故障恢复后是否自动回切首选节点
    pub auto_recovery: Option<bool>,
}
