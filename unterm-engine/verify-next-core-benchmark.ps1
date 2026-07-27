param(
    [string]$SummaryJsonPath = "",
    [int]$ExpectedGateCount = 16,
    [int]$ExpectedBenchmarkCount = 16,
    [switch]$SkipCommitReachabilityCheck
)

$ErrorActionPreference = "Stop"

$EngineDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $EngineDir "..")
if ([string]::IsNullOrWhiteSpace($SummaryJsonPath)) {
    $SummaryJsonPath = Join-Path $RepoRoot "docs\next-core-benchmark-summary.json"
}

if (-not (Test-Path $SummaryJsonPath)) {
    throw "missing next-core benchmark summary: $SummaryJsonPath"
}

$summary = Get-Content -Raw -Path $SummaryJsonPath | ConvertFrom-Json
if ($summary.ok -ne $true) {
    throw "benchmark summary ok=false"
}
if ([string]::IsNullOrWhiteSpace($summary.commit)) {
    throw "benchmark summary is missing commit"
}
if (-not $SkipCommitReachabilityCheck) {
    $isWorkTree = & git -C $RepoRoot rev-parse --is-inside-work-tree 2>$null
    if ($LASTEXITCODE -ne 0 -or $isWorkTree -ne "true") {
        throw "cannot verify benchmark summary commit without a git worktree; pass -SkipCommitReachabilityCheck to skip"
    }

    & git -C $RepoRoot merge-base --is-ancestor $summary.commit HEAD 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "benchmark summary commit is not reachable from HEAD: $($summary.commit)"
    }
}
if ($summary.json_smoke.Engine -ne "next-core") {
    throw "json smoke engine is not next-core: $($summary.json_smoke.Engine)"
}
if ($summary.json_smoke.ScreenReads -lt 1) {
    throw "json smoke did not include screen read diagnostics"
}
if ($summary.json_smoke.RenderFrameRevision -lt 1) {
    throw "json smoke did not include render frame revision"
}
if ($summary.json_smoke.RenderFrameLines -lt 1) {
    throw "json smoke did not include render frame lines"
}
if ($summary.json_smoke.RenderDeltaLines -ne 0) {
    throw "json smoke unchanged render delta was not empty"
}
if (-not ($summary.json_smoke.PSObject.Properties.Name -contains "DeadReason")) {
    throw "json smoke did not include session dead_reason field"
}
if ([string]::IsNullOrWhiteSpace($summary.json_smoke.ForegroundProcess)) {
    throw "json smoke did not include foreground process diagnostics"
}
if ([string]::IsNullOrWhiteSpace($summary.json_smoke.Cwd)) {
    throw "json smoke did not include session cwd fallback diagnostics"
}
if ($summary.json_smoke.Profile -ne "bench-profile") {
    throw "json smoke did not include launch profile diagnostics"
}
if (-not @($summary.json_smoke.ProxyEnvKeys | Where-Object { $_ -eq "HTTPS_PROXY" })) {
    throw "json smoke did not include launch proxy diagnostics"
}
if ($summary.json_smoke.LifecycleCreated -lt 1) {
    throw "json smoke did not include lifecycle diagnostics"
}

$gates = @($summary.gates)
if ($gates.Count -ne $ExpectedGateCount) {
    throw "expected $ExpectedGateCount gates, got $($gates.Count)"
}

$requiredGates = @(
    "input write p95",
    "key-to-screen p95",
    "input burst p95",
    "echo p95",
    "dual-agent echo p95",
    "agent startup input p95",
    "paste 10kb elapsed",
    "scrollback page p95",
    "viewport scroll p95",
    "viewport scroll under flood p95",
    "screen read under flood p95",
    "render frame p95",
    "render dirty frame p95",
    "focus switch p95",
    "session create p95",
    "session ready p95"
)
foreach ($name in $requiredGates) {
    $gate = @($gates | Where-Object { $_.name -eq $name })
    if ($gate.Count -ne 1) {
        throw "missing or duplicate gate: $name"
    }
    if ($gate[0].ok -ne $true) {
        throw "gate failed: $name actual=$($gate[0].actual) max=$($gate[0].max) $($gate[0].unit)"
    }
    if ($null -eq $gate[0].actual) {
        throw "gate is missing actual value: $name"
    }
}

$benchmarks = @($summary.benchmarks)
if ($benchmarks.Count -ne $ExpectedBenchmarkCount) {
    throw "expected $ExpectedBenchmarkCount benchmarks, got $($benchmarks.Count)"
}

$requiredBenchmarks = @(
    "input write latency",
    "key-to-screen latency",
    "input burst under output",
    "echo latency",
    "output flood",
    "paste 10kb",
    "scrollback paging",
    "viewport scroll paging",
    "viewport scroll during flood",
    "dual pseudo-agent output",
    "agent startup stall",
    "screen read during flood",
    "render frame latency",
    "focus switch latency",
    "session create latency",
    "session ready latency"
)
foreach ($name in $requiredBenchmarks) {
    $benchmark = @($benchmarks | Where-Object { $_.name -eq $name })
    if ($benchmark.Count -ne 1) {
        throw "missing or duplicate benchmark: $name"
    }
    if ($benchmark[0].exit_code -ne 0) {
        throw "benchmark failed: $name exit_code=$($benchmark[0].exit_code)"
    }
    if (@($benchmark[0].summary).Count -eq 0) {
        throw "benchmark has no summary lines: $name"
    }
    if (-not @($benchmark[0].summary | Where-Object { $_ -like "health_io*" })) {
        throw "benchmark is missing health_io summary: $name"
    }
    if (-not @($benchmark[0].summary | Where-Object { $_ -like "health_lifecycle*" })) {
        throw "benchmark is missing health_lifecycle summary: $name"
    }
    if (-not @($benchmark[0].summary | Where-Object { $_ -like "activity_process*" })) {
        throw "benchmark is missing activity_process summary: $name"
    }
}

$commitReachability = if ($SkipCommitReachabilityCheck) { "skipped" } else { "true" }
Write-Host "next-core benchmark summary ok: commit=$($summary.commit) commit_reachable=$commitReachability gates=$($gates.Count) benchmarks=$($benchmarks.Count)"
