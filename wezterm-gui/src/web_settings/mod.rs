//! Built-in HTTP/1.1 server that exposes Unterm's settings to a browser SPA.
//!
//! Same data path as MCP — the routes call directly into
//! `crate::mcp::handler::McpHandler` and the recording / theme primitives,
//! the SPA is just a different transport. No external runtime, no extra
//! workspace deps; ~/.unterm/server.json carries the bound port and the
//! shared auth token.

mod assets;
mod server;

pub use server::start_web_settings_server;
