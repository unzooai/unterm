//! What settings exist, and what counts as a valid one.
//!
//! Without this the config layer can parse a file but not judge it, and an
//! unknown key is only caught if some caller happens to pass the right list.
//! Naming the settings in one place is what lets a config be checked before it
//! is used -- the property the Lua runtime could never offer, because a Lua
//! config only reveals what it sets by running.

use super::config::{Config, ConfigError};
use super::tab_title;

/// Every setting the engine reads.
///
/// Adding a feature means adding its key here; a key that is not here is
/// rejected, which is the whole point.
pub const SETTINGS: &[&str] = &[
    "font_family",
    "font_fallback",
    "font_size",
    "line_height",
    "color_scheme",
    "colors.background",
    "colors.foreground",
    "colors.tab_bar_lift",
    "colors.inactive_dim",
    "scrollback_lines",
    "enable_scroll_bar",
    "shell",
    "path_append",
    "window.background_opacity",
    "window.decorations",
    "window.initial_cols",
    "window.initial_rows",
    "window.close_confirmation",
    "tab_bar.position",
    "tab_bar.max_width",
    "tab_bar.hide_if_only_one_tab",
    "tab_bar.show_index",
    "tab_bar.show_new_tab_button",
    "tab_bar.title_format",
    "tab_bar.fallback_title",
    "tab_bar.strip_extension",
    "tab_bar.capitalize",
    "title_button.style",
    "title_button.alignment",
    "title_button.buttons",
];

/// Sections whose keys the user invents.
///
/// Environment variable names and key chords cannot be listed in advance, so
/// these prefixes accept anything -- but only these, so a typo anywhere else is
/// still caught. Chords are validated where they are parsed, which is where a
/// misspelled one can be reported usefully.
const OPEN_SECTIONS: &[&str] = &["env.", "keys."];

/// Check a config that has already had its platform sections resolved.
///
/// Returns every problem at once. Fixing a config one complaint per run is a
/// miserable loop, and it is the loop a runtime that stops at the first error
/// always produces.
pub fn check(config: &Config) -> Vec<ConfigError> {
    let mut errors: Vec<ConfigError> = config
        .validate_keys(SETTINGS)
        .into_iter()
        .filter(|error| {
            // Keep the complaint only if it is not about an invented key.
            !OPEN_SECTIONS
                .iter()
                .any(|prefix| error.message.contains(&format!("`{prefix}")))
        })
        .collect();

    if let Some(line) = config.line_of("tab_bar.title_format") {
        match config.str_of("tab_bar.title_format") {
            Ok(Some(format)) => {
                for name in tab_title::unknown_placeholders(format) {
                    // A template rendering `{tittle}` literally is the same
                    // failure as a key that does nothing.
                    errors.push(ConfigError {
                        line,
                        message: format!(
                            "`{{{name}}}` is not a tab title placeholder -- try {}",
                            tab_title::PLACEHOLDERS
                                .iter()
                                .map(|known| format!("`{{{known}}}`"))
                                .collect::<Vec<_>>()
                                .join(" or ")
                        ),
                    });
                }
            }
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
    }

    for (key, low, high) in [
        ("window.background_opacity", 0.0, 1.0),
        ("line_height", 0.1, 10.0),
    ] {
        match config.float_of(key) {
            Ok(Some(value)) if !(low..=high).contains(&value) => errors.push(ConfigError {
                line: config.line_of(key).unwrap_or(0),
                message: format!("`{key}` should be between {low} and {high}, but is {value}"),
            }),
            Ok(_) => {}
            Err(error) => errors.push(error),
        }
    }

    errors.sort_by_key(|error| error.line);
    errors
}

#[cfg(test)]
mod tests {
    use super::super::config::parse;
    use super::*;

    fn check_source(source: &str) -> Vec<ConfigError> {
        let config = parse(source).expect("config should parse");
        check(&config.resolve_platform("windows"))
    }

    #[test]
    fn a_config_using_known_settings_passes() {
        let errors = check_source(
            r#"
            font_size = 13
            scrollback_lines = 100000
            [tab_bar]
            position = "Left"
            title_format = "  {title}  "
            "#,
        );

        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn an_unknown_setting_is_rejected() {
        let errors = check_source("font_sze = 13");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("font_size"), "{}", errors[0].message);
    }

    #[test]
    fn environment_variables_are_the_one_place_names_are_invented() {
        let errors = check_source("[env]\nMY_OWN_VARIABLE = \"1\"");

        // These cannot be listed in advance, but nothing else gets the pass.
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn a_typo_outside_that_section_is_still_caught() {
        let errors = check_source("[tab_bar]\npositon = \"Left\"");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("tab_bar.position"), "{}", errors[0].message);
    }

    #[test]
    fn a_bad_title_placeholder_is_rejected_with_the_real_ones() {
        let errors = check_source("[tab_bar]\ntitle_format = \"{tittle}\"");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("{title}"), "{}", errors[0].message);
    }

    #[test]
    fn a_value_outside_its_range_is_rejected() {
        let errors = check_source("[window]\nbackground_opacity = 1.5");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("between 0 and 1"), "{}", errors[0].message);
        // The endpoints themselves are fine.
        assert!(check_source("[window]\nbackground_opacity = 1.0").is_empty());
    }

    #[test]
    fn a_platform_section_is_checked_after_it_is_resolved() {
        let errors = check_source("[platform.windows]\nshel = \"powershell.exe\"");

        // Checking before resolution would see `platform.windows.shel` and
        // suggest nothing useful; checking after sees the real key.
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("did you mean `shell`"), "{}", errors[0].message);
    }

    #[test]
    fn every_problem_is_reported_at_once_and_in_file_order() {
        let errors = check_source(
            r#"
            font_sze = 13
            [window]
            background_opacity = 2.0
            [tab_bar]
            title_format = "{nope}"
            "#,
        );

        assert_eq!(errors.len(), 3);
        assert!(errors[0].line < errors[1].line);
        assert!(errors[1].line < errors[2].line);
    }

    #[test]
    fn the_config_this_project_ships_is_valid_on_every_platform() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/unterm.conf");
        let source = std::fs::read_to_string(path).expect("the shipped config must exist");
        let config = parse(&source).expect("the shipped config must parse");

        // Shipping a default config the engine would reject is the one config
        // bug every user would hit at once.
        for platform in ["windows", "macos", "linux"] {
            let resolved = config.resolve_platform(platform);
            let errors = check(&resolved);
            assert!(errors.is_empty(), "{platform}: {errors:?}");
        }

        // The platform branch the Lua version wrote as if/elseif/else.
        let style = |platform: &str| {
            config
                .resolve_platform(platform)
                .str_of("title_button.style")
                .unwrap()
                .map(str::to_string)
        };
        assert_eq!(style("macos").as_deref(), Some("MacOsCustom"));
        assert_eq!(style("windows").as_deref(), Some("Windows"));
        assert_eq!(style("linux").as_deref(), Some("Gnome"));
    }

    #[test]
    fn a_wrong_type_is_reported_rather_than_ignored() {
        let errors = check_source("line_height = \"tall\"");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("number"), "{}", errors[0].message);
    }
}
