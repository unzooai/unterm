param(
    [switch]$ListOnly
)

$ErrorActionPreference = "Stop"

$CiDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $CiDir "..")

$Suites = @(
    @{
        Name = "draw replacement"
        Filter = "termwindow::render::draw::tests::"
        ExpectedCount = 8
        RequiredTests = @(
            "termwindow::render::draw::tests::next_core_replace_requires_draw_ready_batch",
            "termwindow::render::draw::tests::next_core_replace_keeps_legacy_pane_for_empty_repeat_batch",
            "termwindow::render::draw::tests::next_core_replace_keeps_legacy_pane_when_prepared_frame_is_not_ready",
            "termwindow::render::draw::tests::next_core_replace_requires_matching_batch_diagnostics",
            "termwindow::render::draw::tests::next_core_replace_requires_cached_glyph_upload_for_text_frames"
        )
    },
    @{
        Name = "WebGPU glyph cache"
        Filter = "termwindow::webgpu::tests::"
        ExpectedCount = 12
        RequiredTests = @(
            "termwindow::webgpu::tests::next_core_cached_upload_diagnostics_include_layout_readiness",
            "termwindow::webgpu::tests::next_core_cached_upload_diagnostics_report_readiness_issues",
            "termwindow::webgpu::tests::next_core_cached_upload_layout_parity_reports_drift",
            "termwindow::webgpu::tests::next_core_cached_upload_reports_clean_layout_parity_for_repeat_upload",
            "termwindow::webgpu::tests::next_core_glyph_atlas_state_reuses_cached_placements",
            "termwindow::webgpu::tests::next_core_glyph_atlas_state_resets_when_cell_metrics_change",
            "termwindow::webgpu::tests::next_core_shaped_glyph_atlas_reuses_cached_shape_for_same_font_revision",
            "termwindow::webgpu::tests::next_core_glyph_texture_region_validation_reports_stats"
        )
    },
    @{
        Name = "engine render backend"
        Filter = "engine::render_backend::tests::"
        ExpectedCount = 28
        RequiredTests = @(
            "engine::render_backend::tests::prepared_frame_diagnostics_report_replace_readiness_issues",
            "engine::render_backend::tests::prepared_frame_plan_exposes_textured_glyph_layout_parity",
            "engine::render_backend::tests::prepared_frame_plan_layout_parity_reports_frame_level_drift",
            "engine::render_backend::tests::cached_textured_glyph_upload_uses_cache_placements",
            "engine::render_backend::tests::glyph_atlas_cache_allocates_and_reuses_placements",
            "engine::render_backend::tests::glyph_atlas_cache_wraps_rows_and_reports_overflow",
            "engine::render_backend::tests::glyph_atlas_texture_update_prepares_inserted_regions",
            "engine::render_backend::tests::textured_glyph_upload_maps_instances_to_clip_space_and_uvs",
            "engine::render_backend::tests::textured_glyph_pass_draws_complete_uploads"
        )
    }
)

Push-Location $RepoRoot
try {
    $totalRequired = 0
    foreach ($suite in $Suites) {
        $list = @(& cargo test -p unterm $suite.Filter -- --list 2>&1 | ForEach-Object { $_.ToString() })
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
        $run = @(& cargo test -p unterm $suite.Filter -- --test-threads=1 2>&1 | ForEach-Object { $_.ToString() })
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
