param(
    [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

$CiDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $CiDir "..")

$TestFilter = "screen_buffer_"
$ExpectedCount = 98
$RequiredTests = @(
    "next_core::tests::screen_buffer_tracks_wide_character_cells",
    "next_core::tests::screen_buffer_preserves_combining_marks_on_base_cells",
    "next_core::tests::screen_buffer_attaches_combining_marks_to_previous_wide_cell",
    "next_core::tests::screen_buffer_preserves_emoji_variation_selector_width",
    "next_core::tests::screen_buffer_keeps_zwj_emoji_sequence_in_one_wide_cell",
    "next_core::tests::screen_buffer_keeps_emoji_modifier_in_base_wide_cell",
    "next_core::tests::screen_buffer_keeps_regional_indicator_flag_in_one_wide_cell",
    "next_core::tests::screen_buffer_wraps_wide_cells_before_right_edge",
    "next_core::tests::screen_buffer_repeats_wide_character_with_rep"
)

Push-Location $RepoRoot
try {
    $ErrorActionPreference = "Continue"
    $list = @(& cargo test -p unterm-engine $TestFilter -- --list 2>&1 | ForEach-Object { $_.ToString() })
    $ErrorActionPreference = "Stop"
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test list failed:`n$($list -join "`n")"
    }

    foreach ($test in $RequiredTests) {
        if (-not @($list | Where-Object { $_ -like "$test`: test*" })) {
            throw "missing required next-core Unicode width test: $test"
        }
    }

    if ($ListOnly) {
        Write-Host "next-core Unicode width tests present: required=$($RequiredTests.Count)"
        exit 0
    }

    $ErrorActionPreference = "Continue"
    $run = @(& cargo test -p unterm-engine $TestFilter -- --test-threads=1 2>&1 | ForEach-Object { $_.ToString() })
    $ErrorActionPreference = "Stop"
    if ($LASTEXITCODE -ne 0) {
        throw "next-core Unicode width tests failed:`n$($run -join "`n")"
    }
    if (-not @($run | Where-Object { $_ -match "test result: ok\..*$ExpectedCount passed" })) {
        throw "next-core Unicode width test run did not report $ExpectedCount passed tests"
    }

    Write-Host "next-core Unicode width tests ok: required=$($RequiredTests.Count) module_tests=$ExpectedCount"
} finally {
    Pop-Location
}
