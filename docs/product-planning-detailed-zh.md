# Unterm 产品规划

状态：详细规划稿  
更新时间：2026-07-27  
规划周期：12 个月  
适用范围：当前 WezTerm 内核、engine-neutral 产品层、实验性 `next-core`
当前进展：`next-core` 已具备启动环境元数据、typed launch policy provenance、domain/privilege/proxy-rotation/restart launch policy decision metadata、显式 launch policy 请求诊断、future-launch env overlay、session.create launch decision summary、default-shell launch decision、workspace.restore launch plan、profile/proxy launch-context 诊断、scrollback 文本截图、styled scrollback snapshot 与带配置主题调色板解析和粗体/斜体字体匹配的 styled PNG 渲染、styled_scrollback_png capability 诊断、styled scrollback renderer parity metadata、capture.scrollback 显式 pane id 活 session 校验、host-window bridge capability 诊断、instance title bridge ownership 诊断、实例生命周期 ownership 诊断、instance registry cleanup/active-pointer 诊断、shutdown dry-run lifecycle planning、protected registry unregister 与实例窗口元数据、PTY write confirmation capability/health 诊断、OSC 7 cwd、terminal status/cursor/private cursor/text-area-size/headless window-pixel-size/mode-report/primary and secondary device-attribute query response with parameterized DA forms in input order、session activity、input/output/paste 指标、exec.run_wait engine-neutral shell detection、session/exec/screen/agent.signal 核心 MCP handler 共享 pane-id resolver、active recording YAML markdown/redaction、chunked-output fallback 和 OSC133 command-block markdown、server health 聚合 I/O 诊断、逻辑 viewport goto、列宽感知自动换行/resize 截断、DECCKM application cursor key mode、DECAWM 自动换行模式、DECOM origin mode scroll-region 相对定位、DECSCNM reverse-video 模式、IRM insert mode、组合 mode set/reset 参数处理、HT/CHT/HTS/TBC/CBT tab-stop 光标移动与自定义 tab stop 控制、ESC charset/UTF-8 designator 消费、DCS/APC/PM/SOS control-string 消费、C1 CSI/OSC/string-control 和 IND/NEL/RI 处理、DECALN alignment-test fill、ESC IND/NEL/RI scroll-region 移动、DECSTR/RIS terminal reset、IL/DL scroll-region 内行插入删除、CSI CNL/CPL/HPA/HPR/VPA/VPR 定位、CSI/DEC private save/restore cursor、erase-line/erase-character styled blank backfill、delete-character 右边界 blank backfill、REP repeat-character、CSI 3J scrollback 清除、SGR bold/faint/italic/underline-style/underline-color/strikethrough/hidden/overline/blink/vertical-align/inverse 样式和分号/冒号 SGR 扩展色解析基础能力、agent startup stall benchmark 和 UTF-8 安全粘贴分块基础能力。

## 1. 产品结论

Unterm 的产品目标不是“再做一个终端”，而是做一个本地优先、AI agent 可控、人类可监督的现代终端工作台。

短期继续使用当前 WezTerm 内核交付稳定版本；中期把 MCP、CLI、Agent Cockpit、Fleet、Review、Profile、Recording 等产品能力从 WezTerm 内部抽离；长期用 `next-core` 验证自研终端内核是否能在输入、粘贴、滚动、多 agent 输出和外观一致性上超过当前内核。

关键决策：

- 不做一次性推倒重写。
- 当前内核只做 P0/P1 稳定性、性能和体验修复。
- 新功能优先落在 engine-neutral 产品层。
- `next-core` 先做窄 MVP，用 benchmark 决定是否继续扩大。
- 只有 `next-core` 明确胜出，才进入默认切换。

## 2. 产品定位

一句话：

> Unterm 是外部 AI agent 可以本地驱动、人类可以集中监督的现代终端。

它应该同时满足四类用户：

| 用户 | 主要诉求 | Unterm 必须提供的价值 |
|---|---|---|
| AI-heavy developer | 同时跑 Claude、Codex、Gemini、Aider | 输入不卡、状态可见、等待优先、可快速接管 |
| Agent orchestrator | 通过 MCP 自动操作真实终端 | 创建会话、输入、读屏、截图、录制、审查都有结构化 API |
| 多身份开发者 | 工作/个人/客户账号隔离 | 一个窗口绑定一套 Profile，secret 留在系统保险箱 |
| 终端高频用户 | 日常终端必须快、稳、好看 | tab、pane、scrollback、搜索、粘贴、快捷键、主题都可靠 |

## 3. 北极星指标

产品北极星：

- 用户可以同时运行多个 AI agent，并且 Unterm 仍像原生终端一样顺滑。
- 外部 agent 可以通过 MCP 完成常见终端操作，不依赖脆弱的屏幕识别。
- 人类可以一眼知道哪个 agent 在工作、哪个在等输入、哪个值得 review。

性能北极星：

| 指标 | 目标 |
|---|---:|
| 普通输入到可见字符 p95 | < 16 ms |
| 两个 agent 输出时输入 p95 | < 33 ms |
| UI stall p99 | < 100 ms |
| 右键粘贴 10 KB token-like 文本 | 一次成功，UI 不冻结 |
| PageUp/PageDown 连续翻页 | 无明显停顿 |
| MCP `screen.text` 在输出洪峰下响应 | p95 < 50 ms |
| 实例发现误删 live instance | 0 |

## 4. 产品原则

1. 响应优先  
   输入、粘贴、滚动、tab 切换和补全接受必须高于 sidebar、agent 状态、Git 面板和动画。

2. 本地优先  
   MCP、HTTP Settings、Profile、Recording、Fleet、Review 默认都在本机运行，不依赖云服务。

3. Agent 可控，但人类最终负责  
   agent 可以操作终端，但发布、删除、付款、合并、强制回滚等不可逆动作必须保留明确边界和审计。

4. GUI / CLI / MCP 尽量同构  
   核心能力不能只藏在一个界面里。用户、脚本、agent 都应该能找到同一件事。

5. 引擎无关  
   产品能力不能继续深绑 WezTerm 的 `Mux`、`Pane`、`TermWindow`、renderer 或平台窗口对象。

6. 用数据决定重写是否继续  
   `next-core` 的价值必须由延迟、稳定性、可维护性和功能迁移速度证明。

## 5. 用户场景闭环

### 5.1 多 agent 日常开发

用户打开一个 repo，同时启动 Claude 和 Codex。Unterm 要做到：

- tab/sidebar 显示每个 agent 的名称、项目、状态。
- 两个 agent 输出时，用户打字、滚动、tab 切换不卡。
- agent 需要确认时进入 Inbox。
- Enter 跳到正确实例、tab、pane。
- 用户输入后 agent 状态恢复 working。

验收：

- 2 Claude + 1 Codex + 1 PowerShell 连续工作 30 分钟无卡死。
- tab 切换 100 次不触发 Rename tab。
- Inbox 跨窗口跳转正确。

### 5.2 外部 agent 驱动终端

外部 agent 通过 `~/.unterm/instances/*.json` 找到实例，连接 MCP，完成：

- `auth.login`
- `session.list`
- `session.create`
- `session.input` / `exec.run`
- `screen.text`
- `screen.search`
- `capture.screen`
- `session.export_markdown`

验收：

- 不需要 screen scraping。
- 高输出期间 MCP 仍可响应。
- mutating call 有审计和策略边界。

### 5.3 Fleet 并行做任务

用户把一个任务交给多个 agent：

- Unterm 检查 repo 状态。
- 创建隔离 worktree。
- 每个成员一个 tab。
- Review 展示 diff。
- Verify 跑测试。
- Merge 只把通过验证的候选 staged 到主 repo，commit 由用户完成。

验收：

- base repo 不被覆盖。
- failed member 可 retry。
- force merge / rollback 有审计。

### 5.4 多身份工作

用户为 Work / Personal / Client 配不同 Profile：

- 每个窗口绑定一个 profile。
- 新 pane 自动注入 env、git identity、SSH routing。
- secret 存 OS vault，不写 TOML。
- MCP 只能读 profile metadata，不能读 secret。

验收：

- profile-bound shell 能拿到正确环境。
- UI 明确显示当前 profile。
- MCP response 不含 raw secret。

## 6. 产品模块规划

| 模块 | 当前状态 | 12 个月目标 | 优先级 |
|---|---|---|---|
| Core Terminal | WezTerm 内核，存在卡顿和耦合 | 当前内核稳定，`next-core` 可切换 | P0 |
| MCP Control Plane | 已有 100 个 public methods | engine-neutral，能力矩阵准确 | P0 |
| CLI | 覆盖主要功能 | 与 MCP/GUI 对齐，默认实例路由稳定 | P0 |
| Agent Cockpit | 已有状态、Inbox、Fleet、Review | 状态更准，等待不漏报，迁移到 next-core | P1 |
| Composer / Suggestions | 已有 ghost/suggest | 输入路径零阻塞，右箭头可靠 | P0 |
| Recording | 已有 markdown 导出 | active/inactive 录制统一，next-core 可复用 | P1 |
| Profile | 已有身份规划和部分实现 | 一窗口一身份，secret 全进 OS vault | P1 |
| Proxy | 已有代理 surface | profile/env/agent 场景稳定 | P1 |
| Web Settings | 已有本地 HTTP UI | 复杂配置都进 Web，Review UI 成熟 | P2 |
| Fleet / Review / Verify | 已有差异化能力 | 可日常用，失败可恢复，可审计 | P1 |
| Capture / Screenshot | 已有 screen/window/scrollback | current/next-core 能力清晰，scrollback PNG parity | P2 |
| Distribution | 已有多平台产物规划 | Windows MSI、macOS notarize、Linux artifact 稳定 | P1 |

## 7. 版本路线图

### v0.58：当前内核止血版

目标：先把用户已经遇到的卡死和输入问题修掉。

范围：

- 修复输入慢、粘贴偶发失败、补全右箭头失效。
- 修复 PageUp/PageDown 翻页卡壳。
- 修复 tab 切换误触 Rename tab。
- 修复打开项目栏选择目录时 UI 跑动。
- 定位并修复 Codex 启动瞬间卡死。
- 修复任务栏点击导致死机。
- 修复 Windows instance false deletion。
- 增加 slow-frame / input-latency / paste-latency 诊断。

不做：

- 大 UI 改版。
- 新 agent 功能。
- WezTerm 深层重构。

验收：

- `cargo check -p unterm`
- input/paste/completion 相关单测
- Windows 手动 smoke test
- 2 Claude + 1 Codex 场景无明显冻结

### v0.59：Engine Boundary 版

目标：产品层继续脱离 WezTerm 内部。

范围：

- MCP handler 不再直接触达 WezTerm 深层类型。
- `SessionEngine`、`InputEngine`、`ScreenEngine`、`CaptureEngine`、`WindowEngine` 覆盖核心路径。
- `docs/engine-dependency-map.md` 中 WezTerm-only 保持为 0。
- `capture.scrollback` 的 next-core 语义明确：已支持 styled cell snapshot、配置主题 palette 解析、粗体/斜体字体匹配和基础 styled PNG，后续随 next-core GUI renderer 补真实视口渲染路径。
- `server.capabilities` / `meta.surface` 表达 per-engine 能力和诊断指标，尤其让 agent 能看见 next-core 的受限能力、health I/O 指标、launch-context 诊断能力、typed launch policy provenance 和 domain/privilege/proxy-rotation/restart 决策元数据，而不是猜测。

验收：

- engine-neutral handler tests 全通过。
- current-core 行为不变。
- next-core adapter 能实现最小子集，不链接 WezTerm GUI 深层对象。
- `server.health` 能暴露 next-core 聚合输入、输出、粘贴诊断指标。
- `selftest.run` 能验证 next-core capability 声明、health I/O payload、profile/proxy launch-context redaction、typed launch policy provenance、launch policy decision metadata、逻辑 viewport 滚动能力和 styled scrollback PNG 渲染能力。
- `screen.search(goto)` 和 `screen.scroll(goto/apply)` 在 next-core 中能更新逻辑 viewport，后续 `screen.text` 可读到目标区域。

### v0.60：next-core Spike

目标：证明自研内核值得继续。

范围：

- Windows 11 + ConPTY。
- 单窗口、单 tab、单 shell。
- 基础 VT parse / cols+rows screen model / scrollback。
- keyboard input、paste、visible text read。
- 最小 MCP：`session.list`、`session.create`、`session.input`、`session.paste`、`screen.text`、`exec.run`。
- benchmark harness，包含 input write、echo、output flood、scrollback paging、viewport scroll、paste、dual-agent、agent startup stall、screen-read under flood、focus switch、session create、session ready。

验收：

- key input p95 < 16 ms。
- paste 10 KB < 50 ms。
- 100k lines output 不阻塞输入。
- MCP 在 output flood 下仍响应。
- 架构比 current-core 更简单，核心路径可解释。

### v0.61-v0.62：next-core Alpha

目标：内部可日常使用。

范围：

- tabs、splits、selection、copy/paste、search、scrollback。
- CJK width、emoji fallback MVP。
- title/cwd/process metadata。
- profile/proxy env injection，基于 engine-neutral typed launch policy、launch env overlay 和 next-core future-launch env overlay。
- recording MVP。
- agent state MVP。
- CLI/MCP session/input/screen parity。

验收：

- 工程团队可用 next-core 工作一天。
- Claude/Codex 至少两种 agent 可跑。
- Agent Cockpit 可显示基本状态。
- session export markdown 可用。

### v0.63-v0.65：产品能力迁移

目标：把 Unterm 差异化能力迁到 next-core。

迁移顺序：

1. MCP/CLI core parity
2. Agent Cockpit
3. Inbox
4. Composer / Suggestions
5. Recording
6. Profile / Proxy
7. Workspace
8. Screenshot / scrollback render
9. Fleet
10. Review / Verify / Merge
11. Web Settings / Review UI

验收：

- 每个迁移模块都有 current-core 与 next-core capability 标记。
- current-core 不回退。
- next-core 功能缺口在文档和 API 中明确。

### v0.66+：Beta 与默认切换

目标：让 next-core 成为部分平台默认内核。

发布顺序：

1. 内部 dogfood。
2. hidden env flag。
3. CLI flag。
4. Web Settings experimental toggle。
5. Windows beta default。
6. 跨平台 beta。
7. current-core fallback 保留一个大版本。

默认切换条件：

- Windows 日常使用稳定。
- agent-heavy 场景明显更顺滑。
- MCP core 无回归。
- recording/profile/proxy 无关键回归。
- install/update 可用。
- crash recovery 和 instance discovery 稳定。

## 8. P0 问题拆解

### 8.1 输入、粘贴、补全同源卡顿

现象：

- 打字响应慢。
- 右键粘贴认证码偶发失败。
- 指令补全时右箭头偶发无效。
- 命令补全卡壳。

产品判断：

这些问题高度可能来自同一个主路径：key event、ghost completion、clipboard retry、agent detection、UI invalidate 或历史状态读写互相抢主线程。

要求：

- per-key 不读磁盘。
- per-key 不扫描 agent manifest。
- per-key 不 clone 大历史。
- paste 使用状态机，失败有 retry，但 retry 不阻塞 UI。
- right arrow、application right arrow、End 走同一个 completion accept path。
- 大文本 paste 分块或 bracketed paste。

### 8.2 Tab、项目栏、翻页、任务栏卡死

现象：

- 打开项目栏选择目录 UI 跑动。
- tab 切换进入 Rename tab。
- PageUp/PageDown 卡壳。
- 任务栏点击死机。

产品判断：

这些是 GUI chrome 与 terminal viewport 互相干扰的问题，必须保证 chrome 重算、目录扫描、agent 状态刷新不能进入 paint / input 热路径。

要求：

- left sidebar 数据有 TTL 和后台刷新。
- paint 不做 cwd/process/tree scan。
- tab hit-test 与 rename trigger 解耦。
- scrollback paging 不触发全 sidebar 重排。
- Windows minimize/restore/taskbar click 有专门 smoke test。

### 8.3 Codex 启动卡死

现象：

- 打开 Codex 的瞬间明显卡死。

产品判断：

需要重点排查 Codex 启动时是否触发：

- stdout/stderr flood。
- MCP 初始化。
- AGENTS.md/context 注入。
- agent detection。
- shell integration。
- clipboard / env / cwd 探测。

要求：

- Codex 启动路径增加阶段耗时记录。
- agent setup 不在 UI 线程做。
- process/cwd 探测限流。
- output flood 与 UI render 分离。

### 8.4 实例丢失

现象：

- 运行中的实例偶发从 instance list 消失。

产品判断：

Windows PID inspection 不可靠时不能直接删除实例文件。实例管理要偏保守，宁可标记 uncertain，也不要误删 live instance。

要求：

- per-instance JSON 是主 source。
- 当前进程内存状态优先。
- stale cleanup 必须多次确认。
- `active.json` 和 `server.json` 可自愈。

## 9. next-core 产品定义

`next-core` 不是为了追求“纯自研”而重写。它只解决当前内核难以解决的三件事：

1. 输入、粘贴、滚动、输出调度更简单可控。
2. 渲染和 UI chrome 更容易做成现代、稳定、一致的体验。
3. MCP/agent 读写路径从第一天就是内核能力，不是后挂插件。

MVP 范围：

- Windows-first。
- ConPTY。
- 单窗口、单 tab、本地 shell。
- 基础 ANSI。
- scrollback text。
- input / paste。
- visible screen read。
- MCP session/input/screen/exec。

明确后置：

- SSH。
- mux。
- Lua config。
- 图片协议。
- ligature。
- 全量 TUI 兼容。
- 全量 copy mode。
- Fleet / Review UI。

毕业标准：

- benchmark 明确赢。
- 架构明显简单。
- 多 agent 输出不拖慢输入。
- screen read 不锁 UI。
- 后续迁移成本可控。

## 10. 功能优先级

| 优先级 | 功能 | 理由 |
|---|---|---|
| P0 | 输入响应 | 终端第一体验 |
| P0 | 粘贴可靠性 | Claude/Codex auth 和 prompt 高频刚需 |
| P0 | 滚动/翻页 | agent 输出场景核心 |
| P0 | tab 切换 | 多 agent 场景核心 |
| P0 | instance discovery | MCP 和多窗口基础 |
| P0 | MCP session/input/screen | 产品核心控制面 |
| P1 | Agent Cockpit | 核心差异化 |
| P1 | Profile | 多身份和 secret 安全 |
| P1 | Recording | debug、复盘、agent 审计 |
| P1 | Fleet / Review / Verify | 高阶差异化 |
| P2 | Web Settings | 配置体验，但不是性能 blocker |
| P2 | Capture / Screenshot | 有价值但依赖核心稳定 |
| P3 | SSH / mux parity | 等 next-core 本地 core 成熟 |
| P4 | Lua 兼容 | 避免继承旧复杂度 |

## 11. 验证计划

### 11.1 每日手动 smoke

Windows 优先，每天跑：

1. 打开 Unterm。
2. 启动两个 Claude。
3. 启动一个 Codex。
4. 保留一个普通 PowerShell。
5. 连续输入 100 个字符。
6. 右键粘贴 10 KB token-like 文本。
7. ghost completion 右箭头接受 30 次。
8. 连续 PageUp/PageDown 100 次。
9. 切换 tab 100 次。
10. 打开项目栏并选择目录。
11. 点击任务栏最小化/恢复 20 次。
12. 跑 `unterm-cli instance list`。
13. 跑 `unterm-cli reference --section mcp`。

通过标准：

- 无死机。
- 无明显冻结。
- 无误触 Rename tab。
- 无粘贴丢字符。
- 无 live instance 丢失。

### 11.2 自动测试

必须持续维护：

- `cargo check -p unterm`
- `cargo check -p unterm-engine --bins`
- MCP engine-neutral handler tests
- ghost completion tests
- input/paste path tests
- instance liveness tests
- recording/export tests
- benchmark harness

### 11.3 Benchmark

固定场景：

- key input latency。
- paste 10 KB。
- output flood 100k lines。
- two pseudo-agent output streams。
- scrollback page read 10k lines。
- MCP screen read during flood。
- tab switch latency。
- Codex/agent startup stall probe。

## 12. 组织执行方式

### Track A：当前内核稳定

目的：保护当前用户。

节奏：

- 小 PR。
- 每个 PR 只修一个用户可感知问题。
- 每周一批 Windows build。
- 不做无关重构。

### Track B：产品层抽离

目的：让产品能力可迁移。

节奏：

- 接口优先。
- 行为不变。
- MCP/CLI 输出形状不变。
- 每迁一个方法就更新 dependency map。

### Track C：next-core

目的：验证未来内核。

节奏：

- benchmark 驱动。
- Windows-first。
- spike 里允许快速试错。
- 不承诺全量兼容，先证明核心体验。

## 13. 风险与应对

| 风险 | 影响 | 应对 |
|---|---|---|
| 重写失控 | 一年后无法替代 current-core | MVP 极窄、benchmark 毕业、current-core 继续发版 |
| 终端兼容性不足 | TUI / shell 行为不正确 | 使用成熟 VT parser，建立兼容矩阵 |
| 字体和渲染复杂 | 卡在 CJK/emoji/ligature | 分阶段支持，先 ASCII/CJK width，再 emoji/ligature |
| 双内核维护成本 | bug 修两遍 | 产品层共享，current-core 只修 P0/P1 |
| 当前体验继续伤害口碑 | 用户在 next-core 前流失 | v0.58 先修卡死、输入、粘贴、滚动 |
| MCP surface 漂移 | agent 集成不稳定 | `meta.surface` 做 live source，文档随 PR 更新 |
| Secret 泄露 | 严重安全问题 | Profile secret 只进 OS vault，MCP 只读 metadata |

## 14. 近期 2 周执行清单

按顺序：

1. 固化当前 engine boundary 提交和文档。
2. 补 slow-frame / input-latency / paste-latency 诊断。
3. 排查 Codex startup stall。
4. 排查输入、粘贴、ghost completion 是否共用阻塞路径。
5. 修 right arrow completion accept。
6. 当前内核修右键粘贴 retry；`next-core` 已完成 UTF-8 安全 paste chunk 和 bracketed paste marker 保留。
7. 修 PageUp/PageDown 卡顿。
8. 修 tab switch 误触 Rename tab。
9. 修项目栏目录选择跑动。
10. 修 Windows instance false deletion。
11. 补 Windows 手动 smoke checklist。
12. 启动 next-core ConPTY spike。

## 15. 决策看板

| 决策 | 当前建议 | 复审时间 |
|---|---|---|
| 是否完全重写 | 不一次性重写，做受控迁移 | next-core spike 完成后 |
| 是否保留 WezTerm | 保留 current-core 作为 fallback | next-core beta 后 |
| 是否做 Lua 兼容 | 暂不做 | next-core alpha 后 |
| 是否优先 Windows | 是 | v0.60 后 |
| 是否复制 Warp AI 形态 | 不复制，只学习流畅度和现代感 | 持续 |

## 16. 最终路线

最可控的产品路线是：

1. v0.58 先解决当前用户遇到的卡死和输入体验问题。
2. v0.59 完成 MCP/CLI/product service 的 engine-neutral 边界。
3. v0.60 做最小 `next-core`，用 benchmark 决定是否继续。
4. v0.61-v0.62 把 `next-core` 做到内部日常可用。
5. v0.63-v0.65 迁移 Agent Cockpit、Recording、Profile、Fleet、Review。
6. v0.66+ 在 Windows 先 beta 默认，current-core 保留 fallback。

这条路线能同时保护当前用户、保住 Unterm 已经形成的 agent-first 差异化，并为未来达到 Warp 级别的流畅度和视觉质量留出空间。
