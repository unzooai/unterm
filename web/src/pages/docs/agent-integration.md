---
layout: ../../layouts/Doc.astro
title: Driving Unterm from outside
subtitle: How to wire up Claude Code, Cursor, Aider, or your own scripts to control an Unterm window over MCP — the pattern Unterm was designed around.
kicker: Docs / Agent integration
---

## The mental model

Most terminals embed their AI _inside_ the terminal — Warp's AI lives in a closed cloud orchestrator, ChatGPT desktop opens its own panel, etc. Unterm picks the opposite end: keep AI completely outside the binary, expose the terminal itself as a surface that _any_ external agent can grip via MCP. The terminal is the hand. Whatever's holding it is up to you.

Concretely, every Unterm window opens two local servers on launch:

- **MCP server** on `127.0.0.1:<auto-port>` — line-delimited JSON-RPC over TCP, auth-token gated, exposes every product operation (spawn, exec, read pane, screenshot, recording, settings).
- **HTTP settings server** on `127.0.0.1:<auto-port>` — REST endpoints for the same surface, plus a Tailwind+Alpine SPA at the root for human-driven config.

Both ports plus the auth token are written to `~/.unterm/server.json` on launch. That file is the canonical handshake — every external tool reads it to figure out where to connect.

## Quick start: connecting Claude Code

Claude Code is the canonical example because it ships native MCP support. Two minutes of setup:

1. Install Unterm and launch it once. Verify `~/.unterm/server.json` appears.
2. Add Unterm as an MCP server to your Claude Code config (typically `~/.claude/mcp.json`):

```json
{
  "servers": {
    "unterm": {
      "type": "tcp",
      "host": "127.0.0.1",
      "port_from_file": "~/.unterm/server.json:mcp_port",
      "auth_token_from_file": "~/.unterm/server.json:auth_token"
    }
  }
}
```

3. Restart Claude Code. Type `/mcp` and you should see `unterm` listed as a connected server with its tool list.

Cursor and Aider follow the same pattern — point an MCP client at the local socket described in `server.json`.

## Pattern 1 — Director and worker

The killer use case for an MCP-controllable terminal: an _outer_ agent supervises an _inner_ agent running inside an Unterm pane. The outer agent dispatches concrete work, the inner agent does the actual coding, the outer one watches progress and steps in when it stalls.

Concretely, in your outer Claude Code session:

```js
// 1. spawn a fresh pane in the project dir
mcp.unterm.session.spawn({
  cwd: "/Volumes/Dev/code/some-project",
  prog: ["claude", "code"]
})

// 2. wait for the inner agent to be ready (look for prompt marker)
await mcp.unterm.session.wait_for(pane_id, "▶▶ bypass permissions on")

// 3. dispatch a task
mcp.unterm.session.send_text(pane_id,
  "implement the feature described in TODO.md, run the tests, commit when green\n")

// 4. poll output every 30s, intervene if stuck
while (!await mcp.unterm.session.contains(pane_id, "all green")) {
  const tail = await mcp.unterm.session.read_tail(pane_id, 200)
  if (looksStuck(tail)) {
    mcp.unterm.session.send_text(pane_id, "you appear stuck — try X\n")
  }
  await sleep(30_000)
}
```

The outer agent has the strategic picture (which task next, when to escalate, when to switch from coding to commit prep), the inner agent has the keyboard. Both can be Claude — they're just running with different system prompts and different scopes.

## Pattern 2 — Long-running watcher

Run a long task in a pane, have an external agent notice when it finishes and act:

```js
const pane = await mcp.unterm.session.spawn({ cwd, prog: ["bash", "-c", "make ci-full"] })

while (!await mcp.unterm.session.is_idle(pane.id)) {
  await sleep(60_000)
}
const transcript = await mcp.unterm.session.read_all(pane.id)
if (transcript.match(/PASS \d+ tests/)) {
  notify("CI green, ready to merge")
} else {
  notify(`CI red, last 50 lines:\n${tail(transcript, 50)}`)
}
```

Same pattern works for `cargo build`, `npm run test`, long ETL jobs, or any "kick off something, wait, decide" loop. The agent doesn't have to be a chat session — a cron job calling `unterm-cli` works the same way.

## Pattern 3 — Multi-pane orchestration

When an outer agent fans out work across several projects, each pane is one project's "workspace" and the outer agent aggregates across them:

```js
const repos = ["solomd", "unflick", "unterm"]
const panes = await Promise.all(repos.map(r =>
  mcp.unterm.session.spawn({ cwd: `/Volumes/Dev/code/${r}` })
))

panes.forEach(p => mcp.unterm.session.send_text(p.id, "make lint\n"))

await Promise.all(panes.map(p => mcp.unterm.session.wait_idle(p.id)))
const reports = await Promise.all(panes.map(p =>
  mcp.unterm.session.read_tail(p.id, 100)
))
```

## Pattern 4 — Recording for review

Every pane can be recorded into a markdown transcript with built-in token redaction. An outer agent triggers a recording, drives a session, then ships the markdown for human review or AI fine-tune:

```js
await mcp.unterm.session.recording_start({ pane_id })
await mcp.unterm.session.send_text(pane_id, "<commands the agent runs>")
// ... agent works ...
const result = await mcp.unterm.session.recording_stop({ pane_id })
// result.markdown_path: "<cwd>/.unterm/sessions/2026-05-02/tab-0-104530.md"
// result.block_count: how many OSC 133 prompts were captured
// result.redaction_count: how many tokens were masked
```

Recordings live under `<cwd>/.unterm/sessions/<date>/` when there's a writable project directory; otherwise they fall back to `~/.unterm/sessions/_orphan/`. Tokens, GitHub PATs, and 40+ char base64/hex strings are masked before write.

## MCP method reference

Most-used methods, all under the `unterm` namespace:

| Method | Purpose |
|---|---|
| `session.list` | Enumerate panes (id, cwd, shell, dimensions) |
| `session.spawn` | Open a new pane with given cwd / prog / env |
| `session.send_text` | Type characters into a pane (as if user typed) |
| `session.read_tail` | Read the last N lines of pane output |
| `session.recording_start` / `_stop` | Record to redacted markdown |
| `screenshot.capture` | Region screenshot, returns PNG path |
| `proxy.status` / `.toggle` | Read or flip the proxy on/off |
| `theme.list` / `.switch` | Get or change the active terminal theme |
| `server.info` | Server identity + ports + version |

Full schema in the repo at `wezterm-gui/src/mcp/handler.rs`. The `unterm-cli` binary uses this same surface — every CLI subcommand maps 1:1 to an MCP method.

## Security model

- Both servers bind to `127.0.0.1` only. Nothing on the LAN can reach them; no port forwarding by default.
- Every request must carry the auth token from `server.json` — generated fresh on each launch, written with 0600 permissions. Connections without it get 401 immediately.
- No telemetry, no analytics, no login. The auth token never leaves your machine. Recordings stay on disk under your project dir.
- The `session.spawn` and `session.send_text` methods can run arbitrary shell commands — treat the auth token like an SSH key, not like an API key. If you're checking `~/.unterm/` into version control by accident, you'll be sad.

## CLI parity

Everything an MCP client can do, `unterm-cli` can do too — it's a thin JSON-RPC client over the same MCP server, no duplicated business logic. Useful for cron jobs, shell scripts, CI steps that don't carry an MCP client:

```sh
unterm-cli session list --json | jq '.[] | select(.cwd | endswith("unflick"))'
unterm-cli session record start --id 0
unterm-cli proxy status
unterm-cli screenshot --include-window --output /tmp/cap.png
```

Pipe `--json` through anywhere downstream that wants raw JSON-RPC.

---

Source for everything described here lives at [github.com/unzooai/unterm](https://github.com/unzooai/unterm) under MIT. File issues / PRs there.
