param(
    [switch]$RunBenchmark,
    [switch]$SkipSizeBudget,
    [switch]$SkipGuiRender,
    [switch]$SkipWebGpuRender,
    [switch]$SkipMcp,
    [string]$SummaryJsonPath = "",
    [string]$ReportPath = ""
)

$ErrorActionPreference = "Stop"

$CiDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $CiDir "..")
$EngineDir = Join-Path $RepoRoot "unterm-engine"

if ([string]::IsNullOrWhiteSpace($SummaryJsonPath)) {
    $SummaryJsonPath = Join-Path $RepoRoot "docs\next-core-benchmark-summary.json"
}
if ([string]::IsNullOrWhiteSpace($ReportPath)) {
    $ReportPath = Join-Path $RepoRoot "docs\next-core-benchmark-report.md"
}

if ($RunBenchmark) {
    & (Join-Path $EngineDir "bench-next-core.ps1") `
        -OutputPath $ReportPath `
        -SummaryJsonPath $SummaryJsonPath
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

& (Join-Path $EngineDir "verify-next-core-benchmark.ps1") `
    -SummaryJsonPath $SummaryJsonPath
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if (-not $SkipSizeBudget) {
    & (Join-Path $EngineDir "verify-next-core-size-budget.ps1")
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

if (-not $SkipGuiRender) {
    & (Join-Path $CiDir "next-core-gui-render.ps1")
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

if (-not $SkipWebGpuRender) {
    & (Join-Path $CiDir "next-core-webgpu-render.ps1")
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

if (-not $SkipMcp) {
    & (Join-Path $CiDir "next-core-mcp.ps1")
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
