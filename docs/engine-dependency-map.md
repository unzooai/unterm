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
- `RecordingEngine`
- `HealthEngine`
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
- recording lifecycle/export
- engine readiness

Known gaps:

- GUI viewport scrolling/jump
- pane resolution without WezTerm `Pane`
- PTY write confirmation without WezTerm pane object
- exec wait shell detection without WezTerm pane object
- active recording render parity
- styled scrollback PNG parity for `next-core`
- window capture/focus/title
- instance lifecycle ownership
- foreground argv/process-tree metadata beyond the launch process in `next-core`
- richer profile/proxy launch semantics beyond the current `CreateSessionRequest::env` overlay

## MCP Coverage Summary

| Category | Count | Methods |
|---|---:|---|
| Engine-neutral | 48 | `session.list`, `session.get`, `session.status`, `session.create`, `session.split`, `session.focus`, `session.input`, `session.paste`, `session.idle`, `session.cwd`, `session.env`, `session.history`, `session.resize`, `session.destroy`, `session.recording_start`, `session.recording_stop`, `session.recording_status`, `session.recording_attach_trace`, `session.export_markdown`, `screen.read`, `screen.text`, `screen.scrollback_text`, `screen.cursor`, `screen.search`, `screen.detect_errors`, `exec.run`, `exec.send`, `exec.run_wait`, `exec.status`, `exec.cancel`, `signal.send`, `orchestrate.launch`, `orchestrate.broadcast`, `orchestrate.wait`, `workspace.save`, `workspace.restore`, `screen.scroll`, `agent.status`, `agent.signal`, `cockpit.inbox`, `fleet.launch`, `fleet.retry`, `fleet.clean`, `capture.screen`, `capture.window`, `capture.scrollback`, `server.info`, `server.health` |
| Partial | 0 | |
| Product-only | 51 | `meta.surface`, `session.audit_log`, `session.suggest`, `session.suggest_status`, `session.suggest_cancel`, `session.suggest_list`, `agent.identify`, `agent.whoami`, `agent.list_trusted`, `agent.trust`, `agent.untrust`, `policy.set`, `policy.check`, `server.capabilities`, `profile.list`, `profile.current`, `profile.audit`, `fleet.list`, `review.list`, `review.diff`, `review.verify`, `review.rollback`, `review.merge`, `review.discard`, `proxy.status`, `proxy.nodes`, `proxy.switch`, `proxy.speedtest`, `proxy.configure`, `proxy.disable`, `proxy.env`, `proxy.rotation`, `proxy.set_nodes`, `proxy.clash_status`, `proxy.clash_select`, `proxy.clash_set_controller`, `upload.file`, `system.info`, `system.launch_admin`, `selftest.run`, `workspace.list`, `session.recording_list`, `session.recording_read`, `instance.list`, `instance.info`, `instance.set_title`, `instance.focus`, `ghost.debug`, `capture.clipboard`, `capture.select`, `capture.window_scroll` |
| WezTerm-only | 0 | |
| Unsupported stub | 1 | `session.set_env` |

The counts intentionally include aliases (`session.get` / `session.status`, `exec.send` via `session.input`) because `meta.surface` exposes them as separate public contracts. The current `MCP_METHODS` inventory contains 100 public methods, excluding `auth.login`.

## Session Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `session.list` | Engine-neutral | `SessionEngine::list_sessions` | Keep as baseline adapter test. |
| `session.create` | Engine-neutral | `SessionEngine::create_session` plus `CreateSessionRequest::env` launch overlay for profile/proxy env | Profile/proxy env can now cross the engine boundary; later expand this into a typed launch context for alpha. |
| `session.status` | Engine-neutral | `SessionEngine::get_session` | Alias of `session.get`. |
| `session.get` | Engine-neutral | `SessionEngine::get_session` | Keep output shape stable. |
| `session.split` | Engine-neutral | `SessionEngine::split_session` | `next-core` must decide split semantics before GUI alpha. |
| `session.focus` | Engine-neutral | `SessionEngine::focus_session` | Needs window focus semantics later for cross-instance jumps. |
| `session.input` | Engine-neutral | `InputEngine::write_input` plus pane-id based write gate; `next-core` records write count, bytes, and last write duration on the session activity snapshot | Confirmation, audit, and policy are shared by WezTerm and `next-core`. |
| `session.paste` | Engine-neutral | `InputEngine::paste_input` plus pane-id based write gate; `next-core` chunks large UTF-8 paste payloads, preserves bracketed paste markers, and records paste telemetry on the session activity snapshot | Expand paste telemetry to current-core after the WezTerm paste path exposes completion timing. |
| `session.resize` | Engine-neutral | `SessionEngine::resize_session`; WezTerm adapter owns GUI-layout resize rejection | Handler no longer resolves a WezTerm pane or Mux for resize policy. |
| `session.destroy` | Engine-neutral | `SessionEngine::destroy_session` | Handler resolves pane id without WezTerm pane access. |
| `session.idle` | Engine-neutral | `SessionEngine::activity`; `next-core` uses recent input/output timestamps, liveness, input metrics, and paste metrics; WezTerm reports `input: null` and `paste: null` | Add foreground child-process detection later for precise running-command names. |
| `session.cwd` | Engine-neutral | `SessionEngine::shell`; `next-core` updates cwd from OSC 7 shell-integration sequences | Add process-tree fallback later for shells that do not emit OSC 7. |
| `session.env` | Engine-neutral | `SessionEngine::shell` launch env key snapshot; values are redacted | `next-core` exposes launch env variable names; WezTerm mode reports unsupported because live pane env is not available. |
| `session.set_env` | Unsupported stub | Returns unsupported marker | Prefer launch-context env over mutating live shells. |
| `session.history` | Engine-neutral | `ScreenEngine::read_scrollback` | Rename eventually? It is scrollback, not shell history. |
| `session.audit_log` | Product-only | MCP in-memory audit state | Engine-independent. |
| `session.suggest` | Product-only | MCP suggestion queue plus `SessionEngine::get_session` target validation | Needs UI renderer support in `next-core`, but queue state is product-owned and handler no longer resolves a WezTerm pane. |
| `session.suggest_status` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.suggest_cancel` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.suggest_list` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.recording_start` | Engine-neutral | `RecordingEngine::start_recording` | WezTerm uses pane stream sink; `next-core` taps PTY reader output. |
| `session.recording_stop` | Engine-neutral | `RecordingEngine::stop_recording` | Both engines finalize log/index state. |
| `session.recording_status` | Engine-neutral | `RecordingEngine::recording_status` | Both engines report active state by pane id. |
| `session.recording_list` | Product-only | Recording archive index | No live terminal dependency. |
| `session.recording_read` | Product-only | Recording archive log renderer | No live terminal dependency. |
| `session.recording_attach_trace` | Engine-neutral | `RecordingEngine::attach_recording_trace` | Trace ids are stored in active recording state. |
| `session.export_markdown` | Engine-neutral | `RecordingEngine::export_markdown` for active recordings; `ScreenEngine::read_scrollback_text` for inactive snapshots | Active and inactive export no longer require handler access to WezTerm recorder or pane. |

## Exec and Signal Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `exec.run` | Engine-neutral | `InputEngine::write_input` plus pane-id based write gate | Preserves policy check and audit before sending command + CR. |
| `exec.send` | Engine-neutral | `InputEngine::write_input` plus pane-id based write gate | Accepts documented `bytes` plus `input`/`text` aliases. |
| `exec.run_wait` | Engine-neutral | `SessionEngine::shell`, `ScreenEngine::read_visible_text`, `InputEngine::write_input`, pane-id based write gate | Uses sentinel wrapping and keeps the previous output JSON shape. |
| `exec.status` | Engine-neutral | `SessionEngine::activity` | In `next-core`, status reflects recent I/O activity, liveness, input metrics, and paste metrics; foreground process is still the launch shell until process-tree tracking lands. |
| `exec.cancel` | Engine-neutral | `InputEngine::write_input` plus pane-id based write gate | Sends Ctrl+C after confirmation/audit. |
| `signal.send` | Engine-neutral | `InputEngine::write_input` plus pane-id based write gate | Validates supported signal before confirmation/audit. |

## Screen Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `screen.read` | Engine-neutral | `ScreenEngine::read_screen` | Baseline next-core capability. |
| `screen.text` | Engine-neutral | `ScreenEngine::read_screen` | Baseline next-core capability. |
| `screen.scrollback_text` | Engine-neutral | `ScreenEngine::read_scrollback_text` plus active-session fallback | Active fallback currently uses engine sessions; OK. |
| `screen.cursor` | Engine-neutral | `ScreenEngine::cursor` | Baseline next-core capability. |
| `screen.scroll` | Engine-neutral | `ScreenEngine::read_lines` | Method name is read-only despite "scroll". |
| `screen.search` | Engine-neutral | `ScreenEngine::search`; optional `goto` routes through `WindowEngine::scroll_viewport_to` | `next-core` returns matches and reports `goto_skipped` until it owns GUI viewport state. |
| `screen.detect_errors` | Engine-neutral | `ScreenEngine::read_screen` plus product heuristics | Product-only heuristic on engine snapshot. |

## Agent, Cockpit, Fleet, Review

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `agent.identify` | Product-only | Connection state | Engine-independent. |
| `agent.whoami` | Product-only | Connection state | Engine-independent. |
| `agent.list_trusted` | Product-only | Trust config/state | Engine-independent. |
| `agent.trust` | Product-only | Trust config/state | Engine-independent. |
| `agent.untrust` | Product-only | Trust config/state | Engine-independent. |
| `agent.status` | Engine-neutral | Cockpit registry lookup by pane id; all-pane snapshot from product state | Handler no longer resolves a WezTerm pane for single-pane status. |
| `agent.signal` | Engine-neutral | Explicit pane-id signals write directly to the cockpit registry; omitted pane id resolves through `WindowEngine::active_pane_id` | `next-core` resolves active session from engine session snapshots until it owns GUI focus state. |
| `cockpit.inbox` | Engine-neutral | Agent registry joined with `SessionEngine::list_sessions`; optional tab/window jump metadata comes from `WindowEngine::pane_locations` | `next-core` returns synthetic window/tab locations from its session registry until it owns real GUI tabs/windows. |
| `fleet.launch` | Engine-neutral | Fleet worktree registry plus `SessionEngine::create_session` and `InputEngine::write_input` via a pane spawner | Handler launches members without calling WezTerm tab APIs; default GUI fleet launcher still uses the WezTerm spawner. |
| `fleet.list` | Product-only | Review/fleet registry | Engine-independent except live state enrichment. |
| `fleet.clean` | Engine-neutral | Product worktree/branch cleanup plus engine-backed pane remover | Handler cleans fleets without calling WezTerm Mux. |
| `fleet.retry` | Engine-neutral | Existing fleet worktree validation plus engine-backed pane remover/spawner | Handler retries members without calling WezTerm Mux. |
| `review.list` | Product-only | Review registry and verification enrichment | Live pane enrichment should be optional. |
| `review.diff` | Product-only | Git/worktree diff | Engine-independent. |
| `review.verify` | Product-only | Verification process in worktree | Engine-independent. |
| `review.rollback` | Product-only | Git checkpoint restore | Destructive but engine-independent. |
| `review.merge` | Product-only | Git squash/stage | Engine-independent. |
| `review.discard` | Product-only | Review registry | Engine-independent. |

## Orchestration, Workspace, Ghost

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `ghost.debug` | Product-only | Ghost text predictor registry keyed by pane id | Engine-independent read-only diagnostic state. |
| `orchestrate.launch` | Engine-neutral | `SessionEngine::create_session`, pane-id write gate, `InputEngine::write_input` | Optional command now goes through policy/confirmation/audit. |
| `orchestrate.broadcast` | Engine-neutral | `SessionEngine::get_session`, pane-id write gate, `InputEngine::write_input` | Per-session result shape is preserved. |
| `orchestrate.wait` | Engine-neutral | `SessionEngine::get_session`, `ScreenEngine::read_visible_text` | Timeout result shape is preserved. |
| `workspace.save` | Engine-neutral | `SessionEngine::list_sessions` plus workspace file write | Saves id/title/cwd from engine snapshots. |
| `workspace.restore` | Engine-neutral | Workspace file read plus `SessionEngine::create_session` through `session_create` | Dry-run and archive handling remain product-layer behavior. |
| `workspace.list` | Product-only | Workspace archive directory read | No live terminal dependency. |

## Capture, Upload, System, Instance

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `capture.screen` | Engine-neutral | Text snapshot via `SessionEngine::list_sessions`/`ScreenEngine::read_visible_text`; image via `CaptureEngine::capture_screen_image` | Platform pixels are behind the capture boundary; next-core can reuse the same product capture service. |
| `capture.window` | Engine-neutral | Terminal text match via `SessionEngine::list_sessions`/`ScreenEngine::read_visible_text`; image via `CaptureEngine::capture_window_image` | Platform pixels are behind the capture boundary; next-core can reuse the same product capture service. |
| `capture.select` | Product-only | Platform screen capture fallback for headless MCP | Interactive region selection remains a GUI concern, but the public MCP method no longer requires terminal core. |
| `capture.clipboard` | Product-only | Platform clipboard snapshot | Engine-independent platform service. |
| `capture.scrollback` | Engine-neutral | `CaptureEngine::render_scrollback_png`; WezTerm adapter renders styled pane cells, `next-core` renders a plain-text PNG from `ScreenEngine::read_scrollback_text` | Upgrade `next-core` to styled-cell rendering after styled scrollback snapshots are exposed. |
| `capture.window_scroll` | Product-only | Platform app scrolling/stitching | Product-level platform service; currently macOS-only and engine-independent. |
| `upload.file` | Product-only | Upload config and local file IO | Engine-independent. |
| `system.info` | Product-only | OS/env/server metadata plus `SessionEngine::list_sessions` count | Adds engine label without direct WezTerm mux access. |
| `system.launch_admin` | Product-only | Platform executable relaunch command | Windows UAC launcher; dry-run path is engine-independent. |
| `instance.list` | Product-only | Runtime instance registry files plus PID liveness filtering | GUI focus/window ownership remains separate in `instance.focus`. |
| `instance.info` | Product-only | Current process instance metadata from server-info registry | Engine-independent. |
| `instance.set_title` | Product-only | Server-info title override registry | Current MCP path only updates product metadata; live GUI title application remains separate. |
| `instance.focus` | Product-only | `WindowEngine::focus_current_instance_window` platform/window boundary | Handler no longer reaches into the WezTerm frontend; next-core can reuse the same window service. |

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
| `server.health` | Engine-neutral | `HealthEngine::health` plus product server metadata | WezTerm readiness is adapter-owned; `next-core` readiness does not depend on WezTerm Mux state. |
| `server.capabilities` | Product-only | `MCP_METHODS` inventory plus `_engine_capabilities` | Keeps the legacy namespace map while exposing selected engine support/unsupported method flags. |
| `selftest.run` | Product-only | MCP selftest orchestration plus `HealthEngine`/`SessionEngine` probes | Selftest no longer treats WezTerm mux availability as the engine readiness source. Needs broader per-engine test matrix. |
| `profile.list` | Product-only | Profile registry, no secrets | Engine-independent. |
| `profile.current` | Product-only | Current profile metadata | Engine-independent. |
| `profile.audit` | Product-only | Profile registry/vault metadata | Engine-independent. |
| `meta.surface` | Product-only | Static inventory + live keybindings + selected engine capability flags | Agents can detect current engine support without guessing from docs. |

## Next Extraction Targets

### Target 1: Pane-id write gate

Methods unlocked:

- `orchestrate.broadcast`

Work:

- Continue replacing WezTerm `Pane` write paths with pane-id based gate calls.
- Keep audit output identical.
- Preserve existing confirmation banner behavior in WezTerm mode.
- Keep `next-core` writes on the same policy path.

Acceptance:

- Orchestration writes no longer require a WezTerm `Pane`.
- `cargo test -p unterm mcp::handler::tests -- --test-threads=1` passes or targeted replacement tests exist.
- Existing `session.input` / `session.paste` confirmation behavior is preserved; `exec.*` and `signal.send` use the same write boundary.

### Target 2: Recording text path on `ScreenEngine`

Methods unlocked:

- part of future capture/export polish

Work:

- Keep one-shot markdown export on `ScreenEngine::read_scrollback_text`.
- Keep live stream recording and active export behind `RecordingEngine`.
- Keep raw PTY stream tap implemented in `next-core`.
- Move remaining render formatting differences into product-level recording services.

Acceptance:

- `session.export_markdown` works in `next-core` for inactive scrollback and active recording export.
- Recording lifecycle/export MCP methods call `RecordingEngine` rather than WezTerm helpers directly.
- Active recording state no longer depends on WezTerm pane storage.

## Maintenance Rule

When an MCP method moves from one status to another:

1. Update this document in the same PR.
2. Add or update a targeted test.
3. Confirm `meta.surface` still lists the method.
4. If behavior differs by engine, expose that through capabilities before public beta.
