//! MCP Server 模块
//! JSON-RPC 2.0 over IPC

pub mod protocol;
pub mod router;
pub mod transport;

pub use router::McpRouter;
pub use transport::IpcServer;
