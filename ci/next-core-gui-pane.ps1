param(
    [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

$CiDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $CiDir "..")

# A GUI pane driven by next-core needs two things the render contract does not
# cover: a pane->session binding that never aliases (pane ids and next-core
# session ids are separate allocators that overlap numerically), and a key
# encoder that turns GUI key events into the normal-mode sequences next-core
# expects. Both are gated here.
$Suites = @(
    @{
        Name = "pane binding"
        Package = "unterm"
        Filter = "engine::pane_binding::tests::"
        ExpectedCount = 10
        RequiredTests = @(
            "engine::pane_binding::tests::unbound_pane_resolves_to_error_not_a_numeric_alias",
            "engine::pane_binding::tests::rebinding_a_pane_releases_its_previous_session",
            "engine::pane_binding::tests::rebinding_a_session_detaches_its_previous_pane",
            "engine::pane_binding::tests::unbinding_clears_both_directions",
            "engine::pane_binding::tests::retain_panes_returns_sessions_for_closed_panes",
            "engine::pane_binding::tests::sync_size_reports_only_real_geometry_changes",
            "engine::pane_binding::tests::sync_size_ignores_unbound_panes",
            "engine::pane_binding::tests::rebinding_resets_the_tracked_size"
        )
    },
    @{
        Name = "key encoding"
        Package = "unterm-engine"
        Filter = "next_core::key_encoding::tests::"
        ExpectedCount = 11
        RequiredTests = @(
            "next_core::key_encoding::tests::ctrl_characters_encode_as_control_bytes",
            "next_core::key_encoding::tests::alt_prefixes_escape",
            "next_core::key_encoding::tests::super_chords_produce_no_pty_input",
            "next_core::key_encoding::tests::arrows_encode_in_normal_mode_so_the_session_can_translate",
            "next_core::key_encoding::tests::modified_arrows_use_xterm_modifier_parameters",
            "next_core::key_encoding::tests::function_keys_split_between_ss3_and_tilde_forms",
            "next_core::key_encoding::tests::modifier_keys_alone_produce_no_input",
            "next_core::key_encoding::tests::control_keys_use_canonical_bytes"
        )
    },
    @{
        # The mux pane backed directly by a next-core session: this is what
        # removes the second shell running underneath each replaced pane.
        Name = "next-core mux pane"
        Package = "unterm"
        Filter = "engine::next_core_pane::tests::"
        ExpectedCount = 6
        RequiredTests = @(
            "engine::next_core_pane::tests::styled_line_becomes_a_line_of_the_requested_width",
            "engine::next_core_pane::tests::wide_cells_consume_their_trailing_column",
            "engine::next_core_pane::tests::a_line_wider_than_the_screen_is_not_truncated",
            "engine::next_core_pane::tests::hyperlinks_survive_the_conversion",
            "engine::next_core_pane::tests::pane_reads_a_live_next_core_session",
            "engine::next_core_pane::tests::get_lines_returns_real_session_output_at_the_reported_rows"
        )
    },
    @{
        Name = "mouse encoding"
        Package = "unterm-engine"
        Filter = "next_core::mouse_encoding::tests::"
        ExpectedCount = 12
        RequiredTests = @(
            "next_core::mouse_encoding::tests::tracking_off_reports_nothing",
            "next_core::mouse_encoding::tests::each_tracking_mode_reports_only_what_it_asked_for",
            "next_core::mouse_encoding::tests::legacy_press_biases_every_field_by_32",
            "next_core::mouse_encoding::tests::legacy_releases_lose_the_button_but_sgr_keeps_it",
            "next_core::mouse_encoding::tests::sgr_press_uses_decimal_one_based_coordinates",
            "next_core::mouse_encoding::tests::legacy_format_gives_up_past_its_coordinate_limit",
            "next_core::mouse_encoding::tests::modifiers_add_their_xterm_bits",
            "next_core::mouse_encoding::tests::motion_sets_the_motion_bit_and_free_motion_uses_button_three",
            "next_core::mouse_encoding::tests::sgr_takes_precedence_over_the_other_extensions"
        )
    },
    @{
        # The end-to-end proofs: encoded input reaches a real PTY, mouse
        # reports follow the session's negotiated modes, and wheel scrolling
        # steps the viewport. Unit tests alone would pass with an encoder
        # wired to the wrong writer.
        Name = "input path end to end"
        Package = "unterm-engine"
        Filter = "next_core::tests::encoded_keys_reach_a_real_shell_and_echo_back"
        ExpectedCount = 1
        RequiredTests = @(
            "next_core::tests::encoded_keys_reach_a_real_shell_and_echo_back"
        )
    },
    @{
        Name = "mouse report end to end"
        Package = "unterm-engine"
        Filter = "next_core::tests::mouse_reports_follow_the_session_modes"
        ExpectedCount = 1
        RequiredTests = @(
            "next_core::tests::mouse_reports_follow_the_session_modes"
        )
    },
    @{
        Name = "viewport scroll end to end"
        Package = "unterm-engine"
        Filter = "next_core::tests::relative_viewport_scroll_steps_and_resumes_following"
        ExpectedCount = 1
        RequiredTests = @(
            "next_core::tests::relative_viewport_scroll_steps_and_resumes_following"
        )
    }
)

Push-Location $RepoRoot
try {
    $totalRequired = 0
    foreach ($suite in $Suites) {
        $list = @(& cargo test -p $suite.Package $suite.Filter -- --list 2>&1 | ForEach-Object { $_.ToString() })
        if ($LASTEXITCODE -ne 0) {
            throw "cargo test list failed for $($suite.Name):`n$($list -join "`n")"
        }

        foreach ($test in $suite.RequiredTests) {
            if (-not @($list | Where-Object { $_ -like "$test`: test*" })) {
                throw "missing required next-core GUI pane test: $test"
            }
        }
        $totalRequired += $suite.RequiredTests.Count
    }

    if ($ListOnly) {
        Write-Host "next-core GUI pane tests present: $totalRequired"
        exit 0
    }

    foreach ($suite in $Suites) {
        $run = @(& cargo test -p $suite.Package $suite.Filter -- --test-threads=1 2>&1 | ForEach-Object { $_.ToString() })
        if ($LASTEXITCODE -ne 0) {
            throw "next-core GUI pane tests failed for $($suite.Name):`n$($run -join "`n")"
        }
        $expected = $suite.ExpectedCount
        if (-not @($run | Where-Object { $_ -match "test result: ok\..*$expected passed" })) {
            throw "next-core GUI pane suite '$($suite.Name)' did not report $expected passed tests:`n$($run -join "`n")"
        }
    }

    Write-Host "next-core GUI pane tests ok: required=$totalRequired suites=$($Suites.Count)"
} finally {
    Pop-Location
}
