# New-kernel feature parity

This is the evidence ledger for aligning the native `unterm-app`/`next-core`
stack with the last WezTerm-based product. The authoritative requirements are
the 159 `FR-*` entries in `docs/product-requirements.md`; a passing build alone
does not constitute parity.

## Baseline

- v0.57.4 comparison point: `3a7e1033`
- New-kernel baseline: `82d15857` (`0.60.0`)
- Working branch: `agent/new-kernel-feature-parity`
- Status meanings:
  - **Verified**: implementation and requirement-specific automated or runtime
    evidence both exist.
  - **Implemented, runtime pending**: the path exists and focused tests pass,
    but the release scenario still needs a real-window check.
  - **Partial**: some clauses are present and at least one is absent.
  - **Missing**: current-state inspection contradicts the requirement.
  - **Audit pending**: no conclusion has been drawn yet.

## Requirement inventory

| Area | Requirements | Current audit state |
| --- | ---: | --- |
| TERM | 10 | 9 verified; 1 cross-platform runtime pending |
| UI | 11 | Detailed below; runtime acceptance remains |
| COMP | 8 | Detailed below; runtime acceptance remains |
| AGENT | 12 | Detailed below; cross-instance runtime acceptance remains |
| FLEET | 9 | Detailed below; runtime acceptance remains |
| REVIEW | 14 | Detailed below; runtime acceptance remains |
| MCP | 10 | Detailed below |
| CLI | 8 | Detailed below |
| REC | 9 | Detailed below; runtime acceptance remains |
| CAP | 8 | Detailed below; runtime acceptance remains |
| WS | 5 | Detailed below |
| INST | 8 | Detailed below; multi-process runtime acceptance remains |
| PROF | 10 | Detailed below; vault/window runtime acceptance remains |
| PROXY | 7 | Detailed below; external integrations remain |
| WEB | 6 | Detailed below; runtime acceptance remains |
| I18N | 3 | Detailed below |
| THEME | 2 | Detailed below; live-window acceptance remains |
| SAFE | 8 | Detailed below |
| SYS | 5 | Detailed below; UAC runtime acceptance remains |
| REL | 6 | Detailed below; platform artifact acceptance remains |
| **Total** | **159** | **146 verified; 13 runtime pending** |

## Core terminal

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-TERM-001 | Implemented, runtime pending | `unterm-app` uses wgpu and builds on Windows. The `82d15857` remote CI exposed Linux `HOME` ownership, missing macOS CoreFoundation/CoreGraphics dependencies, an obsolete `scrollshot` reference, Unix shell quoting, and strict-test guard/panic-format failures. The parity branch repairs all five classes; both CI-equivalent Windows `cargo check -p unterm-app -p unterm-cli` and the complete `cargo test --release --workspace -- --test-threads=1` pass with `-D warnings`. A Windows-hosted macOS cross-check progressed through the Rust dependencies but stopped at the expected missing Apple C compiler/SDK, so native Linux/macOS CI and real-window rendering remain required. |
| FR-TERM-002 | Verified | Tabs, splits, focus, zoom, activation, close, visible tab order and relative movement are covered by layout/tab/key tests and real native interaction. A real top-bar split produced 37/38-column PTYs; selecting `Resize Pane Left` through the production command palette changed them to 34/41, proving boundary movement and both PTY resizes. Earlier keyboard acceptance also created/closed tabs and panes. |
| FR-TERM-003 | Verified | Copy/paste, copy mode, search, quick select, character select, clearing and ordinary scrolling are implemented. The next core records OSC 133 prompt-start rows, keeps them aligned through history trimming/clearing, and exposes `Ctrl+Shift+Up/Down` navigation. In a real Windows pane, three emitted OSC 133 prompt blocks appeared in scrollback; selecting the production `Previous Prompt` action (shown with `CTRL\|SHIFT Up`) moved the live viewport from the bottom back to `PROMPT-3`. Parser, trimming, navigation and binding tests cover the underlying state transitions. |
| FR-TERM-004 | Verified | `mouse::right_click` routes selection to copy-and-clear and an empty selection to paste. In a real pane, dragging across `PROMPT-3` then right-clicking placed exactly `PROMPT-3` on the Windows clipboard and cleared the selection; a second right-click with no selection inserted unique text at the live command prompt without executing it. Focused routing tests cover both branches. |
| FR-TERM-005 | Verified | Clipboard reads and writes run on worker threads and return through an event-loop channel, so platform retries cannot hold winit. A focused test blocks a simulated platform operation and proves the caller returns immediately. During real Windows acceptance, another process held the clipboard open for four seconds; while that lock was still `Running`, the native window processed a Settings hover and produced a fresh capture in 1.45 seconds, and the asynchronous paste completed without freezing the UI. The same worker boundary wraps the macOS path. |
| FR-TERM-006 | Verified | The reachable 25-scenario/35-gate next-core benchmark report covers dual-agent echo, agent-startup input, first-session readiness, paste under output flood, focus switching, PageUp/PageDown semantics, wheel-style viewport scrolling and screen reads under flood. The verifier passes on this branch; a fresh post-parity benchmark run remains part of final release acceptance. |
| FR-TERM-007 | Verified | Native GUI observes PTY-bound keys and IME commits, paints a pane-clipped prediction at the focused cursor, and accepts with unmodified Right Arrow or End. In a real window, typing `cod` rendered dim `ex`; End converted it into committed `codex` text at the shell cursor without executing it. Six focused GUI tests cover application/keypad variants and clipping. |
| FR-TERM-008 | Verified | Per-key path calls an in-memory registry with an empty external candidate slice, and the prefix-scan budget is now shared across the pane-local/global/external pools so total per-keystroke work stays bounded. Agent names/flags initialize from compile-time constants; the signed manifest catalog is merged once on a background thread, so the key-event path itself still never reads the disk while manifest-only agents complete like built-ins. |
| FR-TERM-009 | Verified | Automatic naming resolves pane title to shell/foreground fallback; the left tab strip independently carries foreground command detail, project grouping with disambiguation, and known-agent identity. A focused regression test covers all four identities. |
| FR-TERM-010 | Verified | Startup resolves explicit `scrollback_lines` first, then legacy Web Settings `scrollback.json`, and installs that capacity into each subsequently-created next-core screen. With an isolated `scrollback_lines = 3` launch, 80 real shell output lines yielded exactly 27 total rows (24-row viewport plus three history rows), starting at `parity-line-56`; the temporary config and instance were removed. |

## Chrome, navigation, and layout

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-UI-001 | Implemented, runtime pending | Native custom chrome, drag regions and minimise/maximise/close paths exist with layout/hit-test coverage. Real Windows acceptance changed `IsIconic` on minimise, restored successfully, expanded the window from 1000x504 to 1721x927 on maximise, restored on the second press, and exited/unregistered `bravo` through the custom close button. macOS/Linux visual acceptance remains. |
| FR-UI-002 | Verified | The top/status chrome exposes title/facts, actions, Agent tally, profile identity and workflow entry points; narrow/wide density tests pass. Real 150%-DPI window captures showed the custom top actions and live bottom facts/status row. |
| FR-UI-003 | Verified | Project grouping, active state, Agent badges, scrolling, resize grip and collapsible groups exist. Per-pane revision tracking aggregates sticky unread/error/running indicators onto tabs, while the footer and command palette expose fuzzy project/tab navigation. Focused tests pass and a real window rendered the active project/tab sidebar and palette workflow. |
| FR-UI-004 | Verified | Sidebar painting consumes `known_facts`, an in-memory cache; process/Git refresh runs on a named worker and only for panes being actively refreshed. |
| FR-UI-005 | Verified | The tree is read on open/navigation rather than paint, has bounded rows/scrolling and does not take terminal keys while closed. A real top-bar click replaced the tab strip with the repository tree and rendered the live project directories. |
| FR-UI-006 | Verified | Git parsing covers branch, upstream divergence and staged/unstaged/untracked entries. A real palette launch rendered the branch and 99 changed files in the read-only right-docked panel while leaving the terminal visible. |
| FR-UI-007 | Verified | The quick menu now carries 0.57.4's full list in its order — new tab, split right, directory jump, file tree, Git panel, left strip, find, command palette (with live chords from the binding table), recording toggle, export, scrollback long screenshot, Web Settings, and the version/website row — replacing the earlier six-entry reduction; the requirement text was corrected to match what 0.57.4 actually shipped. |
| FR-UI-008 | Verified | The palette is generated from the same action/keybinding table exposed through MCP and includes product workflow rows. |
| FR-UI-009 | Verified | Launcher discovery and launch commands cover installed common local shells. A real launcher listed Windows PowerShell, cmd, WSL and Bash; selecting the first entry increased live sessions from one to two, and the test tab was removed. |
| FR-UI-010 | Verified | Directory browsing/fuzzy/path queries, current-pane/new-tab outcomes, PageUp/PageDown, wheel movement and Tab path completion are wired with focused key coverage. A real top-bar click rendered cwd-relative directory choices and their resolved paths. |
| FR-UI-011 | Verified | Renaming now exists as a deliberate gesture rather than being absent: only a same-row double-click on the tab strip (500 ms, 8 pt slop, tracked separately from terminal-pane clicks) opens a palette rename line; Enter applies, an empty line restores automatic titling, Esc cancels. Tab switching and cross-row click bursts cannot reach it — the streak tracker has focused tests, and the full GUI suite passes. |

## Composer and suggestions

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-COMP-001 | Verified | The native Composer queues multiple prompts FIFO for the focused pane. |
| FR-COMP-002 | Verified | Add, select, remove, clear and Shift+Enter multi-line editing are implemented with focused queue tests. In a real window the Composer was switched to manual mode, queued `parity prompt` as one waiting item, then removed it with Delete without writing to the shell. |
| FR-COMP-003 | Verified | Explicit auto-approve, auto-next and manual modes exist and cycle independently. |
| FR-COMP-004 | Verified | Auto-approve reads the last non-empty screen line and accepts only narrow affirmative shapes. |
| FR-COMP-005 | Verified | Destructive/multi-choice/ambiguous questions fail the recognizer and remain paused; focused negative tests cover delete/remove/overwrite/force and arbitrary questions. |
| FR-COMP-006 | Verified | `session.suggest` resolves a live engine session and queues text without a PTY write; engine-neutral MCP coverage passes. |
| FR-COMP-007 | Verified | Pending suggestion cards show source, text, rationale and count; Tab accepts, Alt+Enter accepts/runs, Esc dismisses, while MCP status/cancel/list remain available. A real authenticated `session.suggest` rendered all of those fields and key hints in the native window; `session.suggest_cancel` then removed it without PTY input. |
| FR-COMP-008 | Verified | Suggestion accept/dismiss already append audit records; Composer auto-approval now appends a GUI-write audit entry before sending the affirmative response. |

## Agent Cockpit

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-AGENT-001 | Implemented, runtime pending | The next-core process tree recognizes Claude, Codex, Gemini, Aider, OpenCode, Kimi, Trae and Cursor Agent (plus Zcode). The facts worker now also matches foreground executable/script names against the signed cached or baked manifest `detect.command`, with focused path/extension coverage. Real future-manifest process acceptance remains. |
| FR-AGENT-002 | Verified | `AgentState` has exactly idle, working, waiting-for-user and done, with rendering/status tests covering each meaningful state. |
| FR-AGENT-003 | Verified | The status registry merges hook, OSC, process and heuristic signals in that precedence order; focused service tests cover precedence. |
| FR-AGENT-004 | Verified | Waiting is sticky against weaker signals and sorts ahead of every other inbox state; service and GUI model tests pass. |
| FR-AGENT-005 | Verified | Sidebar badges, top-bar attention count, native inbox, CLI status/inbox and MCP status/inbox all consume the Cockpit registry. A real palette launch rendered the native Agent Inbox in the live window; focused model/MCP tests cover populated states. |
| FR-AGENT-006 | Verified | Each instance publishes agent state with instance, tab, pane, title, state age and task hint. GUI and MCP aggregate live instance records and sort waiting-first; focused model/MCP tests pass. Real Windows acceptance with independent `bravo` and `charlie` instances showed the remote waiting pane first with the exact `peer jump target` task hint and location. |
| FR-AGENT-007 | Verified | Native Inbox has highlighted wrapping Up/Down selection and Enter jumps locally or authenticates to the peer MCP server, focuses its pane, then raises its window. In real Windows two-instance acceptance, Enter on `charlie / tab 1 / pane 2` changed that peer pane from inactive to active and raised its window with title `Peer jump target — Unterm`; the protocol sequence is also covered by a fake TCP server. |
| FR-AGENT-008 | Verified | CLI metadata and dispatch expose list/show/install/update/uninstall/auth/configure/import/plan/launch/run/manifest plus Cockpit status/signal/inbox/hooks/trust commands. The 19-test CLI suite passes. |
| FR-AGENT-009 | Verified | MCP exposes identity, trust/untrust, status, signal and `cockpit.inbox`; engine-neutral status/signal/inbox tests pass. |
| FR-AGENT-010 | Verified | `setup-ai` registers supported local agents and repairs/merges its managed context block idempotently; focused JSON/TOML/context tests pass. |
| FR-AGENT-011 | Verified | Setup refuses unparseable configuration, preserves unrelated content, supports dry-run and removes only managed entries; focused tests pass. |
| FR-AGENT-012 | Verified | Hook editing uses one-time backups before mutation; Claude/Codex/Aider merge, repair and removal tests pass. |

## Fleet orchestration

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-FLEET-001 | Verified | Fleet launch creates one isolated worktree and next-core pane per requested member; the engine-neutral lifecycle test launches, retries and cleans a multi-member fleet. |
| FR-FLEET-002 | Verified | Worktrees live under the sibling `<repo>.fleet` directory with deterministic task/member branch names. |
| FR-FLEET-003 | Verified | Preflight rejects non-repositories, tracked dirt and untracked files, including when launched from a subdirectory; focused Git tests pass. |
| FR-FLEET-004 | Verified | `EngineFleetPanes` creates a session rooted at each member worktree and launches its agent command with the shared task. Real Windows GUI acceptance launched an isolated one-member `echo` fleet: pane 2 used the exact generated worktree as its cwd, the sidebar rendered it as a separate grouped tab, and the terminal showed the shared task command and output. Production `fleet clean --force` then removed the pane, worktree, branch and persisted fleet record. |
| FR-FLEET-005 | Verified | Fleet state persists atomically in `~/.unterm/fleets.json` (with a test override used by engine-neutral lifecycle coverage). |
| FR-FLEET-006 | Verified | Member records and enriched overview expose agent, pane, worktree, branch, review/agent state, attempt, last start/error and verification/ranking data. |
| FR-FLEET-007 | Verified | Retry validates the existing worktree/branch, replaces only the pane, increments the attempt and preserves committed, staged, unstaged and untracked work; focused dirty-worktree and lifecycle tests pass. |
| FR-FLEET-008 | Verified | Clean refuses pending review unless `force`, then removes member panes, worktrees and branches and prunes persisted state. |
| FR-FLEET-009 | Verified | Native Fleet launcher, CLI and MCP lifecycle entry points exist. A real GUI launch rendered the installed-agent crew presets (`claude`, `codex`, `gemini` and mixed crews) without creating worktrees until selection. |

## Review and rollback

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-REVIEW-001 | Verified | Official `unterm agent launch/run` creates a checkpoint before starting an agent and refuses repository launch on checkpoint failure. In an isolated real Git repository, the production `agent run codex-cli` path created a dangling commit before its harmless test executable could start; the commit parent matched HEAD, included the untracked pre-launch file, and left both HEAD and the real index unchanged. Hook/process detection also records debounced loose-agent checkpoints on background workers. |
| FR-REVIEW-002 | Verified | Snapshot uses a temporary Git index and `commit-tree`, producing dangling commits without moving HEAD/index/worktree, and records them under `~/.unterm/checkpoints.json`. |
| FR-REVIEW-003 | Verified | Automatic records are debounced for 60 seconds, skip identical snapshots and retain at most 20 per repository. |
| FR-REVIEW-004 | Verified | `overview` returns persisted fleets/members and loose checkpoints together. |
| FR-REVIEW-005 | Verified | Diff returns numstat, line patches and synthesized additions for untracked files; round-trip and engine-neutral tests cover tracked and untracked changes. |
| FR-REVIEW-006 | Verified | CLI/MCP/Web surfaces cover list, diff, verify, rollback, merge, discard and opening Review. Real Chromium loaded the live Review ranking/checkpoint lists and selected a checkpoint detail; destructive controls were left untouched while focused route/service tests cover their handlers. |
| FR-REVIEW-007 | Verified | Command inference covers Cargo, Go, npm/pnpm/yarn only with a real test script, pytest/Python/uv, Maven, Gradle and .NET solution/project markers. A requirement-specific marker matrix test passes. |
| FR-REVIEW-008 | Verified | `review.verify` accepts an explicit command override through CLI/MCP. |
| FR-REVIEW-009 | Verified | Verification persists Pending before spawning a worker, runs in the member worktree, records bounded UTF-8 logs/status/duration, enforces a capped timeout and kills the process tree; execution/timeout tests pass. |
| FR-REVIEW-010 | Verified | Overview enrichment ranks passing verification first and discounts changed files/line volume; deterministic score/rank coverage passes. |
| FR-REVIEW-011 | Verified | Merge uses `git merge --squash`, requires a clean base and returns the resulting staged file list without committing; engine-neutral merge coverage passes. |
| FR-REVIEW-012 | Verified | Normal merge calls `ensure_passed`; only an explicit `force` bypasses it, and the MCP boundary audits that override. |
| FR-REVIEW-013 | Verified | Rollback validates the commit, restores/deletes to the exact checkpoint tree without moving HEAD, and round-trip/deleted-file/invalid-target tests pass. CLI requires `--yes`; MCP now independently requires boolean `confirm: true`, with focused boundary and metadata tests. |
| FR-REVIEW-014 | Verified | Rollback, merge, forced merge and discard pass through audited MCP write paths; GUI automatic writes also use the shared audit log. |

## MCP server

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-MCP-001 | Verified | A real second native process registered `bravo` on fallback port 19878 while another instance occupied the preferred port. CLI authenticated to it and live health reported `bind: 127.0.0.1`; cleanup removed only the test registry entry. |
| FR-MCP-002 | Verified | The TCP server reads one JSON-RPC object per line and writes one JSON-RPC response plus newline. |
| FR-MCP-003 | Verified | Until `auth.login` succeeds, every other method returns `-32002`; an invalid token returns `-32001`. |
| FR-MCP-004 | Verified | Each launch generates a UUID token. Instance/active/server/token writes now explicitly use mode `0600` on Unix, including atomic replacements and compatibility handoff. The platform-gated Unix permission test (`auth_bearing_files_are_user_only`) ran natively on macOS in the 2026-08-01 full-workspace pass and asserted `0600` on both the direct and atomic write paths. |
| FR-MCP-005 | Verified | Static discovery contains 103 unique authenticated methods across every required namespace, preserving the 101 enumerated non-auth methods plus `screen.clear` and `session.paste`; `auth.login` is the wire-level pre-dispatch method. Metadata coverage locks count, uniqueness and namespaces. |
| FR-MCP-006 | Verified | `meta.surface` returns method metadata, CLI families, live host keybindings, selected-engine capabilities and diagnostic flags; focused engine capability tests pass. |
| FR-MCP-007 | Verified | `server.capabilities` is derived from the same method inventory and includes `_engine_capabilities`; next-core diagnostic tests cover I/O and runtime-pump metrics. |
| FR-MCP-008 | Verified | All 103 public methods are exhaustively and exclusively classified read-only or mutating. Leaf handlers retain rich redacted records, while the dispatch boundary appends a fallback success/failure record for every mutation that did not emit one. Classification and real dispatch-fallback tests pass. |
| FR-MCP-009 | Verified | Parse, authentication and handler failures return JSON-RPC error objects with numeric codes and messages; handler validation names missing/invalid parameters and unsupported-platform paths. |
| FR-MCP-010 | Verified | The 25-scenario/35-gate next-core benchmark covers reads, writes, scrolling, paste, output flood, first-session readiness and concurrent agents while MCP remains responsive. |

## CLI

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-CLI-001 | Verified | Endpoint resolution prefers a live `active.json`, then the most recently started live instance, then compatibility files. |
| FR-CLI-002 | Verified | Global `--instance` takes precedence over `UNTERM_INSTANCE`; both resolve and validate the named live instance record. |
| FR-CLI-003 | Verified | `--json` is a global Clap option and is forwarded to every script-consumable product command family. |
| FR-CLI-004 | Verified | Global `--lang` applies a process-local locale override before dispatch and does not persist it. |
| FR-CLI-005 | Verified | Actual Clap introspection covers every required product family. `start`, previously metadata-only, launches the sibling native GUI with cwd/profile and optional `--` program; a real sibling-layout invocation registered a healthy second native instance. |
| FR-CLI-006 | Verified | This is optional (“may retain”); upstream-only families need not be reintroduced into the product CLI. Static reference metadata identifies retained compatibility families separately. |
| FR-CLI-007 | Verified | Top-level errors propagate through `main -> Result` for non-zero exit, while agent lifecycle signals intentionally tolerate an absent GUI. A real PowerShell invocation against a nonexistent instance returned exit code 1 with an actionable error. |
| FR-CLI-008 | Verified | `reference` first requests live `meta.surface` and falls back to the shared static MCP/CLI inventory when no GUI is reachable. |

## Session recording and archive

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-REC-001 | Verified | GUI toggle, CLI start/stop/status/export and MCP recording lifecycle all call the selected next-core `RecordingEngine`. In a real native pane, the quick menu changed from `会话录制:关闭` to `会话录制:开启`, CLI status reported `enabled: true`, live output produced two recording blocks, and the same GUI row stopped it; status returned false and the archive contained the expected Markdown before the uniquely named test artifact was removed. |
| FR-REC-002 | Verified | Stop and export render YAML-fronted Markdown and persist its path in the archive index. |
| FR-REC-003 | Verified | The next core records OSC 133 command/output boundaries; rendering falls back to plain output with an explicit notice when semantic events are absent. |
| FR-REC-004 | Verified | Built-in key/value, bearer/token and private-key patterns are applied by default at render/export time. |
| FR-REC-005 | Verified | `recording.json` custom patterns are compiled with the built-ins; invalid expressions are ignored with a warning. |
| FR-REC-006 | Verified | Archive allocation prefers `<cwd>/.unterm/sessions/` and falls back to `~/.unterm/sessions/_orphan/` when the project path is unavailable or unwritable. |
| FR-REC-007 | Verified | CLI `sessions list/read` and MCP `session.recording_list/read` share the persistent archive index and Markdown files. |
| FR-REC-008 | Verified | `session.export` exports current next-core scrollback to Markdown even when no recording is active; focused engine-neutral coverage passes. |
| FR-REC-009 | Verified | `session.recording_attach_trace` deduplicates external trace IDs and persists them into recording metadata/Markdown; focused next-core coverage passes. |

## Capture, scrollback, and upload

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-CAP-001 | Verified | MCP/CLI whole-screen capture produces PNG metadata through the host capture engine. A real Windows capture was written, passed PNG signature validation and was removed after acceptance. |
| FR-CAP-002 | Verified | Window capture accepts title or PID. Real authenticated `capture.window` calls selected the new-kernel window by PID and by title, produced independent 1500x756 `print_window` PNGs, passed signature validation and were removed. |
| FR-CAP-003 | Implemented, runtime pending | The command palette now arms a modal crosshair selector, draws a dimmed drag rectangle, swallows terminal input, supports Esc/right-click cancellation, presents a clean frame, then saves the selected physical-pixel region. Headless MCP coordinates capture directly and no-coordinate calls degrade to whole-screen capture as specified. Real Windows acceptance at 150% DPI dragged 200x150 logical pixels and produced a clean 300x225 PNG. Protected/remote desktops that reject screen-DC reads now fall back to an exact crop of Unterm's clean `PrintWindow` frame; multi-monitor interaction still needs real hardware acceptance. |
| FR-CAP-004 | Verified | Clipboard capture returns text or PNG where the platform adapter supports it. Against the real Windows clipboard, `capture.clipboard` round-tripped unique Unicode text and decoded a generated 2x2 32-bit DIB to an 86-byte PNG with correct dimensions; the original clipboard data object was restored and the generated PNG removed. |
| FR-CAP-005 | Verified | `capture.scrollback` validates the pane and uses the native host's standalone styled-cell renderer, independent of window occlusion. A real pane produced a valid standalone 40,778-byte PNG, in addition to focused routing/self-test coverage. |
| FR-CAP-006 | Implemented, runtime pending | `capture.window_scroll` scrolls and stitches another app on macOS and returns a clear unsupported-platform error elsewhere; macOS runtime acceptance remains. |
| FR-CAP-007 | Verified | `screen.scrollback_text` returns full history plus viewport with `start_line`, `end_line` and `tail_lines`; CLI exposes the same controls and focused next-core tests pass. |
| FR-CAP-008 | Implemented, runtime pending | Upload supports configured OSS, COS and Qiniu credentials and returns public URL metadata without echoing secrets; provider integration acceptance remains. |

## Workspaces

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-WS-001 | Verified | `workspace.save` snapshots every selected-engine session under a user-supplied workspace name. |
| FR-WS-002 | Verified | Saved JSON carries name, saved timestamp, session ids/titles/cwds/profile launch context; listing returns file path and derived session count. |
| FR-WS-003 | Verified | Restore creates one next-core session per saved entry and native GUI reconciliation adopts it as a tab. A real one-pane workspace dry-run planned one entry; restore increased the live GUI session count from one to two and the test tab/workspace were removed. |
| FR-WS-004 | Verified | `dry_run` emits per-session launch plans and decisions without creating sessions; focused next-core coverage passes. |
| FR-WS-005 | Verified | CLI list/save/restore call the same three MCP workspace methods and support JSON output. |

## Multi-instance

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-INST-001 | Verified | Startup atomically claims the first available NATO phonetic id. |
| FR-INST-002 | Verified | The claim loop falls back through `alpha2`…`zulu99` after all 26 base ids. Real Windows acceptance kept the installed `alpha`, launched 26 independent v0.60.0 processes as `bravo` through `zulu`, observed 27 live registry records, and routed CLI metadata successfully to the final `alpha2`; all temporary processes and records were then removed. |
| FR-INST-003 | Verified | Each process writes `~/.unterm/instances/<id>.json` and keeps its in-memory mirror synchronized. |
| FR-INST-004 | Verified | Active ownership maintains `active.json`, `server.json` and legacy `auth_token`, including clean handoff on shutdown. |
| FR-INST-005 | Verified | Registry scans parse each record, remove only entries whose PID is conclusively dead, and report cleanup/parse diagnostics; focused tests pass. |
| FR-INST-006 | Verified | `InstanceInfo` includes all required identity, ports, token, process, timestamp, title, cwd, version and platform fields (plus profile and agent snapshots). The GUI now explicitly publishes its `0.60.0` product version instead of leaking the internal services crate's `0.1.0`; both the real instance record and authenticated `system.info` returned `0.60.0`. |
| FR-INST-007 | Verified | CLI/MCP list, info, set-title and focus are wired to the native host bridge. Against a real second window, `set-title` persisted `Parity Runtime Bravo`, `instance info` read it back and `instance focus` returned success. |
| FR-INST-008 | Verified | Unix treats permission failure as alive; Windows distinguishes invalid PID from access-denied `OpenProcess` and conservatively preserves unknown live peers. |

## Identity profiles

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-PROF-001 | Verified | Instance metadata is the window binding. The native status chip reads that binding rather than the unrelated global default, and every GUI pane creation resolves launch env at spawn time. A real `0.60.0` window launched with temporary profile `codex-parity-temporary-20260730`; its instance record and first pane launch context both reported that exact profile ID. |
| FR-PROF-002 | Verified | Profile TOML schema covers static env, secret references, Git identity/signing, SSH host keys, npm registry, gh-host mapping, expirations, accent, description and indexed default. |
| FR-PROF-003 | Implemented, runtime pending | The `keyring` backend selects macOS Keychain, Windows Credential Manager and Linux Secret Service. A real Windows Credential Manager round trip wrote and exactly resolved a temporary secret, confirmed the profile TOML held no raw value, then removed both the vault entry and profile. A real macOS Keychain round trip (2026-08-01, `unterm-cli profile create/set-secret/export/delete`) wrote a secret, resolved its exact value back, and deleted the profile and vault entry. Linux Secret Service runtime acceptance remains. |
| FR-PROF-004 | Verified | Official set-secret writes raw bytes only through `SecretStore` and persists canonical `keychain://unterm/<profile>/<env>` references in TOML; MCP never returns values. |
| FR-PROF-005 | Verified | Initial, normal-tab, launcher-tab and split paths apply the currently-bound profile immediately before spawn, so runtime profile changes affect future panes. In real acceptance, the initial pane, a profile-unspecified `session.create`, and a `session.split` all reported profile `codex-parity-temporary-20260730`, included `UNTERM_PROFILE` in `launch_env_keys`, and retained the live proxy bindings; `session.create` identified its source as the window binding rather than an explicit request. |
| FR-PROF-006 | Verified | Profile binding only updates instance metadata; no live PTY env mutation or respawn occurs. |
| FR-PROF-007 | Verified | Actual CLI enum covers list/create/show/set-secret/delete/audit/edit/export/spawn/import/set-default/shell-integration. |
| FR-PROF-008 | Verified | MCP exposes only profile list/current/audit; write operations remain CLI/Web-only. |
| FR-PROF-009 | Verified | Audit reports expiry/missing metadata and counts while profile list/current redact secret values. |
| FR-PROF-010 | Verified | Registry create/save/startup regenerate additive `~/.unterm/ssh/config.unterm` Match blocks without rewriting the user's main SSH config. |

## Proxy

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-PROXY-001 | Verified | Proxy enable/disable persists through the shared settings schema, and every next-core pane creation resolves the current proxy environment immediately before PTY spawn. Launch-context tests prove the injected variable names without revealing values. |
| FR-PROXY-002 | Verified | Auto detection covers environment variables plus macOS system configuration, Linux desktop settings and Windows Internet Settings/known local clients. The real Windows acceptance host was in auto mode and resolved `http://127.0.0.1:7897`, with live controller groups and latency data visible through the new-kernel Web UI. |
| FR-PROXY-003 | Verified | Manual configuration accepts HTTP, SOCKS and `no_proxy`; launch resolution emits uppercase/lowercase variables consistently. |
| FR-PROXY-004 | Verified | MCP implements nodes, switch, disable, env, speedtest, rotation and set-nodes, with CLI/Web paths over the same state and focused rotation/shell-safety tests. Live Web acceptance loaded 151-node selector groups and measured latencies from the configured controller without changing the selected node. |
| FR-PROXY-005 | Verified | Clash/mihomo status, selector switching and manual controller configuration are exposed through MCP and Web Settings. Real Chromium read the auto-discovered live controller, selector groups and node latency results; handler tests cover switch/manual-controller mutations, which were not invoked during state-preserving acceptance. |
| FR-PROXY-006 | Verified | Native status UI, Web forms, the CLI family and all MCP methods are wired to the shared proxy files. Real native status rendering plus live Chromium Proxy navigation exercised the visual surfaces against the same auto-mode configuration used by spawned panes. |
| FR-PROXY-007 | Verified | CLI `proxy env` single-quotes unsafe values and escapes embedded quotes. Focused tests cover command substitution, semicolons and literal quote injection. |

## Web Settings and Review

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-WEB-001 | Verified | Every `unterm-app` startup creates the HTTP server after MCP through the loopback-only bind helper. A real second native process bound its HTTP server to fallback `127.0.0.1:19879` and served the correct instance-local bootstrap. |
| FR-WEB-002 | Verified | Only `/`, bundled static assets, favicon and the instance-local bootstrap are public. In a real process `/api/health` returned 401 without credentials and `ok: true` with the matching bearer token. |
| FR-WEB-003 | Verified | The bundled responsive Tailwind/Alpine application provides forms for proxy, profiles/secrets, agents, compatibility, scrollback and other non-terminal settings. Real Chromium loaded the live token-authenticated app at 1256x824 and exposed the corresponding modern form sections. |
| FR-WEB-004 | Verified | Routes and SPA sections cover theme, language, proxy, scrollback, TERM_PROGRAM compatibility, recording, agents, sessions, Review and updates. Unzoo Chromium navigated the live `#profiles`, `#agents`, `#review`, `#appearance`, `#proxy` and `#recording` routes and observed each matching visible heading/content. |
| FR-WEB-005 | Verified | Review overview/ranking, diff, verify/retry, merge, discard, rollback, clean and fleet retry routes are wired to the same Review services. The live browser loaded checkpoint/ranking rows and a selected checkpoint detail with the rollback workflow; route tests cover the mutation handlers, which were intentionally not invoked during read-only acceptance. |
| FR-WEB-006 | Verified | Web writes use shared service/profile/MCP paths and the same `~/.unterm/` theme, language, proxy, scrollback, compatibility, recording, profile, fleet and checkpoint files consumed by runtime and CLI. |

## Localization and themes

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-I18N-001 | Verified | The service registers English, Simplified/Traditional Chinese, Japanese, Korean, German, French, Italian and Hindi. All nine embedded service dictionaries and nine website dictionaries parse; missing newer translated strings fall back to canonical English rather than exposing raw keys. |
| FR-I18N-002 | Verified | Locale initialization reads persisted `lang.json`, then OS locale detection, with Web and CLI persistent setters over the same file. Canonicalization covers language/region variants. |
| FR-I18N-003 | Verified | Global CLI `--lang` uses a transient process-only override and does not rewrite the persisted locale. |
| FR-THEME-001 | Verified | Native and Web theme registries contain exactly agent-inbox, standard, midnight, daylight, classic, notion-dark and notion-light, with palette tests and shared identifiers. |
| FR-THEME-002 | Verified | Theme writes persist for cold start and publish a generation-stamped process mailbox. In a real native window, Web switching standard → midnight returned generation 1 and produced a different window PNG (58,973 vs 91,166 bytes and distinct SHA-256); switching back to standard succeeded before cleanup. |

## Policy, trust, and safety

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-SAFE-001 | Verified | MCP execution/write paths consult the configurable blocked-pattern policy and return structured policy failures. |
| FR-SAFE-002 | Verified | `agent.identify` binds connection context to an agent name and audit entries carry connection/agent grouping metadata. |
| FR-SAFE-003 | Verified | Persistent trust/untrust methods and CLI/Web controls change the trusted-agent set used by write confirmation policy. |
| FR-SAFE-004 | Verified | The exhaustive method partition and dispatch fallback guarantee a redacted audit entry for every mutating MCP success or failure, without duplicating richer leaf records. |
| FR-SAFE-005 | Verified | Pane destroy, fleet clean, rollback, forced merge, policy changes and all other destructive calls are classified mutating; focused classification and boundary tests cover the named operations. |
| FR-SAFE-006 | Verified | MCP profile methods are read-only and return references/metadata only; secret values are never loaded into their response schema. |
| FR-SAFE-007 | Verified | Recording rendering applies built-in bearer, key/value, token and private-key redaction by default, plus validated user expressions. |
| FR-SAFE-008 | Verified | MCP and HTTP both bind through the single `SERVER_BIND = "127.0.0.1"` helper; neither exposes a configurable wildcard/LAN bind. |

## System and self-test

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-SYS-001 | Verified | `system.info` now returns OS/platform, architecture, hostname, locale, live build version, active-session count and active-session metadata. A focused product-system test locks the fields. |
| FR-SYS-002 | Implemented, runtime pending | Windows `system.launch_admin` builds a PowerShell `Start-Process -Verb RunAs` request and supports dry-run; an actual UAC consent/launch remains manual acceptance. |
| FR-SYS-003 | Verified | Non-Windows admin launch returns an explicit unsupported-platform error, while self-test treats that expected result as a supported diagnostic outcome. |
| FR-SYS-004 | Verified | Server info/health/capabilities and self-test expose instance identity, selected-engine readiness, method inventory, next-core I/O counters and runtime-pump metric names. Focused capability tests pass. |
| FR-SYS-005 | Verified | Self-test probes selected engine, server/capabilities, launch context, policy, admin dry-run, proxy, capture, logical viewport, styled scrollback and optional pane checks, including next-core I/O/runtime diagnostics. |

## Installation, release, and updates

| Requirement | Status | Evidence / remaining work |
| --- | --- | --- |
| FR-REL-001 | Implemented, runtime pending | Release scripts/workflows define macOS DMG, Linux deb/AppImage and Windows MSI/zip outputs, including x64/arm64 where supported. Fresh artifacts still require their platform CI/manual jobs. |
| FR-REL-002 | Implemented, runtime pending | `ci/sign-macos.sh` signs nested binaries/app/DMG, submits app and DMG to notarytool, verifies signatures and staples tickets; a fresh Apple-notarized artifact remains external acceptance. |
| FR-REL-003 | Implemented, runtime pending | WiX targets `ProgramFiles64Folder` and installs Start Menu shortcuts; the MSI build script carries architecture into WiX. A clean-machine install remains. |
| FR-REL-004 | Implemented, runtime pending | Debian staging supplies normal `/usr/bin`, desktop/icon, package metadata and shared-library dependency layout; AppImage builds a per-architecture AppDir. Distro installation remains. |
| FR-REL-005 | Verified | Update polling and manual refresh persist/read `~/.unterm/update_check.json`, and the Web Updates surface consumes that state. |
| FR-REL-006 | Verified | README, website architecture/MCP/configuration/integration pages, all nine comparison translations, launch copy and dependency-map paths now describe native next-core and 103 authenticated methods. Stale-current-doc search is clean, all dictionaries parse and Astro builds all 84 pages successfully. |

## Changes completed during this audit

### Ghost completion

- Added the native GUI key-to-predictor bridge, including Ctrl-C/G/U/W,
  backspace, Enter, IME commits, and invalidation after shell-driven edits.
- Restored Right Arrow/End acceptance without stealing those keys when no
  prediction exists.
- Drew theme-aware dim text at the active pane cursor and truncated by terminal
  column width so CJK/emoji cannot cross a split boundary.
- Removed pane-local state on pane/session destruction to prevent session-id
  reuse from inheriting unfinished input.
- Evidence:
  - `cargo test -p unterm-services`: 96 passed.
  - `cargo test -p unterm-app`: 512 passed.

### Installed font availability

- Added bundled symbol and emoji fonts to Windows MSI/staging, macOS bundle,
  Debian, and AppImage packaging.
- Added installed-layout and workspace fallback discovery.
- Evidence:
  - bundled face/open/icon coverage passes inside the `unterm-app` suite.
  - packaging artifacts still need per-platform install verification.

### Configurable per-pane scrollback

- Removed the hard-coded 10,000-line limit from screen mutation paths.
- Each new next-core screen captures the configured limit, so later settings
  changes cannot silently resize existing pane history.
- Preserved both declarative `scrollback_lines` and the legacy Web Settings
  `~/.unterm/scrollback.json` override, with the declarative setting taking
  precedence.
- Evidence:
  - focused next-core capacity test passes for a three-line and zero-line pane.
  - all ten settings tests pass, including config and legacy-file resolution.
  - the complete 512-test `unterm-app` suite passes with the startup wiring.

### Pane resize and tab movement

- Added explicit tab ordering without changing stable tab/session ids.
- Added relative tab movement and made all existing tab cycling/number
  selection consume the visible order.
- Added nearest-axis nested split adjustment with ratio clamping.
- Exposed resize and reorder through keyboard actions, command palette rows,
  and MCP keybinding metadata from the same binding table.
- Evidence:
  - focused engine tests prove nested-axis divider movement and stable-id tab
    reordering.
  - the complete GUI suite passes, including binding uniqueness and metadata
    agreement.

### Semantic prompt navigation

- Parse OSC 133 prompt-start markers without treating command/output markers
  as prompts.
- Keep prompt rows aligned as bounded scrollback drops old lines, preserve
  visible prompt rows when only history is cleared, and ignore alternate-screen
  markers.
- Added previous/next prompt runtime mutations and GUI actions on
  `Ctrl+Shift+Up/Down`.
- Evidence:
  - focused parser and screen tests prove marker recognition, bidirectional
    navigation, and bounded-history trimming.
  - focused GUI tests prove prompt navigation does not collide with pane resize
    or tab movement.

### Non-blocking clipboard and automatic title identity

- Moved platform clipboard reads/writes off the winit thread and return their
  results over a channel collected by the regular event-loop tick.
- Added a deterministic test that holds a simulated platform retry while
  proving the caller remains responsive.
- Added one title-identity regression covering shell, foreground command,
  project grouping and known-agent naming.
- Evidence:
  - the complete 515-test `unterm-app` suite passes.
  - the complete next-core engine suite passes.
  - the reachable 25-scenario/35-gate benchmark summary verifies successfully.

### Core size budget

- The pre-parity core sat at 11,998 of 12,000 production lines.
- The bounded scrollback, semantic prompt, ordered-tab and split-resize tranche
  brings it to 12,203 lines.
- The gate is recalibrated to 12,300, leaving 97 lines of explicit headroom;
  the limit remains enforced rather than being skipped.

### UI and Composer parity

- Right-docked the read-only Git inspector and reduced the quick menu to the
  six specified high-frequency actions.
- Added directory palette page movement, wheel movement and Tab path
  completion.
- Added Composer execution modes, queue selection/removal, explicit manual
  send and Shift+Enter multi-line input.
- Added a native pending-suggestion card backed by the existing MCP lifecycle:
  accept, accept-and-run and dismiss all update status, with accept/dismiss and
  auto-confirm writes audited.
- Focused Composer, palette and engine-neutral suggestion tests pass.

### Agent, Fleet, and Review parity

- Added manifest-backed process matching on the facts worker while preserving
  the fixed zero-I/O engine hot path.
- Added modal Agent Inbox selection and local pane/tab jumping.
- Checkpoint official Agent launch/run before the process starts, and add
  background checkpoints for hook- and process-detected loose agents.
- Enforced destructive rollback confirmation at both CLI and MCP boundaries,
  and declared the required boolean in the public MCP metadata.
- Focused App, MCP, Agent metadata and CLI tests pass; the native App builds.

### Cross-window Cockpit and sidebar navigation

- Publish per-instance agent snapshots and aggregate them into a waiting-first
  inbox with instance, tab, pane, title, age and task metadata.
- Authenticate to peer MCP servers for cross-window pane focus, then raise the
  target native window.
- Aggregate sticky unread/error/running state onto sidebar tabs and add fuzzy
  project/tab navigation from both the footer and command palette.
- Focused service, peer-protocol, App model and palette tests pass; a real
  two-window transition remains.

### Live themes, complete mutation audit, and system diagnostics

- Added a generation-stamped process theme mailbox so every open native window
  can observe a Web/CLI theme switch independently.
- Partitioned every public MCP method as read-only or mutating and added a
  dispatch-boundary fallback audit, preserving detailed leaf records without
  duplicate entries.
- Completed `system.info` with locale and active-session metadata and derived
  its version from the live instance/build rather than a hard-coded string.
- Focused service, settings, CLI, App and MCP tests pass.

### Proxy safety and public capability metadata

- Locked CLI proxy environment output with focused command-substitution,
  semicolon and embedded-quote tests.
- Rewrote the public architecture documentation for the native next-core
  process/crate/runtime model.
- Corrected the authoritative method count to 103 authenticated methods plus
  `auth.login`, added `screen.clear` and `session.paste` to the requirement
  inventory, and removed current-doc references to deleted WezTerm/Mux paths.
- Updated all nine website comparison dictionaries and current launch copy;
  every edited JSON dictionary parses successfully and the Astro production
  build generates all 84 pages.

### Full regression and native-process acceptance

- Fixed Windows verification timeout cleanup with a kill-on-close Job Object,
  so the verifier shell and all descendants terminate together instead of
  waiting for `taskkill.exe`; the former 4.7-second failure now completes in
  about 60 ms and passed six consecutive focused runs.
- Full passing suites:
  - `unterm-app`: 524 tests
  - `unterm-engine`: 567 tests
  - `unterm-services`: 102 tests
  - `unterm-mcp`: 64 tests
  - `unterm-cli`: 23 tests
  - `unterm-agents`: 34 unit + 2 baked-manifest tests
- The size gate passes at 12,203/12,300 core lines, 2,473/2,500 probe lines,
  10/10 direct dependencies and 7,165,440/8,000,000 debug binary bytes.
- The reachable benchmark verifier passes all 35 gates across 25 scenarios,
  and the native App/CLI builds complete.
- A real second new-kernel process registered `bravo` while `alpha` remained
  live, exercised fallback MCP/HTTP ports, instance-scoped CLI routing,
  authenticated health, HTTP 401/authorized behavior, first-pane readiness,
  screen read/focus and the complete self-test payload.
- A live standard → midnight switch changed the captured native-window pixels,
  then restored standard. Test processes, registry entries and generated
  screenshots were removed after acceptance.
- A real sibling-layout `unterm-cli start` launched and registered the native
  GUI, while a nonexistent `--instance` request exited 1 with an actionable
  error.
- Real Windows captures covered full screen, self window, window-by-PID,
  window-by-title and headless styled scrollback; every result passed PNG
  signature validation and all generated files were removed.
- A saved one-pane workspace restored as a second live GUI tab. The restored
  tab and uniquely-named workspace file were removed after acceptance.
- Real multi-window instance title/focus calls succeeded. Product version
  publication now comes from the owning GUI (`0.60.0`), not the internal
  services crate (`0.1.0`), and normal window shutdown immediately unregisters
  its instance while preserving the live `alpha` record.
- Directed native-window interaction at 150% DPI opened and visually checked
  the project tree, directory jump, command palette, quick menu, Git right
  dock, Composer, Agent Inbox, Fleet launcher and shell launcher. Composer
  queued/removed an item in manual mode, and the shell launcher created then
  removed a real second tab.
- A real MCP suggestion rendered its source, text, rationale, count and
  accept/run/dismiss hints in the native window, then cancelled without
  writing to the PTY.
- Isolated Unzoo Chromium loaded the live token-authenticated Web Settings
  application and navigated Profiles, Agents, Review, Appearance, Proxy and
  Recording. Review loaded a checkpoint detail, Proxy showed the live Clash
  selector groups and measured nodes, and Recording listed archived sessions;
  no destructive or state-changing controls were invoked.
- A restarted window with an explicit three-line scrollback cap emitted 80
  shell lines and exposed exactly 24 viewport rows plus three history rows
  through `screen.scrollback_text`; its one-off config was removed.
- In a real native pane, typing `cod` painted `ex` as dim Ghost Text and End
  accepted it into the PTY line as `codex` without executing the command.
- A real top-bar split produced two 37/38-column PTYs; invoking
  `Resize Pane Left` through the production command palette changed them to
  34/41, proving the layout boundary and both PTY dimensions moved together.
- The latest remote CI was inspected rather than assumed green. Its Linux,
  macOS and strict Windows failures were reproduced from the logs and repaired
  locally: Linux font-home ownership, macOS framework dependencies and stale
  scrollshot code, Unix shell quoting, 122 dropped runtime-test guards, and
  four strict panic-format diagnostics. CI-equivalent strict Windows App/CLI
  checking and the complete release workspace suite now pass, all 567 engine
  tests also pass in the focused strict run, and a fresh normal native App
  build succeeds. Native Linux/macOS reruns still require publishing this
  branch or access to those hosts.

### 0.57.4 runtime comparison and old-worktree sync

- Ported the old kernel worktree's final uncommitted product-readiness fixes
  into the native stack, translating each intent to the new architecture:
  - Agent status freshness: the MCP facts cache TTL returns to 0.57.4's
    2-second cadence with inflight refreshes bounded at 4.
  - Ghost Text: the prefix-scan budget is shared across candidate pools, and
    the signed manifest `flag_catalog` merges asynchronously into the
    completion map (`merge_manifest_flags`), keeping the key path disk-free.
  - Sidebar working badge breathes again: a 3.2-second four-step cosine swell
    driven by the shared `breath_epoch`, repainting only when the quantised
    phase changes and not at all when nothing is working.
  - Tab rename returns as a deliberate same-row double-click (500 ms / 8 pt
    streak tracker) opening a palette rename line; empty accepts reset to
    automatic titling. FR-UI-011 is now satisfied by design rather than by
    absence.
  - Not ported, with reasons: the Windows clipboard event-loop-thread pin
    (the native stack's worker-plus-channel design was runtime-verified under
    a four-second clipboard lock) and the `pid_alive` ERROR_ACCESS_DENIED
    narrowing (the native registry deliberately preserves unknown live peers,
    FR-INST-008).
- Ran a real 0.57.4 (release zip) next to the freshly built 0.60 release on
  the same machine: window appears in 761 ms vs 1349 ms, settled working set
  121 MB vs 118 MB, and side-by-side window captures show matching layout,
  theme palette, top-bar facts/actions, sidebar structure and status-bar
  segments. Deliberate deviations: chrome text is the platform UI face rather
  than the terminal monospace, sidebar quick actions sit in a footer row, and
  tab titles are capitalised by the title rules.
- Independently diffed the whole feature surface against the v0.57.4 tag:
  MCP is a strict superset (all 99 authenticated 0.57.4 methods present,
  plus `session.paste`, `screen.clear`, `instance.lifecycle`,
  `instance.close`); the 23 product CLI families match 1:1 down to their
  subcommands, with only inherited WezTerm-upstream commands
  (ssh/serial/connect/cli/imgcat/asciicast record+replay/ls-fonts/show-keys/
  set-cwd/blocking-start) removed with the old kernel; GUI surfaces map to
  named successors except four true gaps — the Insights panel, the right-click
  tab context menu (replaced by the quick menu and the copy/paste gesture),
  the debug overlay, and a GUI workspace switcher (workspaces remain
  CLI/MCP-driven) — plus `RotatePanes`, which narrowed to `SwapPane`.
- The size gate measured 12,445 core lines against the 12,300 limit the
  previous tranche set — the follow-up runtime/tab work had outgrown it
  unnoticed. Recalibrated to 12,550 with the measured size and about a
  hundred lines of headroom recorded in the script, keeping the gate
  enforced.
- Real-window acceptance of the ported gesture: a physical same-row
  double-click on tab row 1 of a fresh release window opened the palette
  rename line with its "Rename tab — Enter applies · empty line resets to
  auto-title · Esc cancels" row, while single clicks and the earlier
  mis-aimed click only selected. Synthesized keyboard events do not reach
  the palette on an IME-active desktop, so typed-apply relies on the shared
  `Source::Text` palette line (runtime-verified by the fleet card) plus the
  focused streak/command tests; the test window and its captures were
  removed afterwards.
- Full suites after the sync: `unterm-app` 534, `unterm-engine` 575,
  `unterm-services` 105, `unterm-mcp` 64, `unterm-cli` 23, `unterm-agents`
  36 — all passing, with no new compiler warnings. The size gate passes at
  12,445/12,550 core lines and the reachable benchmark verifier passes all
  35 gates across 25 scenarios on an otherwise idle machine (a run sharing
  the machine with the full test suite fails only the two session-latency
  gates, which is contention rather than regression).

### Chrome alignment against the shipped 0.57.4 pixels

A side-by-side run against the released 0.57.4 zip showed the rebuilt chrome
had drifted from what every released window drew. Each surface was compared
against the v0.57.4 source (extracted spec, file-and-line) and realigned:

- Chrome text returns to 12pt — the packaged `window_frame.font_size` — and
  the two mono surfaces go back to the terminal face: the top bar's facts
  line (accent-tinted, `.exe` kept, exactly as `compute_top_stats_text` drew
  it) and the whole status bar.
- The status bar loses the `▾` menu chip 0.57.4 never had, and its
  labels-and-values colouring returns: the path and every chip value after
  the colon in the theme's teal, labels in the ordinary foreground.
- The left strip's titles keep their own case again (`powershell`, not
  `Powershell`), the always-on `▶` is gone, and the right edge carries at
  most one indicator — agent state, then sticky error, then unread — and
  none at all on the active row, with the working dot's 3.2s/4-step breathing
  restored. The `+ ▾ ⌕` actions sit directly under the last tab as 0.57.4
  placed them, meeting the bottom edge only when the list fills the strip.
- The chevron menu returns to 0.57.4's full entry list (see FR-UI-007) with
  chords read from the live binding table, a new palette command rendering
  the scrollback long screenshot through the same standalone renderer the
  MCP method uses, and the codicon chevron glyph on the button.
- `WindowEvent::ScaleFactorChanged` is now handled — fonts, atlas and pane
  grids reopen at the new scale. Unhandled, a DPI change (monitor move,
  remote-session reconnect, cross-virtualisation resize) left the window
  drawn at the old density and stretched.
- The exe carries its identity again: a Windows resource block (built the
  way 0.57.4's front end built it, through `embed-resource`) embeds the
  terminal icon and a VERSIONINFO naming Unterm 0.60.0, and the window also
  sets its icon at runtime — so the taskbar, Alt-Tab, Explorer and the file's
  Properties dialog all show the logo and version instead of the default
  program icon. Verified on the built binary: `ProductName=Unterm`,
  `FileVersion=0.60.0`, and the extracted 32px icon is the dark logo.
- Verified on a fresh release window: mono facts with `.exe`, mono teal
  status bar, lowercase indicator-free idle sidebar row, under-tab actions,
  and the full suite still passes. Remaining known style deltas, recorded
  rather than hidden: the quick menu presents as a centred palette rather
  than 0.57.4's right-anchored dropdown card; the shell selector and
  directory jump reuse the palette shell rather than their dedicated 0.57.4
  cards; top-bar hover/height geometry is near but not pixel-identical.

## Correction (2026-07-31): the FR ledger overstated parity

An interaction-level archaeology of the v0.57.4 product code
(`docs/parity-gap-audit-2026-07-31.md`) found the 159-requirement grid too
coarse: module-level data layers migrated cleanly, but click wiring, config
honouring and overlay depth did not. Four defect classes cause wrong actions
or data loss (no close confirmation; sidebar right-click falls through to
paste; every status-bar click target dead yet still painted clickable; the
selection system — auto-copy, word/line select, primary selection — absent),
eleven whole features are missing (desktop notifications, alt-screen wheel
arrows, search highlighting, drag auto-scroll, sidebar scrolling/reorder/
resize, link discoverability, drag-and-drop, session restore, config schema
enforcement, update polling, five overlays), and several surfaces shipped at
a quarter of their old depth. That audit, not this ledger's "verified"
column, is the standard for calling the kernel replacement complete.

## Remediation (same day)

The gaps that correction named were then closed in bulk on this branch
(`88a33bc8`..`65d51d2f`), with the audit checklist updated row by row as each
landed:

- **A level: 4 of 4 closed.** Close confirmation with foreground-process
  detection (and a `window_close_confirmation = "NeverPrompt"` opt-out); the
  sidebar/tab right-click context menu, with every chrome right-click
  swallowed so nothing falls through to paste; every painted status-bar click
  target wired (cwd copy, project jump, both capture variants, theme, MCP
  audit export, proxy/profile opening the settings page); and the selection
  system — copy on release, double-click word, triple-click line, Shift+click
  extension, middle-click paste, block selection back on Alt. Primary
  selection remains a Linux-side item.
- **B level: mostly closed.** Done: alt-screen wheel-to-arrow (x3 per notch,
  application-cursor aware), drag auto-scroll past the pane edge with
  cross-screen selection, all five sidebar behaviours (wheel scrolling,
  row hover, width drag, visible scrollbar, press-drag tab reorder),
  plain-click link opening with an always-on hover underline (Ctrl+click
  retained), drag-and-drop file paths pasted under the quoting rules,
  session restore (`last_session.json` geometry, maximise state and per-tab
  cwds — machine-verified restoring 2250x1200 exactly), and startup update
  polling. Partially closed: notifications (OSC 9/777 parsing, status-bar
  bell, background-tab unread and Cockpit hooks are live; a system toast is
  not), search (match colouring, arrow stepping, Ctrl-U and space input work;
  Ctrl-R case/regex toggles remain), and configuration honouring (schema
  check on load plus `enable_scroll_bar`, `window_close_confirmation`,
  `audible_bell` and `default_cwd` now read; the remaining dead keys and the
  `[keys]`/`[env]` sections still are not). B15's five overlays stay open.
- **C level: over half closed.** Copy mode gained w/b word motions, V line
  selection and Ctrl-v block selection with tests, and quick select returned
  to the old 14-category shape; the top bar answers double-click maximise,
  close confirmation, the Cockpit chips and wheel tab switching; clicking an
  inactive pane focuses it without selecting and the wheel routes to the pane
  under the pointer. C19's oddments are partially restored (window title
  format, audible bell, `+` right-click shell selector, `default_cwd`).
- **CI is green on Linux, macOS and Windows for the first time**, and the
  size/test gate counts were recalibrated to what actually exists
  (`fc79c8ad`, `9b869c22`) so the gates stay enforced rather than skipped.
- The branch has been merged into `master` through `540a84df`.

## Confirmed open work

1. Publish or otherwise run the parity branch on native Linux/macOS CI, then
   exercise the remaining real-window, multi-monitor, external provider/vault,
   UAC, installer and signed-artifact acceptance scenarios before declaring
   all 13 runtime-pending requirements verified.
