# MCP 目录提交包(最高杠杆渠道)

在这些目录里,用户**主动搜索**"能让 agent 干 X 的 server"。Unterm 是极少数
"真实桌面 app 本体 = MCP server"的条目,品类几乎无竞争。

## 通用字段(各家表单直接粘)

- **Name**: Unterm
- **一句话**: The terminal AI agents can drive — a real desktop terminal (macOS/Linux/Windows) that runs a local MCP server: spawn panes, run commands, read screens, take scrolling screenshots, record sessions.
- **Categories**: Developer Tools / Terminal / Desktop Automation
- **Repo**: https://github.com/zhitongblog/unterm
- **Site**: https://unterm.app
- **License**: MIT
- **Transport**: TCP (line-delimited JSON-RPC) via `unterm-cli mcp-stdio` bridge → stdio
- **客户端配置 JSON**(stdio bridge):

```json
{
  "mcpServers": {
    "unterm": {
      "command": "unterm-cli",
      "args": ["mcp-stdio"]
    }
  }
}
```

(注:GUI 安装后 `setup-ai` 会自动写好这份配置,人工提交目录时照贴即可。)

## 提交清单

| 目录 | 方式 | 状态 |
|---|---|---|
| punkpeye/awesome-mcp-servers (GitHub ~60k star) | PR,一行条目 | 待发 PR |
| mcp.so | 网页表单提交 | 待提交 |
| PulseMCP | 表单/自动爬 GitHub topic `mcp-server`(topic 已有 ✓) | 待确认收录 |
| Glama.ai/mcp | GitHub App / 自动收录,可主动 claim | 待 claim |
| Smithery | smithery.yaml + 注册 | 评估(偏 stdio 托管,桌面 app 模式特殊) |
| mcpservers.org | PR | 待发 PR |
| Anthropic 官方 modelcontextprotocol/servers README "Community" 区 | PR(门槛高,审得严) | 准备好再发 |

## awesome-mcp-servers PR 条目(直接可用)

分类放 `🖥️ <a name="command-line">Command Line</a>` 或 OS Automation:

```markdown
- [zhitongblog/unterm](https://github.com/zhitongblog/unterm) 🦀 🏠 🍎 🪟 🐧 - A full desktop terminal that is itself an MCP server: spawn tabs/panes, run commands with structured output, read screens and scrollback, take (scrolling) screenshots, record sessions to markdown, manage identity profiles.
```

(图例:🦀 Rust、🏠 local、🍎🪟🐧 平台 —— 按该仓库 legend 核对后再发。)
