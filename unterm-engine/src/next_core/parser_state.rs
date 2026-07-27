#[derive(Default)]
pub(super) enum ParserState {
    #[default]
    Ground,
    Escape,
    EscapeIgnoreOne,
    EscapeHash,
    Csi(String),
    Osc(String),
    OscEscape(String),
    IgnoredString,
    IgnoredStringEscape,
}
