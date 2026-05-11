//! Display-name → ID slugification.
//!
//! Users only ever type a free-text `display_name` ("Work — Acme",
//! "工作", "🚀 Personal"). Unterm derives a stable internal ID from
//! that string for:
//!
//! - the on-disk filename (`<id>.toml`)
//! - the OS keychain entry name (`unterm/<id>/<env>`)
//! - CLI/MCP method parameters
//!
//! The rules below (locked 2026-05-11, see design doc §4) collapse a
//! display name into a safe identifier with **minimal surprise**:
//!
//! 1. lowercase the ASCII portion; CJK / Arabic / Cyrillic / etc. all
//!    pass through unchanged
//! 2. strip emoji, control characters, and unprintable Unicode
//! 3. strip apostrophes and quote characters (so `Mom's` → `moms`)
//! 4. collapse whitespace and separator-like punctuation (em-dash,
//!    underscore, slash, period, etc.) into a single `-`
//! 5. trim leading/trailing `-`
//! 6. truncate to 64 graphemes
//!
//! Non-ASCII is preserved on purpose. macOS Keychain, Windows
//! Credential Manager, Linux Secret Service, and APFS / NTFS / ext4
//! all handle UTF-8 natively. Pinyin transliteration of CJK names
//! would be culturally fraught and information-lossy — we don't do it.

use unicode_segmentation::UnicodeSegmentation;

/// Characters that are forbidden in filenames on at least one major OS
/// (Windows being the strictest). When we see one in a display name we
/// treat it as a word separator: it gets replaced by `-`, not deleted,
/// so structural meaning ("Work/Project" reads as two words) survives.
const FILESYSTEM_FORBIDDEN: &[char] = &['/', '\\', ':', '|', '?', '*', '<', '>', '"', '\0'];

/// Characters we silently *delete* rather than treat as separators.
/// Apostrophes and quotes — common in possessive forms like "Mom's
/// Gmail" — should glue letters together, not split the word.
fn is_deletable_punct(ch: char) -> bool {
    matches!(
        ch,
        '\'' | '\u{2018}' | '\u{2019}' | '\u{201C}' | '\u{201D}' | '`'
    )
}

/// Characters that conceptually separate words and should collapse
/// into a single `-` in the output.
fn is_word_separator(ch: char) -> bool {
    if ch.is_whitespace() {
        return true;
    }
    matches!(
        ch,
        '-' | '_'
            | '\u{2014}'  // em dash
            | '\u{2013}'  // en dash
            | '.'
            | ','
            | ';'
            | '!'
            | '?'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '@'
            | '#'
            | '$'
            | '%'
            | '^'
            | '&'
            | '+'
            | '='
            | '~'
    )
}

/// Heuristic: is this codepoint in a range we consider "emoji"?
///
/// We don't need bit-exact Unicode emoji semantics — slugify just needs
/// to drop pictographic glyphs that have no useful sort order or
/// linguistic meaning as part of an identifier. The ranges below cover
/// the bulk of what users actually paste in.
fn is_emoji_like(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp,
        0x1F300..=0x1F9FF  // Misc Symbols & Pictographs, Emoticons, Transport, Supplemental Symbols
        | 0x2600..=0x26FF  // Misc Symbols (☀ ☂ ⚡ etc)
        | 0x2700..=0x27BF  // Dingbats (✅ ❤ ✨)
        | 0x2B00..=0x2BFF  // Misc Symbols and Arrows (⭐ ⬆)
        | 0x1F000..=0x1F0FF // Mahjong / Domino / Cards
        | 0x1F100..=0x1F2FF // Enclosed Alphanumeric / Ideographic Supplement
        | 0x1FA00..=0x1FAFF // Symbols & Pictographs Extended-A
        | 0xFE0F            // Variation Selector-16 (emoji presentation)
        | 0x200D            // Zero-Width Joiner
    )
}

/// Slugify a display name into a Unterm profile ID.
///
/// Returns `None` if the result would be empty after all transforms —
/// callers should fall back to a generated ID (e.g. `profile-<unix-ts>`).
pub fn slugify(display_name: &str) -> Option<String> {
    let mut out = String::new();
    let mut prev_was_dash = false;

    for ch in display_name.chars() {
        // Step 1: ASCII letters lowercased; everything else passes through.
        let ch = if ch.is_ascii_alphabetic() {
            ch.to_ascii_lowercase()
        } else {
            ch
        };

        // Step 2: word separators (whitespace, punctuation, filesystem-
        // forbidden chars). This MUST come before the control-character
        // check below: `\t` and `\n` are simultaneously whitespace AND
        // control, and we want them to act as separators so "Tab\nNew"
        // becomes "tab-new", not "tabnew".
        if is_word_separator(ch) || FILESYSTEM_FORBIDDEN.contains(&ch) {
            if !prev_was_dash && !out.is_empty() {
                out.push('-');
                prev_was_dash = true;
            }
            continue;
        }

        // Step 3: silently drop characters that have no meaning as part
        // of an identifier — non-whitespace control chars, emoji,
        // apostrophes/quotes.
        if ch.is_control() || is_emoji_like(ch) || is_deletable_punct(ch) {
            continue;
        }

        out.push(ch);
        prev_was_dash = false;
    }

    // Step 5: trim trailing dashes (leading already prevented by the
    // !out.is_empty() guard).
    while out.ends_with('-') {
        out.pop();
    }

    // Step 6: truncate to 64 graphemes (not bytes — a CJK char is one
    // grapheme but 3 UTF-8 bytes).
    let truncated: String = out.graphemes(true).take(64).collect();

    if truncated.is_empty() {
        None
    } else {
        Some(truncated)
    }
}

/// Append `-2`, `-3`, ... to `base` until we find a name not present
/// in `taken`. Used when two different display names slugify to the
/// same value (e.g. "Work" and "WORK" both become "work").
pub fn disambiguate(base: &str, taken: &[String]) -> String {
    if !taken.iter().any(|t| t == base) {
        return base.to_string();
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}-{n}");
        if !taken.iter().any(|t| t == &candidate) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_slug(input: &str, expected: &str) {
        assert_eq!(
            slugify(input).as_deref(),
            Some(expected),
            "slugify({input:?})"
        );
    }

    #[test]
    fn ascii_basic() {
        assert_slug("Work", "work");
        assert_slug("My Side Project", "my-side-project");
        assert_slug("Personal", "personal");
    }

    #[test]
    fn em_dash_and_spaces_collapse() {
        assert_slug("Work — Acme", "work-acme");
        assert_slug("Work – Acme", "work-acme"); // en dash
        assert_slug("Work - Acme", "work-acme"); // ascii hyphen
    }

    #[test]
    fn apostrophes_glue_letters() {
        assert_slug("Mom's Gmail", "moms-gmail");
        assert_slug("It’s Mine", "its-mine"); // Unicode right single quote
    }

    #[test]
    fn cjk_preserved() {
        assert_slug("工作", "工作");
        assert_slug("个人账号", "个人账号");
    }

    #[test]
    fn mixed_cjk_and_ascii() {
        assert_slug("Work 工作", "work-工作");
        assert_slug("个人 Personal", "个人-personal");
    }

    #[test]
    fn emoji_stripped() {
        assert_slug("🚀 Personal", "personal");
        assert_slug("⭐ Starred", "starred");
        assert_slug("👨‍💻 Dev", "dev"); // ZWJ sequence
    }

    #[test]
    fn forbidden_chars_become_separators() {
        assert_slug("a/b\\c:d", "a-b-c-d");
        assert_slug("path|with?bad*chars", "path-with-bad-chars");
    }

    #[test]
    fn whitespace_collapses() {
        assert_slug("Multi   Space", "multi-space");
        assert_slug("\tTab\nNewline", "tab-newline");
    }

    #[test]
    fn trim_outer_dashes() {
        assert_slug("---Work---", "work");
        assert_slug("  trim me  ", "trim-me");
    }

    #[test]
    fn empty_becomes_none() {
        assert_eq!(slugify(""), None);
        assert_eq!(slugify("   "), None);
        assert_eq!(slugify("---"), None);
        assert_eq!(slugify("🚀"), None);
    }

    #[test]
    fn truncate_to_64_graphemes() {
        let long: String = "a".repeat(100);
        let slug = slugify(&long).unwrap();
        assert_eq!(slug.chars().count(), 64);
    }

    #[test]
    fn truncate_counts_graphemes_not_bytes() {
        // 70 CJK chars: 70 graphemes but 210 UTF-8 bytes.
        let long: String = "工".repeat(70);
        let slug = slugify(&long).unwrap();
        assert_eq!(slug.chars().count(), 64);
    }

    #[test]
    fn disambiguate_returns_base_when_free() {
        assert_eq!(disambiguate("work", &[]), "work");
        assert_eq!(
            disambiguate("work", &["other".to_string()]),
            "work"
        );
    }

    #[test]
    fn disambiguate_finds_next_free_suffix() {
        assert_eq!(
            disambiguate("work", &["work".to_string()]),
            "work-2"
        );
        assert_eq!(
            disambiguate(
                "work",
                &["work".to_string(), "work-2".to_string(), "work-3".to_string()]
            ),
            "work-4"
        );
    }
}
