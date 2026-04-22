//! PTY 管理模块
//! 封装 portable-pty，提供跨平台的伪终端创建和管理。

use std::collections::HashMap;
use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize, PtySystem, MasterPty, Child};

/// PTY 配置
pub struct PtyConfig {
    pub shell: Option<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub cols: u16,
    pub rows: u16,
    /// 代理环境变量（由 proxy 模块注入）
    pub proxy_env: Option<HashMap<String, String>>,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            shell: None,
            cwd: None,
            env: None,
            cols: 80,
            rows: 24,
            proxy_env: None,
        }
    }
}

/// PTY 句柄，持有 master 端和子进程
pub struct PtyHandle {
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn Child + Send + Sync>,
}

impl PtyHandle {
    /// 调整终端尺寸
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}

/// PTY 管理器
pub struct PtyManager {
    system: Box<dyn PtySystem + Send>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            system: native_pty_system(),
        }
    }

    /// 获取平台默认 shell
    fn default_shell() -> String {
        if cfg!(target_os = "windows") {
            "pwsh.exe".to_string()
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
        }
    }

    /// 创建新的 PTY 进程
    pub fn create_pty(&self, config: PtyConfig) -> Result<PtyHandle> {
        let size = PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = self.system.openpty(size)?;

        let shell = config.shell.unwrap_or_else(Self::default_shell);
        let mut cmd = CommandBuilder::new(&shell);

        // 设置工作目录
        if let Some(cwd) = &config.cwd {
            cmd.cwd(cwd);
        }

        // 注入用户环境变量
        if let Some(env) = &config.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        // 注入代理环境变量
        if let Some(proxy_env) = &config.proxy_env {
            for (k, v) in proxy_env {
                cmd.env(k, v);
            }
        }

        // 清除可能从父进程继承的干扰变量
        cmd.env_remove("CLAUDECODE");

        // 强制 UTF-8
        cmd.env("LANG", "en_US.UTF-8");

        let child = pair.slave.spawn_command(cmd)?;

        Ok(PtyHandle {
            master: pair.master,
            child,
        })
    }
}
