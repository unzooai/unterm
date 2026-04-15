//! Core 通信桥接模块
//!
//! 解决 winit 同步事件循环与 tokio 异步 IPC 的桥接问题。
//! 后台线程运行 tokio runtime，通过 std::sync::mpsc channel 与 UI 线程通信。

use std::collections::HashMap;
use std::sync::mpsc;

/// UI 线程发给后台的命令
#[derive(Debug)]
pub enum UiCommand {
    /// 为指定 pane 创建新 session
    CreateSession {
        pane_id: u64,
        shell: Option<String>,
        cwd: Option<String>,
    },
    /// 发送键盘输入到 session
    SendInput {
        session_id: String,
        input: Vec<u8>,
    },
    /// 调整 session 尺寸
    ResizeSession {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    /// 销毁 session
    DestroySession { session_id: String },
}

/// 后台发给 UI 线程的事件
#[derive(Debug, Clone)]
pub enum CoreEvent {
    /// session 创建成功
    SessionCreated { pane_id: u64, session_id: String },
    /// 屏幕内容更新
    ScreenUpdate { session_id: String, content: String },
    /// 已连接到 core
    Connected,
    /// 连接断开
    Disconnected,
    /// 错误
    Error(String),
}

/// Core 通信桥接器
///
/// 在后台线程运行 tokio runtime，通过 channel 与 UI 线程通信。
pub struct CoreBridge {
    /// 发送命令到后台
    cmd_tx: mpsc::Sender<UiCommand>,
    /// 接收后台事件
    event_rx: mpsc::Receiver<CoreEvent>,
    /// pane_id -> session_id 映射
    pane_sessions: HashMap<u64, String>,
    /// session_id -> 最新屏幕内容缓存
    screen_cache: HashMap<String, String>,
    /// 是否已连接
    pub connected: bool,
}

impl CoreBridge {
    /// 启动后台通信线程
    ///
    /// 参数：
    /// - `core_address`: daemon 地址（如 "127.0.0.1:19876"）
    /// - `poll_interval_ms`: 屏幕轮询间隔（毫秒）
    pub fn start(core_address: String, poll_interval_ms: u64) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<UiCommand>();
        let (event_tx, event_rx) = mpsc::channel::<CoreEvent>();

        // 后台线程运行 tokio runtime
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("无法创建 tokio runtime");
            rt.block_on(async move {
                Self::background_loop(core_address, poll_interval_ms, cmd_rx, event_tx).await;
            });
        });

        Self {
            cmd_tx,
            event_rx,
            pane_sessions: HashMap::new(),
            screen_cache: HashMap::new(),
            connected: false,
        }
    }

    /// 后台异步主循环
    ///
    /// 负责连接 core daemon、处理 UI 命令、轮询屏幕内容。
    /// reader/writer 都用 `Arc<tokio::sync::Mutex>` 包装，保证同一时间
    /// 只有一个请求在发送+接收，避免轮询 task 和主循环的响应对不上。
    async fn background_loop(
        address: String,
        poll_interval_ms: u64,
        cmd_rx: mpsc::Receiver<UiCommand>,
        event_tx: mpsc::Sender<CoreEvent>,
    ) {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::Arc;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpStream;
        use tokio::sync::Mutex;

        // 重试连接，直到成功
        let stream = loop {
            match TcpStream::connect(&address).await {
                Ok(s) => {
                    let _ = event_tx.send(CoreEvent::Connected);
                    break s;
                }
                Err(e) => {
                    tracing::debug!("连接 core 失败，1秒后重试: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        };

        let (read_half, write_half) = tokio::io::split(stream);
        // 用 Arc<Mutex> 包装，保证轮询 task 和主循环互斥访问
        let reader = Arc::new(Mutex::new(BufReader::new(read_half)));
        let writer = Arc::new(Mutex::new(write_half));
        let next_id = AtomicU64::new(1);

        // 追踪活跃的 session_id 列表（用于屏幕轮询）
        let active_sessions: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // 启动屏幕轮询 task
        let poll_reader = reader.clone();
        let poll_writer = writer.clone();
        let poll_sessions = active_sessions.clone();
        let poll_event_tx = event_tx.clone();
        // 轮询 task 使用独立的 id 区间，避免与主循环冲突
        let poll_next_id = Arc::new(AtomicU64::new(10000));

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(poll_interval_ms));
            loop {
                interval.tick().await;
                let sessions = poll_sessions.lock().await.clone();
                for session_id in &sessions {
                    let id = poll_next_id.fetch_add(1, Ordering::Relaxed);
                    let req = serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "screen.read",
                        "params": {"session_id": session_id},
                        "id": id
                    });
                    let req_str = format!("{}\n", serde_json::to_string(&req).unwrap());

                    // 获取写锁 -> 发请求
                    let mut w = poll_writer.lock().await;
                    if w.write_all(req_str.as_bytes()).await.is_err() {
                        let _ = poll_event_tx.send(CoreEvent::Disconnected);
                        return;
                    }
                    let _ = w.flush().await;
                    drop(w);

                    // 获取读锁 -> 读响应
                    let mut r = poll_reader.lock().await;
                    let mut line = String::new();
                    match r.read_line(&mut line).await {
                        Ok(0) | Err(_) => {
                            let _ = poll_event_tx.send(CoreEvent::Disconnected);
                            return;
                        }
                        Ok(_) => {
                            if let Ok(resp) =
                                serde_json::from_str::<serde_json::Value>(line.trim())
                            {
                                if let Some(result) = resp.get("result") {
                                    if let Some(content) =
                                        result.get("content").and_then(|c| c.as_str())
                                    {
                                        let _ = poll_event_tx.send(CoreEvent::ScreenUpdate {
                                            session_id: session_id.clone(),
                                            content: content.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    drop(r);
                }
            }
        });

        // 主循环：处理 UI 命令
        loop {
            // 用 try_recv 非阻塞检查，避免阻塞 tokio
            match cmd_rx.try_recv() {
                Ok(cmd) => match cmd {
                    UiCommand::CreateSession {
                        pane_id,
                        shell,
                        cwd,
                    } => {
                        let id = next_id.fetch_add(1, Ordering::Relaxed);
                        let req = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "session.create",
                            "params": {
                                "shell": shell,
                                "cwd": cwd,
                                "name": format!("pane-{}", pane_id),
                            },
                            "id": id
                        });
                        let req_str = format!("{}\n", serde_json::to_string(&req).unwrap());

                        let mut w = writer.lock().await;
                        if w.write_all(req_str.as_bytes()).await.is_err() {
                            let _ = event_tx.send(CoreEvent::Disconnected);
                            break;
                        }
                        let _ = w.flush().await;
                        drop(w);

                        let mut r = reader.lock().await;
                        let mut line = String::new();
                        match r.read_line(&mut line).await {
                            Ok(0) | Err(_) => {
                                let _ = event_tx.send(CoreEvent::Disconnected);
                                break;
                            }
                            Ok(_) => {
                                if let Ok(resp) =
                                    serde_json::from_str::<serde_json::Value>(line.trim())
                                {
                                    if let Some(result) = resp.get("result") {
                                        if let Some(session_id) =
                                            result.get("id").and_then(|v| v.as_str())
                                        {
                                            active_sessions
                                                .lock()
                                                .await
                                                .push(session_id.to_string());
                                            let _ = event_tx.send(CoreEvent::SessionCreated {
                                                pane_id,
                                                session_id: session_id.to_string(),
                                            });
                                        }
                                    } else if let Some(error) = resp.get("error") {
                                        let msg = error
                                            .get("message")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("未知错误");
                                        let _ = event_tx.send(CoreEvent::Error(msg.to_string()));
                                    }
                                }
                            }
                        }
                        drop(r);
                    }

                    UiCommand::SendInput { session_id, input } => {
                        let id = next_id.fetch_add(1, Ordering::Relaxed);
                        let input_str = String::from_utf8_lossy(&input);
                        let req = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "exec.send",
                            "params": {
                                "session_id": session_id,
                                "input": input_str,
                            },
                            "id": id
                        });
                        let req_str = format!("{}\n", serde_json::to_string(&req).unwrap());

                        let mut w = writer.lock().await;
                        let _ = w.write_all(req_str.as_bytes()).await;
                        let _ = w.flush().await;
                        drop(w);

                        // 读取响应（不需要处理结果）
                        let mut r = reader.lock().await;
                        let mut line = String::new();
                        let _ = r.read_line(&mut line).await;
                        drop(r);
                    }

                    UiCommand::ResizeSession {
                        session_id,
                        cols,
                        rows,
                    } => {
                        let id = next_id.fetch_add(1, Ordering::Relaxed);
                        let req = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "session.resize",
                            "params": {
                                "session_id": session_id,
                                "cols": cols,
                                "rows": rows,
                            },
                            "id": id
                        });
                        let req_str = format!("{}\n", serde_json::to_string(&req).unwrap());

                        let mut w = writer.lock().await;
                        let _ = w.write_all(req_str.as_bytes()).await;
                        let _ = w.flush().await;
                        drop(w);

                        let mut r = reader.lock().await;
                        let mut line = String::new();
                        let _ = r.read_line(&mut line).await;
                        drop(r);
                    }

                    UiCommand::DestroySession { session_id } => {
                        // 从活跃列表移除，停止轮询该 session
                        active_sessions
                            .lock()
                            .await
                            .retain(|s| s != &session_id);

                        let id = next_id.fetch_add(1, Ordering::Relaxed);
                        let req = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "session.destroy",
                            "params": {
                                "session_id": session_id,
                            },
                            "id": id
                        });
                        let req_str = format!("{}\n", serde_json::to_string(&req).unwrap());

                        let mut w = writer.lock().await;
                        let _ = w.write_all(req_str.as_bytes()).await;
                        let _ = w.flush().await;
                        drop(w);

                        let mut r = reader.lock().await;
                        let mut line = String::new();
                        let _ = r.read_line(&mut line).await;
                        drop(r);
                    }
                },
                Err(mpsc::TryRecvError::Empty) => {
                    // 没有命令，短暂 sleep 避免忙等
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // UI 线程已退出
                    break;
                }
            }
        }
    }

    /// 发送命令到后台（非阻塞）
    pub fn send_command(&self, cmd: UiCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// 轮询后台事件（非阻塞），更新内部状态
    ///
    /// 在 winit 事件循环中每帧调用，返回本次收到的所有事件。
    pub fn poll_events(&mut self) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            match &event {
                CoreEvent::Connected => {
                    self.connected = true;
                }
                CoreEvent::Disconnected => {
                    self.connected = false;
                }
                CoreEvent::SessionCreated {
                    pane_id,
                    session_id,
                } => {
                    self.pane_sessions.insert(*pane_id, session_id.clone());
                }
                CoreEvent::ScreenUpdate {
                    session_id,
                    content,
                } => {
                    self.screen_cache
                        .insert(session_id.clone(), content.clone());
                }
                CoreEvent::Error(_) => {}
            }
            events.push(event);
        }
        events
    }

    /// 获取 pane 对应的 session_id
    pub fn get_session_id(&self, pane_id: u64) -> Option<&str> {
        self.pane_sessions.get(&pane_id).map(|s| s.as_str())
    }

    /// 获取 session 的最新屏幕内容
    pub fn get_screen_content(&self, session_id: &str) -> Option<&str> {
        self.screen_cache.get(session_id).map(|s| s.as_str())
    }

    /// 获取 pane 的屏幕内容（通过 pane_id 查找）
    pub fn get_pane_content(&self, pane_id: u64) -> Option<&str> {
        self.pane_sessions
            .get(&pane_id)
            .and_then(|sid| self.screen_cache.get(sid))
            .map(|s| s.as_str())
    }

    /// 为 pane 创建新 session
    pub fn create_session_for_pane(
        &self,
        pane_id: u64,
        shell: Option<String>,
        cwd: Option<String>,
    ) {
        self.send_command(UiCommand::CreateSession {
            pane_id,
            shell,
            cwd,
        });
    }

    /// 向 pane 发送输入
    pub fn send_input_to_pane(&self, pane_id: u64, input: Vec<u8>) {
        if let Some(session_id) = self.pane_sessions.get(&pane_id) {
            self.send_command(UiCommand::SendInput {
                session_id: session_id.clone(),
                input,
            });
        }
    }

    /// 删除 pane 关联的 session
    pub fn destroy_pane_session(&mut self, pane_id: u64) {
        if let Some(session_id) = self.pane_sessions.remove(&pane_id) {
            self.screen_cache.remove(&session_id);
            self.send_command(UiCommand::DestroySession { session_id });
        }
    }
}
