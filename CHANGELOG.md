# Changelog

## Unreleased

### Added

- **Search button in the tab bar.** A magnifying-glass quick-action button
  next to tree / split / dir-jump / settings opens the scrollback search
  for the active pane — same as `Ctrl+Shift+F` / `Cmd+F`, but
  discoverable. Search was previously reachable only via keybinding or
  the Edit menu.
- **`screen.search` can now jump to its results.** New `goto: true` /
  `goto_match: N` params scroll the user-visible viewport so the match is
  on screen (with a quarter-viewport of context above it) — search-and-
  locate instead of search-and-report. Matches now carry the stable row
  index (same coordinate space as `screen.scrollback_text`) plus the
  match column, so agents can address results even as new output streams
  in. Previously the returned `row` was a bare enumeration index that
  drifted once the scrollback was trimmed, and there was no way to bring
  a match into view. The in-terminal search UI (`Ctrl+Shift+F` /
  `Cmd+F`) already jumped to matches; this brings the MCP surface to
  parity.

### Changed

- **Search bar UX revamp.** The in-pane search bar (toolbar 🔍 /
  `Ctrl+Shift+F`) is no longer a cryptic English-only one-liner:
  - Localized in all 9 languages, with a dim "type to search…"
    placeholder and the match count + match mode right-aligned so they
    don't jitter while you type.
  - Key hints are shown in the bar itself (`Enter/↑↓ jump · Ctrl+R
    mode · Esc close`) — previously none of the search keys were
    discoverable. Hints degrade gracefully on narrow panes.
  - `Shift+Enter` now jumps in the opposite direction of `Enter`.
  - Interactive search now defaults to **ignore-case** (Ctrl+R still
    cycles exact / regex). Agent-driven `screen.search` is unchanged.

### Fixed

- **Windows: multi-second freeze at launch (and on every new tab) when the
  proxy toggle is on but the proxy app isn't running.** System-proxy
  auto-detection ran twice on the GUI thread before the first prompt, and
  each pass swept 8 well-known local proxy ports serially — on Windows a
  TCP connect to a closed loopback port only fails after winsock's internal
  retry, so every closed port ate its full 120 ms timeout (~1 s per sweep,
  ~2.5 s total). Detection stages now run concurrently, the port sweep is
  parallel, and results are cached for 5 s and shared by the startup,
  spawn, ▼ menu, and Web Settings paths. Worst-case cold detection is now
  ~150 ms; spawns within the cache window are free.
- **Proxy auto-detection no longer scans ports 8080 / 8888.** These are
  far more often dev servers than proxies, and the scan's only signal is
  a successful TCP connect — a dev server on 8080 would be "detected" as
  a proxy and injected as `HTTP_PROXY` into every spawned shell, cutting
  that shell off from the network. Proxies genuinely listening there can
  still be configured explicitly in `proxy.json` (manual mode).

## v0.16.0 — 2026-05-18

Two-stage AI-friendly terminal release: locks down the MCP write
surface, adds proposal-style AI integration, and ships fish-style
inline ghost text plus a read-only insights panel.

### MCP security & audit (P0)

- **Every `session.input` / `exec.send` is audited**. The audit log
  records timestamp, method, pane id, content preview (truncated +
  escape-aware), and the calling agent's identity. Cap 1 000 entries
  (configurable via `mcp_audit_log_capacity`).
- **`agent.identify` / `agent.whoami`** — agents self-tag a
  connection so audit entries group by name instead of by socket.
- **First-time-per-agent confirmation banner**. When a new agent
  first writes to the PTY a blocking banner appears above the
  status bar:
  `⚠ <agent> wants to write to pane #N: <preview>` ·
  `[Enter] allow [Esc] block [Alt+A] always allow`.
  Worker thread parks until the user decides or the policy timeout
  (default 30 s) expires (auto-block). Policy configurable via
  `mcp_input_confirmation = Always | FirstTimePerAgent (default) | Never`.
- **Status bar chip `mcp:N`** shows cumulative MCP PTY writes. Click
  copies the recent audit log as JSON to the clipboard.
- New config: `mcp_input_confirmation`, `mcp_confirmation_timeout_ms`,
  `mcp_audit_log_capacity`, `mcp_suggest_queue_capacity`,
  `mcp_suggest_default_ttl_ms`, `mcp_trusted_agents`.

### MCP suggest API + UI (P1)

- **`session.suggest{,_status,_cancel,_list}`** — agents propose
  text that **never reaches the PTY directly**. Lifecycle:
  Pending → Accepted / Dismissed / Expired / Cancelled. Default
  TTL 60 s. Queue capacity 256.
- **Suggest bar UI** — single half-transparent row above the
  status bar showing `✨ <agent>: <text>` plus the keybind hints.
- **Default keybindings**:
  - `Tab` — accept suggestion (passes through to shell completion
    when no suggestion is pending).
  - `Esc` — dismiss suggestion / block pending confirmation
    (passes through to vim, less, etc. when neither is pending).
  - `Alt+Enter` — accept suggestion and append `\n`.
  - `Enter` — allow pending confirmation (passes through when no
    banner is up).
  - `Alt+A` — always-allow the calling agent for this session.
- Acceptance / dismissal is audited under
  `session.suggest.accept` / `session.suggest.dismiss`
  with `agent="user"`.

### Ghost Text (fish-style inline completion)

- Per-pane keystroke observer tracks the current input buffer.
  Inline grey italic continuation rendered to the right of the
  cursor when the buffer prefix-matches a previously-committed
  command.
- **Right Arrow / End** — accept the prediction (writes the
  continuation to the PTY). Returns `Unhandled` when no prediction
  is showing so the keys still move the cursor.
- **Cross-pane history pool** (512 commits) so freshly opened
  panes have something to predict from day one.
- **↑/↓ history navigation, ←/→, Tab, Home** all clear the
  ghost buffer to keep it in sync with what the shell rewrites.
- **OSC 133 prompt detection** — when shell integration is on,
  the overlay only renders inside `Input` semantic zones; TUI
  applications and command output are left alone.
- **`ghost.debug` MCP method** for inspecting per-pane buffer,
  prediction, and cross-pane commit pool from outside the GUI.

### Insights panel

- Read-only dashboard bound to **`Ctrl+Shift+I`** (or
  `KeyAssignment::ShowInsights`). Shows:
  - Shell, cwd, terminal size for the active pane.
  - Most recent 10 commands across all panes.
  - Top-5 most-typed commands (cross-pane frequency).
  - MCP write counters, time since last write, agents seen,
    pending suggestions / confirmations.
  - Last 8 audit log entries.
- Pure local data, no AI, no network round-trip.
- `q` / `Esc` / `Ctrl+C` to dismiss.

### Misc

- Status bar `mcp:N` chip click now copies up to 200 audit entries
  as JSON to the clipboard. The full overlay (Ctrl+Shift+A) is a
  follow-up.
- `agent_first_input` flag added to audit log entries when a new
  agent first hits a PTY-writing method, even when the policy
  doesn't block.
- Lock order documented: `registry()` (per-pane ghost state)
  before `global_commits()` (cross-pane pool). Future contributors
  please honor this — concurrent observe calls would deadlock if
  the order flips.

### Known limitations

- Ghost Text only learns from Enter-committed commands; commands
  recalled via shell history (↑/↓) don't enter the pool, so users
  who lean heavily on history navigation will see fewer
  predictions until they type a command at least once.
- The `Ctrl+Shift+A` audit log overlay is still placeholder — the
  status bar chip copies JSON to clipboard as a stand-in.
- AI Ghost Text (Claude API-driven completion) is intentionally
  not shipped — slated for v0.17.
