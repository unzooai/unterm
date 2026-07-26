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
