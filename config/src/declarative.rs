//! Load a config without running it.
//!
//! The Lua path builds a `Config` by executing the user's file and converting
//! the table it returns. This one parses a declarative file straight into the
//! same dynamic value the converter consumes, so no Lua context is created and
//! nothing in the config can hang, crash, or read the disk on startup.
//!
//! The mapping below is the one place two naming schemes meet: the declarative
//! file groups settings into sections, the `Config` struct is flat. Every entry
//! is checked against a real field by a test, because a mapping that names a
//! field which does not exist would silently drop the setting -- the failure
//! this whole format exists to prevent.

use crate::Config;
use anyhow::{anyhow, Context};
use std::path::Path;
use unterm_engine::next_core::config as declarative;
use unterm_engine::next_core::config_schema;
use wezterm_dynamic::{FromDynamic, Object, ToDynamic, Value};

/// Declarative key on the left, `Config` field on the right.
///
/// Settings absent from this table are next-core's own -- tab titles, font
/// fallback, PATH additions -- and are read from the parsed config directly
/// rather than routed through `Config`.
const FIELD_MAP: &[(&str, &str)] = &[
    ("font_size", "font_size"),
    ("line_height", "line_height"),
    ("color_scheme", "color_scheme"),
    ("scrollback_lines", "scrollback_lines"),
    ("enable_scroll_bar", "enable_scroll_bar"),
    ("shell", "default_prog"),
    ("window.background_opacity", "window_background_opacity"),
    ("window.decorations", "window_decorations"),
    ("window.initial_cols", "initial_cols"),
    ("window.initial_rows", "initial_rows"),
    ("window.close_confirmation", "window_close_confirmation"),
    ("tab_bar.max_width", "tab_max_width"),
    ("tab_bar.hide_if_only_one_tab", "hide_tab_bar_if_only_one_tab"),
    ("tab_bar.show_index", "show_tab_index_in_tab_bar"),
    ("tab_bar.show_new_tab_button", "show_new_tab_button_in_tab_bar"),
    ("title_button.style", "integrated_title_button_style"),
    ("title_button.alignment", "integrated_title_button_alignment"),
    ("title_button.buttons", "integrated_title_buttons"),
    ("tab_bar.position", "tab_bar_position"),
    ("tab_bar.title_format", "tab_title_format"),
    ("tab_bar.fallback_title", "tab_title_fallback"),
    ("tab_bar.strip_extension", "tab_title_strip_extension"),
    ("tab_bar.capitalize", "tab_title_capitalize"),
];

/// Read and validate a declarative config, returning it as a `Config`.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    from_source(&source, &path.display().to_string())
}

/// Parse, validate and convert, reporting every problem with its line.
pub fn from_source(source: &str, name: &str) -> anyhow::Result<Config> {
    let parsed = declarative::parse(source).map_err(|errors| report(name, &errors))?;
    let resolved = parsed.resolve_platform(declarative::current_platform());

    let errors = config_schema::check(&resolved);
    if !errors.is_empty() {
        return Err(report(name, &errors));
    }

    let mut object = Object::default();
    for (declared, field) in FIELD_MAP {
        let Some(value) = resolved.get(declared) else {
            continue;
        };
        object.insert(
            Value::String((*field).to_string()),
            to_dynamic(value, declared)?,
        );
    }

    // The font is a family plus an ordered fallback list, which `Config` holds
    // as one list of font attributes. Leaving these unmapped would mean a user
    // writing `font_family` and seeing nothing happen -- the exact failure this
    // format exists to prevent, so they are wired here rather than dropped.
    let mut families: Vec<Value> = Vec::new();
    if let Ok(Some(family)) = resolved.str_of("font_family") {
        families.push(font_attributes(family, false));
    }
    if let Ok(Some(fallbacks)) = resolved.list_of("font_fallback") {
        for fallback in fallbacks {
            if let declarative::Value::Str(family) = fallback {
                families.push(font_attributes(family, true));
            }
        }
    }
    if !families.is_empty() {
        let mut style = Object::default();
        style.insert(
            Value::String("font".to_string()),
            Value::Array(families.into()),
        );
        object.insert(
            Value::String("font".to_string()),
            Value::Object(style),
        );
    }

    // Bindings are validated here rather than by the schema: a chord is not a
    // key name, and a misspelled one deserves to be told apart from a
    // misspelled setting.
    let (bindings, binding_errors) = crate::keybinding::bindings_from(&resolved);
    if !binding_errors.is_empty() {
        return Err(report(name, &binding_errors));
    }
    if !bindings.is_empty() {
        object.insert(
            Value::String("keys".to_string()),
            Value::Array(
                bindings
                    .iter()
                    .map(|binding| binding.to_dynamic())
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
    }

    // `shell` is one program, but `default_prog` is a command line.
    if let Some(Value::String(program)) = object
        .get(&Value::String("default_prog".to_string()))
        .cloned()
    {
        object.insert(
            Value::String("default_prog".to_string()),
            Value::Array(vec![Value::String(program)].into()),
        );
    }

    Config::from_dynamic(&Value::Object(object), Default::default())
        .with_context(|| format!("converting {name}"))
}

fn font_attributes(family: &str, is_fallback: bool) -> Value {
    let mut attributes = Object::default();
    attributes.insert(
        Value::String("family".to_string()),
        Value::String(family.to_string()),
    );
    // Entries after the first really are fallbacks, and the field has no
    // default, so saying so is both required and correct.
    attributes.insert(
        Value::String("is_fallback".to_string()),
        Value::Bool(is_fallback),
    );
    attributes.insert(
        Value::String("is_synthetic".to_string()),
        Value::Bool(false),
    );
    Value::Object(attributes)
}

fn report(name: &str, errors: &[declarative::ConfigError]) -> anyhow::Error {
    let detail = errors
        .iter()
        .map(|error| format!("  {error}"))
        .collect::<Vec<_>>()
        .join("\n");
    anyhow!("{name} has {} problem(s):\n{detail}", errors.len())
}

/// Convert a declarative value to the dynamic one `Config` is built from.
pub(crate) fn to_dynamic_value(value: &declarative::Value) -> Value {
    match value {
        declarative::Value::Bool(value) => Value::Bool(*value),
        declarative::Value::Int(value) => Value::I64(*value),
        declarative::Value::Float(value) => Value::F64((*value).into()),
        declarative::Value::Str(value) => Value::String(value.clone()),
        declarative::Value::List(values) => {
            Value::Array(values.iter().map(to_dynamic_value).collect::<Vec<_>>().into())
        }
    }
}

fn to_dynamic(value: &declarative::Value, key: &str) -> anyhow::Result<Value> {
    Ok(match value {
        declarative::Value::Bool(value) => Value::Bool(*value),
        declarative::Value::Int(value) => Value::I64(*value),
        declarative::Value::Float(value) => Value::F64((*value).into()),
        declarative::Value::Str(value) => Value::String(value.clone()),
        declarative::Value::List(values) => Value::Array(
            values
                .iter()
                .map(|item| to_dynamic(item, key))
                .collect::<anyhow::Result<Vec<_>>>()?
                .into(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mapped_field_exists_on_the_config_struct() {
        // A mapping naming a field that does not exist would drop the setting
        // without a word, which is the failure this format exists to prevent.
        let source = include_str!("config.rs");
        for (declared, field) in FIELD_MAP {
            assert!(
                source.contains(&format!("pub {field}:")),
                "`{}` maps to `{}`, which is not a Config field",
                declared,
                field
            );
        }
    }

    #[test]
    fn a_declarative_config_becomes_a_config_without_lua() {
        let config = from_source(
            r#"
            font_size = 13
            scrollback_lines = 100000
            enable_scroll_bar = true
            [window]
            initial_cols = 120
            initial_rows = 30
            "#,
            "test",
        )
        .expect("should load");

        assert_eq!(config.font_size, 13.0);
        assert_eq!(config.scrollback_lines, 100000);
        assert!(config.enable_scroll_bar);
        assert_eq!(config.initial_cols, 120);
        assert_eq!(config.initial_rows, 30);
    }

    #[test]
    fn a_setting_left_out_keeps_its_compiled_default() {
        let config = from_source("font_size = 13", "test").expect("should load");
        let defaults = Config::default_config();

        // The declarative file states what the user changed, not everything.
        assert_eq!(config.scrollback_lines, defaults.scrollback_lines);
    }

    #[test]
    fn an_unknown_setting_is_refused_with_its_line() {
        let error = from_source("\nfont_sze = 13", "test").expect_err("should be refused");

        let message = format!("{error:#}");
        assert!(message.contains("line 2"), "{}", message);
        assert!(message.contains("font_size"), "{}", message);
    }

    #[test]
    fn every_problem_is_listed_at_once() {
        // Fixing a config one complaint per run is a miserable loop, so both
        // stages report everything they found. Parsing comes first, because
        // there is nothing to validate in a file that did not parse.
        let syntax = from_source("a = \nb = \n", "test").expect_err("should be refused");
        assert!(format!("{syntax:#}").contains("2 problem"), "{:#}", syntax);

        let unknown = from_source("font_sze = 13\nscrollbak_lines = 1\n", "test")
            .expect_err("should be refused");
        assert!(format!("{unknown:#}").contains("2 problem"), "{:#}", unknown);
    }

    #[test]
    fn the_font_family_and_its_fallbacks_become_one_ordered_list() {
        let config = from_source(
            r#"
            font_family = "JetBrains Mono"
            font_fallback = ["PingFang SC", "Symbols Nerd Font Mono"]
            "#,
            "test",
        )
        .expect("should load");

        // A setting in the schema that reached no field would leave the user
        // editing their font and seeing nothing change.
        let families: Vec<&str> = config
            .font
            .font
            .iter()
            .map(|attributes| attributes.family.as_str())
            .collect();
        assert_eq!(
            families,
            vec!["JetBrains Mono", "PingFang SC", "Symbols Nerd Font Mono"]
        );
    }

    #[test]
    fn every_setting_in_the_schema_reaches_somewhere() {
        // The schema is what tells a user a setting exists. One that is
        // accepted and then ignored is worse than one that is rejected.
        let handled_elsewhere = [
            "font_family",
            "font_fallback",
            "path_append",
            "colors.background",
            "colors.foreground",
        ];
        for setting in config_schema::SETTINGS {
            let mapped = FIELD_MAP.iter().any(|(declared, _)| declared == setting);
            let known = handled_elsewhere.contains(setting);
            assert!(
                mapped || known,
                "`{}` is in the schema but reaches no field",
                setting
            );
        }
    }

    #[test]
    fn a_keys_section_becomes_bindings() {
        let config = from_source(
            "[keys]
CTRL|SHIFT+N = \"SpawnWindow\"
F11 = \"ToggleFullScreen\"",
            "test",
        )
        .expect("should load");

        // Binding a key was the last thing that needed Lua; removing the
        // interpreter without this would have left users unable to change a
        // single shortcut.
        assert_eq!(config.keys.len(), 2);
    }

    #[test]
    fn a_misspelled_binding_refuses_the_whole_config() {
        let error = from_source("[keys]
CTRL+T = \"SpwanWindow\"", "test")
            .expect_err("should be refused");

        // A binding that silently does nothing is the worst kind: the key does
        // not work and nothing says why.
        assert!(format!("{error:#}").contains("SpwanWindow"), "{:#}", error);
    }

    #[test]
    fn a_shell_becomes_a_command_line() {
        let config = from_source(r#"shell = "powershell.exe""#, "test").expect("should load");

        // `default_prog` is argv, not a program name.
        assert_eq!(
            config.default_prog,
            Some(vec!["powershell.exe".to_string()])
        );
    }

    #[test]
    fn the_shipped_config_loads_without_lua() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/unterm.conf");
        let source = std::fs::read_to_string(path).expect("the shipped config must exist");

        let config = from_source(&source, "assets/unterm.conf").expect("must load");

        assert_eq!(config.font_size, 13.0);
        assert_eq!(config.scrollback_lines, 100000);
    }
}
