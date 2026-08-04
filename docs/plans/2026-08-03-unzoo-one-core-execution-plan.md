# Unterm Core / Unzoo One 开发执行计划

> 状态：执行草案，供开发团队按阶段拆分 Issue、PR 和 Release
>
> 基线：Unterm `v0.61.1` / `master@d84236dc`（2026-08-03）
>
> 关联 Issue：#12–#24
>
> 目标：在不破坏现有终端能力的前提下，把 Unterm 建设为 Unzoo One 的本地控制平面

## 1. 执行摘要

`v0.61.1` 已基本完成从 WezTerm 内核到 next-core 的终端功能迁移：159 项功能台账中 147 项已验证，12 项等待真实平台或外部环境验收；MCP 和 CLI 的核心测试稳定，PTY 写入旁路与命令白名单问题已经修复，审计也已具备最小持久化能力。

下一阶段不能继续以“在 GUI 内增加功能”的方式演进。项目必须沿以下关键路径实施：

```text
发布与版本一致性
  -> True Headless Core
  -> Durable Task Engine
  -> 统一 Action Context + Policy/Approval
  -> Brain Adapter Runtime
  -> Provider Registry + Unzoo Binding
  -> Workspace / Audit / Artifact / Tool Routing 闭环
  -> 三进程 Supervisor 与一体化安装
  -> Scheduler / Memory / Connector / Model Router
```

第一阶段目标不是“做出所有个人助理功能”，而是形成一个可以持续运行、崩溃可恢复、所有副作用可授权和审计的最小控制内核。

## 2. 项目完成定义

### 2.1 P0 MVP

P0 完成时，产品必须同时满足：

- 不启动 GUI，也能创建 PTY、运行 MCP、启动 Codex/Claude 和执行后台任务。
- GUI 关闭、重开后，可以连接原来的 Core，并恢复 Task、PTY、Approval 和 Provider 状态。
- Codex 与 Claude 通过统一 Brain Event Protocol 运行，而不是仅由 CLI 启动后等待退出码。
- Task、Run、Step、ToolCall、Approval 在 Core 重启后保持一致。
- 所有终端、浏览器和 Computer Use 副作用都经过统一 Policy、Grant、Lease 和 Audit 链路。
- Unzoo 能作为认证的 Browser/Computer Provider 被发现、绑定、调用、取消和撤销。
- 重试相同 `idempotency_key` 不会重复发布、删除、付款、发送或写入。
- Screenshot、Diff、Download、Recording、Report 等产物可关联 Task 并导出证据包。
- 用户只安装一个产品；Core、Unzoo Runtime、Unzoo One UI 可以独立健康检查、升级和恢复。

### 2.2 非目标

P0 暂不承诺：

- 任意桌面软件都能自动操作。
- 无审批自动支付、发布、删除账号或不可逆外部操作。
- 企业多租户、组织 RBAC 或跨机器分布式调度。
- 完整移动端；P0 只保留通知和审批协议扩展点。
- 把 Unzoo 浏览器代码合并进 Unterm。Unzoo 必须保持独立 Provider 进程。

## 3. 不可破坏的架构边界

```text
Unzoo One UI
    |
    | authenticated local IPC
    v
Unterm Core
    |- Task Engine
    |- Brain Runtime
    |- Policy / Approval
    |- Workspace / Identity / Vault references
    |- Provider Registry
    |- Audit / Artifact
    `- Scheduler / Recovery
          |                         |
          | Brain Adapter Bus       | Capability Provider Bus
          v                         v
   Codex / Claude / ...      Unterm Terminal / Unzoo / Connectors
```

职责必须保持：

- Unterm Core 持有全局 Task、授权、审批、恢复、审计和 Provider 编排。
- Unterm Terminal Provider 持有 PTY、Screen、Session 和 Process 生命周期。
- Unzoo 持有 Browser Profile、Cookie、网页/桌面观察和动作执行。
- Brain 只能提出结构化 Tool Request，不能获得绕过 Core 的无限制宿主能力。
- GUI 是客户端，不是 Core 的生命周期所有者。

## 4. 当前基线与已知问题

### 4.1 已有能力

- next-core PTY、Screen、Session、Scrollback、Capture 和 Recording。
- 103 个 MCP 方法、32 个一级 CLI 命令。
- Agent Cockpit、Fleet、Review、Workspace Template、Profile、Secret Vault。
- Codex、Claude、Gemini、OpenCode、Kimi、Trae 等 CLI 安装和启动。
- PTY 写入统一进入确认闸门，`allowed_patterns` 已生效。
- 审计按日写入脱敏 JSONL，重启可回填，默认保留 30 天。

### 4.2 启动本计划前必须记录的基线缺口

- 12 项终端能力仍处于 `Implemented, runtime pending`。
- MSI 安装版、运行 GUI、CLI 和 MCP Bridge 可能处于不同版本。
- 旧 MCP Bridge 在新 Server 已运行时可能仍误报“GUI 未运行”。
- `unterm.exe --version` 的进程生命周期需要验证，不能进入完整 GUI/Server 常驻流程。
- MCP/PTY Server 仍由 `unterm-app` 启动，没有独立 `unterm-core`。
- 当前 Workspace 是会话模板，不是权限边界。
- 当前 Agent Run 是 CLI 进程启动器，不是 Brain Runtime。
- 没有通用 Task Store、Provider Registry、Capability Lease 或 Artifact Registry。

## 5. 实施原则

1. **先协议、后界面**：Core API、状态机和数据迁移先稳定，再连接 GUI。
2. **先单机正确、后扩展**：P0 只支持本机单用户，不提前设计分布式共识。
3. **副作用默认经网关**：任何新增写入入口若未注册 Action Gateway，CI 必须失败。
4. **状态先落盘、再对外确认**：对外副作用前后必须有事务状态和幂等记录。
5. **版本必须握手**：CLI、GUI、Brain Adapter、Provider 与 Core 都必须声明协议和构建版本。
6. **兼容优先于重写**：现有 MCP 方法继续工作，通过兼容层路由到新 Core。
7. **每阶段可独立发布**：每个里程碑必须有迁移、回滚、观测和验收方案。
8. **不以单测代替真机验收**：Windows/macOS 安装、升级、GUI 重连和进程崩溃必须端到端验证。

## 6. 建议代码结构

初期可以先在现有 crate 内建立模块，接口稳定后再拆 crate，避免一次性目录重构。目标边界如下：

```text
unterm-core/          后台 Core 二进制与生命周期
unterm-protocol/      IPC DTO、版本协商、错误码、事件定义
unterm-store/         SQLite、迁移、事务、加密引用
unterm-task/          Task 状态机、执行队列、恢复、幂等
unterm-brain/         Brain Adapter 接口和事件归一化
unterm-policy/        Policy、Approval、Grant、Action Gateway
unterm-provider/      Provider Registry、Lease、Invoke、Events
unterm-artifact/      Artifact 元数据、内容寻址、证据导出
unterm-mcp/           MCP 兼容入口；调用 Core Service，不持有全局状态
unterm-engine/        Terminal Provider；不持有全局 Task
unterm-app/           GUI Client；不启动或拥有 Core
unterm-cli/           CLI Client；不直接绕过 Core 执行副作用
```

若暂不拆 crate，至少要用相同边界建立模块和 trait，禁止 Task、Policy、Provider 状态继续堆入 `unterm-mcp/src/handler.rs`。

## 7. 里程碑与工作分解

## M0：发布、版本和基础验收收口

目标：建立可信基线，保证后续问题不是由版本混用或旧进程造成。

### 交付物

- 完成 12 项 `runtime pending` 真机验收，无法完成的项目明确平台、负责人和阻塞条件。
- `unterm.exe --version`、`unterm-cli --version`、MCP `serverInfo`、Core `build_version` 和 Release tag 一致。
- 进程握手至少返回：

```text
product_version
build_commit
protocol_version
data_schema_version
process_role
pid
started_at
```

- MCP Bridge 检测到协议或版本不匹配时退出并由 Supervisor/launcher 替换。
- 新版本启动时识别旧 Bridge，完成 drain、退出和重启，不遗留孤儿进程。
- CI 增加“同一制品版本一致性”和“旧版本升级后进程全部换代”测试。

### 验收

- [ ] 干净机器安装、覆盖升级、失败回滚均有记录。
- [ ] 升级后不存在旧 CLI/MCP Bridge。
- [ ] `--version` 在 1 秒内输出并退出，不创建窗口、Server、注册表实例或 PTY。
- [ ] Windows、macOS 发布制品完成签名/安装验证；Linux 至少完成 deb/AppImage smoke test。
- [ ] #24 阶段 A 验收结果回填到功能台账。

## M1：True Headless Core

关联：#12

目标：让 Core、PTY 和 MCP 脱离 GUI 生命周期。

### 交付物

- 新增独立 `unterm-core` 进程。
- Core 持有 Terminal Engine、MCP、Task Store、Policy、Audit 和实例发现。
- GUI/CLI 使用认证本地 IPC 连接 Core。
- Windows 首期采用每用户后台进程；macOS 采用 LaunchAgent。除非确有系统级需求，不使用管理员服务。
- 原子 single-instance 锁，多个 GUI/CLI 并发启动只能产生一个 Core。
- Core 支持：

```text
core.discover
core.info
core.health
core.readiness
core.drain
core.shutdown
core.events
```

- GUI 退出提供：后台继续、排空后退出、立即取消三个语义清晰的选项。
- Core 崩溃后，未完成会话标记为 `interrupted`，不能伪装为成功。

### 验收

- [ ] 不启动 GUI，可以创建 PTY 并通过 MCP 执行命令。
- [ ] 关闭 GUI 后 PTY 和 Agent 继续运行。
- [ ] 重开 GUI 后可以看到相同 Pane、Scrollback 和任务状态。
- [ ] 20 个并发 Client 启动不会产生重复 Core 或端口争用。
- [ ] Core/GUI 任一单独崩溃不会被整体健康检查误报为正常。
- [ ] 旧 MCP API 通过兼容层全部通过现有测试。

## M2：Durable Task Engine

关联：#14

目标：建立所有 Brain、Terminal 和 Provider 工作的统一持久化生命周期。

### 7.2.1 最小数据模型

```text
workspaces
tasks
runs
steps
brain_threads
tool_calls
approvals
capability_grants
provider_leases
artifacts
events
```

关键字段至少包括：

```text
id
workspace_id
parent_id
status
attempt
idempotency_key
created_at / updated_at
started_at / finished_at
heartbeat_at
timeout_at
error_code / error_detail
version
```

### 7.2.2 状态机

```text
queued
  -> running
  -> waiting_approval
  -> waiting_human
  -> waiting_provider
  -> retry_scheduled
  -> succeeded | failed | cancelled | interrupted
```

所有迁移必须：

- 在 SQLite 事务中完成。
- 进行合法状态校验。
- 增加单调 `version` 防止并发覆盖。
- 写入 Event。
- 对副作用 ToolCall 预先占用 `idempotency_key`。

### 交付物

- SQLite WAL、迁移版本、备份与回滚策略。
- Task CRUD、Run/Step 状态迁移、查询和事件订阅。
- Heartbeat、超时、取消、重试、恢复和补偿接口。
- Core 重启时扫描 `running`，根据 heartbeat/lease 转为恢复或 `interrupted`。
- Cockpit/Fleet 可以投影为 Task View，不再形成独立的全局状态真相。

### 验收

- [ ] 在每个状态点强杀 Core，重启后状态一致。
- [ ] 重复提交相同幂等键只产生一次副作用。
- [ ] 并发 Worker 不会执行同一个 Step 两次。
- [ ] Task Cancel 能传播到 Brain、PTY 和 Provider。
- [ ] 数据迁移失败可恢复上一份兼容快照。

## M3：Action Context、Policy 与 Approval

关联：#15、#16、#21、#22

目标：所有副作用调用使用同一种安全上下文和唯一执行网关。

### 7.3.1 统一 Action Context

```json
{
  "action_id": "act_...",
  "task_id": "task_...",
  "run_id": "run_...",
  "step_id": "step_...",
  "tool_call_id": "call_...",
  "workspace_id": "ws_...",
  "brain_id": "brain:codex",
  "provider_id": "unterm.terminal",
  "capability": "terminal.exec",
  "resource_scope": {},
  "risk": "local_mutation",
  "idempotency_key": "...",
  "grant_id": null,
  "approval_id": null
}
```

### 7.3.2 唯一执行流水线

```text
Validate Context
  -> Resolve Workspace Scope
  -> Resolve Provider/Capability
  -> Policy Decision
  -> Approval / Grant
  -> Acquire Lease
  -> Persist attempted
  -> Invoke
  -> Verify
  -> Persist result/artifact
  -> Release Lease
```

任何 MCP、CLI、Brain、Workflow 或内部调用都不得绕开此流水线。

### 交付物

- 风险等级：`readonly`、`local_mutation`、`external_side_effect`、`credential_access`、`financial`、`destructive`。
- Grant 支持 allow once、for task、for resource、always、deny、TTL 和撤销。
- Approval 是持久对象，支持等待、允许、拒绝、过期和撤销。
- Policy dry-run 返回命中的规则、资源范围、风险和拒绝理由。
- Terminal 所有写入继续通过 PTY Gateway，并接入 Action Context。
- 未声明风险的 Mutation 默认拒绝或按高风险处理。

### 验收

- [ ] 同一命令从 MCP、CLI、Brain 进入时得到相同 Policy 结果。
- [ ] 禁止 `external.publish` 后所有 Provider 的发布动作均被拒绝。
- [ ] Grant 过期或撤销后，正在等待和后续动作都不能继续。
- [ ] 新增副作用入口未注册 Action Gateway 时 CI 失败。
- [ ] Approval 重启后仍可继续等待并接受用户决定。

## M4：Unified Brain Adapter Runtime

关联：#13

目标：把 Agent CLI 启动器升级为可恢复、可观测、可授权的 Brain Runtime。

### 7.4.1 统一接口

```text
brain.start
brain.resume
brain.events
brain.submit_input
brain.approve
brain.interrupt
brain.snapshot
brain.usage
brain.close
```

### 7.4.2 统一事件

```text
thread.started
message.delta
reasoning.summary
tool.requested
tool.started
tool.completed
approval.requested
artifact.created
usage.updated
thread.interrupted
thread.completed
thread.failed
```

### Adapter 顺序

1. Codex CLI JSONL，作为首个端到端参考实现。
2. Claude CLI 结构化输出。
3. Codex SDK。
4. Claude Agent SDK。
5. Gemini CLI/ACP、OpenCode。

SDK 与 CLI Adapter 必须输出相同 Core Event，不允许上层依赖供应商事件格式。

### 交付物

- Adapter trait、事件归一化、能力声明和版本探测。
- Thread 与 Task/Run 的持久关联。
- stdout/stderr/exit/signal 结构化处理。
- Tool Request 进入 M3 Action Gateway。
- Token、费用、延迟、模型和上下文使用统计。
- Interrupt、Resume 和 Adapter 崩溃恢复。
- Adapter 可用性与模型路由健康检查。

### 验收

- [ ] Codex 和 Claude 对同一测试任务生成相同事件类型序列。
- [ ] Brain 请求工具时无法直接绕过 Policy。
- [ ] 强杀 Adapter 后 Task 不丢失，可恢复或明确失败。
- [ ] Interrupt 在限定时间内传播到实际子进程。
- [ ] 同一 Task 可以按策略切换 Brain，并记录理由和上下文损失。

## M5：Provider Registry 与 Unzoo Binding

关联：#17，以及 Unzoo 对应 Provider Issues

目标：通过通用 Provider Contract 原生连接 Unzoo，不硬编码端口和工具清单。

### 7.5.1 Provider Contract

```text
provider.discover
provider.register
provider.info
provider.health
provider.capabilities
provider.acquire_lease
provider.renew_lease
provider.release_lease
provider.invoke
provider.cancel
provider.events
```

Provider Manifest：

```json
{
  "provider_id": "unzoo.local",
  "product_version": "...",
  "protocol_version": "...",
  "schema_version": "...",
  "endpoint": "...",
  "capabilities": [],
  "risk_metadata_version": "...",
  "health": "ready"
}
```

### 7.5.2 Lease

Lease 至少绑定：

```text
lease_id
provider_id
task_id
grant_id
capabilities
resource_scopes
issued_at
expires_at
renew_after
revoked_at
```

### 交付物

- 本机 Provider discovery，不依赖固定 Unzoo 端口。
- 双向认证、协议协商和 Client Identity。
- Unzoo Browser Profile、Browser、Computer 能力显示。
- Provider offline/retry/waiting 状态。
- Task Cancel 调用 `provider.cancel`。
- Unzoo Action Evidence 返回 Artifact Registry。
- 设置页支持绑定、暂停、重连、撤销和诊断。

### 验收

- [ ] Unzoo 改端口或升级后能够重新发现。
- [ ] 协议不兼容时阻止调用并给出升级建议。
- [ ] Lease 过期或撤销后 Unzoo 拒绝动作。
- [ ] Unzoo 离线时 Task 进入 `waiting_provider`，不静默换用其他浏览器。
- [ ] 取消 Task 后 Workflow/CUA/Desktop Action 都停止。
- [ ] 每个动作可反查 Task、Brain、Grant、Lease 和证据。

## M6：Workspace、Audit、Artifact 与强制工具路由

关联：#18、#21、#22

### 7.6.1 Workspace 安全上下文

Workspace 至少包含：

```text
root_paths + read/write mode
repository/remotes/default_branch
identity_profile_id
browser_profile_grants
secret_scope
memory_namespace
connector_grants
brain/model_policy
budget/quota
policy_profile
artifact/audit_retention
```

路径必须覆盖 canonical path、symlink、junction、UNC、大小写和 `..` 穿越校验。

### 7.6.2 Artifact Registry

Artifact 元数据至少包含：

```text
artifact_id
task_id/run_id/step_id/tool_call_id
kind
mime_type
content_hash
size
storage_uri
source_provider
created_at
retention_policy
redaction_state
```

大文件不写入 SQLite；使用内容寻址文件存储或外部对象存储引用。

### 7.6.3 Audit

在现有 JSONL 基础上补齐：

- Event ID、Task/Run/Step、Brain、Provider、Grant、Approval。
- attempted/approved/denied/executed/verified 状态。
- Hash-chain 或等价完整性校验。
- 脱敏后的 request/result。
- 查询、保留、删除、配额和单 Task 导出。

### 7.6.4 Tool Routing Enforcement

```text
Terminal intent  -> Unterm Terminal Provider
Browser intent   -> Unzoo Browser Provider
Computer intent  -> Unzoo Computer Provider
Connector intent -> Registered Connector Provider
```

Managed Brain Runtime 默认不得获得裸宿主 Shell、未授权 CDP/WebDriver 或自行安装的浏览器自动化栈。开发 Workspace 可以启用带 TTL、可审计的显式例外。

### 验收

- [ ] Workspace A 无法读取 Workspace B 的 Secret、Cookie Grant 和 Memory。
- [ ] Shell、Git、Provider 间接路径同样不能越过 Workspace 范围。
- [ ] Artifact 可以反查到 Task、ToolCall、Approval 和 Provider。
- [ ] 单 Task 可导出完整证据包并验证完整性。
- [ ] Codex/Claude 请求浏览器动作时只获得 Unzoo 能力。
- [ ] Unzoo 离线时不得自动回退到 Playwright、WebDriver、裸 CDP 或无约束 Shell。

## M7：Runtime Supervisor 与一体化交付

关联：#23

目标：用户安装一个产品，内部可靠管理三个进程。

```text
unterm-core       控制平面
unzoo-runtime     Browser/Computer Provider
unzoo-one-ui      超级工作台客户端
```

### 交付物

- Supervisor：discover/start/readiness/health/drain/stop/restart。
- 记录 owner、pid、endpoint、product/build/protocol/schema version。
- 系统注销、关机、睡眠和唤醒状态迁移。
- 原子升级、健康验证、失败回滚和迁移前备份。
- 已独立安装 Unterm/Unzoo 时的复用、升级、隔离和冲突处理。
- 脱敏诊断包：版本、健康、日志、最近故障、迁移记录。
- 卸载时分别处理程序、Workspace、Audit、Artifact、Memory、Profile、Cookie。

### 验收

- [ ] 一个安装包完成三进程安装和首次绑定。
- [ ] 不启动 UI 也能运行定时任务、PTY、Brain 和 Provider。
- [ ] Core 或 Provider 单独崩溃不会被误报为整体正常。
- [ ] 升级失败可以恢复上一可用版本和兼容数据快照。
- [ ] 并发启动不产生重复 Core、端口争用或数据库双写。
- [ ] Windows/macOS 安装、升级、回滚和卸载都有端到端测试。

## M8：P1 个人助理平台能力

关联：#19

在 M0–M7 全部通过后再进入：

- Scheduler：Cron、Webhook、文件变化、Provider Event、邮件/日历事件。
- 长期记忆：Working/Episodic/Semantic 分层，可查看、编辑、遗忘和设定保留期。
- Connectors：Mail、Calendar、Contacts、Documents、Cloud Drive、IM。
- Model Router：按能力、成本、延迟、健康和用户锁定选择模型。
- Budget/Quota：Token、费用、调用次数和并发上限。
- 通知与移动审批：查看状态、Allow/Deny、人工接管、紧急停止。

P1 必须复用 Task、Policy、Provider、Artifact 和 Audit，不允许创建另一套任务状态系统。

## 8. 跨阶段协议规范

## 8.1 ID 与幂等

- 所有 ID 使用不可预测、全局唯一格式，例如 UUIDv7 或带类型前缀的随机 ID。
- 副作用 ToolCall 必须包含由 Core 分配的 `idempotency_key`。
- Provider 必须保存足够长的幂等记录，至少覆盖最大重试和恢复窗口。
- 幂等冲突返回原执行结果或明确的 `idempotency_conflict`，不得再次执行。

## 8.2 错误码

禁止只返回自由文本。至少定义：

```text
invalid_context
unauthenticated
permission_denied
approval_required
grant_expired
lease_expired
provider_unavailable
protocol_incompatible
resource_out_of_scope
idempotency_conflict
timeout
cancelled
interrupted
verification_failed
uncertain_outcome
data_migration_failed
```

`uncertain_outcome` 必须触发 reconcile，不能直接自动重试可能已提交的外部副作用。

## 8.3 取消传播

```text
Task Cancel
  -> Run/Step Cancel
  -> Brain Interrupt
  -> ToolCall Cancel
  -> Provider Cancel
  -> PTY Signal/Process Stop
  -> Lease Revoke
  -> Audit Final State
```

每层必须返回已接受、已完成、不支持或结果不确定。

## 8.4 事件顺序

- 每个 Task Event 有单调序号。
- Event 持久化成功后再推送给 GUI。
- GUI 断线重连通过 cursor 补发，不依赖内存广播。
- Provider/Brain 事件允许重复投递，Core 必须去重。

## 9. 测试与发布门禁

## 9.1 测试层级

### 单元测试

- 状态迁移、Policy、Scope、幂等、Lease、事件归一化。
- 每个公共副作用方法必须有风险分类。
- 每个 IPC DTO 做向前/向后兼容测试。

### 合约测试

- Brain Adapter Contract Suite。
- Provider Contract Suite。
- Core Client Compatibility Suite。
- Unzoo Provider 使用 fake server 和真实 daemon 各跑一套。

### 故障注入

- 在 ToolCall 提交前、提交中、提交后强杀 Core。
- Brain、Provider、GUI、数据库分别断开。
- 模拟磁盘满、数据库锁、网络超时、Lease 过期和版本不兼容。
- 对 `uncertain_outcome` 验证不会盲目重复副作用。

### 端到端

- Windows、macOS 必跑；Linux 跑 Core/CLI/Provider smoke。
- 安装、首次绑定、升级、回滚、卸载。
- GUI 关闭后继续任务，重开后恢复。
- Codex 和 Claude 各完成 Terminal + Unzoo Browser 组合任务。
- 高风险动作必须出现审批并可拒绝。

## 9.2 性能预算

每个 Release 至少报告：

- Core 冷启动到 readiness。
- GUI 冷启动到可交互。
- 空闲 CPU、RSS、线程和唤醒次数。
- 1/4/20 Pane 的资源占用。
- 10 万/20 万行 PTY 吞吐。
- Task Event、SQLite 写入和 Provider Invoke 的 P50/P95。
- GUI 重连和事件补发耗时。

性能退化超过既定预算时必须显式批准，不能静默放行。

## 9.3 安全测试

- CLI/MCP/Brain/Workflow 等价入口旁路测试。
- Workspace 路径穿越、symlink/junction、UNC 和大小写测试。
- Token、Cookie、Secret、密码和授权票据日志泄漏扫描。
- 未授权 CDP/WebDriver、裸 Shell 和浏览器自动化依赖启动测试。
- Grant/Lease 过期、撤销和重放攻击测试。

## 10. PR 拆分规则

每个 PR 必须：

- 只跨越一个主要架构边界。
- 包含数据迁移和回滚说明。
- 包含用户可见行为、错误码和观测字段。
- 新增公共 API 时同时更新协议文档和合约测试。
- 新增副作用入口时同时更新风险分类和旁路测试。
- 不在同一个 PR 同时大规模重命名和改变行为。

建议前 12 个 PR：

1. 版本握手 DTO、`--version` 退出语义和制品一致性测试。
2. 旧 MCP Bridge 发现、drain 和升级换代。
3. `unterm-core` 空壳、single-instance 和 authenticated IPC。
4. 将 Terminal Engine 与 MCP Server 的生命周期迁入 Core。
5. GUI/CLI 改为 Core Client，并完成重连。
6. SQLite Store、migration、Event 表和备份。
7. Task/Run/Step 状态机与恢复扫描。
8. Action Context、幂等占位和唯一 Action Gateway。
9. Policy/Approval/Grant 持久模型。
10. Codex CLI JSONL Brain Adapter。
11. Claude CLI Brain Adapter 与统一 Contract Suite。
12. Provider Registry、Lease 和 fake Provider Contract Suite。

之后再提交真实 Unzoo Binding、Artifact Registry 和 Supervisor，避免第一个集成 PR 同时承担所有风险。

## 11. 依赖、并行与工作量

以下为工程量级，不是承诺日期。一个资深工程师串行完成 P0 预计约 18–24 个工程周；三个稳定工作流并行可压缩到约 10–14 周，但 M1/M2/M3 的关键路径不能靠增加人力完全并行。

### 工作流 A：Core 与 Store

```text
M0 -> M1 -> M2 -> M7
```

### 工作流 B：Policy、Brain 与 Tool Routing

```text
M2 schema 稳定 -> M3 -> M4 -> M6 routing
```

### 工作流 C：Provider、Artifact 与 Unzoo

```text
M3 context 稳定 -> M5 -> M6 artifact -> M7 integration
```

跨团队冻结点：

- M1 冻结 Core discovery/health/IPC envelope。
- M2 冻结 Task ID、状态和 Event envelope。
- M3 冻结 Action Context、Risk、Grant、Approval。
- M5 冻结 Provider Manifest、Lease、Invoke Result 和 Evidence。

冻结后允许以向后兼容方式增加字段，不允许直接改变已有字段语义。

## 12. 进度管理

每个里程碑建立一个 GitHub Milestone，并按下列标签管理：

```text
area/core
area/task
area/brain
area/policy
area/provider
area/artifact
area/workspace
area/packaging
risk/security
risk/migration
platform/windows
platform/macos
platform/linux
```

每周状态只报告：

- 已完成且有证据的验收项。
- 当前关键路径阻塞。
- 新增风险和数据迁移影响。
- 下周准备合并的 PR。
- 性能、安全和端到端门禁状态。

禁止使用“代码大致完成”“功能应该可用”作为状态。只有合约测试、真机测试、迁移测试或可复现证据通过后才标记完成。

## 13. Issue 对照表

| Issue | 本计划位置 | 当前判断 |
|---|---|---|
| #12 True Headless Core | M1 | P0，关键路径起点 |
| #13 Brain Adapter Runtime | M4 | P0，尚未形成统一 Runtime |
| #14 Durable Task Engine | M2 | P0，所有平台能力的数据底座 |
| #15 Policy/Approval | M3 | P0，当前仅有命令级策略 |
| #16 PTY 授权旁路 | M3 回归门禁 | 基础修复已完成，必须持续防回归 |
| #17 Provider Registry | M5 | P0，Unzoo Binding 前置条件 |
| #18 Audit/Artifact | M6 | Audit 最小版已完成，Artifact/完整性未完成 |
| #19 Memory/Scheduler/Connector | M8 | P1，P0 闭环后开始 |
| #20 Unterm Core Epic | 全部 | 总 Epic |
| #21 Workspace Isolation | M3/M6 | P0，安全边界 |
| #22 Tool Routing Enforcement | M3/M6 | P0，不能由 Prompt 替代 |
| #23 Runtime Supervisor | M0/M7 | P0，解决版本混用和统一交付 |
| #24 0.60 两阶段整改 | M0 + 全计划 | 阶段 A 收口、阶段 B 执行入口 |

## 14. 最终发布门禁

Unzoo One Core MVP 只有同时满足以下条件才允许标记完成：

- [ ] M0 的版本、升级和 12 项真实环境验收关闭。
- [ ] Core 在 GUI 关闭时持续工作。
- [ ] Task/Run/Step/ToolCall/Approval 可跨重启恢复。
- [ ] Codex、Claude 通过同一 Brain Contract。
- [ ] Terminal 和 Unzoo 动作全部通过 Action Gateway。
- [ ] Provider Lease、取消和离线恢复可用。
- [ ] Workspace 隔离经过路径和身份旁路测试。
- [ ] Audit/Artifact 可以导出单 Task 证据包。
- [ ] 三进程安装、升级、回滚、卸载通过 Windows/macOS E2E。
- [ ] 无 P0/P1 安全缺陷，无已知重复副作用风险。
- [ ] 性能没有突破批准预算。

达到以上门禁后，Unterm 才从“AI 原生终端”正式升级为“Unzoo One 的本地控制平面”。
