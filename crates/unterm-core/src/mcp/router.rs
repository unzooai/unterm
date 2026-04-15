//! MCP Tool 路由器

use std::sync::Arc;

use serde_json::json;

use crate::session::SessionManager;
use super::protocol::*;
use unterm_proto::session::CreateSessionRequest;

/// MCP 路由器
pub struct McpRouter {
    session_manager: Arc<SessionManager>,
}

impl McpRouter {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self { session_manager }
    }

    /// 路由 JSON-RPC 请求到对应的 handler
    pub fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            // Session 管理
            "session.create" => self.handle_session_create(req.id, req.params),
            "session.list" => self.handle_session_list(req.id),
            "session.destroy" => self.handle_session_destroy(req.id, req.params),
            "session.resize" => self.handle_session_resize(req.id, req.params),
            "session.status" => self.handle_session_status(req.id, req.params),

            // 命令执行
            "exec.run" => self.handle_exec_run(req.id, req.params),
            "exec.send" => self.handle_exec_send(req.id, req.params),

            // 屏幕读取
            "screen.read" => self.handle_screen_read(req.id, req.params),

            // 其他方法暂未实现
            _ => JsonRpcResponse::error(
                req.id,
                JsonRpcError::method_not_found(&req.method),
            ),
        }
    }

    /// 创建新 session
    fn handle_session_create(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let req: CreateSessionRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(&e.to_string()),
                )
            }
        };
        match self.session_manager.create_session(req) {
            Ok(info) => JsonRpcResponse::success(id, serde_json::to_value(info).unwrap()),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 列出所有 session
    fn handle_session_list(&self, id: serde_json::Value) -> JsonRpcResponse {
        let sessions = self.session_manager.list_sessions();
        JsonRpcResponse::success(id, serde_json::to_value(sessions).unwrap())
    }

    /// 销毁 session
    fn handle_session_destroy(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing session_id"),
                )
            }
        };
        match self.session_manager.destroy_session(&session_id) {
            Ok(()) => JsonRpcResponse::success(id, json!({"destroyed": true})),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 调整 session 尺寸
    fn handle_session_resize(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing session_id"),
                )
            }
        };
        let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
        let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
        match self.session_manager.resize_session(&session_id, cols, rows) {
            Ok(()) => JsonRpcResponse::success(id, json!({"resized": true})),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 查询 session 状态
    fn handle_session_status(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing session_id"),
                )
            }
        };
        match self.session_manager.get_session(session_id) {
            Some(info) => JsonRpcResponse::success(id, serde_json::to_value(info).unwrap()),
            None => JsonRpcResponse::error(id, JsonRpcError::internal_error("Session not found")),
        }
    }

    /// 执行命令（自动追加换行）
    fn handle_exec_run(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing session_id"),
                )
            }
        };
        let command = match params.get("command").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing command"),
                )
            }
        };
        match self
            .session_manager
            .send_input(&session_id, &format!("{}\n", command))
        {
            Ok(()) => JsonRpcResponse::success(id, json!({"sent": true})),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 发送原始输入（不追加换行）
    fn handle_exec_send(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing session_id"),
                )
            }
        };
        let input = match params.get("input").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing input"),
                )
            }
        };
        match self.session_manager.send_input(&session_id, &input) {
            Ok(()) => JsonRpcResponse::success(id, json!({"sent": true})),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 读取屏幕内容
    fn handle_screen_read(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing session_id"),
                )
            }
        };
        match self.session_manager.read_screen(&session_id) {
            Ok(content) => JsonRpcResponse::success(id, json!({"content": content})),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }
}
