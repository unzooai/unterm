# X/Twitter 线程包

**时机**:配合 Show HN 同日,互相导流。英文为主(MCP 圈在英文区)。
**标签/@**:#MCP #ClaudeCode;可 @AnthropicAI 的开发者关系相关账号(别 @ 主号,显 spam)。

## 英文线程(7 条)

1/
Every terminal in 2026 is racing to put AI *inside* the box.
We made the opposite bet: a terminal with zero AI inside — built to be *driven* by AI.
Meet Unterm. MIT, local-first, macOS/Linux/Windows. 🧵

2/
The reasoning: terminals are thirty-year tools. AI models are six-month components.
Weld the model into the terminal and your longest-lived tool is defined by its shortest-lived part.

3/
And you already HAVE agents — Claude Code, Codex, Gemini CLI. They get better every month.
What they're missing isn't a brain. It's hands.

4/
So the terminal itself is an MCP server. 67 methods, 11 namespaces:
spawn panes → run commands → read screen & scrollback as text → screenshot → record sessions → switch identity profiles. All local JSON-RPC, token-gated.

5/
My favorite bit: scrolling screenshots. The agent can render your ENTIRE scrollback
into one tall PNG — exact re-render with your fonts & theme, works even when the
window is buried. And it can long-shot OTHER apps' windows by scroll-stitching.

6/
Install it and it auto-registers into the MCP configs of Claude Code / Codex /
Gemini / OpenCode / Aider on your machine. The agent just... finds its hands.

7/
No account. No cloud. No subscription. No telemetry. MIT.
https://unterm.app
https://github.com/zhitongblog/unterm

## 中文版(微博/X 中文圈,3 条)

1/ 2026 年所有终端都在往里塞 AI。我们反着做:一个内部零 AI 的终端,专门被 AI 驱动。终端是三十年的工具,模型是六个月的组件——焊死就是寿命错配。你已经有 Claude Code 了,它缺的是手,不是脑。

2/ Unterm 把终端做成 MCP server:agent 能开分屏、跑命令拿结构化结果、读完整回滚、滚屏长截图(整个历史渲染成一张长图)、录会话。装好自动注册进本机所有 AI CLI 的配置,零设置。

3/ 全本地 127.0.0.1,无账号无订阅无遥测,MIT 开源,mac/Linux/Windows 全平台。unterm.app
