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
