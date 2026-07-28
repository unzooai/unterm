# unterm-engine

Engine-neutral terminal traits plus the experimental `next-core` implementation.

This crate is the boundary for moving Unterm product behavior away from WezTerm GUI internals. The GUI still uses the WezTerm adapter, but next-core can now be built, tested, and probed without starting the GUI.

## Probe

Build and run the standalone next-core probe:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --wait-ms 500 -- cmd.exe /c "echo next-core-probe"
```

The probe creates a next-core session, optionally writes text or runs a command, waits, then prints the visible screen snapshot plus compact `health_io`, `health_lifecycle`, and `health_runtime_pump` summary lines.

On Windows, the command above should print `next-core-probe`. A blank snapshot means the standalone ConPTY smoke path has regressed before the GUI is involved. The in-memory screen model is cols-aware: printable text wraps at the configured width, DECCKM `CSI ? 1 h/l` switches cursor/navigation key input into and out of application cursor mode, DECCOLM `CSI ? 3 h/l` switches between 132/80-column screen modes with DECRQM reporting, cursor blink `CSI ? 12 h/l`, application keypad `ESC =`/`ESC >` and `CSI ? 66 h/l` modes are tracked for query/reporting, DECLRMM `CSI ? 69 h/l` plus DECSLRM `CSI Pl ; Pr s` left/right margins constrain horizontal cursor movement and wrapping, focus event reporting `CSI ? 1004 h/l`, synchronized output `CSI ? 2026 h/l`, meta-sends-escape `CSI ? 1034 h/l`, mouse reporting/encoding `CSI ? 1000/1002/1003/1005/1006/1007/1015/1016 h/l` modes, and alternate-screen `CSI ? 47/1047/1049 $ p` DECRQM reports are tracked per mode for query/reporting with reverse-video state isolated between main and alternate screens, XTWINOPS title stack `CSI 22/23 ; 0/2 t` is modeled for session title metadata, DECAWM `CSI ? 7 h/l` controls automatic wrapping at the right edge, DECOM `CSI ? 6 h/l` makes cursor positioning relative to scroll regions and constrains vertical cursor motion to the active scroll region, DECSCNM `CSI ? 5 h/l` applies reverse-video mode to styled cells, IRM `CSI 4 h/l` inserts printable cells before overwriting, combined mode set/reset parameters are applied together, HT/VT/FF/CHT moves the cursor or viewport through tab/newline semantics, HTS/TBC set and clear custom tab stops, CBT reverse-tabs to prior tab stops, SL/SR `CSI Ps SP @/A` horizontal scroll shifts the active row range, ESC charset/UTF-8 designators are consumed without leaking designator bytes, DCS/APC/PM/SOS control strings are consumed through BEL or ST terminators, C1 CSI/OSC/string-control and IND/NEL/RI forms are handled even when PTY chunks split the sequence, OSC 8 hyperlinks are preserved on styled cells, DECALN `ESC # 8` fills the viewport for alignment tests, DECFRA `CSI Pc;Pt;Pl;Pb;Pr $ x` fills clipped rectangular cell regions with current attributes, DECERA `CSI Pt;Pl;Pb;Pr $ z` erases clipped rectangular cell regions with current attributes, DECSCA tracks protected/erasable cells, DECSED `CSI ? Ps J`, DECSEL `CSI ? Ps K`, and DECSERA `CSI Pt;Pl;Pb;Pr ${` selectively erase only erasable cells, DECCARA `CSI Pt;Pl;Pb;Pr;Pm $ r` changes rectangular cell attributes for documented SGR modes, DECRARA `CSI Pt;Pl;Pb;Pr;Pm $ t` reverses rectangular cell attributes for documented SGR modes, ESC IND/NEL/RI, RIS, and IL/DL line mutation honor scroll regions with index/reverse-index scrolling only at the active region boundaries, DECSTR soft reset keeps the current alternate screen active while resetting modes, CSI CNL/CPL/HPA/HPR/VPA/VPR plus CSI/DEC private save/restore cursor preserve cursor position and styled attributes, erase-line/erase-character styled blank backfill and display erase mode 2 preserves cursor position, insert/delete-character operations preserve cells outside the active right margin, delete-character right-margin backfill, REP repeat-character, `CSI 3 J` scrollback-clear, SGR bold/faint/italic/underline-style/underline-color/strikethrough/hidden/overline/blink/vertical-align/inverse styles plus semicolon/colon extended-color sequences are handled, logical viewport position remains stable when the scrollback ring trims old rows and returns to live-tail following when scrolled to the bottom, wide cells wrap before the edge, zero-width combining marks attach to the preceding visible cell without advancing the cursor or replacing the base character, and resize clamps existing rows to the new column count.

Machine-readable probe output:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --json --wait-ms 500 -- cmd.exe /c "echo next-core-probe"
```

The JSON output includes the session snapshot, screen snapshot, activity snapshot, engine health, raw output byte count, visible text, styled render-frame snapshots, render draw-plan snapshots, render geometry-plan snapshots, render submission-plan snapshots, and render commit-plan snapshots. Use this for CI or agent dogfood checks instead of parsing the human-readable snapshot. In `next-core`, session snapshots include `dead_reason` for exited/closed PTY paths, `shell.launch_context` summarizes the selected profile id plus proxy env key names without exposing env values, terminal status/cursor/private cursor/text-area-size/headless window-pixel-size/mode-report/primary and secondary device-attribute queries, including parameterized DA forms, are answered through a PTY writer query scanner that preserves input order across split chunks, styled render-frame snapshots expose full-frame and dirty-row deltas for the future renderer contract, render draw-plan snapshots expose full and unchanged-delta glyph runs, cell style runs, and cursor draw state for the future GPU renderer contract, render geometry-plan snapshots expose 8x16 cell-metric pixel rectangles for renderer layout smoke checks, render submission and commit plans expose damage/text/background/cursor commands plus first-frame full-repaint state for renderer consumption smoke checks, `health.lifecycle` summarizes session create/destroy/dead counters and the latest death reason, `health.io` summarizes input writes/bytes, output chunks/bytes, paste counts/text bytes, screen reads, and viewport scrolls across live sessions, and `health.runtime_pump` reports lane dispatch counts, response wait/immediate-completion counts, and total/max dispatch and drain latency.

Launch-context probe:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --json --wait-ms 500 --env UNTERM_PROFILE=bench-profile --env HTTPS_PROXY=http://127.0.0.1:7890 --write "echo launch-context`r"
```

The `--env KEY=VALUE` probe option is intended for diagnostics and tests. JSON output reports env key names, profile id, proxy env key names, and env key count; secret values remain redacted.

## Benchmark Report

Run the Phase 2 next-core benchmark suite and generate `docs/next-core-benchmark-report.md` plus `docs/next-core-benchmark-summary.json`:

```powershell
.\unterm-engine\bench-next-core.ps1
```

The runner builds `unterm-next-core`, verifies the machine-readable `--json` probe output, executes the current input-write/input-burst/echo/output/scrollback/viewport-scroll/viewport-page-cycle/viewport-scroll-under-flood/paste/paste-under-output-flood/dual-agent/agent-startup-stall/screen-read/render-frame empty, dirty, cursor-move delta, application-cursor-move delta, render draw-plan/render geometry-plan/render submission-plan/render commit-plan/focus-switch/session-create/session-ready benchmarks, writes human-readable summary and raw output into the Markdown report, and writes machine-readable gate results into the JSON summary. Any failed benchmark or gate exits non-zero.

Verify an existing JSON summary without rerunning the benchmark suite:

```powershell
.\unterm-engine\verify-next-core-benchmark.ps1
```

The verifier checks the required gate and benchmark names, requires every gate to pass, and exits non-zero if the summary is missing or stale in shape.

Verify that next-core stays within its lightweight source/dependency/binary budget:

```powershell
.\unterm-engine\verify-next-core-size-budget.ps1
```

The size verifier checks the next-core source line budget, standalone probe line budget, direct dependency count, and debug binary size. Use `-SkipBinarySizeCheck` only when CI verifies a source snapshot without a built `target\debug\unterm-next-core.exe`.

CI can use the lightweight wrapper to verify the committed summary:

```powershell
.\ci\next-core-benchmark.ps1
```

Use `-RunBenchmark` when the job should refresh the Markdown/JSON artifacts before verifying them. The wrapper also runs the size budget verifier unless `-SkipSizeBudget` is passed.
It also runs the GUI-side next-core render replacement contract tests through `ci\next-core-gui-render.ps1` unless `-SkipGuiRender` is passed.

## Experimental MCP Engine Selector

The GUI defaults to the current WezTerm-backed engine. For MCP/product-service experiments, start Unterm with:

```powershell
$env:UNTERM_ENGINE = "next-core"
cargo run -p unterm -- start
```

This routes the engine-neutral session/screen/input/paste calls through next-core where supported. It does not make next-core the default GUI renderer yet.

## Experimental next-core GUI Pane

`UNTERM_NEXT_CORE_WEBGPU_PANE` drives the active pane from next-core inside the real GUI window. It is independent of `UNTERM_ENGINE`, which only affects MCP/product services.

```powershell
# Draw next-core's frame on top of the legacy pane, for visual comparison.
$env:UNTERM_NEXT_CORE_WEBGPU_PANE = "append"

# Replace the legacy pane: next-core owns the pixels *and* the keyboard.
$env:UNTERM_NEXT_CORE_WEBGPU_PANE = "replace"
cargo run -p unterm -- start
```

Unset or unrecognized values leave the legacy renderer fully in charge. WebGPU is required — the mode is a no-op on the Glium front end.

The two modes differ in who owns input:

- `append` draws next-core's frame but leaves keyboard, paste, and IME with the legacy pane. The pane you read is still the pane you type into.
- `replace` hides the legacy pane's quads and hands next-core the keyboard, clipboard paste, and IME-composed text. Input follows the pixels: routing keystrokes to a session the user cannot see would make the visible pane look dead.

In `replace` mode the pane gets its own next-core session on first paint, sized to the pane's viewport and started in the pane's current directory with the same proxy env `spawn` injects. The legacy WezTerm pane keeps running underneath — it still owns tab and split geometry — so the window holds two shells until the pane closes.

Pane ids and next-core session ids come from independent allocators and overlap numerically, so the binding is explicit (`engine::pane_binding`) and one-to-one in both directions. An unbound pane resolves to an error rather than to whichever session shares its number. Closing the pane destroys its next-core session; resizing the pane resizes the session on the next frame.

Gate the GUI pane contracts with:

```powershell
.\ci\next-core-gui-pane.ps1
```

This covers the pane/session binding invariants, the GUI key encoder, and an end-to-end check that encoded keystrokes reach a real shell and its output comes back on the next-core screen.

In `replace` mode next-core drives every visible pane's pixels, keyboard, clipboard paste, IME text, mouse reporting, wheel scrolling, resize, and teardown. Replacing is all-or-nothing across a tab: if any pane is not ready to draw, the whole tab keeps the legacy renderer rather than leaving a blank rectangle.

## next-core Panes

`UNTERM_NEXT_CORE_PANE=1` makes every mux pane *be* a next-core session instead of a local PTY terminal. No second shell runs underneath: `LocalDomain::spawn_pane` asks the alternate-pane factory before it opens a PTY, so nothing local is created at all.

```powershell
$env:UNTERM_NEXT_CORE_PANE = "1"
cargo run -p unterm -- start
```

On the WebGpu front end this one flag is enough: a pane that *is* a next-core session is drawn by next-core, because keeping the legacy renderer in charge there would mean maintaining two renderers for the same content. `UNTERM_NEXT_CORE_WEBGPU_PANE` remains as an override:

- unset — next-core draws its own panes (and a typo in the value does not silently hand them back to wezterm).
- `append` — draw next-core's frame on top of the legacy one, to compare them.
- `off` / `0` / `legacy` — keep the legacy renderer while investigating.

Without next-core panes the render flag behaves as before: unset means the legacy renderer stays fully in charge. Requires `front_end = "WebGpu"`; the mode is a no-op on the Glium front end.

With both flags on, next-core owns the terminal *and* the pixels: verified by screenshot against a live shell, with `replace_ready=true` and no fallbacks on any frame. Text, CJK, and the cursor render correctly through next-core's own GPU pipeline while the legacy pane quads stay suppressed. Splits work too — `LocalDomain::split_pane` routes through `spawn_pane`, so a split gets its own next-core session, and a two-pane window renders both through next-core with the placement offset putting each in its own half.

Three defects had to be fixed to get there, all of which only surfaced by running the GUI:

- The buffer plan was read through `engine::current()`, which follows `UNTERM_ENGINE` and defaults to wezterm, so a next-core session id went to the WezTerm engine and every frame failed with "session not found". The mode was inert unless `UNTERM_ENGINE` happened to be set too.
- `read_render_commit_plan` skips a revision it has already submitted, which is right for damage tracking but leaves a static screen with no geometry at all. The last drawable plan is now replayed so the pane keeps drawing.
- Text runs pushed a solid quad filling the run's rect in the foreground colour — a leftover from before the textured glyph pass existed. Once the glyph pass worked, that quad painted opaque blocks over every glyph.

The pane owns its session's lifetime and destroys it on drop. Because next-core runs its own PTY pump it hands the mux no reader, so the pane carries a change watcher that samples the screen revision and raises `PaneOutput`; without it the GUI would never be told to repaint.

Known gaps at this stage:
- Selection and copy still read the legacy pane's screen, so selecting inside a replaced pane copies from the hidden session.
- Overlays (copy mode, launcher, debug output) keep their pane on the legacy renderer entirely.
- next-core implements neither the kitty keyboard protocol nor win32-input-mode, so keys are sent in their plain xterm forms. Applications that negotiate those protocols will not see the extended encodings in a replaced pane.
- Real X10 mouse tracking (`CSI ? 9 h`) is not parsed; `CSI ? 1000/1002/1003 h` are.

Interactive input smoke test:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --wait-ms 1200 --write "echo next-core-write`rexit`r" -- cmd.exe
```

Input write benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-input-writes 1000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The input write benchmark measures the engine `write_input` call path with right-arrow escape sequences and reports min/p50/p95/max microsecond latency. It intentionally does not wait for shell echo, so it isolates the path behind typing and completion-accept writes.

Key-to-screen benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-key-to-screen 50 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The key-to-screen benchmark writes unique command markers into the PTY and reports min/p50/p95/max microsecond latency until each marker is visible through a next-core screen snapshot. It covers input write, ConPTY echo, parser update, and screen-read visibility, but does not measure final GUI paint latency.

Render cursor-move benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-render-cursor-moves 200 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
cargo run -p unterm-engine --bin unterm-next-core -- --bench-render-application-cursor-moves 200 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The cursor-move benchmarks type a live command-line marker, send alternating left/right arrow inputs through ConPTY using both normal CSI and application-cursor SS3 forms, wait for the screen cursor to move, report completed left/right move counts plus missed moves, then require each render-frame delta to include the cursor row without falling back to a full frame. They cover the core contract behind completion navigation, command-line cursor movement, and tab-switch repaint correctness before a GUI renderer is involved.

Render draw-plan and geometry-plan benchmarks:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-render-plans 1000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
cargo run -p unterm-engine --bin unterm-next-core -- --bench-render-geometry-plans 1000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
cargo run -p unterm-engine --bin unterm-next-core -- --bench-render-submission-plans 1000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
cargo run -p unterm-engine --bin unterm-next-core -- --bench-render-commit-plans 1000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The draw-plan benchmark validates `ScreenEngine::read_render_draw_plan`, which converts a styled render-frame into merged glyph runs, cell style runs, and cursor draw state. The geometry-plan benchmark then measures `RenderDrawPlan::to_geometry_plan`, which maps those terminal-grid runs to pixel rectangles using explicit cell metrics. The submission-plan benchmark measures `RenderGeometryPlan::to_submission_plan`, which turns that geometry into damage rects, background quads, text runs, and a cursor quad. The commit-plan benchmark measures `ScreenEngine::read_render_commit_plan`, which ties that chain to `RenderConsumerState`, giving the future GPU renderer a small CPU-side submit contract before font shaping and actual `wgpu` submission.

Input burst benchmark under output pressure:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-input-burst 1000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The burst benchmark starts two background pseudo-agent sessions that emit output while the interactive session receives repeated right-arrow escape writes. It tracks the p95 latency of the foreground write path under load, which is the closest standalone proxy for completion-accept and cursor-move stalls while Codex/Claude panes are streaming.

Echo latency benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-echo 50 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The benchmark writes unique `echo` markers into the PTY and reports min/p50/p95/max microsecond latency until each marker is visible in the raw next-core output buffer.

Output flood benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-flood-lines 1000 --timeout-ms 10000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The flood benchmark emits many lines through `cmd.exe`, waits for a completion marker, and reports elapsed time plus line/byte throughput.

Scrollback paging benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-scrollback-lines 10000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The scrollback benchmark first fills the terminal through the PTY, then reads the captured history in viewport-sized pages and reports per-page read latency.

Viewport page-cycle benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-viewport-page-cycle-lines 10000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The page-cycle benchmark fills scrollback, simulates PageUp to the top and PageDown back to live-tail through logical viewport jumps, then requires no missed pages or boundary misses. It covers the core contract behind PageUp/PageDown before GUI viewport ownership fully moves into next-core.

Viewport scroll under output flood benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-viewport-scroll-flood 5000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

This benchmark keeps the PTY output stream active while repeatedly scrolling the logical viewport and reading the screen snapshot. It covers the core cost behind PageUp/PageDown smoothness while an agent or shell is still producing output.

Paste benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-paste-kb 10 --timeout-ms 10000 --wait-ms 100 --write "exit`r" -- cmd.exe
cargo run -p unterm-engine --bin unterm-next-core -- --bench-paste-under-flood-kb 10 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The paste benchmarks feed a large single-line payload through the engine paste path and report time until the shell consumes it. The under-flood variant keeps a background pseudo-agent session emitting output while the foreground session waits for the pasted payload, then requires the completion marker to appear without misses. It covers right-click auth-code paste reliability while Codex/Claude panes are streaming.

`next-core` writes paste payloads in UTF-8 safe chunks. When bracketed paste mode is enabled, the start/end markers are kept intact around the chunked body.

Paste telemetry probe:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --json --paste "AUTH-CODE-123456" --wait-ms 100 -- cmd.exe
```

In JSON mode, `activity.input` reports total writes, bytes, and the most recent input write duration. `activity.output` reports PTY output chunks, bytes, and the most recent output update duration. `activity.paste` reports total paste count, text bytes, wire bytes, chunk count, bracketed-paste state, and write duration for the most recent paste.

Dual pseudo-agent benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-dual-agent-lines 5000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The dual-agent benchmark starts two background `cmd.exe` sessions that emit output concurrently, then measures echo latency in the interactive session while those streams are active.

Agent startup stall benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-agent-startup-lines 5000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The startup-stall benchmark starts one pseudo-agent session that immediately emits a large startup burst, then repeatedly sends right-arrow writes and reads the interactive screen until the burst completes. It reports input and screen-read p95 latency, covering the stall class seen when Codex or Claude starts while the user is typing or accepting completion.

Screen text during output flood benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-screen-read-lines 5000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The screen-read benchmark emits output while repeatedly reading the visible screen snapshot, mirroring the core cost behind MCP `screen.text`.

Focus switch lifecycle benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-focus-switches 1000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The focus-switch benchmark keeps one session streaming output while repeatedly focusing across four sessions. Each round verifies exactly one active session, no missing session ids, and no duplicate session ids, covering the lifecycle contract behind tab/taskbar switching and lost-instance regressions.

## Shell Metadata

`next-core` records the launch cwd and updates `SessionEngine::shell().cwd` when the shell emits OSC 7 current-directory sequences such as:

```text
ESC ] 7 ; file://localhost/C:/Users/alex/project BEL
```

Values are decoded from `file://` URIs. On Windows, `/C:/...` paths are normalized to `C:\...`. Shells that do not emit OSC 7 fall back to the foreground/root process cwd when available; `session.idle` / `exec.status` additionally expose the same process-tree cwd summary so callers can distinguish the launch shell from active child CLIs.

`SessionEngine::activity()` is based on `next-core`'s own PTY liveness and recent input/output timestamps. A session is reported as running shortly after input or output and idle after a quiet period. The activity snapshot also includes input, output, paste, screen-read, viewport-scroll, and process-tree counters so agents can diagnose slow typing, completion, auth-code paste, heavy agent output, PageUp/PageDown stalls, active child CLIs such as Codex or Claude, and cwd fallback behavior. Screen-read counters include visible screen reads, range reads, scrollback reads, styled scrollback reads, and search scans. `SessionEngine::shell()` also reports launch env key names and launch-context diagnostics for profile/proxy handoff checks without leaking profile secrets. `HealthEngine::health()` exposes the same class of counters as an aggregate `io` summary across live `next-core` sessions, plus runtime queue pending/rejected counts and runtime pump lane/latency telemetry for server-level diagnostics.
