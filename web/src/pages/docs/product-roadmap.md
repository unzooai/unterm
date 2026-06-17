---
layout: ../../layouts/Doc.astro
title: Product roadmap
subtitle: "The five directions that matter most for Unterm right now: UI polish, workflows, AI integration, collaboration, and distribution. This is the execution order we are using, not a wish list."
kicker: Docs / Product roadmap
date: 2026-06-16
---

## Strategy in one sentence

Unterm wins when it feels like a terminal that is visibly easier to use than Warp for agent-heavy work, while staying local-first, MCP-driven, and vendor-neutral.

That means we are not trying to out-Warp Warp on cloud AI or bundled chat. We are building a tighter control plane: the terminal as a first-class surface for humans and agents, with better ergonomics, better orchestration, and fewer hidden costs.

## The five directions

### 1. UI and chrome fidelity

Goal: make the product look and feel deliberate before we ask users to trust it with their daily terminal.

Why this comes first:
- It is the most visible quality gap.
- It sets the baseline for every other feature.
- It makes the product feel finished instead of "interesting."

What we are executing:
- Normalize the UI token system: font size, line height, spacing, radius, selection color, scrollbars.
- Finish the left tab / sidebar experience: stronger selected state, cleaner scroll behavior, tighter alignment, less chrome clutter.
- Keep the top bar and action area visually consistent across platforms.
- Remove ornamental UI that fights density or scanning.

Exit criteria:
- Selected state is obvious without adding a new decorative layer.
- Scrollbars feel attached to the edge, not floating.
- Top-level chrome reads as one system on macOS, Windows, and Linux.

### 2. Workflows and command surface

Goal: reduce the number of steps between "I know what I want to do" and "the terminal is doing it."

Why it matters:
- Warp's advantage is that it collapses common actions into a polished flow.
- Unterm can do the same, but with typed control surfaces instead of embedded chat.

What we are executing:
- Make the command palette a true front door for high-frequency actions.
- Keep the quick actions small and predictable.
- Improve session record / export / restore so a pane can be saved, resumed, and reviewed with less ceremony.
- Tighten the gap between GUI actions, CLI actions, and MCP actions so they stay functionally identical.

Exit criteria:
- The most common actions are reachable from one place.
- A session can be recorded, exported, and recovered without hunting through hidden paths.
- CLI and GUI stay behaviorally aligned.

### 3. AI integration

Goal: make external agents better at using Unterm without turning the terminal into an AI product.

Why this is a core pillar:
- Our product bet is not "AI inside the terminal."
- Our bet is "any agent can drive the terminal cleanly."

What we are executing:
- Expand and harden the MCP surface where agent control is still awkward.
- Improve discovery and onboarding for Claude Code, Codex, Gemini, Cursor, Windsurf, and any custom client.
- Keep read/write boundaries obvious: agents can control actions, but configuration stays on the user side.
- Make agent-facing outputs more structured and easier to reason about.
- Treat provider-native auth as the default story: official subscription / official account first, API key fallback second. The OpenClaw-style routing idea belongs to the next version, not this one.

Exit criteria:
- A new agent can connect with minimal setup.
- The control model is discoverable from the product itself, not only from docs.
- Agent-driven workflows do not require fragile screen scraping.

### 4. Collaboration and identity

Goal: make one machine support several distinct working identities and concurrent tasks without confusion.

Why this matters:
- This is one of the few areas where Unterm can be meaningfully different from Warp.
- Multi-instance and identity routing are not side features for us; they are the product shape.

What we are executing:
- Keep per-window identity profiles coherent: git, SSH, API tokens, and shell identity should travel together.
- Make multi-instance discovery reliable and human-readable.
- Improve naming, titles, and context so an agent can route work across windows with less guesswork.
- Continue to support concurrent director / worker patterns for agent fleets.

Exit criteria:
- Multiple windows stay distinguishable at a glance.
- The active identity is obvious from the UI.
- A router agent can reliably choose the right window by title, cwd, or instance metadata.

### 5. Distribution and trust

Goal: make installation, release, and verification boring.

Why this matters:
- The best feature dies if release confidence is low.
- Local notarization, signed binaries, and cross-platform parity are part of the product, not release chores.

What we are executing:
- Keep macOS notarization, Windows packaging, and Linux artifacts on the same release cadence.
- Keep the website and release metadata synchronized with the shipped version.
- Keep the public docs honest about what is implemented versus planned.
- Maintain a clear privacy posture: local-only control plane, no account required, no cloud dependency.

Exit criteria:
- A release can be built, verified, and published without manual archaeology.
- The website always points at the current release.
- Users can trust the download story before they trust the product.

## Execution order

This is the order we are using now:

1. UI and chrome fidelity
2. Workflows and command surface
3. AI integration
4. Collaboration and identity
5. Distribution and trust

That order is intentional. UI quality is the multiplier. Workflows turn capability into habit. AI integration preserves the core bet. Collaboration makes the product useful for serious multi-repo work. Distribution and trust keep the whole thing shippable.

## What we are not doing

- We are not building a cloud AI subscription product.
- We are not turning the terminal into a chat app.
- We are not adding every possible power-user feature before the core experience feels finished.
- We are not shipping disconnected UI experiments that do not map to the MCP / CLI / settings model.

## Near-term milestones

### 0 to 30 days

- Finish the chrome and left-sidebar polish work.
- Keep the tab bar / selected-state / scroll behavior stable across platforms.
- Tighten the public site copy so it matches the actual product shape.

### 30 to 60 days

- Make the command surface more obvious and faster to reach.
- Improve session recording and recovery flows.
- Smooth the MCP onboarding path for the supported AI agents.

### 60 to 90 days

- Strengthen identity and multi-instance workflows.
- Make routing across windows more reliable for agent fleets.
- Reduce the number of release-time surprises by tightening packaging and verification.

### Release target: v0.50.0

This release is the batch for directions 1, 2, and 3:

- UI and chrome fidelity
- Workflows and command surface
- AI integration, with provider-native auth first and API fallback only when explicitly chosen

The OpenClaw-style subscription-routing expansion is deliberately deferred to the next release after v0.50.0.

## How to judge progress

We should be able to answer these questions positively:

- Can a new user understand the product in one minute?
- Can an agent connect and do useful work without a custom script?
- Can a power user keep several windows and identities straight?
- Can we ship a release without hand-fixing the same class of packaging issues?

If a change does not move one of those answers, it is probably not the right thing to do next.
