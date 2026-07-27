param(
    [string]$OutputPath = "",
    [string]$SummaryJsonPath = "",
    [int]$InputWrites = 1000,
    [int]$KeyToScreenRounds = 50,
    [int]$InputBurstWrites = 1000,
    [int]$EchoRounds = 50,
    [int]$FloodLines = 100000,
    [int]$ScrollbackLines = 10000,
    [int]$ViewportScrollLines = 10000,
    [int]$ViewportScrollFloodLines = 5000,
    [int]$PasteKb = 10,
    [int]$DualAgentLines = 5000,
    [int]$AgentStartupLines = 5000,
    [int]$ScreenReadLines = 5000,
    [int]$RenderFrameRounds = 1000,
    [int]$RenderPlanRounds = 1000,
    [int]$RenderGeometryPlanRounds = 1000,
    [int]$RenderSubmissionPlanRounds = 1000,
    [int]$RenderCommitPlanRounds = 1000,
    [int]$RenderCursorMoveRounds = 200,
    [int]$FocusSwitches = 1000,
    [int]$SessionCreates = 20,
    [int]$SessionReadyRounds = 20,
    [int]$TimeoutMs = 120000,
    [int]$MaxInputWriteP95Us = 16000,
    [int]$MaxKeyToScreenP95Us = 16000,
    [int]$MaxInputBurstP95Us = 33000,
    [int]$MaxEchoP95Us = 16000,
    [int]$MaxDualAgentEchoP95Us = 33000,
    [int]$MaxAgentStartupInputP95Us = 33000,
    [int]$MaxPaste10KbMs = 50,
    [int]$MaxScrollbackPageP95Us = 1000,
    [int]$MaxViewportScrollP95Us = 1000,
    [int]$MaxViewportScrollFloodP95Us = 50000,
    [int]$MaxScreenReadFloodP95Us = 50000,
    [int]$MaxRenderFrameP95Us = 1000,
    [int]$MaxRenderPlanP95Us = 1000,
    [int]$MaxRenderGeometryPlanP95Us = 1000,
    [int]$MaxRenderSubmissionPlanP95Us = 1000,
    [int]$MaxRenderCommitPlanP95Us = 1000,
    [int]$MaxRenderDirtyFrameP95Us = 1000,
    [int]$MaxRenderCursorMoveP95Us = 1000,
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
            $_ -match "bench_|session id|activity_process|health_|timed out|Error"
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
    $output = & $script:ExePath --json --wait-ms 500 --env "UNTERM_PROFILE=bench-profile" --env "HTTPS_PROXY=http://127.0.0.1:7890" --write "echo $marker`r" 2>&1
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
    if ($null -eq $json.render_frame) {
        throw "JSON probe did not include render_frame"
    }
    if ($json.render_frame.revision -ne $json.screen.revision) {
        throw "JSON probe render_frame revision did not match screen revision"
    }
    if ($json.render_frame.full -ne $true) {
        throw "JSON probe render_frame did not report a full frame"
    }
    if ($json.render_frame.lines.Count -ne $json.screen.rows) {
        throw "JSON probe render_frame line count $($json.render_frame.lines.Count) did not match screen rows $($json.screen.rows)"
    }
    $renderFrameCellCounts = @($json.render_frame.lines | ForEach-Object { @($_.cells).Count })
    $badCellCounts = @($renderFrameCellCounts | Where-Object { $_ -ne $json.screen.cols })
    if ($badCellCounts.Count -ne 0) {
        throw "JSON probe render_frame cell counts did not match screen cols $($json.screen.cols): $($renderFrameCellCounts -join ',')"
    }
    if ($null -eq $json.render_delta) {
        throw "JSON probe did not include render_delta"
    }
    if ($json.render_delta.revision -ne $json.render_frame.revision) {
        throw "JSON probe render_delta revision did not match render_frame revision"
    }
    if ($json.render_delta.full -eq $true -or $json.render_delta.lines.Count -ne 0) {
        throw "JSON probe render_delta for unchanged revision was not empty"
    }
    if ($null -eq $json.render_draw_plan) {
        throw "JSON probe did not include render_draw_plan"
    }
    if ($json.render_draw_plan.revision -ne $json.render_frame.revision) {
        throw "JSON probe render_draw_plan revision did not match render_frame revision"
    }
    if ($json.render_draw_plan.cols -ne $json.screen.cols -or $json.render_draw_plan.rows -ne $json.screen.rows) {
        throw "JSON probe render_draw_plan dimensions did not match screen dimensions"
    }
    if ($json.render_draw_plan.full -ne $true) {
        throw "JSON probe render_draw_plan did not report a full plan"
    }
    if (@($json.render_draw_plan.glyph_runs).Count -lt 1) {
        throw "JSON probe render_draw_plan did not include glyph runs"
    }
    if (@($json.render_draw_plan.cell_runs).Count -lt $json.screen.rows) {
        throw "JSON probe render_draw_plan did not include enough cell runs"
    }
    if ($null -eq $json.render_draw_plan.cursor) {
        throw "JSON probe render_draw_plan did not include cursor draw state"
    }
    if ($null -eq $json.render_draw_delta) {
        throw "JSON probe did not include render_draw_delta"
    }
    if ($json.render_draw_delta.revision -ne $json.render_frame.revision) {
        throw "JSON probe render_draw_delta revision did not match render_frame revision"
    }
    if ($json.render_draw_delta.full -eq $true) {
        throw "JSON probe unchanged render_draw_delta reported a full plan"
    }
    if (@($json.render_draw_delta.glyph_runs).Count -ne 0 -or @($json.render_draw_delta.cell_runs).Count -ne 0) {
        throw "JSON probe unchanged render_draw_delta was not empty"
    }
    if ($null -eq $json.render_draw_delta.cursor) {
        throw "JSON probe unchanged render_draw_delta did not include cursor draw state"
    }
    if ($null -eq $json.render_geometry_plan) {
        throw "JSON probe did not include render_geometry_plan"
    }
    if ($json.render_geometry_plan.revision -ne $json.render_frame.revision) {
        throw "JSON probe render_geometry_plan revision did not match render_frame revision"
    }
    if ($json.render_geometry_plan.viewport.width -ne ($json.screen.cols * 8) -or $json.render_geometry_plan.viewport.height -ne ($json.screen.rows * 16)) {
        throw "JSON probe render_geometry_plan viewport dimensions did not match 8x16 cell metrics"
    }
    if (@($json.render_geometry_plan.glyph_runs).Count -lt 1 -or @($json.render_geometry_plan.cell_runs).Count -lt $json.screen.rows) {
        throw "JSON probe render_geometry_plan did not include expected run geometry"
    }
    if ($null -eq $json.render_geometry_plan.cursor) {
        throw "JSON probe render_geometry_plan did not include cursor geometry"
    }
    if ($null -eq $json.render_submission_plan) {
        throw "JSON probe did not include render_submission_plan"
    }
    if ($json.render_submission_plan.revision -ne $json.render_frame.revision) {
        throw "JSON probe render_submission_plan revision did not match render_frame revision"
    }
    if ($json.render_submission_plan.viewport.width -ne $json.render_geometry_plan.viewport.width -or $json.render_submission_plan.viewport.height -ne $json.render_geometry_plan.viewport.height) {
        throw "JSON probe render_submission_plan viewport did not match render_geometry_plan viewport"
    }
    if (@($json.render_submission_plan.damage_rects).Count -lt 1) {
        throw "JSON probe render_submission_plan did not include damage rects"
    }
    if (@($json.render_submission_plan.text_runs).Count -lt 1 -or @($json.render_submission_plan.background_quads).Count -lt $json.screen.rows) {
        throw "JSON probe render_submission_plan did not include expected renderer commands"
    }
    if ($null -eq $json.render_submission_plan.cursor) {
        throw "JSON probe render_submission_plan did not include cursor quad"
    }
    if ($null -eq $json.render_commit_plan) {
        throw "JSON probe did not include render_commit_plan"
    }
    if ($json.render_commit_plan.revision -ne $json.render_frame.revision) {
        throw "JSON probe render_commit_plan revision did not match render_frame revision"
    }
    if ($json.render_commit_plan.submit -ne $true) {
        throw "JSON probe render_commit_plan did not request first-frame submission"
    }
    if ($json.render_commit_plan.requires_full_repaint -ne $true) {
        throw "JSON probe render_commit_plan did not force first-frame full repaint"
    }
    if ($null -eq $json.render_commit_plan.submission -or @($json.render_commit_plan.submission.damage_rects).Count -lt 1) {
        throw "JSON probe render_commit_plan did not include submission damage"
    }
    if ($null -eq $json.activity) {
        throw "JSON probe did not include an activity snapshot"
    }
    if ($null -eq $json.activity.process -or [string]::IsNullOrWhiteSpace($json.activity.process.foreground_process)) {
        throw "JSON probe did not include next-core process activity diagnostics"
    }
    if ([string]::IsNullOrWhiteSpace($json.session.shell.cwd)) {
        throw "JSON probe did not include next-core session cwd fallback"
    }
    if ([string]::IsNullOrWhiteSpace($json.activity.process.foreground_cwd) -and [string]::IsNullOrWhiteSpace($json.activity.process.root_cwd)) {
        throw "JSON probe did not include next-core process cwd diagnostics"
    }
    if ($json.session.shell.launch_context.profile -ne "bench-profile") {
        throw "JSON probe did not include launch profile diagnostics"
    }
    if (-not @($json.session.shell.launch_context.proxy_env_keys | Where-Object { $_ -eq "HTTPS_PROXY" })) {
        throw "JSON probe did not include launch proxy env diagnostics"
    }
    if ($null -eq $json.activity.screen -or $json.activity.screen.total_reads -lt 1) {
        throw "JSON probe did not include screen activity counters"
    }
    if ($null -eq $json.health.io -or $json.health.io.screen_reads -lt 1) {
        throw "JSON probe did not include aggregate screen read counters"
    }
    if ($json.session.is_dead -eq $true -and [string]::IsNullOrWhiteSpace($json.session.dead_reason)) {
        throw "JSON probe reported a dead session without dead_reason"
    }
    if ($null -eq $json.health.lifecycle -or $json.health.lifecycle.total_created -lt 1) {
        throw "JSON probe did not include lifecycle health counters"
    }
    if ($null -eq $json.health.runtime_pump) {
        throw "JSON probe did not include runtime pump health counters"
    }
    if ($json.health.runtime_pump.drain_calls -lt 1) {
        throw "JSON probe runtime pump did not record drain calls"
    }
    if ($json.health.runtime_pump.dispatched_commands -lt 1) {
        throw "JSON probe runtime pump did not record dispatches"
    }
    if ($json.health.runtime_pump.dispatched_render_commands -lt 1 -or $json.health.runtime_pump.dispatched_screen_commands -lt 1) {
        throw "JSON probe runtime pump did not record render/screen lane dispatches"
    }
    if ($json.health.runtime_pump.max_dispatch_elapsed_micros -gt $json.health.runtime_pump.total_dispatch_elapsed_micros) {
        throw "JSON probe runtime pump dispatch max exceeded total"
    }
    if ($json.health.runtime_pump.max_drain_elapsed_micros -gt $json.health.runtime_pump.total_drain_elapsed_micros) {
        throw "JSON probe runtime pump drain max exceeded total"
    }

    [pscustomobject]@{
        Marker = $marker
        Engine = $json.health.engine
        Screen = "$($json.screen.cols)x$($json.screen.rows)"
        RawBytes = $json.raw_bytes
        ScreenReads = $json.health.io.screen_reads
        RenderFrameRevision = $json.render_frame.revision
        RenderFrameLines = $json.render_frame.lines.Count
        RenderFrameCols = $json.screen.cols
        RenderFrameCellCounts = @($renderFrameCellCounts)
        RenderFrameGridCells = ($renderFrameCellCounts | Measure-Object -Sum).Sum
        RenderDeltaLines = $json.render_delta.lines.Count
        RenderDrawPlanRevision = $json.render_draw_plan.revision
        RenderDrawPlanGlyphRuns = @($json.render_draw_plan.glyph_runs).Count
        RenderDrawPlanCellRuns = @($json.render_draw_plan.cell_runs).Count
        RenderDrawPlanCursor = $null -ne $json.render_draw_plan.cursor
        RenderDrawDeltaGlyphRuns = @($json.render_draw_delta.glyph_runs).Count
        RenderDrawDeltaCellRuns = @($json.render_draw_delta.cell_runs).Count
        RenderDrawDeltaCursor = $null -ne $json.render_draw_delta.cursor
        RenderGeometryViewportWidth = $json.render_geometry_plan.viewport.width
        RenderGeometryViewportHeight = $json.render_geometry_plan.viewport.height
        RenderGeometryGlyphRuns = @($json.render_geometry_plan.glyph_runs).Count
        RenderGeometryCellRuns = @($json.render_geometry_plan.cell_runs).Count
        RenderGeometryCursor = $null -ne $json.render_geometry_plan.cursor
        RenderSubmissionDamageRects = @($json.render_submission_plan.damage_rects).Count
        RenderSubmissionTextRuns = @($json.render_submission_plan.text_runs).Count
        RenderSubmissionBackgroundQuads = @($json.render_submission_plan.background_quads).Count
        RenderSubmissionCursor = $null -ne $json.render_submission_plan.cursor
        RenderCommitSubmit = $json.render_commit_plan.submit
        RenderCommitFullRepaint = $json.render_commit_plan.requires_full_repaint
        RenderCommitDamageRects = @($json.render_commit_plan.submission.damage_rects).Count
        ForegroundProcess = $json.activity.process.foreground_process
        Cwd = $json.session.shell.cwd
        Profile = $json.session.shell.launch_context.profile
        ProxyEnvKeys = @($json.session.shell.launch_context.proxy_env_keys)
        DeadReason = $json.session.dead_reason
        LifecycleCreated = $json.health.lifecycle.total_created
        RuntimePumpDrainCalls = $json.health.runtime_pump.drain_calls
        RuntimePumpDispatched = $json.health.runtime_pump.dispatched_commands
        RuntimePumpLifecycle = $json.health.runtime_pump.dispatched_lifecycle_commands
        RuntimePumpInput = $json.health.runtime_pump.dispatched_input_commands
        RuntimePumpRender = $json.health.runtime_pump.dispatched_render_commands
        RuntimePumpScreen = $json.health.runtime_pump.dispatched_screen_commands
        RuntimePumpBackground = $json.health.runtime_pump.dispatched_background_commands
        RuntimePumpWaitedForResponse = $json.health.runtime_pump.waited_for_response
        RuntimePumpCompletedWithoutWait = $json.health.runtime_pump.completed_without_wait
        RuntimePumpMaxDispatchUs = $json.health.runtime_pump.max_dispatch_elapsed_micros
        RuntimePumpMaxDrainUs = $json.health.runtime_pump.max_drain_elapsed_micros
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
    $results += Invoke-Benchmark -Name "key-to-screen latency" -BenchArgs ([string[]](@("--bench-key-to-screen", "$KeyToScreenRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "input burst under output" -BenchArgs ([string[]](@("--bench-input-burst", "$InputBurstWrites") + $commonTail))
    $results += Invoke-Benchmark -Name "echo latency" -BenchArgs ([string[]](@("--bench-echo", "$EchoRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "output flood" -BenchArgs ([string[]](@("--bench-flood-lines", "$FloodLines") + $commonTail))
    $results += Invoke-Benchmark -Name "paste 10kb" -BenchArgs ([string[]](@("--bench-paste-kb", "$PasteKb") + $commonTail))
    $results += Invoke-Benchmark -Name "scrollback paging" -BenchArgs ([string[]](@("--bench-scrollback-lines", "$ScrollbackLines") + $commonTail))
    $results += Invoke-Benchmark -Name "viewport scroll paging" -BenchArgs ([string[]](@("--bench-viewport-scrolls", "$ViewportScrollLines") + $commonTail))
    $results += Invoke-Benchmark -Name "viewport scroll during flood" -BenchArgs ([string[]](@("--bench-viewport-scroll-flood", "$ViewportScrollFloodLines") + $commonTail))
    $results += Invoke-Benchmark -Name "dual pseudo-agent output" -BenchArgs ([string[]](@("--bench-dual-agent-lines", "$DualAgentLines") + $commonTail))
    $results += Invoke-Benchmark -Name "agent startup stall" -BenchArgs ([string[]](@("--bench-agent-startup-lines", "$AgentStartupLines") + $commonTail))
    $results += Invoke-Benchmark -Name "screen read during flood" -BenchArgs ([string[]](@("--bench-screen-read-lines", "$ScreenReadLines") + $commonTail))
    $results += Invoke-Benchmark -Name "render frame latency" -BenchArgs ([string[]](@("--bench-render-frames", "$RenderFrameRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "render draw plan latency" -BenchArgs ([string[]](@("--bench-render-plans", "$RenderPlanRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "render geometry plan latency" -BenchArgs ([string[]](@("--bench-render-geometry-plans", "$RenderGeometryPlanRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "render submission plan latency" -BenchArgs ([string[]](@("--bench-render-submission-plans", "$RenderSubmissionPlanRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "render commit plan latency" -BenchArgs ([string[]](@("--bench-render-commit-plans", "$RenderCommitPlanRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "render cursor move latency" -BenchArgs ([string[]](@("--bench-render-cursor-moves", "$RenderCursorMoveRounds") + $commonTail))
    $results += Invoke-Benchmark -Name "focus switch latency" -BenchArgs ([string[]](@("--bench-focus-switches", "$FocusSwitches") + $commonTail))
    $results += Invoke-Benchmark -Name "session create latency" -BenchArgs ([string[]](@("--bench-session-create", "$SessionCreates") + $commonTail))
    $results += Invoke-Benchmark -Name "session ready latency" -BenchArgs ([string[]](@("--bench-session-ready", "$SessionReadyRounds") + $commonTail))

    $inputWrite = Find-BenchmarkResult -Results $results -Name "input write latency"
    $keyToScreen = Find-BenchmarkResult -Results $results -Name "key-to-screen latency"
    $inputBurst = Find-BenchmarkResult -Results $results -Name "input burst under output"
    $echo = Find-BenchmarkResult -Results $results -Name "echo latency"
    $paste = Find-BenchmarkResult -Results $results -Name "paste 10kb"
    $scrollback = Find-BenchmarkResult -Results $results -Name "scrollback paging"
    $viewportScroll = Find-BenchmarkResult -Results $results -Name "viewport scroll paging"
    $viewportScrollFlood = Find-BenchmarkResult -Results $results -Name "viewport scroll during flood"
    $dualAgent = Find-BenchmarkResult -Results $results -Name "dual pseudo-agent output"
    $agentStartup = Find-BenchmarkResult -Results $results -Name "agent startup stall"
    $screenRead = Find-BenchmarkResult -Results $results -Name "screen read during flood"
    $renderFrame = Find-BenchmarkResult -Results $results -Name "render frame latency"
    $renderPlan = Find-BenchmarkResult -Results $results -Name "render draw plan latency"
    $renderGeometryPlan = Find-BenchmarkResult -Results $results -Name "render geometry plan latency"
    $renderSubmissionPlan = Find-BenchmarkResult -Results $results -Name "render submission plan latency"
    $renderCommitPlan = Find-BenchmarkResult -Results $results -Name "render commit plan latency"
    $renderCursorMove = Find-BenchmarkResult -Results $results -Name "render cursor move latency"
    $focusSwitch = Find-BenchmarkResult -Results $results -Name "focus switch latency"
    $sessionCreate = Find-BenchmarkResult -Results $results -Name "session create latency"
    $sessionReady = Find-BenchmarkResult -Results $results -Name "session ready latency"
    $gates = @()
    $gates += New-Gate -GateName "input write p95" -Actual (Get-BenchMetric -Result $inputWrite -LinePrefix "bench_input_write" -Metric "p95_us") -Max $MaxInputWriteP95Us -Unit "us"
    $gates += New-Gate -GateName "key-to-screen p95" -Actual (Get-BenchMetric -Result $keyToScreen -LinePrefix "bench_key_to_screen" -Metric "p95_us") -Max $MaxKeyToScreenP95Us -Unit "us"
    $gates += New-Gate -GateName "input burst p95" -Actual (Get-BenchMetric -Result $inputBurst -LinePrefix "bench_input_burst" -Metric "p95_us") -Max $MaxInputBurstP95Us -Unit "us"
    $gates += New-Gate -GateName "echo p95" -Actual (Get-BenchMetric -Result $echo -LinePrefix "bench_echo" -Metric "p95_us") -Max $MaxEchoP95Us -Unit "us"
    $gates += New-Gate -GateName "dual-agent echo p95" -Actual (Get-BenchMetric -Result $dualAgent -LinePrefix "bench_dual_agents_echo" -Metric "p95_us") -Max $MaxDualAgentEchoP95Us -Unit "us"
    $gates += New-Gate -GateName "agent startup input p95" -Actual (Get-BenchMetric -Result $agentStartup -LinePrefix "bench_agent_startup_stall" -Metric "input_p95_us") -Max $MaxAgentStartupInputP95Us -Unit "us"
    $gates += New-Gate -GateName "paste 10kb elapsed" -Actual (Get-BenchMetric -Result $paste -LinePrefix "bench_paste" -Metric "elapsed_ms") -Max $MaxPaste10KbMs -Unit "ms"
    $gates += New-Gate -GateName "scrollback page p95" -Actual (Get-BenchMetric -Result $scrollback -LinePrefix "bench_scrollback" -Metric "p95_us") -Max $MaxScrollbackPageP95Us -Unit "us"
    $gates += New-Gate -GateName "viewport scroll p95" -Actual (Get-BenchMetric -Result $viewportScroll -LinePrefix "bench_viewport_scroll" -Metric "p95_us") -Max $MaxViewportScrollP95Us -Unit "us"
    $gates += New-Gate -GateName "viewport scroll under flood p95" -Actual (Get-BenchMetric -Result $viewportScrollFlood -LinePrefix "bench_viewport_scroll_flood" -Metric "p95_us") -Max $MaxViewportScrollFloodP95Us -Unit "us"
    $gates += New-Gate -GateName "screen read under flood p95" -Actual (Get-BenchMetric -Result $screenRead -LinePrefix "bench_screen_read_flood" -Metric "p95_us") -Max $MaxScreenReadFloodP95Us -Unit "us"
    $gates += New-Gate -GateName "render frame p95" -Actual (Get-BenchMetric -Result $renderFrame -LinePrefix "bench_render_frame" -Metric "p95_us") -Max $MaxRenderFrameP95Us -Unit "us"
    $gates += New-Gate -GateName "render draw plan p95" -Actual (Get-BenchMetric -Result $renderPlan -LinePrefix "bench_render_plan" -Metric "p95_us") -Max $MaxRenderPlanP95Us -Unit "us"
    $gates += New-Gate -GateName "render geometry plan p95" -Actual (Get-BenchMetric -Result $renderGeometryPlan -LinePrefix "bench_render_geometry_plan" -Metric "p95_us") -Max $MaxRenderGeometryPlanP95Us -Unit "us"
    $gates += New-Gate -GateName "render submission plan p95" -Actual (Get-BenchMetric -Result $renderSubmissionPlan -LinePrefix "bench_render_submission_plan" -Metric "p95_us") -Max $MaxRenderSubmissionPlanP95Us -Unit "us"
    $gates += New-Gate -GateName "render commit plan p95" -Actual (Get-BenchMetric -Result $renderCommitPlan -LinePrefix "bench_render_commit_plan" -Metric "full_p95_us") -Max $MaxRenderCommitPlanP95Us -Unit "us"
    $gates += New-Gate -GateName "render dirty frame p95" -Actual (Get-BenchMetric -Result $renderFrame -LinePrefix "bench_render_frame" -Metric "dirty_p95_us") -Max $MaxRenderDirtyFrameP95Us -Unit "us"
    $gates += New-Gate -GateName "render cursor move p95" -Actual (Get-BenchMetric -Result $renderCursorMove -LinePrefix "bench_render_cursor_move" -Metric "p95_us") -Max $MaxRenderCursorMoveP95Us -Unit "us"
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
    $report.Add("- JSON smoke: ``$($jsonSmoke.Engine) $($jsonSmoke.Screen) raw_bytes=$($jsonSmoke.RawBytes) foreground=$($jsonSmoke.ForegroundProcess) cwd=$($jsonSmoke.Cwd) profile=$($jsonSmoke.Profile) proxy_keys=$($jsonSmoke.ProxyEnvKeys -join ',') screen_reads=$($jsonSmoke.ScreenReads) render_frame_revision=$($jsonSmoke.RenderFrameRevision) render_frame_lines=$($jsonSmoke.RenderFrameLines) render_frame_cols=$($jsonSmoke.RenderFrameCols) render_frame_grid_cells=$($jsonSmoke.RenderFrameGridCells) render_delta_lines=$($jsonSmoke.RenderDeltaLines) render_draw_plan_revision=$($jsonSmoke.RenderDrawPlanRevision) render_draw_plan_glyph_runs=$($jsonSmoke.RenderDrawPlanGlyphRuns) render_draw_plan_cell_runs=$($jsonSmoke.RenderDrawPlanCellRuns) render_draw_plan_cursor=$($jsonSmoke.RenderDrawPlanCursor) render_draw_delta_glyph_runs=$($jsonSmoke.RenderDrawDeltaGlyphRuns) render_draw_delta_cell_runs=$($jsonSmoke.RenderDrawDeltaCellRuns) render_draw_delta_cursor=$($jsonSmoke.RenderDrawDeltaCursor) render_geometry_viewport=$($jsonSmoke.RenderGeometryViewportWidth)x$($jsonSmoke.RenderGeometryViewportHeight) render_geometry_glyph_runs=$($jsonSmoke.RenderGeometryGlyphRuns) render_geometry_cell_runs=$($jsonSmoke.RenderGeometryCellRuns) render_geometry_cursor=$($jsonSmoke.RenderGeometryCursor) render_submission_damage_rects=$($jsonSmoke.RenderSubmissionDamageRects) render_submission_text_runs=$($jsonSmoke.RenderSubmissionTextRuns) render_submission_background_quads=$($jsonSmoke.RenderSubmissionBackgroundQuads) render_submission_cursor=$($jsonSmoke.RenderSubmissionCursor) render_commit_submit=$($jsonSmoke.RenderCommitSubmit) render_commit_full_repaint=$($jsonSmoke.RenderCommitFullRepaint) render_commit_damage_rects=$($jsonSmoke.RenderCommitDamageRects) runtime_pump_dispatches=$($jsonSmoke.RuntimePumpDispatched) runtime_pump_lanes=lifecycle:$($jsonSmoke.RuntimePumpLifecycle),input:$($jsonSmoke.RuntimePumpInput),render:$($jsonSmoke.RuntimePumpRender),screen:$($jsonSmoke.RuntimePumpScreen),background:$($jsonSmoke.RuntimePumpBackground) runtime_pump_waited=$($jsonSmoke.RuntimePumpWaitedForResponse) runtime_pump_completed_without_wait=$($jsonSmoke.RuntimePumpCompletedWithoutWait) runtime_pump_max_dispatch_us=$($jsonSmoke.RuntimePumpMaxDispatchUs) runtime_pump_max_drain_us=$($jsonSmoke.RuntimePumpMaxDrainUs) lifecycle_created=$($jsonSmoke.LifecycleCreated) dead_reason=$($jsonSmoke.DeadReason)``")
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
