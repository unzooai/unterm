param(
    [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

$CiDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $CiDir "..")

$TestFilter = "runtime"
$ExpectedCount = 103
$RequiredTests = @(
    "next_core::runtime::command::tests::classifies_input_as_latency_sensitive_write_path",
    "next_core::runtime::command::tests::classifies_render_reads_as_latency_sensitive_read_path",
    "next_core::runtime::command::tests::default_queue_policy_sets_bounded_backpressure_budget",
    "next_core::runtime::queue::tests::rejects_when_command_budget_is_full",
    "next_core::runtime::queue::tests::rejects_when_input_byte_budget_is_full",
    "next_core::runtime::queue::tests::dequeue_lane_preserves_other_lane_backlog",
    "next_core::runtime::consumer::tests::dispatch_next_scheduled_uses_input_first_policy",
    "next_core::runtime::pump::tests::drain_until_response_pumps_until_attached_response_completes",
    "next_core::runtime::pump::tests::drain_until_response_report_counts_rejected_immediate_completion",
    "next_core::runtime::scheduler::tests::submit_input_dispatches_before_older_screen_backlog",
    "next_core::runtime::scheduler::tests::viewport_scrolls_dispatch_before_background_backlog",
    "next_core::runtime::scheduler::tests::lifecycle_dispatches_before_older_render_and_screen_backlog",
    "next_core::runtime::scheduler::tests::session_query_backlog_waits_behind_input",
    "next_core::runtime::scheduler::tests::recording_backlog_waits_behind_lifecycle",
    "next_core::runtime::scheduler::tests::render_frame_reads_enter_runtime_queue_before_dispatch",
    "next_core::runtime::screen_executor::tests::rejects_non_render_screen_reads_before_dispatch",
    "next_core::health_snapshot::tests::runtime_pump_health_reports_accumulated_stats"
)

Push-Location $RepoRoot
try {
    $list = @(& cargo test -p unterm-engine $TestFilter -- --list 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test list failed:`n$($list -join "`n")"
    }

    foreach ($test in $RequiredTests) {
        if (-not @($list | Where-Object { $_ -like "$test`: test*" })) {
            throw "missing required next-core runtime test: $test"
        }
    }

    if ($ListOnly) {
        Write-Host "next-core runtime tests present: required=$($RequiredTests.Count)"
        exit 0
    }

    $run = @(& cargo test -p unterm-engine $TestFilter -- --test-threads=1 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0) {
        throw "next-core runtime tests failed:`n$($run -join "`n")"
    }
    if (-not @($run | Where-Object { $_ -match "test result: ok\..*$ExpectedCount passed" })) {
        throw "next-core runtime test run did not report $ExpectedCount passed tests"
    }

    Write-Host "next-core runtime tests ok: required=$($RequiredTests.Count) module_tests=$ExpectedCount"
} finally {
    Pop-Location
}
