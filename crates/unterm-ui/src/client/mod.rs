//! IPC Client 模块
//! 连接 unterm-core daemon，发送/接收 MCP 消息。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i32,
    message: String,
}

/// 异步 MCP 客户端
pub struct McpClient {
    writer: Arc<Mutex<tokio::io::WriteHalf<TcpStream>>>,
    reader: Arc<Mutex<BufReader<tokio::io::ReadHalf<TcpStream>>>>,
    next_id: Arc<Mutex<u64>>,
}

impl McpClient {
    /// 连接到 unterm-core
    pub async fn connect() -> Result<Self> {
        let stream = TcpStream::connect("127.0.0.1:19876").await?;
        let (read_half, write_half) = tokio::io::split(stream);
        Ok(Self {
            writer: Arc::new(Mutex::new(write_half)),
            reader: Arc::new(Mutex::new(BufReader::new(read_half))),
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    /// 发送 JSON-RPC 请求
    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = {
            let mut next_id = self.next_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id,
        };

        let req_json = serde_json::to_string(&req)?;
        let mut writer = self.writer.lock().await;
        writer.write_all(format!("{}\n", req_json).as_bytes()).await?;
        writer.flush().await?;

        let mut reader = self.reader.lock().await;
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let resp: JsonRpcResponse = serde_json::from_str(line.trim())?;

        if let Some(error) = resp.error {
            anyhow::bail!("{}", error.message);
        }

        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }
}
