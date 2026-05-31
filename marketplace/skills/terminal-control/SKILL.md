---
name: terminal-control
description: Drive a real macOS / Linux / Windows terminal from this conversation — spawn shells, split panes, type commands, read scrollback. Activates whenever the user wants you to "run X", "open a shell and ...", "split the pane", or "look at what's on the terminal", and the Unterm MCP server is available.
license: MIT
---

# terminal-control

You have access to a real, running terminal via the Unterm MCP server.
Treat it like a remote pair-programming buddy with hands: you can open
panes, type into them, read what came back, and chain commands.

## Before you call anything

Run `meta.surface` once at the start of any session that needs the terminal.
It returns the full live API (every MCP method with its param schema, every
`unterm-cli` subcommand, every keybinding) — your single source of truth.

If `instance.list` returns more than one entry, the user has multiple
Unterm windows open. Pick the right one by `cwd` or `title`, then pass
`{ "instance": "<id>" }` on subsequent calls (or connect to that instance's
port directly — see `~/.unterm/instances/<id>.json` for host + token).

## The 8 methods you'll reach for

| Method | When |
| --- | --- |
| `session.list` | "what panes are open?" |
| `session.create` | "open a new pane in /foo" — pass `{cwd, command?}` |
| `session.split` | "split the current pane right/down with X running" |
| `session.focus` | always call this after `session.create`/`split` so the user sees the new pane |
| `session.input` | type into a pane char-by-char (preserves shell history); pass the literal newline `"\n"` to submit |
| `exec.run_wait` | run a command and block for `{exit_code, stdout, stderr}` — strongly prefer over `session.input` when you don't need the user to see the prompt |
| `screen.scrollback_text` | dump the entire pane history + viewport as one string — feed yourself for context instead of begging for a screenshot |
| `capture.window` | take a real screenshot (returns a file path) — use sparingly; `screen.scrollback_text` is cheaper and parses natively |

## Don't do this

- **Don't run destructive commands without confirming**. `rm -rf`, force-pushes,
  `DROP TABLE`, deleting branches, killing PIDs — ask first, then run via
  `exec.run_wait` so the exit code is unambiguous.
- **Don't type into an existing pane the user is actively using**. Use
  `session.create` or `session.split` to make your own.
- **Don't loop on `screen.scrollback_text`** — call once, parse, decide. The
  scrollback is bounded; polling adds no signal.
- **Don't bypass the audit banner**. First write to a fresh pane triggers a
  user prompt; that's by design — don't try to suppress it.

## Recipe — start a coding session in a new pane

```jsonc
// 1. open a pane at the project root
{ "method": "session.create",
  "params": { "cwd": "/Users/me/repo" } }
// → { "id": 7, "pid": 41208 }

// 2. focus it so the user sees what you're doing
{ "method": "session.focus", "params": { "id": 7 } }

// 3. run the build; block on exit
{ "method": "exec.run_wait",
  "params": { "id": 7, "command": "cargo test" } }
// → { "exit_code": 0, "stdout": "...", "stderr": "...", "duration_ms": 4321 }

// 4. dump full scrollback into your context if you need to reason about output
{ "method": "screen.scrollback_text", "params": { "id": 7 } }
```

## Recipe — side-by-side reviewer

```jsonc
// split the active pane right; new pane gets a fresh shell
{ "method": "session.split",
  "params": { "direction": "right", "cwd": "/Users/me/repo" } }
// → { "id": 9, ... }
{ "method": "session.focus", "params": { "id": 9 } }
{ "method": "session.input",
  "params": { "id": 9, "input": "git diff main\n" } }
```

## Recipe — handing a pane to the user

If the user needs to type their own commands (e.g. login, paste a secret),
**stop typing** and tell them which pane id to use. Don't poll
`screen.scrollback_text` waiting for them — you have no idea when they're
done. Ask them to ping you when ready.

## When the user asks "what's running on my terminal"

Don't guess. Call `session.list` first to enumerate panes, then
`screen.scrollback_text` on each pane of interest. Summarise concisely:
which panes are alive, what command each was last running, any obvious
errors at the tail.
