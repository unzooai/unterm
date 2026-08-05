# 发布就绪审计：M0/M1 主体完成后

日期：2026-08-05  
代码基线：`efbd1c34` 之后  
机器：Windows 11，2560×1440 @150%，2560×1440 物理  
制品：`cargo build --release`（unterm / unterm-core / unterm-cli / unterm-next-core）

## 1. 结论

**Local 模式（默认）可发布。Core 模式（`UNTERM_CORE_CLIENT=1`）仍是实验开关，
不建议在本版本转为默认**——理由见第 5 节。

本轮审计逐功能核对「这个功能在 Core 模式下还对吗」，共发现并修复
**8 个发布级缺陷**，其中 7 个是单元测试发现不了的（只在重启一次、
或另一个进程死掉之后才显形）。

## 2. 门禁与性能（release 制品实测）

| 门禁 | 要求 | 实测 | 判定 |
|---|---|---|---|
| 版本探针 | < 1 秒，无副作用 | 中位 **76 ms**，最大 669 ms | 通过 |
| Core 冷启动 | 亚秒 | 中位 **46 ms**，最大 401 ms | 通过 |
| Core 空闲 CPU | 可忽略 | **0.016 %** | 通过 |
| Core 空闲内存 | 稳定 | **9.7 MB**，30 秒无增长 | 通过 |
| 20 pane | 不爆内存 | **13.0 MB**（+3.3 MB） | 通过 |
| 20 万行 PTY | 内存有界 | **39.9 MB**，scrollback 按上限裁剪 | 通过 |
| 20 并发 Client | 只产生一个 Core | 1.1 秒内收敛，败者不覆盖 discovery | 通过 |
| 无 GUI 经 MCP 执行 | 可用 | E2E + CLI 真机 | 通过 |

### release vs debug（同机同口径）

| 指标 | debug | release | 变化 |
|---|---:|---:|---|
| 输入写入 p50 | 2 µs | **1 µs** | 2× |
| scrollback 分页读 p50 | 51 µs | **8 µs** | 6× |
| 10 万行洪流 | 11 471 行/秒 | **12 054 行/秒** | +5% |
| key-to-screen p50 | 5530 µs | 5511 µs | 持平 |
| echo 往返 p50 | 5518 µs | 5291 µs | 持平 |
| 滚动 1000 行 | 10 ms | **2 ms** | 5× |
| 取回 1 万行 | 85 ms | **25 ms** | 3.4× |

**key-to-screen 与 echo 不随构建变化，因为它们等的是 ConPTY 不是我们**：
同一次运行里纯输入写入是 1 µs 量级。这两项的 5.5 ms 是 Windows 的往返成本，
不是引擎开销，也不是可以靠优化代码消除的部分。

### IPC 渲染路径（release，持久连接）

| 路径 | p50 | p95 |
|---|---:|---:|
| 进程内直读 styled screen | 37 µs | 82 µs |
| 跨 IPC 全量 styled screen | 4.8 ms | 19.8 ms |
| 跨 IPC 未变化探询 | 338 µs | 381 µs |

## 3. 本轮发现并修复的缺陷

按严重度排序。**每一条都是真机或跨重启才暴露的**：

1. **Core 崩溃后窗口看着正常**——GUI 存活、响应正常，屏幕是上一帧缓存，
   输入无反应且无任何说明。M1 门禁禁止把崩溃报成正常，静默冻结是最糟的
   一种。现检测事件流非预期中断并画出贯穿窗口的红色告知条。
2. **Core 模式下会话每次重启翻倍**——领养既有会话后又按保存的 cwd 列表
   各开一个。实测 6 pane 重开变 9。现仅冷启动时恢复额外 tab。
3. **Fleet 走错引擎**——多 agent 并行对进程内引擎建会话，Core 模式下
   落在空世界里，GUI 永远看不见。
4. **statsbar / cockpit / 录制导出 / scrollback-PNG 走错引擎**——同因，
   四处后台线程直接构造本地引擎。现统一走 `host_engine()`。
5. **光标错位**——确认横幅接进 status_bar_height 后，终端高度随「是否有
   待确认」突变，而 PTY 行数只在窗口 Resized 时同步：程序以为 N 行、
   画布按 N−1 行画。现横幅改为覆盖层，不占布局高度。
6. **Core 模式空转 IPC**——每个空闲 tick 对每个 pane 一次 `screen_revision`，
   即每 pane 一次 TCP 往返，20 pane 窗口每 tick 20 次，只为问「变了吗」。
   现改读 FrameCache 计数器（一次原子读、零上线）。
7. **窗口边缘不可拖拽**——全应用从未调用 `set_cursor`，边缘无任何提示；
   抓取带固定 6 物理像素，150% DPI 下仅 4 逻辑像素。现按点计量并补全
   八向光标与 L 形转角。
8. **状态目录隔离漏洞（3 处）**——`server_info`、`settings`、
   `session_restore` 都不认 `UNTERM_STATE_DIR`：测试与 headless 运行
   读写真实用户的实例注册表、配置与会话文件。

另修一处我自己在本轮引入的：新写的错误分支把「读超时」吞进通用错误分支，
worker 第一次超时就把活着的 Core 判成已死——由 frame_cache 收敛测试当场抓住。

## 4. 测试

| 套件 | 数量 | 状态 |
|---|---:|---|
| unterm-app | 611 | 全过 |
| unterm-engine | 584 | 全过 |
| unterm-mcp | 66 | 全过 |
| unterm-core（单元） | 13 | 全过 |
| unterm-core（进程 E2E） | 4 | 全过 |
| unterm-services | 121 | 全过 |
| unterm-cli | 31 | 全过 |

**注意口径**：`unterm-core` 的库测试共享进程内全局引擎，必须按 CI 的
Windows 口径 `--test-threads=1` 运行，否则并行测试互相看见对方的会话。

## 5. Core 模式为何还不宜转默认

已验证可用：会话建于 Core、杀 GUI 会话存活、重开领养且 scrollback 与
分屏排布保真、MCP 建会话落 Core、崩溃有告知、事件驱动渲染。

仍缺：

- **MCP server 生命周期仍随 GUI**。Core 自带一个 headless MCP 作故障转移，
  但 GUI 在场时 agent 连的是 GUI 那个——GUI 退出，agent 的控制面就断了。
  单一 MCP + McpHost 反向 IPC 是终态（M1-03c 阶段二）。
- **Core 崩溃后只能重启，不能重连**。告知条是诚实的，但恢复路径还是
  「重启 Unterm」。
- **GUI 渲染帧率与 TUI 抖动未量化**。需目视与录屏，不在自动化范围。
- **Core 模式端到端按键延迟未测**。现有数字是组件级。

## 6. 建议的发布形态

以 Local 为默认发布，Core 模式保留 `UNTERM_CORE_CLIENT=1` 实验开关并在
发行说明中如实标注其能力与缺口。转默认的前置条件：MCP 迁入 Core、
Core 崩溃可重连、TUI 抖动目视验收通过。
