# Unterm Engine Dependency Map

Status: migration tracker  
Owner: product / engineering  
Last updated: 2026-07-26  
Source of truth: `unterm-agents/src/mcp_meta.rs`, `wezterm-gui/src/mcp/handler.rs`, `wezterm-gui/src/engine/*`

## Purpose

This document tracks which product surfaces are already isolated behind the engine-neutral terminal layer and which still depend on WezTerm GUI internals.

It exists to keep the `next-core` migration concrete. A method is not considered migrated because it compiles; it is migrated only when the MCP handler can call an engine trait or product service without reaching through WezTerm-specific `Mux`, `Pane`, `TermWindow`, render, capture, or platform-window internals.

## Legend

| Status | Meaning |
|---|---|
| Engine-neutral | Handler uses `CurrentTerminalEngine` traits or product-only services; usable by WezTerm and `next-core` where the trait is implemented. |
| Partial | Core operation uses engine traits, but parameter resolution, policy, waiting, GUI jump, or fallback behavior still depends on WezTerm internals. |
| Product-only | Does not require terminal engine state; should work with either engine if runtime files/services are present. |
| WezTerm-only | Depends on WezTerm mux, pane, GUI window, renderer, capture, or platform integration. Needs an interface or product-service extraction. |
| Unsupported stub | Current behavior returns an unsupported marker; define target semantics before migrating. |

## Current Engine Interfaces

Implemented in `unterm-engine` and dispatched by `wezterm-gui/src/engine/mod.rs`:

- `SessionEngine`
- `ScreenEngine`
- `InputEngine`
- `TerminalEngine`

Current covered operations:

- list/get/create/split/focus/resize/destroy sessions
- shell/cwd/activity snapshots
- visible screen read
- styled screen read
- visible text read
- line/scrollback reads
- scrollback text export
- search
- cursor
- write input
- paste input

Known gaps:

- GUI viewport scrolling/jump
- pane resolution without WezTerm `Pane`
- PTY write confirmation without WezTerm pane object
- exec wait shell detection without WezTerm pane object
- recording stream attachment
- scrollback PNG rendering
- window capture/focus/title
- instance lifecycle ownership
- agent foreground-process/cwd refresh independent of WezTerm
- profile/proxy injection as first-class engine launch context

## MCP Coverage Summary

| Category | Count | Methods |
|---|---:|---|
| Engine-neutral | 17 | `session.list`, `session.get`, `session.status`, `session.create`, `session.split`, `session.focus`, `session.idle`, `session.cwd`, `session.history`, `screen.read`, `screen.text`, `screen.scrollback_text`, `screen.cursor`, `screen.detect_errors`, `exec.status`, `screen.scroll`, `server.info` |
| Partial | 11 | `session.input`, `session.paste`, `session.resize`, `session.destroy`, `exec.run`, `exec.send`, `exec.run_wait`, `exec.cancel`, `signal.send`, `screen.search`, `server.health` |
| Product-only | 39 | `meta.surface`, `session.audit_log`, `session.suggest`, `session.suggest_status`, `session.suggest_cancel`, `session.suggest_list`, `agent.identify`, `agent.whoami`, `agent.list_trusted`, `agent.trust`, `agent.untrust`, `policy.set`, `policy.check`, `server.capabilities`, `profile.list`, `profile.current`, `profile.audit`, `fleet.list`, `review.list`, `review.diff`, `review.verify`, `review.rollback`, `review.merge`, `review.discard`, `proxy.status`, `proxy.nodes`, `proxy.switch`, `proxy.speedtest`, `proxy.configure`, `proxy.disable`, `proxy.env`, `proxy.rotation`, `proxy.set_nodes`, `proxy.clash_status`, `proxy.clash_select`, `proxy.clash_set_controller`, `upload.file`, `system.info`, `selftest.run` |
| WezTerm-only | 31 | `agent.status`, `agent.signal`, `cockpit.inbox`, `fleet.launch`, `fleet.clean`, `fleet.retry`, `ghost.debug`, `orchestrate.launch`, `orchestrate.broadcast`, `orchestrate.wait`, `workspace.save`, `workspace.restore`, `workspace.list`, `capture.screen`, `capture.window`, `capture.select`, `capture.clipboard`, `capture.scrollback`, `capture.window_scroll`, `session.recording_start`, `session.recording_stop`, `session.recording_status`, `session.recording_list`, `session.recording_read`, `session.recording_attach_trace`, `session.export_markdown`, `instance.list`, `instance.info`, `instance.set_title`, `instance.focus`, `system.launch_admin` |
| Unsupported stub | 2 | `session.env`, `session.set_env` |

The counts intentionally include aliases (`session.get` / `session.status`, `exec.send` via `session.input`) because `meta.surface` exposes them as separate public contracts. The current `MCP_METHODS` inventory contains 100 public methods, excluding `auth.login`.

## Session Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `session.list` | Engine-neutral | `SessionEngine::list_sessions` | Keep as baseline adapter test. |
| `session.create` | Engine-neutral | `SessionEngine::create_session` plus product profile env preparation | Move profile/proxy/env launch context into `CreateSessionRequest` before alpha. |
| `session.status` | Engine-neutral | `SessionEngine::get_session` | Alias of `session.get`. |
| `session.get` | Engine-neutral | `SessionEngine::get_session` | Keep output shape stable. |
| `session.split` | Engine-neutral | `SessionEngine::split_session` | `next-core` must decide split semantics before GUI alpha. |
| `session.focus` | Engine-neutral | `SessionEngine::focus_session` | Needs window focus semantics later for cross-instance jumps. |
| `session.input` | Partial | `InputEngine::write_input`, but WezTerm mode resolves/gates through `Pane` | Introduce pane-id based write-gate so `next-core` does not bypass confirmation. |
| `session.paste` | Partial | `InputEngine::paste_input`, but WezTerm mode resolves/gates through `Pane` | Same as `session.input`; add paste-size and bracketed-paste semantics to trait. |
| `session.resize` | Partial | `SessionEngine::resize_session`, but detects GUI layout through WezTerm `Mux` | Add engine capability for resize policy/layout ownership. |
| `session.destroy` | Partial | `SessionEngine::destroy_session`, but resolves `Pane` first | Convert to pane-id path. |
| `session.idle` | Engine-neutral | `SessionEngine::activity` | `next-core` must provide foreground activity. |
| `session.cwd` | Engine-neutral | `SessionEngine::shell` | `next-core` needs cwd tracking source. |
| `session.env` | Unsupported stub | Returns unsupported marker | Define per-pane env read semantics or keep explicit unsupported capability. |
| `session.set_env` | Unsupported stub | Returns unsupported marker | Prefer launch-context env over mutating live shells. |
| `session.history` | Engine-neutral | `ScreenEngine::read_scrollback` | Rename eventually? It is scrollback, not shell history. |
| `session.audit_log` | Product-only | MCP in-memory audit state | Engine-independent. |
| `session.suggest` | Product-only | MCP suggestion queue | Needs UI renderer support in `next-core`, but service is product-only. |
| `session.suggest_status` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.suggest_cancel` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.suggest_list` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.recording_start` | WezTerm-only | Recording attaches to current pane stream/scrollback model | Needs `RecordingEngine` or raw PTY stream tap. |
| `session.recording_stop` | WezTerm-only | Recording registry plus pane stream state | Extract recording service from WezTerm pane storage. |
| `session.recording_status` | WezTerm-only | Recording registry keyed by WezTerm pane | Make pane-id engine-neutral. |
| `session.recording_list` | WezTerm-only | Recording archive plus project/cwd assumptions | Archive is product-only; active recording state is not. |
| `session.recording_read` | WezTerm-only | Recording archive | Can become product-only after archive service extraction. |
| `session.recording_attach_trace` | WezTerm-only | Active recording state | Needs engine-neutral recording id. |
| `session.export_markdown` | WezTerm-only | Scrollback export/recording helpers | Can use `ScreenEngine::read_scrollback_text` for text path. |

## Exec and Signal Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `exec.run` | Partial | Resolves WezTerm `Pane`, then `InputEngine::write_input` | Convert command writes to pane-id path and reuse write-gate. |
| `exec.send` | Partial | Alias to `session.input` | Same migration as `session.input`. |
| `exec.run_wait` | Partial | WezTerm `Pane` for shell detection and repeated screen reads | Add shell kind to `SessionSnapshot` or `ShellSnapshot`; use `ScreenEngine`. |
| `exec.status` | Engine-neutral | `SessionEngine::activity` | Good next-core smoke candidate. |
| `exec.cancel` | Partial | Resolves WezTerm `Pane`, then sends Ctrl+C | Convert to pane-id and possibly `SignalEngine`. |
| `signal.send` | Partial | Resolves WezTerm `Pane`, then writes control bytes | Add `SignalEngine` only if raw control bytes are insufficient. |

## Screen Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `screen.read` | Engine-neutral | `ScreenEngine::read_screen` | Baseline next-core capability. |
| `screen.text` | Engine-neutral | `ScreenEngine::read_screen` | Baseline next-core capability. |
| `screen.scrollback_text` | Engine-neutral | `ScreenEngine::read_scrollback_text` plus active-session fallback | Active fallback currently uses engine sessions; OK. |
| `screen.cursor` | Engine-neutral | `ScreenEngine::cursor` | Baseline next-core capability. |
| `screen.scroll` | Engine-neutral | `ScreenEngine::read_lines` | Method name is read-only despite "scroll". |
| `screen.search` | Partial | `ScreenEngine::search`, optional GUI viewport jump still WezTerm-specific | Split read-only search from GUI jump capability. |
| `screen.detect_errors` | Engine-neutral | `ScreenEngine::read_screen` plus product heuristics | Product-only heuristic on engine snapshot. |

## Agent, Cockpit, Fleet, Review

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `agent.identify` | Product-only | Connection state | Engine-independent. |
| `agent.whoami` | Product-only | Connection state | Engine-independent. |
| `agent.list_trusted` | Product-only | Trust config/state | Engine-independent. |
| `agent.trust` | Product-only | Trust config/state | Engine-independent. |
| `agent.untrust` | Product-only | Trust config/state | Engine-independent. |
| `agent.status` | WezTerm-only | Cockpit scans pane/process/cwd through WezTerm paths | Needs engine-neutral pane metadata and agent state service. |
| `agent.signal` | WezTerm-only | Hook signal maps to pane ids and cockpit state | Product service can be engine-neutral once pane ids are canonical. |
| `cockpit.inbox` | WezTerm-only | Cockpit pane/window jump metadata | Needs engine-neutral instance/window/pane location model. |
| `fleet.launch` | WezTerm-only | Creates tabs and launches agents through current GUI/session path | Use `SessionEngine::create_session` plus launch context; keep worktree service product-only. |
| `fleet.list` | Product-only | Review/fleet registry | Engine-independent except live state enrichment. |
| `fleet.clean` | WezTerm-only | May close panes and clean worktrees | Split worktree cleanup from pane cleanup. |
| `fleet.retry` | WezTerm-only | Relaunches agent in an existing worktree tab | Use `SessionEngine::create_session` after extraction. |
| `review.list` | Product-only | Review registry and verification enrichment | Live pane enrichment should be optional. |
| `review.diff` | Product-only | Git/worktree diff | Engine-independent. |
| `review.verify` | Product-only | Verification process in worktree | Engine-independent. |
| `review.rollback` | Product-only | Git checkpoint restore | Destructive but engine-independent. |
| `review.merge` | Product-only | Git squash/stage | Engine-independent. |
| `review.discard` | Product-only | Review registry | Engine-independent. |

## Orchestration, Workspace, Ghost

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `ghost.debug` | WezTerm-only | Ghost text predictor tied to current pane/UI state | Extract predictor state by pane id. |
| `orchestrate.launch` | WezTerm-only | Session/tab creation through current GUI assumptions | Rebuild on `SessionEngine::create_session`. |
| `orchestrate.broadcast` | WezTerm-only | Resolves panes and writes commands | Rebuild on pane-id input path. |
| `orchestrate.wait` | WezTerm-only | Polls visible text through WezTerm pane helpers | Rebuild on `ScreenEngine::read_visible_text`. |
| `workspace.save` | WezTerm-only | Enumerates live panes/cwd/title from WezTerm | Rebuild on `SessionEngine::list_sessions`. |
| `workspace.restore` | WezTerm-only | Opens tabs through WezTerm session path | Rebuild on `SessionEngine::create_session`; archive remains product-only. |
| `workspace.list` | WezTerm-only | Currently lives in handler workspace helpers | Can become product-only once live enrichment is split. |

## Capture, Upload, System, Instance

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `capture.screen` | WezTerm-only | Platform/window capture helpers | Needs `CaptureEngine` or product capture service. |
| `capture.window` | WezTerm-only | Platform window enumeration/capture | Product-level platform service, not terminal core. |
| `capture.select` | WezTerm-only | Interactive GUI selection | Needs GUI/capture boundary. |
| `capture.clipboard` | WezTerm-only | Platform clipboard snapshot | Product-level platform service. |
| `capture.scrollback` | WezTerm-only | Headless scrollback renderer tied to current rendering stack | Needs `CaptureEngine::render_scrollback` fed by screen model. |
| `capture.window_scroll` | WezTerm-only | Platform app scrolling/stitching | Product-level platform service. |
| `upload.file` | Product-only | Upload config and local file IO | Engine-independent. |
| `system.info` | Product-only | OS/env/server metadata | Adds engine label; no terminal dependency. |
| `system.launch_admin` | WezTerm-only | Relaunches GUI/platform executable | Product-level platform service. |
| `instance.list` | WezTerm-only | Runtime instance files plus liveness/focus assumptions | Split registry from GUI focus/window ownership. |
| `instance.info` | WezTerm-only | Current instance metadata | Can become product-only if engine exposes label only. |
| `instance.set_title` | WezTerm-only | Window title override | Needs `WindowEngine`. |
| `instance.focus` | WezTerm-only | Platform window focus | Needs `WindowEngine`. |

## Proxy, Profile, Policy, Governance

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `proxy.status` | Product-only | OS/config proxy service | Engine-independent. |
| `proxy.nodes` | Product-only | Proxy config | Engine-independent. |
| `proxy.switch` | Product-only | Proxy config/service | Engine-independent. |
| `proxy.speedtest` | Product-only | Proxy service | Engine-independent. |
| `proxy.configure` | Product-only | OS proxy service | Engine-independent. |
| `proxy.disable` | Product-only | OS proxy service | Engine-independent. |
| `proxy.env` | Product-only | Proxy config/env formatting | Engine-independent. |
| `proxy.rotation` | Product-only | Proxy rotation state | Engine-independent. |
| `proxy.set_nodes` | Product-only | Proxy config | Engine-independent. |
| `proxy.clash_status` | Product-only | Clash controller HTTP API | Engine-independent. |
| `proxy.clash_select` | Product-only | Clash controller HTTP API | Engine-independent. |
| `proxy.clash_set_controller` | Product-only | Clash controller config | Engine-independent. |
| `policy.set` | Product-only | Policy config | Engine-independent. |
| `policy.check` | Product-only | Policy checker | Engine-independent. |
| `server.info` | Engine-neutral | Server metadata plus engine label | Already reports selected engine. |
| `server.health` | Partial | Server metadata plus engine label; health still mostly current runtime | Add engine-specific readiness checks. |
| `server.capabilities` | Product-only | `MCP_METHODS` inventory | Should later include per-engine support flags. |
| `selftest.run` | Product-only | MCP selftest orchestration | Needs per-engine test matrix. |
| `profile.list` | Product-only | Profile registry, no secrets | Engine-independent. |
| `profile.current` | Product-only | Current profile metadata | Engine-independent. |
| `profile.audit` | Product-only | Profile registry/vault metadata | Engine-independent. |
| `meta.surface` | Product-only | Static inventory + live keybindings | Needs per-engine capabilities before beta. |

## Next Extraction Targets

### Target 1: Pane-id write gate

Methods unlocked:

- `session.input`
- `session.paste`
- `exec.run`
- `exec.send`
- `exec.cancel`
- `signal.send`
- `orchestrate.broadcast`

Work:

- Replace `gate_pty_write(method, &Pane, input)` with a pane-id based gate.
- Keep audit output identical.
- Preserve existing confirmation banner behavior in WezTerm mode.
- Make `next-core` input go through the same policy path.

Acceptance:

- `session.input` in `UNTERM_ENGINE=next-core` no longer bypasses write confirmation.
- `cargo test -p unterm mcp::handler::tests -- --test-threads=1` passes or targeted replacement tests exist.
- Existing WezTerm write confirmation behavior is unchanged.

### Target 2: Exec wait without `Pane`

Methods unlocked:

- `exec.run_wait`
- `orchestrate.wait`

Work:

- Add shell kind/name to engine snapshots if missing.
- Use `ScreenEngine::read_visible_text` instead of `read_pane_text(&Pane)`.
- Keep sentinel wrapping output-compatible.

Acceptance:

- `exec.run_wait` works through both engines for `cmd.exe`/PowerShell.
- Timeout path returns the same JSON shape.

### Target 3: Workspace on `SessionEngine`

Methods unlocked:

- `workspace.save`
- `workspace.restore`
- part of `fleet.launch`

Work:

- Save workspace from `SessionEngine::list_sessions`.
- Restore workspace through `SessionEngine::create_session`.
- Keep dry-run behavior product-only.

Acceptance:

- Workspace list/save/restore does not import WezTerm mux types.

### Target 4: Recording text path on `ScreenEngine`

Methods unlocked:

- `session.export_markdown`
- part of `session.recording_*`

Work:

- Implement one-shot markdown export from `ScreenEngine::read_scrollback_text`.
- Keep live stream recording as a later `RecordingEngine`.

Acceptance:

- `session.export_markdown` works in `next-core` for plain text scrollback.

## Maintenance Rule

When an MCP method moves from one status to another:

1. Update this document in the same PR.
2. Add or update a targeted test.
3. Confirm `meta.surface` still lists the method.
4. If behavior differs by engine, expose that through capabilities before public beta.
