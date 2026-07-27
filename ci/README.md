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

The default mode verifies `docs\next-core-benchmark-summary.json` with `unterm-engine\verify-next-core-benchmark.ps1`, runs the next-core size budget verifier, runs `ci\next-core-gui-render.ps1` to cover the GUI render replacement contract, and runs `ci\next-core-mcp.ps1` to cover the MCP/product boundary contract for the selected next-core engine. The benchmark verifier checks that every required benchmark/gate is present and passing, and that the summary commit is reachable from the current Git history. Use `-SkipGuiRender` only for jobs that cannot build the GUI test binary, and `-SkipMcp` only for jobs that intentionally skip the GUI binary's MCP handler tests.

The full mode runs `unterm-engine\bench-next-core.ps1`, writes the Markdown/JSON benchmark artifacts, then verifies the JSON summary.

GitHub Actions workflow changes require a token with `workflow` scope. If a local agent cannot push `.github\workflows\*.yml`, keep this script and documentation committed, then add the workflow step from an account or token with that scope.
