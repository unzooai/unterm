//! MCP IPC 客户端
//! 连接 unterm-core daemon，发送 JSON-RPC 请求

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

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

/// MCP 客户端
pub struct McpClient {
    stream: TcpStream,
    next_id: u64,
}

impl McpClient {
    /// 连接到 unterm-core（自动认证）
    pub fn connect() -> Result<Self> {
        // 临时使用 TCP，后续切换为 IPC
        let stream = TcpStream::connect("127.0.0.1:19876")
            .map_err(|_| anyhow::anyhow!("{}", rust_i18n::t!("messages.not_running")))?;
        let mut client = Self { stream, next_id: 1 };

        // 自动认证：读取 ~/.unterm/auth_token
        let token_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".unterm")
            .join("auth_token");
        if let Ok(token) = std::fs::read_to_string(&token_path) {
            let token = token.trim().to_string();
            if !token.is_empty() {
                client.call("auth.login", serde_json::json!({"token": token}))?;
            }
        }

        Ok(client)
    }

    /// 发送 JSON-RPC 请求并获取响应
    pub fn call(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id: self.next_id,
        };
        self.next_id += 1;

        let req_json = serde_json::to_string(&req)?;
        writeln!(self.stream, "{}", req_json)?;
        self.stream.flush()?;

        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let resp: JsonRpcResponse = serde_json::from_str(line.trim())?;

        if let Some(error) = resp.error {
            anyhow::bail!("{}", error.message);
        }

        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }
}
