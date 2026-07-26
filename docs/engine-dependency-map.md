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
| Engine-neutral | 35 | `session.list`, `session.get`, `session.status`, `session.create`, `session.split`, `session.focus`, `session.input`, `session.paste`, `session.idle`, `session.cwd`, `session.history`, `session.destroy`, `session.recording_start`, `session.recording_stop`, `session.recording_status`, `session.recording_attach_trace`, `screen.read`, `screen.text`, `screen.scrollback_text`, `screen.cursor`, `screen.search`, `screen.detect_errors`, `exec.run`, `exec.send`, `exec.run_wait`, `exec.status`, `exec.cancel`, `signal.send`, `orchestrate.launch`, `orchestrate.broadcast`, `orchestrate.wait`, `workspace.save`, `workspace.restore`, `screen.scroll`, `server.info` |
| Partial | 3 | `session.resize`, `session.export_markdown`, `server.health` |
| Product-only | 42 | `meta.surface`, `session.audit_log`, `session.suggest`, `session.suggest_status`, `session.suggest_cancel`, `session.suggest_list`, `agent.identify`, `agent.whoami`, `agent.list_trusted`, `agent.trust`, `agent.untrust`, `policy.set`, `policy.check`, `server.capabilities`, `profile.list`, `profile.current`, `profile.audit`, `fleet.list`, `review.list`, `review.diff`, `review.verify`, `review.rollback`, `review.merge`, `review.discard`, `proxy.status`, `proxy.nodes`, `proxy.switch`, `proxy.speedtest`, `proxy.configure`, `proxy.disable`, `proxy.env`, `proxy.rotation`, `proxy.set_nodes`, `proxy.clash_status`, `proxy.clash_select`, `proxy.clash_set_controller`, `upload.file`, `system.info`, `selftest.run`, `workspace.list`, `session.recording_list`, `session.recording_read` |
| WezTerm-only | 18 | `agent.status`, `agent.signal`, `cockpit.inbox`, `fleet.launch`, `fleet.clean`, `fleet.retry`, `ghost.debug`, `capture.screen`, `capture.window`, `capture.select`, `capture.clipboard`, `capture.scrollback`, `capture.window_scroll`, `instance.list`, `instance.info`, `instance.set_title`, `instance.focus`, `system.launch_admin` |
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
| `session.input` | Engine-neutral | `InputEngine::write_input` plus pane-id based write gate | Confirmation, audit, and policy are shared by WezTerm and `next-core`. |
| `session.paste` | Engine-neutral | `InputEngine::paste_input` plus pane-id based write gate | Add paste-size and bracketed-paste semantics to trait before next-core alpha. |
| `session.resize` | Partial | `SessionEngine::resize_session`, but detects GUI layout through WezTerm `Mux` | Add engine capability for resize policy/layout ownership. |
| `session.destroy` | Engine-neutral | `SessionEngine::destroy_session` | Handler resolves pane id without WezTerm pane access. |
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
| `session.recording_start` | Engine-neutral | `RecordingEngine::start_recording` | WezTerm uses pane stream sink; `next-core` taps PTY reader output. |
| `session.recording_stop` | Engine-neutral | `RecordingEngine::stop_recording` | Both engines finalize log/index state. |
| `session.recording_status` | Engine-neutral | `RecordingEngine::recording_status` | Both engines report active state by pane id. |
| `session.recording_list` | Product-only | Recording archive index | No live terminal dependency. |
| `session.recording_read` | Product-only | Recording archive log renderer | No live terminal dependency. |
| `session.recording_attach_trace` | Engine-neutral | `RecordingEngine::attach_recording_trace` | Trace ids are stored in active recording state. |
| `session.export_markdown` | Partial | Inactive export uses `ScreenEngine::read_scrollback_text`; active recording still renders from WezTerm-backed recorder log | Finish by extracting recording registry/stream tap behind a `RecordingEngine`. |

## Exec and Signal Methods

| Method | Status | Current dependency | Migration note |
|---|---|---|---|
| `exec.run` | Engine-neutral | `InputEngine::write_input` plus pane-id based write gate | Preserves policy check and audit before sending command + CR. |
| `exec.send` | Engine-neutral | `InputEngine::write_input` plus pane-id based write gate | Accepts documented `bytes` plus `input`/`text` aliases. |
| `exec.run_wait` | Engine-neutral | `SessionEngine::shell`, `ScreenEngine::read_visible_text`, `InputEngine::write_input`, pane-id based write gate | Uses sentinel wrapping and keeps the previous output JSON shape. |
| `exec.status` | Engine-neutral | `SessionEngine::activity` | Good next-core smoke candidate. |
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
| `screen.search` | Engine-neutral | `ScreenEngine::search`; optional `goto` only applies when current engine owns a WezTerm GUI viewport | `next-core` returns matches and reports `goto_skipped` instead of reaching into WezTerm Mux/window state. |
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
| `orchestrate.launch` | Engine-neutral | `SessionEngine::create_session`, pane-id write gate, `InputEngine::write_input` | Optional command now goes through policy/confirmation/audit. |
| `orchestrate.broadcast` | Engine-neutral | `SessionEngine::get_session`, pane-id write gate, `InputEngine::write_input` | Per-session result shape is preserved. |
| `orchestrate.wait` | Engine-neutral | `SessionEngine::get_session`, `ScreenEngine::read_visible_text` | Timeout result shape is preserved. |
| `workspace.save` | Engine-neutral | `SessionEngine::list_sessions` plus workspace file write | Saves id/title/cwd from engine snapshots. |
| `workspace.restore` | Engine-neutral | Workspace file read plus `SessionEngine::create_session` through `session_create` | Dry-run and archive handling remain product-layer behavior. |
| `workspace.list` | Product-only | Workspace archive directory read | No live terminal dependency. |

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

- `session.export_markdown`
- part of `session.recording_*`

Work:

- Keep one-shot markdown export on `ScreenEngine::read_scrollback_text`.
- Route live stream recording through `RecordingEngine`.
- Keep raw PTY stream tap implemented in `next-core`.

Acceptance:

- `session.export_markdown` works in `next-core` for plain text scrollback.
- Recording lifecycle MCP methods call `RecordingEngine` rather than WezTerm helpers directly.
- Active recording state no longer depends on WezTerm pane storage.

## Maintenance Rule

When an MCP method moves from one status to another:

1. Update this document in the same PR.
2. Add or update a targeted test.
3. Confirm `meta.surface` still lists the method.
4. If behavior differs by engine, expose that through capabilities before public beta.
