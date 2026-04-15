//! Session 生命周期管理
//!
//! 每个 session 对应一个 PTY 进程 + VT 解析器（alacritty_terminal）。
//! 负责：创建/销毁 session、状态跟踪、审计日志记录、滚动缓冲区持久化。
