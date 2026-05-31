# Unterm MCP — client config snippets

Drop-in config for every major MCP-speaking client. All snippets invoke the
same bridge — `unterm-cli mcp-stdio` — which talks to the Unterm desktop app's
local TCP server on `127.0.0.1` over a Unix-domain stdio JSON-RPC pipe.

## Step 0 — install Unterm (required)

These snippets are **not** standalone npm packages. They invoke a binary
that ships with the Unterm desktop app. Install it first:

> **Download Unterm** → <https://unterm.app/api/download>
>
> Auto-detects your OS at the edge and redirects to the right artifact
> (macOS `.dmg`, Linux `.deb` + `.AppImage`, Windows `.msi` + `.zip`).
> MIT-licensed, $0, Apple-notarized on macOS, signed on Windows.

After install, **launch Unterm once** so its MCP server boots and writes
`~/.unterm/active.json` (port + auth token). The bridge reads that file
transparently — you don't have to wire credentials yourself.

## Step 1 — pick your client

| Client | File | Notes |
| --- | --- | --- |
| [Claude Desktop](./claude-desktop.json) | `~/Library/Application Support/Claude/claude_desktop_config.json` (mac) / `%APPDATA%\Claude\claude_desktop_config.json` (win) / `~/.config/Claude/claude_desktop_config.json` (linux) | Restart Claude after editing |
| [Claude Code](./claude-code.json)       | `.mcp.json` (project root) or `~/.claude/.mcp.json` (global) | Project scope picked up next session |
| [Cursor](./cursor.json)                 | `~/.cursor/mcp.json` or via Settings → MCP → Add | Settings → MCP → Reload to apply |
| [Codex CLI](./codex.toml)               | `~/.codex/config.toml` (append `[mcp_servers.unterm]`) | TOML, not JSON |
| [Gemini CLI](./gemini.json)             | `~/.gemini/settings.json` (merge `mcpServers`) | Preserve other keys |
| [OpenCode](./opencode.json)             | `opencode.json` (per-project) or `~/.config/opencode/opencode.json` | `mcp` (not `mcpServers`), `command` is an array, `type: "local"` |

## Skip the snippet entirely — Unterm auto-wires

If you launch your agent **from Unterm itself** — either via the GUI menu
(`Shell → AI Agents → <agent>`) or via the CLI (`unterm-cli agent launch
<id>`) — Unterm writes the right config block to the agent's native location
for you, with `preserve_unknown_keys` semantics. No copy-paste required.

Supported: Claude Code, Codex CLI, Gemini CLI, OpenCode. (Aider isn't an MCP
client so no wiring is needed.)

## Step 2 — discover the surface

Once connected, the cheapest way to learn what's available is to ask your
agent to call `meta.surface`. It returns the full live API — every method
with its parameter schema, every CLI subcommand, every currently-bound
keybinding — in one round-trip. No docs to read.

```jsonc
// example MCP request your agent can issue
{ "jsonrpc": "2.0", "id": 1, "method": "meta.surface" }
```

## Common methods to start with

| Method | Use case |
| --- | --- |
| `session.list` | enumerate panes in this window |
| `session.create` | spawn a new pane with a given cwd / command |
| `session.split` | carve the active pane left / right / up / down |
| `session.focus` | make a pane active so the user sees it |
| `session.input` | type into a pane, character by character |
| `screen.scrollback_text` | dump the full pane history (LLM-friendly: no OCR, parses natively) |
| `exec.run_wait` | run a command and block for its exit code |
| `capture.window` | screenshot a window and return file path |
| `upload.file` | push a local file to your OSS bucket, return public URL |
| `instance.list` | enumerate every live Unterm on this machine (multi-window) |
| `selftest.run` | built-in health check (every namespace probed) |

## Security defaults

- All servers bind to **`127.0.0.1` only** — no network exposure.
- **Bearer-token auth** in `~/.unterm/active.json` (chmod 600). Without it the
  TCP socket replies "auth required" to every method.
- Every `session.input` / `exec.send` is **logged with the calling agent's
  identity**. First write from a new agent triggers a blocking
  Allow / Block / Always-allow banner; trust persists to
  `~/.unterm/trusted_agents.json` and is one-click revocable in Web Settings.

## Trouble?

- "command not found: unterm-cli" → the Unterm app isn't installed yet, or
  the install didn't add it to PATH. macOS: open Unterm.app once (it
  registers the CLI symlink). Linux/Windows: `.deb` / `.msi` add it to PATH
  automatically; the AppImage doesn't — extract or symlink manually.
- Bridge connects but returns "auth required" → `~/.unterm/active.json` is
  missing or empty. Launch Unterm.app once to repopulate it.
- Multiple Unterm windows open, agent connects to the wrong one → call
  `instance.list` first and pin to a specific NATO-named instance.
