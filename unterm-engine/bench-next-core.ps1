param(
    [string]$OutputPath = "",
    [int]$InputWrites = 1000,
    [int]$EchoRounds = 50,
    [int]$FloodLines = 100000,
    [int]$ScrollbackLines = 10000,
    [int]$PasteKb = 10,
    [int]$DualAgentLines = 5000,
    [int]$ScreenReadLines = 5000,
    [int]$TimeoutMs = 120000,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$EngineDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $EngineDir "..")
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $RepoRoot "docs\next-core-benchmark-report.md"
}

function Invoke-Benchmark {
    param(
        [string]$Name,
        [string[]]$BenchArgs
    )

    Write-Host "Running $Name..."
    $output = & $script:ExePath @BenchArgs 2>&1
    $exitCode = $LASTEXITCODE
    $lines = @($output | ForEach-Object { $_.ToString() })
    $summary = @($lines | Where-Object {
            $_ -match "bench_|session id|timed out|Error"
        })

    [pscustomobject]@{
        Name = $Name
        ExitCode = $exitCode
        Args = (($BenchArgs | ForEach-Object {
                    $_.Replace("`r", "\r").Replace("`n", "\n")
                }) -join " ")
        Summary = $summary
        Output = $lines
    }
}

function Invoke-JsonSmoke {
    Write-Host "Running JSON probe smoke..."
    $marker = "next-core-json-smoke"
    $output = & $script:ExePath --json --wait-ms 500 -- cmd.exe /c "echo $marker" 2>&1
    $exitCode = $LASTEXITCODE
    $lines = @($output | ForEach-Object { $_.ToString() })
    if ($exitCode -ne 0) {
        throw "JSON probe failed with exit code $exitCode`: $($lines -join "`n")"
    }

    $text = $lines -join "`n"
    try {
        $json = $text | ConvertFrom-Json
    } catch {
        throw "JSON probe output was not parseable JSON: $text"
    }

    if ($json.health.engine -ne "next-core") {
        throw "JSON probe reported wrong engine: $($json.health.engine)"
    }
    if ($json.visible_text -notlike "*$marker*") {
        throw "JSON probe visible_text did not contain marker '$marker'"
    }
    if ($json.screen.cols -le 0 -or $json.screen.rows -le 0) {
        throw "JSON probe screen dimensions were invalid"
    }
    if ($null -eq $json.activity) {
        throw "JSON probe did not include an activity snapshot"
    }

    [pscustomobject]@{
        Marker = $marker
        Engine = $json.health.engine
        Screen = "$($json.screen.cols)x$($json.screen.rows)"
        RawBytes = $json.raw_bytes
    }
}

Push-Location $RepoRoot
try {
    if (-not $SkipBuild) {
        cargo build -p unterm-engine --bin unterm-next-core
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    }

    $script:ExePath = Join-Path $RepoRoot "target\debug\unterm-next-core.exe"
    if (-not (Test-Path $script:ExePath)) {
        throw "missing next-core probe: $script:ExePath"
    }

    $jsonSmoke = Invoke-JsonSmoke

    $commonTail = @("--timeout-ms", "$TimeoutMs", "--wait-ms", "0", "--write", "exit`r", "--", "cmd.exe")
    $results = @()
    $results += Invoke-Benchmark -Name "input write latency" -BenchArgs ([string[]](@("--bench-input-writes", "$InputWrites") + $commonTail))
    $results += Invoke-Benchmark -Name "echo latency" -BenchArgs ([string[]](@("--bench-echo", "$EchoRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "output flood" -BenchArgs ([string[]](@("--bench-flood-lines", "$FloodLines") + $commonTail))
    $results += Invoke-Benchmark -Name "paste 10kb" -BenchArgs ([string[]](@("--bench-paste-kb", "$PasteKb") + $commonTail))
    $results += Invoke-Benchmark -Name "scrollback paging" -BenchArgs ([string[]](@("--bench-scrollback-lines", "$ScrollbackLines") + $commonTail))
    $results += Invoke-Benchmark -Name "dual pseudo-agent output" -BenchArgs ([string[]](@("--bench-dual-agent-lines", "$DualAgentLines") + $commonTail))
    $results += Invoke-Benchmark -Name "screen read during flood" -BenchArgs ([string[]](@("--bench-screen-read-lines", "$ScreenReadLines") + $commonTail))

    $commit = (& git rev-parse --short HEAD).Trim()
    $date = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss zzz")
    $machine = "$([System.Environment]::MachineName)"
    $os = [System.Environment]::OSVersion.VersionString

    $report = New-Object System.Collections.Generic.List[string]
    $report.Add("# Next-Core Benchmark Report")
    $report.Add("")
    $report.Add("- Generated: $date")
    $report.Add("- Commit: ``$commit``")
    $report.Add("- Machine: ``$machine``")
    $report.Add("- OS: ``$os``")
    $report.Add("- Binary: ``target\debug\unterm-next-core.exe``")
    $report.Add("- JSON smoke: ``$($jsonSmoke.Engine) $($jsonSmoke.Screen) raw_bytes=$($jsonSmoke.RawBytes)``")
    $report.Add("")
    $report.Add("## Summary")
    $report.Add("")
    foreach ($result in $results) {
        $status = if ($result.ExitCode -eq 0) { "ok" } else { "failed" }
        $report.Add("### $($result.Name)")
        $report.Add("")
        $report.Add("- Status: $status")
        $report.Add("- Args: ``$($result.Args)``")
        $report.Add("")
        $report.Add('```text')
        if ($result.Summary.Count -eq 0) {
            $report.Add("(no summary lines captured)")
        } else {
            foreach ($line in $result.Summary) {
                $report.Add($line)
            }
        }
        $report.Add('```')
        $report.Add("")
    }

    $report.Add("## Raw Output")
    $report.Add("")
    foreach ($result in $results) {
        $report.Add("### $($result.Name)")
        $report.Add("")
        $report.Add('```text')
        foreach ($line in $result.Output) {
            $report.Add($line)
        }
        $report.Add('```')
        $report.Add("")
    }

    $outputDir = Split-Path -Parent $OutputPath
    if (-not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir | Out-Null
    }
    $report | Set-Content -Path $OutputPath -Encoding UTF8
    Write-Host "Wrote $OutputPath"

    $failed = @($results | Where-Object { $_.ExitCode -ne 0 })
    if ($failed.Count -gt 0) {
        throw "$($failed.Count) benchmark(s) failed"
    }
}
finally {
    Pop-Location
}
