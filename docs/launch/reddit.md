# Reddit 发帖包

**规则**:每个 sub 间隔至少 1–2 天,别同日轰炸(会被标 spam)。先用账号在目标 sub 正常评论几天攒 karma 更稳。
**时机**:美东工作日早上。

---

## r/ClaudeAI(最对口,Claude Code 用户聚集地)

**Title**: `I built a terminal that Claude Code can fully drive over MCP — spawn panes, read the screen, take scrolling screenshots`

**Body**:

```
Claude Code is great at editing files, but it's blind to my actual terminal —
the sessions, splits, and output history where my real work state lives.

So I built Unterm: a cross-platform terminal with its own native Rust
kernel and a local MCP server. On first launch it auto-registers
itself into Claude Code's MCP config, so Claude can immediately:

- list/spawn/focus tabs and panes
- run commands and block on structured output (exit status + text, no scraping)
- read any pane's screen or full scrollback as text
- screenshot the window, or render the ENTIRE scrollback into one tall PNG
- record sessions to markdown (with token/secret redaction)

There is deliberately NO AI inside the terminal — no chat box, no copilot.
The whole bet is: you already have the best agent; it just needs hands.

Local-only (127.0.0.1 + auth token), no account, MIT, macOS/Linux/Windows.

GitHub: https://github.com/zhitongblog/unterm
Site: https://unterm.app

Would love feedback from people who drive Claude Code hard all day.
```

---

## r/mcp(小但精准)

**Title**: `Unterm: a desktop terminal that IS an MCP server — 103 authenticated methods (exec, screen, capture, session, profile…)`

**Body**(短版):

```
Most MCP servers wrap an API. This one wraps a real desktop app: the
terminal itself. Any MCP client can spawn shells, run commands with
structured results, read screens/scrollback, take (scrolling) screenshots,
record sessions, switch identity profiles.

Auto-discovery: on install it registers itself into the global MCP configs
of Claude Code / Codex / Gemini CLI / OpenCode / Aider.

MIT, local-first, macOS/Linux/Windows. Native Rust terminal kernel.

https://github.com/zhitongblog/unterm
```

---

## r/commandline

**Title**: `Unterm — a cross-platform terminal where every feature has a CLI and an API (and your AI agent can drive it)`

角度调整:这个 sub 反感 AI 营销,主打 **CLI-first/可脚本化**,AI 只是顺带提。

```
Every product feature ships with a CLI subcommand and a JSON-RPC method on
day one: screenshots (incl. rendering the whole scrollback to one tall PNG),
session recording → markdown, identity profiles, proxy management, theme
switching. `unterm-cli reference` prints the full surface.

It happens to make the terminal drivable by AI agents over MCP, but
everything works equally well from a shell script or cron job.

MIT, native Rust terminal kernel, macOS/Linux/Windows.
https://github.com/zhitongblog/unterm
```
