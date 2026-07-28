//! Configuration: a declarative file, not a program.
//!
//! The engine reads settings; it does not execute the user's config. That is
//! the whole difference from the Lua runtime this replaces, and it buys three
//! things: a config can never hang or crash startup, it can be validated
//! without running it, and a tool can rewrite it safely.
//!
//! The format is line-oriented `key = value` with `[section]` headers, because
//! that is what a terminal config actually is. Deliberate choices, each one a
//! complaint this avoids:
//!
//! - A typo is an error, not silence. `font_szie = 12` doing nothing while the
//!   user stares at an unchanged window is the single worst config failure, so
//!   unknown keys are rejected and the nearest known key is suggested.
//! - A duplicate key is an error. Last-wins means editing the top of your file
//!   has no effect and nothing says why.
//! - Every error carries a line number. An error without one sends the user
//!   hunting.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
}

impl Value {
    /// What to call this in an error message.
    fn type_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "boolean",
            Value::Int(_) => "integer",
            Value::Float(_) => "number",
            Value::Str(_) => "string",
            Value::List(_) => "list",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    /// 1-based, as an editor counts. An error the user cannot locate is barely
    /// better than no error.
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ConfigError {}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    entries: BTreeMap<String, (usize, Value)>,
}

impl Config {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key).map(|(_, value)| value)
    }

    /// The line a key was set on, for errors raised after parsing.
    pub fn line_of(&self, key: &str) -> Option<usize> {
        self.entries.get(key).map(|(line, _)| *line)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(|key| key.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn bool_of(&self, key: &str) -> Result<Option<bool>, ConfigError> {
        match self.entries.get(key) {
            None => Ok(None),
            Some((_, Value::Bool(value))) => Ok(Some(*value)),
            Some((line, other)) => Err(mismatch(*line, key, "boolean", other)),
        }
    }

    pub fn int_of(&self, key: &str) -> Result<Option<i64>, ConfigError> {
        match self.entries.get(key) {
            None => Ok(None),
            Some((_, Value::Int(value))) => Ok(Some(*value)),
            Some((line, other)) => Err(mismatch(*line, key, "integer", other)),
        }
    }

    pub fn float_of(&self, key: &str) -> Result<Option<f64>, ConfigError> {
        match self.entries.get(key) {
            None => Ok(None),
            // An integer where a number is wanted is not a mistake worth
            // reporting: `font_size = 12` means 12.0.
            Some((_, Value::Int(value))) => Ok(Some(*value as f64)),
            Some((_, Value::Float(value))) => Ok(Some(*value)),
            Some((line, other)) => Err(mismatch(*line, key, "number", other)),
        }
    }

    pub fn str_of(&self, key: &str) -> Result<Option<&str>, ConfigError> {
        match self.entries.get(key) {
            None => Ok(None),
            Some((_, Value::Str(value))) => Ok(Some(value.as_str())),
            Some((line, other)) => Err(mismatch(*line, key, "string", other)),
        }
    }

    pub fn list_of(&self, key: &str) -> Result<Option<&[Value]>, ConfigError> {
        match self.entries.get(key) {
            None => Ok(None),
            Some((_, Value::List(values))) => Ok(Some(values.as_slice())),
            Some((line, other)) => Err(mismatch(*line, key, "list", other)),
        }
    }

    /// Fold the `[platform.*]` sections down for one platform.
    ///
    /// A config has to say different things on different machines -- a shell,
    /// a title button style -- and in Lua that was a branch on the target
    /// triple. Declaring it instead keeps the file readable on a machine it is
    /// not for, which a branch never did.
    ///
    /// Later wins: base, then `[platform.other]`, then the named platform.
    pub fn resolve_platform(&self, platform: &str) -> Config {
        let specific = format!("platform.{platform}.");
        let mut resolved = Config::default();

        for (key, (line, value)) in &self.entries {
            if !key.starts_with("platform.") {
                resolved.entries.insert(key.clone(), (*line, value.clone()));
            }
        }
        for prefix in ["platform.other.", specific.as_str()] {
            for (key, (line, value)) in &self.entries {
                if let Some(name) = key.strip_prefix(prefix) {
                    resolved
                        .entries
                        .insert(name.to_string(), (*line, value.clone()));
                }
            }
        }
        resolved
    }

    /// Reject keys the engine does not know, naming the closest one it does.
    ///
    /// This runs separately from parsing so a caller can parse a config whose
    /// schema it does not have -- the migration tool does exactly that.
    pub fn validate_keys(&self, known: &[&str]) -> Vec<ConfigError> {
        let mut errors = Vec::new();
        for (key, (line, _)) in &self.entries {
            if known.contains(&key.as_str()) {
                continue;
            }
            let message = match nearest_key(key, known) {
                Some(suggestion) => {
                    format!("unknown setting `{key}` -- did you mean `{suggestion}`?")
                }
                None => format!("unknown setting `{key}`"),
            };
            errors.push(ConfigError {
                line: *line,
                message,
            });
        }
        errors
    }
}

/// The platform name `[platform.*]` sections are matched against.
pub fn current_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn mismatch(line: usize, key: &str, wanted: &str, got: &Value) -> ConfigError {
    ConfigError {
        line,
        message: format!(
            "`{key}` should be a {wanted}, but is a {}",
            got.type_name()
        ),
    }
}

/// Closest known key by edit distance, if one is close enough to be a likely
/// typo rather than a coincidence.
fn nearest_key(key: &str, known: &[&str]) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for candidate in known {
        let distance = edit_distance(key, candidate);
        if best.map_or(true, |(best_distance, _)| distance < best_distance) {
            best = Some((distance, candidate));
        }
    }
    let (distance, candidate) = best?;
    // A third of the key length, so short keys need a close match and long
    // ones tolerate a couple of slips. Suggesting something unrelated is worse
    // than suggesting nothing.
    let tolerance = (key.chars().count() / 3).max(2);
    (distance <= tolerance).then(|| candidate.to_string())
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];

    for (i, left_char) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right_chars.iter().enumerate() {
            let substitution = previous[j] + usize::from(left_char != *right_char);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right_chars.len()]
}

/// Parse a config file.
///
/// Returns every error found, not just the first: fixing a config one error per
/// run is a miserable loop.
pub fn parse(source: &str) -> Result<Config, Vec<ConfigError>> {
    let mut config = Config::default();
    let mut errors = Vec::new();
    let mut section = String::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line = index + 1;
        let text = strip_comment(raw_line).trim();
        if text.is_empty() {
            continue;
        }

        if let Some(rest) = text.strip_prefix('[') {
            match rest.strip_suffix(']') {
                Some(name) if !name.trim().is_empty() => {
                    section = format!("{}.", name.trim());
                }
                _ => errors.push(ConfigError {
                    line,
                    message: "section header is missing its closing `]`".to_string(),
                }),
            }
            continue;
        }

        let Some((key, value_text)) = text.split_once('=') else {
            errors.push(ConfigError {
                line,
                message: format!("expected `key = value`, found `{text}`"),
            });
            continue;
        };

        let key = key.trim();
        if key.is_empty() {
            errors.push(ConfigError {
                line,
                message: "setting name is empty".to_string(),
            });
            continue;
        }
        let key = format!("{section}{key}");

        match parse_value(value_text.trim()) {
            Ok(value) => {
                if let Some((first, _)) = config.entries.get(&key) {
                    // Last-wins would mean an edit at the top of the file does
                    // nothing and nothing says why.
                    errors.push(ConfigError {
                        line,
                        message: format!("`{key}` is already set on line {first}"),
                    });
                    continue;
                }
                config.entries.insert(key, (line, value));
            }
            Err(message) => errors.push(ConfigError { line, message }),
        }
    }

    if errors.is_empty() {
        Ok(config)
    } else {
        Err(errors)
    }
}

/// Drop a trailing `#` comment, honouring quotes so a `#` inside a string
/// survives -- colours are written `"#1e1e2e"` and truncating one would turn a
/// valid config into a confusing parse error.
fn strip_comment(line: &str) -> &str {
    let mut in_quotes = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes => return &line[..index],
            _ => {}
        }
    }
    line
}

fn parse_value(text: &str) -> Result<Value, String> {
    if text.is_empty() {
        return Err("value is missing".to_string());
    }
    if text == "true" {
        return Ok(Value::Bool(true));
    }
    if text == "false" {
        return Ok(Value::Bool(false));
    }
    if let Some(rest) = text.strip_prefix('[') {
        let inner = rest
            .strip_suffix(']')
            .ok_or_else(|| "list is missing its closing `]`".to_string())?;
        let inner = inner.trim();
        if inner.is_empty() {
            return Ok(Value::List(Vec::new()));
        }
        let mut values = Vec::new();
        for item in split_list(inner) {
            values.push(parse_value(item.trim())?);
        }
        return Ok(Value::List(values));
    }
    if text.starts_with('"') {
        return parse_string(text).map(Value::Str);
    }
    if let Ok(value) = text.parse::<i64>() {
        return Ok(Value::Int(value));
    }
    if let Ok(value) = text.parse::<f64>() {
        return Ok(Value::Float(value));
    }
    Err(format!(
        "`{text}` is not a value -- strings need quotes, as in `\"{text}\"`"
    ))
}

/// Split on commas that are not inside a string or a nested list.
fn split_list(inner: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut in_quotes = false;
    let mut escaped = false;
    let mut start = 0usize;

    for (index, ch) in inner.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            '[' if !in_quotes => depth += 1,
            ']' if !in_quotes => depth = depth.saturating_sub(1),
            ',' if !in_quotes && depth == 0 => {
                parts.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&inner[start..]);
    parts
}

fn parse_string(text: &str) -> Result<String, String> {
    let mut chars = text.chars();
    chars.next();

    let mut out = String::new();
    let mut closed = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                closed = true;
                break;
            }
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                // A Windows path is the common case here, and demanding
                // doubled backslashes in one would be a papercut on every
                // line. An unknown escape keeps both characters.
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => return Err("string ends with a dangling `\\`".to_string()),
            },
            other => out.push(other),
        }
    }
    if !closed {
        return Err("string is missing its closing quote".to_string());
    }
    let trailing: String = chars.collect();
    if !trailing.trim().is_empty() {
        return Err(format!("unexpected `{}` after the string", trailing.trim()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> Config {
        parse(source).expect("config should parse")
    }

    fn parse_err(source: &str) -> Vec<ConfigError> {
        parse(source).expect_err("config should not parse")
    }

    #[test]
    fn reads_the_basic_value_types() {
        let config = parse_ok(
            r#"
            font_size = 12.5
            scrollback_lines = 10000
            use_ime = true
            font_family = "Cascadia Mono"
            "#,
        );

        assert_eq!(config.float_of("font_size").unwrap(), Some(12.5));
        assert_eq!(config.int_of("scrollback_lines").unwrap(), Some(10000));
        assert_eq!(config.bool_of("use_ime").unwrap(), Some(true));
        assert_eq!(
            config.str_of("font_family").unwrap(),
            Some("Cascadia Mono")
        );
    }

    #[test]
    fn an_integer_is_accepted_where_a_number_is_wanted() {
        let config = parse_ok("font_size = 12");

        // `font_size = 12` obviously means 12.0; rejecting it would be pedantry.
        assert_eq!(config.float_of("font_size").unwrap(), Some(12.0));
    }

    #[test]
    fn a_missing_setting_is_absent_not_an_error() {
        let config = parse_ok("font_size = 12");

        // The caller supplies the default; the config layer does not invent one.
        assert_eq!(config.int_of("scrollback_lines").unwrap(), None);
    }

    #[test]
    fn sections_prefix_their_keys() {
        let config = parse_ok(
            r##"
            [colors]
            background = "#1e1e2e"
            [window]
            background = "#000000"
            "##,
        );

        assert_eq!(config.str_of("colors.background").unwrap(), Some("#1e1e2e"));
        assert_eq!(config.str_of("window.background").unwrap(), Some("#000000"));
    }

    #[test]
    fn a_hash_inside_a_string_is_not_a_comment() {
        let config = parse_ok(r##"background = "#1e1e2e" # the real comment"##);

        // Colours are written with a leading `#`; truncating there would turn a
        // valid config into a baffling parse error.
        assert_eq!(config.str_of("background").unwrap(), Some("#1e1e2e"));
    }

    #[test]
    fn reads_lists_including_nested_ones() {
        let config = parse_ok(r#"fonts = ["Cascadia Mono", "Noto Sans", 3, [true]]"#);

        let values = config.list_of("fonts").unwrap().unwrap();
        assert_eq!(values.len(), 4);
        assert_eq!(values[0], Value::Str("Cascadia Mono".to_string()));
        assert_eq!(values[2], Value::Int(3));
        assert_eq!(values[3], Value::List(vec![Value::Bool(true)]));
    }

    #[test]
    fn a_comma_inside_a_string_does_not_split_a_list() {
        let config = parse_ok(r#"labels = ["one, two", "three"]"#);

        let values = config.list_of("labels").unwrap().unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::Str("one, two".to_string()));
    }

    #[test]
    fn a_windows_path_does_not_need_doubled_backslashes() {
        let config = parse_ok(r#"shell = "C:\Users\me\shell.exe""#);

        // Demanding `\\` on every path would be a papercut on the platform
        // where paths are most common.
        assert_eq!(
            config.str_of("shell").unwrap(),
            Some(r"C:\Users\me\shell.exe")
        );
    }

    #[test]
    fn known_escapes_still_work() {
        let config = parse_ok(r#"label = "a\tb\nc\"d""#);

        assert_eq!(config.str_of("label").unwrap(), Some("a\tb\nc\"d"));
    }

    #[test]
    fn a_typo_is_rejected_with_the_nearest_real_setting() {
        let config = parse_ok("font_szie = 12");

        let errors = config.validate_keys(&["font_size", "font_family"]);

        // Silently ignoring this is the worst config failure there is: the user
        // edits, nothing changes, and nothing explains why.
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 1);
        assert!(
            errors[0].message.contains("did you mean `font_size`"),
            "{}",
            errors[0].message
        );
    }

    #[test]
    fn an_unrelated_key_is_rejected_without_a_wild_guess() {
        let config = parse_ok("completely_unrelated_setting = 1");

        let errors = config.validate_keys(&["font_size"]);

        assert_eq!(errors.len(), 1);
        // Suggesting something unrelated is worse than suggesting nothing.
        assert!(!errors[0].message.contains("did you mean"), "{}", errors[0].message);
    }

    #[test]
    fn a_known_key_passes_validation() {
        let config = parse_ok("font_size = 12\n[colors]\nbackground = \"#000\"");

        assert!(config
            .validate_keys(&["font_size", "colors.background"])
            .is_empty());
    }

    #[test]
    fn setting_a_key_twice_is_an_error_naming_both_lines() {
        let errors = parse_err("font_size = 12\nfont_size = 14");

        // Last-wins means editing the top of the file silently does nothing.
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 2);
        assert!(errors[0].message.contains("line 1"), "{}", errors[0].message);
    }

    #[test]
    fn every_error_is_reported_not_just_the_first() {
        let errors = parse_err("font_size = \nnonsense line\nfamily = unquoted");

        // Fixing a config one error per run is a miserable loop.
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].line, 1);
        assert_eq!(errors[1].line, 2);
        assert_eq!(errors[2].line, 3);
    }

    #[test]
    fn an_unquoted_string_says_how_to_fix_it() {
        let errors = parse_err("font_family = Cascadia");

        assert!(
            errors[0].message.contains("\"Cascadia\""),
            "{}",
            errors[0].message
        );
    }

    #[test]
    fn a_wrong_type_names_the_key_and_both_types() {
        let config = parse_ok("font_size = \"big\"");

        let error = config.float_of("font_size").unwrap_err();

        assert_eq!(error.line, 1);
        assert!(error.message.contains("font_size"), "{}", error.message);
        assert!(error.message.contains("number"), "{}", error.message);
        assert!(error.message.contains("string"), "{}", error.message);
    }

    #[test]
    fn unterminated_constructs_are_errors_not_silent_truncation() {
        assert_eq!(parse_err(r#"a = "unterminated"#).len(), 1);
        assert_eq!(parse_err("a = [1, 2").len(), 1);
        assert_eq!(parse_err("[section").len(), 1);
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let config = parse_ok("# a comment\n\n   \nfont_size = 12  # trailing\n");

        assert_eq!(config.keys().collect::<Vec<_>>(), vec!["font_size"]);
    }

    #[test]
    fn an_empty_config_is_valid() {
        let config = parse_ok("# nothing but a comment\n");

        assert!(config.is_empty());
        assert!(config.validate_keys(&["font_size"]).is_empty());
    }

    #[test]
    fn a_platform_section_overrides_the_base_value() {
        let config = parse_ok(
            r#"
            shell = "/bin/sh"
            [platform.windows]
            shell = "powershell.exe"
            "#,
        );

        let windows = config.resolve_platform("windows");
        assert_eq!(windows.str_of("shell").unwrap(), Some("powershell.exe"));

        // The section for another machine is inert, not an error -- a config
        // has to stay readable on a machine it is not for.
        let linux = config.resolve_platform("linux");
        assert_eq!(linux.str_of("shell").unwrap(), Some("/bin/sh"));
    }

    #[test]
    fn the_named_platform_beats_the_catch_all() {
        let config = parse_ok(
            r#"
            [platform.other]
            style = "Gnome"
            [platform.windows]
            style = "Windows"
            "#,
        );

        assert_eq!(
            config.resolve_platform("windows").str_of("style").unwrap(),
            Some("Windows")
        );
        assert_eq!(
            config.resolve_platform("macos").str_of("style").unwrap(),
            Some("Gnome")
        );
    }

    #[test]
    fn resolving_leaves_no_platform_keys_behind() {
        let config = parse_ok("[platform.windows]\nshell = \"powershell.exe\"");

        let resolved = config.resolve_platform("windows");

        // Anything still prefixed would fail validation as an unknown setting.
        assert!(!resolved.keys().any(|key| key.starts_with("platform.")));
        assert!(resolved.validate_keys(&["shell"]).is_empty());
    }

    #[test]
    fn a_typo_inside_a_platform_section_is_still_caught() {
        let config = parse_ok("[platform.windows]\nshel = \"powershell.exe\"");

        let errors = config.resolve_platform("windows").validate_keys(&["shell"]);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("did you mean `shell`"), "{}", errors[0].message);
    }

    #[test]
    fn errors_render_with_their_line() {
        let error = ConfigError {
            line: 7,
            message: "boom".to_string(),
        };

        assert_eq!(error.to_string(), "line 7: boom");
    }
}
