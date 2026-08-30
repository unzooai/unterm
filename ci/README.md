# CI Helpers

## Next-Core Benchmark

Use the lightweight verifier in pull-request CI when the job should not spend time rerunning the full benchmark suite:

```powershell
pwsh -File ci\next-core-benchmark.ps1
```

Use the full mode for scheduled/manual benchmark jobs that are allowed to refresh artifacts:

```powershell
pwsh -File ci\next-core-benchmark.ps1 -RunBenchmark
```

The default mode verifies `docs\next-core-benchmark-summary.json` with `unterm-engine\verify-next-core-benchmark.ps1`, runs the next-core size budget verifier, runs `ci\next-core-runtime.ps1` to cover input-first scheduling, bounded queue backpressure, lifecycle/focus priority, viewport scroll priority, response pump telemetry, and screen/render read dispatch contracts, runs `ci\next-core-unicode.ps1` to cover Unicode width and emoji/ZWJ screen-model contracts, runs `ci\next-core-gui-render.ps1` to cover the GUI facade render replacement contract, runs `ci\next-core-webgpu-render.ps1` to cover WebGPU pane replacement, cached glyph upload, glyph atlas, and render-backend contracts, and runs `ci\next-core-mcp.ps1` to cover the MCP/product boundary contract for the selected next-core engine. The benchmark verifier checks that every required benchmark/gate is present and passing, and that the summary commit is reachable from the current Git history. Use `-SkipRuntime` only for jobs that intentionally skip unterm-engine runtime scheduler tests, `-SkipUnicode` only for jobs that intentionally skip unterm-engine Unicode screen-model tests, `-SkipGuiRender` only for jobs that cannot build the GUI test binary, `-SkipWebGpuRender` only for jobs that intentionally skip WebGPU renderer-side unit contracts, and `-SkipMcp` only for jobs that intentionally skip the GUI binary's MCP handler tests.

The full mode runs `unterm-engine\bench-next-core.ps1`, writes the Markdown/JSON benchmark artifacts, then verifies the JSON summary.

GitHub Actions workflow changes require a token with `workflow` scope. If a local agent cannot push `.github\workflows\*.yml`, keep this script and documentation committed, then add the workflow step from an account or token with that scope.

## Installers

Two products ship out of this repo's build scripts, and they stay independent:

```powershell
# Just the terminal. Same MSI as always; the console rides along when its
# static build is present, and the build succeeds without it either way.
pwsh -File ci\build-msi.ps1

# Unzoo One: one download that installs the terminal and the browser.
gh release download -R unzooai/unzoo -p "UnzooSetup-*.exe" --dir dist
pwsh -File ci\build-bundle.ps1 -UnzooSetup dist\UnzooSetup-2.5.32.exe
```

The bundle is additive. It carries `Unterm.msi` byte-for-byte and hands Unzoo's
own installer its silent switch; neither product is repackaged or renamed, and
the bundle's `UpgradeCode` is its own. Installing through the bundle and
installing the two pieces by hand leave the machine in the same state, and each
product keeps updating on its own schedule.

`build-bundle.ps1` builds an MSI first unless you point `-UntermMsi` at one.
The browser is chained as non-vital: if its installer fails, the terminal is
still installed and usable, just without the browser-driven capabilities. A
machine that already has the same or a newer Unzoo Browser skips that 183 MB
step entirely.

### The console

The Unzoo One console is built in the `unzoo-one` repo:

```bash
cd ..\unzoo-one && npm run build:static   # -> dist\client
```

`build-msi.ps1 -ConsoleDir` defaults to `..\unzoo-one\dist\client`. When it
finds a build there it stages it into `<install dir>\console`, which
`unterm-settings` serves at `/console/`. No build, no console: the MSI is a
couple of MB smaller and the menu entry stays hidden.
