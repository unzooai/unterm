# Unterm Core 内核迁移说明

更新时间：2026-08-04

## 结论

Unterm 的应用终端运行时已经切换到独立的 `unterm-core` 进程。GUI、CLI、独立 mux，以及 WSL、Exec、Serial、SSH 域不再创建旧的本地 Pane/PTY 内核。

## 当前架构

```text
GUI / CLI / MCP
       |
       | authenticated local IPC
       v
unterm-core
  |- PTY process lifecycle
  |- Session / Screen / Scrollback
  |- Cursor / Resize / Input
  |- Capture / Recording hooks
  `- Health / Readiness / Discovery
```

核心实现位置：

- `unterm-core/src/lib.rs`：Core 服务、协议、PTY 和会话生命周期。
- `mux/src/corepane.rs`：mux Pane 到 Core Session 的适配层。
- `mux/src/domain.rs`：`CoreDomain`，负责本地、WSL、Exec、Serial 和命令型远端会话。
- `wezterm-gui/src/main.rs`：GUI 启动和默认域接入 Core。
- `wezterm-mux-server/src/main.rs`：独立 mux 默认域接入 Core。

## IPC 会话接口

已实现并由客户端使用：

```text
core.info
core.health
core.readiness
core.drain
core.shutdown
session.create
session.external.create
session.serial.create
session.list
session.close
session.write
session.input
session.feed
session.read
session.screen
session.lines
session.changed
session.zones
session.resize
```

`session.external.create`、`session.feed` 和 `session.input` 用于 tmux-control 等外部协议桥接；终端状态仍由 Core 的 Screen/Terminal 实例维护。

渲染保真接口说明：

- `session.screen` 支持 `meta_only`，返回 seqno、title、cwd、alt_screen、mouse_grabbed 和完整光标状态（形状/可见性），供 GUI 每帧一次拉取渲染元数据。
- `session.lines` 返回 serde 序列化的完整 `Line`（含颜色和单元格属性），CorePane 直接反序列化用于渲染，不再退化为纯文本。
- `session.changed` 基于 Core 终端的 `get_changed_stable_rows` 返回增量脏行，CorePane 用真实 seqno 做增量重绘。
- `session.zones` 返回 Core 终端的语义区域（prompt/output 分区）。

## 进程生命周期硬化（对应 issue #12 / #23 部分验收项）

- **Single-instance 锁**：Core 启动时以独占文件锁（Windows `share_mode(0)` / Unix `flock`）竞争 `core.lock`；并发启动多个 Core 时只有锁持有者能 bind 端口并发布 discovery，落败进程静默退出，父进程继续轮询胜者的 discovery。进程崩溃时由 OS 释放锁，无陈旧锁问题。
- **版本握手**：`core.info` 返回 `product_version`、`build_commit`（构建时注入 `UNTERM_BUILD_COMMIT`，默认 `dev`）、`protocol_version`、`data_schema_version`、`process_role`、`pid`、`started_at`。
- **协议校验**：客户端 `CoreClient::handshake()` 在 `ensure_running` 连接路径上强制校验 `protocol_version`，不匹配立即报错，不会与不兼容的 Core 继续通信，也不会在其旁边再拉起第二个 Core。
- **Drain**：`core.drain` 后 `core.health`/`core.readiness` 报告 `draining`，新建会话（`session.create`/`external.create`/`serial.create`）返回 `draining` 错误码，存量会话继续可读写。这是 GUI "排空后退出" 语义的 Core 侧基础。

尚未实现（M1 剩余项）：`core.discover`、`core.events` 事件订阅、MCP Server 迁入 Core、GUI 重连恢复。

## 已删除的旧路径

- `mux/src/localpane.rs` 及 `LocalPane` 类型。
- 旧 `RemoteSshDomain` 运行时和 `WrappedSshPty` 包装器。
- 旧本地 `LocalDomain` 和本地 PTY 启动逻辑。
- 录制模块对旧 Pane 的兼容分支。
- 旧失败 PTY 标记和旧 Writer 包装器。

SSH 配置仍保留协议配置转换工具，供远端客户端协商使用；实际终端会话由 Core PTY 边界承载。tmux-control 仍是外部协议适配器，但其 Pane 状态和输入输出已经通过 Core external session bridge 管理。

## 打包

`unterm-core` 已加入 Windows MSI、Linux AppImage/deb 和 macOS bundle 的构建及发布脚本，安装后与 GUI 使用同一版本的 Core 二进制。

## 验证记录

```text
cargo check --workspace --offline       passed
cargo test -p unterm-core --offline     6 passed
git diff --check                        passed
```

Core 测试覆盖：协议版本、认证客户端、真实 PTY 会话、会话读写、Screen、external session 的输入/输出桥接，以及富文本 Line 序列化、渲染元数据（title/seqno/alt-screen）、增量脏行和语义区域。

## 维护规则

新增终端会话必须通过 `CoreDomain`/`CorePane` 或 Core IPC；不得重新引入本地 Pane、直接持有 PTY 生命周期或绕过 Core 保存 Screen 状态。
