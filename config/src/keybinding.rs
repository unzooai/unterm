//! Key bindings, declared rather than programmed.
//!
//! Binding a key was the last thing that needed Lua, and removing the
//! interpreter without replacing this would have left users unable to change a
//! single shortcut. A binding is data -- a chord and the name of an action --
//! so it belongs in the config file like everything else.
//!
//! In `unterm.conf`:
//!
//! ```text
//! [keys]
//! CTRL|SHIFT+T = "SpawnWindow"
//! CTRL|SHIFT+W = "ToggleFullScreen"
//! ```
//!
//! The two things this has to get right are the two that waste an afternoon
//! otherwise: a misspelled key or action must say so with the line, and two
//! bindings for the same chord must be an error rather than one quietly
//! winning.

use crate::keys::{DeferredKeyCode, Key, KeyNoAction};
use std::convert::TryFrom;
use crate::keyassignment::KeyAssignment;
use unterm_engine::next_core::config::{Config, ConfigError};
use wezterm_dynamic::FromDynamic;
use wezterm_input_types::Modifiers;

/// The section bindings live in.
pub const SECTION: &str = "keys.";

/// Read `[keys]` into the binding list, reporting every problem with its line.
pub fn bindings_from(config: &Config) -> (Vec<Key>, Vec<ConfigError>) {
    let mut keys = Vec::new();
    let mut errors = Vec::new();
    // Chord as written, and the line it was written on, so a second binding
    // for the same chord can name the first.
    let mut seen: Vec<(String, usize)> = Vec::new();

    for key in config.keys() {
        let Some(chord) = key.strip_prefix(SECTION) else {
            continue;
        };
        let line = config.line_of(key).unwrap_or(0);

        let action = match config.str_of(key) {
            Ok(Some(action)) => action,
            Ok(None) => {
                errors.push(ConfigError {
                    line,
                    message: format!("`{chord}` should be bound to an action name in quotes"),
                });
                continue;
            }
            Err(error) => {
                errors.push(error);
                continue;
            }
        };

        let parsed = match parse_chord(chord) {
            Ok(parsed) => parsed,
            Err(message) => {
                errors.push(ConfigError { line, message });
                continue;
            }
        };

        let action = match parse_action(action) {
            Ok(action) => action,
            Err(message) => {
                errors.push(ConfigError { line, message });
                continue;
            }
        };

        // Compare the normalized form: `CTRL|SHIFT+T` and `SHIFT|CTRL+t` are
        // the same chord, and letting both through would mean one silently
        // losing to the other at runtime.
        let normalized = normalize(&parsed);
        if let Some((_, first)) = seen.iter().find(|(chord, _)| *chord == normalized) {
            errors.push(ConfigError {
                line,
                message: format!("`{chord}` is already bound on line {first}"),
            });
            continue;
        }
        seen.push((normalized, line));

        keys.push(Key {
            key: parsed,
            action,
        });
    }

    (keys, errors)
}

/// Parse `CTRL|SHIFT+T`, `CMD+t`, `F5`, `LeftArrow`.
///
/// Modifiers are separated by `|` and the key follows the last `+`, so a chord
/// can bind `+` itself as `CTRL++`.
pub fn parse_chord(chord: &str) -> Result<KeyNoAction, String> {
    let chord = chord.trim();
    if chord.is_empty() {
        return Err("chord is empty".to_string());
    }

    // The key follows the last separator. When the chord ends in `+`, that
    // trailing character *is* the key, so the separator is the one before it --
    // otherwise `CTRL++` could never bind the plus key.
    let (mods_text, key_text) = if let Some(head) = chord.strip_suffix('+') {
        match head.rfind('+') {
            Some(index) => (&head[..index], "+"),
            None => (head, "+"),
        }
    } else {
        match chord.rfind('+') {
            Some(index) => (&chord[..index], &chord[index + 1..]),
            None => ("", chord),
        }
    };

    let mut mods = Modifiers::NONE;
    for name in mods_text.split(['|', '+']).filter(|part| !part.is_empty()) {
        mods |= parse_modifier(name.trim())?;
    }

    let key = DeferredKeyCode::try_from(key_text.trim())
        .map_err(|err| format!("`{key_text}` is not a key: {err:#}"))?;

    Ok(KeyNoAction { key, mods })
}

fn parse_modifier(name: &str) -> Result<Modifiers, String> {
    // Accepting the names people actually type, not just the canonical ones:
    // `CMD` and `SUPER` are the same key and both appear in every config in
    // the wild.
    Ok(match name.to_ascii_uppercase().as_str() {
        "CTRL" | "CONTROL" => Modifiers::CTRL,
        "ALT" | "OPT" | "OPTION" | "META" => Modifiers::ALT,
        "SHIFT" => Modifiers::SHIFT,
        "SUPER" | "CMD" | "WIN" | "WINDOWS" => Modifiers::SUPER,
        "LEADER" => Modifiers::LEADER,
        other => {
            return Err(format!(
                "`{other}` is not a modifier -- try CTRL, ALT, SHIFT, SUPER or LEADER"
            ))
        }
    })
}

/// Resolve an action name.
///
/// Only actions that take no arguments can be named this way; anything else
/// says so rather than failing with a deserializer's wording.
pub fn parse_action(name: &str) -> Result<KeyAssignment, String> {
    KeyAssignment::from_dynamic(
        &wezterm_dynamic::Value::String(name.to_string()),
        Default::default(),
    )
    .map_err(|_| {
        format!("`{name}` is not an action that can be named on its own. Actions taking arguments cannot be bound from the config yet.")
    })
}

/// A chord's canonical form, for comparing two spellings of one binding.
fn normalize(key: &KeyNoAction) -> String {
    format!("{:?}+{:?}", key.mods, key.key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use unterm_engine::next_core::config::parse;

    fn bindings(source: &str) -> (Vec<Key>, Vec<ConfigError>) {
        let config = parse(source).expect("config should parse");
        bindings_from(&config)
    }

    #[test]
    fn a_chord_and_an_action_become_a_binding() {
        let (keys, errors) = bindings("[keys]\nCTRL|SHIFT+T = \"SpawnWindow\"");

        assert!(errors.is_empty(), "{:?}", errors);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key.mods, Modifiers::CTRL | Modifiers::SHIFT);
    }

    #[test]
    fn modifiers_may_be_separated_by_either_character() {
        // Configs in the wild use both, and neither is worth an error.
        let (keys, errors) = bindings("[keys]\nCTRL+SHIFT+W = \"ToggleFullScreen\"");

        assert!(errors.is_empty(), "{:?}", errors);
        assert_eq!(keys[0].key.mods, Modifiers::CTRL | Modifiers::SHIFT);
    }

    #[test]
    fn the_names_people_actually_type_are_accepted() {
        for spelling in ["CMD+T", "SUPER+T", "WIN+T", "cmd+T"] {
            let parsed = parse_chord(spelling).expect(spelling);
            assert_eq!(parsed.mods, Modifiers::SUPER, "{spelling}");
        }
        assert_eq!(parse_chord("OPT+T").unwrap().mods, Modifiers::ALT);
    }

    #[test]
    fn a_chord_with_no_modifier_is_fine() {
        let parsed = parse_chord("F5").expect("F5 is a key");

        assert_eq!(parsed.mods, Modifiers::NONE);
    }

    #[test]
    fn the_plus_key_can_itself_be_bound() {
        // The last `+` is the separator, so a trailing one is the key.
        let parsed = parse_chord("CTRL++").expect("CTRL++ binds the plus key");

        assert_eq!(parsed.mods, Modifiers::CTRL);
    }

    #[test]
    fn a_misspelled_modifier_says_what_the_real_ones_are() {
        let error = parse_chord("CTLR+T").expect_err("CTLR is not a modifier");

        assert!(error.contains("CTRL"), "{error}");
    }

    #[test]
    fn a_misspelled_key_is_reported_with_its_line() {
        let (_, errors) = bindings("[keys]\n\nCTRL+NotAKeyAtAll = \"SpawnWindow\"");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 3);
        assert!(errors[0].message.contains("NotAKeyAtAll"), "{}", errors[0].message);
    }

    #[test]
    fn a_misspelled_action_is_reported_rather_than_ignored() {
        let (keys, errors) = bindings("[keys]\nCTRL+T = \"SpwanWindow\"");

        // A binding that silently does nothing is the worst kind: the key does
        // not work and nothing says why.
        assert!(keys.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("SpwanWindow"), "{}", errors[0].message);
    }

    #[test]
    fn binding_one_chord_twice_is_an_error_naming_both_lines() {
        let (keys, errors) = bindings(
            "[keys]\nCTRL|SHIFT+T = \"SpawnWindow\"\nSHIFT|CTRL+T = \"ToggleFullScreen\"",
        );

        // Modifier order carries no meaning, so these are one chord written
        // two ways, and letting both through means one silently loses at
        // runtime. `T` and `t` are a different matter -- they are distinct key
        // codes -- so only the modifiers are normalized.
        assert_eq!(keys.len(), 1);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 3);
        assert!(errors[0].message.contains("line 2"), "{}", errors[0].message);
    }

    #[test]
    fn an_action_needing_arguments_says_so() {
        let error = parse_action("ActivateTab").expect_err("ActivateTab takes an index");

        assert!(error.contains("arguments"), "{error}");
    }

    #[test]
    fn a_config_with_no_keys_section_binds_nothing() {
        let (keys, errors) = bindings("font_size = 13");

        assert!(keys.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn a_binding_that_is_not_a_string_is_reported() {
        let (_, errors) = bindings("[keys]\nCTRL+T = 12");

        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].message.contains("string"),
            "{}",
            errors[0].message
        );
    }
}
