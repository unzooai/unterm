# unterm-engine

Engine-neutral terminal traits plus the experimental `next-core` implementation.

This crate is the boundary for moving Unterm product behavior away from WezTerm GUI internals. The GUI still uses the WezTerm adapter, but next-core can now be built, tested, and probed without starting the GUI.

## Probe

Build and run the standalone next-core probe:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --wait-ms 500 -- cmd.exe /c "echo next-core-probe"
```

The probe creates a next-core session, optionally writes text or runs a command, waits, then prints the visible screen snapshot.

On Windows, the command above should print `next-core-probe`. A blank snapshot means the standalone ConPTY smoke path has regressed before the GUI is involved.

## Benchmark Report

Run the Phase 2 next-core benchmark suite and generate `docs/next-core-benchmark-report.md`:

```powershell
.\unterm-engine\bench-next-core.ps1
```

The runner builds `unterm-next-core`, executes the current latency/output/scrollback/paste/dual-agent/screen-read benchmarks, and writes both summary lines and raw output into the report.

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

Paste benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-paste-kb 10 --timeout-ms 10000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The paste benchmark feeds a large single-line payload through the engine paste path and reports time until the shell consumes it.

Dual pseudo-agent benchmark:

```powershell
cargo run -p unterm-engine --bin unterm-next-core -- --bench-dual-agent-lines 5000 --timeout-ms 30000 --wait-ms 100 --write "exit`r" -- cmd.exe
```

The dual-agent benchmark starts two background `cmd.exe` sessions that emit output concurrently, then measures echo latency in the interactive session while those streams are active.

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

Values are decoded from `file://` URIs. On Windows, `/C:/...` paths are normalized to `C:\...`. Shells that do not emit OSC 7 still report the launch cwd until a process-tree fallback is added.

`SessionEngine::activity()` is based on `next-core`'s own PTY liveness and recent input/output timestamps. A session is reported as running shortly after input or output and idle after a quiet period. The foreground process name is still the launch shell until child-process tracking is added.
