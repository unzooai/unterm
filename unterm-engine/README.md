# unterm-engine

Engine-neutral terminal traits plus the experimental `next-core` implementation.

This crate is the boundary for moving Unterm product behavior away from WezTerm GUI internals. The GUI still uses the WezTerm adapter, but next-core can now be built, tested, and probed without starting the GUI.

## Probe

Build and run the standalone next-core probe:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --wait-ms 500 -- cmd.exe /c "echo next-core-probe"
```

The probe creates a next-core session, optionally writes text or runs a command, waits, then prints the visible screen snapshot plus compact `health_io` and `health_lifecycle` summary lines.

On Windows, the command above should print `next-core-probe`. A blank snapshot means the standalone ConPTY smoke path has regressed before the GUI is involved. The in-memory screen model is cols-aware: printable text wraps at the configured width, DECAWM `CSI ? 7 h/l` controls automatic wrapping at the right edge, DECOM `CSI ? 6 h/l` makes cursor positioning relative to scroll regions, HT moves the cursor to the next tab stop without forcing a line wrap, HTS/TBC set and clear custom tab stops, CBT reverse-tabs to prior tab stops, ESC IND/NEL/RI and IL/DL line mutation honor scroll regions, CSI CNL/CPL/HPA/HPR/VPA/VPR plus CSI/DEC private save/restore cursor, erase-character, REP repeat-character, `CSI 3 J` scrollback-clear, and semicolon/colon SGR extended-color sequences are handled, wide cells wrap before the edge, and resize clamps existing rows to the new column count.

Machine-readable probe output:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --json --wait-ms 500 -- cmd.exe /c "echo next-core-probe"
```

The JSON output includes the session snapshot, screen snapshot, activity snapshot, engine health, raw output byte count, and visible text. Use this for CI or agent dogfood checks instead of parsing the human-readable snapshot. In `next-core`, session snapshots include `dead_reason` for exited/closed PTY paths, `shell.launch_context` summarizes the selected profile id plus proxy env key names without exposing env values, `health.lifecycle` summarizes session create/destroy/dead counters and the latest death reason, and `health.io` summarizes input writes/bytes, output chunks/bytes, paste counts/text bytes, screen reads, and viewport scrolls across live sessions.

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

The runner builds `unterm-next-core`, verifies the machine-readable `--json` probe output, executes the current input-write/input-burst/echo/output/scrollback/viewport-scroll/viewport-scroll-under-flood/paste/dual-agent/agent-startup-stall/screen-read/focus-switch/session-create/session-ready benchmarks, writes human-readable summary and raw output into the Markdown report, and writes machine-readable gate results into the JSON summary. Any failed benchmark or gate exits non-zero.

Verify an existing JSON summary without rerunning the benchmark suite:

```powershell
.\unterm-engine\verify-next-core-benchmark.ps1
```

The verifier checks the required gate and benchmark names, requires every gate to pass, and exits non-zero if the summary is missing or stale in shape.

CI can use the lightweight wrapper to verify the committed summary:

```powershell
.\ci\next-core-benchmark.ps1
```

Use `-RunBenchmark` when the job should refresh the Markdown/JSON artifacts before verifying them.

## Experimental MCP Engine Selector

The GUI defaults to the current WezTerm-backed engine. For MCP/product-service experiments, start Unterm with:

```powershell
$env:UNTERM_ENGINE = "next-core"
cargo run -p unterm -- start
```

This routes the engine-neutral session/screen/input/paste calls through next-core where supported. It does not make next-core the default GUI renderer yet.

Interactive input smoke test:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --wait-ms 1200 --write "echo next-core-write`rexit`r" -- cmd.exe
```

Input write benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-input-writes 1000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The input write benchmark measures the engine `write_input` call path with right-arrow escape sequences and reports min/p50/p95/max microsecond latency. It intentionally does not wait for shell echo, so it isolates the path behind typing and completion-accept writes.

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

Viewport scroll under output flood benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-viewport-scroll-flood 5000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

This benchmark keeps the PTY output stream active while repeatedly scrolling the logical viewport and reading the screen snapshot. It covers the core cost behind PageUp/PageDown smoothness while an agent or shell is still producing output.

Paste benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-paste-kb 10 --timeout-ms 10000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The paste benchmark feeds a large single-line payload through the engine paste path and reports time until the shell consumes it.

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

## Shell Metadata

`next-core` records the launch cwd and updates `SessionEngine::shell().cwd` when the shell emits OSC 7 current-directory sequences such as:

```text
ESC ] 7 ; file://localhost/C:/Users/alex/project BEL
```

Values are decoded from `file://` URIs. On Windows, `/C:/...` paths are normalized to `C:\...`. Shells that do not emit OSC 7 fall back to the foreground/root process cwd when available; `session.idle` / `exec.status` additionally expose the same process-tree cwd summary so callers can distinguish the launch shell from active child CLIs.

`SessionEngine::activity()` is based on `next-core`'s own PTY liveness and recent input/output timestamps. A session is reported as running shortly after input or output and idle after a quiet period. The activity snapshot also includes input, output, paste, screen-read, viewport-scroll, and process-tree counters so agents can diagnose slow typing, completion, auth-code paste, heavy agent output, PageUp/PageDown stalls, active child CLIs such as Codex or Claude, and cwd fallback behavior. `SessionEngine::shell()` also reports launch env key names and launch-context diagnostics for profile/proxy handoff checks without leaking profile secrets. `HealthEngine::health()` exposes the same class of counters as an aggregate `io` summary across live `next-core` sessions for server-level diagnostics.
