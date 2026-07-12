---
layout: ../../layouts/Doc.astro
title: Agent Cockpit
subtitle: See, aggregate, and orchestrate every AI coding agent running inside your terminal — live state per pane, an inbox for agents that need you, parallel fleets in isolated git worktrees, and a review page where nothing an agent does goes untracked.
kicker: Docs / Agent Cockpit
date: 2026-07-11
---

Unterm's thesis is "the terminal AI agents can drive": every window exposes an MCP server so external agents can operate it. The Agent Cockpit (v0.55) is the other half of that thesis. When Claude Code, Codex CLI, Gemini CLI, or Aider run *inside* your panes, the terminal now sees them, tells you what they're doing, routes you to the one that's blocked, runs one task across several of them in parallel, and gives you a diff-level review of everything they produced.

Everything described on this page is equally available three ways: the GUI, `unterm-cli`, and MCP methods — so an external agent can drive the cockpit itself.

## Agent state, per pane

Unterm continuously answers one question for every pane: *which agent is running here, and what is it doing right now?* The answer is one of four states:

| State | Meaning | Shown as |
|-------|---------|----------|
| `working` | mid-turn, running | breathing blue dot |
| `waiting` | blocked on you — approval, confirmation, input | amber dot, amber chip, Inbox top |
| `done` | turn just finished (decays to idle after a few seconds) | green dot |
| `idle` | agent process alive, sitting at its prompt | grey dot |

### How detection works — four signal layers

The state engine folds four independent signal sources, strongest first. A weaker layer never overrides a stronger layer's recent verdict, with one deliberate exception: a `waiting` verdict always penetrates — losing an "agent needs you" is the worst possible failure, and a false positive clears on your first keystroke in that pane.

1. **Official lifecycle hooks** — the agent itself reports `working` / `waiting` / `done` via `unterm-cli agent signal`, wired into Claude Code's hooks, Codex's `notify`, and Aider's `notifications-command` by `unterm-cli agent enable-hooks` (also run automatically by `setup-ai`). All writes are merge-only and leave a `<file>.unterm-bak` backup; an existing user setting is never overwritten.
2. **OSC parsing** — zero-config. Claude Code and Codex animate a braille spinner in the terminal title while working (Claude idles as `✳ <task summary>`); Gemini CLI's dynamic title encodes all four states outright (`✋ Action Required`, `◇ Ready`, `⏲ Working…`); OSC 9;4 progress sequences give exact turn boundaries; OSC 9/777 notifications carry approval requests and turn completions.
3. **Foreground-process detection** — recognizes `claude`, `codex`, `gemini`, `aider`, `opencode`, `kimi`, `trae`, `cursor-agent` in the pane's process tree, via a cached, non-blocking lookup.
4. **Screen-text heuristics** — the fallback for agents that emit nothing (Aider's line-based REPL): `esc to interrupt` reads as working, a confirmation prompt as waiting.

Net result: **Claude Code and Gemini CLI are fully tracked with zero configuration.** One `unterm-cli agent enable-hooks` upgrades Claude Code, Codex, and Aider to exact hook-level reporting.

### Where you see it

- **Sidebar tabs** carry a state dot per tab (aggregated across the tab's panes; waiting outranks working outranks done). The working dot breathes on a 3.2s cycle; when nothing is working the animation costs exactly zero.
- **The top bar** shows a cross-window tally — `⏦ 2 🔔 1` means two agents working, one waiting — and turns amber the moment anything waits on you. Click it to open the Inbox.

## The Agent Inbox

`Ctrl+Shift+A` (or click the top-bar chip). One palette listing every tracked agent across **all** windows, sorted waiting-first, each row showing the agent, its state, how long it's been there, and its task summary. `Enter` jumps straight to that pane wherever it lives. Two persistent rows launch the rest of the cockpit: **Launch fleet…** and **Open review**.

```
unterm-cli agent status          # same list in your shell
unterm-cli agent inbox           # with tab/window locations
```

MCP: `agent.status`, `cockpit.inbox`.

## Fleet — one task, N agents, N isolated worktrees

A fleet runs the same task through several agents in parallel, each in its own **git worktree** so they can't trample each other — and can't touch your working copy.

```
unterm-cli fleet launch --agents claude,claude,codex -- fix the login bug
```

or from the Inbox → *Launch fleet…*: type (or paste) the task, pick a crew preset — presets are generated from the agents actually installed on your PATH, including `claude + codex + gemini` for a three-model bake-off — and press Enter. For each member, Unterm:

1. creates `../<repo>.fleet/<task-slug>-<n>/` on branch `fleet/<task-slug>-<n>` (worktrees live beside your repo, same disk, no cross-volume surprises on Windows);
2. opens a tab whose shell starts in that worktree, and types the agent's launch command with your task;
3. tracks the member's state on that tab's badge — you can watch three attempts progress from the tab bar.

Fleets persist in `~/.unterm/fleets.json`; a closed pane doesn't lose the work, because the work product lives in the worktree until you review it. `unterm-cli fleet clean <id>` removes worktrees, branches, and panes — refused while members are still pending review, unless you `--force`.

Launching requires a clean working tree (commit or stash first); Unterm will tell you rather than stash silently.

## Review — nothing an agent does is untracked

Two kinds of review baseline exist:

- **Fleet members** — the worktree's start commit is the baseline, for free.
- **Loose agents** (you just ran `claude` in some repo) — the moment the cockpit sees the agent start working, it snapshots the entire worktree as a **dangling commit** via a temporary index: HEAD, your index, and your files are untouched. Snapshots are debounced (60s) and capped (20 per repo), recorded in `~/.unterm/checkpoints.json`.

The **Review page** (Web Settings → Review, or Inbox → *Open review*, or `unterm-cli review open`) shows every fleet member and every checkpoint:

- **Line-level diffs**, untracked files included (a plain `git diff` misses files the agent created; the Review API synthesizes them in).
- **Merge** squash-merges a member's branch into your repo and stops at *staged* — the commit, its message, and the moment it lands stay yours.
- **Discard** marks a member's take as rejected.
- **Roll back** restores a repo's worktree to any checkpoint (destructive for anything newer; double-confirmed).
- **Compare** puts two members' diffs side by side — the fastest way to judge a `claude` vs `codex` bake-off.

```
unterm-cli review list
unterm-cli review diff --fleet <id> --member 1
unterm-cli review merge --fleet <id> --member 1
unterm-cli review rollback --repo /path --sha <checkpoint> --yes
```

MCP: `review.list`, `review.diff`, `review.rollback`, `review.merge`, `review.discard`. Rollback and merge are audited write operations like every other MCP write.

## MCP & CLI surface

| MCP method | What it does |
|---|---|
| `agent.status` | per-pane agent state (all panes, or one) |
| `agent.signal` | hook ingestion: `working` / `waiting` / `done` / `idle` (pane defaults to `$WEZTERM_PANE`) |
| `cockpit.inbox` | everything that wants attention, waiting-first, with tab/window locations |
| `fleet.launch` / `fleet.list` / `fleet.clean` | run / inspect / retire fleets |
| `review.list` / `review.diff` / `review.rollback` / `review.merge` / `review.discard` | the review lifecycle |

CLI: `unterm-cli agent status|signal|inbox|enable-hooks`, `unterm-cli fleet launch|list|clean`, `unterm-cli review list|diff|merge|discard|rollback|open`. Add `--json` to any of them for machine-readable output.

`agent.signal` deserves one note: with several Unterm windows open, pane ids repeat across instances, so the CLI routes each signal to the instance that owns the calling pane via the instance-unique `gui-sock-<pid>` in the pane's inherited environment. Hooks can't mis-report across windows.

## Configuration

Three options, all defaulting to on / sensible:

```lua
config.cockpit_enabled = true         -- master switch
config.cockpit_auto_checkpoint = true -- snapshot repos when an agent starts working
config.cockpit_done_hold_secs = 8     -- how long "done" shows before settling to idle
```

## Design notes

- The cockpit never talks to any cloud. State detection, fleets, checkpoints, and review are all local; the only network traffic is whatever your agents themselves do.
- The cockpit is agent-neutral by design: it doesn't replace Claude Code / Codex / Gemini — it makes the terminal the best place to run all of them, including at the same time, including against each other.
- Full design document: [`docs/design-agent-cockpit.md`](https://github.com/zhitongblog/unterm/blob/master/docs/design-agent-cockpit.md) in the repo, including the signal-precedence rules and every implementation deviation.
