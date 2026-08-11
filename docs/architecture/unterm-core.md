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

- **发现**：Core 启动后将 `{endpoint, token, pid, product_version, build_commit,
  protocol_version, data_schema_version, process_role, started_at}` 原子写入
  `%LOCALAPPDATA%\Unterm\core.json`（Unix 为 data_local_dir 对应路径）。
  `UNTERM_STATE_DIR` 覆盖该目录（discovery 与锁一起走）——与 M0-02 给
  bridge registry 的隔离约定一致，测试/headless 环境不会碰真实用户 Core。
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
- **drain**：`core.drain` 后 `core.health` 继续报告进程存活但状态为
  `draining`，`core.readiness` 报告 `not_ready`，`session.create` 返回
  `draining` 错误码，存量会话继续可用。`core.drain`、`core.health` 与
  `core.readiness` 都返回 `active_session_count` 和 `drained`，供托管方判断
  何时排空完成。
  这是 GUI「排空后退出」语义的 Core 侧基础。`core.shutdown` 报告
  `stopping` 并停止服务。

## IPC 方法

```text
core.info                BuildHandshake 身份与版本
core.health              进程存活/排空状态（ready | draining，alive=true，含排空计数）
core.readiness           是否接受新工作（ready | not_ready，含排空计数）
core.drain               拒新会话（create/split），保存量，返回排空计数
core.shutdown            停止服务
core.events              连接转为单向事件推送流（见下）
session.create           cols/rows/cwd/argv/env/launch_policy -> SessionSnapshot
session.split            源 pane + direction/size_percent -> SessionSnapshot
session.get              单个会话快照
session.list             全部会话快照
session.focus            置为活动会话
session.write            写入 PTY（InputEngine::write_input）
session.paste            粘贴（bracketed paste 语义）
session.screen           ScreenSnapshot（lines/cells/cursor/revision/dirty_rows）
session.styled_screen    StyledScreenSnapshot（逐 cell 样式，GUI 渲染输入）
session.styled_frame     同上但带 since_revision：未变化只回小信封，不序列化 cell 网格
session.frame            RenderFrameSnapshot，支持 since_revision 增量
session.visible_text     可见区纯文本
session.lines            指定区间 ScreenLine
session.scrollback       尾部 scrollback 行
session.scrollback_text  ScrollbackTextSnapshot（range/tail/escapes）
session.styled_scrollback 带样式 scrollback
session.search           模式匹配（case_sensitive|case_insensitive）
session.cursor           光标快照
session.modes            PaneModesSnapshot（mouse/alt-screen/bracketed paste）
session.shell            ShellSnapshot（进程、cwd、launch context）
session.activity         SessionActivitySnapshot（进程树、IO 计数）
session.erase_scrollback 清除历史（可含视口）
session.resize           调整尺寸
session.close            销毁会话
session.revision         廉价 revision 探询（u64）
session.scroll_to        视口滚动到绝对位置
session.scroll_by        视口相对滚动
session.scroll_to_prompt 按 prompt 标记滚动
session.report_mouse     鼠标事件上报（显式线格式，不继承 termwiz 布局）
session.recording_start  开始录制 -> RecordingStartResult
session.recording_stop   停止录制 -> RecordingStopResult
session.recording_status 录制状态
session.recording_attach_trace 关联 trace id
session.recording_export 导出 markdown
core.engine_health       EngineHealthSnapshot（引擎自身健康，非进程健康）
core.set_scrollback_lines 之后新建会话的 scrollback 容量（客户端连上后回传自己的配置）
```

线协议为按行分隔的 JSON 请求/响应，每个请求携带 token；错误以
`{code, message}` 返回（`unauthenticated` / `draining` /
`method_not_found` / `internal_error` / `invalid_request`）。

## core.events 事件流

`core.events` 把该连接变成单向推送流：先回 `{subscribed: true}`，之后每行
一个事件（`session_created` / `session_closed` / `session_dead` /
`screen_updated{revision}` / `draining`）。客户端用
`CoreEventStream::connect` 订阅——这是 GUI 摆脱 `about_to_wait` 定时轮询
的基础：革命性变化不在轮询是否存在，而在轮询只发生在 Core 内一处
（`core-event-watcher` 线程，有订阅者 25ms、无订阅者 250ms 降频），而非
每个客户端各自的帧循环。引擎日后提供真正的唤醒钩子时，只需改 watcher
实现，线协议不变。

事件是边沿通知而非可回放日志：晚到的订阅者用 `session.list` 自举，只听
之后的变化。持久化、cursor 可寻址的事件存储是 M2（Durable Task Engine）
的工作，不在此处提前造第二套协议。

## Path Scope

MCP 调用可携带 `path_scope`。`session.create` 和 `session.split` 的启动
`cwd` 按 `write_paths` 校验；只在 `read_paths` 中的目录不能作为新 PTY 或
分屏的工作目录。已有 pane 的 `session.input`、`session.paste`、`exec.*`、
`signal.send` 等副作用入口会在执行前按当前 pane cwd 校验同一 scope。

## CoreEngineClient（M1-04 的 GUI/CLI 接入面）

`unterm_core::CoreEngineClient` 在客户端进程内实现完整
`TerminalEngine`（SessionEngine + ScreenEngine + InputEngine +
RecordingEngine + HealthEngine，经 blanket impl），另有 GUI 依赖的
固有方法镜像：`pane_modes` / `screen_revision` / `scroll_viewport_to` /
`scroll_viewport_by` / `scroll_viewport_to_prompt` / `report_mouse`。
每个调用都跨认证 IPC 到 Core 进程执行。
GUI/CLI 把本地 `NextCoreEngine` 换成它即可让会话搬进 Core：这是把
`unterm-engine` 进程级全局单例（`NextCoreRuntime`）从"隐式进程内共享"
换成"显式跨进程 IPC"的迁移路径。单条 TCP 连接由 Mutex 保证请求-响应对
原子，多线程调用不会交错帧。快照类型（unterm-engine）已补
`Deserialize`，与 Core 侧 `Serialize` 对称。

## IPC 渲染成本实测（2026-08-04，release，120x40 pane）

`cargo test -p unterm-core --release -- --ignored --nocapture bench_styled`：

| 路径 | p50 | p95 | max |
|---|---|---|---|
| `session.styled_screen` 全量（IPC） | 5.2ms | 21.2ms | 35.2ms |
| `session.frame` 未变化（IPC） | 291µs | 343µs | 6.8ms |
| `read_styled_screen` 进程内基线 | 37µs | 55µs | 116µs |

TCP_NODELAY 已默认开启（关闭前 max 达 329ms，Nagle 与延迟 ACK 交互）；
全量快照的 5ms 主要是 4800 cell 的 JSON 序列化，不是传输。

**由此定死 M1-04 的渲染路径约束**：GUI 换用 CoreEngineClient 时，
禁止把现有每帧 20+ 次 `read_styled_screen` 直译成 IPC 调用（一帧预算
16ms）。正确形态是：`core.events` 推送 + `session.styled_frame`
增量拉取 + 客户端 FrameCache；搜索/copy mode/link 检测等消费者读
FrameCache，不再各自打 IPC。全量快照仅在缓存缺失（新 pane、重连）时
发生。若将来仍不够，再考虑二进制编码或行级 delta，属于优化项而非
前置条件。

该形态已由 `unterm_core::FrameCache` 实现：后台 `frame-cache` 线程
订阅事件，ScreenUpdated/SessionCreated 触发 `styled_frame(since)`
拉取，SessionClosed 驱逐；GUI 从本地内存读快照（clone 成本），
`generation()` 单调计数供脏检查。事件流按 200ms 读超时轮询停止标志
（Windows 上 `shutdown()` 不会解除已阻塞的 recv，不能靠它退出线程）；
`CoreEventStream` 为此改为手动分帧缓冲，读超时不丢半行数据。

## 测试覆盖

`cargo test -p unterm-core`：握手身份与兼容性、token 拒绝、锁互斥与
释放、真实 PTY 会话经 Core IPC 完整往返（写入命令并从 Screen 读回输出、
frame revision 递增）、drain 拒新保旧、CoreEngineClient 门面全方法往返
（styled screen/增量 frame/search/cursor/modes/shell/activity/resize）、
split 归属（split_from）与 drain 阻断 split、事件流全生命周期
（created -> screen_updated -> closed -> draining）、交互面
（revision/滚动/鼠标/录制/引擎健康 + TerminalEngine 编译期断言）。

`tests/process_e2e.rs` 针对真实二进制：discovery 发布与 shutdown 清理、
会话经真实进程往返、8 进程并发启动只产生一个 Core（败者自行退出且
不覆盖胜者的 discovery）。全部走 `UNTERM_STATE_DIR` 临时目录。

## 维护规则

- 新增终端会话必须经 Core IPC 或（过渡期内）next-core 引擎 trait；
  禁止在 GUI 内直接持有 PTY 生命周期、绕过 Core 保存 Screen 状态。
- IPC 破坏性变更必须升级 `unterm-protocol` 的协议 major（冻结点 F1）。

## GUI 接入状态（M1-04b，2026-08-04）

`unterm-app` 的 `App.engine` 已换为 `engine_backend::AppEngine` 枚举：
`Local`（默认，进程内引擎，行为与 0.61.1 完全一致）或 `Core`
（`UNTERM_CORE_CLIENT=1` 实验开关，经 `ensure_running` 自拉起 Core，
CoreEngineClient + FrameCache 驱动）。AppEngine 实现全部五个引擎
trait，styled 读在 Core 模式走 FrameCache。

真机冒烟已通过（隔离 UNTERM_STATE_DIR）：GUI 首 pane 创建于 Core
进程；**杀 GUI 后 Core 存活、session.list 仍含该会话**——M1 门禁
「关闭 GUI 后 PTY 继续运行」的首次真机证据。

重开领养（M1-04c）已落地：窗口首 pane 优先领养 Core 中的活会话
（偏好 is_active，跳过已死会话），仅在 Core 为空时才新建 shell；
其余会话由 `sync_tabs` 的既有对账逻辑接管（含 split 归属）。真机
验证：GUI 写入 marker -> 杀 GUI -> 重开 -> 会话数不变（领养而非
新建）、marker 内容仍在屏幕上——「重开 GUI 后可以看到相同 Pane 和
Scrollback」门禁的首个真机证据。Local 模式启动时引擎必为空，走的
仍是原来的新建路径，行为不变。

MCP 一致性（M1-03c 第一阶段）已落地：`init_from_environment` 在 MCP
server 启动前为**整个进程**决定引擎后端并装入 `ENGINE_PROVIDER`——
Core 模式下 provider 返回 `CoreHostEngine`（终端面走共享
`CoreEngineClient`，窗口/截图面仍由本进程回答，因为窗口就在这里）。
statsbar/cockpit/录制导出/scrollback PNG 四处后台线程的引擎直构一并
改走 `unterm_engine::host_engine()`。真机验证：`unterm-cli session
create/split` 经 GUI 的 MCP server 创建的 pane 全部落在 Core 进程
（split_from 血缘完整），GUI/MCP/后台线程看到同一个会话世界。
`server_info`（实例注册表）补上 `UNTERM_STATE_DIR` 支持，与 CLI 读端
（client.rs）、bridge registry、Core discovery 的隔离契约对齐——此前
测试实例会污染真实用户注册表。

**Core 托管 MCP（M1-03c 第二阶段第一步）已落地**：unterm-core 进程
启动即在临时端口起 headless MCP server（同一 token，103 方法驱动
本进程引擎），端口写入 core.json 的 `mcp_port` 字段——不碰
server.json，GUI 的实例注册互不冲突。unterm-cli 的端点解析新增
回退级：**活 Core 的 MCP** > 活 GUI 实例 > 旧版 server.json/token。
真机验证：无任何 GUI 时 `unterm-cli session create/list` 直接工作；
E2E `headless_mcp_serves_sessions_without_any_gui` 把 M1 门禁
「不启动 GUI 可经 MCP 执行命令」固化为自动化测试。

headless 安全语义：无窗口时确认门立即 fail-closed（审计
`mcp.confirm.headless_block`，提示 trust/never 两条授权路径），
不再让调用者挂在无人应答的确认超时上。`server.info` 在无实例
记录时自报进程自身 BuildHandshake（ProcessRole::Core），版本
握手不再把沉默当不兼容。

过渡期形态：GUI 在场时 agent 走 GUI 的 MCP（完整确认 UI），GUI
缺席时故障转移到 Core 的 MCP（fail-closed）。剩余缺口：Core 进程
尚不读用户配置（settings 用默认值，trusted agents 不生效）；单一
MCP + McpHost 反向 IPC（GUI 在场时也由 Core 服务）为最终形态。

事件唤醒（M1-04d）已接通：FrameCache 支持 on-change 通知
（`start_with_notify`），Core 模式下回调直连
`window.request_redraw()`——Core 侧屏幕一变，缓存更新完成即唤醒
事件循环重绘，输出延迟不再受 `about_to_wait` 定时节拍限制。链路：
PTY 输出 -> Core watcher（25ms）-> core.events 推送 -> FrameCache
拉增量 -> request_redraw -> 绘制读缓存。

实验开关下的已知缺口（后续切片）：
- 布局（split 树）仍在 GUI 进程，重开后 split 关系靠 split_from
  降级重建，比例/方向不保真——布局入 Core 是后续设计决策
- MCP server 生命周期随 GUI（见上）
- 退出三语义模态提醒（M1-04 原定验收）未实现

scrollback 配置已打通：connect_core 时经 `core.set_scrollback_lines`
把 GUI 的配置值回传 Core（配置文件在客户端侧；对既有 pane 不生效，
与设置页「新 pane 生效」契约一致；多客户端后写覆盖）。

## 渊源

进程模型（discovery/锁/drain/握手语义与测试设计）移植自 wezterm 引擎线
的原型实现（tag `archive/wezterm-line-final`，文档
`docs/architecture/unterm-core-migration.md` 在该 tag 下），会话绑定层
按 next-core 引擎 trait 重写。该原型线已冻结。
