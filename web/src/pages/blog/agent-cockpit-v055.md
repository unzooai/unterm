---
layout: ../../layouts/Doc.astro
title: "Unterm v0.55: the Agent Cockpit"
subtitle: "Your terminal now sees every AI agent running inside it — who's working, who's blocked on you, and what they changed. Plus fleets: one task, three agents, three isolated worktrees, side-by-side review."
kicker: Blog / Release
date: 2026-07-11
---

If you run AI coding agents all day, you know the actual bottleneck isn't the model. It's you, alt-tabbing across panes to find out which agent finished, which one has been silently waiting twenty minutes for a `y`, and what exactly any of them did to your repo.

Unterm v0.55 makes the terminal answer those questions itself. We call it the **Agent Cockpit**.

## Who's doing what, at a glance

Every pane that hosts an agent — Claude Code, Codex CLI, Gemini CLI, Aider, and friends — now carries a live state: **working** (breathing blue dot on the tab), **needs you** (amber), **done** (green), **idle** (grey). The top bar keeps a cross-window tally that flips amber the instant any agent blocks on you.

Detection is zero-config for Claude Code and Gemini CLI: Unterm reads the signals agents already emit — title spinners, OSC progress sequences, desktop-notification escapes. One command, `unterm-cli agent enable-hooks`, upgrades Claude Code, Codex, and Aider to exact hook-level reporting (merge-only config writes, `.unterm-bak` backups, your existing settings untouched).

## The Inbox: stop hunting for blocked agents

`Ctrl+Shift+A` opens the Agent Inbox: every agent across every window, sorted **waiting-first**, with how long it's been stuck and what it's working on. Enter jumps to the pane. The most common multi-agent action — "go press y somewhere" — is now two keystrokes.

## Fleet: race three agents on the same task

```
unterm-cli fleet launch --agents claude,claude,codex -- fix the login bug
```

Each member gets its own git worktree (beside your repo, on its own branch — your working copy is never touched), its own tab, and its own state badge. Crew presets come from whatever agents you actually have installed, including `claude + codex + gemini` for a proper three-model bake-off.

## Review: nothing an agent does is untracked

The moment an agent starts working in a repo, Unterm snapshots the worktree as a dangling commit — HEAD, index, and files untouched, so you'll never notice it happened until you need it. The new **Review** page shows line-level diffs (including files the agent created), squash-merges the take you like into your repo as *staged* changes — the commit stays yours — discards the takes you don't, rolls any repo back to any checkpoint, and puts two fleet members' diffs side by side.

## Also in this release

- CJK input methods now work correctly in every palette — composition previews inline instead of vanishing (this one was found by our own daily dogfooding, ten minutes after shipping the palettes).
- OSC 9 / OSC 777 notifications now actually reach window-level consumers.
- Sidebar no longer rebuilds ~44ms per title-spinner frame while an agent works — a real CPU saving for anyone running agents for hours.

## The thesis, completed

Unterm's founding idea is *"the terminal AI agents can drive"* — every window is an MCP server that external agents operate. The cockpit is the same idea pointed inward: the agents running in your panes are now first-class citizens the terminal understands. And true to that thesis, the cockpit itself is fully scriptable — 12 new MCP methods and three new CLI command families mean an orchestrating agent can launch fleets and review diffs without a human in the chair.

Free, MIT, local-first, no accounts, no telemetry. [Download v0.55](https://github.com/zhitongblog/unterm/releases/latest) · [Agent Cockpit docs](/docs/agent-cockpit)
