# X/Twitter 线程包

**时机**:配合 Show HN 同日,互相导流。英文为主(MCP 圈在英文区)。
**标签/@**:#MCP #ClaudeCode;可 @AnthropicAI 的开发者关系相关账号(别 @ 主号,显 spam)。
**发线程的可靠方法**(2026-06-11 实战验证):`x.com/intent/post?text=...&in_reply_to=<上一条id>`,
每条点 tweetButton,从 `/untermapp/with_replies` 抓新 id。**不要用时间线内联 composer**(布局位移,
曾把文字误打进别人推文的回复框)。X 拒绝完全相同的重复文本——补发时微调措辞。

## 历史:v0.40 线程(2026-06-11 已发,@untermapp)

根推 x.com/untermapp/status/2064946328980091165,T1–T6 已确认,T7 未肉眼确认(下次核对,缺则补发)。
旧文案见 git 历史(数字口径:67 methods / 11 namespaces,已过时,勿再用)。

## v0.57 新线程(Agent Cockpit 角度,8 条,待发)

1/
Every terminal in 2026 is racing to put AI *inside* the box.
We made the opposite bet: a terminal with zero AI inside — built to be *driven* by the agents you already run.
Unterm v0.57 is out. MIT, local-first, macOS/Linux/Windows. 🧵

2/
The reasoning hasn't changed: terminals are thirty-year tools, AI models are six-month components.
Weld the model into the terminal and your longest-lived tool is defined by its shortest-lived part.
You already HAVE agents. What they're missing isn't a brain — it's hands.

3/
So the terminal itself is an MCP server. 99 methods, 21 namespaces:
spawn panes → run commands with structured results → read screen & full scrollback as text → scrolling screenshots → record sessions → identity profiles. All local JSON-RPC, token-gated.

4/
New since launch: Agent Cockpit. Run a *fleet* of CLI agents — each one gets its own tab and its own git worktree, so five Claudes can hack on the same repo without stepping on each other.

5/
The tab strip tells you who's working, who's idle, and who's *waiting for you* — parsed straight from each agent's title/OSC signals. Zero config. One palette shows the inbox: "agents that need a human."

6/
And a review layer that makes agent work reversible: checkpoint before the run, diff after, roll back without touching your index or HEAD. Agent went sideways? One command, you're back.

7/
My favorite party trick is still scrolling screenshots: the agent re-renders your ENTIRE scrollback into one tall PNG — exact re-render with your fonts & theme, works even when the window is buried.

8/
No account. No cloud. No subscription. No telemetry. No AI inside the terminal — ever.
https://unterm.app
https://github.com/zhitongblog/unterm

## 中文版(3 条)

1/ 2026 年所有终端都在往里塞 AI。我们反着做:内部零 AI 的终端,专门被 AI 驱动。v0.57 起它还是 CLI agent 的座舱:一队 agent 各占一个 tab + 独立 git worktree,五个 Claude 改同一个仓库互不踩脚。

2/ tab 栏直接显示每个 agent 在干活/闲着/等你确认(解析 agent 自己的标题和 OSC 信号,零配置);一个面板汇总"等人类的 agent";checkpoint/review 层让 agent 的改动可回滚——跑偏了一键回到跑之前。

3/ 终端本身是 MCP server(99 方法/21 命名空间):开分屏、跑命令拿结构化结果、读完整回滚、滚屏长截图、录会话、身份 profile。全本地无账号无遥测,MIT,mac/Linux/Windows。unterm.app
