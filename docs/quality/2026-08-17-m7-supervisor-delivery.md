# M7 Supervisor 与一体化交付 — 验收与决策记录

2026-08-17。对应计划第 34–38 条。**这一份不是全绿**:代码部分做完了,装机与跨平台 E2E 那半没做,下面写明界线。

## 门禁三条

| 门禁 | 证据 | 结果 |
|---|---|---|
| 不启动 UI 仍可工作 | `supervisor.rs::a_live_core_with_no_window_is_a_working_unterm` —— Core+MCP ready、GUI absent、`can_work_without_ui()` 为真;真机:headless Core 上跑完整个 M5/M6 验证,全程没有窗口 | 通过 |
| 进程独立健康 | 三个角色各自判定:`a_core_that_has_not_opened_its_port_is_not_somewhere_an_agent_can_go`(Core ready 但 MCP 未就绪)、`a_record_without_its_process_is_stale_not_absent`、`a_core_still_starting_is_alive_and_not_usable` | 通过 |
| 升级失败恢复上一可用版本和数据快照 | `upgrade.rs::an_upgrade_that_does_not_come_up_restores_the_binary_and_the_data` —— 新版迁移完数据又起不来,二进制和数据**双双**回到升级前 | 通过(逻辑层) |

## 锁定的决策

**活着 ≠ 可用。** 这是这个模块存在的主要理由。一个还在回放 scrollback 的 Core 是 alive 而 not ready;被告知 "ready" 的客户端接着收到解释不了的错误。所以 `Starting` 和 `Ready` 是两个状态,`is_usable()` 只认后者。

**"记录还在、进程没了" 是 Stale,不是 Absent。** 区别在于**是否欠一次清理**:Absent 说"什么都没发生过",Stale 说"这里死过一个东西"。

**没有窗口不是故障。** GUI absent 是一种受支持的运行方式(M1 起 Core 就是独立的)。把它报成错误,会训练用户忽略这一栏 —— 而那正是真出问题时该看的一栏。

**MCP 是独立的一条主张。** "Core 起来了" 和 "agent 能连上" 不是同一件事,只有后者是 agent 关心的。所以端口没开的 Core,MCP 一栏是 `Starting` 而不是跟着 ready。

**机器事件的处理写成一张表,不写在各自的通知回调里。** 同一条规则散在四处就是四条略有差别的规则。表里的四条:

- **睡眠 → 什么都不做。** 会话要活过合盖。合盖就丢会话的终端在笔记本上没法用 —— 这条是用户最有感的。
- **唤醒 → 全部重新探测。** provider 的端口、网络挂载、时钟,机器睡着的时候都可能变了。
- **注销 → drain**(有几分钟,把手上的活干完);**关机 → 立刻停**(只有几秒,记下位置比干到一半被杀掉强)。
- **崩溃 → reconcile**,把死掉的进程留下的东西变成判决、收回它的认领。

**升级的顺序就是整个设计。** 先快照数据 → 暂存新版 → 交换 → 确认健康 → 不健康就两边都退回。**先确认再交换等于确认了别的东西;先迁移再快照等于快照了损坏**。二进制回退在数据回退之前:任何时刻机器都得是可用的,"跑着旧版看着旧数据" 好过 "机器上没有 Unterm"。

`confirm` 是调用方传进来的闭包 —— 因为"健康"对安装器和对测试不是一回事,更因为**这个模块自己写的确认必然永远通过**。

**回滚本身也要快照。** 误点回滚的人得能再滚回来。

**快照只拷贝值得拷贝的。** tasks.db、settings、instances、providers —— 不含缓存。**拷贝一切的快照是没人愿意做的快照,而没人做的快照什么都保护不了。**

**诊断包是白名单。** 按字段名往里拷,没点名的一律不进。黑名单是自然的写法,而它会在**有人加了个新字段**的那一刻泄漏 —— 恰好是没人在看的时刻。任务只出计数不出标题("修 Henderson 的发票"里那是客户的名字,不是我们的);路径只出**形状**(`<path with 7 components>`),因为家目录里有用户的名字。另有一个 `leaks_in()` 作为第二道:它能失败,有测试证明它会失败。

## 新增的面

- MCP:`supervisor.status/reconcile`、`system.diagnostics/snapshots/snapshot/restore_snapshot`(6 个,总数 127 → **133**)
- CLI:`unterm-cli system status|reconcile|diagnostics|snapshots|snapshot|restore`

## 真机验证

headless Core(无窗口)上:

```
$ unterm-cli system status
core   ready     37735    127.0.0.1:51032
gui    absent
mcp    ready     37735    127.0.0.1:51033
Can work without a window: yes
```

诊断包对着**这个 Core 正在用的真 token** 检查过:`token in bundle: False`、`/Users/alexlee in bundle: False`、`state_dir: "<path with 7 components>"`。

杀掉 Core 之后重启一个,`system status` 报新 pid、`system reconcile` 报 0 条陈旧记录 —— 因为 `fleet_store` 在启动时已经跑过一次 recovery(M2 起的行为),reconcile 只是把同一件事显式化。**这里也暴露了一个真 bug**:`supervisor.status` 第一版把 `Health` 的内部标签枚举直接序列化出去,CLI 打印出三行 `?`。枚举形状对 Rust 是对的,对线是别扭的 —— 每个客户端都得知道 `pid` 藏在 `health` 下面且只在某些状态存在。改成在门口摊平成 `{role, state, pid, usable, detail}`。只有真跑一次才会发现。

一个明确的限制:`supervisor.*` 走 Core,所以 **Core 死了就问不到它** —— 崩溃后的清理是由**下一个** Core 启动时做的。这是对的分工,但值得写下来,免得有人以为 `system reconcile` 能在没有 Core 的情况下用。

## 第二轮补的两块

**唤醒检测不问操作系统。** macOS 在 `NSWorkspace` 自己的通知中心广播睡眠,Windows 发 `WM_POWERBROADCAST`,Linux 有 logind —— 三套 API、三套胶水,而且**不把真机弄睡就没法测**。有个到处都成立、又能喂数字测试的信号:**两个时钟不一致**。单调时钟在挂起期间不走(`mach_absolute_time`/`CLOCK_MONOTONIC`/`QPC` 都如此),墙上时钟走。两者相差超过 30 秒就是睡过。

阈值这么定是因为:NTP 校时、时区变更、调度器繁忙都以**秒**计移动墙钟,没有一个会移动半分钟。墙钟**倒退**不报睡眠 —— 那是有人在校时,重探无害但那个数字会是假的,而那个数字是要给人看的。醒来后先重探 provider、再报哪些租约在睡觉期间到期了 —— "你的 agent 凌晨三点权限到期了" 和 "你的 agent 不干活了" 是两句不同的话。

**能这样测的代价是**:注销和关机这条路检测不到,仍然需要平台挂钩。这是真实的限制不是疏忽,`action_for` 里那两条已经写好了等人接线。GUI 侧已接进 `about_to_wait` 的空闲 tick(两次读时钟)。

**安装勘察与卸载计划。** 打包那部分做不了,但**决策逻辑**能做也能测,而 bug 就长在决策里:找出这台机器上每一份 Unterm(bundle / Homebrew / 系统包 / `cargo install` / 符号链接)、判断哪些会打架、以及卸载会拿走什么。

- **bundle + 指向它的符号链接不算冲突** —— 那正是 macOS 装好之后上 PATH 的方式,把它标成冲突等于给每一次正确安装报警,而人人都看见的警告等于没人看。
- **符号链接指向 build tree 要单独点名** —— 开发机的经典:shell 跑的是构建产物,图标点开的是发行版,bug 报告写"有时候好使"。
- **两份真安装的提示要说后果**,不能只说"你装了两份":它们共用一个 state 目录,谁先起来谁拥有会话,而两个版本可能对同一个数据库做出不同的迁移。
- **卸载只出计划不动手**。"删掉我的数据吗"只有在答案说清数据是什么的时候才是个能回答的问题 —— 所以每一项都带一句人话("tasks.db —— 每一个 task、run、step、审批与租约")。默认**保留数据**。

真机上跑过:找到 `/Applications/Unterm.app`、零冲突;卸载计划列出它并说明数据会留下。

## 没做的(明确)

- **M7-04 的打包那半。** 三平台流水线(DMG/MSI/deb+AppImage)仍是各自独立的产物,勘察与计划逻辑有了,把它接进安装器/卸载器没做。
- **M7-05 的 E2E 那半。** Windows/macOS 真机装机-升级-回滚全链路没跑。本轮的升级/回滚只在逻辑层验证(临时目录里的假二进制 + 假数据),**真机上换掉一个正在运行的应用是另一回事**:文件锁、公证、Gatekeeper、MSI 的三套 staging 都会介入。发版前必须在 UTM 的 Win11-ARM 和 macOS 上实跑。
- **M7-02 的 OS 挂钩。** 状态迁移表和 reconcile 落地了,但把 macOS 的 `NSWorkspace` 睡眠/唤醒通知、Windows 的 `WM_POWERBROADCAST`/`WM_QUERYENDSESSION` 接到这张表上,属于 GUI 进程那一侧,这次没接。表先有,接线在后。
