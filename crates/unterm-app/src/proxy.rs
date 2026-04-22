//! 代理管理模块（Tauri 侧）
//!
//! 支持三种模式：
//! - off: 关闭代理
//! - manual: 手动设置 HTTP/SOCKS 地址
//! - clash: 使用 mihomo 内核，支持订阅、节点管理、测速

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;
use tracing::{info, warn};

// ─── 数据类型 ───

/// 代理模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMode {
    Off,
    Manual,
    Clash,
}

/// Clash 策略类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ClashStrategy {
    UrlTest,
    Fallback,
    LoadBalance,
    Select,
}

impl Default for ClashStrategy {
    fn default() -> Self {
        Self::UrlTest
    }
}

impl ClashStrategy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::UrlTest => "url-test",
            Self::Fallback => "fallback",
            Self::LoadBalance => "load-balance",
            Self::Select => "select",
        }
    }
}

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    pub name: String,
    pub node_type: String,
    pub latency_ms: Option<u64>,
    pub available: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub subscription: String,
    /// 原始 Clash YAML 节点配置（mihomo 需要完整配置）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_config: Option<serde_yaml::Value>,
}

fn default_true() -> bool {
    true
}

/// 手动代理配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManualConfig {
    pub http: String,
    pub socks: String,
}

/// Clash 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashConfig {
    pub subscriptions: Vec<String>,
    pub strategy: ClashStrategy,
    pub current_node: Option<String>,
    pub nodes: Vec<ProxyNode>,
    pub port: u16,
    pub socks_port: u16,
}

impl Default for ClashConfig {
    fn default() -> Self {
        Self {
            subscriptions: vec![],
            strategy: ClashStrategy::default(),
            current_node: None,
            nodes: vec![],
            port: 17890,
            socks_port: 17891,
        }
    }
}

/// 持久化的代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyPersist {
    pub mode: ProxyMode,
    pub manual: ManualConfig,
    pub clash: ClashConfig,
}

impl Default for ProxyPersist {
    fn default() -> Self {
        Self {
            mode: ProxyMode::Off,
            manual: ManualConfig::default(),
            clash: ClashConfig::default(),
        }
    }
}

/// 代理状态（返回给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub mode: ProxyMode,
    pub manual: ManualConfig,
    pub clash_strategy: ClashStrategy,
    pub clash_subscriptions: Vec<String>,
    pub current_node: Option<String>,
    pub nodes: Vec<ProxyNode>,
    pub clash_port: u16,
    pub clash_socks_port: u16,
    /// mihomo 进程是否在运行
    #[serde(default)]
    pub mihomo_running: bool,
    /// mihomo 二进制是否已安装
    #[serde(default)]
    pub mihomo_installed: bool,
}

// ─── Mihomo 进程管理 ───

/// mihomo 子进程管理器
struct MihomoProcess {
    child: Option<Child>,
    api_port: u16,
    api_secret: String,
    mixed_port: u16,
    config_dir: PathBuf,
    binary_path: PathBuf,
}

impl MihomoProcess {
    /// 查找可用端口
    fn find_available_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .unwrap_or(17890)
    }

    /// 生成随机 secret
    fn random_secret() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..32).map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 { (b'0' + idx) as char }
            else { (b'a' + idx - 10) as char }
        }).collect()
    }

    /// 创建新的 mihomo 进程管理器（不启动进程）
    fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let mihomo_dir = home.join(".unterm").join("mihomo");
        let _ = std::fs::create_dir_all(&mihomo_dir);

        let binary_name = if cfg!(target_os = "windows") { "mihomo.exe" } else { "mihomo" };
        let binary_path = mihomo_dir.join(binary_name);
        let config_dir = mihomo_dir.join("config");
        let _ = std::fs::create_dir_all(&config_dir);

        Self {
            child: None,
            api_port: Self::find_available_port(),
            api_secret: Self::random_secret(),
            mixed_port: Self::find_available_port(),
            config_dir,
            binary_path,
        }
    }

    /// 二进制是否已安装
    fn is_installed(&self) -> bool {
        self.binary_path.exists()
    }

    /// 进程是否在运行
    fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// 生成 mihomo 配置文件
    fn generate_config(&self, nodes: &[ProxyNode], strategy: &ClashStrategy) -> Result<(), String> {
        let enabled_nodes: Vec<&ProxyNode> = nodes.iter()
            .filter(|n| n.enabled && n.raw_config.is_some())
            .collect();

        if enabled_nodes.is_empty() {
            return Err("没有可用的节点（需要带完整配置的 YAML 订阅节点）".into());
        }

        // 构建 proxies 列表
        let proxies: Vec<serde_yaml::Value> = enabled_nodes.iter()
            .filter_map(|n| n.raw_config.clone())
            .collect();

        // 构建 proxy-group
        let proxy_names: Vec<serde_yaml::Value> = enabled_nodes.iter()
            .map(|n| serde_yaml::Value::String(n.name.clone()))
            .collect();

        let mut proxy_group = serde_yaml::Mapping::new();
        proxy_group.insert(
            serde_yaml::Value::String("name".into()),
            serde_yaml::Value::String("unterm-proxy".into()),
        );
        proxy_group.insert(
            serde_yaml::Value::String("type".into()),
            serde_yaml::Value::String(strategy.as_str().into()),
        );
        proxy_group.insert(
            serde_yaml::Value::String("proxies".into()),
            serde_yaml::Value::Sequence(proxy_names),
        );

        // url-test / fallback 需要测速 URL
        if *strategy == ClashStrategy::UrlTest || *strategy == ClashStrategy::Fallback {
            proxy_group.insert(
                serde_yaml::Value::String("url".into()),
                serde_yaml::Value::String("http://www.gstatic.com/generate_204".into()),
            );
            proxy_group.insert(
                serde_yaml::Value::String("interval".into()),
                serde_yaml::Value::Number(serde_yaml::Number::from(300)),
            );
        }

        // 构建完整配置
        let mut config = serde_yaml::Mapping::new();
        config.insert(
            serde_yaml::Value::String("mixed-port".into()),
            serde_yaml::Value::Number(serde_yaml::Number::from(self.mixed_port as u64)),
        );
        config.insert(
            serde_yaml::Value::String("external-controller".into()),
            serde_yaml::Value::String(format!("127.0.0.1:{}", self.api_port)),
        );
        config.insert(
            serde_yaml::Value::String("secret".into()),
            serde_yaml::Value::String(self.api_secret.clone()),
        );
        config.insert(
            serde_yaml::Value::String("mode".into()),
            serde_yaml::Value::String("rule".into()),
        );
        config.insert(
            serde_yaml::Value::String("log-level".into()),
            serde_yaml::Value::String("warning".into()),
        );
        config.insert(
            serde_yaml::Value::String("allow-lan".into()),
            serde_yaml::Value::Bool(false),
        );
        config.insert(
            serde_yaml::Value::String("proxies".into()),
            serde_yaml::Value::Sequence(proxies),
        );
        config.insert(
            serde_yaml::Value::String("proxy-groups".into()),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::Mapping(proxy_group)]),
        );
        // 所有流量走代理
        config.insert(
            serde_yaml::Value::String("rules".into()),
            serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("MATCH,unterm-proxy".into()),
            ]),
        );

        let config_path = self.config_dir.join("config.yaml");
        let yaml_str = serde_yaml::to_string(&serde_yaml::Value::Mapping(config))
            .map_err(|e| format!("生成配置失败: {}", e))?;

        std::fs::write(&config_path, yaml_str)
            .map_err(|e| format!("写入配置失败: {}", e))?;

        info!("mihomo 配置已生成: {:?}, mixed-port={}, api-port={}, 节点数={}",
            config_path, self.mixed_port, self.api_port, enabled_nodes.len());
        Ok(())
    }

    /// 启动 mihomo 进程
    fn start(&mut self, nodes: &[ProxyNode], strategy: &ClashStrategy) -> Result<(), String> {
        if !self.is_installed() {
            return Err(format!(
                "mihomo 二进制未找到: {:?}\n请下载 mihomo 并放置到该路径",
                self.binary_path
            ));
        }

        // 先停止已有进程
        self.stop();

        // 重新分配端口（避免上次残留占用）
        self.mixed_port = Self::find_available_port();
        self.api_port = Self::find_available_port();
        self.api_secret = Self::random_secret();

        // 生成配置
        self.generate_config(nodes, strategy)?;

        // 启动进程
        let mut cmd = std::process::Command::new(&self.binary_path);
        cmd.arg("-d").arg(&self.config_dir);

        // Windows: 不创建控制台窗口
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let child = cmd.spawn()
            .map_err(|e| format!("启动 mihomo 失败: {}", e))?;

        info!("mihomo 进程已启动: pid={}", child.id());
        self.child = Some(child);
        Ok(())
    }

    /// 停止 mihomo 进程
    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            info!("正在停止 mihomo 进程: pid={}", child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// REST API 基础 URL
    fn api_base(&self) -> String {
        format!("http://127.0.0.1:{}", self.api_port)
    }

    /// 构建带认证的 reqwest 客户端请求头
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_secret)
    }

    /// 等待 API 就绪（阻塞版本）
    fn wait_ready_blocking(&self) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .map_err(|e| format!("创建客户端失败: {}", e))?;

        let url = format!("{}/version", self.api_base());
        for i in 0..30 {
            match client.get(&url)
                .header("Authorization", self.auth_header())
                .send()
            {
                Ok(resp) if resp.status().is_success() => {
                    info!("mihomo API 就绪 (第 {} 次尝试)", i + 1);
                    return Ok(());
                }
                _ => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        Err("mihomo API 3秒内未就绪".into())
    }

    /// 通过 REST API 测速
    fn test_latency_blocking(&self) -> Result<HashMap<String, Option<u64>>, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("创建客户端失败: {}", e))?;

        let url = format!(
            "{}/group/unterm-proxy/delay?url=http://www.gstatic.com/generate_204&timeout=5000",
            self.api_base()
        );

        let resp = client.get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .map_err(|e| format!("测速请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("测速失败 HTTP {}: {}", status, body));
        }

        // mihomo 返回 {"node_name": delay_ms, ...} 或 {"node_name": {"message":"..."}}
        let data: HashMap<String, serde_json::Value> = resp.json()
            .map_err(|e| format!("解析测速结果失败: {}", e))?;

        let mut result = HashMap::new();
        for (name, val) in data {
            if let Some(delay) = val.as_u64() {
                result.insert(name, Some(delay));
            } else {
                result.insert(name, None); // 超时或错误
            }
        }
        Ok(result)
    }

    /// 通过 REST API 切换节点（select 模式）
    fn switch_node_blocking(&self, node_name: &str) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("创建客户端失败: {}", e))?;

        let url = format!("{}/proxies/unterm-proxy", self.api_base());
        let body = serde_json::json!({"name": node_name});

        let resp = client.put(&url)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .map_err(|e| format!("切换节点请求失败: {}", e))?;

        if resp.status().is_success() || resp.status().as_u16() == 204 {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            Err(format!("切换节点失败 HTTP {}: {}", status, body))
        }
    }

    /// 重载配置（不重启进程）
    fn reload_config_blocking(&self) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("创建客户端失败: {}", e))?;

        let config_path = self.config_dir.join("config.yaml");
        let body = serde_json::json!({
            "path": config_path.to_string_lossy().to_string()
        });

        let url = format!("{}/configs?force=true", self.api_base());
        let resp = client.put(&url)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .map_err(|e| format!("重载配置请求失败: {}", e))?;

        if resp.status().is_success() || resp.status().as_u16() == 204 {
            info!("mihomo 配置已热重载");
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            Err(format!("重载配置失败 HTTP {}: {}", status, body))
        }
    }

    /// 下载 mihomo 二进制
    fn download_binary(&self) -> Result<(), String> {
        let (os, arch, ext) = if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86_64") { ("windows", "amd64", ".zip") }
            else if cfg!(target_arch = "aarch64") { ("windows", "arm64", ".zip") }
            else { return Err("不支持的 Windows 架构".into()); }
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") { ("darwin", "arm64", ".gz") }
            else { ("darwin", "amd64", ".gz") }
        } else {
            if cfg!(target_arch = "x86_64") { ("linux", "amd64", ".gz") }
            else if cfg!(target_arch = "aarch64") { ("linux", "arm64", ".gz") }
            else { return Err("不支持的 Linux 架构".into()); }
        };

        // 获取最新版本号
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("unterm/1.0")
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        // 通过 GitHub API 获取最新 release
        let api_url = "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest";
        let release_resp = client.get(api_url)
            .send()
            .map_err(|e| format!("获取最新版本失败: {}", e))?;

        if !release_resp.status().is_success() {
            return Err(format!("获取最新版本失败 HTTP {}", release_resp.status()));
        }

        let release: serde_json::Value = release_resp.json()
            .map_err(|e| format!("解析版本信息失败: {}", e))?;

        let tag = release.get("tag_name")
            .and_then(|v| v.as_str())
            .ok_or("无法获取版本号")?;

        info!("mihomo 最新版本: {}", tag);

        // 构建下载文件名
        let filename = format!("mihomo-{}-{}-{}{}", os, arch, tag, ext);
        let download_url = format!(
            "https://github.com/MetaCubeX/mihomo/releases/download/{}/{}",
            tag, filename
        );

        info!("下载 mihomo: {}", download_url);
        let resp = client.get(&download_url)
            .send()
            .map_err(|e| format!("下载失败: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("下载失败 HTTP {}", resp.status()));
        }

        let bytes = resp.bytes()
            .map_err(|e| format!("读取下载内容失败: {}", e))?;

        // 解压
        if ext == ".zip" {
            self.extract_zip(&bytes)?;
        } else {
            self.extract_gz(&bytes)?;
        }

        // 保存版本号
        let version_file = self.binary_path.parent().unwrap().join("version.txt");
        let _ = std::fs::write(version_file, tag);

        info!("mihomo {} 已安装到 {:?}", tag, self.binary_path);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn extract_zip(&self, data: &[u8]) -> Result<(), String> {
        use std::io::{Cursor, Read};
        let reader = Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| format!("解压 ZIP 失败: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;

            let name = file.name().to_string();
            if name.contains("mihomo") && !name.ends_with('/') {
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)
                    .map_err(|e| format!("读取文件内容失败: {}", e))?;
                std::fs::write(&self.binary_path, contents)
                    .map_err(|e| format!("写入二进制失败: {}", e))?;
                return Ok(());
            }
        }
        Err("ZIP 中未找到 mihomo 二进制".into())
    }

    #[cfg(not(target_os = "windows"))]
    fn extract_zip(&self, _data: &[u8]) -> Result<(), String> {
        Err("非 Windows 平台不使用 ZIP 格式".into())
    }

    #[cfg(not(target_os = "windows"))]
    fn extract_gz(&self, data: &[u8]) -> Result<(), String> {
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut contents = Vec::new();
        decoder.read_to_end(&mut contents)
            .map_err(|e| format!("解压 GZ 失败: {}", e))?;
        std::fs::write(&self.binary_path, &contents)
            .map_err(|e| format!("写入二进制失败: {}", e))?;
        // 设置可执行权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.binary_path,
                std::fs::Permissions::from_mode(0o755));
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn extract_gz(&self, _data: &[u8]) -> Result<(), String> {
        Err("Windows 平台使用 ZIP 格式".into())
    }
}

impl Drop for MihomoProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

// ─── 代理管理器 ───

/// 代理管理器
pub struct ProxyManager {
    persist: ProxyPersist,
    config_path: PathBuf,
    mihomo: MihomoProcess,
}

impl ProxyManager {
    /// 加载或创建代理管理器
    pub fn new() -> Self {
        let config_path = Self::config_file_path();
        let persist = Self::load_from_disk(&config_path);
        let mihomo = MihomoProcess::new();
        Self {
            persist,
            config_path,
            mihomo,
        }
    }

    fn config_file_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let dir = home.join(".unterm");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("proxy.json")
    }

    fn load_from_disk(path: &PathBuf) -> ProxyPersist {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => ProxyPersist::default(),
        }
    }

    fn save_to_disk(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.persist) {
            let _ = std::fs::write(&self.config_path, json);
        }
    }

    /// 获取当前代理状态
    pub fn get_status(&mut self) -> ProxyStatus {
        ProxyStatus {
            mode: self.persist.mode.clone(),
            manual: self.persist.manual.clone(),
            clash_strategy: self.persist.clash.strategy.clone(),
            clash_subscriptions: self.persist.clash.subscriptions.clone(),
            current_node: self.persist.clash.current_node.clone(),
            nodes: self.persist.clash.nodes.iter().map(|n| {
                // 不传 raw_config 给前端（太大）
                ProxyNode {
                    raw_config: None,
                    ..n.clone()
                }
            }).collect(),
            clash_port: self.mihomo.mixed_port,
            clash_socks_port: self.mihomo.mixed_port, // mixed-port 同时处理 HTTP 和 SOCKS
            mihomo_running: self.mihomo.is_running(),
            mihomo_installed: self.mihomo.is_installed(),
        }
    }

    /// 启动 mihomo（如果已配置 Clash 模式）
    pub fn auto_start(&mut self) {
        if self.persist.mode == ProxyMode::Clash {
            match self.start_mihomo() {
                Ok(_) => info!("mihomo 自动启动成功"),
                Err(e) => warn!("mihomo 自动启动失败: {}", e),
            }
        }
    }

    /// 启动 mihomo 进程
    fn start_mihomo(&mut self) -> Result<(), String> {
        self.mihomo.start(&self.persist.clash.nodes, &self.persist.clash.strategy)?;
        // 等待 API 就绪
        self.mihomo.wait_ready_blocking()?;
        // 更新端口到持久化配置
        self.persist.clash.port = self.mihomo.mixed_port;
        self.persist.clash.socks_port = self.mihomo.mixed_port;
        self.save_to_disk();
        Ok(())
    }

    /// 切换代理模式
    /// Clash 模式启动失败时自动回退到 Off
    pub fn set_mode(&mut self, mode: ProxyMode) -> Result<(), String> {
        info!("代理模式切换: {:?}", mode);

        // 停止旧 mihomo
        if self.persist.mode == ProxyMode::Clash && mode != ProxyMode::Clash {
            self.mihomo.stop();
        }

        self.persist.mode = mode.clone();
        self.save_to_disk();

        // 启动新 mihomo
        if mode == ProxyMode::Clash {
            if let Err(e) = self.start_mihomo() {
                warn!("mihomo 启动失败，回退到关闭模式: {}", e);
                self.persist.mode = ProxyMode::Off;
                self.save_to_disk();
                return Err(format!("mihomo 启动失败: {}。已回退到关闭模式。请先更新订阅获取完整节点配置。", e));
            }
        }

        Ok(())
    }

    /// 设置手动代理地址
    pub fn set_manual_config(&mut self, http: String, socks: String) {
        self.persist.manual = ManualConfig { http, socks };
        self.save_to_disk();
    }

    /// 更新订阅链接
    pub fn update_subscription(&mut self, url: String) -> Vec<ProxyNode> {
        info!("更新订阅: {}", url);

        if !self.persist.clash.subscriptions.contains(&url) {
            self.persist.clash.subscriptions.push(url.clone());
        }

        let sub_label = Self::subscription_label(&url);

        // 移除旧节点
        self.persist.clash.nodes.retain(|n| {
            !n.subscription.is_empty() && n.subscription != sub_label
        });

        match Self::fetch_subscription(&url) {
            Ok(nodes) => {
                let parsed: Vec<ProxyNode> = nodes.into_iter().map(|mut n| {
                    n.subscription = sub_label.clone();
                    n
                }).collect();
                info!("订阅 {} 解析到 {} 个节点", sub_label, parsed.len());
                self.persist.clash.nodes.extend(parsed);
            }
            Err(e) => {
                warn!("订阅拉取/解析失败: {}", e);
            }
        }

        if self.persist.clash.current_node.is_none() {
            if let Some(node) = self.persist.clash.nodes.iter().find(|n| n.available && n.enabled) {
                self.persist.clash.current_node = Some(node.name.clone());
            }
        }

        self.save_to_disk();

        // 如果 mihomo 正在运行，热重载配置
        if self.mihomo.is_running() {
            if let Err(e) = self.reload_mihomo_config() {
                warn!("mihomo 配置热重载失败: {}", e);
            }
        }

        self.persist.clash.nodes.clone()
    }

    /// 热重载 mihomo 配置
    fn reload_mihomo_config(&mut self) -> Result<(), String> {
        self.mihomo.generate_config(&self.persist.clash.nodes, &self.persist.clash.strategy)?;
        self.mihomo.reload_config_blocking()
    }

    /// 拉取订阅内容并解析节点
    fn fetch_subscription(url: &str) -> Result<Vec<ProxyNode>, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .danger_accept_invalid_certs(true)
            .user_agent("clash-verge/v1.0")
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let resp = client.get(url).send()
            .map_err(|e| format!("请求失败: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let body = resp.text().map_err(|e| format!("读取响应失败: {}", e))?;

        info!("订阅响应长度: {} bytes, 前 200 字符: {}", body.len(), &body[..body.len().min(200)]);

        // 尝试 YAML 格式
        match Self::parse_clash_yaml(&body) {
            Ok(nodes) if !nodes.is_empty() => {
                info!("YAML 格式解析成功: {} 个节点", nodes.len());
                return Ok(nodes);
            }
            Ok(_) => info!("YAML 格式匹配但无节点"),
            Err(e) => info!("非 YAML 格式: {}", e),
        }

        // 尝试 base64
        match Self::parse_base64_subscription(&body) {
            Ok(nodes) if !nodes.is_empty() => {
                info!("Base64 格式解析成功: {} 个节点", nodes.len());
                return Ok(nodes);
            }
            Ok(_) => info!("Base64 解码成功但无有效节点"),
            Err(e) => info!("非 Base64 格式: {}", e),
        }

        // 直接按行解析
        let nodes = Self::parse_uri_lines(&body);
        if !nodes.is_empty() {
            info!("URI 行解析成功: {} 个节点", nodes.len());
            return Ok(nodes);
        }

        Err(format!("无法识别的订阅格式 (响应长度 {} bytes)", body.len()))
    }

    /// 解析 Clash YAML 配置格式（保留原始配置供 mihomo 使用）
    fn parse_clash_yaml(content: &str) -> Result<Vec<ProxyNode>, String> {
        let yaml: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| format!("YAML 解析失败: {}", e))?;

        let proxies = yaml.get("proxies")
            .and_then(|p| p.as_sequence())
            .ok_or("YAML 中没有 proxies 字段")?;

        let mut nodes = Vec::new();
        for proxy in proxies {
            let name = proxy.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("未知节点")
                .to_string();

            let node_type = proxy.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            nodes.push(ProxyNode {
                name,
                node_type,
                latency_ms: None,
                available: true,
                enabled: true,
                subscription: String::new(),
                raw_config: Some(proxy.clone()), // 保留完整 YAML 配置
            });
        }

        Ok(nodes)
    }

    /// 解析 base64 编码的订阅
    fn parse_base64_subscription(content: &str) -> Result<Vec<ProxyNode>, String> {
        let trimmed = content.trim();
        let decoded = BASE64.decode(trimmed)
            .or_else(|_| {
                use base64::engine::general_purpose::URL_SAFE_NO_PAD;
                URL_SAFE_NO_PAD.decode(trimmed)
            })
            .map_err(|e| format!("base64 解码失败: {}", e))?;

        let text = String::from_utf8(decoded)
            .map_err(|e| format!("UTF-8 解码失败: {}", e))?;

        Ok(Self::parse_uri_lines(&text))
    }

    /// 按行解析节点 URI
    fn parse_uri_lines(content: &str) -> Vec<ProxyNode> {
        content.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| Self::parse_single_uri(l.trim()))
            .collect()
    }

    /// 解析单个节点 URI
    fn parse_single_uri(uri: &str) -> Option<ProxyNode> {
        let (node_type, rest) = if uri.starts_with("vmess://") {
            ("vmess", &uri[8..])
        } else if uri.starts_with("trojan://") {
            ("trojan", &uri[9..])
        } else if uri.starts_with("ss://") {
            ("ss", &uri[5..])
        } else if uri.starts_with("vless://") {
            ("vless", &uri[8..])
        } else if uri.starts_with("ssr://") {
            ("ssr", &uri[6..])
        } else {
            return None;
        };

        let name = match node_type {
            "vmess" => {
                let b64 = rest.split('#').next().unwrap_or(rest);
                if let Ok(decoded) = BASE64.decode(b64.trim()) {
                    if let Ok(text) = String::from_utf8(decoded) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            json.get("ps")
                                .or(json.get("remark"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("vmess node")
                                .to_string()
                        } else {
                            Self::extract_name_from_fragment(rest)
                        }
                    } else {
                        Self::extract_name_from_fragment(rest)
                    }
                } else {
                    Self::extract_name_from_fragment(rest)
                }
            }
            _ => Self::extract_name_from_fragment(rest),
        };

        Some(ProxyNode {
            name,
            node_type: node_type.to_string(),
            latency_ms: None,
            available: true,
            enabled: true,
            subscription: String::new(),
            raw_config: None, // URI 格式无原始 YAML 配置
        })
    }

    fn extract_name_from_fragment(rest: &str) -> String {
        if let Some(idx) = rest.rfind('#') {
            urlencoding_decode(&rest[idx + 1..])
        } else {
            "unnamed".to_string()
        }
    }

    fn subscription_label(url: &str) -> String {
        if let Some(start) = url.find("://") {
            let rest = &url[start + 3..];
            let domain = rest.split('/').next().unwrap_or(rest);
            let domain = domain.split(':').next().unwrap_or(domain);
            let parts: Vec<&str> = domain.split('.').collect();
            if parts.len() >= 2 {
                return parts[parts.len() - 2].to_string();
            }
            return domain.to_string();
        }
        "sub".to_string()
    }

    /// 删除订阅
    pub fn remove_subscription(&mut self, url: &str) {
        let sub_label = Self::subscription_label(url);
        self.persist.clash.subscriptions.retain(|s| s != url);
        self.persist.clash.nodes.retain(|n| n.subscription != sub_label);
        if let Some(ref current) = self.persist.clash.current_node {
            if !self.persist.clash.nodes.iter().any(|n| &n.name == current) {
                self.persist.clash.current_node = self.persist.clash.nodes.iter()
                    .find(|n| n.available && n.enabled)
                    .map(|n| n.name.clone());
            }
        }
        self.save_to_disk();

        // 热重载
        if self.mihomo.is_running() {
            if let Err(e) = self.reload_mihomo_config() {
                warn!("mihomo 配置热重载失败: {}", e);
            }
        }
    }

    /// 获取节点列表
    pub fn list_nodes(&self) -> Vec<ProxyNode> {
        self.persist.clash.nodes.iter().map(|n| {
            ProxyNode {
                raw_config: None, // 不传给前端
                ..n.clone()
            }
        }).collect()
    }

    /// 设置节点是否参与轮换
    pub fn set_node_enabled(&mut self, node_name: &str, enabled: bool) {
        if let Some(node) = self.persist.clash.nodes.iter_mut().find(|n| n.name == node_name) {
            node.enabled = enabled;
        }
        self.save_to_disk();

        // 热重载
        if self.mihomo.is_running() {
            if let Err(e) = self.reload_mihomo_config() {
                warn!("mihomo 配置热重载失败: {}", e);
            }
        }
    }

    /// 批量设置所有节点启用/禁用
    pub fn set_all_nodes_enabled(&mut self, enabled: bool) {
        for node in &mut self.persist.clash.nodes {
            node.enabled = enabled;
        }
        self.save_to_disk();

        if self.mihomo.is_running() {
            if let Err(e) = self.reload_mihomo_config() {
                warn!("mihomo 配置热重载失败: {}", e);
            }
        }
    }

    /// 切换到指定节点
    pub fn switch_node(&mut self, node_name: &str) -> Result<(), String> {
        if !self.persist.clash.nodes.iter().any(|n| n.name == node_name) {
            return Err(format!("节点未找到: {}", node_name));
        }
        info!("切换节点: {}", node_name);
        self.persist.clash.current_node = Some(node_name.to_string());
        self.save_to_disk();

        // 如果是 select 模式且 mihomo 在运行，通过 API 切换
        if self.persist.clash.strategy == ClashStrategy::Select && self.mihomo.is_running() {
            if let Err(e) = self.mihomo.switch_node_blocking(node_name) {
                warn!("通过 API 切换节点失败: {}", e);
            }
        }

        Ok(())
    }

    /// 设置策略
    pub fn set_strategy(&mut self, strategy: ClashStrategy) {
        info!("切换策略: {:?}", strategy);
        self.persist.clash.strategy = strategy;
        self.save_to_disk();

        if self.mihomo.is_running() {
            if let Err(e) = self.reload_mihomo_config() {
                warn!("mihomo 配置热重载失败: {}", e);
            }
        }
    }

    /// 测速（通过 mihomo REST API 或 fallback mock）
    pub fn test_latency(&mut self) -> Vec<ProxyNode> {
        if self.mihomo.is_running() {
            // 真实测速
            match self.mihomo.test_latency_blocking() {
                Ok(results) => {
                    for node in &mut self.persist.clash.nodes {
                        if let Some(delay) = results.get(&node.name) {
                            match delay {
                                Some(ms) => {
                                    node.latency_ms = Some(*ms);
                                    node.available = true;
                                }
                                None => {
                                    node.available = false;
                                    node.latency_ms = None;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("真实测速失败，使用 mock: {}", e);
                    self.mock_latency();
                }
            }
        } else {
            // mihomo 未运行，使用 mock
            self.mock_latency();
        }

        self.save_to_disk();
        self.list_nodes()
    }

    /// Mock 测速（mihomo 未运行时使用）
    fn mock_latency(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::SystemTime;

        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        for (i, node) in self.persist.clash.nodes.iter_mut().enumerate() {
            let mut hasher = DefaultHasher::new();
            (seed as u64 + i as u64).hash(&mut hasher);
            let hash = hasher.finish();

            if hash % 10 == 0 {
                node.available = false;
                node.latency_ms = None;
            } else {
                node.available = true;
                node.latency_ms = Some(30 + (hash % 270));
            }
        }
    }

    /// 生成代理环境变量
    /// Clash 模式下仅当 mihomo 进程在运行时才注入，避免指向不存在的端口
    pub fn proxy_env_vars(&mut self) -> Option<HashMap<String, String>> {
        match &self.persist.mode {
            ProxyMode::Off => None,
            ProxyMode::Manual => {
                let manual = &self.persist.manual;
                if manual.http.is_empty() && manual.socks.is_empty() {
                    return None;
                }
                let mut env = HashMap::new();
                if !manual.http.is_empty() {
                    env.insert("HTTP_PROXY".into(), manual.http.clone());
                    env.insert("HTTPS_PROXY".into(), manual.http.clone());
                    env.insert("http_proxy".into(), manual.http.clone());
                    env.insert("https_proxy".into(), manual.http.clone());
                }
                if !manual.socks.is_empty() {
                    env.insert("ALL_PROXY".into(), manual.socks.clone());
                    env.insert("all_proxy".into(), manual.socks.clone());
                }
                env.insert("NO_PROXY".into(), "localhost,127.0.0.1,::1".into());
                env.insert("no_proxy".into(), "localhost,127.0.0.1,::1".into());
                Some(env)
            }
            ProxyMode::Clash => {
                // mihomo 没在运行就不注入，避免请求全部失败
                if !self.mihomo.is_running() {
                    warn!("Clash 模式但 mihomo 未运行，跳过代理 env 注入");
                    return None;
                }

                let http_proxy = format!("http://127.0.0.1:{}", self.mihomo.mixed_port);
                let socks_proxy = format!("socks5://127.0.0.1:{}", self.mihomo.mixed_port);

                let mut env = HashMap::new();
                env.insert("HTTP_PROXY".into(), http_proxy.clone());
                env.insert("HTTPS_PROXY".into(), http_proxy.clone());
                env.insert("http_proxy".into(), http_proxy.clone());
                env.insert("https_proxy".into(), http_proxy);
                env.insert("ALL_PROXY".into(), socks_proxy.clone());
                env.insert("all_proxy".into(), socks_proxy);
                env.insert("NO_PROXY".into(), "localhost,127.0.0.1,::1".into());
                env.insert("no_proxy".into(), "localhost,127.0.0.1,::1".into());
                Some(env)
            }
        }
    }

    /// 下载 mihomo 二进制
    pub fn download_mihomo(&self) -> Result<(), String> {
        self.mihomo.download_binary()
    }

    /// 停止 mihomo（应用退出时调用）
    pub fn shutdown(&mut self) {
        self.mihomo.stop();
    }
}

/// 简单的 URL percent-decoding
fn urlencoding_decode(input: &str) -> String {
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            result.push(b' ');
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8(result).unwrap_or_else(|_| input.to_string())
}
