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
    // Inside a branch on the target triple: the platform it selects, or None
    // when the branch names one we do not recognise.
    let mut chain: Option<Option<&'static str>> = None;
    let mut nested = 0usize;

    for (index, raw_line) in source.lines().enumerate() {
        let line = index + 1;
        let text = strip_lua_comment(raw_line)
            .trim()
            .trim_end_matches(',')
            .trim();
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

        // A branch on the target triple is the one piece of control flow that
        // is really a declaration: it says "on this platform, use that value".
        // It becomes a `[platform.*]` section rather than being reported.
        if nested == 0 {
            if let Some(branch) = platform_branch(text) {
                chain = Some(branch);
                continue;
            }
            if chain.is_some() {
                if text == "else" {
                    chain = Some(Some("other"));
                    continue;
                }
                if text == "end" {
                    chain = None;
                    continue;
                }
            }
        }

        // Every other block is code. A setting inside one is conditional, and
        // converting it unconditionally would apply it where the user's config
        // never did -- so the whole block is reported instead.
        let delta = block_delta(text);
        let in_code = nested > 0;
        // A line that only closes blocks is structure, not a lost setting.
        let only_closes = delta < 0 && !text.contains('=');

        nested = nested.saturating_add_signed(delta);

        if in_code && only_closes {
            continue;
        }
        if in_code || delta > 0 || (delta == 0 && opens_and_closes_a_block(text)) {
            // `x = function(...)` deserves the specific complaint, whether it
            // closes on this line or runs on for twenty.
            let assigns_function = split_assignment(text)
                .is_some_and(|(_, value)| value.trim_start().starts_with("function"));
            let reason = if assigns_function {
                "is a function; the new config holds values, not code"
            } else {
                "sits inside Lua control flow, so it does not apply unconditionally"
            };
            unconverted.push(Unconverted {
                line,
                snippet: text.to_string(),
                reason: reason.to_string(),
            });
            continue;
        }
        if delta < 0 {
            // A stray `end`; whatever it closed was already reported.
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

        // A branch naming a platform we do not know cannot be turned into a
        // section without guessing which machines it covers.
        if chain == Some(None) {
            unconverted.push(Unconverted {
                line,
                snippet: text.to_string(),
                reason: "inside a platform branch the converter does not recognise".to_string(),
            });
            adjust_frames(&mut frames, opens, closes);
            continue;
        }

        let mut section: Vec<&str> = Vec::new();
        if let Some(Some(platform)) = chain {
            section.push("platform");
            section.push(platform);
        }
        section.extend(
            frames
                .iter()
                .filter_map(|frame| frame.as_deref())
                .filter(|name| !name.is_empty()),
        );
        let full_key = if section.is_empty() {
            key
        } else {
            format!("{}.{}", section.join("."), key)
        };

        if legacy_setting_is_fixed_equivalent(&full_key) {
            adjust_frames(&mut frames, opens, closes);
            continue;
        }

        if full_key.rsplit('.').next() == Some("window_padding") {
            match simple_window_padding(raw_value) {
                Ok(values) => {
                    let prefix = full_key.strip_suffix("window_padding").unwrap_or_default();
                    for (side, value) in values {
                        let output_key = format!("{prefix}window.padding_{side}");
                        if let Some((_, first)) = seen.iter().find(|(name, _)| *name == output_key)
                        {
                            unconverted.push(Unconverted {
                                line,
                                snippet: text.to_string(),
                                reason: format!(
                                    "`{output_key}` is already set on line {first}; pick the one you want"
                                ),
                            });
                        } else {
                            seen.push((output_key.clone(), line));
                            settings.push(Setting {
                                key: output_key,
                                value,
                            });
                        }
                    }
                }
                Err(reason) => unconverted.push(Unconverted {
                    line,
                    snippet: text.to_string(),
                    reason,
                }),
            }
            adjust_frames(&mut frames, opens, closes);
            continue;
        }

        if full_key.rsplit('.').next() == Some("inactive_pane_hsb") {
            match simple_inactive_pane_hsb(raw_value) {
                Ok(values) => {
                    let prefix = full_key
                        .strip_suffix("inactive_pane_hsb")
                        .unwrap_or_default();
                    for (name, value) in values {
                        let output_key = format!("{prefix}inactive_pane.{name}");
                        if let Some((_, first)) =
                            seen.iter().find(|(known, _)| *known == output_key)
                        {
                            unconverted.push(Unconverted {
                                line,
                                snippet: text.to_string(),
                                reason: format!(
                                    "`{output_key}` is already set on line {first}; pick the one you want"
                                ),
                            });
                        } else {
                            seen.push((output_key.clone(), line));
                            settings.push(Setting {
                                key: output_key,
                                value,
                            });
                        }
                    }
                }
                Err(reason) => unconverted.push(Unconverted {
                    line,
                    snippet: text.to_string(),
                    reason,
                }),
            }
            adjust_frames(&mut frames, opens, closes);
            continue;
        }

        let converted = if full_key.rsplit('.').next() == Some("font") {
            match simple_font_family(raw_value) {
                Some(family) => {
                    let prefix = full_key.strip_suffix("font").unwrap_or_default();
                    Ok((format!("{prefix}font_family"), render_string(&family)))
                }
                None => convert_value(raw_value).map(|value| (full_key.clone(), value)),
            }
        } else {
            convert_value(raw_value).map(|value| (canonical_key(&full_key), value))
        };

        match converted {
            Ok((output_key, value)) => {
                if let Some((_, first)) = seen.iter().find(|(name, _)| *name == output_key) {
                    // Platform branches set the same key more than once. Which
                    // one wins is a question only the user can answer, and
                    // emitting both would produce a file that will not parse.
                    unconverted.push(Unconverted {
                        line,
                        snippet: text.to_string(),
                        reason: format!(
                            "`{output_key}` is already set on line {first}; pick the one you want"
                        ),
                    });
                } else {
                    seen.push((output_key.clone(), line));
                    settings.push(Setting {
                        key: output_key,
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

/// Rename settings whose 0.57 Lua spelling became a structured declarative
/// section in next-core. Platform prefixes are retained.
fn canonical_key(key: &str) -> String {
    let (prefix, leaf) = key
        .rsplit_once('.')
        .map(|(prefix, leaf)| (format!("{prefix}."), leaf))
        .unwrap_or_else(|| (String::new(), key));
    let replacement = match leaf {
        "initial_cols" => "window.initial_cols",
        "initial_rows" => "window.initial_rows",
        "window_close_confirmation" => "window.close_confirmation",
        "window_background_opacity" => "window.background_opacity",
        "window_decorations" => "window.decorations",
        "tab_bar_position" => "tab_bar.position",
        "tab_max_width" => "tab_bar.max_width",
        "hide_tab_bar_if_only_one_tab" => "tab_bar.hide_if_only_one_tab",
        "show_tab_index_in_tab_bar" => "tab_bar.show_index",
        "show_new_tab_button_in_tab_bar" => "tab_bar.show_new_tab_button",
        "integrated_title_button_style" => "title_button.style",
        "integrated_title_button_alignment" => "title_button.alignment",
        "integrated_title_buttons" => "title_button.buttons",
        "status_update_interval" => "stats.refresh_ms",
        _ => return key.to_string(),
    };
    format!("{prefix}{replacement}")
}

fn legacy_setting_is_fixed_equivalent(key: &str) -> bool {
    matches!(
        key.rsplit('.').next().unwrap_or(key),
        // next-core has no background updater, always draws its status bar,
        // owns its decorations, and always uses the richer tab strip.
        "check_for_updates"
            | "win32_system_backdrop"
            | "show_unterm_status_bar"
            | "use_fancy_tab_bar"
    )
}

/// How many Lua blocks this line opens, less how many it closes.
///
/// Counting keywords rather than matching on how a line starts or ends is what
/// makes `pcall(function() ... end)` balance: it ends with `end)`, so anything
/// looking at the last characters would treat it as an unclosed block and
/// swallow the whole rest of the file.
fn block_delta(text: &str) -> isize {
    let opens = count_word(text, "function") + count_word(text, "then") + count_word(text, "do");
    let closes = count_word(text, "end");
    // `elseif ... then` continues a block rather than opening one.
    let continues = count_word(text, "elseif");
    opens as isize - closes as isize - continues as isize
}

/// True when a line opens and closes a block by itself, like
/// `if ok then x = 1 end`.
fn opens_and_closes_a_block(text: &str) -> bool {
    count_word(text, "end") > 0
        && (count_word(text, "function") > 0
            || count_word(text, "then") > 0
            || count_word(text, "do") > 0)
}

/// Count whole-word occurrences, so `endpoint` is not an `end`.
fn count_word(text: &str, word: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut start = 0;
    while let Some(offset) = text[start..].find(word) {
        let at = start + offset;
        let end = at + word.len();
        let before_free = at == 0 || !is_word_byte(bytes[at - 1]);
        let after_free = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before_free && after_free {
            count += 1;
        }
        start = end;
    }
    count
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Recognise `if`/`elseif` on the target triple, and which platform it selects.
///
/// Returns the outer Option only for a target-triple branch; the inner one is
/// None when the branch names a platform the converter cannot map, so its body
/// is reported instead of being filed under the wrong machine.
fn platform_branch(text: &str) -> Option<Option<&'static str>> {
    let condition = text
        .strip_prefix("if ")
        .or_else(|| text.strip_prefix("elseif "))?
        .strip_suffix(" then")?;
    if !condition.contains("target_triple") {
        return None;
    }
    let platform = if condition.contains("darwin") || condition.contains("apple") {
        Some("macos")
    } else if condition.contains("windows") {
        Some("windows")
    } else if condition.contains("linux") {
        Some("linux")
    } else {
        None
    };
    Some(platform)
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
        "return config"
            | "return {"
            | "{"
            | "local config = {}"
            | "local wezterm = require 'wezterm'"
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
        if matches!(
            chars.get(index.wrapping_sub(1)),
            Some('=' | '<' | '>' | '~')
        ) {
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

    if let Some(rest) = raw
        .strip_prefix("config[")
        .or_else(|| raw.strip_prefix('['))
    {
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

    if let Some(inner) = raw
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    {
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

/// The overwhelmingly common old spelling for a single font family.
///
/// This call is declarative in practice even though it is expressed through
/// Lua. Recovering the literal family prevents an upgrade from replacing the
/// user's chosen typeface while still refusing computed font expressions.
fn simple_font_family(raw: &str) -> Option<String> {
    let inner = raw
        .trim()
        .strip_prefix("wezterm.font(")?
        .strip_suffix(')')?
        .trim();
    if inner.contains(',') {
        return None;
    }
    lua_string(inner)
}

/// Recover the common one-line WezTerm padding table.
fn simple_window_padding(raw: &str) -> Result<Vec<(&'static str, String)>, String> {
    let inner = raw
        .trim()
        .trim_end_matches(',')
        .trim()
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| "window padding is not a one-line table".to_string())?;
    let mut values = Vec::new();
    for item in inner.split(',') {
        let Some((name, value)) = split_assignment(item.trim()) else {
            continue;
        };
        let side = match name.trim() {
            "left" => "left",
            "right" => "right",
            "top" => "top",
            "bottom" => "bottom",
            other => return Err(format!("unknown window padding side `{other}`")),
        };
        let converted = convert_value(value)?;
        if converted.parse::<f64>().is_err() {
            return Err(format!("window padding `{side}` is not numeric"));
        }
        values.push((side, converted));
    }
    if values.is_empty() {
        return Err("window padding table has no numeric sides".to_string());
    }
    Ok(values)
}

fn simple_inactive_pane_hsb(raw: &str) -> Result<Vec<(&'static str, String)>, String> {
    let inner = raw
        .trim()
        .trim_end_matches(',')
        .trim()
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| "inactive pane HSB is not a one-line table".to_string())?;
    let mut values = Vec::new();
    for item in inner.split(',') {
        let Some((name, value)) = split_assignment(item.trim()) else {
            continue;
        };
        let name = match name.trim() {
            "brightness" => "brightness",
            "saturation" => "saturation",
            // Hue is not currently exposed because the old product never
            // shipped a non-identity value for it.
            "hue" if value.trim().parse::<f64>().ok() == Some(1.0) => continue,
            other => return Err(format!("unsupported inactive pane HSB field `{other}`")),
        };
        let converted = convert_value(value)?;
        if converted.parse::<f64>().is_err() {
            return Err(format!("inactive pane `{name}` is not numeric"));
        }
        values.push((name, converted));
    }
    if values.is_empty() {
        return Err("inactive pane HSB table has no supported numeric fields".to_string());
    }
    Ok(values)
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
    fn a_literal_wezterm_font_keeps_the_users_family() {
        let migration = migrate_lua("config.font = wezterm.font('Cascadia Code')");
        let parsed = config::parse(&migration.text).expect("output should parse");

        assert_eq!(parsed.str_of("font_family").unwrap(), Some("Cascadia Code"));
        assert!(
            migration.unconverted.is_empty(),
            "{:?}",
            migration.unconverted
        );
    }

    #[test]
    fn a_computed_wezterm_font_is_still_reported() {
        let migration = migrate_lua("config.font = wezterm.font(family_name)");

        assert!(migration.text.trim().is_empty());
        assert_eq!(migration.unconverted.len(), 1);
    }

    #[test]
    fn legacy_window_and_tab_keys_use_the_next_core_schema() {
        let migration = migrate_lua(
            r#"
config.initial_cols = 120
config.initial_rows = 30
config.window_close_confirmation = 'NeverPrompt'
config.window_background_opacity = 0.9
config.tab_bar_position = 'Left'
config.tab_max_width = 32
config.hide_tab_bar_if_only_one_tab = false
config.show_tab_index_in_tab_bar = true
config.show_new_tab_button_in_tab_bar = true
config.status_update_interval = 2000
"#,
        );
        let parsed = config::parse(&migration.text).expect("output should parse");

        assert_eq!(parsed.int_of("window.initial_cols").unwrap(), Some(120));
        assert_eq!(parsed.int_of("window.initial_rows").unwrap(), Some(30));
        assert_eq!(
            parsed.str_of("window.close_confirmation").unwrap(),
            Some("NeverPrompt")
        );
        assert_eq!(
            parsed.float_of("window.background_opacity").unwrap(),
            Some(0.9)
        );
        assert_eq!(parsed.str_of("tab_bar.position").unwrap(), Some("Left"));
        assert_eq!(parsed.int_of("tab_bar.max_width").unwrap(), Some(32));
        assert_eq!(
            parsed.bool_of("tab_bar.hide_if_only_one_tab").unwrap(),
            Some(false)
        );
        assert_eq!(parsed.bool_of("tab_bar.show_index").unwrap(), Some(true));
        assert_eq!(
            parsed.bool_of("tab_bar.show_new_tab_button").unwrap(),
            Some(true)
        );
        assert_eq!(parsed.int_of("stats.refresh_ms").unwrap(), Some(2000));
    }

    #[test]
    fn one_line_window_padding_keeps_each_side() {
        let migration = migrate_lua("config.window_padding = { left=4, right=5, top=6, bottom=7 }");
        let parsed = config::parse(&migration.text).expect("output should parse");

        assert_eq!(parsed.int_of("window.padding_left").unwrap(), Some(4));
        assert_eq!(parsed.int_of("window.padding_right").unwrap(), Some(5));
        assert_eq!(parsed.int_of("window.padding_top").unwrap(), Some(6));
        assert_eq!(parsed.int_of("window.padding_bottom").unwrap(), Some(7));
        assert!(
            migration.unconverted.is_empty(),
            "{:?}",
            migration.unconverted
        );
    }

    #[test]
    fn one_line_inactive_pane_hsb_keeps_the_visible_transform() {
        let migration =
            migrate_lua("config.inactive_pane_hsb = { brightness=0.55, saturation=0.8 }");
        let parsed = config::parse(&migration.text).expect("output should parse");

        assert_eq!(
            parsed.float_of("inactive_pane.brightness").unwrap(),
            Some(0.55)
        );
        assert_eq!(
            parsed.float_of("inactive_pane.saturation").unwrap(),
            Some(0.8)
        );
        assert!(
            migration.unconverted.is_empty(),
            "{:?}",
            migration.unconverted
        );
    }

    #[test]
    fn fixed_equivalent_legacy_switches_do_not_become_dead_settings() {
        let migration = migrate_lua(
            r#"
config.check_for_updates = false
config.win32_system_backdrop = 'Disable'
config.show_unterm_status_bar = true
config.use_fancy_tab_bar = true
"#,
        );

        assert!(migration.text.trim().is_empty());
        assert!(migration.unconverted.is_empty());
    }

    #[test]
    fn platform_title_button_keys_keep_their_platform_prefix() {
        let migration = migrate_lua(
            r#"
if wezterm.target_triple:find('windows') then
  config.integrated_title_button_style = 'Windows'
  config.integrated_title_button_alignment = 'Right'
end
"#,
        );
        let parsed = config::parse(&migration.text).expect("output should parse");

        assert_eq!(
            parsed
                .str_of("platform.windows.title_button.style")
                .unwrap(),
            Some("Windows")
        );
        assert_eq!(
            parsed
                .str_of("platform.windows.title_button.alignment")
                .unwrap(),
            Some("Right")
        );
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
        assert_eq!(
            parsed.float_of("window_frame.font_size").unwrap(),
            Some(12.0)
        );

        // The call argument must not become settings, or the file grows keys
        // the user never wrote.
        assert!(parsed.get("family").is_none());
        assert!(!parsed.keys().any(|key| key.contains("font_with_fallback")));

        // The platform branch becomes two sections rather than one key set
        // twice, so both machines keep the value the user wrote for them.
        assert_eq!(
            parsed
                .resolve_platform("macos")
                .str_of("title_button.style")
                .unwrap(),
            Some("MacOsCustom")
        );
        assert_eq!(
            parsed
                .resolve_platform("windows")
                .str_of("title_button.style")
                .unwrap(),
            Some("Windows")
        );

        // A value that came from a local cannot be resolved without running
        // the file, so it is reported rather than guessed at.
        assert!(migration
            .unconverted
            .iter()
            .any(|item| item.snippet.contains("active_titlebar_bg")));
    }

    #[test]
    fn a_target_triple_branch_becomes_platform_sections() {
        let migration = migrate_and_check(
            r#"
if wezterm.target_triple:find('darwin') then
  config.shell = '/bin/zsh'
elseif wezterm.target_triple:find('windows') then
  config.shell = 'powershell.exe'
else
  config.shell = '/bin/bash'
end
"#,
        )
        .expect("converted config should parse");

        // The one piece of control flow that is really a declaration.
        assert!(migration.is_complete(), "{:?}", migration.unconverted);
        let parsed = config::parse(&migration.text).unwrap();
        assert_eq!(
            parsed.resolve_platform("macos").str_of("shell").unwrap(),
            Some("/bin/zsh")
        );
        assert_eq!(
            parsed.resolve_platform("windows").str_of("shell").unwrap(),
            Some("powershell.exe")
        );
        // The `else` arm covers every machine the named arms did not.
        assert_eq!(
            parsed.resolve_platform("linux").str_of("shell").unwrap(),
            Some("/bin/bash")
        );
    }

    #[test]
    fn an_unrecognised_platform_branch_is_reported_not_filed_wrongly() {
        let migration = migrate_lua(
            "if wezterm.target_triple:find('freebsd') then\n  config.shell = '/bin/sh'\nend",
        );

        // Guessing which machines this covers would put a setting on the wrong
        // ones, which is worse than saying so.
        assert_eq!(migration.unconverted.len(), 1);
        assert!(
            migration.unconverted[0].reason.contains("platform branch"),
            "{}",
            migration.unconverted[0].reason
        );
    }

    #[test]
    fn a_value_chosen_by_a_probe_is_never_converted_to_one_branch() {
        // Straight out of the config this project ships: prefer pwsh if it is
        // installed, otherwise fall back.
        let migration = migrate_lua(
            r#"
local f = io.open(pwsh, 'r')
if f then
  config.default_prog = { pwsh, '-NoLogo' }
else
  config.default_prog = { 'powershell.exe', '-NoLogo' }
end
"#,
        );

        // Picking either branch would hand the user a setting they never wrote
        // -- the fallback silently becoming the only choice is exactly the kind
        // of quiet wrong value this converter must not produce.
        let parsed = config::parse(&migration.text).unwrap();
        assert!(parsed.get("default_prog").is_none());
        assert_eq!(
            migration
                .unconverted
                .iter()
                .filter(|item| item.snippet.contains("default_prog"))
                .count(),
            2
        );
    }

    #[test]
    fn a_function_call_that_closes_on_the_same_line_does_not_swallow_the_file() {
        let migration = migrate_lua(
            r#"
local ok = pcall(function() return wezterm.color.parse('#fff') end)
config.font_size = 13
"#,
        );

        // This line ends with `end)`, so anything matching on the last
        // characters would treat the block as open and lose everything after.
        let parsed = config::parse(&migration.text).unwrap();
        assert_eq!(parsed.int_of("font_size").unwrap(), Some(13));
    }

    #[test]
    fn an_ordinary_branch_is_still_not_control_flow_we_understand() {
        let migration = migrate_lua("if some_condition then\n  config.shell = '/bin/sh'\nend");

        // Only a branch on the target triple is a declaration in disguise.
        assert!(!migration.is_complete());
        assert!(migration.text.trim().is_empty());
    }

    #[test]
    fn a_comparison_is_not_mistaken_for_an_assignment() {
        let migration = migrate_lua("if a == b then");

        // Reported, not converted into a setting called `if a`.
        assert_eq!(migration.unconverted.len(), 1);
        assert!(migration.text.trim().is_empty());
    }
}

#[cfg(test)]
mod shipped_default_tests {
    use super::*;

    /// The config we ship converts, and what it converts to is valid.
    ///
    /// The installer used to place a Lua file that the terminal no longer
    /// reads. Converting it is how the out-of-box defaults survive the format
    /// change -- and a conversion that produces a config the parser rejects
    /// would ship a terminal that starts on nothing.
    #[test]
    fn the_shipped_default_config_converts_and_parses() {
        let Ok(source) = std::fs::read_to_string("../assets/unterm.lua") else {
            eprintln!("no shipped config next to this crate; skipping");
            return;
        };
        let migration = migrate_lua(&source);
        assert!(
            !migration.text.trim().is_empty(),
            "the shipped config converted to nothing"
        );
        let parsed = crate::next_core::config::parse(&migration.text);
        assert!(
            parsed.is_ok(),
            "the converted default does not parse: {:?}",
            parsed.err()
        );
        eprintln!(
            "shipped default: {} lines out, {} settings left behind",
            migration.text.lines().count(),
            migration.unconverted.len()
        );
    }
}
