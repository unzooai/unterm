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
            "screen.read_raw" => self.handle_screen_read_raw(req.id, req.params),

            // 图片管理（AI multimodal 支持）
            "image.store" => self.handle_image_store(req.id, req.params),
            "image.list" => self.handle_image_list(req.id, req.params),
            "image.get" => self.handle_image_get(req.id, req.params),

            // Session 历史
            "session.history" => self.handle_session_history(req.id, req.params),

            // 信号处理
            "signal.send" => self.handle_signal_send(req.id, req.params),

            // 屏幕扩展
            "screen.cursor" => self.handle_screen_cursor(req.id, req.params),
            "screen.scroll" => self.handle_screen_scroll(req.id, req.params),

            // AI agent 编排调度
            "orchestrate.launch" => self.handle_orchestrate_launch(req.id, req.params),
            "orchestrate.broadcast" => self.handle_orchestrate_broadcast(req.id, req.params),
            "orchestrate.wait" => self.handle_orchestrate_wait(req.id, req.params),

            // 代理管理
            "proxy.status" => self.handle_proxy_status(req.id),
            "proxy.nodes" => self.handle_proxy_nodes(req.id),
            "proxy.switch" => self.handle_proxy_switch(req.id, req.params),
            "proxy.speedtest" => self.handle_proxy_speedtest(req.id, req.params),

            // 工作区快照
            "workspace.save" => self.handle_workspace_save(req.id, req.params),
            "workspace.restore" => self.handle_workspace_restore(req.id, req.params),
            "workspace.list" => self.handle_workspace_list(req.id),

            // 截图与剪贴板
            "capture.screen" => self.handle_capture_screen(req.id),
            "capture.window" => self.handle_capture_window(req.id, req.params),
            "capture.select" => self.handle_capture_select(req.id),
            "capture.clipboard" => self.handle_capture_clipboard(req.id),

            // ──────── AI 完全控制接口 ────────
            "exec.run_wait" => self.handle_exec_run_wait(req.id, req.params),
            "exec.status" => self.handle_exec_status(req.id, req.params),
            "exec.cancel" => self.handle_exec_cancel(req.id, req.params),
            "screen.text" => self.handle_screen_text(req.id, req.params),
            "screen.search" => self.handle_screen_search(req.id, req.params),
            "session.idle" => self.handle_session_idle(req.id, req.params),
            "session.cwd" => self.handle_session_cwd(req.id, req.params),
            "session.env" => self.handle_session_env(req.id, req.params),
            "session.set_env" => self.handle_session_set_env(req.id, req.params),
            "session.audit_log" => self.handle_session_audit_log(req.id, req.params),
            "system.info" => self.handle_system_info(req.id),
            "policy.set" => self.handle_policy_set(req.id, req.params),
            "policy.check" => self.handle_policy_check(req.id, req.params),

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
        // 策略检查
        let policy_result = self.session_manager.check_policy(&command);
        if policy_result.get("allowed").and_then(|v| v.as_bool()) == Some(false) {
            let reason = policy_result.get("reason").and_then(|v| v.as_str()).unwrap_or("blocked");
            return JsonRpcResponse::error(id, JsonRpcError::internal_error(reason));
        }

        match self
            .session_manager
            .send_input(&session_id, &format!("{}\r", command))
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
            Ok(screen_data) => JsonRpcResponse::success(id, screen_data),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 读取原始 PTY 输出（供 xterm.js 直接消费）
    fn handle_screen_read_raw(
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
        match self.session_manager.read_raw_output(&session_id) {
            Ok(data) => {
                // base64 编码二进制数据
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                JsonRpcResponse::success(id, serde_json::json!({
                    "content": b64,
                    "encoding": "base64",
                    "length": data.len(),
                }))
            }
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 存储图片到 session
    fn handle_image_store(
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
        let data = match params.get("data").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing data (base64)"),
                )
            }
        };
        let mime_type = params
            .get("mime_type")
            .and_then(|v| v.as_str())
            .unwrap_or("image/png")
            .to_string();

        match self.session_manager.store_image(&session_id, data, mime_type) {
            Ok(image_id) => JsonRpcResponse::success(id, json!({"image_id": image_id})),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 列出 session 中的所有图片
    fn handle_image_list(
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
        match self.session_manager.list_images(session_id) {
            Ok(images) => JsonRpcResponse::success(id, json!({"images": images})),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// 获取指定图片（含 base64 数据，供 AI vision 使用）
    fn handle_image_get(
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
        let image_id = match params.get("image_id").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("missing image_id"),
                )
            }
        };
        match self.session_manager.get_image(session_id, image_id) {
            Ok(Some(img)) => JsonRpcResponse::success(id, json!({
                "image_id": img.id,
                "data": img.data_base64,
                "mime_type": img.mime_type,
                "timestamp": img.timestamp,
            })),
            Ok(None) => JsonRpcResponse::error(id, JsonRpcError::internal_error("Image not found")),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    // ──────── session.history ────────

    fn handle_session_history(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        let since = params.get("since").and_then(|v| v.as_str()).map(|s| s.to_string());
        let limit = params.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32);
        match self.session_manager.get_history(&session_id, since.as_deref(), limit) {
            Ok(entries) => JsonRpcResponse::success(id, serde_json::to_value(entries).unwrap()),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    // ──────── signal.send ────────

    fn handle_signal_send(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        let signal = match params.get("signal").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing signal")),
        };
        match self.session_manager.send_signal(&session_id, &signal) {
            Ok(()) => JsonRpcResponse::success(id, json!({"sent": true, "signal": signal})),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    // ──────── screen.cursor ────────

    fn handle_screen_cursor(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        match self.session_manager.read_cursor(&session_id) {
            Ok(cursor) => JsonRpcResponse::success(id, cursor),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    // ──────── screen.scroll ────────

    fn handle_screen_scroll(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(100) as u32;
        match self.session_manager.read_scrollback(&session_id, offset, count) {
            Ok(data) => JsonRpcResponse::success(id, data),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    // ──────── orchestrate.launch ────────

    fn handle_orchestrate_launch(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let command = match params.get("command").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing command")),
        };
        let name = params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let cwd = params.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
        match self.session_manager.launch(&command, name, cwd) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    // ──────── orchestrate.broadcast ────────

    fn handle_orchestrate_broadcast(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let command = match params.get("command").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing command")),
        };
        let sessions: Vec<String> = match params.get("sessions").and_then(|v| v.as_array()) {
            Some(arr) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing sessions")),
        };
        match self.session_manager.broadcast(&command, &sessions) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    // ──────── orchestrate.wait ────────

    fn handle_orchestrate_wait(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        let pattern = match params.get("pattern").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing pattern")),
        };
        let timeout_ms = params.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(10000);
        match self.session_manager.wait_for_pattern(&session_id, &pattern, timeout_ms) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    // ──────── proxy.* (代理功能由 App 层管理，Core 层返回提示) ────────

    fn handle_proxy_status(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(id, json!({
            "enabled": false,
            "message": "代理功能由 App 层管理，请通过 UI 操作",
        }))
    }

    fn handle_proxy_nodes(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(id, json!({
            "nodes": [],
            "message": "代理功能由 App 层管理，请通过 UI 操作",
        }))
    }

    fn handle_proxy_switch(
        &self,
        id: serde_json::Value,
        _params: serde_json::Value,
    ) -> JsonRpcResponse {
        JsonRpcResponse::error(id, JsonRpcError::internal_error("代理切换功能由 App 层管理"))
    }

    fn handle_proxy_speedtest(
        &self,
        id: serde_json::Value,
        _params: serde_json::Value,
    ) -> JsonRpcResponse {
        JsonRpcResponse::error(id, JsonRpcError::internal_error("测速功能由 App 层管理"))
    }

    // ──────── workspace.* ────────

    fn handle_workspace_save(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing name")),
        };
        match self.session_manager.workspace_save(&name) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_workspace_restore(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing name")),
        };
        match self.session_manager.workspace_restore(&name) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_workspace_list(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(id, self.session_manager.workspace_list())
    }

    // ──────── capture.* (终端文本截图 + 剪贴板) ────────

    fn handle_capture_screen(&self, id: serde_json::Value) -> JsonRpcResponse {
        // 返回所有活跃 session 的屏幕文本快照
        let sessions = self.session_manager.list_sessions();
        let mut captures = Vec::new();
        for info in &sessions {
            if let Ok(screen) = self.session_manager.read_screen(&info.id) {
                captures.push(json!({
                    "session_id": info.id,
                    "name": info.name,
                    "screen": screen,
                }));
            }
        }
        JsonRpcResponse::success(id, json!({
            "type": "text",
            "captures": captures,
            "message": "终端文本快照（非图像截图）",
        }))
    }

    fn handle_capture_window(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        // 如果指定了 session（通过 title 匹配 session name），返回该 session 的屏幕
        let title = params.get("title").and_then(|v| v.as_str());
        let sessions = self.session_manager.list_sessions();
        for info in &sessions {
            let matches = title.map_or(true, |t| {
                info.name.as_deref().unwrap_or("").contains(t) || info.id.contains(t)
            });
            if matches {
                if let Ok(screen) = self.session_manager.read_screen(&info.id) {
                    return JsonRpcResponse::success(id, json!({
                        "session_id": info.id,
                        "name": info.name,
                        "screen": screen,
                        "type": "text",
                    }));
                }
            }
        }
        JsonRpcResponse::error(id, JsonRpcError::internal_error("未找到匹配的 session"))
    }

    fn handle_capture_select(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::error(id, JsonRpcError::internal_error("框选截图需要 UI 交互，请通过 App 操作"))
    }

    // ──────── AI 完全控制接口 handlers ────────

    fn handle_exec_run_wait(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        let command = match params.get("command").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing command")),
        };
        let timeout_ms = params.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(30000);
        self.session_manager.audit("exec.run_wait", Some(&session_id), &command, true);
        match self.session_manager.run_wait(&session_id, &command, timeout_ms) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_exec_status(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        match self.session_manager.exec_status(&session_id) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_exec_cancel(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        self.session_manager.audit("exec.cancel", Some(&session_id), "cancel", true);
        match self.session_manager.cancel_command(&session_id) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_screen_text(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        match self.session_manager.read_text(&session_id) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_screen_search(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        let pattern = match params.get("pattern").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing pattern")),
        };
        let max_results = params.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        match self.session_manager.search_screen(&session_id, &pattern, max_results) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_session_idle(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        match self.session_manager.is_idle(&session_id) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_session_cwd(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        match self.session_manager.get_cwd(&session_id) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_session_env(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing name")),
        };
        match self.session_manager.get_env(&session_id, &name) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_session_set_env(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing session_id")),
        };
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing name")),
        };
        let value = match params.get("value").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing value")),
        };
        self.session_manager.audit("session.set_env", Some(&session_id), &format!("{}={}", name, value), true);
        match self.session_manager.set_env(&session_id, &name, &value) {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    fn handle_session_audit_log(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let limit = params.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32);
        let session_id = params.get("session_id").and_then(|v| v.as_str());
        let entries = self.session_manager.get_audit_log(limit, session_id);
        JsonRpcResponse::success(id, serde_json::to_value(entries).unwrap())
    }

    fn handle_system_info(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(id, self.session_manager.system_info())
    }

    fn handle_policy_set(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        use crate::session::CommandPolicy;
        let policy: CommandPolicy = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return JsonRpcResponse::error(id, JsonRpcError::invalid_params(&e.to_string())),
        };
        self.session_manager.audit("policy.set", None, &format!("enabled={}", policy.enabled), true);
        self.session_manager.set_policy(policy);
        JsonRpcResponse::success(id, json!({"set": true}))
    }

    fn handle_policy_check(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let command = match params.get("command").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return JsonRpcResponse::error(id, JsonRpcError::invalid_params("missing command")),
        };
        JsonRpcResponse::success(id, self.session_manager.check_policy(&command))
    }

    fn handle_capture_clipboard(&self, id: serde_json::Value) -> JsonRpcResponse {
        // 读取系统剪贴板文本（仅 Windows）
        #[cfg(target_os = "windows")]
        {
            match read_clipboard_text() {
                Ok(text) => JsonRpcResponse::success(id, json!({
                    "type": "text",
                    "content": text,
                })),
                Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            JsonRpcResponse::error(id, JsonRpcError::internal_error("剪贴板读取暂仅支持 Windows"))
        }
    }
}

/// 读取 Windows 剪贴板文本
#[cfg(target_os = "windows")]
fn read_clipboard_text() -> anyhow::Result<String> {
    use std::process::Command;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", "Get-Clipboard"])
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        anyhow::bail!("读取剪贴板失败")
    }
}
