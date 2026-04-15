# Unterm 设计文档

> Unzoo Terminal — AI 时代的超级工作台

## 愿景

为 Claude Code 用户打造的超级工作台。不只是一个终端模拟器，而是 AI agent 的运行时环境——让 AI 能够启动、监督、编排终端任务的基础设施。在 Windows 上取代 PowerShell，在 macOS 上取代系统终端。

核心场景：Claude Code 运行在 Unterm 中，通过 MCP 接口启动和监督其他 AI agent。一个 AI 调度另一个 AI。

## Claude Code 用户痛点 → Unterm 解法

### 1. 多 agent 无法同时监督

**痛点**：用户开 3 个 Claude Code 分别做前端、后端、测试，需要不停切换终端窗口，无法一眼看到全局进度。

**解法**：**多 session 仪表盘视图**
- 分屏同时展示多个 session 的实时输出
- 每个 session 显示状态标签（运行中 / 等待输入 / 已完成 / 出错）
- AI 编排器通过 `session.list` + `screen.read` 汇总全局状态

### 2. 输出洪流，关键信息被淹没

**痛点**：Claude Code 执行大量命令，终端输出几千行，用户找不到关键错误或需要确认的地方。

**解法**：**智能输出标记**
- `orchestrate.wait` 匹配到关键模式（error、prompt、需要确认）时高亮 + 通知
- 支持输出过滤/折叠：编译日志折叠，只展开错误行
- `screen.read` 让监督 AI 可以精准提取需要关注的内容

### 3. session 丢失，工作中断

**痛点**：不小心关了终端窗口，正在运行的 Claude Code 进程直接死掉，所有上下文丢失。

**解法**：**session 持久化**
- `unterm-core` 是独立 daemon，关闭 UI 不影响后台任务
- 重新打开 UI 自动重连所有活跃 session
- session 的滚动缓冲区持久化到磁盘，重启后可回溯历史输出

### 4. Windows 终端体验极差

**痛点**：PowerShell 权限管理混乱、编码问题（UTF-8 vs GBK）、启动慢、无法被外部程序可靠控制。

**解法**：
- 默认管理员模式，一次 UAC 永久生效
- 强制 UTF-8（`[Console]::OutputEncoding`、代码页 65001）
- daemon 常驻，session 创建瞬间完成
- MCP 接口提供可靠的程序化控制，不依赖 PowerShell 的不可控行为

### 5. AI 之间无法协作

**痛点**：用户想让一个 Claude Code 做总指挥，分派任务给其他 agent，但现有终端没有这种能力。

**解法**：**编排调度层**
- `orchestrate.launch` 一键启动新 agent
- `orchestrate.wait` 等待特定输出模式，实现 agent 间的同步
- `orchestrate.broadcast` 向多个 agent 广播指令
- 监督 AI 通过 `screen.read` 实时审查其他 agent 的工作

### 6. 没有工作上下文记忆

**痛点**：每次打开终端都是空白起点，要重新 cd 到项目目录、设置环境变量、启动 dev server。

**解法**：**Workspace 配置**
- 配置默认 cwd、shell、环境变量
- 未来支持 workspace profile：一键恢复完整工作环境（多个 session + 各自的 cwd + 预执行命令）

### 7. AI 出错无法回滚

**痛点**：Claude Code 执行了一串命令，中间某步出了问题，用户不知道从哪里开始错的，也不知道如何回滚。

**解法**：**操作审计日志**
- 每个 session 的所有输入/输出带时间戳记录到结构化日志
- 通过 MCP tool `session.history` 查询：谁在什么时候执行了什么，输出是什么
- 未来可与 git 联动：每次 AI 操作前自动 snapshot，出问题一键回退

### 8. 安全边界缺失

**痛点**：让 AI 自主执行命令，用户担心 AI 误操作（删库、推错分支、覆盖文件）。

**解法**：**权限沙箱 + 审批机制**
- 可配置命令黑名单/白名单（如禁止 `rm -rf /`、`git push --force`）
- 高危命令拦截：匹配到危险模式时暂停执行，等待人类或监督 AI 审批
- 通过 MCP tool `session.approve` / `session.deny` 实现编程化审批
- 每个 session 可设独立权限等级（只读 / 受限执行 / 完全控制）

### 9. 不知道 AI 卡在哪里

**痛点**：Claude Code 跑了很久没反应，不知道是在思考、在等网络、还是挂了。

**解法**：**session 健康监测**
- 心跳检测：core 监控每个 PTY 进程的存活状态
- 活动指标：最后一次输出时间、CPU 占用、是否在等待输入
- 通过 `session.status` 查询，监督 AI 可以判断是否需要干预
- 超时自动通知：session 超过配置时间无输出时触发告警

### 10. 终端不理解项目上下文

**痛点**：终端只是一个哑管道，不理解当前项目是什么、用什么技术栈、有哪些常用命令。

**解法**：**项目感知**
- 自动检测项目类型（package.json → Node、Cargo.toml → Rust、go.mod → Go）
- 根据项目类型预置快捷命令和环境
- 与 CLAUDE.md 联动：读取项目约定，注入到 AI agent 的上下文中

### 11. 多机器/远程开发断裂

**痛点**：本地开发、远程服务器、CI 环境之间切换，每次都要重新建立上下文。

**解法**：**远程 core 连接**
- `unterm-ui` 可以连接远程机器上的 `unterm-core`（通过 SSH 隧道或直连）
- 一个 UI 窗口内同时管理本地和远程 session
- AI agent 不关心 session 在本地还是远程，统一的 MCP 接口

## 架构：前后端分离双进程

```
                    ┌─────────────┐
                    │  AI Agent   │  (Claude Code, 自定义 agent...)
                    └──────┬──────┘
                           │ MCP (JSON-RPC over IPC)
                           ▼
┌──────────────┐    ┌──────────────────────────────────┐
│  unterm-ui   │───▶│          unterm-core              │
│  (wgpu 渲染)  │IPC │                                  │
│              │◀───│  ┌─────────┐  ┌───────────────┐  │
│  - 文字渲染   │    │  │ MCP     │  │ Session       │  │
│  - 输入处理   │    │  │ Server  │──│ Manager       │  │
│  - Tab/分屏  │    │  └─────────┘  └───────┬───────┘  │
└──────────────┘    │                       │          │
                    │               ┌───────▼───────┐  │
┌──────────────┐    │               │ PTY Pool      │  │
│   unterm     │───▶│               │ (portable-pty)│  │
│  (CLI client)│MCP │               └───────────────┘  │
└──────────────┘    └──────────────────────────────────┘
```

### 三个可执行文件

| 组件 | 二进制名 | 职责 |
|------|---------|------|
| daemon | `unterm-core` | 核心引擎：PTY 管理、Session 生命周期、MCP Server |
| GUI | `unterm-ui` | 渲染进程：wgpu 文字渲染、输入处理、Tab/分屏 |
| CLI | `unterm` | 命令行工具，MCP client 的薄封装 |

### 关键设计决策

**双进程分离的好处：**
- UI 崩溃不影响后台运行的任务
- 可以无头运行 core（headless 模式，CI/服务器场景）
- 多个 UI 实例可以连接同一个 core
- AI agent 和人类用户通过同一个 core 交互，地位平等

**自管理能力：**
- `unterm` CLI 运行在 unterm 终端内时，连接的是自己的 `unterm-core` daemon
- session 之间互相隔离，但通过 core 可以互相观察和控制
- Claude Code 在 session 1 中运行，可以通过 `unterm exec -s 2 "command"` 控制 session 2

**内置代理（clash-rs）：**
- `unterm-core` 内嵌 clash-rs 引擎，daemon 启动时自动拉起代理
- 所有 PTY session 自动注入代理环境变量（`HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY`）
- 用户只需在配置中填入订阅链接，开箱即用
- 支持通过 MCP tools 动态切换节点、测速、查看流量
- 解决 AI agent 运行中代理断开的问题：自动故障转移到备用节点

## 控制协议：MCP over IPC

- **传输层**：Windows Named Pipe / Unix Socket
- **协议层**：MCP（JSON-RPC 2.0）
- **扩展性**：新增能力 = 新增一个 MCP Tool，无需改协议层
- CLI 子命令是 MCP tool 的 1:1 映射

## MCP Tools

### Session 管理

| Tool | 参数 | 说明 |
|------|------|------|
| `session.create` | `shell?, cwd?, env?, name?` | 创建新 PTY session |
| `session.list` | — | 列出所有活跃 session |
| `session.attach` | `session_id` | 附加到已有 session，开始接收输出流 |
| `session.detach` | `session_id` | 断开附加 |
| `session.destroy` | `session_id` | 销毁 session，杀掉 PTY 进程 |
| `session.resize` | `session_id, cols, rows` | 调整 PTY 尺寸 |

### 命令执行

| Tool | 参数 | 说明 |
|------|------|------|
| `exec.run` | `session_id, command, timeout?` | 执行命令，等待完成，返回输出 |
| `exec.send` | `session_id, input` | 向 PTY 发送原始输入（支持交互式程序） |
| `exec.signal` | `session_id, signal` | 发送信号（SIGINT、SIGTERM 等） |

### 屏幕读取

| Tool | 参数 | 说明 |
|------|------|------|
| `screen.read` | `session_id, lines?` | 读取当前终端屏幕内容（文本） |
| `screen.cursor` | `session_id` | 获取光标位置 |
| `screen.scroll` | `session_id, offset, count` | 读取滚动缓冲区历史 |

### 编排调度

| Tool | 参数 | 说明 |
|------|------|------|
| `orchestrate.launch` | `command, name?, cwd?` | 在新 session 中启动 AI agent |
| `orchestrate.broadcast` | `session_ids, command` | 向多个 session 广播同一命令 |
| `orchestrate.wait` | `session_id, pattern, timeout?` | 等待 session 输出匹配正则 |

### 审计与安全

| Tool | 参数 | 说明 |
|------|------|------|
| `session.history` | `session_id, since?, limit?` | 查询操作审计日志（输入/输出/时间戳） |
| `session.status` | `session_id` | 健康状态（存活、最后活动时间、CPU、是否等待输入） |
| `session.approve` | `session_id, request_id` | 审批被拦截的高危命令 |
| `session.deny` | `session_id, request_id` | 拒绝被拦截的高危命令 |
| `security.set_policy` | `session_id, policy` | 设置 session 权限等级（readonly/restricted/full） |
| `security.set_rules` | `rules` | 配置命令黑名单/白名单规则 |

### 代理网络

| Tool | 参数 | 说明 |
|------|------|------|
| `proxy.status` | — | 当前代理状态（节点、延迟、上下行流量） |
| `proxy.nodes` | — | 列出所有可用节点及延迟 |
| `proxy.switch` | `node_name` | 切换到指定节点 |
| `proxy.speedtest` | `node_name?` | 对指定节点或全部节点测速 |
| `proxy.set_subscription` | `url` | 设置/更新订阅链接 |
| `proxy.set_auto_switch` | `enabled, threshold_ms?` | 开启自动切换：延迟超阈值时自动换节点 |

### Workspace

| Tool | 参数 | 说明 |
|------|------|------|
| `workspace.save` | `name` | 保存当前工作环境快照（所有 session 及其状态） |
| `workspace.restore` | `name` | 恢复工作环境快照 |
| `workspace.list` | — | 列出已保存的 workspace |

## 项目结构

```
unterm/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── unterm-core/            # daemon: PTY + Session + MCP Server
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── pty/            # portable-pty 封装
│   │   │   ├── session/        # session 生命周期管理
│   │   │   ├── mcp/            # MCP server (JSON-RPC over IPC)
│   │   │   ├── screen/         # 屏幕状态读取 (alacritty_terminal)
│   │   │   ├── orchestrate/    # AI agent 编排
│   │   │   └── proxy/          # clash-rs 代理引擎管理
│   │   └── Cargo.toml
│   ├── unterm-ui/              # GUI: wgpu 渲染
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── render/         # wgpu + glyphon 文字渲染
│   │   │   ├── input/          # 键盘/鼠标输入
│   │   │   ├── layout/         # tab/分屏布局
│   │   │   └── client/         # 连接 unterm-core 的 IPC client
│   │   └── Cargo.toml
│   ├── unterm-cli/             # CLI 工具: `unterm` 命令
│   │   ├── src/main.rs         # MCP client → 子命令映射
│   │   └── Cargo.toml
│   └── unterm-proto/           # 共享协议定义
│       ├── src/lib.rs          # MCP tool 定义、IPC 消息类型
│       └── Cargo.toml
├── docs/
│   └── plans/
└── .gitignore
```

## 技术栈

| 用途 | Crate | 说明 |
|------|-------|------|
| 语言 | Rust | 跨平台、性能、安全 |
| PTY | `portable-pty` | 跨平台伪终端 |
| VT 解析 | `alacritty_terminal` | 成熟的终端模拟核心 |
| GPU 渲染 | `wgpu` | Vulkan/Metal/DX12 抽象层 |
| 文字渲染 | `glyphon` | wgpu 原生文字渲染 |
| 窗口管理 | `winit` | 跨平台窗口 |
| IPC | `interprocess` | Named Pipe / Unix Socket |
| JSON-RPC | `jsonrpsee` 或轻量自实现 | MCP 协议层 |
| 异步运行时 | `tokio` | 异步 I/O |
| 序列化 | `serde` + `serde_json` | JSON 序列化 |
| 代理引擎 | `clash-rs` | 内置代理核心（Rust 实现的 Clash） |

## 平台支持

- Windows 11（Named Pipe，ConPTY）
- macOS（Unix Socket，POSIX PTY）
- 双平台同步开发，通过 `cfg(target_os)` 隔离平台差异

## 数据流

### 人类用户输入流
```
键盘输入 → unterm-ui → IPC → unterm-core → session → PTY → shell
```

### AI agent 输入流
```
AI agent → MCP (exec.run) → unterm-core → session → PTY → shell
```

### 输出流
```
shell → PTY → unterm-core → alacritty_terminal (VT解析)
                          → IPC → unterm-ui (渲染)
                          → MCP notification → AI agent (文本)
```

### 自管理流
```
unterm 终端 session 1:
  Claude Code → `unterm exec -s 2 "npm test"`
    → unterm CLI (MCP client)
    → IPC → unterm-core
    → session 2 PTY → 执行 npm test
    → 输出回传给 Claude Code
```

## 默认配置

配置文件路径：
- Windows: `%APPDATA%\unterm\config.toml`
- macOS: `~/.config/unterm/config.toml`

### 配置示例

```toml
[defaults]
cwd = "E:\\code"             # 默认工作目录，新 session 自动 cd 到此
shell = "pwsh.exe"           # 默认 shell（Windows）
# shell = "/bin/zsh"         # 默认 shell（macOS）

[defaults.env]               # 默认注入的环境变量
EDITOR = "code"

[elevated]
enabled = true               # Windows: 默认以管理员权限运行
auto_restart = true          # 如果非管理员启动，自动请求 UAC 提权重启

[proxy]
enabled = true               # 启用内置代理
mode = "builtin"             # "builtin" = 内置 clash-rs | "external" = 复用本机 Clash | "off"
subscription = "https://your-sub-url.com/api/v1/client/subscribe?token=xxx"
port = 17890                 # 内置代理监听端口（避开本机 Clash 的 7890）
socks_port = 17891           # SOCKS5 端口
auto_switch = true           # 网络异常时自动切换节点
auto_switch_threshold = 3000 # 延迟超过 3000ms 触发自动切换
auto_switch_interval = 30    # 每 30 秒检测一次延迟
fallback_nodes = ["auto", "hk-01", "jp-01"]  # 自动切换优先节点列表
```

### 管理员模式（Windows 特有）

Windows 下终端权限是个老大难问题——PowerShell 需要手动右键"以管理员身份运行"，且提权后 cwd 会重置到 `C:\Windows\System32`。Unterm 解决这两个痛点：

**实现方式：**
1. `unterm-core` 的 exe manifest 声明 `requireAdministrator`（编译时嵌入）
2. 首次启动触发一次 UAC 弹窗，之后 core 作为 daemon 常驻，不再重复弹窗
3. 所有 PTY session 继承 core 的管理员权限
4. `auto_restart = true` 时，如果用户意外以普通权限启动，自动用 `ShellExecuteW("runas")` 提权重启

**与默认目录联动：**
- 提权后不会丢失 cwd——core 启动后读取 `config.toml` 中的 `defaults.cwd`，所有新 session 自动以此为工作目录
- 通过 `session.create` 的 `cwd` 参数可覆盖默认值

### macOS 权限处理

macOS 不需要全局提权，按需处理：
- 需要 sudo 的命令由用户或 AI 在 session 内自行 `sudo`
- 可选配置 Touch ID 授权 sudo（通过 `/etc/pam.d/sudo` 配置 `pam_tid.so`）

### 内置代理（clash-rs）

**与本机 Clash 共存，互不干扰：**

1. **不修改系统代理** — 不动注册表、不改系统 proxy 设置，本机 Clash 照常工作
2. **进程级隔离** — 只给 Unterm 的 PTY session 注入 `HTTP_PROXY=127.0.0.1:17890`，只影响 Unterm 内的进程
3. **独立端口** — 本机 Clash 默认 7890，Unterm 内置的用 17890，互不冲突
4. **智能检测** — `mode = "external"` 时自动检测本机 Clash 端口并复用，不启动内置引擎

**自动切换节点（保障 AI 工作不中断）：**

自动切换**严格限定在用户指定的 `fallback_nodes` 列表内**，按顺序逐个尝试，不会跳到列表外的节点。

```
正常工作 → 每 30s 检测当前节点延迟
              │
              ├─ 延迟 < 阈值 → 继续使用当前节点
              │
              └─ 延迟 > 阈值 或 连接超时
                    │
                    └─ 按 fallback_nodes 顺序逐个尝试
                       ["hk-01"] → 测速 OK → 切换，结束
                       ["hk-01"] → 失败 → 尝试 ["jp-01"]
                       ["jp-01"] → 测速 OK → 切换，结束
                       全部失败 → 通知用户/监督 AI，保持最后可用节点
```

- `fallback_nodes` 由用户配置，只有列表内的节点参与自动切换
- 切换顺序严格按列表顺序，不随机
- 当故障恢复后，可配置是否自动回切到首选节点（`auto_fallback_recovery = true`）

这保证了 Claude Code 长时间运行时，代理断线不会导致 AI 任务中断，且节点选择完全在用户掌控之内。
