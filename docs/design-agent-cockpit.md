# Agent Cockpit — 完整设计

> 状态：设计锁定 2026-07-10。目标版本 v0.55。
> 决策已与产品负责人确认：① hooks 随 setup-ai 自动配置；② Review 界面放 Web UI；③ Fleet worktree 用仓库旁挂目录。

## 0. 定位

Unterm 的定位是 "The terminal AI agents can drive"。Agent Cockpit 是这个定位的下半场：
agent 不仅能从外面**驱动**终端，终端也要**看见、聚合、编排**在它里面跑的所有 agent。

对标对象是 Warp 2.0 的 agentic 方向，而不是 Cursor：我们不做代码生成本身
（Claude Code / Codex / Gemini CLI / Aider 已经做得很好），我们做**运行、监督、
编排它们的最佳场所**。

不变的约束（沿用既有定位决策）：
- 本地优先，无云、无登录。
- 所有座舱能力必须同时暴露为 MCP 工具 + CLI 子命令（外部 agent 也能用座舱本身）。
- 三平台（macOS / Windows / Linux）同权。
- 深度 UI 放浏览器（Web Settings SPA），终端里只放轻交互（palette / chip / badge）。
- 终端内不做 AI chat 面板、不内嵌代码生成（scope guardrails 不变）。

## 1. 五大支柱

| # | 支柱 | 一句话 | 终端侧 UI | Web 侧 UI | MCP/CLI |
|---|------|--------|-----------|-----------|---------|
| P1 | Agent 状态引擎 | 每个 pane 里跑的是哪个 agent、处于什么状态 | tab 角标 + 顶栏 chip | Agents 页概览 | `agent.status` |
| P2 | Inbox | 所有"正在等你"的 agent 一个队列，一键跳达 | Inbox palette (Ctrl+Shift+A) | — | `cockpit.inbox` |
| P3 | Fleet | 一个任务 × N 个 worktree × N 个 agent 并行 | Fleet 启动 palette + Inbox 分组 | Review 页按 fleet 分组 | `fleet.*` |
| P4 | Review | agent 动工前自动 checkpoint，改动进 diff 审阅队列 | Inbox 内入口 | Review 页（diff + 回滚 + 合并） | `review.*` |
| P5 | 跨 agent 对比 | 同一任务发给不同 agent 并排比较 | Fleet palette 的 preset | Review 页并排 diff | `fleet.launch` 的 agents 参数 |

P5 不是独立系统：它是 P3 的一个预设（agents=[claude,codex,gemini]）+ P4 的分组视图。

## 2. P1 — Agent 状态引擎

### 2.1 数据模型

新文件 `wezterm-gui/src/cockpit/mod.rs` + `wezterm-gui/src/cockpit/status.rs`：

```rust
pub enum AgentKind { ClaudeCode, Codex, Gemini, Aider, Custom(String) }

pub enum AgentState {
    Working,        // agent 正在干活
    WaitingForUser, // 等确认/等输入 —— Inbox 的入队条件
    Idle,           // agent 起着但空闲（提示符下）
    Done,           // turn 完成（短暂态，N 秒后归 Idle）
}

pub struct PaneAgentStatus {
    pub pane_id: PaneId,
    pub kind: AgentKind,
    pub state: AgentState,
    pub since: Instant,          // 当前状态开始时间
    pub task_hint: String,       // 从标题提取的任务摘要
    pub last_signal: &'static str, // 最后驱动状态的信号名（调试/审计用）
    pub fleet_id: Option<String>,
}
```

全局注册表 `COCKPIT: Mutex<CockpitState>`（模式同 `mcp::handler` 的全局状态），
GUI 渲染、MCP handler、Inbox palette 都从这里读。

### 2.2 信号源（按可靠度分层）

**第 1 层 — OSC 解析（零配置，主信号）**
- **OSC 9;4 进度协议**：engine 已解析为 `PaneInformation.progress`。
  `9;4;3`（indeterminate）→ Working；`9;4;0`（clear）→ 离开 Working。
  Claude Code 对支持的终端发这个序列，给出精确 turn 边界。
- **OSC 0/2 标题语法**：per-pane title 缓存已存在，变更时跑解析器：
  - 标题首字符 ∈ U+2800–U+28FF（braille spinner）→ Working（Claude Code、Codex 通用）；
  - `✳ <摘要>` → Claude Code Idle，摘要进 task_hint；
  - `✋ Action Required` → Gemini WaitingForUser；`◇ Ready` → Gemini Idle；
    `⏲` / `✦` → Gemini Working；
  - Codex 标题回落为纯目录名 → Idle。
- **OSC 9 / OSC 777 文本通知**：在 toast-notification 消费点旁挂钩：
  Codex `approval-requested` → WaitingForUser、`agent-turn-complete` → Done；
  Gemini attention → WaitingForUser、complete → Done。
  注意拆 tmux passthrough（`ESC Ptmux;`）。
- **BEL**：pane 已有 bell 事件。有 agent 的 pane 收到 BEL 且不在 Working → WaitingForUser 弱信号。

**第 2 层 — 进程画像（判定"谁在跑"）**
- 前台进程（`get_foreground_process_info`，unix 有完整 argv）：
  `claude`（原生 bin）→ ClaudeCode；`node …codex` / 原生 `codex` → Codex；
  `node …gemini.js` → Gemini；`python …aider` → Aider。
- 轮询挂在已有的 2s 状态 tick 上（v0.54.4 修好的那个 timer），不加新定时器。
- 进程消失 → 状态清除（pane 回到普通 shell）。
- 检测表做成 `cockpit/detect.rs` 里的静态表，`Custom` 留给用户 lua 配置
  （`cockpit_agent_patterns`，可选，不在本版实现范围内也先留结构）。

**第 3 层 — 官方 hooks（setup-ai 自动配置，精度最高）**
- 新 MCP 方法 `agent.signal { pane_id?, event, agent, detail? }`，
  event ∈ working|waiting|done|idle。CLI：`unterm-cli agent signal`。
- pane 归属：engine 已给每个 pane 的 shell 注入 `WEZTERM_PANE`，
  hook 子进程天然继承，`unterm-cli agent signal` 自动读取
  （实现时修订：无需新增 `UNTERM_PANE_ID`，现成变量已覆盖）。
- setup-ai 顺手写入（首启总开关确认一次，沿用现有 setup-ai 的确认 UX）：
  - Claude Code `~/.claude/settings.json`：`Notification` + `Stop` hooks →
    `unterm-cli agent signal --claude --event …`；
  - Codex `~/.codex/config.toml`：`notify = ["unterm-cli","agent","signal","--codex"]`
    + `[tui] notifications = true`；
  - Gemini：不动配置（实现时修订：其动态标题已零配置编码全部四态，
    写配置纯属多余——减法原则）；
  - Aider `~/.aider.conf.yml`：`notifications: true` +
    `notifications-command: unterm-cli agent signal --aider --event waiting`。
  - 全部写入是**追加/合并**，绝不覆盖用户已有条目；写前备份 `<file>.unterm-bak`。
- 独立命令 `unterm-cli agent enable-hooks [--dry-run]` 供手动重配。

**第 4 层 — 屏幕文本启发式（兜底，弱信号）**
- 仅当 1/3 层无信号时使用：viewport 尾部匹配 `esc to interrupt`（Working）、
  行尾 `❯ / › / > ` 输入框（Idle）、aider `(Y)es/(N)o` 确认串（WaitingForUser）。
- 挂在 2s tick 的进程轮询之后，只读 viewport 最后 3 行，成本可忽略。

**状态机合并规则**：层号小的信号覆盖层号大的；同层后到覆盖先到；
Done 保持 8s 后自动降为 Idle；WaitingForUser 只能被"用户在该 pane 输入"
或更高层的 working/idle 信号清除（防止 Inbox 条目闪烁）。

### 2.3 终端侧 UI

- **Tab 角标**（fancy_tab_bar `item_to_elem`）：tab 标题前加一个色点：
  Working=蓝（呼吸动画复用 bell_start 模式）、WaitingForUser=橙、Done=绿、Idle=灰。
  无 agent 的 tab 不渲染点。多 pane 的 tab 取最高优先级状态
  （Waiting > Working > Done > Idle）。
- **顶栏 chip**（quick_button 模板，位置在 profile chip 左侧）：
  `⚡2 ✋1` —— N 个 working、M 个 waiting，跨**所有窗口**聚合（mux iter_windows）。
  M>0 时 chip 用警示色。点击 → 打开 Inbox palette。无 agent 时 chip 隐藏。
- 顶栏已有的 "⚡ agent" MCP 连接标记保持不变（那是"谁连着 MCP"，
  与"pane 里跑着谁"是两个维度）。

## 3. P2 — Inbox

### 3.1 交互

- 打开：`Ctrl+Shift+A`（CommandDef 注册 + `compute_default_actions` 列表，
  别再犯 v0.54.5 修的那个漏）+ 顶栏 chip 点击 + 命令面板。
- 形态：Modal palette（照 DirJump 模板：输入行 + 行列表 + 分页 + 鼠标/键盘统一光标）。
- 行内容：`[状态图标] agent名  task_hint  ·  tab标题  ·  相对时间`
  排序：WaitingForUser（按等待时长降序）→ Working → Done → Idle。
  fleet 成员行前缀 `⛵<fleet名>/`。
- Enter / 点击：跳到该 pane（激活 window → tab → pane，跨窗口用 `instance_focus` 同款路径）。
- `Tab` 键在选中行上：对 WaitingForUser 的 pane 直接跳达并聚焦输入
  （最常见动作是"过去按个 y"）。
- 顶部第二行常驻两个动作行（无 agent 等待时也在）：
  `⛵ Launch fleet…`（进 Fleet palette）、`⇄ Open review`（开浏览器 Review 页）。

### 3.2 MCP/CLI

- `cockpit.inbox` → `{ items: [PaneAgentStatus + tab/window 元数据] }`
- `agent.status { pane_id? }` → 单 pane 或全部状态
- CLI：`unterm-cli agent status/signal/inbox/enable-hooks`
  （实现时修订：inbox 收进 agent 命名空间而非顶级命令，保持 CLI 面紧凑）

## 4. P3 — Fleet

### 4.1 模型与持久化

```rust
pub struct Fleet {
    pub id: String,          // fleet_<slug>_<ts>
    pub task: String,        // 任务 prompt
    pub base_repo: PathBuf,  // 主仓库
    pub base_branch: String,
    pub members: Vec<FleetMember>,
    pub created_at: String,  // RFC3339
}
pub struct FleetMember {
    pub agent_cmd: String,   // "claude" / "codex" / "gemini" / 自定义
    pub worktree: PathBuf,   // ../<repo>.fleet/<slug>-<n>/
    pub branch: String,      // fleet/<slug>-<n>
    pub pane_id: Option<PaneId>,
    pub checkpoint: Option<String>, // 动工前快照 sha
}
```

持久化 `~/.unterm/fleets.json`（原子写，模式同 trusted_agents.json）。
pane 死亡不删 fleet（worktree 里的成果还在，review 完才清理）。

### 4.2 launch 流程（`fleet.launch`）

参数：`{ cwd, task, agents: ["claude","claude","codex"], name? }`
（agents 数组即成员列表，同名可重复；跨 agent 对比 = 传不同名字。）

1. 校验 cwd 是 git 仓库、工作区 clean（不 clean 则报错并提示，不静默 stash）。
2. 逐成员：`git worktree add -b fleet/<slug>-<n> ../<repo>.fleet/<slug>-<n> HEAD`。
3. 逐成员：记录 checkpoint（见 P4，worktree 刚建出来时 = HEAD，天然快照）。
4. 逐成员：spawn 新 tab（cwd=worktree）→ 向 shell 键入
   `<agent_cmd> '<task>'` 启动命令（agent 命令模板见 4.4）。
5. **每成员一个独立 tab**（实现时修订：原设计 ≤4 分屏 2×2，但 tab 角标
   是按 tab 聚合的——分屏在一个 tab 里只剩一个聚合角标，各成员状态不可见；
   独立 tab 让每个成员的 working/waiting 状态在 tab 栏一目了然）。
6. 注册进 fleets.json + COCKPIT 状态，返回 fleet 全量元数据。

### 4.3 终端侧 UI — Fleet palette

- 入口：Inbox 里的 `⛵ Launch fleet…` 行、命令面板 `Launch Agent Fleet`。
- 三步都在同一个 palette 内完成（无多级菜单）：
  1. 输入行 = 任务 prompt（多行粘贴折叠为一行显示，实发保留原文）；
  2. 行列表 = agent 选择器：`claude ×2`、`claude ×3`、`claude+codex 对比`、
     `claude+codex+gemini 三方对比`、以及检测到已安装的 agent 单选
     （已安装检测复用 unterm-agents 的 registry）；
  3. Enter 确认 → 调 `fleet.launch` → palette 关闭，跳到 fleet tab。
- 未安装的 agent 不出现在列表（别让用户选一个必然失败的项）。

### 4.4 agent 启动命令模板

内置表（`cockpit/detect.rs` 同处维护）：

| agent | 命令模板 |
|-------|---------|
| claude | `claude "<task>"` |
| codex | `codex "<task>"` |
| gemini | `gemini -i "<task>"` |
| aider | `aider --message "<task>"` |

task 引号转义按各 shell 规则处理（复用 exec 路径的既有转义）。

### 4.5 清理（`fleet.clean`）

- 条件：fleet 所有成员的 review 状态 ∈ {merged, discarded}（见 P4）。
- 动作：kill 成员 pane（若活着）→ `git worktree remove` → `git branch -D fleet/…`
  → fleets.json 删除条目。
- `unterm-cli fleet clean [--id <id>] [--force]`；`--force` 跳过 review 状态检查
  （但 worktree 有未合并改动时仍要求确认）。
- Review 页处理完最后一个成员时自动触发（等效 no-force 清理）。

## 5. P4 — Review（checkpoint + Web 审阅）

### 5.1 Checkpoint

两种来源：
- **Fleet 成员**：worktree 创建时的 HEAD 即 checkpoint，零成本。
- **散跑的 agent**（用户自己在某个 pane 里跑 claude）：状态引擎捕捉到
  `Idle→Working` 转换且 pane cwd 是 git 仓库时，做**非侵入快照**：
  临时 index（`GIT_INDEX_FILE=<tmp>`）→ `git add -A` → `git write-tree` →
  `git commit-tree`（dangling commit，不动 HEAD、不动工作区、不动用户 index）。
  sha 记入 `~/.unterm/checkpoints.json`（按 repo 根路径分组，每 repo 保留最近 20 个）。
  同一 Working 期间只拍一次；距上个 checkpoint <60s 也跳过（防抖）。

### 5.2 Web Review 页

挂在现有 Web Settings SPA（127.0.0.1:<http_port>，同 auth_token）新增 "Review" 标签页。

HTTP API（同现有 settings API 的注册方式）：
- `GET /api/review/overview` → fleet 列表 + 散 checkpoint 列表（repo 分组）
- `GET /api/review/diff?repo=<path>&from=<sha>` → 文件级 numstat +
  逐文件 unified diff（服务端跑 `git diff <sha>`，含 untracked：
  对 untracked 文件用 `git diff --no-index /dev/null <file>` 合成）
- `POST /api/review/rollback { repo, sha }` → 回滚工作区到快照：
  `git read-tree <sha> && git checkout-index -af && git clean -fd`
  （页面上红色按钮 + 明确文案"将丢弃 X 个文件的改动"+ 二次确认）
- `POST /api/review/merge { fleet_id, member }` → 在主仓库
  `git merge --squash fleet/<slug>-<n>`（worktree 共享 object 库，直接可见），
  **停在 staged 不自动 commit**——commit 权留给用户。成员标记 merged。
- `POST /api/review/discard { fleet_id, member }` → 成员标记 discarded。
- 全部 POST 走审计日志（session.audit_log 同款）。

页面结构（Tailwind + Alpine，随现有 SPA 风格）：
- 左栏：fleet 分组（成员卡片：agent 名、状态、+N/-M 行数、耗时）+ 散 checkpoint 列表。
- 右栏：选中成员/checkpoint 的文件树 + 行级 diff（服务端产 unified diff，
  前端解析着色，不引第三方 diff 库——手写 ~100 行解析器够用）。
- Fleet 成员支持并排两列 diff（P5 的对比视图：同一文件在两个成员间的产出并排）。
- 动作按钮：Merge（squash 到主仓库暂存）/ Discard / Rollback（散 checkpoint）。

### 5.3 MCP/CLI

- `review.list` / `review.diff { repo, from }` / `review.rollback { repo, sha }` /
  `review.merge { fleet_id, member }` / `review.discard { fleet_id, member }`
- rollback / merge 属写操作：走 `gate_pty_write` 同级的确认与审计（policy 适用）。
- CLI：`unterm-cli review list|diff|rollback|merge|discard`、
  `unterm-cli review open`（开浏览器直达 Review 页）。

## 6. 配置

新增 lua 配置（全部有默认值，零配置可用）：
- `cockpit_enabled = true` — 总开关（false 时不轮询、不渲染、MCP 返回 disabled）。
- `cockpit_auto_checkpoint = true` — 散 agent 的自动快照开关。
- `cockpit_done_hold_secs = 8` — Done 态保持时长。

不加的：状态颜色、图标、轮询间隔——没有证据表明有人需要改（减法原则）。

## 7. i18n

所有新 UI 字符串走 `t()`，9 个 locale 一次配齐（en 为准）。
新增 key 前缀 `cockpit.`（如 `cockpit.inbox_title`、`cockpit.launch_fleet`）。
Web Review 页复用 SPA 现有 i18n 机制。

## 8. 跨平台注意

- 进程指纹：Windows 用 `QueryFullProcessImageNameW` 的 exe 路径匹配
  （`claude.exe`、`codex.exe`、`node.exe` + 命令行）；WSL 内 agent 由
  标题/OSC 信号覆盖（进程画像看不见 WSL 内部，属已知降级）。
- worktree：`../<repo>.fleet/` 与仓库同盘，避开 Windows 跨盘 worktree 坑。
- OSC 777 / tmux passthrough 解析在 escape-parser 层做，平台无关。
- `unterm-cli agent signal` 的 pane 归属：三平台都靠 `UNTERM_PANE_ID` env。

## 9. 自测计划（按 rule #2，全部走自家 MCP/CLI + 浏览器 MCP）

1. 单测：标题解析器（braille/✳/✋/◇/⏲ 各 agent 样本）、状态机合并规则、
   fleet slug 生成、diff API 的 untracked 合成。
2. 集成（脚本化假 agent）：`tests/fake_agent.sh` 依次发
   标题 OSC → OSC 9;4 → OSC 9 通知 → BEL，在真实 pane 里跑，
   用 `unterm-cli agent status --json` 断言四态迁移。
3. Inbox：假 agent 置 waiting → 截图断言 chip 计数与 palette 行；
   Enter 跳转后断言 active pane。
4. Fleet：临时 git 仓库 → `unterm-cli fleet launch --agents fake,fake` →
   断言 worktree/branch/pane/fleets.json；成员里改文件 →
   Review API 断言 diff → merge → 主仓库 staged 断言 → clean 断言 worktree 消失。
5. Web Review 页：unzoo browser MCP 打开页面，a11y snapshot 断言结构，
   截图归档；rollback 走假仓库全流程。
6. 回归：现有 selftest_run 全绿；tab bar / 顶栏截图对比无错位。

## 10. 明确不做（本版）

- 不做 agent 输出的语义解析（token 数、成本统计）——信号面不稳定。
- 不做跨机器 fleet（远端 SSH domain 的 worktree 语义太复杂）。
- 不做 Review 页的行内编辑——看、回滚、合并，不改。
- 不做自研 agent（`unterm-agent`）——座舱验证后再议。
- 不碰 scope guardrails 划出的禁区（AI chat 面板等）。
