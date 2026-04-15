//! unterm-proto: 共享协议定义
//!
//! MCP tool 类型定义、IPC 消息格式、session 状态枚举等。
//! 被 unterm-core、unterm-ui、unterm-cli 共同依赖。

pub mod session;
pub mod exec;
pub mod screen;
pub mod orchestrate;
pub mod proxy;
pub mod security;
pub mod workspace;
