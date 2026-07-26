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

The default mode verifies `docs\next-core-benchmark-summary.json` with `unterm-engine\verify-next-core-benchmark.ps1`. The verifier checks that every required benchmark/gate is present and passing, and that the summary commit is reachable from the current Git history. Use `-SkipCommitReachabilityCheck` only for source snapshots that do not include `.git`.

The full mode runs `unterm-engine\bench-next-core.ps1`, writes the Markdown/JSON benchmark artifacts, then verifies the JSON summary.

GitHub Actions workflow changes require a token with `workflow` scope. If a local agent cannot push `.github\workflows\*.yml`, keep this script and documentation committed, then add the workflow step from an account or token with that scope.
