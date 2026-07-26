# Unterm 详细产品规划

状态：规划稿  
负责人：产品 / 工程  
更新时间：2026-07-26  
规划周期：12 个月  
关联文档：

- `docs/product-requirements.md`
- `docs/product-plan-next-core.md`
- `docs/next-core-benchmark-report.md`

## 1. 结论

Unterm 应该继续保留当前 WezTerm 内核作为可交付版本，同时启动自研 `next-core`。这不是一次性推倒重来，而是双轨迁移：

1. 当前内核只修 P0/P1 稳定性和体验问题。
2. 产品能力全部抽到引擎无关层。
3. `next-core` 从最窄的 Windows 本地 shell + MCP 基础能力开始。
4. 只有当 `next-core` 在输入、粘贴、滚动、多 agent 输出和 MCP 响应上明确胜出，才进入默认切换。

Unterm 的产品核心不是“又一个终端模拟器”，而是：

> 一个 AI agent 可以本地控制、人可以集中监督的现代终端。

## 2. 当前问题清单

这些问题直接决定近期优先级。

### 2.1 已观察到的问题

1. 打开项目栏选择目录时 UI 出现跑动。
2. 右键粘贴 Claude 认证码时经常第一次失败，后面又成功。
3. 打字时输入内容响应慢。
4. 指令补全时偶发右箭头无效。
5. 任务栏点击后出现直接死机。
6. 昨天出现过莫名丢失实例。
7. 切换 tab 时出现误进入 Rename tab 状态。
8. 同时打开两个 Claude 后仍有卡顿。
9. 打开 Codex 的瞬间明显卡死。
10. 命令补全时卡壳。
11. PageUp/PageDown 或翻页时卡壳。

### 2.2 产品判断

这些不是孤立 bug，而是同一类系统问题：

- 输入路径、补全路径、粘贴路径存在同步阻塞。
- UI 绘制路径可能混入 agent 状态、cwd、进程树、sidebar 等昂贵计算。
- WezTerm 内核和 Unterm 产品层耦合太深，定位和替换成本高。
- 多 agent 场景下，输出、状态检测、MCP、UI chrome 互相抢主线程或锁。
- Windows 上剪贴板、实例 PID 探测、窗口焦点切换更容易触发边界问题。

所以短期要修卡顿，长期要重写核心架构。

## 3. 产品定位

### 3.1 目标用户

1. AI-heavy developer  
   同时跑 Claude Code、Codex、Gemini、Aider，需要终端不卡、人能接管。

2. Agent orchestrator  
   外部 agent 通过 MCP 控制终端，不依赖屏幕识别。

3. 多身份开发者  
   用不同 GitHub、AWS、npm、OpenAI、SSH 身份工作，需要窗口级身份隔离。

4. 高级终端用户  
   需要稳定、快、好看、可配置、跨平台的日常终端。

### 3.2 差异化

| 产品 | 优势 | 局限 | Unterm 的机会 |
|---|---|---|---|
| Warp | 体验现代、输入顺滑、外观强 | AI 偏闭源云服务，agent 可控性弱 | 做到接近 Warp 的体验，同时本地 MCP 开放 |
| Ghostty | 轻、快、终端纯粹 | 不解决 agent 操作和监督 | 在纯终端体验上达标，再加入 Agent Cockpit |
| iTerm2 | Mac 生态成熟 | Mac-only，AI 编排弱 | 跨平台 + agent-first |
| Windows Terminal | Windows 稳定 | 产品层较薄，agent 生态弱 | Windows-first agent 终端 |
| WezTerm | 功能强、跨平台 | 内核大，改动难，产品层深耦合 | 先借力，再迁出 |

### 3.3 北极星

Unterm 要成为“本地 agent 工作台”：

- 终端本身快到可以日常使用。
- 外部 agent 能通过 MCP 操作终端。
- 人能看到所有 agent 的状态。
- 多 agent 的工作可以验证、比较、合并、回滚。
- 身份、密钥、危险操作都留在本地和用户手里。

## 4. 产品原则

1. 响应优先  
   输入、粘贴、滚动、tab 切换、补全必须高于 sidebar、动画、agent 状态刷新。

2. 本地优先  
   MCP、HTTP 设置、录制、profile、fleet、review 都默认本地运行。

3. 引擎无关  
   MCP、CLI、Agent Cockpit、Fleet、Review、Profile 不应该绑定 WezTerm 内部类型。

4. 窄范围验证  
   `next-core` 不追求一开始全功能，只验证最核心体验是否能赢。

5. 不做云 AI 终端  
   Unterm 不把内置 AI chat 作为主产品，而是让外部 agent 控制终端。

6. 功能必须可自动化  
   重要能力应尽量同时有 GUI / CLI / MCP surface。

## 5. 产品架构规划

### 5.1 目标架构

```text
Unterm Product Layer
  MCP server
  CLI bridge
  Web Settings
  Agent Cockpit
  Fleet / Review / Verification
  Profile / Proxy / Recording
  Instance discovery / Policy / Audit
        |
        v
Engine-neutral Interfaces
  SessionEngine
  ScreenEngine
  InputEngine
  MetadataEngine
  CaptureEngine
  WindowEngine
        |
        +-- Current WezTerm Adapter
        +-- Experimental next-core Adapter
```

### 5.2 必须抽象的接口

1. Session
   - list
   - create
   - split
   - focus
   - destroy
   - resize
   - active

2. Input
   - key input
   - paste
   - control signal
   - bracketed paste
   - IME composition boundary

3. Screen
   - visible text
   - cursor
   - scrollback page
   - search
   - selection

4. Metadata
   - title
   - cwd
   - foreground process
   - agent identity
   - agent state
   - profile

5. Capture
   - pane screenshot
   - scrollback screenshot
   - window screenshot
   - clipboard capture

6. Product services
   - MCP handler 不直接依赖 WezTerm GUI 深层对象。
   - CLI 和 Web 通过同一套 product service 调用。
   - `meta.surface` 是 live capability source。

## 6. 阶段规划

## Phase 0：当前内核止血

周期：现在到 2 周  
目标：让当前版本在 agent-heavy 场景下不再明显冻结。

### 6.1 范围

P0：

- 输入卡顿
- 粘贴失败
- 补全右箭头无效
- PageUp/PageDown 卡壳
- tab 切换误触 Rename tab
- Codex 启动瞬间卡死
- 任务栏点击死机
- 实例丢失

P1：

- 项目栏目录选择跑动
- 左侧项目栏重绘性能
- Git panel / agent tally 刷新抖动
- slow-frame 诊断开关

明确不做：

- 新大功能
- 大规模 UI 改版
- WezTerm 深层重构
- Lua 兼容性扩展

### 6.2 技术方向

1. 输入路径
   - per-key 不读磁盘。
   - per-key 不 clone 大历史。
   - per-key 不扫描 agent manifest。
   - right arrow / application right arrow / End 统一走 completion accept。

2. 粘贴路径
   - Windows clipboard retry 放到非 UI 阻塞路径。
   - 右键 paste 必须有明确状态机。
   - 大 token 粘贴走 chunk 或 bracketed paste。
   - paste 完成前不触发无关 UI 刷新。

3. 绘制路径
   - paint 不做进程树扫描。
   - paint 不刷新所有 pane cwd。
   - sidebar 数据缓存 TTL。
   - agent 状态刷新限流。

4. 实例路径
   - Windows PID 探测失败不能立即删除实例。
   - active/server 文件可以自愈。
   - 内存中的当前实例状态优先。

5. Codex 启动卡顿
   - 检查启动瞬间 stdout flood、MCP setup、AGENTS.md 读写、agent detection、shell integration。
   - 所有 agent setup 和 detection 必须后台化、限流、可取消。

### 6.3 验收

手动验收：

1. 同时打开 2 个 Claude + 1 个 Codex。
2. 启动 Codex 的瞬间 UI 不冻结超过 100 ms。
3. 连续输入 100 个字符没有明显延迟。
4. 右键粘贴 10 KB token-like 文本一次成功。
5. 补全 ghost text 用右箭头接受 30 次不失败。
6. PageUp/PageDown 快速翻 10000 行 scrollback 不明显卡住。
7. tab 切换 50 次不误出现 Rename tab。
8. 任务栏点击、最小化、恢复不死机。
9. 多窗口实例列表不丢失 live instance。

自动验收：

- `cargo check -p unterm`
- `cargo check -p unterm-engine --bins`
- input/paste/completion 相关单测
- next-core benchmark report 可生成

## Phase 1：产品层抽离

周期：2 到 6 周  
目标：让 Unterm 产品能力不再绑死 WezTerm 内部结构。

### 6.4 交付物

1. Engine dependency map  
   每个 MCP 方法标注依赖：
   - product-only
   - session
   - input
   - screen
   - metadata
   - capture
   - window
   - WezTerm-only legacy

2. Engine traits v1  
   Rust trait 覆盖 session/input/screen 基础能力。

3. WezTerm adapter  
   当前行为不变，只是从 handler 直接调用改为 adapter 调用。

4. next-core adapter MVP  
   支持最小方法：
   - `session.list`
   - `session.create`
   - `session.input`
   - `session.paste`
   - `screen.text`
   - `screen.read`
   - `exec.run`
   - `server.info`
   - `server.health`

5. Capability matrix  
   文档和 `meta.surface` 标明当前 engine 支持什么。

### 6.5 验收

- 默认 WezTerm engine 行为不变。
- `UNTERM_ENGINE=next-core` 能跑 MCP 基础输入和 screen read。
- MCP handler 对基础 session/input/screen 不再依赖深层 GUI 类型。
- 新增 engine adapter 测试。

## Phase 2：next-core 技术验证

周期：6 到 12 周  
目标：证明自研内核值得继续。

### 6.6 MVP 范围

支持：

- Windows 11
- ConPTY
- 单窗口
- 单 tab
- 本地 shell
- 键盘输入
- paste
- 基础 ANSI 颜色
- scrollback text
- visible screen read
- MCP 基础 session/input/screen/exec

暂不支持：

- SSH
- mux
- 图片协议
- 复杂字体 shaping
- Lua config
- 全量快捷键
- 复杂 copy mode
- Fleet/Review 全集成

### 6.7 核心指标

| 指标 | 目标 |
|---|---:|
| key input to visible glyph p95 | < 16 ms |
| 两个 agent 输出时 input p95 | < 33 ms |
| paste 10 KB | < 50 ms |
| screen read under flood p95 | < 5 ms |
| scrollback page read p95 | < 1 ms |
| UI stall p99 | < 100 ms |
| 100k 行输出期间 MCP 可响应 | 是 |

### 6.8 技术选择

PTY：

- Windows：ConPTY
- Unix：后续用 mature PTY crate

VT parser：

- 优先用成熟 parser，例如 `vte` 或复用可控的 termwiz parser。
- 不从零手写完整 VT parser。

渲染：

- 优先研究 `wgpu`。
- Windows-first，先确保事件循环和输出调度。

字体：

- 第一阶段支持 ASCII / CJK width 基础正确。
- 第二阶段验证 CJK、emoji、fallback。
- ligature 后置。

状态模型：

- 独立 screen model。
- scrollback ring / paged buffer。
- input queue 和 render queue 分离。
- PTY reader 不直接阻塞 UI。

### 6.9 毕业标准

`next-core` 继续投入的条件：

- 基础 benchmark 明显好于当前内核。
- 架构简单，可解释，可测试。
- MCP 基础能力稳定。
- 两个 agent 输出不会拖慢输入。
- scrollback 和 screen read 不锁 UI。

如果达不到，停止扩大范围，只保留研究成果。

## Phase 3：next-core Alpha

周期：3 到 5 个月  
目标：内部可以日常使用。

### 6.10 Alpha 功能

终端基础：

- tabs
- splits
- selection
- copy
- paste
- search
- scrollback
- resize
- title update
- theme colors
- basic keybindings

agent 基础：

- agent process detection
- waiting / working / idle / done
- tab badge
- topbar tally
- inbox

MCP/CLI：

- session/input/screen/exec
- instance
- server/system
- policy/audit basics
- recording MVP

配置：

- profile env injection
- proxy env injection
- theme/lang basic

### 6.11 Alpha 验收

- 工程团队可以用 next-core 工作一整天。
- Claude/Codex/Gemini/Aider 至少两种 agent 可运行。
- 输入、paste、scroll、tab 切换优于当前内核。
- MCP agent 可以创建 pane、输入、读屏、跑命令。
- 录制能导出 markdown。
- profile-bound shell 能拿到 env。

## Phase 4：产品能力迁移

周期：5 到 8 个月  
目标：把 Unterm 差异化能力迁到 next-core。

迁移顺序：

1. MCP / CLI core parity
2. Agent Cockpit
3. Inbox
4. Composer / suggestions
5. Recording
6. Profiles
7. Proxy
8. Workspace
9. Screenshot / scrollback render
10. Fleet launch
11. Review / Verify / Merge
12. Web Settings / Review UI

每迁一个功能，都必须有：

- current-core 行为不回退。
- next-core capability 标记清楚。
- CLI/MCP smoke test。
- 文档同步。

## Phase 5：Beta 与默认切换

周期：8 到 12 个月  
目标：next-core 成为部分平台默认内核。

### 6.12 发布节奏

1. 内部 dogfood。
2. 隐藏环境变量。
3. CLI flag。
4. Web Settings experimental toggle。
5. Windows beta 默认。
6. 跨平台 beta。
7. 保留 WezTerm fallback 一个大版本。
8. 根据使用量和 bug 量决定是否移除 WezTerm。

### 6.13 默认切换条件

- Windows 日常稳定。
- agent-heavy 场景明显更流畅。
- MCP core 无回归。
- recording/profile/proxy 无关键回归。
- install/update 可用。
- crash recovery 可用。
- instance discovery 稳定。
- 文档和 capability matrix 同步。

## 7. 功能优先级

| 功能 | 当前内核 | next-core | 优先级原因 |
|---|---:|---:|---|
| 输入响应 | P0 | P0 | 每天都感知 |
| 粘贴可靠性 | P0 | P0 | Claude/Codex 认证码刚需 |
| 滚动/翻页 | P0 | P0 | agent 输出很多 |
| tab 切换 | P0 | P0 | 多 agent 必用 |
| 补全右箭头 | P0 | P0 | 输入体验破坏大 |
| 任务栏/窗口焦点稳定 | P0 | P1 | Windows 体验底线 |
| 实例发现 | P0 | P0 | MCP 和多窗口基础 |
| MCP session/input/screen | P0 | P0 | 产品核心 |
| Agent Cockpit | P1 | P1 | 差异化 |
| Inbox | P1 | P1 | 人机协作入口 |
| Profile | P1 | P1 | 身份安全 |
| Recording | P1 | P1 | debug 和复盘 |
| Fleet | P1 | P2 | 强差异化但依赖核心稳定 |
| Review/Verify | P1 | P2 | 安全层 |
| Web Settings | P2 | P2 | 重要但非性能 blocker |
| 外部长截图 | P3 | P3 | 平台特性 |
| SSH/mux | P3 | P3 | 等本地 core 成熟 |
| Lua 兼容 | P4 | P4 | 不继承复杂度 |

## 8. 体验规格

### 8.1 终端交互体验

1. 打字
   - 无明显输入延迟。
   - agent 输出期间仍可输入。
   - IME 不被破坏。

2. 粘贴
   - 右键粘贴一次成功。
   - 大文本粘贴不卡 UI。
   - 认证码不丢字符、不重复。

3. 补全
   - ghost text 出现不阻塞。
   - 右箭头接受可靠。
   - 补全计算不读磁盘、不扫全局状态。

4. 滚动
   - PageUp/PageDown 连续操作不卡。
   - 鼠标滚轮平滑。
   - scrollback read 不影响输入。

5. tab
   - 切换立即响应。
   - 不误触 Rename tab。
   - 多 agent tab badge 不拖慢渲染。

### 8.2 Agent Cockpit 体验

1. 等待优先  
   需要用户输入的 agent 必须被顶到 Inbox 前面。

2. 跨实例跳转  
   点击或 Enter 必须跳到正确窗口、tab、pane。

3. 状态可信  
   working / waiting / idle / done 不追求花哨，但不能频繁误报。

4. 不干扰终端  
   agent 状态刷新不能影响打字、粘贴、滚动。

## 9. 工程执行模型

### Track A：当前内核稳定

节奏：小 PR，快速验证  
原则：只修用户能感知的稳定性和性能问题。

每个 PR 必须回答：

- 修哪个用户问题？
- 是否影响输入/粘贴/滚动主路径？
- 如何验证？
- 是否能被 next-core 复用？

### Track B：产品层抽离

节奏：接口优先，行为不变  
原则：把 Unterm 产品能力和 WezTerm 内部解绑。

每个 PR 必须回答：

- 去掉了哪个 WezTerm 深层依赖？
- current-core 行为是否不变？
- next-core 是否能实现同一接口？

### Track C：next-core

节奏：benchmark 驱动  
原则：只在 next-core 内部允许激进重构。

每个 PR 必须回答：

- 哪个 benchmark 变好了？
- 哪个兼容性风险被引入？
- 是否让架构更简单？

## 10. 里程碑

### M1：当前内核可用

目标：2 周  
交付：

- Windows 输入/粘贴/补全/滚动修复。
- Codex 启动卡死定位和修复。
- 实例丢失修复。
- Rename tab 误触修复。
- Windows smoke checklist。

### M2：Engine Interface v1

目标：4 到 6 周  
交付：

- engine dependency map。
- session/input/screen traits。
- WezTerm adapter。
- next-core adapter MVP。
- MCP handler 使用 engine trait。

### M3：next-core Spike

目标：8 到 12 周  
交付：

- standalone next-core binary。
- Windows shell。
- 基础 VT/screen/scrollback。
- input/paste benchmark。
- output flood benchmark。
- agent startup stall benchmark。
- MCP basics。

### M4：next-core Alpha

目标：3 到 5 个月  
交付：

- tabs/splits/selection/search。
- Agent Cockpit MVP。
- recording MVP。
- profile/proxy env。
- internal dogfood build。

### M5：产品迁移

目标：5 到 8 个月  
交付：

- Composer/suggestions。
- Fleet。
- Review/Verify。
- Web Settings。
- Screenshot/scrollback render。

### M6：Public Beta

目标：8 到 12 个月  
交付：

- next-core toggle。
- Windows beta default。
- compatibility matrix。
- fallback engine。
- release artifacts。

## 11. 验证体系

### 11.1 手动日测

每天至少跑：

1. 两个 Claude。
2. 一个 Codex。
3. 一个普通 PowerShell。
4. 连续 tab 切换。
5. 连续 PageUp/PageDown。
6. 右键粘贴长认证码。
7. ghost completion 右箭头接受。
8. 打开目录项目栏。
9. 点击任务栏最小化/恢复。
10. `unterm-cli instance list`。

### 11.2 自动 benchmark

必须持续维护：

- echo latency
- paste 10 KB
- output flood 100k lines
- scrollback paging 10k lines
- dual pseudo-agent output
- screen read during flood
- agent startup stall probe
- completion accept latency
- tab switch latency

### 11.3 兼容矩阵

Shell：

- PowerShell
- cmd
- WSL bash
- zsh
- fish
- nushell

TUI：

- vim / nvim
- less
- fzf
- git
- cargo
- npm / pnpm
- python / pytest

Agent：

- Claude Code
- Codex CLI
- Gemini CLI
- Aider
- OpenCode
- Kimi
- Trae

平台：

- Windows 11 优先
- macOS
- Linux

## 12. 风险

### 12.1 重写失控

风险：重写一年后仍不能替代当前版本。  
缓解：

- next-core 从最窄 MVP 开始。
- 阶段毕业由 benchmark 决定。
- current-core 持续可发布。

### 12.2 终端兼容性不足

风险：基础 shell 能跑，但复杂 TUI 不可用。  
缓解：

- 使用成熟 VT parser。
- 建兼容矩阵。
- 分阶段支持 CJK、emoji、ligature、mouse、alternate screen。

### 12.3 双内核维护成本高

风险：bug 要修两遍。  
缓解：

- 产品层共享。
- current-core 只修 P0/P1。
- next-core 只在通过毕业标准后扩大范围。

### 12.4 当前体验继续伤害品牌

风险：用户在 next-core 之前已经流失。  
缓解：

- Phase 0 优先解决卡死和输入问题。
- 不公开夸大 next-core。
- 每周发布小修。

### 12.5 字体和渲染复杂度

风险：自研渲染陷入字体细节。  
缓解：

- 第一阶段不追求 ligature 和复杂 shaping。
- font fallback 独立模块。
- CJK/emoji 单独验收。

## 13. 成功指标

体验指标：

- 输入 p95 < 16 ms。
- 两个 agent 输出时输入 p95 < 33 ms。
- UI stall p99 < 100 ms。
- paste 10 KB < 50 ms。
- PageUp/PageDown 无明显停顿。

稳定指标：

- freeze report 明显下降。
- paste failure 接近 0。
- instance lost 接近 0。
- Codex/agent startup stall 接近 0。
- MCP during flood 可响应。

产品指标：

- 用户能用 `setup-ai` 完成 agent 接入。
- 用户能同时监督多个 agent。
- Fleet 能跑通、验证、比较、合并。
- Recording 能稳定导出 markdown。
- Profile 不泄露 secret。

## 14. 近期立即行动

按顺序执行：

1. 固化当前未提交的 engine selector 测试和文档。
2. 重新跑 `cargo check -p unterm`、`cargo check -p unterm-engine --bins`。
3. 提交并推送当前 next-core selector 工作。
4. 用 standalone agent startup stall benchmark 固化 Codex 启动卡顿基线。
5. 排查 input/paste/completion 是否共用阻塞路径。
6. 排查 paint path 中 sidebar、agent、cwd、process scan。
7. 修 Rename tab 误触。
8. 修 Windows instance false deletion。
9. 建 engine dependency map。
10. 开始 next-core rendering spike 设计。

## 15. 决策点

### D1：是否全面重写？

结论：不做一次性全面重写，做受控重写。

原因：

- 终端兼容性很深，一次性重写风险过高。
- 现有产品已经有 MCP、Agent Cockpit、Fleet、Review 等差异化资产。
- 先抽产品层，才能避免做第二个 app。
- next-core 必须用 benchmark 赢，而不是靠感觉赢。

### D2：是否继续依赖 WezTerm？

短期：继续依赖，修 P0/P1。  
中期：产品层逐步脱离。  
长期：next-core 达标后默认切换，WezTerm 只保留 fallback。

### D3：是否追求 Warp 一样的体验？

目标是达到同等级的现代感和响应速度，但产品路线不同：

- Warp 是 AI 产品包进终端。
- Unterm 是终端暴露给外部 AI agent，并提供人类监督台。

所以 Unterm 应该学习 Warp 的流畅和视觉质量，不复制它的云 AI 形态。

## 16. 最终路线

最稳的路线是：

1. 先止血，让当前版本不再卡死。
2. 把产品层从 WezTerm 内部抽出来。
3. 做窄 next-core，用数据证明它更快。
4. next-core 通过后逐步迁移 Agent Cockpit、MCP、Profile、Recording、Fleet、Review。
5. 先 Windows beta，再跨平台默认。
6. 保留 fallback，一个大版本后再决定是否彻底移除 WezTerm。

这条路能同时保护当前用户、保住 Unterm 已有产品差异化，并给未来性能和外观留下足够空间。
