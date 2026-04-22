//! IPC 传输层
//! Windows: Named Pipe, macOS/Linux: Unix Socket

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};

use super::protocol::JsonRpcRequest;
use super::router::McpRouter;

/// IPC Server
pub struct IpcServer {
    router: Arc<McpRouter>,
    /// 认证 token（启动时随机生成）
    auth_token: String,
}

impl IpcServer {
    pub fn new(router: Arc<McpRouter>) -> Self {
        let token = uuid::Uuid::new_v4().to_string();
        Self { router, auth_token: token }
    }

    /// 获取认证 token（供内嵌模式使用）
    pub fn auth_token(&self) -> &str {
        &self.auth_token
    }

    /// token 文件路径
    pub fn token_file_path() -> String {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into());
        format!("{}/.unterm/auth_token", home)
    }

    /// 获取 IPC 监听地址
    fn socket_path() -> String {
        if cfg!(target_os = "windows") {
            r"\\.\pipe\unterm".to_string()
        } else {
            std::env::var("XDG_RUNTIME_DIR")
                .map(|dir| format!("{}/unterm.sock", dir))
                .unwrap_or_else(|_| "/tmp/unterm.sock".to_string())
        }
    }

    /// 启动 IPC Server
    pub async fn start(&self) -> anyhow::Result<()> {
        let path = Self::socket_path();
        info!("MCP Server 监听: {}", path);

        // 使用 tokio 的 TCP listener 作为临时方案
        // 后续替换为 interprocess 的 named pipe / unix socket
        let listener = tokio::net::TcpListener::bind("127.0.0.1:19876").await?;
        info!("MCP Server 已启动 (TCP 127.0.0.1:19876)");

        // 将 token 写入文件供外部客户端读取
        let token_path = Self::token_file_path();
        if let Some(parent) = std::path::Path::new(&token_path).parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        tokio::fs::write(&token_path, &self.auth_token).await?;
        info!("认证 token 已写入: {}", token_path);

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("新客户端连接: {}", addr);
            let router = self.router.clone();
            let auth_token = self.auth_token.clone();

            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();
                let mut authenticated = false;

                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            debug!("客户端断开: {}", addr);
                            break;
                        }
                        Ok(_) => {
                            let trimmed = line.trim().to_string();
                            if trimmed.is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<JsonRpcRequest>(&trimmed) {
                                Ok(req) => {
                                    // 首次请求必须是 auth.login
                                    if !authenticated {
                                        if req.method == "auth.login" {
                                            let token = req.params.get("token").and_then(|v| v.as_str()).unwrap_or("");
                                            if token == auth_token {
                                                authenticated = true;
                                                let resp = super::protocol::JsonRpcResponse::success(
                                                    req.id,
                                                    serde_json::json!({"authenticated": true}),
                                                );
                                                let resp_json = serde_json::to_string(&resp).unwrap();
                                                let _ = writer.write_all(format!("{}\n", resp_json).as_bytes()).await;
                                                continue;
                                            } else {
                                                warn!("认证失败: {}", addr);
                                                let resp = super::protocol::JsonRpcResponse::error(
                                                    req.id,
                                                    super::protocol::JsonRpcError::internal_error("认证失败"),
                                                );
                                                let resp_json = serde_json::to_string(&resp).unwrap();
                                                let _ = writer.write_all(format!("{}\n", resp_json).as_bytes()).await;
                                                break;
                                            }
                                        } else {
                                            warn!("未认证的请求，拒绝: {} from {}", req.method, addr);
                                            let resp = super::protocol::JsonRpcResponse::error(
                                                req.id,
                                                super::protocol::JsonRpcError::internal_error("请先调用 auth.login 进行认证"),
                                            );
                                            let resp_json = serde_json::to_string(&resp).unwrap();
                                            let _ = writer.write_all(format!("{}\n", resp_json).as_bytes()).await;
                                            break;
                                        }
                                    }

                                    debug!("收到请求: {} (id={:?})", req.method, req.id);
                                    // 在 spawn_blocking 中执行，避免阻塞 tokio worker
                                    let router_clone = router.clone();
                                    let resp = tokio::task::spawn_blocking(move || {
                                        router_clone.handle_request(req)
                                    }).await.unwrap();
                                    let resp_json = serde_json::to_string(&resp).unwrap();
                                    if let Err(e) = writer
                                        .write_all(format!("{}\n", resp_json).as_bytes())
                                        .await
                                    {
                                        error!("发送响应失败: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("解析请求失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("读取失败: {}", e);
                            break;
                        }
                    }
                }
            });
        }
    }
}
