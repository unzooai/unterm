# Unzoo One Core 详细开发计划

状态：执行中

需求基线：`2026-08-03-unzoo-one-core-execution-plan.md`

代码基线：Unterm `0.61.1` / `d84236dc`
原则：一个 PR 只跨越一个主要架构边界；每项完成必须有测试或真机证据。

## 1. 交付策略

P0 按 `M0 -> M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7` 的依赖推进。M1 后允许
Store、Policy 和 Provider 三条工作流在冻结协议基础上并行；M8 只在 P0 门禁全部通过后启动。

每个切片必须同时提交：代码、协议说明、迁移/回滚说明、观测字段、自动化测试和手工验收记录。
涉及副作用的切片必须声明风险分类、幂等行为、取消行为和审计行为。

## 2. 冻结点

| 冻结点 | 必须稳定的内容 | 变更规则 |
|---|---|---|
| F0 | BuildHandshake、版本来源、错误 envelope | 仅可增加可选字段 |
| F1 | Core discovery、认证 IPC、health/readiness | 破坏性变更升级协议 major |
| F2 | Task ID、状态机、Event envelope、cursor | 数据迁移与旧 reader 必须同时存在 |
| F3 | ActionContext、Risk、Grant、Approval | 所有入口运行同一合约测试 |
| F4 | ProviderManifest、Lease、Invoke/Evidence | Unzoo 和 fake provider 同步验证 |

## 3. PR 级执行清单

### M0 发布与版本收口

1. `M0-01` 统一版本身份（当前切片）
   - 新建无 GUI/Engine 依赖的 `unterm-protocol`。
   - 单一 `product_version` 来源；定义 build commit、协议和 schema 版本。
   - GUI 实例、MCP `server.info`、MCP initialize 返回 `BuildHandshake`。
   - `unterm --version` 在任何初始化前退出；GUI/CLI 输出同一版本。
   - CI 检查 workspace、GUI、CLI、MSI 和二进制版本一致。
2. `M0-02` Bridge 换代
   - Bridge 连接时比较 product/protocol/schema。
   - 不兼容 Bridge 进入 drain，停止接收调用并由 launcher 拉起同制品 Bridge。
   - 记录 owner pid、替换原因、起止时间；超时后才强制终止。
   - Bridge 在 `bridges/<pid>.json` 注册生命周期；新 GUI 启动时为不兼容记录写入 drain 请求。
   - `UNTERM_STATE_DIR` 允许 headless/测试环境隔离状态目录，不污染真实用户实例。
3. `M0-03` 发布验收
   - 完成 12 项 runtime-pending 台账，记录平台和证据路径。
   - Windows MSI/macOS/Linux 安装、覆盖升级和回滚矩阵。
   - Release workflow 从 tag 校验版本，制品解包后再次执行版本探针。

M0 门禁：版本探针 <1 秒、无窗口/Server/PTY/注册表实例；升级后无旧 Bridge；所有制品同版本。

### M1 True Headless Core

4. `M1-01` 新增 `unterm-core` 空壳和 per-user single-instance 锁。
5. `M1-02` 实现认证本地 IPC envelope、discover/info/health/readiness/events。
6. `M1-03` 将 Terminal Engine 和 MCP 生命周期从 GUI 迁入 Core。
7. `M1-04` GUI/CLI 改为 Core Client，支持 cursor 重连和三种退出语义。
   - 当前退出应用提醒视觉权重过低，用户很容易忽略；必须改为主动聚焦的高可见度模态提醒。
   - 明确展示“后台继续运行”“排空任务后退出”“立即取消并退出”三种结果，不能只依赖弱提示文字。
   - 立即取消使用危险操作层级，默认焦点放在可恢复的“后台继续运行”，并支持键盘完整操作。
   - 选择后台继续后显示持续可发现的托盘/状态提示，说明 Core、PTY 和 Agent 仍在运行及如何重新打开。
8. `M1-05` 20 Client 竞争、GUI/Core 崩溃和旧 MCP 合约回归测试。

M1 门禁：无 GUI 创建 PTY；关闭/重开 GUI 后 Pane 和 Scrollback 不变；Core 健康不被 GUI 健康替代；
退出提醒在普通、最大化和多窗口状态下均清晰可见，三种退出结果经过可用性测试且不会误取消后台任务。

### M2 Durable Task Engine

9. `M2-01` SQLite WAL、迁移器、兼容快照、备份/回滚。
10. `M2-02` Task/Run/Step/Event 表和带单调 version 的事务状态机。
11. `M2-03` ToolCall 幂等占位、Worker claim、heartbeat、timeout 和 retry。
12. `M2-04` cancel/recovery/reconcile；running 项按 heartbeat/lease 恢复或 interrupted。
13. `M2-05` Cockpit/Fleet 改为 Task 投影，删除双重全局真相。

M2 门禁：逐状态强杀恢复；相同幂等键不重复副作用；并发 Worker 不重复执行 Step。

### M3 Action Gateway、Policy 与 Approval

14. `M3-01` ActionContext、Risk 枚举、统一错误码和 dry-run decision。
15. `M3-02` 唯一 Action Gateway：validate -> scope -> policy -> approval -> lease -> invoke -> verify。
16. `M3-03` 持久 Approval/Grant，支持 once/task/resource/always、TTL 和撤销。
17. `M3-04` MCP、CLI、Brain、Workflow 和 PTY 写入全部接入 Gateway。
18. `M3-05` CI 静态登记表和动态旁路测试；未分类 mutation 构建失败。

M3 门禁：等价入口得到相同决策；审批可跨重启；Grant 撤销立即阻断等待及后续动作。

### M4 Unified Brain Runtime

19. `M4-01` BrainAdapter trait、统一事件、Thread 与 Task/Run 关联。
20. `M4-02` Codex CLI JSONL adapter，Tool Request 只进入 Action Gateway。
21. `M4-03` Claude CLI adapter 和 Brain Contract Suite。
22. `M4-04` interrupt/resume/snapshot/usage、崩溃恢复和模型健康。
23. `M4-05` SDK adapter 与 CLI adapter 事件等价测试。

M4 门禁：Codex/Claude 产生同构事件；强杀 adapter 不丢 Task；interrupt 传播到真实进程。

### M5 Provider Registry 与 Unzoo Binding

24. `M5-01` ProviderManifest、Registry、动态 discovery 和协议协商。
25. `M5-02` Capability Lease 的签发、续租、撤销、过期和重放防护。
26. `M5-03` fake provider Contract Suite，覆盖 offline/cancel/idempotency/evidence。
27. `M5-04` Unzoo 双向认证绑定及 Browser/Profile/Computer 能力映射。
28. `M5-05` 设置页绑定、暂停、重连、撤销、诊断。

M5 门禁：不依赖固定端口；离线进入 waiting_provider；取消传播；动作可反查完整授权链。

### M6 Workspace、Artifact、Audit 与路由

29. `M6-01` Workspace scope：canonical path、symlink/junction、UNC、大小写、`..`。
30. `M6-02` Artifact Registry 与内容寻址存储，大文件不进入 SQLite。
31. `M6-03` Audit 补齐关联 ID、状态、脱敏和 hash-chain。
32. `M6-04` 单 Task 证据包导出、完整性验证、保留和配额。
33. `M6-05` 强制意图路由；Managed Brain 禁止裸 CDP/WebDriver 和未授权 Shell。

M6 门禁：Workspace 互相隔离；Artifact 可追溯；Unzoo 离线不回退到其他浏览器栈。

### M7 Supervisor 与一体化交付

34. `M7-01` 三进程 Supervisor 的 discover/start/readiness/drain/stop/restart。
35. `M7-02` 睡眠、唤醒、注销、关机和崩溃状态迁移。
36. `M7-03` 原子升级、迁移前备份、健康确认和失败回滚。
37. `M7-04` 单一安装包、已有独立安装冲突处理和选择性数据卸载。
38. `M7-05` Windows/macOS E2E 及脱敏诊断包。

M7 门禁：不启动 UI 仍可工作；进程独立健康；升级失败恢复上一可用版本和数据快照。

## 4. 数据与协议验收矩阵

| 对象 | 持久化 | 幂等 | 取消 | 恢复 | 审计 |
|---|---:|---:|---:|---:|---:|
| Task/Run/Step | 是 | 提交键 | 级联 | heartbeat 扫描 | 全状态迁移 |
| Brain Thread | 是 | event id | interrupt | adapter snapshot | 模型/用量/事件 |
| ToolCall | 是 | 必须 | provider/PTY | reconcile | 请求/结果脱敏 |
| Approval/Grant | 是 | decision version | revoke | 继续等待 | 决策者/范围/TTL |
| Provider Lease | 是 | lease id | revoke | renew/reacquire | capability/scope |
| Artifact | 元数据 | content hash | 不适用 | 内容校验 | 来源与关联链 |

## 5. 通用测试门禁

- 单元：状态迁移、policy、scope、幂等、lease、事件归一化、DTO 兼容。
- 合约：Core Client、Brain Adapter、Provider；fake 与真实 Unzoo 各一套。
- 故障注入：提交前/中/后 kill，数据库锁/磁盘满，provider/brain 断线，uncertain outcome。
- E2E：安装、绑定、升级、回滚、卸载、GUI 重连、Codex/Claude + Terminal + Unzoo。
- 安全：入口旁路、路径穿越、凭据日志泄漏、Grant/Lease 重放、非法自动化栈。
- 性能：Core/GUI 冷启动、空闲资源、1/4/20 Pane、20 万行 PTY、事件/SQLite/invoke P50/P95。

任何门禁失败均不得用“代码已完成”替代；失败项必须保留复现命令、日志和责任里程碑。

## 6. 当前进度

| 切片 | 状态 | 已有证据 | 未完成 |
|---|---|---|---|
| M0-01 | 已完成（源码切片） | Protocol 4、Services 116、MCP 66、CLI 23、GUI probe 1；release build 与真实握手通过 | CI/签名安装制品随发布验收 |
| M0-02 | 开发中 | product/protocol/schema 判定、CLI/Bridge 双向身份、持久 Bridge registry、GUI drain 请求、`-32010` 进程测试 | pre-registry 旧进程扫描、owner 自动重启 E2E、超时强退 |
| M0-03 | 未开始 | 0.61.1 Windows 本机安装记录 | 12 项跨平台台账与回滚矩阵 |
| M1–M7 | 未开始 | 现有 next-core/MCP/Audit 是迁移输入 | 按上述冻结点推进 |

## 7. 下一切片入口条件

M0-01 已通过。M0-02 的协议判定和协作式 drain 已通过，下一步补齐没有 registry 记录的
pre-M0 Bridge 进程扫描、owner 重启 E2E 和超时强退。M0-02 不应提前创建 Task Store 或
Provider Registry，以免在 F0/F1 冻结前形成第二套协议。
