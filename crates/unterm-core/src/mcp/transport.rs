//! IPC 传输层
//! Windows: Named Pipe, macOS/Linux: Unix Socket

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

use super::protocol::JsonRpcRequest;
use super::router::McpRouter;

/// IPC Server
pub struct IpcServer {
    router: Arc<McpRouter>,
}

impl IpcServer {
    pub fn new(router: Arc<McpRouter>) -> Self {
        Self { router }
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
        info!("MCP Server 已启动 (TCP 127.0.0.1:19876，后续切换为 IPC)");

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("新客户端连接: {}", addr);
            let router = self.router.clone();

            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();

                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            debug!("客户端断开: {}", addr);
                            break;
                        }
                        Ok(_) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                                Ok(req) => {
                                    debug!("收到请求: {} (id={:?})", req.method, req.id);
                                    let resp = router.handle_request(req);
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
