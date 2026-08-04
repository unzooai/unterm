# unterm-core 独立进程架构

更新时间：2026-08-04

## 定位

`unterm-core` 是每用户一个的独立 Core 进程（issue #12 True Headless Core 的
服务入口），在 next-core 引擎之上提供认证本地 IPC 边界，使终端会话不依赖
GUI 进程生命周期。对应开发计划 M1-01/M1-02 切片；引擎与 MCP 生命周期完整
迁入 Core（M1-03）和 GUI/CLI Core Client 化（M1-04）在其上继续。

```text
GUI / CLI / MCP Bridge
       |
       | authenticated local IPC（token + discovery 文件）
       v
unterm-core（独立进程，single-instance）
       |
       | SessionEngine / ScreenEngine / InputEngine trait
       v
unterm-engine next-core（PTY、Screen、Scrollback、revision）
```

实现位置：`unterm-core/src/lib.rs`（服务、协议、客户端）、
`unterm-core/src/main.rs`（进程入口）。

## 进程生命周期

- **发现**：Core 启动后将 `{endpoint, token, pid, product_version}` 原子写入
  `%LOCALAPPDATA%\Unterm\core.json`（Unix 为 data_local_dir 对应路径）。
- **single-instance 锁**：启动时以独占文件锁（Windows `share_mode(0)` /
  Unix `flock`）竞争 `core.lock`。仅锁持有者可 bind 端口并发布 discovery；
  落败进程静默退出，父进程继续轮询胜者的 discovery。进程崩溃由 OS 释放锁，
  不存在陈旧锁。
- **自拉起**：客户端 `ensure_running()` 先读 discovery 并握手，失败才拉起
  同目录 `unterm-core(.exe)`，随后轮询就绪。
- **版本握手**：`core.info` 返回 `unterm-protocol` 的 `BuildHandshake`
  （`ProcessRole::Core`）。客户端 `handshake()` 按 `Compatibility` 判定
  （产品版本精确匹配、协议 semver major、schema 单调），不兼容立即报错，
  不与版本偏差的 Core 通信，也不会在其旁边误拉第二个 Core。
- **drain**：`core.drain` 后 `core.health`/`core.readiness` 报告
  `draining`，`session.create` 返回 `draining` 错误码，存量会话继续可用。
  这是 GUI「排空后退出」语义的 Core 侧基础。`core.shutdown` 报告
  `stopping` 并停止服务。

## IPC 方法

```text
core.info          BuildHandshake 身份与版本
core.health        ready | draining
core.readiness     同上
core.drain         拒新会话，保存量
core.shutdown      停止服务
session.create     cols/rows/cwd/argv -> SessionSnapshot
session.list       全部会话快照
session.write      写入 PTY（InputEngine::write_input）
session.screen     ScreenSnapshot（lines/cells/cursor/revision/dirty_rows）
session.frame      RenderFrameSnapshot，支持 since_revision 增量
session.resize     调整尺寸
session.close      销毁会话
```

线协议为按行分隔的 JSON 请求/响应，每个请求携带 token；错误以
`{code, message}` 返回（`unauthenticated` / `draining` /
`method_not_found` / `internal_error` / `invalid_request`）。

## 测试覆盖

`cargo test -p unterm-core`：握手身份与兼容性、token 拒绝、锁互斥与
释放、真实 PTY 会话经 Core IPC 完整往返（写入命令并从 Screen 读回输出、
frame revision 递增）、drain 拒新保旧。

## 维护规则

- 新增终端会话必须经 Core IPC 或（过渡期内）next-core 引擎 trait；
  禁止在 GUI 内直接持有 PTY 生命周期、绕过 Core 保存 Screen 状态。
- IPC 破坏性变更必须升级 `unterm-protocol` 的协议 major（冻结点 F1）。

## 渊源

进程模型（discovery/锁/drain/握手语义与测试设计）移植自 wezterm 引擎线
的原型实现（tag `archive/wezterm-line-final`，文档
`docs/architecture/unterm-core-migration.md` 在该 tag 下），会话绑定层
按 next-core 引擎 trait 重写。该原型线已冻结。
