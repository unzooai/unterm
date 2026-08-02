---
layout: ../../layouts/Doc.astro
title: "Unterm v0.61: our own kernel"
subtitle: "The WezTerm fork is gone. Unterm now runs on next-core — a kernel we wrote ourselves, held to ~12,000 lines and 10 direct dependencies, that goes from launch to a live MCP surface in 22 milliseconds."
kicker: Blog / Release
date: 2026-08-02
---

> 中文版:[Unterm v0.61:自研内核](/blog/next-core-v061-zh/)

Unterm started as a fork of WezTerm. That was the right call for a two-person project that needed a working terminal on day one — WezTerm is an excellent, battle-tested codebase. But it's a codebase built for someone else's goals, and every release we shipped meant carrying tens of thousands of lines we didn't write, didn't need, and increasingly had to fight. The Agent Cockpit, screen reading, the three control surfaces — all of it was grafted onto an architecture that was never designed to be driven by external processes.

v0.61 ends the graft. The fork is gone from the build entirely. Unterm now runs on **next-core**, a terminal kernel written in-house.

## What next-core is

Everything between your keystrokes and the pixels: the escape-sequence parser, the screen model, scrollback, Unicode width handling, selection, panes and sessions, the PTY runtime, font discovery, rasterization and shaping, and a GPU renderer on winit + wgpu. All of it ours, held to a deliberate budget: **~12,000 source lines and 10 direct dependencies**. The budget isn't a brag — it's a constraint we enforce so the kernel stays small enough that one person can hold it in their head, which is the whole point of owning it.

From the old ecosystem, exactly two utility crates remain: `portable-pty` and termwiz's Unicode tables. Both are leaf dependencies doing one job each. The terminal itself — the part that decides what a terminal *is* — has no upstream anymore.

## The numbers

All measured on the same machine, old kernel vs. next-core:

| Metric | before | v0.61 |
|---|---|---|
| Launch → live MCP surface | 7.1s | **22ms** (~300×) |
| Idle CPU at a prompt | 80% of one core | **6.6%** |
| 200k-line output flood | 0.45s | 0.45s (parity) |
| Windows install-build start | 1349ms | **761ms** |

The startup number is the one that matters for what Unterm is. "The terminal AI agents can drive" was always slightly aspirational when the driving surface took seven seconds to exist — an orchestrating agent spawning a window paid that tax on every launch. At 22ms, spawning a terminal is cheaper than most MCP round-trips. Windows, throughput, and idle draw all held or improved; nothing was traded away for the startup win.

## Parity is a ledger, not a feeling

The dangerous part of replacing a mature kernel isn't the code you rewrite — it's the behavior you forget existed. So we didn't ship on vibes. We enumerated the old kernel's behavior into a **159-requirement ledger** and closed it item by item, plus a separate **29-item interaction audit** for the things ledgers miss: what happens when you drag-select during a flood, where the cursor lands after a resize mid-composition, which modifier combos reach the shell.

Typography got the same treatment, rebuilt to platform-native conventions instead of the fork's cross-platform compromises: macOS traffic lights where the OS puts them, pt = px sizing that matches what native apps mean by "13pt", CJK fixed-width correctness, and glyph rendering at CoreText weights — the "why does my terminal font look thinner than everywhere else" bug, fixed at the rasterizer.

## Also in v0.61

- **Enforced command allowlist policy** — the write-gate is now a real policy engine, not a confirmation dialog.
- **Persistent redacted audit trail** — 30 days of JSONL, tokens scrubbed, every agent action attributable after the fact.
- **Native macOS interactive screenshots and folder pickers** — the OS surfaces, not lookalikes.
- **A three-tier render pipeline** so modal overlays composite correctly over live terminal content — no more palette flicker over a running TUI.

## What owning the kernel unlocks

Every agent-facing feature we've shipped so far lived *above* the terminal: reading the screen the kernel produced, injecting input the kernel consumed. Now the kernel itself is ours, agent-first features can live *inside* it — structured screen state straight from the model instead of re-parsing cells, sub-frame damage tracking an agent can subscribe to, per-pane resource accounting. And when the next platform-native paper cut shows up, the fix is a diff in 12,000 lines we wrote, not a patch queue against someone else's roadmap.

Thanks to the WezTerm project — genuinely. Forking it is why Unterm exists; leaving it is why Unterm can become what it's for.

Free, MIT, local-first, no accounts, no telemetry. [Download v0.61](https://github.com/zhitongblog/unterm/releases/latest) · [Architecture docs](/docs/architecture)
