# Show HN — 发帖包

**时机**:美东周二–周四早上 8:00–10:00(北京时间 20:00–22:00)。避开周五/周末和美国节日。
**账号**:用你自己的 HN 账号,发帖后头两小时守在评论区——HN 的算法看早期互动,作者秒回是最强信号。
**规则**:标题不准用感叹号/营销词;正文诚实、第一人称、讲动机。Show HN 允许且鼓励"我做了什么"。

## Title(选一,≤80 字符)

1. `Show HN: Unterm – a terminal built to be driven by AI agents over MCP`
2. `Show HN: Unterm – I made my terminal an MCP server so Claude can use it like I do`
3. `Show HN: A terminal with no AI inside – it's the AI's hands instead`

推荐 1(关键词全:terminal / AI agents / MCP)。

## URL

`https://github.com/zhitongblog/unterm`(HN 偏爱 GitHub 直链胜过官网落地页)

## 正文(text 字段,纯文本)

```
I use Claude Code all day, and I kept hitting the same wall: the agent could
edit files and run commands in its own sandbox, but it couldn't see or drive
my actual terminal — the thing where my sessions, splits, env state and
output history actually live.

The 2026 terminals all answer this by embedding an AI chat into the terminal
(Warp being the biggest). I think that's backwards. Terminals are
thirty-year tools; AI models are six-month components. And I already have
agents I like — what they're missing isn't a brain, it's hands.

So I built Unterm: a cross-platform terminal (customized WezTerm engine,
Rust) where the terminal itself is the MCP server. Any agent — Claude Code,
Codex, Gemini CLI, your own script — gets the same local JSON-RPC surface:

- spawn tabs/panes, run commands, poll or block on output
- read the screen and the entire scrollback as text (no OCR)
- take screenshots, including a "scrolling screenshot" that re-renders the
  whole scrollback into one tall PNG headlessly (we own the text model, so
  it's an exact re-render, works even when the window is occluded)
- record sessions to markdown with secret redaction, manage identity
  profiles (per-window git/SSH/API credentials), drive proxy settings

Everything is 127.0.0.1 + auth token, no account, no cloud, no telemetry.
MIT. macOS / Linux / Windows. There's deliberately no AI inside the
terminal at all — no chat panel, no built-in copilot.

On install it auto-registers itself with the AI CLIs on your machine
(claude/codex/gemini/...) so the agent can discover it without config.

Happy to answer anything — especially curious whether others want their
agents *in* the terminal or *holding* it.
```

## 准备好的高频问答(评论区弹药)

- **"和 Warp 有什么区别?"** Warp puts the AI inside the terminal and routes through their cloud (login + subscription). Unterm has zero AI inside — it makes the terminal drivable by whatever agent you already run, locally. Opposite bet.
- **"为什么不直接用 tmux + send-keys?"** You can! But you get no structured output (exec.run_wait returns exit status + output as JSON), no screen/scrollback reads without scraping, no screenshots, no per-window identity, and the agent has to learn your tmux config. MCP gives every agent the same typed surface with discovery.
- **"安全吗?让 AI 控制终端?"** Default policy is everything local, token-gated, audit-logged; exec has a policy layer with blocked patterns; writes can be gated. You can also run it read-only.
- **"为什么 fork WezTerm 而不是插件?"** The surface needed (MCP server in the GUI process, pane-level PTY access, screenshot pipeline, identity profiles) goes far beyond what a plugin API exposes. Fork is honest about that. Upstream is credited heavily.
- **"开源协议?上游怎么办?"** MIT, same as WezTerm. Vendored, customized, documented divergence.
