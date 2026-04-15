//! Session 生命周期管理
//! 每个 session 对应一个 PTY 进程 + VT 解析器。

use std::collections::HashMap;
use std::io::Read;
use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use tracing::info;

use unterm_proto::session::*;
use crate::pty::{PtyConfig, PtyHandle, PtyManager};

/// 单个 Session
pub struct Session {
    pub info: SessionInfo,
    pub pty_handle: Mutex<PtyHandle>,
    /// PTY writer，创建时通过 take_writer 获取
    pub writer: Mutex<Box<dyn std::io::Write + Send>>,
    /// PTY 输出累积缓冲
    pub output_buffer: Arc<RwLock<String>>,
}

/// Session 管理器
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    pty_manager: Mutex<PtyManager>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            pty_manager: Mutex::new(PtyManager::new()),
        }
    }

    /// 创建新 session
    pub fn create_session(&self, req: CreateSessionRequest) -> Result<SessionInfo> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let shell = req.shell.clone().unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                // 优先 pwsh (PowerShell 7+)，回退 powershell
                std::env::var_os("PATH")
                    .and_then(|paths| {
                        if std::env::split_paths(&paths).any(|d| d.join("pwsh.exe").exists()) {
                            Some("pwsh.exe".into())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "powershell.exe".into())
            } else {
                "/bin/zsh".into()
            }
        });
        let cwd = req.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".into())
        });

        let config = PtyConfig {
            shell: Some(shell.clone()),
            cwd: Some(cwd.clone()),
            env: req.env.clone(),
            cols: 80,
            rows: 24,
            proxy_env: None,
        };

        let pty_handle = self.pty_manager.lock().create_pty(config)?;

        let info = SessionInfo {
            id: id.clone(),
            name: req.name.clone(),
            status: SessionStatus::Running,
            policy: SessionPolicy::Full,
            shell,
            cwd,
            created_at: now.clone(),
            last_activity: now,
            cols: 80,
            rows: 24,
        };

        let output_buffer = Arc::new(RwLock::new(String::new()));

        // 启动 PTY 输出读取线程
        let buffer_clone = output_buffer.clone();
        let mut reader = pty_handle.master.try_clone_reader()
            .map_err(|e| anyhow::anyhow!("无法克隆 PTY reader: {}", e))?;

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(text) = String::from_utf8(buf[..n].to_vec()) {
                            buffer_clone.write().push_str(&text);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // 提前获取 writer（take_writer 只能调用一次）
        let writer = pty_handle.master.take_writer()
            .map_err(|e| anyhow::anyhow!("无法获取 PTY writer: {}", e))?;

        let session = Session {
            info: info.clone(),
            pty_handle: Mutex::new(pty_handle),
            writer: Mutex::new(writer),
            output_buffer,
        };

        self.sessions.write().insert(id, session);
        info!("Session 已创建: {}", &info.id);

        Ok(info)
    }

    /// 列出所有活跃 session
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions.read().values().map(|s| s.info.clone()).collect()
    }

    /// 获取指定 session 信息
    pub fn get_session(&self, id: &str) -> Option<SessionInfo> {
        self.sessions.read().get(id).map(|s| s.info.clone())
    }

    /// 销毁 session
    pub fn destroy_session(&self, id: &str) -> Result<()> {
        if let Some(session) = self.sessions.write().remove(id) {
            session.pty_handle.lock().child.kill()?;
            info!("Session 已销毁: {}", id);
            Ok(())
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 调整 session 尺寸
    pub fn resize_session(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(id) {
            session.pty_handle.lock().resize(cols, rows)?;
            session.info.cols = cols;
            session.info.rows = rows;
            Ok(())
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 向 session 发送输入
    pub fn send_input(&self, id: &str, input: &str) -> Result<()> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            use std::io::Write;
            let mut writer = session.writer.lock();
            writer.write_all(input.as_bytes())?;
            writer.flush()?;
            Ok(())
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }

    /// 读取 session 屏幕内容
    pub fn read_screen(&self, id: &str) -> Result<String> {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(id) {
            Ok(session.output_buffer.read().clone())
        } else {
            anyhow::bail!("Session 未找到: {}", id)
        }
    }
}
