param(
    [int]$MaxCoreSourceLines = 12000,
    [int]$MaxProbeSourceLines = 2500,
    [int]$MaxDirectDependencies = 10,
    # A debug binary carries its debug info, so this tracks the toolchain and
    # the C libraries far more than it tracks next-core. The real size control
    # is MaxCoreSourceLines; this one only catches a sudden jump.
    [int]$MaxDebugBinaryBytes = 8000000,
    [switch]$SkipBinarySizeCheck
)

$ErrorActionPreference = "Stop"

$EngineDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $EngineDir "..")
$SourceRoot = Join-Path $EngineDir "src"
$CoreRoot = Join-Path $SourceRoot "next_core"
$MainCoreFile = Join-Path $SourceRoot "next_core.rs"
$ProbeFile = Join-Path $SourceRoot "bin\unterm-next-core.rs"
$DebugBinaryName = if ($IsWindows) { "unterm-next-core.exe" } else { "unterm-next-core" }
$DebugBinary = Join-Path $RepoRoot "target\debug\$DebugBinaryName"

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

function Count-ProductionRustLines {
    param([string[]]$Paths)

    $total = 0
    foreach ($path in $Paths) {
        if (-not (Test-Path $path)) {
            throw "missing source file: $path"
        }

        $fileName = [System.IO.Path]::GetFileName($path)
        if ($fileName -eq "test_support.rs" -or $fileName -eq "test_facade.rs") {
            continue
        }

        $skipNextTestModule = $false
        $inTestModule = $false
        $braceDepth = 0
        foreach ($line in Get-Content -Path $path) {
            if (-not $inTestModule -and $line -match '^\s*#\[cfg\(test\)\]\s*$') {
                $skipNextTestModule = $true
                continue
            }

            if ($skipNextTestModule) {
                if ($line -match '^\s*mod\s+tests\s*\{') {
                    $inTestModule = $true
                    $braceDepth = ([regex]::Matches($line, '\{').Count - [regex]::Matches($line, '\}').Count)
                    if ($braceDepth -le 0) {
                        $inTestModule = $false
                    }
                    $skipNextTestModule = $false
                    continue
                }

                $skipNextTestModule = $false
            }

            if ($inTestModule) {
                $braceDepth += ([regex]::Matches($line, '\{').Count - [regex]::Matches($line, '\}').Count)
                if ($braceDepth -le 0) {
                    $inTestModule = $false
                }
                continue
            }

            $total += 1
        }
    }
    return $total
}

$coreFiles = @($MainCoreFile)
if (Test-Path $CoreRoot) {
    $coreFiles += @(Get-ChildItem -Path $CoreRoot -Recurse -File -Filter "*.rs" | ForEach-Object { $_.FullName })
}
$coreSourceLines = Count-ProductionRustLines $coreFiles
$probeSourceLines = Count-Lines @($ProbeFile)

$treeLines = @(& cmd /c "cargo tree -p unterm-engine --depth 1 --prefix depth 2>&1" | ForEach-Object { $_.ToString() })
if ($LASTEXITCODE -ne 0) {
    throw "cargo tree failed:`n$($treeLines -join "`n")"
}
$directDependencies = @($treeLines | Where-Object { $_ -match "^1\S" }).Count

$debugBinaryBytes = $null
if (-not $SkipBinarySizeCheck) {
    # Always rebuild. Measuring whatever binary happened to be lying around let
    # this check pass for a long time against an artifact that no longer
    # matched the source.
    & cargo build -p unterm-engine --bin unterm-next-core
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed for unterm-engine --bin unterm-next-core"
    }
    if (-not (Test-Path $DebugBinary)) {
        throw "missing debug binary after build: $DebugBinary"
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
