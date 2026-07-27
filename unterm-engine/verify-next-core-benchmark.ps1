param(
    [string]$SummaryJsonPath = "",
    [int]$ExpectedGateCount = 34,
    [int]$ExpectedBenchmarkCount = 24,
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
if ([string]::IsNullOrWhiteSpace($summary.json_smoke.Screen) -or $summary.json_smoke.Screen -notmatch "^(\d+)x(\d+)$") {
    throw "json smoke did not include parseable screen dimensions: $($summary.json_smoke.Screen)"
}
$jsonSmokeCols = [int]$Matches[1]
$jsonSmokeRows = [int]$Matches[2]
if ($summary.json_smoke.RenderFrameLines -ne $jsonSmokeRows) {
    throw "json smoke render frame lines $($summary.json_smoke.RenderFrameLines) did not match screen rows $jsonSmokeRows"
}
if ($summary.json_smoke.RenderFrameCols -ne $jsonSmokeCols) {
    throw "json smoke render frame cols $($summary.json_smoke.RenderFrameCols) did not match screen cols $jsonSmokeCols"
}
$renderFrameCellCounts = @($summary.json_smoke.RenderFrameCellCounts)
if ($renderFrameCellCounts.Count -ne $jsonSmokeRows) {
    throw "json smoke render frame cell-count rows $($renderFrameCellCounts.Count) did not match screen rows $jsonSmokeRows"
}
$badRenderFrameCellCounts = @($renderFrameCellCounts | Where-Object { $_ -ne $jsonSmokeCols })
if ($badRenderFrameCellCounts.Count -ne 0) {
    throw "json smoke render frame cell counts did not all match screen cols $jsonSmokeCols`: $($renderFrameCellCounts -join ',')"
}
if ($summary.json_smoke.RenderFrameGridCells -ne ($jsonSmokeRows * $jsonSmokeCols)) {
    throw "json smoke render frame grid cells $($summary.json_smoke.RenderFrameGridCells) did not match rows*cols $($jsonSmokeRows * $jsonSmokeCols)"
}
if ($summary.json_smoke.RenderDeltaLines -ne 0) {
    throw "json smoke unchanged render delta was not empty"
}
if ($summary.json_smoke.RenderDrawPlanRevision -ne $summary.json_smoke.RenderFrameRevision) {
    throw "json smoke render draw plan revision $($summary.json_smoke.RenderDrawPlanRevision) did not match render frame revision $($summary.json_smoke.RenderFrameRevision)"
}
if ($summary.json_smoke.RenderDrawPlanGlyphRuns -lt 1) {
    throw "json smoke did not include render draw plan glyph runs"
}
if ($summary.json_smoke.RenderDrawPlanCellRuns -lt $jsonSmokeRows) {
    throw "json smoke render draw plan cell runs $($summary.json_smoke.RenderDrawPlanCellRuns) were fewer than screen rows $jsonSmokeRows"
}
if ($summary.json_smoke.RenderDrawPlanCursor -ne $true) {
    throw "json smoke did not include render draw plan cursor state"
}
if ($summary.json_smoke.RenderDrawDeltaGlyphRuns -ne 0) {
    throw "json smoke unchanged render draw delta glyph runs were not empty"
}
if ($summary.json_smoke.RenderDrawDeltaCellRuns -ne 0) {
    throw "json smoke unchanged render draw delta cell runs were not empty"
}
if ($summary.json_smoke.RenderDrawDeltaCursor -ne $true) {
    throw "json smoke unchanged render draw delta did not include cursor state"
}
if ($summary.json_smoke.RenderGeometryViewportWidth -ne ($jsonSmokeCols * 8)) {
    throw "json smoke render geometry viewport width $($summary.json_smoke.RenderGeometryViewportWidth) did not match cols*8 $($jsonSmokeCols * 8)"
}
if ($summary.json_smoke.RenderGeometryViewportHeight -ne ($jsonSmokeRows * 16)) {
    throw "json smoke render geometry viewport height $($summary.json_smoke.RenderGeometryViewportHeight) did not match rows*16 $($jsonSmokeRows * 16)"
}
if ($summary.json_smoke.RenderGeometryGlyphRuns -lt 1) {
    throw "json smoke did not include render geometry glyph runs"
}
if ($summary.json_smoke.RenderGeometryCellRuns -lt $jsonSmokeRows) {
    throw "json smoke render geometry cell runs $($summary.json_smoke.RenderGeometryCellRuns) were fewer than screen rows $jsonSmokeRows"
}
if ($summary.json_smoke.RenderGeometryCursor -ne $true) {
    throw "json smoke did not include render geometry cursor"
}
if ($summary.json_smoke.RenderSubmissionDamageRects -lt 1) {
    throw "json smoke did not include render submission damage rects"
}
if ($summary.json_smoke.RenderSubmissionTextRuns -lt 1) {
    throw "json smoke did not include render submission text runs"
}
if ($summary.json_smoke.RenderSubmissionBackgroundQuads -lt $jsonSmokeRows) {
    throw "json smoke render submission background quads $($summary.json_smoke.RenderSubmissionBackgroundQuads) were fewer than screen rows $jsonSmokeRows"
}
if ($summary.json_smoke.RenderSubmissionCursor -ne $true) {
    throw "json smoke did not include render submission cursor"
}
if ($summary.json_smoke.RenderCommitSubmit -ne $true) {
    throw "json smoke did not include first-frame render commit submission"
}
if ($summary.json_smoke.RenderCommitFullRepaint -ne $true) {
    throw "json smoke did not include first-frame render commit full repaint"
}
if ($summary.json_smoke.RenderCommitDamageRects -lt 1) {
    throw "json smoke did not include render commit damage rects"
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
if (-not ($summary.json_smoke.PSObject.Properties.Name -contains "RuntimePumpDrainCalls")) {
    throw "json smoke did not include runtime pump drain-call diagnostics"
}
if ($summary.json_smoke.RuntimePumpDrainCalls -lt 1) {
    throw "json smoke runtime pump did not record drain calls"
}
if ($summary.json_smoke.RuntimePumpDispatched -lt 1) {
    throw "json smoke runtime pump did not record dispatches"
}
if ($summary.json_smoke.RuntimePumpRender -lt 1 -or $summary.json_smoke.RuntimePumpScreen -lt 1) {
    throw "json smoke runtime pump did not record render/screen lane dispatches"
}
if (-not ($summary.json_smoke.PSObject.Properties.Name -contains "RuntimePumpWaitedForResponse")) {
    throw "json smoke did not include runtime pump response-wait diagnostics"
}
if (-not ($summary.json_smoke.PSObject.Properties.Name -contains "RuntimePumpCompletedWithoutWait")) {
    throw "json smoke did not include runtime pump immediate-completion diagnostics"
}
if (($summary.json_smoke.RuntimePumpWaitedForResponse + $summary.json_smoke.RuntimePumpCompletedWithoutWait) -ne $summary.json_smoke.RuntimePumpDrainCalls) {
    throw "json smoke runtime pump wait/immediate counters did not add up to drain calls"
}
if ($summary.json_smoke.RuntimePumpMaxDispatchUs -lt 0 -or $summary.json_smoke.RuntimePumpMaxDrainUs -lt 0) {
    throw "json smoke runtime pump latency fields were invalid"
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
    "paste under flood elapsed",
    "paste under flood marker misses",
    "scrollback page p95",
    "viewport scroll p95",
    "viewport page cycle p95",
    "viewport page cycle boundary misses",
    "viewport page cycle missed pages",
    "viewport scroll under flood p95",
    "screen read under flood p95",
    "render frame p95",
    "render draw plan p95",
    "render geometry plan p95",
    "render submission plan p95",
    "render commit plan p95",
    "render dirty frame p95",
    "render cursor move p95",
    "render cursor move full frames",
    "render cursor move missed moves",
    "render application cursor move p95",
    "render application cursor move full frames",
    "render application cursor move missed moves",
    "focus switch p95",
    "focus switch active misses",
    "focus switch missing sessions",
    "focus switch duplicate sessions",
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
    "paste under output flood",
    "scrollback paging",
    "viewport scroll paging",
    "viewport page cycle",
    "viewport scroll during flood",
    "dual pseudo-agent output",
    "agent startup stall",
    "screen read during flood",
    "render frame latency",
    "render draw plan latency",
    "render geometry plan latency",
    "render submission plan latency",
    "render commit plan latency",
    "render cursor move latency",
    "render application cursor move latency",
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
    if (-not @($benchmark[0].summary | Where-Object { $_ -like "health_runtime_pump*" })) {
        throw "benchmark is missing health_runtime_pump summary: $name"
    }
    if (-not @($benchmark[0].summary | Where-Object { $_ -like "activity_process*" })) {
        throw "benchmark is missing activity_process summary: $name"
    }
}

$commitReachability = if ($SkipCommitReachabilityCheck) { "skipped" } else { "true" }
Write-Host "next-core benchmark summary ok: commit=$($summary.commit) commit_reachable=$commitReachability gates=$($gates.Count) benchmarks=$($benchmarks.Count)"
