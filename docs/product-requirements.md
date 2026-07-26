# Unterm Product Requirements Document

Status: living product spec  
Owner: product / engineering  
Last updated: 2026-07-26  
Source of truth inputs: `README.md`, `USAGE.md`, `web/src/pages/docs/*`, live `unterm-cli reference`, current code surface

## 1. Product Summary

Unterm is a local-first, cross-platform terminal that external AI agents can drive through a stable MCP/JSON-RPC control plane, while humans supervise the agents running inside terminal panes through an Agent Cockpit.

The product is not an embedded AI chat terminal. It is a terminal, automation surface, and agent operations cockpit:

- Humans get a polished terminal with tabs, panes, search, copy mode, sidebars, quick actions, web settings, recording, screenshots, profiles, proxy, and modern chrome.
- AI agents get a local MCP server with structured methods for sessions, exec, screen reads, screenshots, workspaces, profiles, policy, fleets, review, and instance discovery.
- Agent-heavy users get orchestration primitives: per-pane agent state, waiting-first inbox, parallel fleets in isolated git worktrees, verification, review, rollback, and merge gates.

## 2. Product Thesis

Unterm wins when it feels like a terminal that is visibly easier to use than Warp for agent-heavy development work, while staying local-first, vendor-neutral, and MCP-driven.

The core bet:

- The terminal itself should be controllable by external agents.
- The terminal should not become a proprietary cloud AI product.
- The human should keep control over identity, destructive actions, review, merge, publishing, and secrets.
- Every major workflow should be available through GUI, CLI, and MCP where appropriate.

## 3. Goals

1. Provide a high-quality daily terminal experience on Windows, macOS, and Linux.
2. Expose a robust local automation surface so any agent can operate the terminal without fragile screen scraping.
3. Make multiple AI agents running in terminal panes observable and manageable from one cockpit.
4. Support parallel agent work safely through isolated worktrees, checkpoints, review, verification, and gated merge.
5. Keep configuration, identity, secrets, proxy, recordings, and screenshots local to the machine.
6. Maintain deterministic multi-instance discovery so agents and scripts can target the correct window.
7. Keep the product easy to install, update, debug, and document across all supported platforms.

## 4. Non-Goals

- Do not build an in-terminal AI chat assistant as the primary product surface.
- Do not depend on cloud services for terminal control, agent state, sessions, or recordings.
- Do not expose secret-writing profile operations through MCP.
- Do not auto-commit or auto-publish agent output; merge leaves changes staged for the user.
- Do not silently run destructive actions. The user or an audited explicit override owns those decisions.
- Do not add disconnected UI features that have no CLI/MCP/settings counterpart when automation parity matters.

## 5. Primary Users

### AI-heavy developer

Runs Claude Code, Codex, Gemini CLI, Aider, OpenCode, Kimi, Trae, or similar tools in terminal panes. Needs to see which agent is working, which one is waiting, and which output is worth merging.

### Agent orchestrator

An external agent or script that needs to create panes, run commands, inspect output, capture screenshots, control sessions, launch fleets, and review results through a local API.

### Multi-account developer

Works across personal and work identities, with different GitHub, AWS, npm, OpenAI, SSH, and git identities. Needs identity isolation by window.

### Power terminal user

Needs fast panes, tabs, copy/search, scrollback, screenshots, session export, SSH/mux compatibility, proxy support, themes, language settings, and predictable shortcuts.

### Release / support maintainer

Needs packaging, self-test, logs, references, update state, and machine-readable capability surfaces to debug user environments.

## 6. Platforms

Supported platforms:

- Windows
- macOS
- Linux

Cross-platform parity is a correctness property. If a feature is platform-specific, the product must document that explicitly and degrade gracefully.

Known platform-specific requirements:

- Windows: native installer, Program Files install, Start Menu shortcut, integrated title buttons, UAC admin launch, Credential Manager secrets.
- macOS: signed and notarized DMG, Finder integration, native title bar, Keychain secrets, external app long screenshot via scroll stitching.
- Linux: deb/AppImage, client-side decoration, Secret Service secrets where available.

## 7. Product Surfaces

Unterm has five product surfaces:

1. GUI terminal window
2. Web Settings / Review UI
3. `unterm-cli`
4. local MCP JSON-RPC server
5. on-disk `~/.unterm/` configuration and runtime files

Every feature must identify which surfaces it owns. User-facing controls should be GUI/Web/CLI. Agent-facing control should be MCP/CLI. Persistent intent should be stored in small documented files under `~/.unterm/`.

## 8. Functional Requirements

### 8.1 Core Terminal

FR-TERM-001: The app must provide GPU-accelerated terminal rendering on Windows, macOS, and Linux.

FR-TERM-002: The terminal must support tabs, panes, splits, pane focus, pane resize, pane zoom, tab activation, tab movement, and tab close.

FR-TERM-003: The terminal must support copy, paste, primary selection where applicable, copy mode, search mode, quick select, character select, clear scrollback, scroll to top/bottom, scroll by line, scroll by page, and scroll to prompt.

FR-TERM-004: The terminal must support right-click direct copy/paste:

- selection present: copy and clear selection
- no selection: paste from clipboard

FR-TERM-005: Clipboard operations must not block the UI thread on Windows/macOS retry paths.

FR-TERM-006: Input handling must remain responsive while multiple agent panes are active, including typing, paste, tab switching, completion, PageUp/PageDown, and wheel scrolling.

FR-TERM-007: The terminal must support command completion ghost text, accepted through right arrow / application right arrow / end as configured.

FR-TERM-008: Ghost completion must not perform disk reads, manifest reads, or large global history clones on the per-key input path.

FR-TERM-009: The terminal must provide automatic tab titles that distinguish shell, foreground command, project, and known agent panes.

FR-TERM-010: Terminal scrollback capacity must be configurable per new pane.

### 8.2 Chrome, Navigation, and Layout

FR-UI-001: The app must provide platform-appropriate window chrome.

FR-UI-002: The top bar must expose the active title, window actions, agent tally, profile indicator where applicable, and entry points to key workflows.

FR-UI-003: The left tab bar must support:

- repository/project grouped tabs
- active tab state
- unread/running/error/done indicators
- agent state indicators
- scrollable tab list
- resize grip
- collapsed project groups
- fuzzy project/tab navigation where implemented

FR-UI-004: Left tab bar painting must avoid per-frame heavy agent/cwd scans during terminal scrolling and busy agent output.

FR-UI-005: The tree sidebar must expose project navigation without disturbing terminal input responsiveness.

FR-UI-006: The Git panel must show repository branch, upstream ahead/behind, and staged/unstaged/untracked files in a read-only right-docked panel.

FR-UI-007: The quick-action overlay must remain intentionally small and include:

- change working directory
- open folder in new tab
- split right
- toggle session recording
- export current session
- settings web link

FR-UI-008: The command palette must expose high-frequency commands, keybindings, and product workflows.

FR-UI-009: The shell selector must allow launching common local shells.

FR-UI-010: The directory jump overlay must allow fuzzy directory navigation from the pane cwd, including PageUp/PageDown, wheel scrolling, Tab completion, open in current pane, and open in new tab.

FR-UI-011: Rename-tab UI must only appear from deliberate rename actions, not accidental tab switching or double-click noise.

### 8.3 Composer and Suggestions

FR-COMP-001: Composer must let users queue multiple prompts or commands for an active agent/shell pane.

FR-COMP-002: Composer must support adding, removing, clearing, selecting, and multi-line editing of queued prompts.

FR-COMP-003: Composer must support execution modes:

- auto-approve
- auto-next
- manual

FR-COMP-004: Auto-approve must inspect screen state and auto-approve simple yes/no confirmations only when confidence is high.

FR-COMP-005: Auto-approve must pause for real multi-choice decisions or ambiguous prompts.

FR-COMP-006: MCP `session.suggest` must allow an external agent to propose text without writing to the PTY.

FR-COMP-007: Suggestion UI must show pending suggestions, support accept, accept-and-run, dismiss, status, cancellation, and list operations.

FR-COMP-008: Suggestions and confirmations must use audited, user-visible write boundaries.

### 8.4 Agent Cockpit

FR-AGENT-001: Unterm must detect known AI coding agents running in panes, including Claude Code, Codex, Gemini CLI, Aider, OpenCode, Kimi, Trae, Cursor Agent, and future manifest-backed agents.

FR-AGENT-002: Agent state must normalize to:

- working
- waiting
- done
- idle

FR-AGENT-003: State detection must merge signal layers by precedence:

1. official lifecycle hooks
2. OSC/title/progress/notification parsing
3. foreground-process detection
4. screen-text heuristics

FR-AGENT-004: Waiting state must be prioritized so an agent needing human input is not hidden by weaker signals.

FR-AGENT-005: Agent state must appear in the sidebar, top bar tally, inbox, CLI, and MCP.

FR-AGENT-006: The Agent Inbox must list all tracked agents across instances, waiting-first, with pane/window location and task hint.

FR-AGENT-007: Inbox selection must jump to the target pane/window.

FR-AGENT-008: CLI must expose agent status, signal, inbox, hooks, trust, untrust, install/config/auth/launch/run where implemented.

FR-AGENT-009: MCP must expose agent identity, trust, status, signal, and cockpit inbox methods.

FR-AGENT-010: `setup-ai` must register Unterm with supported local agents and merge context files idempotently.

FR-AGENT-011: Agent setup must never clobber existing agent configuration and must support dry-run and removal.

FR-AGENT-012: Agent hook enablement must create backups when editing agent config files.

### 8.5 Fleet

FR-FLEET-001: Fleet launch must run one task across N agents in N isolated git worktrees.

FR-FLEET-002: Fleet worktrees must be created beside the base repository, using deterministic task/member naming and per-member branches.

FR-FLEET-003: Fleet launch must refuse unsafe base repository states instead of silently stashing or overwriting user work.

FR-FLEET-004: Each fleet member must open in a tab rooted at its own worktree and launch the selected agent with the task.

FR-FLEET-005: Fleet state must persist under `~/.unterm/fleets.json`.

FR-FLEET-006: Fleet list must expose members, worktrees, branches, agent state, review state, attempts, and latest errors.

FR-FLEET-007: Fleet retry must restart a failed/pending member in its existing worktree without losing committed, staged, unstaged, or untracked changes.

FR-FLEET-008: Fleet clean must remove worktrees, branches, and panes only after review states allow it, with explicit force for override.

FR-FLEET-009: Fleet operations must be available through GUI where applicable, CLI, and MCP.

### 8.6 Review, Checkpoints, Verification, and Merge

FR-REVIEW-001: The system must checkpoint loose agent work before an agent mutates a repository, without touching HEAD or the index.

FR-REVIEW-002: Checkpoints must be stored as dangling commits and recorded under `~/.unterm/checkpoints.json`.

FR-REVIEW-003: Checkpoint creation must be debounced and capped per repo.

FR-REVIEW-004: Review must show fleet members and loose checkpoints.

FR-REVIEW-005: Review diff must include line-level changes and untracked files.

FR-REVIEW-006: Review must support list, diff, verify, rollback, merge, discard, open review UI, and compare where implemented.

FR-REVIEW-007: Verification must infer conventional test commands from auditable project markers:

- Cargo
- Go
- npm/pnpm/yarn
- Python/pytest/uv
- Maven
- Gradle
- .NET

FR-REVIEW-008: Verification must allow explicit command override.

FR-REVIEW-009: Verification must run asynchronously in the member worktree, persist status/log/duration, bound logs, enforce timeout, and kill process trees on timeout.

FR-REVIEW-010: Review ranking must prioritize passing verification, then discount unnecessarily large changes.

FR-REVIEW-011: Merge must squash a fleet member into the base repo and stop with changes staged, not committed.

FR-REVIEW-012: Merge must require the latest verification to pass unless an explicit audited force override is used.

FR-REVIEW-013: Rollback must restore a repo to a checkpoint and remove files that did not exist at that checkpoint after explicit confirmation.

FR-REVIEW-014: Destructive review operations must be audited.

### 8.7 MCP Server

FR-MCP-001: Every Unterm instance must start a local MCP JSON-RPC server bound to `127.0.0.1`.

FR-MCP-002: The MCP server must use line-delimited JSON-RPC 2.0 over TCP.

FR-MCP-003: The first call on a connection must be `auth.login` with the per-launch auth token.

FR-MCP-004: MCP auth tokens must be generated per launch and written to per-instance files with user-only permissions where supported.

FR-MCP-005: MCP must expose 99 documented methods across these namespaces:

- auth
- meta
- session
- exec
- signal
- screen
- ghost
- orchestrate
- workspace
- capture
- upload
- proxy
- policy
- server
- selftest
- agent
- cockpit
- fleet
- review
- system
- profile
- instance

FR-MCP-006: `meta.surface` must expose MCP methods, CLI subcommands, and live keybindings for feature discovery.

FR-MCP-007: `server.capabilities` must remain available for compatibility.

FR-MCP-008: MCP mutating calls must be audited and policy/confirmation gated where appropriate.

FR-MCP-009: MCP should return structured errors with useful messages for missing sessions, invalid params, policy blocks, and unsupported operations.

FR-MCP-010: MCP must stay responsive during high-volume terminal output, scrolling, paste, and multiple active agents.

### 8.8 CLI

FR-CLI-001: `unterm-cli` must route MCP-backed commands to the active/latest instance by default.

FR-CLI-002: `unterm-cli --instance <id>` and `UNTERM_INSTANCE=<id>` must route to a specific live instance.

FR-CLI-003: CLI commands must support `--json` where output may be consumed by scripts.

FR-CLI-004: CLI commands must support `--lang <code>` for one-shot locale override.

FR-CLI-005: CLI must expose these product command families:

- start
- session
- exec
- sessions
- workspace
- instance
- screenshot
- upload
- scrollback
- reference
- server
- setup-ai
- mcp-stdio
- settings
- policy
- proxy
- theme
- profile
- agent
- fleet
- review
- lang
- shell-completion

FR-CLI-006: CLI may retain upstream/engine command families where useful:

- cli mux operations
- show-keys
- ls-fonts
- imgcat
- set-working-directory
- record
- replay
- ssh
- connect

FR-CLI-007: CLI errors must exit non-zero except lifecycle hooks such as `agent signal`, which must not break the calling agent when Unterm is absent.

FR-CLI-008: `reference` must work with static fallback when the GUI is not reachable.

### 8.9 Session Recording and Archive

FR-REC-001: Users must be able to start, stop, check status, and export session recordings per pane.

FR-REC-002: Recordings must render to markdown.

FR-REC-003: Recordings must use OSC 133 block segmentation where available and fallback to plain output where not.

FR-REC-004: Recordings must redact known secret patterns by default.

FR-REC-005: Users must be able to define custom redaction patterns.

FR-REC-006: Recordings must prefer project-local storage under `<cwd>/.unterm/sessions/`, falling back to `~/.unterm/sessions/_orphan/`.

FR-REC-007: Completed recordings must be listable and readable through CLI and MCP.

FR-REC-008: A one-off markdown export of current scrollback must work without an active recording.

FR-REC-009: Recording APIs must support external trace IDs for correlation.

### 8.10 Screenshots, Scrollback, and Upload

FR-CAP-001: Users and agents must be able to capture the screen as PNG.

FR-CAP-002: Users and agents must be able to capture a specific window by title or pid where supported.

FR-CAP-003: Region selection capture must be available in GUI; headless MCP must degrade to screen capture.

FR-CAP-004: Clipboard capture must support text and image where supported.

FR-CAP-005: Scrollback screenshot must render a pane's entire scrollback into one tall PNG headlessly, preserving theme/font, working while occluded.

FR-CAP-006: External app long screenshot must scroll and stitch another app's window on supported platforms.

FR-CAP-007: Text scrollback export must dump full scrollback plus viewport as text, with tail and range controls.

FR-CAP-008: Upload must send local files to configured OSS/COS/Qiniu and return public URL metadata without exposing credentials.

### 8.11 Workspaces

FR-WS-001: Users and agents must be able to save the current set of panes as a named workspace.

FR-WS-002: Saved workspaces must include metadata such as name, saved_at, path, session count, titles, and working directories.

FR-WS-003: Workspace restore must open tabs from saved workspace data.

FR-WS-004: Workspace restore must support dry-run planning.

FR-WS-005: Workspace list/save/restore must be available through CLI and MCP.

### 8.12 Multi-Instance

FR-INST-001: Every running Unterm process must claim a human-readable NATO phonetic instance id.

FR-INST-002: Instance ids must include suffixes when all base names are taken.

FR-INST-003: Each instance must write metadata to `~/.unterm/instances/<id>.json`.

FR-INST-004: `server.json` and `active.json` must remain available for compatibility and active/latest routing.

FR-INST-005: Stale instance files must be cleaned up safely.

FR-INST-006: Instance records must include id, mcp_port, http_port, auth_token, pid, started_at, title, cwd, version, and platform.

FR-INST-007: Users and agents must be able to list, inspect, title, and focus instances.

FR-INST-008: Instance discovery must tolerate PID inspection failures without deleting live Windows instances too aggressively.

### 8.13 Identity Profiles

FR-PROF-001: A profile must bind one window to one coherent identity.

FR-PROF-002: Profiles must support static env vars, secrets, git identity, SSH key routing, npm registry, gh host mapping, expiration metadata, accent color, description, and default profile.

FR-PROF-003: Secrets must live in OS-native secret storage:

- macOS Keychain
- Windows Credential Manager
- Linux Secret Service where available

FR-PROF-004: Profile TOML files must contain references only, not raw secret values.

FR-PROF-005: Profile env injection must apply to newly spawned panes in a profile-bound window.

FR-PROF-006: Existing shells must not be mutated when a profile changes.

FR-PROF-007: CLI must support profile list, create, show, set-secret, delete, audit, edit, export, spawn, import, set-default, and shell integration.

FR-PROF-008: MCP profile namespace must be read-only: list, current, audit.

FR-PROF-009: Profile audit must reveal expiring/missing secret metadata without exposing values.

FR-PROF-010: SSH routing must be additive and generated into `~/.unterm/ssh/config.unterm`.

### 8.14 Proxy

FR-PROXY-001: Unterm must support proxy enable/disable and injection into spawned shells.

FR-PROXY-002: Proxy auto mode must detect OS/system proxy where possible.

FR-PROXY-003: Proxy manual mode must support configured HTTP/SOCKS URLs and no_proxy.

FR-PROXY-004: Proxy nodes must support list, switch, disable, env, speedtest, rotation, and set_nodes.

FR-PROXY-005: Clash/mihomo integration must expose controller status, selector switching, and manual controller config.

FR-PROXY-006: Proxy state must be available through GUI/Web Settings, CLI, and MCP.

FR-PROXY-007: Proxy env output must be shell-safe for eval use.

### 8.15 Web Settings and Review UI

FR-WEB-001: Every GUI instance must start a local HTTP settings server bound to `127.0.0.1`.

FR-WEB-002: HTTP settings must be auth-token gated except bootstrap endpoints that are explicitly local-only.

FR-WEB-003: Web Settings must expose modern form UI for settings that are awkward in terminal UI.

FR-WEB-004: Web Settings must cover:

- theme
- language
- proxy
- scrollback
- compatibility / TERM_PROGRAM
- recording
- AI agents
- sessions / recordings
- review
- updates

FR-WEB-005: Web Review must expose fleet/checkpoint review, diff, verify, merge, discard, rollback, and ranking workflows where implemented.

FR-WEB-006: Settings written in Web UI must share the same `~/.unterm/` files used by CLI and runtime.

### 8.16 Localization and Themes

FR-I18N-001: Unterm must support these UI locales:

- English
- Simplified Chinese
- Traditional Chinese
- Japanese
- Korean
- German
- French
- Italian
- Hindi

FR-I18N-002: Locale must auto-detect from OS and allow override via Web Settings and CLI.

FR-I18N-003: CLI must support per-invocation locale override.

FR-THEME-001: Unterm must ship built-in theme presets:

- standard
- midnight
- daylight
- classic
- notion-dark
- notion-light

FR-THEME-002: Theme switch must apply live when GUI is running and persist for next launch when not.

### 8.17 Policy, Trust, and Safety

FR-SAFE-001: MCP command execution must support policy checks for blocked patterns.

FR-SAFE-002: Agents must be identifiable for audit grouping.

FR-SAFE-003: Users must be able to trust/untrust agent names so future PTY writes can skip confirmation according to policy.

FR-SAFE-004: Mutating MCP calls must append audit entries.

FR-SAFE-005: Destructive operations such as pane destroy, fleet clean, review rollback, review merge override, and policy changes must be auditable.

FR-SAFE-006: Profile secret values must not be returned through MCP.

FR-SAFE-007: Recordings must redact common secrets by default.

FR-SAFE-008: Local servers must bind only to loopback.

### 8.18 System and Self-Test

FR-SYS-001: `system.info` must expose OS, arch, hostname, locale, version, and active session metadata.

FR-SYS-002: Windows admin launch must support UAC elevation and dry-run.

FR-SYS-003: Unsupported platform admin launch must return a clear error.

FR-SYS-004: `server.health`, `server.info`, `server.capabilities`, and `selftest.run` must provide operational diagnostics.

FR-SYS-005: Self-test must probe mux, server, capabilities, policy, admin dry-run where applicable, proxy, capture, and optional pane checks.

### 8.19 Installation, Release, and Updates

FR-REL-001: Release artifacts must be produced for:

- macOS DMG
- Linux deb
- Linux AppImage
- Windows MSI
- Windows zip

FR-REL-002: macOS builds must be signed and notarized.

FR-REL-003: Windows installer must place binaries under Program Files and create Start Menu entries.

FR-REL-004: Linux packages must follow normal distro expectations.

FR-REL-005: Update check state must be persisted in `~/.unterm/update_check.json`.

FR-REL-006: Website/docs/release metadata must match shipped product capabilities.

## 9. MCP Method Coverage Requirement

The product must document and keep working these 99 MCP methods plus `auth.login`:

- `agent.identify`
- `agent.list_trusted`
- `agent.signal`
- `agent.status`
- `agent.trust`
- `agent.untrust`
- `agent.whoami`
- `capture.clipboard`
- `capture.screen`
- `capture.scrollback`
- `capture.select`
- `capture.window`
- `capture.window_scroll`
- `cockpit.inbox`
- `exec.cancel`
- `exec.run`
- `exec.run_wait`
- `exec.send`
- `exec.status`
- `fleet.clean`
- `fleet.launch`
- `fleet.list`
- `fleet.retry`
- `ghost.debug`
- `instance.focus`
- `instance.info`
- `instance.list`
- `instance.set_title`
- `meta.surface`
- `orchestrate.broadcast`
- `orchestrate.launch`
- `orchestrate.wait`
- `policy.check`
- `policy.set`
- `profile.audit`
- `profile.current`
- `profile.list`
- `proxy.clash_select`
- `proxy.clash_set_controller`
- `proxy.clash_status`
- `proxy.configure`
- `proxy.disable`
- `proxy.env`
- `proxy.nodes`
- `proxy.rotation`
- `proxy.set_nodes`
- `proxy.speedtest`
- `proxy.status`
- `proxy.switch`
- `review.diff`
- `review.discard`
- `review.list`
- `review.merge`
- `review.rollback`
- `review.verify`
- `screen.cursor`
- `screen.detect_errors`
- `screen.read`
- `screen.scroll`
- `screen.scrollback_text`
- `screen.search`
- `screen.text`
- `selftest.run`
- `server.capabilities`
- `server.health`
- `server.info`
- `session.audit_log`
- `session.create`
- `session.cwd`
- `session.destroy`
- `session.env`
- `session.export_markdown`
- `session.focus`
- `session.get`
- `session.history`
- `session.idle`
- `session.input`
- `session.list`
- `session.recording_attach_trace`
- `session.recording_list`
- `session.recording_read`
- `session.recording_start`
- `session.recording_status`
- `session.recording_stop`
- `session.resize`
- `session.set_env`
- `session.split`
- `session.status`
- `session.suggest`
- `session.suggest_cancel`
- `session.suggest_list`
- `session.suggest_status`
- `signal.send`
- `system.info`
- `system.launch_admin`
- `upload.file`
- `workspace.list`
- `workspace.restore`
- `workspace.save`

## 10. CLI Coverage Requirement

The product must document and keep working these CLI families:

- `start`
- `cli`
- `session`
- `exec`
- `sessions`
- `workspace`
- `instance`
- `screenshot`
- `upload`
- `scrollback`
- `reference`
- `server`
- `setup-ai`
- `mcp-stdio`
- `settings`
- `policy`
- `proxy`
- `theme`
- `profile`
- `agent`
- `fleet`
- `review`
- `lang`
- `show-keys`
- `ls-fonts`
- `imgcat`
- `set-working-directory`
- `record`
- `replay`
- `ssh`
- `connect`
- `shell-completion`

## 11. Key Default Workflows

### Workflow A: Agent needs attention

1. User runs multiple agent CLIs in panes.
2. Unterm detects pane states.
3. Top bar shows waiting tally.
4. User opens Inbox.
5. Waiting agents are sorted first.
6. User presses Enter on an item.
7. Unterm focuses the correct instance/tab/pane.
8. User answers the agent.

Acceptance: no waiting agent is hidden behind a working/idle signal; jump routing is deterministic across multiple windows.

### Workflow B: External agent drives a pane

1. Agent discovers instance metadata under `~/.unterm/instances/`.
2. Agent connects to MCP on loopback.
3. Agent authenticates with `auth.login`.
4. Agent calls `session.list`.
5. Agent creates/focuses a pane.
6. Agent calls `exec.run`, `screen.text`, `screen.search`, `capture.screen`, or other methods.
7. Mutating calls are audited and policy-checked.

Acceptance: no screen scraping is required for normal automation.

### Workflow C: Run a fleet

1. User or agent launches a fleet with N agents and one task.
2. Unterm validates repo state.
3. Unterm creates N worktrees and branches.
4. Unterm opens N tabs and launches agents.
5. Cockpit tracks every member.
6. Review lists each output.
7. Verification runs for candidates.
8. User merges the best passing member as staged changes.
9. User commits manually.
10. User cleans the fleet.

Acceptance: base working tree is protected; no member can overwrite another; merge is gated by verification unless forced.

### Workflow D: Record and export a session

1. User starts recording on a pane.
2. Terminal captures raw stream and block boundaries.
3. User stops or exports.
4. Markdown is written to project-local or fallback archive.
5. Secrets are redacted.
6. User can list/read recordings later.

Acceptance: tokens are masked by default; markdown path is returned and stable.

### Workflow E: Identity-bound work

1. User creates a profile.
2. User stores secrets in OS vault.
3. User spawns a profile-bound window.
4. New panes inherit profile env, git identity, and SSH routing.
5. Agent can read current profile metadata but cannot read secrets.

Acceptance: no raw secret values appear in profile TOML or MCP profile responses.

## 12. Non-Functional Requirements

### Performance

NFR-PERF-001: Cold start should remain fast enough for daily use and must avoid unnecessary network/disk work on the UI thread.

NFR-PERF-002: Typing, paste, right-click paste, tab switching, PageUp/PageDown, wheel scrolling, and command completion must remain responsive while agent panes produce output.

NFR-PERF-003: Per-key input path must be memory/disk-light.

NFR-PERF-004: Per-frame paint path must avoid whole-window process-tree scans and expensive metadata rebuilds.

NFR-PERF-005: Background agent/cwd/status refresh must be bounded by TTLs and in-flight limits.

### Reliability

NFR-REL-001: Instance discovery must self-heal stale files and avoid false deletion of live Windows instances.

NFR-REL-002: MCP/HTTP startup must handle port collisions.

NFR-REL-003: Recording, fleet, checkpoint, verification, and profile files must tolerate malformed/missing files by falling back or surfacing clear errors.

NFR-REL-004: Long-running verification must not leak child processes after timeout.

NFR-REL-005: GUI should not freeze due to logging, clipboard, process inspection, or agent startup.

### Security and Privacy

NFR-SEC-001: No telemetry is required for core product operation.

NFR-SEC-002: MCP and HTTP servers must bind to loopback only.

NFR-SEC-003: Auth token must rotate per launch.

NFR-SEC-004: Secrets must live in OS vaults, not dotfiles.

NFR-SEC-005: MCP profile methods must not expose secret values.

NFR-SEC-006: Recordings must redact common tokens by default.

NFR-SEC-007: Destructive actions must require explicit user action or audited API override.

### UX

NFR-UX-001: UI must feel deliberate, dense, and scannable rather than decorative.

NFR-UX-002: Common workflows must be reachable through predictable keyboard shortcuts and command palette entries.

NFR-UX-003: Web Settings must be used for settings that need modern forms, previews, or complex controls.

NFR-UX-004: In-terminal overlay menus must remain slim and task-focused.

### Documentation

NFR-DOC-001: README must state the product thesis and core features.

NFR-DOC-002: MCP reference must match live `meta.surface`.

NFR-DOC-003: CLI reference must match live `unterm-cli reference`.

NFR-DOC-004: Configuration reference must document every file under `~/.unterm/`.

NFR-DOC-005: Public docs must clearly separate implemented features from roadmap.

## 13. Success Metrics

Product quality metrics:

- Median cold start stays within target release budget.
- No UI freezes over 120ms in normal typing/paste/scroll workflows under active agent load.
- MCP server remains responsive during output floods.
- Instance discovery produces no missing live instances in multi-window usage.
- Agent waiting state is detected and surfaced reliably.

Adoption metrics:

- Users can set up at least one supported agent with `setup-ai` without manual MCP config.
- Users can launch and review a fleet successfully on a clean repo.
- Users can export a useful markdown session without hand-editing paths.
- Users can switch theme/language/proxy through Web Settings or CLI.

Safety metrics:

- No profile secret values are stored in profile TOML.
- No MCP profile method returns raw secrets.
- Review merge without passing verification requires explicit force and audit.
- Recording redaction covers known token classes.

## 14. Acceptance Criteria by Release

### Current release quality bar

- `cargo check -p unterm` passes on supported targets in CI.
- Core UI does not regress typing, paste, scroll, tab switching, or command completion.
- Live `unterm-cli reference` lists all MCP/CLI surfaces documented here.
- Agent Cockpit, Fleet, Review, Profile, Proxy, Recording, Screenshot, Workspace, and Instance docs reflect shipped behavior.

### Manual smoke test

1. Launch Unterm.
2. Open at least three tabs.
3. Run one shell command, one Claude/Codex/Gemini/Aider pane, and one idle shell.
4. Verify sidebar title/state indicators.
5. Paste a long token-like string through right-click paste.
6. Type and accept ghost completion with right arrow.
7. PageUp/PageDown through large scrollback.
8. Open Composer and queue two prompts.
9. Toggle Git panel.
10. Open DirJump and page through results.
11. Start and stop session recording.
12. Export scrollback markdown.
13. Capture scrollback PNG.
14. Run `unterm-cli session list`.
15. Run `unterm-cli reference --section mcp`.
16. Open Web Settings.
17. If in a git repo, launch a small fleet dry-run or test fleet in a disposable repo.

## 15. Open Product Questions

1. Should Web Review expose force-merge, or should force remain CLI/MCP only for audit friction?
2. Should profile destructive-command guard be promoted from documented integration to default onboarding?
3. Should workspace restore include split layout and commands, or remain cwd/title tabs only?
4. Should long screenshot of external apps be expanded beyond macOS?
5. Should command completion become configurable per shell/project?
6. Should MCP write confirmation policy become persisted user config rather than runtime-only?
7. Should agent install/auth/config surfaces be fully represented in Web Settings?

## 16. Maintenance Rule

When adding a feature, update these in the same PR when applicable:

- `docs/product-requirements.md`
- `README.md`
- `web/src/pages/docs/mcp-reference.md`
- `web/src/pages/docs/cli-reference.md`
- `web/src/pages/docs/configuration.md`
- `unterm-cli reference` source tables / MCP `meta.surface`
- tests or smoke checks for the feature's primary surface

The PRD is complete only if a user, an external agent, and a maintainer can each find the feature from their own surface.
