//! 配置系统模块

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 默认 Shell（Windows: pwsh.exe, macOS: /bin/zsh）
    #[serde(default = "default_shell")]
    pub default_shell: String,

    /// 默认工作目录（None 则用用户 home）
    #[serde(default)]
    pub default_cwd: Option<String>,

    /// Core daemon 地址
    #[serde(default = "default_core_address")]
    pub core_address: String,

    /// 语言（zh-CN / en）
    #[serde(default)]
    pub locale: Option<String>,

    /// 字体大小
    #[serde(default = "default_font_size")]
    pub font_size: f32,

    /// 行高
    #[serde(default = "default_line_height")]
    pub line_height: f32,

    /// 字体族名
    #[serde(default = "default_font_family")]
    pub font_family: String,

    /// 新 Tab 时自动创建 session
    #[serde(default = "default_true")]
    pub auto_create_session: bool,

    /// 屏幕内容轮询间隔（毫秒）
    #[serde(default = "default_poll_interval")]
    pub screen_poll_interval_ms: u64,

    /// 代理配置
    #[serde(default)]
    pub proxy: ProxySettings,
}

/// 代理设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxySettings {
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
    /// HTTP 代理地址
    #[serde(default)]
    pub http_proxy: Option<String>,
    /// SOCKS5 代理地址
    #[serde(default)]
    pub socks_proxy: Option<String>,
}

fn default_shell() -> String {
    if cfg!(target_os = "windows") {
        // 优先 pwsh (PowerShell 7+)，不存在则回退 powershell
        if which_exists("pwsh.exe") {
            "pwsh.exe".into()
        } else {
            "powershell.exe".into()
        }
    } else {
        "/bin/zsh".into()
    }
}

/// 检查命令是否存在于 PATH 中
fn which_exists(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| dir.join(cmd).exists())
        })
        .unwrap_or(false)
}

fn default_core_address() -> String {
    "127.0.0.1:19876".into()
}

fn default_font_size() -> f32 {
    16.0
}

fn default_line_height() -> f32 {
    20.0
}

fn default_font_family() -> String {
    "Cascadia Code".into()
}

fn default_true() -> bool {
    true
}

fn default_poll_interval() -> u64 {
    50
}

impl AppConfig {
    /// 配置文件路径：~/.unterm/config.toml
    pub fn config_path() -> PathBuf {
        let home = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".into())
        } else {
            std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
        };
        PathBuf::from(home).join(".unterm").join("config.toml")
    }

    /// 加载配置文件，不存在则用默认值
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        tracing::warn!("配置文件解析失败，使用默认值: {}", e);
                    }
                },
                Err(e) => {
                    tracing::warn!("配置文件读取失败，使用默认值: {}", e);
                }
            }
        }
        Self::default()
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// 获取默认工作目录
    pub fn effective_cwd(&self) -> String {
        self.default_cwd.clone().unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".into())
            } else {
                std::env::var("HOME").unwrap_or_else(|_| "/".into())
            }
        })
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_shell: default_shell(),
            default_cwd: None,
            core_address: default_core_address(),
            locale: None,
            font_size: default_font_size(),
            line_height: default_line_height(),
            font_family: default_font_family(),
            auto_create_session: true,
            screen_poll_interval_ms: default_poll_interval(),
            proxy: ProxySettings::default(),
        }
    }
}
