# Research: Orca ADE — what it validates, what to steal, where to interoperate

2026-08-13. Sources: onorca.dev, github.com/stablyai/orca (MIT, ~24k stars,
first stable 2026-03, near-daily releases), third-party reviews
(aistarted.com, volanea.com, daily.dev).

## What Orca is

An Electron "Agent Development Environment": worktree-isolated terminals
(xterm.js-class, WebGL), 25+ preconfigured CLI agents (Claude Code, Codex,
Gemini, ...), parallel fleets with side-by-side compare, embedded Chromium
"design mode", GitHub/Linear integration, mobile companion (iOS/Android,
pairing via SSH/Tailscale/self-hosted server), and an `orca` CLI that
agents themselves call (`worktree create`, `snapshot`, `click`, `fill`).
MCP, hooks and skills are integration surfaces.

## What it validates about Unterm

Orca's premise — agents drive the environment through a CLI, humans
supervise through status surfaces — is Unterm's founding thesis with an
IDE skin. Their `orca` CLI maps to our MCP + `unterm-cli`; their worktree
fleets to Agent Cockpit `fleet`/`review`; their agent states
(working / waiting / completed / completed-but-unread) to our cockpit
states + `agent.signal`. We are not behind on architecture; we are behind
on a handful of supervision UX surfaces.

## Where Unterm is already ahead

- **Sessions survive the front end.** Their "scrollback survives restart"
  is a persistence file; our Core keeps the live shells running through
  GUI crash/quit. Stronger claim, under-told on the site.
- **Native Rust terminal** vs Electron+xterm.js: startup, memory, latency,
  real PTY fidelity.
- **Governance**: audit log, trusted agents, write-confirmation banner,
  path scope, identity profiles, proxy policy. Orca has nothing comparable.
- **A 102-tool MCP surface** vs their CLI-first automation.

## What to learn (fits our scope; priority order)

1. **Agent inbox -> OS notifications.** Their killer loop: "get notified
   when an agent finishes or needs input, reply without walking back".
   We already have `agent.signal` firing today and an agent inbox; the
   missing 20% is a native notification when a pane's agent flips to
   waiting/done while the window is unfocused, plus a
   completed-but-unread marker on the tab. Small, native, high value.
2. **Fleet compare UX.** `fleet launch` + `review diff/verify/merge` are
   CLI-honest, but "same prompt, three agents, side-by-side" is where
   Orca demos win. A read-only compare surface over what the CLI already
   knows is worth a design pass — not an IDE.
3. **Tell the persistence story.** "Close the window, shells keep
   running, reattach later" beats "scrollback restored" on the site.
4. **One-line agent onboarding.** They preconfigure 25+ agents; audit our
   `agent install/launch` long tail and first-run ergonomics.

## What NOT to copy (subtraction principle)

- Embedded Chromium / design mode — out of scope; the Unzoo browser MCP
  already gives agents a real browser on this user's machines.
- GitHub/Linear panels — `gh` inside a pane is our answer.
- Editor — Unterm is a terminal.
- Mobile companion app — a product of its own; the honest Unterm-shaped
  version is the MCP surface over Tailscale (an agent inbox any MCP
  client can read from a phone). Revisit only with real demand.

## Seamless integration paths (cheap -> deep)

1. **Unterm tools inside Orca-launched agents (zero work, verify+doc).**
   AI auto-discovery registers `unterm-cli mcp-stdio` in the global
   configs of Claude Code/Codex/etc. Agents launched inside Orca's
   terminals read the same configs — they get Unterm's real-terminal
   tools (audited PTY, screenshots, scrollback search, profiles, proxy)
   for free. Action: verify once inside Orca, then a docs page
   "Using Unterm with Orca".
2. **Unterm's MCP registered in Orca itself.** Orca supports MCP; adding
   our server to its config gives Orca's orchestration real terminal
   capabilities its embedded xterm.js cannot offer.
3. **Cross-launch.** `unterm://open?path=` + `unterm start --cwd` make
   Unterm a click away from an Orca worktree; fleets on both sides are
   plain git worktrees, directory-compatible either way.
4. **Not worth it:** becoming Orca's embedded terminal — swapping a
   native process into their Electron shell is their call, not ours.

## Bottom line

Orca is the strongest external proof yet that "the terminal AI agents can
drive" is the right bet — built by others, on a heavier stack, with the
governance layer missing. Don't chase the IDE surface; close the three
supervision gaps (notifications, unread markers, compare view), keep the
CLI/MCP contract superior, and publish the interop story so Orca users
discover the serious terminal under their agents already exists.
