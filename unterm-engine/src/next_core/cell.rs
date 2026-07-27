use crate::{
    CellStyle, StyledBlink, StyledCell, StyledColor, StyledUnderline, StyledVerticalAlign,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ScreenCell {
    pub(super) ch: char,
    pub(super) combining: String,
    pub(super) attr: CellAttributes,
    pub(super) width: usize,
}

impl ScreenCell {
    pub(super) fn new(ch: char, attr: CellAttributes) -> Self {
        Self {
            ch,
            combining: String::new(),
            attr,
            width: Self::char_width(ch),
        }
    }

    pub(super) fn blank(attr: CellAttributes) -> Self {
        Self {
            ch: ' ',
            combining: String::new(),
            attr,
            width: 1,
        }
    }

    pub(super) fn continuation(attr: CellAttributes) -> Self {
        Self {
            ch: ' ',
            combining: String::new(),
            attr,
            width: 0,
        }
    }

    pub(super) fn char_width(ch: char) -> usize {
        let mut buf = [0u8; 4];
        termwiz::cell::unicode_column_width(ch.encode_utf8(&mut buf), None)
    }

    pub(super) fn push_combining(&mut self, ch: char) {
        self.combining.push(ch);
    }

    #[allow(dead_code)]
    pub(super) fn styled(&self) -> StyledCell {
        self.styled_with_reverse_video(false, &[])
    }

    pub(super) fn styled_with_reverse_video(
        &self,
        reverse_video: bool,
        hyperlinks: &[String],
    ) -> StyledCell {
        let mut style = self.attr.style(hyperlinks);
        if reverse_video {
            style.inverse = !style.inverse;
        }
        StyledCell {
            ch: self.ch,
            style,
            width: self.width,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CellAttributes {
    pub(super) bold: bool,
    pub(super) faint: bool,
    pub(super) italic: bool,
    pub(super) underline: bool,
    pub(super) underline_style: Option<StyledUnderline>,
    pub(super) underline_color: Option<TerminalColor>,
    pub(super) strikethrough: bool,
    pub(super) hidden: bool,
    pub(super) overline: bool,
    pub(super) blink: Option<StyledBlink>,
    pub(super) vertical_align: Option<StyledVerticalAlign>,
    pub(super) inverse: bool,
    pub(super) protected: bool,
    pub(super) fg: Option<TerminalColor>,
    pub(super) bg: Option<TerminalColor>,
    pub(super) hyperlink: Option<usize>,
}

impl CellAttributes {
    pub(super) fn set_underline(&mut self, style: StyledUnderline) {
        self.underline = true;
        self.underline_style = Some(style);
    }

    pub(super) fn clear_underline(&mut self) {
        self.underline = false;
        self.underline_style = None;
    }

    pub(super) fn set_protected(&mut self, protected: bool) {
        self.protected = protected;
    }

    fn style(&self, hyperlinks: &[String]) -> CellStyle {
        CellStyle {
            bold: self.bold,
            faint: self.faint,
            italic: self.italic,
            underline: self.underline,
            underline_style: self.underline_style,
            underline_color: self.underline_color.map(Into::into),
            strikethrough: self.strikethrough,
            hidden: self.hidden,
            overline: self.overline,
            blink: self.blink,
            vertical_align: self.vertical_align,
            inverse: self.inverse,
            fg: self.fg.map(Into::into),
            bg: self.bg.map(Into::into),
            hyperlink: self
                .hyperlink
                .and_then(|idx| hyperlinks.get(idx).filter(|uri| !uri.is_empty()).cloned()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TerminalColor {
    Palette(u8),
    Rgb(u8, u8, u8),
}

impl From<TerminalColor> for StyledColor {
    fn from(color: TerminalColor) -> Self {
        match color {
            TerminalColor::Palette(idx) => StyledColor::Palette(idx),
            TerminalColor::Rgb(r, g, b) => StyledColor::Rgb(r, g, b),
        }
    }
}
