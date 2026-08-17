# M4 Unified Brain Runtime — 验收与决策记录

2026-08-17。对应计划 `docs/plans/2026-08-03-unzoo-one-core-development-plan.md` 第 19–23 条。

## 门禁三条

| 门禁 | 证据 | 结果 |
|---|---|---|
| Codex/Claude 产生同构事件 | `unterm-brain/src/adapters.rs::the_two_adapters_describe_the_same_turn_the_same_way` — 两份真实 wire 格式喂进去，事件多重集逐项相等 | 通过 |
| 强杀 adapter 不丢 Task | `unterm-brain/src/supervisor.rs::killing_the_whole_runtime_does_not_lose_the_task` — 认领后不续租，`reconcile()` 把无人持有的 claim 判成 `Interrupted`，task/step 都还在 | 通过 |
| interrupt 传播到真实进程 | `unterm-brain/src/runtime.rs::interrupting_reaches_the_children_the_agent_started` — 真起进程组，孙进程用文件标记证明确实停了 | 通过（修了一个真 bug，见下） |

额外一条计划外但必须成立的：SDK 与 CLI 两条路产出同构事件 —— `unterm-brain/src/sdk.rs::the_sdk_and_the_cli_describe_the_same_turn_the_same_way`。

## 这一版的骨架

```
unterm-brain
├── lib.rs        BrainEvent（冻结点）、BrainAdapter（纯解析器）、Thread↔Task/Run
├── adapters.rs   CodexAdapter / ClaudeAdapter
├── sdk.rs        SdkAdapter（流式分片，装配后才出事件）
├── runtime.rs    进程：spawn / interrupt / snapshot / usage / stderr 尾巴
├── supervisor.rs 把一次运行绑在 step 的租约上
└── health.rs     模型健康（连续失败才算病）

unterm-services/src/brain_tools.rs   工具请求 → Action Gateway 的唯一桥
```

## 锁定的决策

**适配器是纯的。** 字节进、事件出；不起进程、不开 socket、不执行工具。等价测试之所以能成立全靠这一条 —— 两个适配器喂录像就能比。`brain_tools.rs` 里有一条结构性守卫：brain crate 中除了启动 brain 自己的 `runtime.rs`，任何文件出现 `Command::new` 即失败。

**ToolRequested 是请求不是调用。** 唯一能对它动手的是 M3 的网关，走 `Entry::Brain` 这扇门，和其它五扇门同题同判。

**不认识的工具不等于安全的工具。** CLI 升级出一个新工具名时，桥把它映射到 `brain.tool` —— 网关未分类即按最高风险处理，要人批。反过来做（猜"大概没事"）就是每个未来工具的洞。

**模型的文件工具有自己的动词。** `brain.read` / `brain.list` / `brain.write` / `brain.fetch`，与 MCP 自己的 `workspace.*` 分开命名：它们不是同一扇门，审计线不该靠猜。`brain.fetch` 判 destructive —— 数据离开这台机器，事后谁也收不回来。

**中断以进程组为准，不以那个 agent 为准。** 见下。

**健康只记在内存。** 重启后重新乐观。能跨重启的悲观是没人能清掉的悲观：服务商恢复了、文件还说它挂着，用户不知道自己的模型为什么被跳过。用户连点三次中断也不算故障 —— 那是他改主意，不是模型病了。

**成本一次报完。** SDK 把用量拆在消息两头报，适配器攒到回合真正结束才发一个 `Usage`；否则每个下游都得自己做加法。缓存输入始终单列，两者不同价，合并了就再也拆不开。

## 测出来的两个真 bug

**中断只到 agent 就返回，孙进程被孤儿化。** 第一版逻辑是"发 SIGINT，宽限期内看直接子进程死没死，死了就算成功"。真跑起来：`sh` 收到 SIGINT 就死了，而它后台起的任务按 POSIX 要求忽略 SIGINT —— 于是运行时在**幸存者刚被孤儿化的那一刻**宣布中断成功。现在宽限期是对**整个进程组**计的（`kill(-pgid, 0)` 探活），到点还站着的一律 SIGKILL。这个 bug 只有真起进程、真起孙进程、用文件标记验证才能发现。

**租约测试自己和自己抢时间。** 30 秒租约每 10 秒续一次，测试睡 10.5 秒等它 —— 而 200 次 50ms 的睡眠累计超发就能吃掉那 0.5 秒。改成按 deadline 等（不是数片数），并把租约长度做成参数，测试用 3 秒租约在 1.6 秒内问出真问题。顺带修掉的是生产里同样的隐患：漏一拍就是别人可以抢走的 claim。

## 还没做的

M4 交付的是库和桥，**没有用户可见的面**。Brain 的 CLI/MCP 面按计划在 M5（Provider Registry 与 Unzoo Binding）落，那里才有真正的调用方。在此之前 `unterm-brain` 只被 `unterm-services` 依赖，不进 GUI 路径 —— 也就是说这一版不改变任何现有用户行为。

真实 agent CLI 的端到端跑（真的 `codex exec --json` / `claude -p`）也留在 M5：适配器的 wire 格式来自两个 CLI 的实际输出，但版本会漂，届时要用录像回放做版本回归。
