//! Proxy Settings overlay for reviewing and switching persisted proxy nodes.

use mux::termwiztermtab::TermWizTerminal;
use serde_json::{json, Value};
use termwiz::cell::CellAttributes;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, Position};
use termwiz::terminal::Terminal;

struct ProxyState {
    enabled: bool,
    current_node: String,
    http_proxy: String,
    socks_proxy: String,
    no_proxy: String,
    nodes: Vec<ProxyNode>,
    active_idx: usize,
    message: Option<String>,
}

#[derive(Clone)]
struct ProxyNode {
    name: String,
    url: String,
    kind: String,
}

const MANTLE: (u8, u8, u8) = (0x1a, 0x1a, 0x1a);
const CRUST: (u8, u8, u8) = (0x10, 0x10, 0x10);
const SURFACE0: (u8, u8, u8) = (0x2d, 0x2d, 0x2d);
const SURFACE1: (u8, u8, u8) = (0x3f, 0x3f, 0x3f);
const TEXT: (u8, u8, u8) = (0xe0, 0xe0, 0xe0);
const SUBTEXT0: (u8, u8, u8) = (0xbb, 0xbb, 0xbb);
const OVERLAY0: (u8, u8, u8) = (0x80, 0x80, 0x80);
const BLUE: (u8, u8, u8) = (0x61, 0xaf, 0xef);
const GREEN: (u8, u8, u8) = (0xa6, 0xe3, 0xa1);
const RED: (u8, u8, u8) = (0xf3, 0x8b, 0xa8);

pub fn proxy_settings(term: &mut TermWizTerminal) -> anyhow::Result<()> {
    let mut state = ProxyState::load();
    state.render(term)?;
    state.run_loop(term)?;
    Ok(())
}

impl ProxyState {
    fn load() -> Self {
        let value = read_proxy_json();
        let enabled = value
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let current_node = value
            .get("current_node")
            .and_then(Value::as_str)
            .unwrap_or("local")
            .to_string();
        let http_proxy = value
            .get("http_proxy")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let socks_proxy = value
            .get("socks_proxy")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let no_proxy = value
            .get("no_proxy")
            .and_then(Value::as_str)
            .unwrap_or("localhost,127.0.0.1,::1")
            .to_string();
        let mut nodes = value
            .get("nodes")
            .and_then(Value::as_array)
            .map(|nodes| {
                nodes
                    .iter()
                    .filter_map(|node| {
                        Some(ProxyNode {
                            name: node.get("name")?.as_str()?.to_string(),
                            url: node.get("url")?.as_str()?.to_string(),
                            kind: node
                                .get("kind")
                                .or_else(|| node.get("protocol"))
                                .and_then(Value::as_str)
                                .unwrap_or("http")
                                .to_string(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if nodes.is_empty() {
            nodes.push(ProxyNode {
                name: "local".to_string(),
                url: "http://127.0.0.1:7890".to_string(),
                kind: "http".to_string(),
            });
        }

        let active_idx = nodes
            .iter()
            .position(|node| node.name == current_node)
            .unwrap_or(0);

        Self {
            enabled,
            current_node,
            http_proxy,
            socks_proxy,
            no_proxy,
            nodes,
            active_idx,
            message: None,
        }
    }

    fn render(&self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let size = term.get_screen_size()?;
        let term_w = size.cols;
        let term_h = size.rows;
        let card_w = 68usize.min(term_w.saturating_sub(4));
        let card_h = (9 + self.nodes.len() * 2).min(term_h.saturating_sub(2));
        let start_x = (term_w.saturating_sub(card_w)) / 2;
        let start_y = (term_h.saturating_sub(card_h)) / 3;
        let mut changes = vec![Change::ClearScreen(termwiz::color::ColorAttribute::Default)];

        for y in 0..term_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(y),
            });
            changes.push(fg_bg(" ".repeat(term_w), CRUST, CRUST));
        }

        for y in 0..card_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(start_y + y),
            });
            changes.push(fg_bg(" ".repeat(card_w), TEXT, MANTLE));
        }

        let mut row = start_y;
        self.line(
            &mut changes,
            start_x,
            row,
            card_w,
            &crate::i18n::t("proxy.title"),
            BLUE,
        );
        row += 1;
        let status = if self.enabled {
            crate::i18n::t("proxy.status.enabled")
        } else {
            crate::i18n::t("proxy.status.disabled")
        };
        let status_color = if self.enabled { GREEN } else { RED };
        self.text_row(
            &mut changes,
            start_x,
            row,
            card_w,
            &crate::i18n::t_args(
                "proxy.status_line",
                &[("status", &status), ("node", &self.current_node)],
            ),
            status_color,
        );
        row += 1;
        let unset = crate::i18n::t("proxy.value.unset");
        let http_value = if self.http_proxy.is_empty() {
            unset.clone()
        } else {
            self.http_proxy.clone()
        };
        let socks_value = if self.socks_proxy.is_empty() {
            unset
        } else {
            self.socks_proxy.clone()
        };
        self.text_row(
            &mut changes,
            start_x,
            row,
            card_w,
            &crate::i18n::t_args("proxy.http", &[("value", &http_value)]),
            SUBTEXT0,
        );
        row += 1;
        self.text_row(
            &mut changes,
            start_x,
            row,
            card_w,
            &crate::i18n::t_args("proxy.socks", &[("value", &socks_value)]),
            SUBTEXT0,
        );
        row += 1;
        self.separator(&mut changes, start_x, row, card_w);
        row += 1;

        for (idx, node) in self.nodes.iter().enumerate() {
            let selected = idx == self.active_idx;
            let bg = if selected { SURFACE0 } else { MANTLE };
            let indicator = if selected { ">" } else { " " };
            let current = if node.name == self.current_node {
                "*"
            } else {
                " "
            };
            self.text_row_bg(
                &mut changes,
                start_x,
                row,
                card_w,
                &format!("{indicator}{current} {} [{}]", node.name, node.kind),
                if selected { TEXT } else { SUBTEXT0 },
                bg,
            );
            row += 1;
            self.text_row_bg(
                &mut changes,
                start_x,
                row,
                card_w,
                &format!("   {}", node.url),
                OVERLAY0,
                bg,
            );
            row += 1;
        }

        self.separator(&mut changes, start_x, row, card_w);
        row += 1;
        let footer = self
            .message
            .clone()
            .unwrap_or_else(|| crate::i18n::t("proxy.footer.hint"));
        self.text_row(&mut changes, start_x, row, card_w, &footer, OVERLAY0);

        changes.push(Change::AllAttributes(CellAttributes::default()));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(0),
        });
        term.render(&changes)
    }

    fn line(
        &self,
        changes: &mut Vec<Change>,
        start_x: usize,
        row: usize,
        card_w: usize,
        title: &str,
        color: (u8, u8, u8),
    ) {
        self.text_row(changes, start_x, row, card_w, title, color);
    }

    fn separator(&self, changes: &mut Vec<Change>, start_x: usize, row: usize, card_w: usize) {
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(row),
        });
        changes.push(fg_bg("-".repeat(card_w), SURFACE1, MANTLE));
    }

    fn text_row(
        &self,
        changes: &mut Vec<Change>,
        start_x: usize,
        row: usize,
        card_w: usize,
        text: &str,
        fg: (u8, u8, u8),
    ) {
        self.text_row_bg(changes, start_x, row, card_w, text, fg, MANTLE);
    }

    fn text_row_bg(
        &self,
        changes: &mut Vec<Change>,
        start_x: usize,
        row: usize,
        card_w: usize,
        text: &str,
        fg: (u8, u8, u8),
        bg: (u8, u8, u8),
    ) {
        let visible = truncate_chars(text, card_w.saturating_sub(2));
        let pad = card_w.saturating_sub(visible.chars().count());
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(row),
        });
        changes.push(fg_bg(
            format!(" {}{}", visible, " ".repeat(pad - 1)),
            fg,
            bg,
        ));
    }

    fn switch_selected(&mut self) {
        if let Some(node) = self.nodes.get(self.active_idx).cloned() {
            self.enabled = true;
            self.current_node = node.name.clone();
            if node.kind.eq_ignore_ascii_case("socks") || node.url.starts_with("socks") {
                self.socks_proxy = node.url.clone();
                self.http_proxy = node.url.clone();
            } else {
                self.http_proxy = node.url.clone();
            }
            self.save();
            self.message = Some(crate::i18n::t_args(
                "proxy.switched_to",
                &[("name", &node.name)],
            ));
        }
    }

    fn toggle(&mut self) {
        if self.enabled {
            self.enabled = false;
            self.message = Some(crate::i18n::t("proxy.disabled_msg"));
        } else {
            self.switch_selected();
            return;
        }
        self.save();
    }

    fn disable(&mut self) {
        self.enabled = false;
        self.save();
        self.message = Some(crate::i18n::t("proxy.disabled_msg"));
    }

    fn save(&self) {
        let nodes: Vec<Value> = self
            .nodes
            .iter()
            .map(|node| json!({"name": node.name, "url": node.url, "kind": node.kind}))
            .collect();
        let value = json!({
            "enabled": self.enabled,
            "current_node": self.current_node,
            "http_proxy": if self.http_proxy.is_empty() { Value::Null } else { json!(self.http_proxy) },
            "socks_proxy": if self.socks_proxy.is_empty() { Value::Null } else { json!(self.socks_proxy) },
            "no_proxy": self.no_proxy,
            "nodes": nodes,
        });
        let path = proxy_config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(&value) {
            let _ = std::fs::write(path, text);
        }
    }

    fn run_loop(&mut self, term: &mut TermWizTerminal) -> anyhow::Result<()> {
        while let Ok(Some(event)) = term.poll_input(None) {
            match event {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => break,
                InputEvent::Key(KeyEvent {
                    key: KeyCode::UpArrow,
                    ..
                })
                | InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('k'),
                    ..
                }) => {
                    self.active_idx = self.active_idx.saturating_sub(1);
                    self.message = None;
                    self.render(term)?;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::DownArrow,
                    ..
                })
                | InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('j'),
                    ..
                }) => {
                    self.active_idx = (self.active_idx + 1).min(self.nodes.len().saturating_sub(1));
                    self.message = None;
                    self.render(term)?;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Enter,
                    ..
                }) => {
                    self.switch_selected();
                    self.render(term)?;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('t') | KeyCode::Char('T'),
                    ..
                }) => {
                    self.toggle();
                    self.render(term)?;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('d') | KeyCode::Char('D'),
                    ..
                }) => {
                    self.disable();
                    self.render(term)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn read_proxy_json() -> Value {
    let path = proxy_config_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(|| {
            json!({
                "enabled": false,
                "current_node": "local",
                "http_proxy": "http://127.0.0.1:7890",
                "socks_proxy": "socks5://127.0.0.1:7890",
                "no_proxy": "localhost,127.0.0.1,::1",
                "nodes": [{"name": "local", "url": "http://127.0.0.1:7890", "kind": "http"}],
            })
        })
}

fn proxy_config_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("proxy.json")
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

fn fg_bg(text: String, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Change {
    Change::Text(format!(
        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
        fg.0, fg.1, fg.2, bg.0, bg.1, bg.2, text
    ))
}
