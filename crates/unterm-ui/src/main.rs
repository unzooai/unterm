//! unterm-ui: Unterm GUI 渲染进程
//!
//! 职责：
//! - wgpu + glyphon 文字渲染
//! - 键盘/鼠标输入处理
//! - Tab/分屏布局管理
//! - 连接 unterm-core 的 IPC client
//! - 多 session 仪表盘视图

mod render;
mod input;
mod layout;
mod client;

use anyhow::Result;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("unterm_ui=debug")
        .init();

    info!("unterm-ui starting...");

    // TODO: 连接 unterm-core daemon
    // TODO: 创建窗口 (winit)
    // TODO: 初始化 wgpu 渲染管线
    // TODO: 进入事件循环

    info!("unterm-ui ready");

    Ok(())
}
