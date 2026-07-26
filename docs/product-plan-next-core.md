# Unterm Product Plan: Stabilize Current Core, Build Next Core

Status: strategic planning draft  
Owner: product / engineering  
Last updated: 2026-07-26  
Planning horizon: 12 months

## 1. Executive Decision

Unterm should not remain a WezTerm fork forever.

The current WezTerm-based engine gives Unterm a working terminal foundation, but it is too large, too entangled, and too hard to shape into the level of responsiveness and product polish expected from a modern agent-first terminal like Warp.

The product direction is:

1. Stabilize the current WezTerm-based app enough to ship and retain users.
2. Extract Unterm's product layer behind stable internal interfaces.
3. Build an experimental `next-core` terminal engine focused on Windows-first responsiveness, clean architecture, and agent-native control.
4. Migrate feature groups only after `next-core` proves it beats the current core on latency, stability, and development velocity.

This is not a vanity rewrite. It is a controlled platform transition.

## 2. Product North Star

Unterm should become the fastest local agent terminal:

- as smooth as Warp in daily terminal interaction
- more open than Warp through local MCP
- more agent-aware than Ghostty/iTerm/Windows Terminal
- safer for multi-agent coding through Fleet, Review, Verification, and Profile isolation

One-line target:

> A modern terminal where humans supervise work and AI agents operate the machine through a first-class local control plane.

## 3. Strategic Principles

### 3.1 Keep Shipping While Rebuilding

The current product must continue improving. Users should not wait a year for a rewrite.

Current-core work focuses only on:

- freezes
- input latency
- scrolling latency
- paste reliability
- tab/sidebar stability
- instance discovery
- critical Windows bugs
- release packaging

No large current-core feature expansion unless it directly supports the next-core migration.

### 3.2 Product Layer Must Outlive the Terminal Engine

MCP, Agent Cockpit, Fleet, Review, Profile, Recording, Workspace, Proxy, and Settings are Unterm's product moat. They must be separated from WezTerm internals.

Every product feature should move behind an engine-neutral boundary:

- sessions
- panes
- screen reads
- input writes
- cwd/process metadata
- scrollback
- screenshots
- recordings
- window/instance metadata

### 3.3 Next-Core Starts Narrow

Do not rebuild the entire terminal at once.

The first next-core milestone only needs:

- one local shell
- fast input
- fast output
- correct basic VT rendering
- scrollback
- copy/paste
- tabs
- MCP session/input/screen/exec basics

If this cannot clearly beat the current engine, the rewrite does not graduate.

### 3.4 Use Proven Low-Level Libraries Where They Save Years

"Self-built core" does not mean hand-writing every protocol.

Prefer mature libraries for:

- PTY creation
- VT parsing
- Unicode width
- font shaping
- GPU abstraction
- platform windows where appropriate

Own the architecture, scheduler, pane model, render pipeline, input pipeline, and product integration.

## 4. Product Pillars

### Pillar 1: Modern Terminal Feel

Target user perception:

- typing feels instant
- paste never freezes the app
- scrolling is continuous
- tab switching is immediate
- agent output does not make UI chrome lag
- the app feels like a modern native tool, not a terminal emulator with plugins bolted on

Key outcomes:

- p95 input-to-paint latency under 16 ms during normal shell use
- p95 input-to-paint latency under 33 ms with two active agents
- no UI thread stall above 100 ms during paste, scroll, tab switch, or agent startup
- large-output flood does not block MCP responsiveness

### Pillar 2: Agent Control Plane

Unterm remains the terminal AI agents can drive.

Key outcomes:

- MCP stays local, auth-token gated, and loopback-only
- `meta.surface` remains the live capability source
- core session APIs behave identically across current-core and next-core
- agents can create panes, send input, read screen, search, capture, record, and inspect state without screen scraping

### Pillar 3: Agent Cockpit

Unterm is the human cockpit for agents running inside panes.

Key outcomes:

- state detection is reliable
- waiting agents are never missed
- Inbox routes across instances
- Fleet launches remain isolated and reviewable
- Review/Verify/Merge stays safer than manual agent work

### Pillar 4: Identity and Safety

Users must not lose track of accounts, secrets, or destructive actions.

Key outcomes:

- one window = one identity
- secrets remain in OS vaults
- profile state is visible
- destructive commands and review operations have clear confirmation/audit paths

### Pillar 5: Distribution and Trust

Install and release must become boring.

Key outcomes:

- Windows MSI install works consistently
- macOS remains signed/notarized
- Linux deb/AppImage remain current
- release artifacts match docs
- selftest catches environment problems before users do

## 5. Roadmap Overview

| Phase | Timeframe | Theme | Main Result |
|---|---:|---|---|
| Phase 0 | Now-2 weeks | Current-core stabilization | Existing app becomes usable under agent load |
| Phase 1 | 2-6 weeks | Product layer boundary | Unterm product services separated from WezTerm internals |
| Phase 2 | 6-12 weeks | next-core spike | Minimal terminal proves latency and architecture |
| Phase 3 | 3-5 months | next-core alpha | Agent-operable terminal with tabs, scrollback, MCP basics |
| Phase 4 | 5-8 months | product migration | Cockpit/recording/settings/profile attach to next-core |
| Phase 5 | 8-12 months | beta and switch | next-core becomes default for selected users/platforms |

## 6. Phase 0: Current-Core Stabilization

### Objective

Make the existing app reliable enough that users can run Claude/Codex/Gemini/Aider without freezes.

### Scope

Current-core work is limited to critical user experience and correctness defects.

### Required Work

1. Windows input and paste path
   - Keep clipboard retry work off UI thread.
   - Ensure right-click paste is deterministic.
   - Test large auth-code paste into Claude/Codex.
   - Verify IME/composition is not regressed.

2. Command completion path
   - Keep ghost completion fully memory-only on key events.
   - Avoid manifest/disk reads on key events.
   - Avoid global history clone per key.
   - Ensure right arrow and application right arrow both accept completion.

3. Paint path
   - Keep agent/sidebar metadata cached during scroll paints.
   - Remove periodic agent breathing animation invalidations.
   - Bound left sidebar recomputation.
   - Add opt-in slow-frame diagnostics for paint sections.

4. Agent status refresh
   - Keep process-tree scans off UI path.
   - Enforce TTL and max in-flight limits.
   - Avoid scanning all panes on every frame.

5. Instance discovery
   - Avoid deleting live Windows instances when PID probing is uncertain.
   - Keep current instance state in memory.
   - Self-heal active/server files.
   - Verify multi-window routing.

6. Install/release
   - Fix Windows elevated install script reliability.
   - Verify Program Files binary replacement.
   - Add version/build stamp visibility in UI/CLI.

### Acceptance Criteria

- Two Claude panes plus one Codex pane do not freeze UI.
- Opening Codex does not stall UI above 100 ms.
- PageUp/PageDown and wheel scroll remain responsive.
- Right-click paste of long auth code succeeds first time.
- Command completion accept works with right arrow.
- Instance list does not lose live windows.
- `cargo check -p unterm` passes.
- Manual smoke test passes on Windows.

### Deliverables

- Stabilization PRs
- Windows smoke test checklist
- Known-current-core limitations doc

## 7. Phase 1: Product Layer Boundary

### Objective

Separate Unterm product behavior from WezTerm-specific implementation details.

### Why

Without this phase, next-core becomes a second app and every feature must be rewritten twice. With this phase, current-core and next-core can share product services.

### Architecture Target

Create an engine-neutral product boundary:

```text
Unterm Product Services
  MCP
  CLI bridge
  Agent Cockpit
  Fleet / Review / Verification
  Profile
  Recording
  Workspace
  Proxy
  Settings
        |
        v
Terminal Engine Interface
  sessions
  panes
  input
  screen
  scrollback
  cwd/process metadata
  capture
  window/instance
        |
        +-- wezterm-engine adapter
        +-- next-core adapter
```

### Required Interfaces

1. `TerminalEngine`
   - list sessions
   - create session
   - split session
   - focus session
   - destroy session
   - resize session
   - active session

2. `PaneIo`
   - write input
   - send signal/control character
   - read visible text
   - read scrollback range
   - search scrollback
   - cursor state

3. `PaneMetadata`
   - title
   - cwd
   - foreground process
   - dimensions
   - busy/idle
   - progress

4. `CaptureEngine`
   - screen capture
   - window capture
   - scrollback render
   - clipboard snapshot

5. `WindowEngine`
   - instance info
   - focus window
   - set title
   - current profile

### Migration Plan

1. Document every MCP method's engine dependency.
2. Extract method handlers that are already product-only.
3. Introduce traits for session/screen/input.
4. Implement WezTerm adapter against existing code.
5. Keep public MCP/CLI behavior unchanged.
6. Add adapter tests for core methods.

Tracking document: [`docs/engine-dependency-map.md`](engine-dependency-map.md).

### Acceptance Criteria

- MCP handler can call engine traits for session/input/screen operations.
- Product services compile without directly importing deep `TermWindow` internals where avoidable.
- `meta.surface` remains unchanged.
- Current app behavior is unchanged.
- next-core can implement a minimal subset of the interface without linking WezTerm GUI.

## 8. Phase 2: Next-Core Spike

### Objective

Build the smallest possible terminal core that proves Unterm can beat the current engine on latency and code control.

### Spike Scope

Supported:

- Windows first
- one window
- one local shell
- one tab
- plain text output
- common ANSI styles/colors
- scrollback
- keyboard input
- paste
- visible screen text read
- `session.list`
- `session.input`
- `screen.text`
- `exec.run`

Not supported yet:

- SSH
- mux server
- complex splits
- ligatures
- images
- advanced copy mode
- full Web Settings integration
- Fleet/Review UI
- external screenshots

### Technology Candidates

PTY:

- Windows ConPTY
- Unix PTY via existing crates or narrow custom wrapper

VT parser:

- `vte` crate or extracted termwiz parser
- avoid writing parser from scratch initially

Rendering:

- `wgpu` preferred for cross-platform GPU path
- fallback investigation only if wgpu blocks Windows latency

Window/app:

- `winit` or native platform shell
- choose the minimum that allows tight event-loop control

Text:

- `cosmic-text`, `swash`, or shaped glyph stack evaluation
- first spike can begin with simpler shaping, then validate CJK/emoji later

### Performance Tests

The spike must run a local benchmark harness:

1. Engine input-write call latency
2. Key press to visible glyph
3. Paste 10 KB
4. Print 100k lines
5. PageUp/PageDown through 10k scrollback
6. Two pseudo-agent output streams
7. Agent startup burst while interactive input remains responsive
8. MCP `screen.text` during output flood

### Graduation Criteria

The spike graduates only if:

- input latency is clearly better than current-core
- output flood does not stall input
- scrollback paging is smooth
- MCP basics work
- architecture is simpler than current-core
- code size and dependency graph are understandable

If it does not graduate, keep it as research and continue current-core optimization.

## 9. Phase 3: Next-Core Alpha

### Objective

Turn the spike into a usable alpha terminal for internal daily use.

### Scope

Terminal:

- tabs
- splits
- scrollback
- selection
- copy/paste
- search
- basic copy mode
- title updates
- cwd tracking
- theme colors
- basic font fallback
- CJK width correctness
- emoji fallback MVP

Product integration:

- MCP server
- CLI routing
- instance discovery
- session APIs
- exec APIs
- screen APIs
- policy/audit basics
- profile env injection
- proxy env injection
- session recording MVP

### Out of Scope

- full parity with every WezTerm feature
- Lua config compatibility
- every legacy keybinding
- terminal image protocol
- SSH/mux parity

### Acceptance Criteria

- Engineering team can use next-core for ordinary shell work for one day.
- CLI can create/focus/input/read panes.
- Agent can run a simple command loop through MCP.
- Two active agent panes do not make UI unusable.
- Profile-bound shell receives env.
- Recording export produces markdown.

## 10. Phase 4: Product Migration

### Objective

Attach Unterm's differentiated product features to next-core.

### Migration Order

1. MCP and CLI parity for core session methods
2. Agent state detection
3. Inbox
4. Composer and suggestions
5. Recording
6. Profiles
7. Workspaces
8. Screenshots / scrollback rendering
9. Fleet launch
10. Review / Verify / Merge
11. Web Settings / Review UI

### Key Requirement

Each migrated feature must support dual-engine operation until the default switch:

- current-core remains stable
- next-core gains feature parity gradually
- docs indicate which engine supports which feature if there is a temporary gap

### Acceptance Criteria

- Agent Cockpit works in next-core.
- Fleet can launch agents into next-core panes.
- Review can verify and merge work created from next-core fleets.
- Profile and proxy injection work.
- Session export and scrollback text work.
- `meta.surface` reflects correct capabilities.

## 11. Phase 5: Beta and Default Switch

### Objective

Make next-core the default for selected users/platforms once it is better than current-core.

### Rollout Strategy

1. Internal dogfood
2. Hidden config flag
3. CLI launch flag
4. Web Settings experimental toggle
5. Windows beta default
6. Cross-platform beta default
7. Remove WezTerm dependency only after parity confidence

### Default Switch Criteria

Next-core can become default when:

- Windows daily use is stable
- agent-heavy workflows are smoother than current-core
- no core MCP method regressions
- no critical recording/profile/proxy regressions
- install/update works
- crash recovery and instance discovery work
- documentation is updated

### Keep or Remove Current-Core

After next-core default:

- keep current-core behind compatibility flag for one major release
- collect bug reports
- remove current-core only when usage and bug volume justify it

## 12. Feature Priority Matrix

| Feature | Current-Core | Next-Core Priority | Rationale |
|---|---:|---:|---|
| Input latency | P0 | P0 | Product feel depends on it |
| Paste reliability | P0 | P0 | Auth/code workflows break without it |
| Scroll/page performance | P0 | P0 | Daily use and agent logs |
| MCP session/input/screen | P0 | P0 | Product thesis |
| Instance discovery | P0 | P0 | Multi-window routing |
| Agent state | P0 | P1 | Core differentiator |
| Inbox | P1 | P1 | Human supervision |
| Fleet | P1 | P2 | Differentiator, but after core stable |
| Review/Verify | P1 | P2 | Safety layer |
| Profiles | P1 | P1 | Identity safety |
| Recording | P1 | P1 | Agent/debug workflow |
| Web Settings | P2 | P2 | Important, not first alpha blocker |
| External long screenshot | P3 | P3 | Valuable but platform-specific |
| SSH/mux parity | P3 | P3 | Defer until local core proves itself |
| Lua config parity | P4 | P4 | Avoid inheriting old complexity |

## 13. Team Execution Model

### Track A: Current-Core Stabilization

Purpose: keep users productive.

Cadence:

- weekly bugfix batches
- Windows-first manual smoke test
- PRs stay small
- no broad refactors unless they unblock next-core boundary

### Track B: Product Layer Extraction

Purpose: prepare migration.

Cadence:

- interface-first PRs
- adapter tests
- no user-visible behavior changes
- keep MCP/CLI snapshots stable

### Track C: Next-Core

Purpose: build the future engine.

Cadence:

- prototype branch
- benchmark-driven
- aggressive deletion allowed inside next-core only
- no promise of parity until graduation

## 14. Engineering Milestones

### Milestone 1: Current-Core Stable

Target: 2 weeks

Deliverables:

- Windows input/paste/scroll/tab fixes
- install script verified
- slow-frame diagnostics
- updated troubleshooting guide

### Milestone 2: Engine Interface Draft

Target: 4 weeks

Deliverables:

- `TerminalEngine` trait draft
- WezTerm adapter for session/input/screen
- MCP handlers using interface for core methods
- engine dependency map

### Milestone 3: Next-Core Terminal Spike

Target: 8-12 weeks

Deliverables:

- standalone next-core binary
- ConPTY shell on Windows
- VT parse/render MVP
- scrollback model
- MCP basics
- benchmark report vs current-core

### Milestone 4: Next-Core Alpha

Target: 3-5 months

Deliverables:

- tabs/splits
- selection/copy/paste/search
- profile/proxy env injection
- recording MVP
- agent state MVP
- internal dogfood guide

### Milestone 5: Product Feature Migration

Target: 5-8 months

Deliverables:

- Agent Cockpit
- Composer/suggestions
- Fleet launch
- Review/Verify
- Web Settings integration
- full smoke test suite

### Milestone 6: Public Beta

Target: 8-12 months

Deliverables:

- next-core toggle
- migration docs
- compatibility matrix
- crash report/debug bundle
- release artifacts

## 15. Risks and Mitigations

### Risk: Rewrite Takes Too Long

Mitigation:

- keep current-core shipping
- next-core starts narrow
- hard graduation criteria
- no promise of full parity early

### Risk: Terminal Compatibility Regressions

Mitigation:

- use proven VT parser initially
- build app compatibility test suite
- test shells/TUIs: PowerShell, cmd, bash, zsh, fish, nu, vim, less, fzf, tmux, git, cargo, npm, Python, Claude, Codex, Gemini, Aider

### Risk: Font/Text Complexity Explodes

Mitigation:

- phase shaping support
- test ASCII first, then CJK, emoji, ligatures
- keep font fallback isolated

### Risk: Product Layer Remains Entangled

Mitigation:

- engine interface becomes mandatory for new MCP/session work
- direct deep imports from WezTerm product services are blocked by review

### Risk: Current-Core Quality Continues Hurting Brand

Mitigation:

- stabilize first
- prioritize user-visible latency
- avoid shipping next-core promises publicly until there is a working alpha

### Risk: Two Engines Double Maintenance

Mitigation:

- dual-engine period is temporary
- current-core receives only P0/P1 fixes after next-core alpha
- product services shared

## 16. Metrics

### Performance Metrics

- input-to-paint p50/p95/p99
- paste-to-commit latency
- scroll frame time p95
- output flood throughput
- MCP response latency during flood
- tab switch latency
- startup time

### Reliability Metrics

- freeze reports per build
- crash reports per build
- instance discovery failures
- paste failures
- agent state false waiting / missed waiting
- failed recording exports

### Product Metrics

- `setup-ai` success rate
- number of active MCP clients per user
- fleet launch success rate
- review verification pass/fail completion rate
- session recording usage
- profile-bound window usage

## 17. Validation Plan

### Manual Daily Dogfood

Run:

- two Claude panes
- one Codex pane
- one normal shell
- one repo with Git panel
- one recording session
- repeated PageUp/PageDown
- repeated right-click paste
- command completion accept
- tab switching

Pass condition:

- no visible freeze
- no accidental rename overlay
- no lost instance
- no missed waiting agent

### Automated Benchmarks

Build benchmark harness for:

- input latency
- output flood
- agent startup stall
- scrollback paging
- MCP response under load
- paint section timings

### Compatibility Matrix

Test matrix:

- Windows 11 PowerShell
- Windows 11 cmd
- Windows 11 WSL bash
- macOS zsh
- Linux bash
- fish
- nushell
- vim
- less
- fzf
- tmux
- Claude Code
- Codex CLI
- Gemini CLI
- Aider

## 18. Public Messaging

Do not market next-core as a rewrite until it is usable.

Near-term public message:

- Unterm is improving Windows responsiveness and agent-heavy workflows.
- Agent Cockpit, Fleet, Review, and MCP remain the product center.

When next-core alpha is real:

- "Unterm is building a new lightweight engine for agent-heavy terminal work."
- "The current engine remains supported while the new one reaches parity."
- "The goal is lower latency and simpler architecture, not breaking compatibility for its own sake."

## 19. Immediate Next Actions

1. Finish current-core stabilization PR and merge.
2. Fix Windows Program Files install verification.
3. Add slow-frame diagnostics guide.
4. Create engine dependency map from MCP methods to WezTerm internals.
5. Draft `TerminalEngine` trait.
6. Build `next-core` spike folder with a Windows shell MVP.
7. Add benchmark harness before optimizing.
8. Decide next-core rendering stack after spike measurements.

## 20. Final Recommendation

The right plan is not "rewrite everything now."

The right plan is:

- stabilize the existing app
- extract the product layer
- build a narrow next-core
- benchmark it honestly
- migrate only after it proves better

This keeps Unterm moving toward a Warp-level modern terminal experience without sacrificing the agent-native product surface that already makes it different.
