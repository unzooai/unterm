param(
    [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

$CiDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $CiDir "..")

$TestFilter = "mcp::handler::engine_neutral_handler_tests::"
$RequiredTests = @(
    "mcp::handler::engine_neutral_handler_tests::server_health_uses_selected_next_core_engine",
    "mcp::handler::engine_neutral_handler_tests::server_health_exposes_next_core_io_summary",
    "mcp::handler::engine_neutral_handler_tests::capability_surfaces_expose_next_core_health_io_diagnostics",
    "mcp::handler::engine_neutral_handler_tests::selftest_run_uses_selected_terminal_engine",
    "mcp::handler::engine_neutral_handler_tests::screen_search_goto_scrolls_next_core_logical_viewport",
    "mcp::handler::engine_neutral_handler_tests::screen_scroll_goto_updates_next_core_logical_viewport",
    "mcp::handler::engine_neutral_handler_tests::screen_scrollback_text_resolves_active_next_core_session_without_pane_param",
    "mcp::handler::engine_neutral_handler_tests::session_destroy_uses_next_core_pane_id_path",
    "mcp::handler::engine_neutral_handler_tests::session_resize_uses_next_core_pane_id_path",
    "mcp::handler::engine_neutral_handler_tests::session_env_reads_next_core_launch_env_keys_without_values",
    "mcp::handler::engine_neutral_handler_tests::session_set_env_applies_next_core_future_launch_overlay_without_values",
    "mcp::handler::engine_neutral_handler_tests::activity_methods_expose_next_core_io_metrics",
    "mcp::handler::engine_neutral_handler_tests::core_status_history_cursor_methods_use_next_core_engine",
    "mcp::handler::engine_neutral_handler_tests::screen_detect_errors_uses_next_core_screen_snapshot",
    "mcp::handler::engine_neutral_handler_tests::capture_screen_text_snapshot_uses_terminal_engine",
    "mcp::handler::engine_neutral_handler_tests::capture_window_text_snapshot_uses_terminal_engine",
    "mcp::handler::engine_neutral_handler_tests::capture_scrollback_renders_next_core_styled_png",
    "mcp::handler::engine_neutral_handler_tests::orchestrate_methods_use_next_core_sessions_and_screen",
    "mcp::handler::engine_neutral_handler_tests::recording_status_and_trace_attach_use_next_core_engine",
    "mcp::handler::engine_neutral_handler_tests::active_recording_export_uses_next_core_engine",
    "mcp::handler::engine_neutral_handler_tests::inactive_scrollback_export_markdown_uses_next_core_screen_engine",
    "mcp::handler::engine_neutral_handler_tests::cockpit_inbox_uses_engine_session_snapshot",
    "mcp::handler::engine_neutral_handler_tests::review_diff_does_not_require_wezterm_mux_in_next_core_mode",
    "mcp::handler::engine_neutral_handler_tests::review_verify_and_merge_work_for_next_core_fleet_member",
    "mcp::handler::engine_neutral_handler_tests::fleet_lifecycle_uses_next_core_session_engine"
)

Push-Location $RepoRoot
try {
    $list = @(& cargo test -p unterm $TestFilter -- --list 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test list failed:`n$($list -join "`n")"
    }

    foreach ($test in $RequiredTests) {
        if (-not @($list | Where-Object { $_ -like "$test`: test*" })) {
            throw "missing required next-core MCP contract test: $test"
        }
    }

    if ($ListOnly) {
        Write-Host "next-core MCP contract tests present: $($RequiredTests.Count)"
        exit 0
    }

    $run = @(& cargo test -p unterm $TestFilter -- --test-threads=1 2>&1 | ForEach-Object { $_.ToString() })
    if ($LASTEXITCODE -ne 0) {
        throw "next-core MCP contract tests failed:`n$($run -join "`n")"
    }
    if (-not @($run | Where-Object { $_ -match "test result: ok\..*43 passed" })) {
        throw "next-core MCP contract test run did not report 43 passed tests"
    }

    Write-Host "next-core MCP contract tests ok: required=$($RequiredTests.Count) module_tests=43"
} finally {
    Pop-Location
}
