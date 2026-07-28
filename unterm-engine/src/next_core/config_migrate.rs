//! Convert an existing Lua config into a declarative one.
//!
//! Almost every real terminal config is assignments -- `font_size = 12`,
//! `colors = { background = "#1e1e2e" }` -- dressed as a program. Those convert
//! mechanically, so nobody has to retype their config.
//!
//! The rest does not convert, and that is the part this module actually exists
//! to get right. A migration that silently drops what it does not understand is
//! worse than no migration: the user gets a config that parses, looks complete,
//! and quietly lost their keybindings. So anything not understood comes back in
//! `unconverted`, with the line and the reason, and callers are expected to put
//! it in front of the user.

use super::config;

/// Something the converter would not translate, kept so it can be shown rather
/// than lost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unconverted {
    /// 1-based line in the original Lua file.
    pub line: usize,
    pub snippet: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Migration {
    /// The declarative config, ready to write out.
    pub text: String,
    /// Everything the converter refused to guess at.
    pub unconverted: Vec<Unconverted>,
}

impl Migration {
    /// True when the whole file converted, so a caller can skip the report.
    pub fn is_complete(&self) -> bool {
        self.unconverted.is_empty()
    }
}

/// A key/value pair recovered from the Lua source, before rendering.
struct Setting {
    key: String,
    value: String,
}

/// Convert Lua config source.
///
/// This is a translator for the declarative subset, not a Lua interpreter --
/// the point of the new format is that configs are not executed. Anything that
/// needs evaluation is reported instead.
pub fn migrate_lua(source: &str) -> Migration {
    let mut settings: Vec<Setting> = Vec::new();
    let mut unconverted = Vec::new();
    // Where each setting was defined, so a second definition can name the
    // first instead of silently overwriting it.
    let mut seen: Vec<(String, usize)> = Vec::new();
    // One frame per open brace. Named frames become sections; anonymous ones
    // are call arguments and list elements, whose contents are not settings.
    // Tracking every brace -- not just the ones we understand -- is what keeps
    // the stack aligned with the file.
    let mut frames: Vec<Option<String>> = Vec::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line = index + 1;
        let text = strip_lua_comment(raw_line).trim().trim_end_matches(',').trim();
        if text.is_empty() {
            continue;
        }

        let (opens, closes) = brace_balance(text);

        // A line that only closes braces just leaves sections.
        if text.chars().all(|ch| matches!(ch, '}' | ')' | ',' | ' ')) {
            for _ in 0..closes {
                frames.pop();
            }
            continue;
        }

        if is_structural(text) {
            if text == "return {" {
                // This brace opens the settings table itself, so what follows
                // are settings -- not a call argument. An empty name keeps it
                // out of the section path.
                frames.push(Some(String::new()));
            } else {
                adjust_frames(&mut frames, opens, closes);
            }
            continue;
        }

        let Some((raw_key, raw_value)) = split_assignment(text) else {
            unconverted.push(Unconverted {
                line,
                snippet: text.to_string(),
                reason: "not an assignment".to_string(),
            });
            adjust_frames(&mut frames, opens, closes);
            continue;
        };

        if raw_key.starts_with("local ") {
            unconverted.push(Unconverted {
                line,
                snippet: text.to_string(),
                reason: "a local variable, which a declarative config has no place for".to_string(),
            });
            adjust_frames(&mut frames, opens, closes);
            continue;
        }

        let Some(key) = normalize_key(raw_key) else {
            unconverted.push(Unconverted {
                line,
                snippet: text.to_string(),
                reason: "setting name is computed, so it cannot be read statically".to_string(),
            });
            adjust_frames(&mut frames, opens, closes);
            continue;
        };

        // A table opened on its own line becomes a section.
        if raw_value == "{" && closes == 0 {
            frames.push(Some(key));
            continue;
        }

        // Inside a call argument or a list of tables, these are not settings at
        // all; naming them would invent keys the user never wrote.
        if frames.iter().any(|frame| frame.is_none()) {
            unconverted.push(Unconverted {
                line,
                snippet: text.to_string(),
                reason: "sits inside a Lua call, not in the settings table".to_string(),
            });
            adjust_frames(&mut frames, opens, closes);
            continue;
        }

        let section: Vec<&str> = frames
            .iter()
            .filter_map(|frame| frame.as_deref())
            .filter(|name| !name.is_empty())
            .collect();
        let full_key = if section.is_empty() {
            key
        } else {
            format!("{}.{}", section.join("."), key)
        };

        match convert_value(raw_value) {
            Ok(value) => {
                if let Some((_, first)) = seen.iter().find(|(name, _)| *name == full_key) {
                    // Platform branches set the same key more than once. Which
                    // one wins is a question only the user can answer, and
                    // emitting both would produce a file that will not parse.
                    unconverted.push(Unconverted {
                        line,
                        snippet: text.to_string(),
                        reason: format!(
                            "`{full_key}` is already set on line {first}; pick the one you want"
                        ),
                    });
                } else {
                    seen.push((full_key.clone(), line));
                    settings.push(Setting {
                        key: full_key,
                        value,
                    });
                }
            }
            Err(reason) => unconverted.push(Unconverted {
                line,
                snippet: text.to_string(),
                reason,
            }),
        }
        adjust_frames(&mut frames, opens, closes);
    }

    Migration {
        text: render(&settings),
        unconverted,
    }
}

/// Keep the frame stack level with the braces actually on the line.
///
/// Frames opened here are anonymous: they came from a call or a list, so
/// nothing inside them is a setting.
fn adjust_frames(frames: &mut Vec<Option<String>>, opens: usize, closes: usize) {
    for _ in 0..opens.saturating_sub(closes) {
        frames.push(None);
    }
    for _ in 0..closes.saturating_sub(opens) {
        frames.pop();
    }
}

/// Count braces that are not inside a string.
fn brace_balance(text: &str) -> (usize, usize) {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut in_quotes: Option<char> = None;
    let mut escaped = false;

    for ch in text.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match in_quotes {
            Some(quote) => match ch {
                '\\' => escaped = true,
                _ if ch == quote => in_quotes = None,
                _ => {}
            },
            None => match ch {
                '\'' | '"' => in_quotes = Some(ch),
                '{' => opens += 1,
                '}' => closes += 1,
                _ => {}
            },
        }
    }
    (opens, closes)
}

/// Lines that only shape the file: the local/return/config boilerplate every
/// Lua config carries.
fn is_structural(text: &str) -> bool {
    matches!(
        text,
        "return config" | "return {" | "{" | "local config = {}" | "local wezterm = require 'wezterm'"
    ) || text.starts_with("local wezterm = require")
        || text.starts_with("local config = wezterm.config_builder")
        || text == "}"
}

fn strip_lua_comment(line: &str) -> &str {
    let mut in_quotes: Option<char> = None;
    let bytes: Vec<char> = line.chars().collect();
    for index in 0..bytes.len() {
        let ch = bytes[index];
        match in_quotes {
            Some(quote) => {
                if ch == '\\' {
                    continue;
                }
                if ch == quote {
                    in_quotes = None;
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    in_quotes = Some(ch);
                } else if ch == '-' && bytes.get(index + 1) == Some(&'-') {
                    let byte_index = line
                        .char_indices()
                        .nth(index)
                        .map(|(offset, _)| offset)
                        .unwrap_or(line.len());
                    return &line[..byte_index];
                }
            }
        }
    }
    line
}

/// Split on the first `=` that is an assignment, not `==` or `<=`.
fn split_assignment(text: &str) -> Option<(&str, &str)> {
    let chars: Vec<char> = text.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if *ch != '=' {
            continue;
        }
        if chars.get(index + 1) == Some(&'=') {
            return None;
        }
        if matches!(chars.get(index.wrapping_sub(1)), Some('=' | '<' | '>' | '~')) {
            return None;
        }
        let byte_index = text.char_indices().nth(index).map(|(offset, _)| offset)?;
        return Some((text[..byte_index].trim(), text[byte_index + 1..].trim()));
    }
    None
}

/// `config.font_size`, `config['font_size']` and a bare `font_size` all name the
/// same setting.
fn normalize_key(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let raw = raw
        .strip_prefix("config.")
        .or_else(|| raw.strip_prefix("M."))
        .unwrap_or(raw);

    if let Some(rest) = raw.strip_prefix("config[").or_else(|| raw.strip_prefix('[')) {
        let inner = rest.strip_suffix(']')?.trim();
        let name = inner
            .strip_prefix('\'')
            .and_then(|value| value.strip_suffix('\''))
            .or_else(|| {
                inner
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
            })?;
        return Some(name.to_string());
    }

    let valid = !raw.is_empty()
        && raw
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    valid.then(|| raw.to_string())
}

/// Translate a Lua value into the declarative format, or say why not.
fn convert_value(raw: &str) -> Result<String, String> {
    let raw = raw.trim().trim_end_matches(',').trim();
    if raw.is_empty() {
        return Err("value is empty".to_string());
    }

    if raw == "true" || raw == "false" {
        return Ok(raw.to_string());
    }
    if raw.parse::<i64>().is_ok() || raw.parse::<f64>().is_ok() {
        return Ok(raw.to_string());
    }

    if let Some(text) = lua_string(raw) {
        return Ok(render_string(&text));
    }

    if let Some(inner) = raw.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
        let inner = inner.trim();
        if inner.is_empty() {
            return Ok("[]".to_string());
        }
        // A table with named fields is a section, which only the line-by-line
        // walk can express; refuse rather than flatten it wrongly.
        if split_assignment(inner).is_some() {
            return Err("nested table written on one line -- convert it by hand".to_string());
        }
        let mut items = Vec::new();
        for item in inner.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            items.push(convert_value(item)?);
        }
        return Ok(format!("[{}]", items.join(", ")));
    }

    if raw.contains("wezterm.") || raw.contains("require") {
        return Err("calls into the Lua runtime, which the new config does not have".to_string());
    }
    if raw.contains("function") {
        return Err("is a function; the new config holds values, not code".to_string());
    }
    if raw.contains("..") {
        return Err("builds a string at runtime -- write the finished value".to_string());
    }

    Err(format!(
        "`{raw}` is not a plain value the converter recognises"
    ))
}

/// Read a Lua string literal, single- or double-quoted.
fn lua_string(raw: &str) -> Option<String> {
    let quote = raw.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let inner = raw.strip_prefix(quote)?.strip_suffix(quote)?;
    // A quote of the other kind inside is fine; one of the same kind means this
    // is a concatenation or something else we should not be parsing.
    if inner.contains(quote) {
        return None;
    }
    Some(inner.to_string())
}

/// Quote a value for the declarative format.
fn render_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Render settings, grouping section keys under headers.
fn render(settings: &[Setting]) -> String {
    let mut out = String::new();
    let mut current_section = String::new();

    let mut ordered: Vec<&Setting> = settings.iter().collect();
    // Group by section while keeping the user's order inside each one, so the
    // result reads like the file they wrote.
    ordered.sort_by_key(|setting| section_of(&setting.key).to_string());

    for setting in ordered {
        let section = section_of(&setting.key);
        if section != current_section {
            if !out.is_empty() {
                out.push('\n');
            }
            if !section.is_empty() {
                out.push_str(&format!("[{section}]\n"));
            }
            current_section = section.to_string();
        }
        let leaf = setting
            .key
            .rsplit_once('.')
            .map(|(_, leaf)| leaf)
            .unwrap_or(&setting.key);
        out.push_str(&format!("{leaf} = {}\n", setting.value));
    }
    out
}

fn section_of(key: &str) -> &str {
    key.rsplit_once('.').map(|(head, _)| head).unwrap_or("")
}

/// Convert, then confirm the result actually parses.
///
/// A converter that emits something the parser rejects would hand the user a
/// broken file and call it a migration.
pub fn migrate_and_check(source: &str) -> Result<Migration, Vec<config::ConfigError>> {
    let migration = migrate_lua(source);
    config::parse(&migration.text)?;
    Ok(migration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_the_ordinary_assignments() {
        let migration = migrate_lua(
            r#"
local wezterm = require 'wezterm'
local config = wezterm.config_builder()
config.font_size = 12.5
config.scrollback_lines = 10000
config.use_ime = true
config.font_family = 'Cascadia Mono'
return config
"#,
        );

        assert!(migration.is_complete(), "{:?}", migration.unconverted);
        let parsed = config::parse(&migration.text).expect("output should parse");
        assert_eq!(parsed.float_of("font_size").unwrap(), Some(12.5));
        assert_eq!(parsed.int_of("scrollback_lines").unwrap(), Some(10000));
        assert_eq!(parsed.bool_of("use_ime").unwrap(), Some(true));
        assert_eq!(parsed.str_of("font_family").unwrap(), Some("Cascadia Mono"));
    }

    #[test]
    fn a_nested_table_becomes_a_section() {
        let migration = migrate_lua(
            r##"
config.colors = {
  background = '#1e1e2e',
  foreground = '#cdd6f4',
}
"##,
        );

        assert!(migration.is_complete(), "{:?}", migration.unconverted);
        let parsed = config::parse(&migration.text).expect("output should parse");
        assert_eq!(parsed.str_of("colors.background").unwrap(), Some("#1e1e2e"));
        assert_eq!(parsed.str_of("colors.foreground").unwrap(), Some("#cdd6f4"));
    }

    #[test]
    fn a_list_table_becomes_a_list() {
        let migration = migrate_lua("config.fonts = { 'Cascadia Mono', 'Noto Sans' }");

        assert!(migration.is_complete(), "{:?}", migration.unconverted);
        let parsed = config::parse(&migration.text).expect("output should parse");
        let values = parsed.list_of("fonts").unwrap().unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], config::Value::Str("Cascadia Mono".to_string()));
    }

    #[test]
    fn bracket_indexing_names_the_same_setting_as_a_dot() {
        let migration = migrate_lua("config['font_size'] = 14");

        let parsed = config::parse(&migration.text).expect("output should parse");
        assert_eq!(parsed.int_of("font_size").unwrap(), Some(14));
    }

    #[test]
    fn a_function_is_reported_rather_than_dropped() {
        let migration = migrate_lua(
            r#"
config.font_size = 12
config.format_tab_title = function(tab) return tab.index end
"#,
        );

        // Silently losing this is the failure mode that matters: the config
        // would parse, look complete, and have quietly lost a setting.
        assert!(!migration.is_complete());
        assert_eq!(migration.unconverted.len(), 1);
        assert_eq!(migration.unconverted[0].line, 3);
        assert!(
            migration.unconverted[0].reason.contains("function"),
            "{}",
            migration.unconverted[0].reason
        );
        // What did convert is still converted.
        let parsed = config::parse(&migration.text).expect("output should parse");
        assert_eq!(parsed.int_of("font_size").unwrap(), Some(12));
    }

    #[test]
    fn a_lua_call_is_reported_with_its_reason() {
        let migration = migrate_lua("config.color_scheme = wezterm.get_builtin_color_schemes()");

        assert_eq!(migration.unconverted.len(), 1);
        assert!(
            migration.unconverted[0].reason.contains("Lua runtime"),
            "{}",
            migration.unconverted[0].reason
        );
    }

    #[test]
    fn a_concatenated_value_is_reported() {
        let migration = migrate_lua("config.shell = home .. '/bin/sh'");

        assert_eq!(migration.unconverted.len(), 1);
        assert!(
            migration.unconverted[0].reason.contains("runtime"),
            "{}",
            migration.unconverted[0].reason
        );
    }

    #[test]
    fn every_unconverted_line_carries_its_source() {
        let migration = migrate_lua("config.a = function() end\nconfig.b = wezterm.x()");

        assert_eq!(migration.unconverted.len(), 2);
        // Without the snippet the user has to go find it themselves.
        assert!(migration.unconverted[0].snippet.contains("config.a"));
        assert!(migration.unconverted[1].snippet.contains("config.b"));
    }

    #[test]
    fn a_comment_does_not_become_a_setting() {
        let migration = migrate_lua("-- font_size = 12\nconfig.font_size = 14 -- bigger");

        assert!(migration.is_complete(), "{:?}", migration.unconverted);
        let parsed = config::parse(&migration.text).expect("output should parse");
        assert_eq!(parsed.int_of("font_size").unwrap(), Some(14));
    }

    #[test]
    fn a_dash_inside_a_string_is_not_a_comment() {
        let migration = migrate_lua("config.title = 'well -- actually'");

        let parsed = config::parse(&migration.text).expect("output should parse");
        assert_eq!(parsed.str_of("title").unwrap(), Some("well -- actually"));
    }

    #[test]
    fn the_return_table_form_converts_too() {
        let migration = migrate_lua("return {\n  font_size = 12,\n  use_ime = true,\n}");

        assert!(migration.is_complete(), "{:?}", migration.unconverted);
        let parsed = config::parse(&migration.text).expect("output should parse");
        assert_eq!(parsed.int_of("font_size").unwrap(), Some(12));
        assert_eq!(parsed.bool_of("use_ime").unwrap(), Some(true));
    }

    #[test]
    fn a_quoted_value_survives_requoting() {
        let migration = migrate_lua(r#"config.title = 'say "hi"'"#);

        let parsed = config::parse(&migration.text).expect("output should parse");
        assert_eq!(parsed.str_of("title").unwrap(), Some(r#"say "hi""#));
    }

    #[test]
    fn the_converted_output_is_checked_against_the_parser() {
        let migration = migrate_and_check(
            r##"
config.font_size = 12
config.colors = {
  background = '#1e1e2e',
}
"##,
        )
        .expect("converted config should parse");

        // A converter that emits something the parser rejects hands the user a
        // broken file and calls it a migration.
        assert!(migration.is_complete());
    }

    #[test]
    fn an_empty_config_converts_to_an_empty_config() {
        let migration = migrate_lua("local wezterm = require 'wezterm'\nreturn config\n");

        assert!(migration.is_complete());
        assert!(migration.text.trim().is_empty());
    }

    #[test]
    fn a_realistic_config_converts_to_something_that_parses() {
        // Cut down from the config this project actually ships: a runtime call
        // with a nested table, a platform branch setting one key three times,
        // locals, and an event handler.
        let migration = migrate_and_check(
            r##"
local wezterm = require 'wezterm'
local config = wezterm.config_builder()
local theme_bg = '#111315'
config.font_size = 13
config.font = wezterm.font_with_fallback({
  { family = 'JetBrains Mono' },
  'Noto Sans CJK SC',
})
if wezterm.target_triple:find('darwin') then
  config.integrated_title_button_style = 'MacOsCustom'
elseif wezterm.target_triple:find('windows') then
  config.integrated_title_button_style = 'Windows'
end
config.window_frame = {
  active_titlebar_bg = theme_bg,
  font_size = 12.0,
}
wezterm.on('update-status', function(window, pane)
  window:set_right_status('')
end)
return config
"##,
        )
        .expect("whatever converts must still parse");

        let parsed = config::parse(&migration.text).unwrap();
        assert_eq!(parsed.int_of("font_size").unwrap(), Some(13));
        assert_eq!(parsed.float_of("window_frame.font_size").unwrap(), Some(12.0));

        // The call argument must not become settings, or the file grows keys
        // the user never wrote.
        assert!(parsed.get("family").is_none());
        assert!(!parsed.keys().any(|key| key.contains("font_with_fallback")));

        // The second branch is reported, not emitted alongside the first --
        // two definitions of one key would not parse, and which wins is the
        // user's call, not ours.
        assert_eq!(
            parsed.str_of("integrated_title_button_style").unwrap(),
            Some("MacOsCustom")
        );
        let duplicate = migration
            .unconverted
            .iter()
            .find(|item| item.snippet.contains("'Windows'"))
            .expect("the second branch must be reported");
        assert!(
            duplicate.reason.contains("already set"),
            "{}",
            duplicate.reason
        );

        // A value that came from a local cannot be resolved without running
        // the file, so it is reported rather than guessed at.
        assert!(migration
            .unconverted
            .iter()
            .any(|item| item.snippet.contains("active_titlebar_bg")));
    }

    #[test]
    fn a_comparison_is_not_mistaken_for_an_assignment() {
        let migration = migrate_lua("if a == b then");

        // Reported, not converted into a setting called `if a`.
        assert_eq!(migration.unconverted.len(), 1);
        assert!(migration.text.trim().is_empty());
    }
}
