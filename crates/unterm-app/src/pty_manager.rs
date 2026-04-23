//! 直连 PTY 管理器
//!
//! 在 Tauri 进程内直接创建 PTY，通过 Tauri Event 推送输出到前端，
//! 绕过 TCP/JSON-RPC 瓶颈，延迟 <1ms。

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// PTY 会话
struct PtySession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    alive: Arc<AtomicBool>,
}

/// 直连 PTY 管理器
pub struct DirectPtyManager {
    sessions: HashMap<u64, PtySession>,
}

/// PTY 输出事件 payload
#[derive(Clone, Serialize, Deserialize)]
pub struct PtyOutputPayload {
    pub pane_id: u64,
    pub data: String,
}

/// PTY 退出事件 payload
#[derive(Clone, Serialize, Deserialize)]
pub struct PtyExitPayload {
    pub pane_id: u64,
}

impl DirectPtyManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// 创建新的 PTY 会话
    pub fn create_session(
        &mut self,
        app_handle: AppHandle,
        pane_id: u64,
        shell: Option<String>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        cols: u16,
        rows: u16,
    ) -> Result<(), String> {
        // 如果已存在则先销毁
        if self.sessions.contains_key(&pane_id) {
            self.destroy_session(pane_id);
        }

        let pty_system = native_pty_system();

        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| format!("打开 PTY 失败: {}", e))?;

        // 确定默认 shell
        let shell_cmd = shell.unwrap_or_else(|| {
            if cfg!(windows) {
                "pwsh.exe".to_string()
            } else {
                std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
            }
        });

        let mut cmd = CommandBuilder::new(&shell_cmd);

        // 设置工作目录
        if let Some(ref dir) = cwd {
            cmd.cwd(dir);
        }

        // 设置环境变量
        if let Some(ref envs) = env {
            for (k, v) in envs {
                cmd.env(k, v);
            }
        }

        // 启动子进程
        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("启动 shell 失败: {}", e))?;

        // 丢弃 slave 端，master 端负责 I/O
        drop(pair.slave);

        // 获取 reader 和 writer
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("克隆 PTY reader 失败: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("获取 PTY writer 失败: {}", e))?;

        let alive = Arc::new(AtomicBool::new(true));
        let alive_clone = alive.clone();

        // 启动 reader 线程
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // PTY 关闭
                        alive_clone.store(false, Ordering::SeqCst);
                        let _ = app_handle.emit(
                            &format!("pty-exit-{}", pane_id),
                            PtyExitPayload { pane_id },
                        );
                        break;
                    }
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = app_handle.emit(
                            &format!("pty-output-{}", pane_id),
                            PtyOutputPayload {
                                pane_id,
                                data,
                            },
                        );
                    }
                    Err(_) => {
                        alive_clone.store(false, Ordering::SeqCst);
                        let _ = app_handle.emit(
                            &format!("pty-exit-{}", pane_id),
                            PtyExitPayload { pane_id },
                        );
                        break;
                    }
                }
            }
        });

        self.sessions.insert(
            pane_id,
            PtySession {
                writer,
                master: pair.master,
                alive,
            },
        );

        Ok(())
    }

    /// 向 PTY 写入数据
    pub fn write_input(&mut self, pane_id: u64, data: &[u8]) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(&pane_id)
            .ok_or_else(|| format!("PTY 会话 {} 不存在", pane_id))?;

        session
            .writer
            .write_all(data)
            .map_err(|e| format!("写入 PTY 失败: {}", e))?;

        session
            .writer
            .flush()
            .map_err(|e| format!("刷新 PTY 失败: {}", e))?;

        Ok(())
    }

    /// 调整 PTY 尺寸
    pub fn resize(&mut self, pane_id: u64, cols: u16, rows: u16) -> Result<(), String> {
        let session = self
            .sessions
            .get(&pane_id)
            .ok_or_else(|| format!("PTY 会话 {} 不存在", pane_id))?;

        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("调整 PTY 尺寸失败: {}", e))?;

        Ok(())
    }

    /// 销毁 PTY 会话
    pub fn destroy_session(&mut self, pane_id: u64) {
        if let Some(session) = self.sessions.remove(&pane_id) {
            session.alive.store(false, Ordering::SeqCst);
            // writer 和 master 在 drop 时自动关闭
            drop(session);
        }
    }
}
