---
name: parallel-agents
description: Orchestrate multiple AI coding agents in parallel terminal panes — fan work out (e.g. "fix this bug 3 ways", "have Claude/Codex/Gemini each take a stab"), watch their progress, and reconcile results. Uses Unterm MCP to spawn a pane per agent, drive each one, and aggregate. Activates when the user asks to "run agents in parallel", "compare X agents on Y task", or "fan out to multiple agents".
license: MIT
---

# parallel-agents

You're the *outer* agent. The user wants you to fan a task out across
multiple *inner* AI coding agents (Claude Code, Codex CLI, Gemini CLI,
OpenCode, Aider), each running in its own Unterm pane, then bring the
results back together.

## Why this works on Unterm

- `session.split` carves panes from outside — one pane per inner agent.
- `unterm-cli agent launch <id>` (or the MCP equivalent) starts the inner
  agent in a pane with the right cwd + auto-wired MCP back to Unterm.
- `screen.scrollback_text` reads each inner agent's output as a single
  string — feed it back to yourself to decide what to do next.
- Every pane is independent (own PTY, own auth profile, own shell history)
  so the agents don't trip over each other.

## The pattern

```
                       ┌────────────────────┐
                       │  outer (you)       │
                       │  · plans the fan   │
                       │  · reads results   │
                       │  · reconciles      │
                       └────────┬───────────┘
                                │ Unterm MCP
                ┌───────────────┼────────────────┐
                │               │                │
        pane 1 (claude)  pane 2 (codex)   pane 3 (gemini)
        gets task A     gets task A      gets task A
        runs its loop   runs its loop    runs its loop
                │               │                │
                └───────────────┼────────────────┘
                                ▼
                          you, again
                          read each pane's
                          scrollback, diff
                          their solutions,
                          present to user
```

## Recipe — three agents, same task

```jsonc
// 1. snapshot what's already open so you don't clobber the user's panes
{ "method": "session.list" }
// → { "sessions": [{"id": 0, ...}] }   (just the user's current pane)

// 2. open three side-by-side panes
{ "method": "session.split", "params": { "id": 0, "direction": "right", "cwd": "/repo" } }  // → id 5
{ "method": "session.split", "params": { "id": 5, "direction": "down",  "cwd": "/repo" } }  // → id 7
{ "method": "session.split", "params": { "id": 0, "direction": "down",  "cwd": "/repo" } }  // → id 9

// 3. launch one agent per pane
{ "method": "exec.run",       "params": { "id": 5, "command": "unterm-cli agent launch claude-code" } }
{ "method": "exec.run",       "params": { "id": 7, "command": "unterm-cli agent launch codex-cli"   } }
{ "method": "exec.run",       "params": { "id": 9, "command": "unterm-cli agent launch gemini-cli"  } }

// 4. give each one the SAME prompt
const task = "Add a regression test for the bug in src/parser.rs:142. Then fix it. Commit.\n"
{ "method": "session.input", "params": { "id": 5, "input": task } }
{ "method": "session.input", "params": { "id": 7, "input": task } }
{ "method": "session.input", "params": { "id": 9, "input": task } }

// 5. wait until they're idle (poll session.idle every 30s, exponential backoff)
{ "method": "session.idle", "params": { "id": 5, "timeout_s": 600 } }
{ "method": "session.idle", "params": { "id": 7, "timeout_s": 600 } }
{ "method": "session.idle", "params": { "id": 9, "timeout_s": 600 } }

// 6. read each pane's transcript
{ "method": "screen.scrollback_text", "params": { "id": 5 } }
{ "method": "screen.scrollback_text", "params": { "id": 7 } }
{ "method": "screen.scrollback_text", "params": { "id": 9 } }

// 7. each inner agent committed on its own branch — diff them
{ "method": "exec.run_wait", "params": { "id": 0, "command": "git log --oneline --all --since '1 hour ago'" } }
{ "method": "exec.run_wait", "params": { "id": 0, "command": "git diff master..agent/claude" } }
// ... etc. then present the diffs to the user.
```

## Identity isolation (important)

If you're going to send the same prompt to three different vendor agents,
each in their own pane, give each pane its own **profile** so credentials
don't mix:

```jsonc
{ "method": "exec.run_wait",
  "params": { "id": 0, "command": "unterm-cli profile create claude-test" } }
{ "method": "exec.run_wait",
  "params": { "id": 0, "command": "unterm-cli profile spawn claude-test" } }
```

That opens a new Unterm window bound to the `claude-test` profile, with
its own keychain-scoped tokens (GitHub PAT, npm, AWS, ...). Useful when
you don't want one agent's force-push to land under your "real" git
identity.

## Hard rules

- **Don't auto-merge agent outputs**. Show diffs, let the user pick.
- **Don't run more than ~3 agents in parallel by default** — the host
  machine fans out CPU+IO and can wedge. Ask before going to 5+.
- **Stop polling `session.idle` once any pane errors out** (non-zero exit
  detected via `screen.detect_errors`). Show what went wrong, don't keep
  the others running pointlessly.
- **Clean up at the end**: `session.destroy` on each pane you opened,
  *after* the user has acknowledged the results.

## When NOT to use this

- Tasks where agents disagree on intent (architecture decisions, naming
  bikesheds) — fanning out wastes their context without converging.
- Anything writing to shared mutable state (databases, deploy targets) —
  three agents pushing to prod in parallel is a bad day.
- Trivial tasks where one good agent does fine — fan-out has overhead.
