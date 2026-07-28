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
        ExpectedCount = 15
        RequiredTests = @(
            "engine::next_core_pane::tests::styled_line_becomes_a_line_of_the_requested_width",
            "engine::next_core_pane::tests::wide_cells_consume_their_trailing_column",
            "engine::next_core_pane::tests::a_line_wider_than_the_screen_is_not_truncated",
            "engine::next_core_pane::tests::hyperlinks_survive_the_conversion",
            "engine::next_core_pane::tests::pane_reads_a_live_next_core_session",
            "engine::next_core_pane::tests::get_lines_returns_real_session_output_at_the_reported_rows",
            "engine::next_core_pane::tests::pane_factory_flag_needs_an_explicit_opt_in",
            "engine::next_core_pane::tests::session_revision_advances_when_output_arrives",
            "engine::next_core_pane::tests::spawning_a_pane_creates_a_session_and_dropping_it_destroys_one",
            "engine::next_core_pane::tests::a_wrapped_row_marks_its_last_cell",
            "engine::next_core_pane::tests::cursor_shapes_round_trip_through_their_names",
            "engine::next_core_pane::tests::killing_a_pane_ends_its_session",
            "engine::next_core_pane::tests::unseen_output_tracks_revisions_since_focus_was_lost",
            "engine::next_core_pane::tests::layout_tree_matches_mux_tab_geometry",
            "engine::next_core_pane::tests::a_mux_tab_layout_can_be_adopted_by_next_core"
        )
    },
    @{
        # next-core rasterizing glyphs on FreeType directly, without
        # wezterm-font's terminal-specific font policy.
        Name = "font rasterization"
        Package = "unterm-engine"
        Filter = "next_core::font_raster::tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "next_core::font_raster::tests::rasterizes_a_glyph_with_partial_coverage",
            "next_core::font_raster::tests::a_space_has_no_ink_but_still_advances",
            "next_core::font_raster::tests::resizing_changes_the_rasterized_size",
            "next_core::font_raster::tests::rgba_puts_coverage_in_the_alpha_channel",
            "next_core::font_raster::tests::a_missing_font_file_is_an_error_not_a_panic"
        )
    },
    @{
        # Choosing which font file to rasterize, without wezterm-font's
        # fontconfig/CoreText/DirectWrite stack.
        Name = "font discovery"
        Package = "unterm-engine"
        Filter = "next_core::font_discovery::tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "next_core::font_discovery::tests::the_platform_has_font_directories",
            "next_core::font_discovery::tests::scanning_finds_a_usable_monospace_face",
            "next_core::font_discovery::tests::family_lookup_is_case_insensitive_and_prefers_regular",
            "next_core::font_discovery::tests::monospace_fallback_is_deterministic_and_skips_proportional_faces",
            "next_core::font_discovery::tests::font_extensions_are_recognized_case_insensitively"
        )
    },
    @{
        # Text -> positioned glyph ids, on HarfBuzz directly. The end-to-end
        # case also proves discovery, shaping, and rasterization compose.
        Name = "font shaping"
        Package = "unterm-engine"
        Filter = "next_core::font_shaper::tests::"
        ExpectedCount = 6
        RequiredTests = @(
            "next_core::font_shaper::tests::shapes_ascii_into_one_glyph_per_character",
            "next_core::font_shaper::tests::a_monospace_face_advances_every_glyph_equally",
            "next_core::font_shaper::tests::clusters_map_glyphs_back_to_their_bytes",
            "next_core::font_shaper::tests::shaping_follows_the_faces_pixel_size",
            "next_core::font_shaper::tests::empty_text_shapes_to_nothing",
            "next_core::font_shaper::tests::discovered_font_shapes_and_rasterizes_its_own_glyphs"
        )
    },
    @{
        # The split tree and the geometry it produces -- the foundation for
        # owning tab layout rather than borrowing mux's.
        Name = "pane layout"
        Package = "unterm-engine"
        Filter = "next_core::layout::tests::"
        ExpectedCount = 18
        RequiredTests = @(
            "next_core::layout::tests::a_horizontal_split_leaves_a_cell_for_the_divider",
            "next_core::layout::tests::a_vertical_split_divides_height_instead",
            "next_core::layout::tests::nested_splits_stay_inside_their_parent",
            "next_core::layout::tests::closing_a_pane_gives_its_space_to_its_sibling",
            "next_core::layout::tests::closing_an_inner_pane_promotes_the_right_subtree",
            "next_core::layout::tests::closing_the_last_pane_empties_the_layout",
            "next_core::layout::tests::closing_panes_one_by_one_ends_empty",
            "next_core::layout::tests::ratios_are_clamped_so_neither_side_disappears",
            "next_core::layout::tests::a_tab_too_small_to_split_still_gives_every_pane_a_cell",
            "next_core::layout::tests::resizing_a_split_moves_the_divider_and_survives_a_tab_resize",
            "next_core::layout::tests::every_pane_is_positioned_exactly_once",
            "next_core::layout::tests::a_tree_round_trips_through_its_own_rectangles",
            "next_core::layout::tests::rebuilding_recovers_the_pane_set",
            "next_core::layout::tests::a_layout_no_terminal_could_produce_is_refused",
            "next_core::layout::tests::an_odd_dimension_splits_the_way_mux_does"
        )
    },
    @{
        # Tab state around the layout tree: which panes exist, which one has
        # focus, and what closing one does to its tab.
        Name = "tab registry"
        Package = "unterm-engine"
        Filter = "next_core::tabs::tests::"
        ExpectedCount = 16
        RequiredTests = @(
            "next_core::tabs::tests::splitting_focuses_the_new_pane",
            "next_core::tabs::tests::closing_the_active_pane_moves_focus_to_a_survivor",
            "next_core::tabs::tests::closing_the_last_pane_closes_the_tab",
            "next_core::tabs::tests::closing_the_active_tab_focuses_another_one",
            "next_core::tabs::tests::a_pane_belongs_to_exactly_one_tab",
            "next_core::tabs::tests::tabs_are_independent",
            "next_core::tabs::tests::focus_follows_the_pane_across_tabs",
            "next_core::tabs::tests::positions_come_from_the_tabs_own_layout",
            "next_core::tabs::tests::tab_ids_are_not_reused_after_a_tab_closes",
            "next_core::tabs::tests::adopting_a_tab_mirrors_its_panes_and_focus",
            "next_core::tabs::tests::re_adopting_a_tab_drops_panes_that_went_away",
            "next_core::tabs::tests::adoption_refuses_what_it_cannot_represent",
            "next_core::tabs::tests::forgetting_a_tab_releases_its_panes"
        )
    },
    @{
        # A declarative config the engine reads instead of executes.
        Name = "config runtime"
        Package = "unterm-engine"
        Filter = "next_core::config::tests::"
        ExpectedCount = 20
        RequiredTests = @(
            "next_core::config::tests::a_typo_is_rejected_with_the_nearest_real_setting",
            "next_core::config::tests::an_unrelated_key_is_rejected_without_a_wild_guess",
            "next_core::config::tests::setting_a_key_twice_is_an_error_naming_both_lines",
            "next_core::config::tests::every_error_is_reported_not_just_the_first",
            "next_core::config::tests::a_hash_inside_a_string_is_not_a_comment",
            "next_core::config::tests::a_windows_path_does_not_need_doubled_backslashes",
            "next_core::config::tests::a_wrong_type_names_the_key_and_both_types",
            "next_core::config::tests::unterminated_constructs_are_errors_not_silent_truncation"
        )
    },
    @{
        # Existing Lua configs must survive the move -- and whatever cannot be
        # converted must be reported, never dropped.
        Name = "config migration"
        Package = "unterm-engine"
        Filter = "next_core::config_migrate::tests::"
        ExpectedCount = 16
        RequiredTests = @(
            "next_core::config_migrate::tests::converts_the_ordinary_assignments",
            "next_core::config_migrate::tests::a_nested_table_becomes_a_section",
            "next_core::config_migrate::tests::a_function_is_reported_rather_than_dropped",
            "next_core::config_migrate::tests::every_unconverted_line_carries_its_source",
            "next_core::config_migrate::tests::the_converted_output_is_checked_against_the_parser",
            "next_core::config_migrate::tests::a_realistic_config_converts_to_something_that_parses"
        )
    },
    @{
        # The state behind copy mode, without its UI: two points, a shape, and
        # the text that comes out.
        Name = "selection model"
        Package = "unterm-engine"
        Filter = "next_core::selection::tests::"
        ExpectedCount = 13
        RequiredTests = @(
            "next_core::selection::tests::dragging_backwards_keeps_the_anchor",
            "next_core::selection::tests::a_linear_selection_takes_partial_ends_and_full_middles",
            "next_core::selection::tests::a_block_selection_takes_the_same_columns_from_every_row",
            "next_core::selection::tests::columns_never_run_past_the_row",
            "next_core::selection::tests::extraction_trims_the_padding_a_terminal_adds",
            "next_core::selection::tests::extraction_keeps_leading_indentation",
            "next_core::selection::tests::a_soft_wrapped_row_joins_the_next_without_a_newline",
            "next_core::selection::tests::a_hard_break_between_rows_becomes_a_newline",
            "next_core::selection::tests::extraction_does_not_add_a_trailing_newline"
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
        Name = "scrollback erase end to end"
        Package = "unterm-engine"
        Filter = "next_core::tests::erase_scrollback_drops_history_and_optionally_the_viewport"
        ExpectedCount = 1
        RequiredTests = @(
            "next_core::tests::erase_scrollback_drops_history_and_optionally_the_viewport"
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
