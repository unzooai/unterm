# Product Hunt 发布包

**时机**:太平洋时间 00:01 上架(北京 15:01/16:01),周二–周四。整天守评论。
**资产需求**:logo 240×240(有)、gallery 图 ≥3 张(建议:宣言区截图 / 终端+菜单截图 / 滚屏长截图演示 / Web 设置页)、可选 30s 视频。

- **Name**: Unterm
- **Tagline**(60 字符内): `The terminal AI agents can drive`
- **备选 tagline**: `Zero AI inside. Built to be driven by yours.`

**Description**:

```
Every terminal is racing to embed AI. Unterm makes the opposite bet: zero AI
inside — the terminal itself is an MCP server, so the agents you already use
(Claude Code, Codex, Gemini CLI) can drive it like a person: spawn panes,
run commands with structured output, read the screen and full scrollback,
take scrolling screenshots, record sessions to markdown, switch identity
profiles. Local-first (127.0.0.1 + token), no account, no telemetry.
MIT open source. macOS / Linux / Windows.
```

**First comment(maker comment,发布即贴)**:

```
Hi PH! Maker here.

I use Claude Code all day and kept hitting the same wall: my agent could
edit files, but it was blind to my actual terminal — the sessions, splits
and output history where my real state lives.

The industry answer is "put a chat box in the terminal." I think terminals
are thirty-year tools and AI models are six-month components — welding them
together means your longest-lived tool is defined by its shortest-lived
part. You already have great agents. They don't need a brain. They need
hands.

So: Unterm has NO AI inside at all. Instead it exposes 67 MCP methods —
exec, screen reads, scrolling screenshots (it re-renders your entire
scrollback into one tall PNG), session recording, identity profiles. On
install it auto-registers with every AI CLI on your machine.

Ask me anything — and tell me honestly: do you want AI *in* your terminal,
or *holding* it?
```
