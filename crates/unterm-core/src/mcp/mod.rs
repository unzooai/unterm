//! MCP Server 模块
//!
//! JSON-RPC 2.0 over IPC（Windows Named Pipe / Unix Socket）。
//! 将所有 MCP tools 注册为 JSON-RPC 方法，路由请求到对应模块。
