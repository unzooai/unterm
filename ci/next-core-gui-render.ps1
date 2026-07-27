param(
    [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

$CiDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $CiDir "..")

$RequiredTests = @(
    "engine::tests::next_core_facade_reads_render_commit_plan",
    "engine::tests::engine_render_consumer_skips_repeated_next_core_revision",
    "engine::tests::command_list_backend_prepares_next_core_commit_commands",
    "engine::tests::engine_render_consumer_reads_next_core_buffer_plan",
    "engine::tests::engine_render_prepared_pane_frame_builds_replace_diagnostics",
    "engine::tests::engine_render_consumer_set_reuses_state_and_resizes_metrics"
)

Push-Location $RepoRoot
try {
    $list = @(& cargo test -p unterm engine::tests:: -- --list 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test list failed:`n$($list -join "`n")"
    }

    foreach ($test in $RequiredTests) {
        if (-not @($list | Where-Object { $_ -like "$test`: test*" })) {
            throw "missing required next-core GUI render test: $test"
        }
    }

    if ($ListOnly) {
        Write-Host "next-core GUI render tests present: $($RequiredTests.Count)"
        exit 0
    }

    $run = @(& cargo test -p unterm engine::tests:: -- --test-threads=1 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0) {
        throw "next-core GUI render tests failed:`n$($run -join "`n")"
    }
    if (-not @($run | Where-Object { $_ -match "test result: ok\..*8 passed" })) {
        throw "next-core GUI render test run did not report 8 passed tests"
    }

    Write-Host "next-core GUI render tests ok: required=$($RequiredTests.Count) module_tests=8"
} finally {
    Pop-Location
}
