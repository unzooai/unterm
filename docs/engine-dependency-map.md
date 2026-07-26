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
- next-core cols-aware screen wrapping, DECCKM application cursor key mode, DECAWM auto-wrap mode, DECOM origin mode for scroll-region-relative cursor positioning, DECSCNM reverse-video mode, IRM insert mode, combined mode set/reset handling, HT/CHT/HTS/TBC/CBT tab-stop cursor movement/control, ESC charset/UTF-8 designator consumption, DCS/APC/PM/SOS control-string consumption, C1 CSI/OSC/string-control and IND/NEL/RI handling, DECALN alignment-test fill, ESC IND/NEL/RI scroll-region movement, DECSTR/RIS terminal reset, IL/DL line mutation within scroll regions, CSI CNL/CPL/HPA/HPR/VPA/VPR positioning, CSI/DEC private save/restore cursor, erase-line/erase-character styled blank backfill, delete-character right-margin blank backfill, REP repeat-character handling, CSI 3J scrollback clearing, SGR bold/faint/italic/underline-style/underline-color/strikethrough/hidden/overline/blink/vertical-align/inverse styles, semicolon/colon SGR extended-color parsing, and resize truncation
- visible text read
- line/scrollback reads
- scrollback text export
- search
- cursor
- write input
- paste input
- PTY write confirmation diagnostics without WezTerm pane-object ownership
- recording lifecycle/export with next-core chunked-output fallback and OSC133 command-block markdown
- validated explicit `capture.scrollback` pane ids through the shared session resolver
- redacted `session.create` launch decision summary for profile/proxy/overlay/command provenance
- redacted `workspace.restore` template launch plan for cwd/profile/command provenance
- redacted default-shell launch decision summary for command-less `session.create`
- typed launch policy decision metadata for domain, privilege, proxy rotation, and restart handling, including explicit `session.create` request diagnostics
- instance lifecycle ownership diagnostics for server-info registry vs host GUI window ownership
- instance title bridge result metadata through `WindowEngine`
- instance registry cleanup, active-pointer diagnostics, shutdown dry-run lifecycle planning, and protected registry unregister execution
- styled scrollback renderer metadata for WezTerm pane rendering vs next-core standalone rendering
- configured theme palette resolution for next-core standalone styled scrollback PNG rendering
- bold/italic font matching for next-core standalone styled scrollback PNG rendering
- engine readiness and next-core aggregate I/O health counters
- next-core terminal status, cursor-position, DEC private cursor-position, and primary/secondary device-attribute query responses, including parameterized DA forms, through the PTY writer in input order

Known gaps:

- real GUI viewport scrolling/jump for the future next-core renderer
- native window capture/focus/title ownership beyond the explicit host-window and title bridge contracts
- native instance create/close execution beyond server-info registry cleanup, active-pointer observability, shutdown dry-run planning, and protected registry unregister
- enforcement of non-local domain, privilege elevation, proxy rotation, and restart launch policy behavior beyond current typed explicit-request decision metadata

## MCP Coverage Summary

| Category | Count | Methods |
|---|---:|---|
| Engine-neutral | 48 | `session.list`, `session.get`, `session.status`, `session.create`, `session.split`, `session.focus`, `session.input`, `session.paste`, `session.idle`, `session.cwd`, `session.env`, `session.history`, `session.resize`, `session.destroy`, `session.recording_start`, `session.recording_stop`, `session.recording_status`, `session.recording_attach_trace`, `session.export_markdown`, `screen.read`, `screen.text`, `screen.scrollback_text`, `screen.cursor`, `screen.search`, `screen.detect_errors`, `exec.run`, `exec.send`, `exec.run_wait`, `exec.status`, `exec.cancel`, `signal.send`, `orchestrate.launch`, `orchestrate.broadcast`, `orchestrate.wait`, `workspace.save`, `workspace.restore`, `screen.scroll`, `agent.status`, `agent.signal`, `cockpit.inbox`, `fleet.launch`, `fleet.retry`, `fleet.clean`, `capture.screen`, `capture.window`, `capture.scrollback`, `server.info`, `server.health` |
| Partial | 0 | |
| Product-only | 54 | `meta.surface`, `session.audit_log`, `session.set_env`, `session.suggest`, `session.suggest_status`, `session.suggest_cancel`, `session.suggest_list`, `agent.identify`, `agent.whoami`, `agent.list_trusted`, `agent.trust`, `agent.untrust`, `policy.set`, `policy.check`, `server.capabilities`, `profile.list`, `profile.current`, `profile.audit`, `fleet.list`, `review.list`, `review.diff`, `review.verify`, `review.rollback`, `review.merge`, `review.discard`, `proxy.status`, `proxy.nodes`, `proxy.switch`, `proxy.speedtest`, `proxy.configure`, `proxy.disable`, `proxy.env`, `proxy.rotation`, `proxy.set_nodes`, `proxy.clash_status`, `proxy.clash_select`, `proxy.clash_set_controller`, `upload.file`, `system.info`, `system.launch_admin`, `selftest.run`, `workspace.list`, `session.recording_list`, `session.recording_read`, `instance.list`, `instance.info`, `instance.lifecycle`, `instance.close`, `instance.set_title`, `instance.focus`, `ghost.debug`, `capture.clipboard`, `capture.select`, `capture.window_scroll` |
| WezTerm-only | 0 | |
| Unsupported stub | 0 | |

The counts intentionally include aliases (`session.get` / `session.status`, `exec.send` via `session.input`) because `meta.surface` exposes them as separate public contracts. The current `MCP_METHODS` inventory contains 102 public methods, excluding `auth.login`.

## Session Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `session.list` | Engine-neutral | `SessionEngine::list_sessions` | Keep as baseline adapter test. |
| `session.create` | Engine-neutral | `SessionEngine::create_session` plus `CreateSessionRequest::env` and typed `launch_policy`; `next-core` records `ShellSnapshot.launch_context` profile/proxy diagnostics, env provenance, explicit-command/default-shell decisions, explicit domain/privilege/proxy-rotation/restart request decisions, and no secret values | Later enforce non-local domain, privilege elevation, proxy rotation, and restart behavior instead of only reporting the decision state. |
| `session.status` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::get_session` | Alias of `session.get`. |
| `session.get` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::get_session` | Keep output shape stable. |
| `session.split` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::split_session` | `next-core` must decide split semantics before GUI alpha. |
| `session.focus` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::focus_session` | Needs window focus semantics later for cross-instance jumps. |
| `session.input` | Engine-neutral | Shared pane-id resolver, `InputEngine::write_input`, and pane-id based write gate; `next-core` records write count, bytes, and last write duration on the session activity snapshot | Confirmation, audit, and policy are shared by WezTerm and `next-core`. |
| `session.paste` | Engine-neutral | Shared pane-id resolver, `InputEngine::paste_input`, and pane-id based write gate; `next-core` chunks large UTF-8 paste payloads, preserves bracketed paste markers, and records paste telemetry on the session activity snapshot | Expand paste telemetry to current-core after the WezTerm paste path exposes completion timing. |
| `session.resize` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::resize_session`; WezTerm adapter owns GUI-layout resize rejection | Handler no longer resolves a WezTerm pane or Mux for resize policy. |
| `session.destroy` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::destroy_session` | Handler resolves pane id without WezTerm pane access. |
| `session.idle` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::activity`; `next-core` uses recent input/output timestamps, liveness, input metrics, output metrics, paste metrics, and a process-tree activity summary with root/foreground pid, cwd, argv, child count, and known agent detection; WezTerm reports `process: null`, `input: null`, `output: null`, and `paste: null` | Keep process-tree scans off UI paint paths; query only from explicit status/cwd calls. |
| `session.cwd` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::shell`; `next-core` updates cwd from OSC 7 shell-integration sequences and falls back to foreground/root process cwd when shell integration is unavailable | OSC 7 remains preferred because it follows shell-level directory changes immediately. |
| `session.env` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::shell` launch env key snapshot and `launch_context`; values are redacted | `next-core` exposes launch env variable names, selected profile id, proxy env key names, env key count, and typed policy provenance for proxy/profile/overlay/explicit env sources; WezTerm mode reports unsupported because live pane env is not available. |
| `session.set_env` | Product-only | MCP launch env overlay for future `session.create`; existing shells are not mutated | `next-core` supports future-launch env overlays. WezTerm mode still reports unsupported because live pane env mutation is not available and current-core launch ownership remains WezTerm-bound. |
| `session.history` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::read_scrollback` | Rename eventually? It is scrollback, not shell history. |
| `session.audit_log` | Product-only | MCP in-memory audit state | Engine-independent. |
| `session.suggest` | Product-only | MCP suggestion queue plus shared pane-id resolver target validation | Needs UI renderer support in `next-core`, but queue state is product-owned and handler no longer resolves a WezTerm pane. |
| `session.suggest_status` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.suggest_cancel` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.suggest_list` | Product-only | MCP suggestion queue | Engine-independent. |
| `session.recording_start` | Engine-neutral | Shared pane-id resolver plus `RecordingEngine::start_recording` | WezTerm uses pane stream sink; `next-core` taps PTY reader output. |
| `session.recording_stop` | Engine-neutral | Shared pane-id resolver plus `RecordingEngine::stop_recording` | Both engines finalize log/index state; `next-core` now writes YAML-fronted redacted markdown for active recordings. |
| `session.recording_status` | Engine-neutral | Shared pane-id resolver plus `RecordingEngine::recording_status` | Both engines report active state by pane id. |
| `session.recording_list` | Product-only | Recording archive index | No live terminal dependency. |
| `session.recording_read` | Product-only | Recording archive log renderer | No live terminal dependency. |
| `session.recording_attach_trace` | Engine-neutral | Shared pane-id resolver plus `RecordingEngine::attach_recording_trace` | Trace ids are stored in active recording state. |
| `session.export_markdown` | Engine-neutral | Shared pane-id resolver plus `RecordingEngine::export_markdown` for active recordings; `ScreenEngine::read_scrollback_text` for inactive snapshots | Active and inactive export no longer require handler access to WezTerm recorder or pane. `next-core` active exports include front matter, trace ids, block/byte counts, OSC133 command blocks when shell markers are present, ANSI stripping, and basic redaction. |

## Exec and Signal Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `exec.run` | Engine-neutral | Shared pane-id resolver, `InputEngine::write_input`, and pane-id based write gate | Preserves policy check and audit before sending command + CR. |
| `exec.send` | Engine-neutral | Shared pane-id resolver, `InputEngine::write_input`, and pane-id based write gate | Accepts documented `bytes` plus `input`/`text` aliases. |
| `exec.run_wait` | Engine-neutral | Shared pane-id resolver, `SessionEngine::shell`, `SessionEngine::activity`, `ScreenEngine::read_visible_text`, `InputEngine::write_input`, pane-id based write gate | Uses sentinel wrapping and resolves shell syntax from engine-neutral shell metadata, next-core process-tree root/foreground summaries, and platform fallback without reaching through a WezTerm pane. |
| `exec.status` | Engine-neutral | Shared pane-id resolver plus `SessionEngine::activity` | In `next-core`, status reflects recent I/O activity, liveness, input metrics, output metrics, paste metrics, and process-tree foreground/agent diagnostics. |
| `exec.cancel` | Engine-neutral | Shared pane-id resolver, `InputEngine::write_input`, and pane-id based write gate | Sends Ctrl+C after confirmation/audit. |
| `signal.send` | Engine-neutral | Shared pane-id resolver, `InputEngine::write_input`, and pane-id based write gate | Validates supported signal before confirmation/audit. |

## Screen Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `screen.read` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::read_screen` | Baseline next-core capability. |
| `screen.text` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::read_screen` | Baseline next-core capability. |
| `screen.scrollback_text` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::read_scrollback_text` with missing/stale-id active-session fallback | Active fallback resolves through `WindowEngine::active_pane_id`, which maps to next-core active session snapshots outside WezTerm. |
| `screen.cursor` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::cursor` | Baseline next-core capability. |
| `screen.scroll` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::read_lines`; optional `goto`/`apply` routes through `WindowEngine::scroll_viewport_to` | Default remains read-only. With `goto`/`apply`, `next-core` updates its logical viewport and WezTerm updates the GUI viewport. |
| `screen.search` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::search`; optional `goto` routes through `WindowEngine::scroll_viewport_to` | `next-core` updates its logical viewport so later `screen.read`/`screen.text` calls show the matched region; real GUI viewport integration comes with the next-core renderer. |
| `screen.detect_errors` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::read_screen` and product heuristics | Product-only heuristic on engine snapshot. |

## Agent, Cockpit, Fleet, Review

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `agent.identify` | Product-only | Connection state | Engine-independent. |
| `agent.whoami` | Product-only | Connection state | Engine-independent. |
| `agent.list_trusted` | Product-only | Trust config/state | Engine-independent. |
| `agent.trust` | Product-only | Trust config/state | Engine-independent. |
| `agent.untrust` | Product-only | Trust config/state | Engine-independent. |
| `agent.status` | Engine-neutral | Cockpit registry lookup by pane id; all-pane snapshot from product state | Handler no longer resolves a WezTerm pane for single-pane status. |
| `agent.signal` | Engine-neutral | Shared pane-id resolver; explicit pane ids are validated as live sessions, omitted pane id resolves through `WindowEngine::active_pane_id` | `next-core` resolves active session from engine session snapshots until it owns GUI focus state. |
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
| `orchestrate.wait` | Engine-neutral | Shared pane-id resolver plus `ScreenEngine::read_visible_text` | Timeout result shape is preserved. |
| `workspace.save` | Engine-neutral | `SessionEngine::list_sessions` plus workspace file write | Saves id/title/cwd from engine snapshots. |
| `workspace.restore` | Engine-neutral | Workspace file read plus `SessionEngine::create_session` through `session_create`; dry-run exposes redacted workspace-template launch decisions | Restore reuses the same launch path as `session.create`, including created-session launch decisions; archive handling remains product-layer behavior. |
| `workspace.list` | Product-only | Workspace archive directory read | No live terminal dependency. |

## Capture, Upload, System, Instance

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `capture.screen` | Engine-neutral | Text snapshot via `SessionEngine::list_sessions`/`ScreenEngine::read_visible_text`; image via `CaptureEngine::capture_screen_image` | Platform pixels are behind the capture boundary; next-core can reuse the same product capture service. |
| `capture.window` | Engine-neutral | Terminal text match via `SessionEngine::list_sessions`/`ScreenEngine::read_visible_text`; image via `CaptureEngine::capture_window_image` | Platform pixels are behind the capture boundary; next-core can reuse the same product capture service. |
| `capture.select` | Product-only | Platform screen capture fallback for headless MCP | Interactive region selection remains a GUI concern, but the public MCP method no longer requires terminal core. |
| `capture.clipboard` | Product-only | Platform clipboard snapshot | Engine-independent platform service. |
| `capture.scrollback` | Engine-neutral | `CaptureEngine::render_scrollback_png`; WezTerm adapter renders styled pane cells, `next-core` reads `ScreenEngine::read_styled_scrollback` and renders cell-level foreground/background/inverse/underline/bold/italic styles with configured theme palette resolution and without WezTerm pane access; capability surfaces expose `diagnostics.styled_scrollback_png` and renderer parity metadata | Still uses a standalone headless renderer until next-core owns the real GUI renderer. |
| `capture.window_scroll` | Product-only | Platform app scrolling/stitching | Product-level platform service; currently macOS-only and engine-independent. |
| `upload.file` | Product-only | Upload config and local file IO | Engine-independent. |
| `system.info` | Product-only | OS/env/server metadata plus `SessionEngine::list_sessions` count | Adds engine label without direct WezTerm mux access. |
| `system.launch_admin` | Product-only | Platform executable relaunch command | Windows UAC launcher; dry-run path is engine-independent. |
| `instance.list` | Product-only | Runtime instance registry files plus PID liveness filtering, cleanup counters, active-pointer diagnostics, and lifecycle ownership diagnostics | GUI focus/window ownership remains separate in `instance.focus`; registry ownership and cleanup side effects are now explicit for agents. |
| `instance.info` | Product-only | Current process instance metadata from server-info registry plus lifecycle ownership diagnostics | Engine-independent; native window lifecycle is still host-owned. |
| `instance.lifecycle` | Product-only | Server-info registration state plus shutdown dry-run plan for registry removal, active-pointer handoff, and legacy server pointer update | Read-only by design; native window close execution remains host-owned until next-core owns windows. |
| `instance.close` | Product-only | Protected server-info registry unregister execution with dry-run default and explicit confirmation for apply | Does not close the native window yet; it is the product-layer hook that future host/native close lifecycle will call. |
| `instance.set_title` | Product-only | `WindowEngine::set_current_instance_title` writes server-info title override registry metadata and returns title/native-window ownership diagnostics | Current MCP path is engine-neutral and reports that live GUI title application remains host-owned. |
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
| `server.health` | Engine-neutral | `HealthEngine::health` plus product server metadata | WezTerm readiness is adapter-owned; `next-core` readiness does not depend on WezTerm Mux state and includes aggregate input/output/paste health counters. |
| `server.capabilities` | Product-only | `MCP_METHODS` inventory plus `_engine_capabilities` | Keeps the legacy namespace map while exposing selected engine support/unsupported method flags and diagnostic capability flags such as next-core health I/O summaries, launch-context diagnostics, and styled scrollback PNG availability. |
| `selftest.run` | Product-only | MCP selftest orchestration plus `HealthEngine`/`SessionEngine` probes | Selftest no longer treats WezTerm mux availability as the engine readiness source and verifies next-core health I/O diagnostics, launch-context profile/proxy redaction, typed launch policy provenance and decision metadata, logical viewport scrolling, and styled scrollback PNG capture when that engine is selected. Needs broader per-engine test matrix. |
| `profile.list` | Product-only | Profile registry, no secrets | Engine-independent. |
| `profile.current` | Product-only | Current profile metadata | Engine-independent. |
| `profile.audit` | Product-only | Profile registry/vault metadata | Engine-independent. |
| `meta.surface` | Product-only | Static inventory + live keybindings + selected engine capability/diagnostic flags | Agents can detect current engine support and diagnostic surfaces without guessing from docs. |

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
- Keep OSC133 command block parsing behind `RecordingEngine` so product services can consume command-aware markdown without touching a pane.

Acceptance:

- `session.export_markdown` works in `next-core` for inactive scrollback and active recording export with markdown front matter/redaction plus OSC133 command blocks when markers are present.
- Recording lifecycle/export MCP methods call `RecordingEngine` rather than WezTerm helpers directly.
- Active recording state no longer depends on WezTerm pane storage.

## Maintenance Rule

When an MCP method moves from one status to another:

1. Update this document in the same PR.
2. Add or update a targeted test.
3. Confirm `meta.surface` still lists the method.
4. If behavior differs by engine, expose that through capabilities before public beta.
