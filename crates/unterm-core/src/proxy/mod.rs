//! 内置代理引擎管理模块
//!
//! 第一期：通过进程管理控制 clash 二进制
//! 第二期：集成 clash-rs 库，直接嵌入
//!
//! 功能：
//! - 启动/停止代理进程
//! - 订阅链接管理
//! - 节点延迟检测和自动切换（严格按 fallback_nodes 列表顺序）
//! - 为 PTY session 生成代理环境变量
//! - 与本机 Clash 共存（独立端口，不修改系统代理）

use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 是否启用代理
    pub enabled: bool,
    /// 模式：builtin（内置）| external（复用本机）| off
    pub mode: ProxyMode,
    /// 订阅链接
    pub subscription: Option<String>,
    /// HTTP 代理端口
    pub port: u16,
    /// SOCKS5 代理端口
    pub socks_port: u16,
    /// 自动切换配置
    pub auto_switch: AutoSwitchConfig,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: ProxyMode::Off,
            subscription: None,
            port: 17890,
            socks_port: 17891,
            auto_switch: AutoSwitchConfig::default(),
        }
    }
}

/// 代理模式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMode {
    /// 内置代理引擎
    Builtin,
    /// 复用本机已有的代理
    External,
    /// 关闭代理
    Off,
}

/// 自动切换配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSwitchConfig {
    /// 是否启用自动切换
    pub enabled: bool,
    /// 延迟阈值（毫秒），超过则触发切换
    pub threshold_ms: u64,
    /// 检测间隔（秒）
    pub interval_secs: u32,
    /// 指定的切换节点列表，严格按顺序切换
    pub fallback_nodes: Vec<String>,
    /// 故障恢复后是否自动回切首选节点
    pub auto_recovery: bool,
}

impl Default for AutoSwitchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_ms: 3000,
            interval_secs: 30,
            fallback_nodes: vec![],
            auto_recovery: true,
        }
    }
}

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    /// 节点名称
    pub name: String,
    /// 节点类型（如 ss、vmess、trojan 等）
    pub node_type: String,
    /// 延迟（毫秒），None 表示尚未检测
    pub latency_ms: Option<u64>,
    /// 是否可用
    pub available: bool,
}

/// 代理状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    /// 是否启用
    pub enabled: bool,
    /// 当前模式
    pub mode: ProxyMode,
    /// 当前使用的节点名称
    pub current_node: Option<String>,
    /// 当前节点延迟（毫秒）
    pub latency_ms: Option<u64>,
    /// HTTP 代理端口
    pub port: u16,
    /// SOCKS5 代理端口
    pub socks_port: u16,
}

/// 代理管理器
///
/// 负责管理代理进程的生命周期、节点切换和环境变量注入。
pub struct ProxyManager {
    /// 代理配置
    config: ProxyConfig,
    /// 当前选中的节点
    current_node: Option<String>,
    /// 所有可用节点
    nodes: Vec<ProxyNode>,
}

impl ProxyManager {
    /// 创建新的代理管理器
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            current_node: None,
            nodes: vec![],
        }
    }

    /// 获取代理状态
    pub fn status(&self) -> ProxyStatus {
        ProxyStatus {
            enabled: self.config.enabled,
            mode: self.config.mode.clone(),
            current_node: self.current_node.clone(),
            latency_ms: None,
            port: self.config.port,
            socks_port: self.config.socks_port,
        }
    }

    /// 获取所有节点
    pub fn list_nodes(&self) -> &[ProxyNode] {
        &self.nodes
    }

    /// 切换到指定节点
    pub fn switch_node(&mut self, node_name: &str) -> Result<()> {
        // 验证节点存在
        if !self.nodes.iter().any(|n| n.name == node_name) {
            anyhow::bail!("节点未找到: {}", node_name);
        }
        info!("代理切换到节点: {}", node_name);
        self.current_node = Some(node_name.to_string());
        // TODO: 通知 clash 切换节点
        Ok(())
    }

    /// 生成代理环境变量，注入到 PTY session
    ///
    /// 返回 None 表示代理未启用，不需要注入环境变量。
    pub fn proxy_env_vars(&self) -> Option<HashMap<String, String>> {
        if !self.config.enabled {
            return None;
        }

        let mut env = HashMap::new();
        let http_proxy = format!("http://127.0.0.1:{}", self.config.port);
        let socks_proxy = format!("socks5://127.0.0.1:{}", self.config.socks_port);

        // 同时设置大小写版本，兼容不同工具的读取习惯
        env.insert("HTTP_PROXY".into(), http_proxy.clone());
        env.insert("HTTPS_PROXY".into(), http_proxy.clone());
        env.insert("http_proxy".into(), http_proxy.clone());
        env.insert("https_proxy".into(), http_proxy);
        env.insert("ALL_PROXY".into(), socks_proxy.clone());
        env.insert("all_proxy".into(), socks_proxy);
        // 不代理本地地址
        env.insert("NO_PROXY".into(), "localhost,127.0.0.1,::1".into());
        env.insert("no_proxy".into(), "localhost,127.0.0.1,::1".into());

        Some(env)
    }

    /// 启动自动切换检测（在后台 tokio task 中运行）
    ///
    /// 工作流程：
    /// 1. 每 interval 秒检测当前节点延迟
    /// 2. 延迟超过 threshold 时，按 fallback_nodes 顺序逐个尝试
    /// 3. 切换成功后通知
    /// 4. 全部失败则保持当前节点并告警
    pub async fn start_auto_switch(&self) {
        if !self.config.auto_switch.enabled {
            return;
        }
        let interval = self.config.auto_switch.interval_secs;
        let threshold = self.config.auto_switch.threshold_ms;
        let fallback_nodes = self.config.auto_switch.fallback_nodes.clone();

        info!(
            "代理自动切换已启用: 间隔={}s, 阈值={}ms, 节点={:?}",
            interval, threshold, fallback_nodes
        );

        // TODO: 启动定期检测任务
        // 1. 每 interval 秒检测当前节点延迟
        // 2. 延迟超过 threshold 时，按 fallback_nodes 顺序逐个尝试
        // 3. 切换成功后通知
        // 4. 全部失败则保持当前节点并告警
    }
}
