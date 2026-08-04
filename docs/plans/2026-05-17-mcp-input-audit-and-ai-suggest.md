# MCP 输入审计 + AI Suggest 面板

**日期**：2026-05-17
**负责人**：xuyao
**目标版本**：v0.16
**触发事件**：发现 MCP `session.input` 可在用户无感知下往 PTY 写键盘输入，且无审计；这同时也是 Unterm 区别于普通终端的核心差异化（AI 驱动终端）。

---

## 背景与威胁模型

### 现状
- `wezterm-gui/src/mcp/handler.rs:652-661` 的 `session_input` 直接 `pane.writer().write_all(...)`，等同手敲
- **未调用 `self.audit()`** → audit_log 查不到任何输入注入痕迹
- `exec.send` 是 `session.input` 的别名，同样未审计
- 用户在 Claude Code 输入框看到 "把 12 语言也翻了吧" 但没敲过——经分析极可能是 MCP 客户端注入，但当前架构无法溯源

### 威胁
1. **静默注入**：任何拿到 auth_token 的本地进程（包括 Claude Code MCP 客户端）能在用户无感知下塞任意按键
2. **混淆视听**：注入的字符与用户手敲完全无法区分（无视觉标记）
3. **审计缺失**：事后无法追查"这条命令是谁打的"

### 产品机会
`session.input` 的能力本身是 Unterm 的**核心竞争力**——别的终端没有"让 AI 驱动 shell"这种一等公民接口。但要做大需要把不可见的副作用**变成产品化的、可见的、可控的**协作模式。

---

## 总体设计

将"AI 写入终端"分成两条路径：

| 路径 | 接口 | 目标 | 可见性 |
|------|------|------|--------|
| **直接注入** | `session.input` | 给信任 agent 的脚本化场景（CI、自动化） | 全程审计 + UI 视觉标记 + 可选用户确认 |
| **建议注入** | `session.suggest`（新增） | 给 AI 助手（"建议下一步") | UI 上 Tab/Accept 才入栈，永远不直接进 PTY |

加上**审计可视化**让一切可查。

---

## P0：扎紧口袋（v0.16-rc1）

> 目标：当前架构下让 `session.input` 不再是"看不见的后门"。所有现有功能不破坏。

### P0.1 给 `session.input` 加审计
**文件**：`wezterm-gui/src/mcp/handler.rs`
**改动**：
- `session_input` 在 `write_all` 之前调 `self.audit("session.input", Some(&pane.pane_id().to_string()), &preview)`
- `preview`：把 input 转 escape-aware 摘要（前 80 字符 + 总长度 + 是否含控制字符）。不记完整内容（防止密码泄到 log）但保证可识别 "把 12 语言也翻了吧" 这类纯文本
- 同步加到 `exec.send` 别名调用前
- 现有 `AuditEntry` 加 `client_peer` 字段（从 server.rs 传 peer_addr 进来，需要小改 handler 签名 / 改用 thread-local）

**验收**：调用 `session.input` 后立即 `session.audit_log` 能查到一条带预览 + 时间戳的记录

### P0.2 标记 MCP 来源的字符串
**文件**：
- `mux/src/pane.rs` — pane trait 加 `writer_with_origin(origin: InputOrigin)` 或在 LocalPane 内部记录"上一次写入来源"
- `wezterm-gui/src/mcp/handler.rs` — 调用上述带 origin 的接口
- `wezterm-gui/src/termwindow/render/paint_pane.rs` — 渲染时如有 MCP-origin 标记，在该范围下加底纹/下划线

**改动**：
- 引入 `InputOrigin::User | InputOrigin::Mcp { method, peer }` enum 放在 `mux/src/lib.rs`
- 由于 PTY 是字节流，"标记字节范围"无法在 PTY 层做。改用**事件标记法**：在 LocalPane 里维护一个最近 N 条 MCP-input 记录的环形缓冲（时间戳 + 长度），渲染时拿当前 cursor 行号反查，命中范围加视觉标记
- 渲染细节：使用 `attr.set_underline_color(Color::rgb(80,140,255))` + 细虚线下划线

**验收**：调一次 `session.input` 注入 "hello"，输入框里 "hello" 显示蓝色虚线下划线；用户手敲的 "world" 不带下划线

### P0.3 默认配置加确认门（opt-in）
**文件**：`config/src/config.rs`、`wezterm-gui/src/mcp/handler.rs`
**改动**：
- 新增 config：`mcp_input_confirmation: McpInputConfirmation`，枚举 `Always | FirstTimePerSession | Never`，默认 `FirstTimePerSession`
- 触发时：`session_input` 阻塞等待主线程 GUI 弹一个**非模态**横幅"MCP client wants to send '<preview>' to this pane — [Allow] [Block] [Always allow this session]"
- 拒绝/超时（默认 10s）→ 返回 `-32004 user_denied` 给 MCP 客户端

**验收**：首次注入时 GUI 出现横幅；点 "Always allow this session" 后同 session 再注入直接通过

---

## P1：产品化（v0.16）

### P1.1 新增 `session.suggest` —— 永不直接写 PTY
**文件**：`wezterm-gui/src/mcp/handler.rs`、`mux/src/pane.rs`、新建 `wezterm-gui/src/suggest/mod.rs`

**协议**：
```jsonc
// Request
{"method": "session.suggest", "params": {
  "id": 0,
  "text": "git rebase -i HEAD~3",
  "rationale": "上一条 commit 信息写错了，建议改用 rebase 修正",  // optional
  "ttl_ms": 30000,                                                  // optional, default 60s
  "source": {"agent": "claude-code", "session": "abc123"}           // optional, audit
}}

// Response
{"result": {"suggestion_id": "sg_01HXYZ...", "status": "queued"}}
```

**行为**：
- 不写 PTY
- 在 pane 底部"建议条"显示文本 + rationale（按 `?` 展开）
- 用户按 `Tab` 接受 → 字符进入 PTY（带 `InputOrigin::McpAccepted`，渲染区分）
- 用户按 `Esc` 拒绝 → 建议消失，回 `session.suggest_status` 报告 dismissed
- 用户按 `Alt+Enter` 接受并立即回车
- TTL 超时自动消失

**附加方法**：
- `session.suggest_status(suggestion_id)` — 查这条建议被接受/拒绝/超时
- `session.suggest_cancel(suggestion_id)` — agent 撤回建议
- `session.suggest_list(pane_id)` — 列当前 pane 待决建议

### P1.2 Suggest UI 条
**文件**：新建 `wezterm-gui/src/suggest/render.rs`，挂到 `termwindow/render/`
**位置**：状态栏正上方，单行高度，半透明背景

```
┌─────────────────────────────────────────────────────────────────┐
│ ✨ Claude: git rebase -i HEAD~3                              ?  │
│    [Tab] accept   [Esc] dismiss   [Alt+Enter] accept & run      │
└─────────────────────────────────────────────────────────────────┘
```

- 按 `?` 展开 rationale 到 popover
- 多条建议时显示 `1/3` 翻页
- 关键：**hover 时不偷焦点**，鼠标点接受按钮再接受

### P1.3 审计日志 Overlay
**文件**：新建 `wezterm-gui/src/overlay/audit_log.rs`，参照 `overlay/copy.rs` 结构
**入口**：默认 keybinding `Ctrl+Shift+A`（可配）
**显示**：
- 倒序列出 `audit_log` 全部条目
- 列：时间、method、pane id、peer、内容预览
- 支持 `/` 过滤
- `Enter` 跳到该 pane（如果还活着）

### P1.4 配置项整理
**文件**：`config/src/config.rs`
新增结构：
```rust
pub struct McpConfig {
    pub enabled: bool,
    pub allow_session_input: bool,           // 默认 false — 强制走 suggest
    pub input_confirmation: McpInputConfirmation,
    pub suggest_keybinds: SuggestKeybinds,   // accept / dismiss / accept_run
    pub audit_log_capacity: usize,           // 默认 1000
    pub trusted_peers: Vec<TrustedPeer>,     // 白名单（按 process 路径 / 自签名）
}
```

**默认行为（已决策 2026-05-17：保持开启 + 强审计）**：
- `allow_session_input` 默认 `true`，**不**强制 agent 走 `session.suggest`
- 但每次调用必须经过：审计 → 视觉标记 → 首次每 agent 弹一次确认横幅
- 文档同步：`web/src/pages/docs/agent-integration.md` 增加 `session.suggest` 推荐章节，但不弃用 `session.input`

---

## P2：多 Agent 协作（v0.17 +）

### P2.1 跨 pane suggest
- `session.suggest` 的 `target` 可指向**另一个 pane**
- 渲染时该 pane 的建议条要显示来源 pane（"from pane 3"）

### P2.2 Agent 身份与权限
**新建**：`wezterm-gui/src/agent/registry.rs`
- 每个 MCP 连接首次 auth 后可选 `agent.identify` 自报身份（名字、capabilities）
- 配置文件支持 per-agent 权限：
  ```toml
  [[agent_policy]]
  agent = "claude-code"
  allow = ["session.suggest", "screen.read", "session.list"]
  deny = ["session.input", "signal.send"]
  ```
- 不识别的连接退化到"匿名"权限组

### P2.3 协作可视化
- Title bar 显示当前活跃 agent 列表 + 各自最近活动
- `Ctrl+Shift+G` 打开"Agent Dashboard" overlay，查看所有连接、最近调用、暂停按钮

---

## 实施顺序与拆分

| 阶段 | 范围 | 估计 | 单独可发布？ |
|------|------|------|--------------|
| P0.1 | audit 补全 | 30 分钟 | ✅ 单独 patch 发 v0.15.1 |
| P0.2 | 视觉标记 | 2 小时 | ✅ |
| P0.3 | 确认门 | 2 小时 | ✅ |
| P1.1 + P1.2 | suggest API + UI | 4 小时 | ✅ 必须一起 |
| P1.3 | 审计 overlay | 2 小时 | ✅ |
| P1.4 | 配置整理 + 文档 | 1 小时 | 必须和 P1.1 同发 |
| P2.* | 多 agent | 单独 milestone | ❌ 跨多版本 |

**建议**：P0 一气呵成发 v0.15.1 安全补丁；P1 整段做 v0.16 发布；P2 起独立 milestone。

---

## 验收清单

### P0
- [ ] `session.input` 调用后 `session.audit_log` 必能查到
- [ ] MCP 注入的字符显示蓝色虚线下划线，用户手敲的不显示
- [ ] 首次 MCP 注入弹横幅，用户能 allow/block/always-allow
- [ ] 既有依赖 `session.input` 的测试不破坏（unterm-cli 自检）

### P1
- [ ] 调 `session.suggest` 出现建议条而不是 PTY 写入
- [ ] Tab 接受、Esc 拒绝、Alt+Enter 接受并 run 三个 keybind 都生效
- [ ] `session.suggest_status` 能查到生命周期
- [ ] `Ctrl+Shift+A` 打开审计 overlay，能看到所有历史调用
- [ ] `allow_session_input=false` 时 `session.input` 返回 `-32005 disabled_by_policy`

### 回归
- [ ] Claude Code 仍可正常在 Unterm 里跑（auth + 终端渲染零变化）
- [ ] 现有 MCP 客户端（unterm-cli）所有命令仍可用
- [ ] WezTerm 原生功能（分屏、Tab、复制粘贴）无回归

---

## 风险与开放问题

1. **InputOrigin 实现**：PTY 字节流没有"染色"概念，用事件时间戳 + 行号反查是近似的——快速连续注入可能渲染时漂移。可接受，因为这是辅助提示而非安全机制（安全靠审计）。
2. **暂停**：`mcp_input_confirmation = Always` + agent 频繁注入 → 用户疲劳。靠 `FirstTimePerSession` 默认 + per-pane "always allow" 缓解。
3. **跨平台**：横幅 UI 在 macOS/Linux/Windows 三端实现一致性。复用现有 `wezterm-gui/src/overlay/` 设施。
4. **`session.input` deprecation**：默认禁用是破坏性变更。要在 release note 写清楚 + 给 6 个月过渡期。

---

## 不在本计划范围

- AI Ghost Text（输入补全建议）—— 这是 Phase 3 单独大功能
- Insights 面板 / Error Fixer —— 同上
- 远程 agent（非本机连接）—— 当前仍只允许 127.0.0.1

