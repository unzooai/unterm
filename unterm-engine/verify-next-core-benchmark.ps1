param(
    [string]$SummaryJsonPath = "",
    [int]$ExpectedGateCount = 10,
    [int]$ExpectedBenchmarkCount = 11
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
if ($summary.json_smoke.Engine -ne "next-core") {
    throw "json smoke engine is not next-core: $($summary.json_smoke.Engine)"
}

$gates = @($summary.gates)
if ($gates.Count -ne $ExpectedGateCount) {
    throw "expected $ExpectedGateCount gates, got $($gates.Count)"
}

$requiredGates = @(
    "input write p95",
    "echo p95",
    "dual-agent echo p95",
    "paste 10kb elapsed",
    "scrollback page p95",
    "viewport scroll p95",
    "screen read under flood p95",
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
    "echo latency",
    "output flood",
    "paste 10kb",
    "scrollback paging",
    "viewport scroll paging",
    "dual pseudo-agent output",
    "screen read during flood",
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
}

Write-Host "next-core benchmark summary ok: commit=$($summary.commit) gates=$($gates.Count) benchmarks=$($benchmarks.Count)"
