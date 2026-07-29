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
        ExpectedCount = 21
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
            "next_core::tabs::tests::forgetting_a_tab_releases_its_panes",
            "next_core::tabs::tests::zooming_gives_one_pane_the_whole_tab",
            "next_core::tabs::tests::unzooming_restores_the_arrangement_exactly",
            "next_core::tabs::tests::zooming_also_focuses_the_pane"
        )
    },
    @{
        # A declarative config the engine reads instead of executes.
        Name = "config runtime"
        Package = "unterm-engine"
        Filter = "next_core::config::tests::"
        ExpectedCount = 28
        RequiredTests = @(
            "next_core::config::tests::a_typo_is_rejected_with_the_nearest_real_setting",
            "next_core::config::tests::an_unrelated_key_is_rejected_without_a_wild_guess",
            "next_core::config::tests::setting_a_key_twice_is_an_error_naming_both_lines",
            "next_core::config::tests::every_error_is_reported_not_just_the_first",
            "next_core::config::tests::a_hash_inside_a_string_is_not_a_comment",
            "next_core::config::tests::a_windows_path_does_not_need_doubled_backslashes",
            "next_core::config::tests::a_wrong_type_names_the_key_and_both_types",
            "next_core::config::tests::unterminated_constructs_are_errors_not_silent_truncation",
            "next_core::config::tests::a_platform_section_overrides_the_base_value",
            "next_core::config::tests::the_named_platform_beats_the_catch_all",
            "next_core::config::tests::a_typo_inside_a_platform_section_is_still_caught",
            "next_core::config::tests::the_catch_all_skips_a_platform_that_has_its_own_section",
            "next_core::config::tests::a_list_may_run_across_several_lines",
            "next_core::config::tests::an_unclosed_section_header_does_not_swallow_the_file"
        )
    },
    @{
        # Existing Lua configs must survive the move -- and whatever cannot be
        # converted must be reported, never dropped.
        Name = "config migration"
        Package = "unterm-engine"
        Filter = "next_core::config_migrate::tests::"
        ExpectedCount = 21
        RequiredTests = @(
            "next_core::config_migrate::tests::converts_the_ordinary_assignments",
            "next_core::config_migrate::tests::a_nested_table_becomes_a_section",
            "next_core::config_migrate::tests::a_function_is_reported_rather_than_dropped",
            "next_core::config_migrate::tests::every_unconverted_line_carries_its_source",
            "next_core::config_migrate::tests::the_converted_output_is_checked_against_the_parser",
            "next_core::config_migrate::tests::a_realistic_config_converts_to_something_that_parses",
            "next_core::config_migrate::tests::a_target_triple_branch_becomes_platform_sections",
            "next_core::config_migrate::tests::a_value_chosen_by_a_probe_is_never_converted_to_one_branch",
            "next_core::config_migrate::tests::a_function_call_that_closes_on_the_same_line_does_not_swallow_the_file",
            "next_core::config_migrate::tests::an_unrecognised_platform_branch_is_reported_not_filed_wrongly"
        )
    },
    @{
        # What settings exist, so a config can be judged before it is used.
        Name = "config schema"
        Package = "unterm-engine"
        Filter = "next_core::config_schema::tests::"
        ExpectedCount = 10
        RequiredTests = @(
            "next_core::config_schema::tests::an_unknown_setting_is_rejected",
            "next_core::config_schema::tests::a_typo_outside_that_section_is_still_caught",
            "next_core::config_schema::tests::environment_variables_are_the_one_place_names_are_invented",
            "next_core::config_schema::tests::a_platform_section_is_checked_after_it_is_resolved",
            "next_core::config_schema::tests::every_problem_is_reported_at_once_and_in_file_order",
            "next_core::config_schema::tests::a_value_outside_its_range_is_rejected",
            "next_core::config_schema::tests::the_config_this_project_ships_is_valid_on_every_platform"
        )
    },
    @{
        # Tab titles from a template, replacing the one callback every config
        # reached for code to write.
        Name = "tab titles"
        Package = "unterm-engine"
        Filter = "next_core::tab_title::tests::"
        ExpectedCount = 15
        RequiredTests = @(
            "next_core::tab_title::tests::an_empty_title_falls_back_to_the_running_program",
            "next_core::tab_title::tests::a_placeholder_title_is_treated_as_no_title",
            "next_core::tab_title::tests::a_windows_program_loses_its_extension",
            "next_core::tab_title::tests::both_path_separators_are_understood_on_every_platform",
            "next_core::tab_title::tests::nothing_at_all_still_names_the_tab",
            "next_core::tab_title::tests::capitalizing_leaves_scripts_without_case_alone",
            "next_core::tab_title::tests::an_unknown_placeholder_is_reported_rather_than_rendered_literally",
            "next_core::tab_title::tests::every_executable_suffix_windows_runs_is_dropped",
            "next_core::tab_title::tests::the_name_is_available_without_the_template"
        )
    },
    @{
        # The two colour adjustments a theme-following config needs.
        Name = "colour derivation"
        Package = "unterm-engine"
        Filter = "next_core::color::tests::"
        ExpectedCount = 11
        RequiredTests = @(
            "next_core::color::tests::the_short_form_doubles_each_digit",
            "next_core::color::tests::lightening_a_dark_colour_makes_a_visible_difference",
            "next_core::color::tests::a_bright_colour_lightens_less_than_a_dark_one",
            "next_core::color::tests::an_out_of_range_amount_is_clamped_rather_than_wrapping",
            "next_core::color::tests::rejects_what_is_not_a_colour"
        )
    },
    @{
        # The renderer that lets next-core draw its own pixels: the join that
        # did not exist, and the reason its font modules had no caller.
        Name = "glyph atlas"
        Package = "unterm-render"
        Filter = "atlas::tests::"
        ExpectedCount = 8
        RequiredTests = @(
            "atlas::tests::the_same_glyph_is_placed_once",
            "atlas::tests::an_atlas_that_runs_out_of_height_grows_instead_of_dropping_a_glyph",
            "atlas::tests::growing_keeps_earlier_glyphs_where_they_were",
            "atlas::tests::neighbours_do_not_touch",
            "atlas::tests::a_space_gets_a_slot_with_no_pixels"
        )
    },
    @{
        # Styled cells to vertices. Pure, so a rendering bug is an assertion
        # failure rather than a wrong-looking window.
        Name = "render quads"
        Package = "unterm-render"
        Filter = "quads::tests::"
        ExpectedCount = 10
        RequiredTests = @(
            "quads::tests::a_glyph_sits_on_the_baseline_by_its_bearings",
            "quads::tests::texture_coordinates_follow_the_atlas_when_it_grows",
            "quads::tests::a_wide_cell_covers_both_its_columns",
            "quads::tests::inverse_swaps_the_two_colours",
            "quads::tests::a_cell_with_the_frame_background_draws_no_background_quad",
            "quads::tests::a_hidden_cell_keeps_its_background_and_loses_its_glyph"
        )
    },
    @{
        # The whole font path with a real face, which is the first code
        # anywhere to use next-core's font modules.
        Name = "font path"
        Package = "unterm-render"
        Filter = "text::tests::"
        ExpectedCount = 4
        RequiredTests = @(
            "text::tests::a_real_font_fills_a_real_atlas",
            "text::tests::a_run_advances_left_to_right",
            "text::tests::placing_the_same_run_twice_reuses_the_atlas"
        )
    },
    @{
        # Drawn on real hardware and read back as pixels. Every bug this layer
        # produced before compiled, submitted, and showed something else.
        Name = "offscreen render"
        Package = "unterm-render"
        Filter = "offscreen_"
        ExpectedCount = 5
        RequiredTests = @(
            "offscreen_a_background_quad_lands_where_it_was_put",
            "offscreen_the_top_left_of_a_quad_is_the_top_left_of_the_image",
            "offscreen_a_glyph_is_tinted_by_its_colour_and_shaped_by_the_atlas",
            "offscreen_a_glyph_draws_over_its_own_background",
            "offscreen_an_empty_frame_is_the_clear_colour"
        )
    },
    @{
        # The frame the independent front end builds: font metrics to grid,
        # screen snapshot to quads. No window involved, so it is assertable.
        Name = "app frame"
        Package = "unterm-app"
        Filter = "terminal::tests::"
        ExpectedCount = 16
        RequiredTests = @(
            "terminal::tests::a_window_sized_to_exactly_n_cells_gets_n",
            "terminal::tests::a_window_too_small_for_one_cell_still_asks_for_one",
            "terminal::tests::every_glyph_is_rasterized_before_any_quad_is_built",
            "terminal::tests::text_becomes_glyph_quads",
            "terminal::tests::a_config_without_colours_still_gives_readable_ones",
            "terminal::tests::a_character_the_primary_face_lacks_still_gets_ink",
            "terminal::tests::the_cursor_is_drawn_where_the_screen_says",
            "terminal::tests::a_hidden_cursor_is_not_drawn",
            "terminal::tests::a_block_cursor_leaves_its_character_readable",
            "terminal::tests::a_bar_cursor_does_not_cover_its_cell",
            "terminal::tests::a_cursor_off_the_screen_is_not_drawn"
        )
    },
    @{
        # Which shell the window starts. Falling back to %COMSPEC% is not what
        # a config naming pwsh meant.
        Name = "app shell"
        Package = "unterm-app"
        Filter = "window::tests::"
        ExpectedCount = 7
        RequiredTests = @(
            "window::tests::the_configured_shell_is_used",
            "window::tests::a_config_naming_no_shell_leaves_the_choice_to_the_engine",
            "window::tests::a_shell_can_carry_its_arguments",
            "window::tests::cycling_forward_from_the_last_tab_wraps_to_the_first",
            "window::tests::cycling_back_from_the_first_tab_wraps_to_the_last",
            "window::tests::a_tab_that_is_no_longer_there_cycles_from_the_start",
            "window::tests::cycling_with_no_tabs_answers_rather_than_dividing_by_zero"
        )
    },
    @{
        # Which keys a search takes decides which keys the shell never sees,
        # and taking one too many breaks a program running underneath it.
        Name = "app search"
        Package = "unterm-app"
        Filter = "search::tests::"
        ExpectedCount = 13
        RequiredTests = @(
            "search::tests::stepping_past_the_last_match_wraps_to_the_first",
            "search::tests::stepping_with_no_matches_answers_rather_than_dividing_by_zero",
            "search::tests::narrowing_the_pattern_keeps_the_match_the_user_was_looking_at",
            "search::tests::a_control_chord_is_not_text_to_search_for",
            "search::tests::a_key_the_search_has_no_use_for_reaches_the_shell"
        )
    },
    @{
        # OSC 52 and focus reporting: the two things a program tells the
        # terminal that the new front end was not listening for.
        Name = "engine osc and focus"
        Package = "unterm-engine"
        Filter = "next_core::osc_params::clipboard_tests::"
        ExpectedCount = 7
        RequiredTests = @(
            "next_core::osc_params::clipboard_tests::a_program_can_put_text_on_the_clipboard",
            "next_core::osc_params::clipboard_tests::reading_the_clipboard_is_refused",
            "next_core::osc_params::clipboard_tests::text_that_is_not_utf8_is_ignored"
        )
    },
    @{
        Name = "engine focus and clipboard modes"
        Package = "unterm-engine"
        Filter = "next_core::screen_state::focus_and_clipboard_tests::"
        ExpectedCount = 3
        RequiredTests = @(
            "next_core::screen_state::focus_and_clipboard_tests::a_program_asking_for_focus_events_is_reported",
            "next_core::screen_state::focus_and_clipboard_tests::a_clipboard_write_reaches_the_front_end",
            "next_core::screen_state::focus_and_clipboard_tests::a_clipboard_read_request_leaves_nothing_to_honour"
        )
    },
    @{
        # A picture of the terminal. The kernel has no window -- it draws into
        # whatever surface a front end gives it -- so `capture.window` has to
        # reach the front end, which is also what `selftest.run` checks.
        Name = "capture and focus reach the window's owner"
        Package = "unterm-engine"
        Filter = "host_capture_tests::"
        ExpectedCount = 1
        RequiredTests = @(
            "host_capture_tests::capture_reaches_the_front_end_that_owns_the_window"
        )
    },
    @{
        # Finding and photographing a window, and not mistaking a compositor's
        # blank answer for a picture of one.
        Name = "window capture"
        Package = "unterm-services"
        Filter = "window_capture::tests::"
        ExpectedCount = 9
        RequiredTests = @(
            "window_capture::tests::a_capture_has_to_say_which_window",
            "window_capture::tests::a_blank_bitmap_is_not_mistaken_for_a_picture",
            "window_capture::tests::a_process_with_no_window_is_told_so_rather_than_handed_one",
            "window_capture::tests::a_region_is_the_same_whichever_way_it_was_dragged",
            "window_capture::tests::a_click_is_not_a_region",
            "window_capture::tests::a_region_on_a_monitor_left_of_the_first_keeps_its_position"
        )
    },
    @{
        # The six colour schemes the product ships, recovered whole. Every one
        # was chosen against a terminal; a theme that is nearly the old one is
        # one people notice is wrong without being able to say why.
        Name = "themes"
        Package = "unterm-app"
        Filter = "theme::tests::"
        ExpectedCount = 7
        RequiredTests = @(
            "theme::tests::the_six_bundled_themes_are_all_here",
            "theme::tests::text_is_readable_on_its_own_background",
            "theme::tests::the_cursor_stands_out_from_the_background",
            "theme::tests::no_background_is_pure_black",
            "theme::tests::every_theme_has_sixteen_usable_colours"
        )
    },
    @{
        # A theme has to reach the colours programs actually ask for.
        Name = "themed palette"
        Package = "unterm-render"
        Filter = "quads::themed_palette_tests::"
        ExpectedCount = 4
        RequiredTests = @(
            "quads::themed_palette_tests::a_theme_decides_what_the_first_sixteen_colours_are",
            "quads::themed_palette_tests::the_colour_cube_is_not_themed",
            "quads::themed_palette_tests::truecolor_is_never_themed"
        )
    },
    @{
        # The frame's own colours, ported from the front end that was deleted:
        # the tones were chosen against linear-light mixing and checked for
        # contrast, and that work is not worth guessing at twice.
        Name = "chrome colours"
        Package = "unterm-app"
        Filter = "chrome::tests::"
        ExpectedCount = 9
        RequiredTests = @(
            "chrome::tests::every_bundled_theme_keeps_its_text_and_focus_visible",
            "chrome::tests::chrome_text_meets_aa_in_light_and_dark",
            "chrome::tests::mixing_is_done_in_linear_light",
            "chrome::tests::the_frame_stays_close_to_the_terminals_background"
        )
    },
    @{
        # Font size in points at the display's scale. Treating the point size
        # as pixels drew every glyph at a fraction of its size.
        Name = "font size in points"
        Package = "unterm-app"
        Filter = "terminal::dpi_tests::"
        ExpectedCount = 3
        RequiredTests = @(
            "terminal::dpi_tests::points_become_more_pixels_than_points",
            "terminal::dpi_tests::a_scaled_display_gets_proportionally_more_pixels",
            "terminal::dpi_tests::nothing_rounds_away_to_no_font_at_all"
        )
    },
    @{
        # The gap between the window's edge and the text. Its absence is the
        # difference between a terminal and a wall of text in a corner.
        Name = "window padding"
        Package = "unterm-app"
        Filter = "topbar::padding_tests::"
        ExpectedCount = 3
        RequiredTests = @(
            "topbar::padding_tests::there_is_a_gap_before_the_first_column",
            "topbar::padding_tests::the_gap_comes_out_of_the_grid_rather_than_the_window",
            "topbar::padding_tests::a_tiny_window_still_has_a_cell_of_terminal"
        )
    },
    @{
        # A borderless window has no system resize handles, so it grows its
        # own. Without them the window is stuck at the size it opened at.
        Name = "window resize edges"
        Package = "unterm-app"
        Filter = "topbar::resize_tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "topbar::resize_tests::each_side_reports_its_own_direction",
            "topbar::resize_tests::each_corner_resizes_in_two_directions",
            "topbar::resize_tests::the_edge_is_wide_enough_to_aim_at",
            "topbar::resize_tests::the_edge_does_not_reach_the_window_buttons"
        )
    },
    @{
        # Where each piece of the top bar goes, and what a click there hits.
        Name = "top bar layout"
        Package = "unterm-app"
        Filter = "topbar::tests::"
        ExpectedCount = 13
        RequiredTests = @(
            "topbar::tests::the_window_buttons_are_never_dropped",
            "topbar::tests::the_close_button_is_the_rightmost_thing",
            "topbar::tests::no_two_pieces_share_a_column",
            "topbar::tests::a_click_finds_what_was_drawn_there",
            "topbar::tests::the_empty_parts_of_the_bar_drag_the_window",
            "topbar::tests::a_tiny_window_still_has_a_close_button"
        )
    },
    @{
        # Minimise, maximise and close, drawn rather than typeset.
        Name = "window buttons"
        Package = "unterm-app"
        Filter = "window_buttons::tests::"
        ExpectedCount = 9
        RequiredTests = @(
            "window_buttons::tests::every_button_draws_something",
            "window_buttons::tests::an_icon_stays_inside_its_button",
            "window_buttons::tests::only_the_close_button_turns_red",
            "window_buttons::tests::the_close_cross_is_white_on_its_red"
        )
    },
    @{
        # Thin lines from rectangles: a gap in a close button reads as a
        # rendering fault, which is what it would be.
        Name = "strokes"
        Package = "unterm-render"
        Filter = "strokes::tests::"
        ExpectedCount = 10
        RequiredTests = @(
            "strokes::tests::a_diagonal_has_no_gaps_along_its_length",
            "strokes::tests::a_shallow_diagonal_has_no_gaps_either",
            "strokes::tests::a_diagonal_reaches_both_of_its_ends",
            "strokes::tests::a_stroke_is_never_thinner_than_a_pixel"
        )
    },
    @{
        # Nine languages, and the front end's own chrome among them. A label
        # in Chinese is mostly double-width characters, and drawing each one
        # cell wide puts the next on top of the last.
        Name = "front end languages"
        Package = "unterm-services"
        Filter = "i18n::catalogue_tests::"
        ExpectedCount = 2
        RequiredTests = @(
            "i18n::catalogue_tests::the_interfaces_own_strings_exist_in_every_language",
            "i18n::catalogue_tests::a_counted_string_keeps_its_placeholder_in_every_language"
        )
    },
    @{
        Name = "character widths in the chrome"
        Package = "unterm-app"
        Filter = "terminal::width_tests::"
        ExpectedCount = 4
        RequiredTests = @(
            "terminal::width_tests::a_wide_character_takes_two_cells",
            "terminal::width_tests::nothing_is_narrower_than_a_cell",
            "terminal::width_tests::a_label_after_a_wide_character_is_not_drawn_on_top_of_it"
        )
    },
    @{
        # A batch of prompts fed to an agent one at a time. The two rules that
        # make it safe to leave running: nothing goes in while the pane is
        # busy, and nothing goes in twice.
        Name = "prompt queue"
        Package = "unterm-app"
        Filter = "composer::tests::"
        ExpectedCount = 13
        RequiredTests = @(
            "composer::tests::nothing_is_sent_while_the_pane_is_busy",
            "composer::tests::the_next_prompt_waits_for_the_pane_to_actually_start",
            "composer::tests::a_blank_line_is_not_a_prompt",
            "composer::tests::a_destructive_question_is_never_answered_automatically",
            "composer::tests::clearing_lets_the_next_queue_start_immediately"
        )
    },
    @{
        # What git says about where a pane is. Read-only: the shell is right
        # there for the rest, and what is missing when you look at a terminal
        # is knowing which branch you are on, not the ability to run `git add`.
        Name = "git status"
        Package = "unterm-app"
        Filter = "git::tests::"
        ExpectedCount = 15
        RequiredTests = @(
            "git::tests::a_missing_git_does_not_claim_the_folder_is_untracked",
            "git::tests::a_detached_head_says_so_rather_than_naming_a_branch",
            "git::tests::ahead_alone_does_not_invent_a_behind",
            "git::tests::a_rename_shows_where_the_file_is_now",
            "git::tests::a_quoted_path_loses_its_quotes",
            "git::tests::a_line_too_short_to_be_an_entry_is_ignored",
            "git::tests::empty_output_is_a_status_rather_than_a_panic"
        )
    },
    @{
        # Which agent wants you, readable from the tab bar. Four agents
        # running and no marker means visiting each pane to find out.
        Name = "agent badges"
        Package = "unterm-app"
        Filter = "badge"
        ExpectedCount = 9
        RequiredTests = @(
            "cockpit::badge_tests::every_state_that_means_something_has_a_badge",
            "cockpit::badge_tests::an_idle_pane_is_not_marked",
            "cockpit::badge_tests::the_three_badges_are_three_different_colours",
            "window::tab_badge_tests::a_badge_stays_within_its_own_tab",
            "window::tab_badge_tests::a_badge_sits_after_the_number_it_belongs_to",
            "window::tab_badge_tests::the_badges_are_told_apart_by_colour"
        )
    },
    @{
        # Picking a folder without a folder dialog. A native picker differs on
        # every platform, and parity here is a correctness property -- so the
        # picker is the palette.
        Name = "directory picker"
        Package = "unterm-app"
        Filter = "directory::tests::"
        ExpectedCount = 8
        RequiredTests = @(
            "directory::tests::the_folders_are_listed_and_the_files_are_not",
            "directory::tests::a_folder_row_descends_into_it",
            "directory::tests::a_picker_opened_for_a_new_tab_ends_in_a_new_tab",
            "directory::tests::an_unreadable_directory_still_offers_a_way_out",
            "directory::tests::a_root_has_no_parent_row"
        )
    },
    @{
        # Right-click is a gesture, not a menu.
        Name = "right click"
        Package = "unterm-app"
        Filter = "mouse::right_click_tests::"
        ExpectedCount = 2
        RequiredTests = @(
            "mouse::right_click_tests::a_selection_is_copied_and_let_go_of",
            "mouse::right_click_tests::with_nothing_selected_it_pastes"
        )
    },
    @{
        # The bar along the bottom. Its row comes out of the terminal area --
        # drawn over the grid it would hide the last row while the shell still
        # believed it was there.
        Name = "status bar"
        Package = "unterm-app"
        Filter = "statusbar::tests::"
        ExpectedCount = 20
        RequiredTests = @(
            "statusbar::tests::the_chips_survive_a_laptop_sized_window",
            "statusbar::tests::a_waiting_agent_is_counted_on_the_bar",
            "statusbar::tests::a_notice_replaces_the_directory_while_it_is_up",
            "statusbar::tests::a_notice_does_not_hide_the_chips",
            "statusbar::tests::the_menu_button_is_the_first_thing_on_the_bar",
            "statusbar::tests::a_click_on_the_button_is_recognised_and_one_beside_it_is_not",
            "statusbar::tests::a_long_path_keeps_its_end",
            "statusbar::tests::no_segment_runs_off_the_end",
            "statusbar::tests::the_left_text_stops_before_the_chips",
            "statusbar::tests::a_narrow_pane_keeps_the_directory_and_drops_the_chips",
            "statusbar::tests::a_pending_confirmation_leads_the_chips"
        )
    },
    @{
        # The front end's own text -- labels, banners, bars -- was looked up by
        # code point while it was stored by glyph index, so every one of them
        # drew whichever glyph carried that number.
        Name = "front end text"
        Package = "unterm-app"
        Filter = "terminal::furniture_tests::"
        ExpectedCount = 3
        RequiredTests = @(
            "terminal::furniture_tests::a_labels_glyphs_are_found_where_they_were_put",
            "terminal::furniture_tests::a_label_draws_one_glyph_per_visible_character",
            "terminal::furniture_tests::a_labels_characters_advance_by_one_cell_each"
        )
    },
    @{
        # The keys a terminal is expected to have. Font size, tab numbers,
        # full screen and pane navigation were all missing, which is what
        # "the shortcuts are not all implemented" meant.
        Name = "key bindings"
        Package = "unterm-app"
        Filter = "keys::"
        ExpectedCount = 13
        RequiredTests = @(
            "keys::added_binding_tests::the_font_size_keys_exist",
            "keys::added_binding_tests::ctrl_and_a_digit_goes_to_that_tab",
            "keys::added_binding_tests::alt_arrows_move_between_panes_and_alt_letters_do_not",
            "keys::added_binding_tests::an_unmodified_arrow_still_belongs_to_the_shell",
            "keys::added_binding_tests::f11_goes_full_screen_and_the_other_function_keys_are_the_programs",
            "keys::added_binding_tests::a_new_window_and_a_pane_of_ones_own_are_both_reachable",
            "keys::added_binding_tests::every_bound_action_is_named_distinctly",
            "keys::tests::plain_ctrl_c_stays_the_programs_interrupt",
            "keys::tests::plain_tab_still_completes_in_the_shell"
        )
    },
    @{
        # Where a direction key goes, and which tab a number key means.
        Name = "pane and tab navigation"
        Package = "unterm-app"
        Filter = "direction_tests::"
        ExpectedCount = 6
        RequiredTests = @(
            "panes::direction_tests::a_split_moves_both_ways",
            "panes::direction_tests::there_is_no_pane_past_the_last_one",
            "panes::direction_tests::the_pane_beside_this_one_wins_over_a_nearer_one_off_to_a_side",
            "panes::direction_tests::a_single_pane_has_no_neighbours"
        )
    },
    @{
        Name = "tab number keys"
        Package = "unterm-app"
        Filter = "topbar::number_key_tests::"
        ExpectedCount = 4
        RequiredTests = @(
            "topbar::number_key_tests::nine_is_the_last_tab_not_the_ninth",
            "topbar::number_key_tests::a_number_past_the_last_tab_does_nothing",
            "topbar::number_key_tests::there_is_no_tab_when_there_are_no_tabs"
        )
    },
    @{
        # Lines, blocks and separators drawn rather than looked up. A font's
        # own box-drawing glyphs are laid out for the font's metrics, so a
        # table built from them shows a hairline gap at every join.
        Name = "box glyphs"
        Package = "unterm-render"
        Filter = "box_glyphs::"
        ExpectedCount = 24
        RequiredTests = @(
            "box_glyphs::tests::every_drawn_character_produces_something",
            "box_glyphs::tests::a_line_and_a_corner_meet_at_the_same_place",
            "box_glyphs::junction_tests::the_whole_double_block_is_ours",
            "box_glyphs::junction_tests::the_double_cross_stays_open_in_the_middle",
            "box_glyphs::junction_tests::a_double_corner_does_not_overshoot",
            "box_glyphs::junction_tests::a_single_line_through_a_double_one_is_not_broken"
        )
    },
    @{
        # The drawn glyphs have to reach the frame, and a font that ships its
        # own box-drawing characters must not get the column first.
        Name = "drawn cells reach the frame"
        Package = "unterm-render"
        Filter = "quads::drawn_glyph_tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "quads::drawn_glyph_tests::a_box_character_is_drawn_rather_than_looked_up",
            "quads::drawn_glyph_tests::two_horizontals_side_by_side_leave_no_gap",
            "quads::drawn_glyph_tests::an_ordinary_character_still_comes_from_the_font"
        )
    },
    @{
        Name = "shaping leaves drawn cells alone"
        Package = "unterm-app"
        Filter = "shape::drawn_cell_tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "shape::drawn_cell_tests::a_character_the_renderer_draws_is_left_out_of_every_run",
            "shape::drawn_cell_tests::a_powerline_separator_is_skipped_too",
            "shape::drawn_cell_tests::an_undrawn_neighbour_is_still_shaped"
        )
    },
    @{
        # The colours programs actually send. Palette indices resolved to the
        # frame's foreground, so `ls --color`, git diffs and every coloured
        # prompt came out the same shade as ordinary text; only truecolor
        # worked, and truecolor is what programs use least.
        Name = "palette colours"
        Package = "unterm-render"
        Filter = "quads::palette_tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "quads::palette_tests::a_palette_colour_is_resolved_rather_than_dropped",
            "quads::palette_tests::the_bright_half_of_the_palette_differs_from_the_dim_half",
            "quads::palette_tests::the_256_colour_cube_resolves_too",
            "quads::palette_tests::truecolor_still_arrives_exactly"
        )
    },
    @{
        # Ctrl+C. On a pty the byte is the whole story; Windows has no line
        # discipline, so a running command needs more -- and an editor
        # reading that same byte as a keystroke needs to be left alone.
        Name = "interrupt"
        Package = "unterm-services"
        Filter = "interrupt::tests::"
        ExpectedCount = 10
        RequiredTests = @(
            "interrupt::tests::a_process_with_its_own_console_does_not_give_it_up",
            "interrupt::tests::the_console_mode_probe_leaves_our_console_alone_too",
            "interrupt::tests::a_shell_at_its_prompt_is_never_stopped",
            "interrupt::tests::a_program_reading_keys_is_left_alone",
            "interrupt::tests::the_parent_process_is_never_interrupted_by_accident",
            "interrupt::tests::a_process_that_is_gone_is_reported_rather_than_silently_ignored",
            "interrupt::tests::raising_an_interrupt_repeatedly_does_not_end_us"
        )
    },
    @{
        # What a key sends the shell. Modifiers were dropped entirely, so
        # Ctrl+C typed a `c` and every readline binding was unreachable.
        Name = "app key encoding"
        Package = "unterm-app"
        Filter = "window::encode_tests::"
        ExpectedCount = 14
        RequiredTests = @(
            "window::encode_tests::ctrl_letter_sends_its_control_byte",
            "window::encode_tests::the_arrows_carry_their_modifiers",
            "window::encode_tests::the_function_keys_exist_at_all",
            "window::encode_tests::backspace_sends_delete_as_readline_expects",
            "window::encode_tests::a_modifier_key_on_its_own_sends_nothing"
        )
    },
    @{
        # The inbox's ordering is the whole feature: a list in pane order is
        # a list of panes, not an inbox.
        Name = "app agent inbox"
        Package = "unterm-app"
        Filter = "cockpit::tests::"
        ExpectedCount = 9
        RequiredTests = @(
            "cockpit::tests::waiting_panes_come_first",
            "cockpit::tests::the_longest_wait_is_at_the_top",
            "cockpit::tests::done_outranks_working_but_not_waiting",
            "cockpit::tests::only_waiting_and_done_want_the_person"
        )
    },
    @{
        # Selecting without a mouse, and grabbing what is already on screen.
        # Both are modes: a stray keystroke reaching the shell behind one is
        # the worst thing a mode can do.
        Name = "app copy mode"
        Package = "unterm-app"
        Filter = "copy_mode::tests::"
        ExpectedCount = 13
        RequiredTests = @(
            "copy_mode::tests::the_cursor_stops_at_the_edges",
            "copy_mode::tests::moving_to_a_shorter_line_pulls_the_column_in",
            "copy_mode::tests::a_selection_is_ordered_however_it_was_made",
            "copy_mode::tests::ordinary_words_are_not_labelled",
            "copy_mode::tests::labels_never_run_out"
        )
    },
    @{
        # A palette that finds the wrong row is worse than no palette: the
        # order it puts matches in is the whole feature.
        Name = "app command palette"
        Package = "unterm-app"
        Filter = "palette::tests::"
        ExpectedCount = 12
        RequiredTests = @(
            "palette::tests::initials_find_the_command_they_stand_for",
            "palette::tests::consecutive_letters_beat_scattered_ones",
            "palette::tests::an_exact_command_beats_one_that_merely_contains_it",
            "palette::tests::the_palette_takes_the_keyboard_while_it_is_open"
        )
    },
    @{
        # The rows themselves: built from the key table so a chord and a
        # palette row cannot drift, and probed so the launcher never offers a
        # shell this machine does not have.
        Name = "app palette rows"
        Package = "unterm-app"
        Filter = "window::palette_entry_tests::"
        ExpectedCount = 3
        RequiredTests = @(
            "window::palette_entry_tests::the_palette_lists_what_the_keys_do",
            "window::palette_entry_tests::the_launcher_offers_only_shells_that_exist",
            "window::palette_entry_tests::this_machine_has_at_least_one_shell_to_offer"
        )
    },
    @{
        # Scrollbar arithmetic. Every one of these is off by a bit until
        # someone drags to the very bottom and checks.
        Name = "app scrollbar"
        Package = "unterm-app"
        Filter = "scrollbar::"
        ExpectedCount = 11
        RequiredTests = @(
            "scrollbar::tests::scrolled_to_the_bottom_puts_the_thumb_at_the_bottom",
            "scrollbar::tests::a_drag_and_the_thumb_it_produces_agree",
            "scrollbar::tests::a_very_long_history_still_leaves_something_to_grab",
            "scrollbar::live_shape_tests::a_pane_with_history_above_it_gets_a_bar"
        )
    },
    @{
        # A bell is counted, not flagged: a flag is missed when two land
        # between frames, and shown twice if a reader forgets to clear it.
        Name = "engine bell"
        Package = "unterm-engine"
        Filter = "next_core::screen_state::bell_tests::"
        ExpectedCount = 2
        RequiredTests = @(
            "next_core::screen_state::bell_tests::ringing_the_bell_is_counted_rather_than_flagged",
            "next_core::screen_state::bell_tests::a_bell_inside_an_osc_string_terminates_it_rather_than_ringing"
        )
    },
    @{
        # Links a program marked, and links it merely printed. Opening is
        # behind a modifier: a terminal where a stray click launches a
        # browser is one you cannot select text in.
        Name = "app links"
        Package = "unterm-app"
        Filter = "links::tests::"
        ExpectedCount = 11
        RequiredTests = @(
            "links::tests::a_printed_url_is_a_link",
            "links::tests::a_marked_link_wins_over_the_text_that_looks_like_one",
            "links::tests::a_click_only_opens_with_the_modifier",
            "links::tests::an_unrecognised_scheme_is_refused_rather_than_handed_to_the_shell"
        )
    },
    @{
        # Underlines and the rest. A line the program asked for is
        # information; dropping it silently loses what it sent.
        Name = "app decorations"
        Package = "unterm-render"
        Filter = "decorations::tests::"
        ExpectedCount = 10
        RequiredTests = @(
            "decorations::tests::an_underline_sits_below_the_baseline",
            "decorations::tests::a_double_underline_is_two_lines_that_do_not_overlap",
            "decorations::tests::strikethrough_crosses_the_text_not_the_gap_below_it",
            "decorations::tests::lines_scale_with_the_cell_rather_than_staying_one_pixel"
        )
    },
    @{
        # The cursor inverts its own cell and no other. A window of one cell
        # in each direction painted the row above the prompt in the
        # background colour, and characters there vanished.
        Name = "app cursor inversion"
        Package = "unterm-app"
        Filter = "terminal::cursor_inversion_tests::"
        ExpectedCount = 1
        RequiredTests = @(
            "terminal::cursor_inversion_tests::the_cursor_inverts_its_own_cell_and_no_other"
        )
    },
    @{
        # Shaping a row: where a ligature lands, and where a run has to stop.
        # A run that crosses a face is shaped by the wrong font, which is how
        # a CJK character mid-word comes out as a box.
        Name = "app text shaping"
        Package = "unterm-app"
        Filter = "shape::tests::"
        ExpectedCount = 9
        RequiredTests = @(
            "shape::tests::a_font_change_ends_a_run",
            "shape::tests::a_colour_change_ends_a_run",
            "shape::tests::a_glyph_finds_the_column_its_cluster_came_from",
            "shape::tests::wide_characters_advance_the_column_by_their_width"
        )
    },
    @{
        # The fallback list has to name families as the font files report
        # them. It said "Microsoft YaHei UI", which matches nothing, and
        # Chinese drew as boxes on the platform most likely to need it.
        Name = "app font fallback"
        Package = "unterm-app"
        Filter = "fonts::cjk_tests::"
        ExpectedCount = 2
        RequiredTests = @(
            "fonts::cjk_tests::a_chinese_character_finds_a_face_that_has_it",
            "fonts::cjk_tests::the_stack_actually_has_a_face_with_chinese"
        )
    },
    @{
        # Who owns the mouse. Getting this wrong means either vim never sees a
        # click, or a click in vim also drags out a selection nobody asked for.
        Name = "app mouse ownership"
        Package = "unterm-app"
        Filter = "mouse::tests::"
        ExpectedCount = 9
        RequiredTests = @(
            "mouse::tests::with_reporting_off_the_terminal_keeps_the_mouse",
            "mouse::tests::a_program_that_asked_for_clicks_gets_them",
            "mouse::tests::shift_takes_the_mouse_back_from_the_program",
            "mouse::tests::button_motion_mode_gets_motion_only_while_dragging"
        )
    },
    @{
        # Composing CJK text: where it is drawn, and how wide it measures.
        # Both are off by a character until someone types Chinese and looks.
        Name = "app input method"
        Package = "unterm-app"
        Filter = "ime::tests::"
        ExpectedCount = 8
        RequiredTests = @(
            "ime::tests::a_chinese_character_is_two_columns_wide",
            "ime::tests::the_caret_is_measured_in_columns_not_bytes",
            "ime::tests::a_caret_past_the_end_is_clamped_rather_than_panicking",
            "ime::tests::composing_near_the_right_edge_stays_inside_the_pane"
        )
    },
    @{
        # What an agent is told the keys do has to be what the window does.
        Name = "app keys"
        Package = "unterm-app"
        Filter = "keys::tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "keys::tests::plain_ctrl_c_stays_the_programs_interrupt",
            "keys::tests::plain_tab_still_completes_in_the_shell",
            "keys::tests::unshifted_pages_belong_to_the_program",
            "keys::tests::every_binding_has_a_distinct_key"
        )
    },
    @{
        # A parked agent write has to be visible and answerable, or it times
        # out into a refusal the agent cannot explain.
        Name = "app confirmation banner"
        Package = "unterm-app"
        Filter = "confirm::tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "confirm::tests::the_banner_says_who_is_asking_and_for_what",
            "confirm::tests::every_option_is_offered",
            "confirm::tests::a_long_command_is_cut_rather_than_allowed_to_cover_the_screen",
            "confirm::tests::only_the_offered_keys_decide"
        )
    },
    @{
        # Pixels to cells, and when a drag has actually begun. Both are off by
        # one cell until someone drags across a line and looks.
        Name = "selection input"
        Package = "unterm-app"
        Filter = "select::tests::"
        ExpectedCount = 8
        RequiredTests = @(
            "select::tests::a_pixel_lands_in_the_cell_that_contains_it",
            "select::tests::the_row_is_measured_from_the_top_of_the_scrollback",
            "select::tests::a_position_outside_the_window_does_not_go_negative",
            "select::tests::a_click_that_never_moves_is_not_a_selection",
            "select::tests::dragging_backwards_keeps_the_anchor_where_it_started"
        )
    },
    @{
        # A pane rectangle one cell out is a line of text sliding under the
        # divider -- easier to assert than to notice.
        Name = "pane placement"
        Package = "unterm-app"
        Filter = "panes::tests::"
        ExpectedCount = 7
        RequiredTests = @(
            "panes::tests::a_pane_to_the_right_starts_after_the_cells_before_it",
            "panes::tests::a_pane_below_starts_after_the_rows_above_it",
            "panes::tests::a_pane_the_engine_sized_to_nothing_still_asks_for_a_cell",
            "panes::tests::placements_do_not_overlap"
        )
    },
    @{
        # Wheel and page arithmetic: all of it wrong by a sign or a factor of
        # three until someone tries it.
        Name = "scroll amounts"
        Package = "unterm-app"
        Filter = "scroll::tests::"
        ExpectedCount = 7
        RequiredTests = @(
            "scroll::tests::a_wheel_notch_moves_three_lines",
            "scroll::tests::a_trackpad_moves_the_text_it_is_pushing",
            "scroll::tests::a_small_trackpad_movement_still_moves",
            "scroll::tests::a_zero_height_cell_does_not_divide_by_zero",
            "scroll::tests::a_page_keeps_one_line_of_context"
        )
    },
    @{
        # One face cannot draw everything. Without this, whole writing systems
        # come out as the empty box.
        Name = "font fallback"
        Package = "unterm-app"
        Filter = "fonts::tests::"
        ExpectedCount = 5
        RequiredTests = @(
            "fonts::tests::latin_comes_from_the_primary_face",
            "fonts::tests::a_character_the_primary_lacks_finds_another_face",
            "fonts::tests::a_character_nobody_has_still_draws_something",
            "fonts::tests::rasterizing_reports_which_face_drew_it"
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
