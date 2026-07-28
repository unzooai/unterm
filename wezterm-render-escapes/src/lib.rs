//! Rendering helpers that never needed Lua.
//!
//! These lived in the Lua binding crate because that is where the terminfo
//! renderer happened to be built. They are terminal rendering, not scripting,
//! so they outlive it.

use finl_unicode::grapheme_clusters::Graphemes;
use std::str::FromStr;
use termwiz::caps::{Capabilities, ColorLevel, ProbeHints};
use termwiz::cell::{grapheme_column_width, unicode_column_width, AttributeChange, CellAttributes};
use termwiz::color::{AnsiColor, ColorAttribute, ColorSpec, SrgbaTuple};
use termwiz::render::terminfo::TerminfoRenderer;
use termwiz::surface::change::Change;
use termwiz::surface::Line;
use wezterm_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, FromDynamic, ToDynamic, Clone, PartialEq, Eq)]
pub enum FormatColor {
    AnsiColor(AnsiColor),
    Color(String),
    Default,
}

impl FormatColor {
    fn to_attr(self) -> ColorAttribute {
        let spec: ColorSpec = self.into();
        let attr: ColorAttribute = spec.into();
        attr
    }
}

impl From<FormatColor> for ColorSpec {
    fn from(val: FormatColor) -> Self {
        match val {
            FormatColor::AnsiColor(c) => c.into(),
            FormatColor::Color(s) => {
                let rgba = SrgbaTuple::from_str(&s).unwrap_or_else(|()| (0xff, 0xff, 0xff).into());
                rgba.into()
            }
            FormatColor::Default => ColorSpec::Default,
        }
    }
}

#[derive(Debug, FromDynamic, ToDynamic, Clone, PartialEq, Eq)]
pub enum FormatItem {
    Foreground(FormatColor),
    Background(FormatColor),
    Attribute(AttributeChange),
    ResetAttributes,
    Text(String),
}

impl From<FormatItem> for Change {
    fn from(val: FormatItem) -> Self {
        match val {
            FormatItem::Attribute(change) => change.into(),
            FormatItem::Text(t) => t.into(),
            FormatItem::Foreground(c) => AttributeChange::Foreground(c.to_attr()).into(),
            FormatItem::Background(c) => AttributeChange::Background(c.to_attr()).into(),
            FormatItem::ResetAttributes => Change::AllAttributes(CellAttributes::default()),
        }
    }
}

struct FormatTarget {
    target: Vec<u8>,
}

impl std::io::Write for FormatTarget {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.target, buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl termwiz::render::RenderTty for FormatTarget {
    fn get_size_in_cells(&mut self) -> termwiz::Result<(usize, usize)> {
        Ok((80, 24))
    }
}

pub fn format_as_escapes(items: Vec<FormatItem>) -> anyhow::Result<String> {
    let mut changes: Vec<Change> = items.into_iter().map(Into::into).collect();
    changes.push(Change::AllAttributes(CellAttributes::default()).into());
    let mut renderer = new_wezterm_terminfo_renderer();
    let mut target = FormatTarget { target: vec![] };
    renderer.render_to(&changes, &mut target)?;
    Ok(String::from_utf8(target.target)?)
}

pub fn pad_right(mut result: String, width: usize) -> String {
    let mut len = unicode_column_width(&result, None);
    while len < width {
        result.push(' ');
        len += 1;
    }

    result
}

pub fn pad_left(mut result: String, width: usize) -> String {
    let mut len = unicode_column_width(&result, None);
    while len < width {
        result.insert(0, ' ');
        len += 1;
    }

    result
}

pub fn truncate_left(s: &str, max_width: usize) -> String {
    let mut result = vec![];
    let mut len = 0;
    let graphemes: Vec<_> = Graphemes::new(s).collect();
    for &g in graphemes.iter().rev() {
        let g_len = grapheme_column_width(g, None);
        if g_len + len > max_width {
            break;
        }
        result.push(g);
        len += g_len;
    }

    result.reverse();
    result.join("")
}

pub fn truncate_right(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut len = 0;
    for g in Graphemes::new(s) {
        let g_len = grapheme_column_width(g, None);
        if g_len + len > max_width {
            break;
        }
        result.push_str(g);
        len += g_len;
    }
    result
}

lazy_static::lazy_static! {
    static ref CAPS: Capabilities = {
        let data = include_bytes!("../../termwiz/data/xterm-256color");
        let db = terminfo::Database::from_buffer(&data[..]).unwrap();
        Capabilities::new_with_hints(
            ProbeHints::new_from_env()
                .term(Some("xterm-256color".into()))
                .terminfo_db(Some(db))
                .color_level(Some(ColorLevel::TrueColor))
                .colorterm(None)
                .colorterm_bce(None)
                .term_program(Some("WezTerm".into()))
                .term_program_version(Some(config::wezterm_version().into())),
        )
        .expect("cannot fail to make internal Capabilities")
    };
}

pub fn new_wezterm_terminfo_renderer() -> TerminfoRenderer {
    TerminfoRenderer::new(CAPS.clone())
}

pub fn lines_to_escapes(lines: Vec<Line>) -> anyhow::Result<String> {
    let mut changes = vec![];
    let mut attr = CellAttributes::blank();
    for line in lines {
        changes.append(&mut line.changes(&attr));
        changes.push(Change::Text("\r\n".to_string()));
        if let Some(a) = line.visible_cells().last().map(|cell| cell.attrs().clone()) {
            attr = a;
        }
    }
    changes.push(Change::AllAttributes(CellAttributes::blank()));
    let mut renderer = new_wezterm_terminfo_renderer();

    struct Target {
        target: Vec<u8>,
    }

    impl std::io::Write for Target {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            std::io::Write::write(&mut self.target, buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl termwiz::render::RenderTty for Target {
        fn get_size_in_cells(&mut self) -> termwiz::Result<(usize, usize)> {
            Ok((80, 24))
        }
    }

    let mut target = Target { target: vec![] };
    renderer.render_to(&changes, &mut target)?;
    Ok(String::from_utf8(target.target)?)
}
