param(
    [string]$OutputPath = "",
    [string]$SummaryJsonPath = "",
    [int]$InputWrites = 1000,
    [int]$InputBurstWrites = 1000,
    [int]$EchoRounds = 50,
    [int]$FloodLines = 100000,
    [int]$ScrollbackLines = 10000,
    [int]$ViewportScrollLines = 10000,
    [int]$ViewportScrollFloodLines = 5000,
    [int]$PasteKb = 10,
    [int]$DualAgentLines = 5000,
    [int]$ScreenReadLines = 5000,
    [int]$FocusSwitches = 1000,
    [int]$SessionCreates = 20,
    [int]$SessionReadyRounds = 20,
    [int]$TimeoutMs = 120000,
    [int]$MaxInputWriteP95Us = 16000,
    [int]$MaxInputBurstP95Us = 33000,
    [int]$MaxEchoP95Us = 16000,
    [int]$MaxDualAgentEchoP95Us = 33000,
    [int]$MaxPaste10KbMs = 50,
    [int]$MaxScrollbackPageP95Us = 1000,
    [int]$MaxViewportScrollP95Us = 1000,
    [int]$MaxViewportScrollFloodP95Us = 50000,
    [int]$MaxScreenReadFloodP95Us = 50000,
    [int]$MaxFocusSwitchP95Us = 100000,
    [int]$MaxSessionCreateP95Us = 100000,
    [int]$MaxSessionReadyP95Us = 100000,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$EngineDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $EngineDir "..")
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $RepoRoot "docs\next-core-benchmark-report.md"
}
if ([string]::IsNullOrWhiteSpace($SummaryJsonPath)) {
    $SummaryJsonPath = Join-Path $RepoRoot "docs\next-core-benchmark-summary.json"
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

function ConvertTo-BenchmarkSummary {
    param(
        [pscustomobject]$Result
    )

    [pscustomobject]@{
        name = $Result.Name
        exit_code = $Result.ExitCode
        args = $Result.Args
        summary = @($Result.Summary)
    }
}

function Get-BenchMetric {
    param(
        [pscustomobject]$Result,
        [string]$LinePrefix,
        [string]$Metric
    )

    $line = @($Result.Summary | Where-Object { $_ -like "$LinePrefix*" } | Select-Object -First 1)
    if ($line.Count -eq 0) {
        return $null
    }

    $match = [regex]::Match($line[0], "(^|\s)$Metric=([0-9]+(?:\.[0-9]+)?)")
    if (-not $match.Success) {
        return $null
    }
    return [double]$match.Groups[2].Value
}

function Find-BenchmarkResult {
    param(
        [pscustomobject[]]$Results,
        [string]$Name
    )

    return @($Results | Where-Object { $_.Name -eq $Name } | Select-Object -First 1)[0]
}

function New-Gate {
    param(
        [string]$GateName,
        [Nullable[double]]$Actual,
        [double]$Max,
        [string]$Unit
    )

    $ok = ($null -ne $Actual) -and ($Actual -le $Max)
    [pscustomobject]@{
        Name = $GateName
        Actual = $Actual
        Max = $Max
        Unit = $Unit
        Ok = $ok
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
    $results += Invoke-Benchmark -Name "input burst under output" -BenchArgs ([string[]](@("--bench-input-burst", "$InputBurstWrites") + $commonTail))
    $results += Invoke-Benchmark -Name "echo latency" -BenchArgs ([string[]](@("--bench-echo", "$EchoRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "output flood" -BenchArgs ([string[]](@("--bench-flood-lines", "$FloodLines") + $commonTail))
    $results += Invoke-Benchmark -Name "paste 10kb" -BenchArgs ([string[]](@("--bench-paste-kb", "$PasteKb") + $commonTail))
    $results += Invoke-Benchmark -Name "scrollback paging" -BenchArgs ([string[]](@("--bench-scrollback-lines", "$ScrollbackLines") + $commonTail))
    $results += Invoke-Benchmark -Name "viewport scroll paging" -BenchArgs ([string[]](@("--bench-viewport-scrolls", "$ViewportScrollLines") + $commonTail))
    $results += Invoke-Benchmark -Name "viewport scroll during flood" -BenchArgs ([string[]](@("--bench-viewport-scroll-flood", "$ViewportScrollFloodLines") + $commonTail))
    $results += Invoke-Benchmark -Name "dual pseudo-agent output" -BenchArgs ([string[]](@("--bench-dual-agent-lines", "$DualAgentLines") + $commonTail))
    $results += Invoke-Benchmark -Name "screen read during flood" -BenchArgs ([string[]](@("--bench-screen-read-lines", "$ScreenReadLines") + $commonTail))
    $results += Invoke-Benchmark -Name "focus switch latency" -BenchArgs ([string[]](@("--bench-focus-switches", "$FocusSwitches") + $commonTail))
    $results += Invoke-Benchmark -Name "session create latency" -BenchArgs ([string[]](@("--bench-session-create", "$SessionCreates") + $commonTail))
    $results += Invoke-Benchmark -Name "session ready latency" -BenchArgs ([string[]](@("--bench-session-ready", "$SessionReadyRounds") + $commonTail))

    $inputWrite = Find-BenchmarkResult -Results $results -Name "input write latency"
    $inputBurst = Find-BenchmarkResult -Results $results -Name "input burst under output"
    $echo = Find-BenchmarkResult -Results $results -Name "echo latency"
    $paste = Find-BenchmarkResult -Results $results -Name "paste 10kb"
    $scrollback = Find-BenchmarkResult -Results $results -Name "scrollback paging"
    $viewportScroll = Find-BenchmarkResult -Results $results -Name "viewport scroll paging"
    $viewportScrollFlood = Find-BenchmarkResult -Results $results -Name "viewport scroll during flood"
    $dualAgent = Find-BenchmarkResult -Results $results -Name "dual pseudo-agent output"
    $screenRead = Find-BenchmarkResult -Results $results -Name "screen read during flood"
    $focusSwitch = Find-BenchmarkResult -Results $results -Name "focus switch latency"
    $sessionCreate = Find-BenchmarkResult -Results $results -Name "session create latency"
    $sessionReady = Find-BenchmarkResult -Results $results -Name "session ready latency"
    $gates = @()
    $gates += New-Gate -GateName "input write p95" -Actual (Get-BenchMetric -Result $inputWrite -LinePrefix "bench_input_write" -Metric "p95_us") -Max $MaxInputWriteP95Us -Unit "us"
    $gates += New-Gate -GateName "input burst p95" -Actual (Get-BenchMetric -Result $inputBurst -LinePrefix "bench_input_burst" -Metric "p95_us") -Max $MaxInputBurstP95Us -Unit "us"
    $gates += New-Gate -GateName "echo p95" -Actual (Get-BenchMetric -Result $echo -LinePrefix "bench_echo" -Metric "p95_us") -Max $MaxEchoP95Us -Unit "us"
    $gates += New-Gate -GateName "dual-agent echo p95" -Actual (Get-BenchMetric -Result $dualAgent -LinePrefix "bench_dual_agents_echo" -Metric "p95_us") -Max $MaxDualAgentEchoP95Us -Unit "us"
    $gates += New-Gate -GateName "paste 10kb elapsed" -Actual (Get-BenchMetric -Result $paste -LinePrefix "bench_paste" -Metric "elapsed_ms") -Max $MaxPaste10KbMs -Unit "ms"
    $gates += New-Gate -GateName "scrollback page p95" -Actual (Get-BenchMetric -Result $scrollback -LinePrefix "bench_scrollback" -Metric "p95_us") -Max $MaxScrollbackPageP95Us -Unit "us"
    $gates += New-Gate -GateName "viewport scroll p95" -Actual (Get-BenchMetric -Result $viewportScroll -LinePrefix "bench_viewport_scroll" -Metric "p95_us") -Max $MaxViewportScrollP95Us -Unit "us"
    $gates += New-Gate -GateName "viewport scroll under flood p95" -Actual (Get-BenchMetric -Result $viewportScrollFlood -LinePrefix "bench_viewport_scroll_flood" -Metric "p95_us") -Max $MaxViewportScrollFloodP95Us -Unit "us"
    $gates += New-Gate -GateName "screen read under flood p95" -Actual (Get-BenchMetric -Result $screenRead -LinePrefix "bench_screen_read_flood" -Metric "p95_us") -Max $MaxScreenReadFloodP95Us -Unit "us"
    $gates += New-Gate -GateName "focus switch p95" -Actual (Get-BenchMetric -Result $focusSwitch -LinePrefix "bench_focus_switch" -Metric "p95_us") -Max $MaxFocusSwitchP95Us -Unit "us"
    $gates += New-Gate -GateName "session create p95" -Actual (Get-BenchMetric -Result $sessionCreate -LinePrefix "bench_session_create" -Metric "p95_us") -Max $MaxSessionCreateP95Us -Unit "us"
    $gates += New-Gate -GateName "session ready p95" -Actual (Get-BenchMetric -Result $sessionReady -LinePrefix "bench_session_ready" -Metric "p95_us") -Max $MaxSessionReadyP95Us -Unit "us"

    $commit = (& git rev-parse --short HEAD).Trim()
    $date = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss zzz")
    $machine = "$([System.Environment]::MachineName)"
    $os = [System.Environment]::OSVersion.VersionString
    $failed = @($results | Where-Object { $_.ExitCode -ne 0 })
    $failedGates = @($gates | Where-Object { -not $_.Ok })

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
    $report.Add("## Gates")
    $report.Add("")
    $report.Add("| Gate | Actual | Max | Status |")
    $report.Add("| --- | ---: | ---: | --- |")
    foreach ($gate in $gates) {
        $actual = if ($null -eq $gate.Actual) { "missing" } else { "$($gate.Actual) $($gate.Unit)" }
        $max = "$($gate.Max) $($gate.Unit)"
        $status = if ($gate.Ok) { "ok" } else { "failed" }
        $report.Add("| $($gate.Name) | $actual | $max | $status |")
    }
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
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    $resolvedOutputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputPath)
    [System.IO.File]::WriteAllText($resolvedOutputPath, (($report -join "`n") + "`n"), $utf8NoBom)
    Write-Host "Wrote $OutputPath"

    $summary = [pscustomobject]@{
        ok = (($failed.Count -eq 0) -and ($failedGates.Count -eq 0))
        generated = $date
        commit = $commit
        machine = $machine
        os = $os
        binary = "target\debug\unterm-next-core.exe"
        json_smoke = $jsonSmoke
        gates = @($gates | ForEach-Object {
            [pscustomobject]@{
                name = $_.Name
                actual = $_.Actual
                max = $_.Max
                unit = $_.Unit
                ok = $_.Ok
            }
        })
        benchmarks = @($results | ForEach-Object { ConvertTo-BenchmarkSummary -Result $_ })
    }
    $summaryJson = (($summary | ConvertTo-Json -Depth 6) -replace "`r`n", "`n") -replace "(?m)[ \t]+$", ""
    $summaryJsonDir = Split-Path -Parent $SummaryJsonPath
    if (-not (Test-Path $summaryJsonDir)) {
        New-Item -ItemType Directory -Path $summaryJsonDir | Out-Null
    }
    $resolvedSummaryJsonPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($SummaryJsonPath)
    [System.IO.File]::WriteAllText($resolvedSummaryJsonPath, ($summaryJson + "`n"), $utf8NoBom)
    Write-Host "Wrote $SummaryJsonPath"

    if ($failed.Count -gt 0) {
        throw "$($failed.Count) benchmark(s) failed"
    }
    if ($failedGates.Count -gt 0) {
        throw "$($failedGates.Count) benchmark gate(s) failed"
    }
}
finally {
    Pop-Location
}
