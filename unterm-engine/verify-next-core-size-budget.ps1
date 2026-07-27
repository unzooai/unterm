param(
    [int]$MaxCoreSourceLines = 12000,
    [int]$MaxProbeSourceLines = 2500,
    [int]$MaxDirectDependencies = 10,
    [int]$MaxDebugBinaryBytes = 4000000,
    [switch]$SkipBinarySizeCheck
)

$ErrorActionPreference = "Stop"

$EngineDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $EngineDir "..")
$SourceRoot = Join-Path $EngineDir "src"
$CoreRoot = Join-Path $SourceRoot "next_core"
$MainCoreFile = Join-Path $SourceRoot "next_core.rs"
$ProbeFile = Join-Path $SourceRoot "bin\unterm-next-core.rs"
$DebugBinary = Join-Path $RepoRoot "target\debug\unterm-next-core.exe"

function Count-Lines {
    param([string[]]$Paths)

    $total = 0
    foreach ($path in $Paths) {
        if (-not (Test-Path $path)) {
            throw "missing source file: $path"
        }
        $total += @((Get-Content -Path $path)).Count
    }
    return $total
}

$coreFiles = @($MainCoreFile)
if (Test-Path $CoreRoot) {
    $coreFiles += @(Get-ChildItem -Path $CoreRoot -Recurse -File -Filter "*.rs" | ForEach-Object { $_.FullName })
}
$coreSourceLines = Count-Lines $coreFiles
$probeSourceLines = Count-Lines @($ProbeFile)

$treeLines = @(& cargo tree -p unterm-engine --depth 1 --prefix depth 2>&1 | ForEach-Object { $_.ToString() })
if ($LASTEXITCODE -ne 0) {
    throw "cargo tree failed:`n$($treeLines -join "`n")"
}
$directDependencies = @($treeLines | Where-Object { $_ -match "^1\S" }).Count

$debugBinaryBytes = $null
if (-not $SkipBinarySizeCheck) {
    if (-not (Test-Path $DebugBinary)) {
        throw "missing debug binary: $DebugBinary; run cargo build -p unterm-engine --bin unterm-next-core or pass -SkipBinarySizeCheck"
    }
    $debugBinaryBytes = (Get-Item $DebugBinary).Length
}

$failures = New-Object System.Collections.Generic.List[string]
if ($coreSourceLines -gt $MaxCoreSourceLines) {
    $failures.Add("core_source_lines=$coreSourceLines exceeds max=$MaxCoreSourceLines")
}
if ($probeSourceLines -gt $MaxProbeSourceLines) {
    $failures.Add("probe_source_lines=$probeSourceLines exceeds max=$MaxProbeSourceLines")
}
if ($directDependencies -gt $MaxDirectDependencies) {
    $failures.Add("direct_dependencies=$directDependencies exceeds max=$MaxDirectDependencies")
}
if ((-not $SkipBinarySizeCheck) -and $debugBinaryBytes -gt $MaxDebugBinaryBytes) {
    $failures.Add("debug_binary_bytes=$debugBinaryBytes exceeds max=$MaxDebugBinaryBytes")
}

if ($failures.Count -gt 0) {
    throw "next-core size budget failed: $($failures -join '; ')"
}

$binarySummary = if ($SkipBinarySizeCheck) { "skipped" } else { $debugBinaryBytes }
Write-Host "next-core size budget ok: core_source_lines=$coreSourceLines/$MaxCoreSourceLines probe_source_lines=$probeSourceLines/$MaxProbeSourceLines direct_dependencies=$directDependencies/$MaxDirectDependencies debug_binary_bytes=$binarySummary/$MaxDebugBinaryBytes"
