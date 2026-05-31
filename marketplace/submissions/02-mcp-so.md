# mcp.so submission

mcp.so is a directory site; submission is a form at <https://mcp.so/submit>
(or via PR to their backing repo if they have one — check their footer).

## Fields to paste

| Field | Value |
| --- | --- |
| **Name** | `Unterm` |
| **Tagline** | `The terminal AI agents can drive — 67 MCP methods.` |
| **Repository** | `https://github.com/unzooai/unterm` |
| **Homepage** | `https://unterm.app` |
| **Logo URL** | `https://unterm.app/assets/icon-256.png` |
| **Screenshot URL** | `https://unterm.app/assets/og.png` |
| **License** | `MIT` |
| **Categories** | `Developer Tools`, `Terminal`, `Automation` |

## Long description (paste verbatim)

> **Unterm** is a cross-platform terminal (macOS / Linux / Windows) that
> exposes itself as a Model Context Protocol server. Any MCP-speaking
> client — Claude Desktop, Claude Code, Cursor, Codex CLI, Gemini CLI,
> OpenCode — gets **67 methods across 11 namespaces** to drive your
> terminal: spawn shells, split panes, type into them character-by-character,
> read pane scrollback as a single string, take screenshots, manage
> multi-instance sessions (NATO-named: alpha / bravo / charlie ...), and
> launch other AI coding agents (Claude Code, Codex, Gemini, OpenCode,
> Aider) one-click each — each pre-wired back to this terminal's MCP.
>
> **Local-first**: every server binds `127.0.0.1` only, is bearer-token
> gated, and every `session.input` / `exec.send` is logged with the
> calling agent's identity. First write from a new agent triggers a
> blocking Allow / Block / Always-allow banner; trust persists and is
> one-click revocable.
>
> Free, MIT, signed + Apple-notarized on macOS. No telemetry, no login,
> no paid tier.

## Install command (for the listing's "How to install" field)

```jsonc
// In your MCP client config (Claude Desktop, Cursor, etc.):
{
  "mcpServers": {
    "unterm": {
      "command": "unterm-cli",
      "args": ["mcp-stdio"]
    }
  }
}
```

**IMPORTANT prerequisite** (please make this prominent in the listing):

> Install the Unterm desktop app first: <https://unterm.app/api/download>
> (auto-detects your OS, redirects to the right artifact). Open Unterm
> once after install so the MCP server boots. Then paste the snippet above
> into your MCP client's config and restart it.

## Links to include in the listing

- Docs: <https://unterm.app/docs/mcp-reference>
- Per-client config snippets: <https://github.com/unzooai/unterm/tree/master/marketplace/mcp/configs>
- 3 ready-made Skills: <https://github.com/unzooai/unterm/tree/master/marketplace/skills>
