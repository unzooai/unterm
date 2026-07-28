param(
    [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

$CiDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $CiDir "..")

$Suites = @(
    @{
        Name = "engine render backend"
        Package = "unterm-render"
        Filter = "backend::tests::"
        ExpectedCount = 33
        RequiredTests = @(
            "backend::tests::prepared_frame_diagnostics_report_replace_readiness_issues",
            "backend::tests::prepared_frame_plan_exposes_textured_glyph_layout_parity",
            "backend::tests::prepared_frame_plan_layout_parity_reports_frame_level_drift",
            "backend::tests::cached_textured_glyph_upload_uses_cache_placements",
            "backend::tests::glyph_atlas_cache_allocates_and_reuses_placements",
            "backend::tests::glyph_atlas_cache_wraps_rows_and_reports_overflow",
            "backend::tests::glyph_atlas_texture_update_prepares_inserted_regions",
            "backend::tests::textured_glyph_upload_maps_instances_to_clip_space_and_uvs",
            "backend::tests::textured_glyph_pass_draws_complete_uploads",
            "backend::tests::fullscreen_placement_maps_target_corners_to_clip_corners",
            "backend::tests::offset_placement_shifts_a_pane_into_its_own_corner",
            "backend::tests::buffer_plan_for_placement_offsets_every_vertex",
            "backend::tests::buffer_plan_for_viewport_still_means_a_fullscreen_placement"
        )
    }
)

Push-Location $RepoRoot
try {
    $totalRequired = 0
    foreach ($suite in $Suites) {
        $list = @(& cargo test -p $($suite.Package) $suite.Filter -- --list 2>&1 | ForEach-Object { $_.ToString() })
        if ($LASTEXITCODE -ne 0) {
            throw "cargo test list failed for $($suite.Name):`n$($list -join "`n")"
        }

        foreach ($test in $suite.RequiredTests) {
            if (-not @($list | Where-Object { $_ -like "$test`: test*" })) {
                throw "missing required next-core WebGPU render test: $test"
            }
        }
        $totalRequired += $suite.RequiredTests.Count
    }

    if ($ListOnly) {
        Write-Host "next-core WebGPU render tests present: required=$totalRequired suites=$($Suites.Count)"
        exit 0
    }

    foreach ($suite in $Suites) {
        $run = @(& cargo test -p $($suite.Package) $suite.Filter -- --test-threads=1 2>&1 | ForEach-Object { $_.ToString() })
        if ($LASTEXITCODE -ne 0) {
            throw "next-core WebGPU render tests failed for $($suite.Name):`n$($run -join "`n")"
        }
        if (-not @($run | Where-Object { $_ -match "test result: ok\..*$($suite.ExpectedCount) passed" })) {
            throw "next-core WebGPU render test run for $($suite.Name) did not report $($suite.ExpectedCount) passed tests"
        }
    }

    $totalTests = ($Suites | ForEach-Object { $_.ExpectedCount } | Measure-Object -Sum).Sum
    Write-Host "next-core WebGPU render tests ok: required=$totalRequired module_tests=$totalTests"
} finally {
    Pop-Location
}
