//! Find a character by its name and type it.
//!
//! A terminal is the one place where the operating system's own emoji picker
//! is no use: it types into a text field, and there is no text field here.
//! So the terminal has to have its own.
//!
//! Every emoji by name and by shortcode, plus the punctuation and arrows
//! nobody can type -- an em dash, a non-breaking space, the box-drawing
//! characters. Picking one writes it at the prompt.
//!
//! What is picked is remembered and floats to the top, ranked by frecency
//! rather than by recency: somebody who uses one arrow every day and picked a
//! flag once last month wants the arrow first, and a plain most-recent list
//! puts the flag there.

use std::borrow::Cow;

/// One offer: what it types, and what it is called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// What gets typed.
    pub glyph: String,
    /// What it is called, which is what gets matched.
    pub name: Cow<'static, str>,
    /// Where it came from, shown after the name.
    pub group: Group,
}

/// Which part of the list an offer belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// Picked before. Ranked by frecency and offered first.
    Recent,
    Emoji,
    /// `:rocket:` and the like.
    Shortcode,
    /// The characters a keyboard has no key for.
    Punctuation,
}

impl Group {
    pub fn heading(self) -> &'static str {
        match self {
            Group::Recent => "recent",
            Group::Emoji => "emoji",
            Group::Shortcode => "shortcode",
            Group::Punctuation => "punctuation",
        }
    }
}

/// The characters a keyboard cannot type but a terminal needs.
///
/// Deliberately short and hand-picked. A full Unicode name table is thirty
/// thousand rows of things nobody is looking for, and it buries the dozen that
/// people actually come here for.
const PUNCTUATION: &[(&str, &str)] = &[
    ("\u{2014}", "em dash"),
    ("\u{2013}", "en dash"),
    ("\u{2026}", "ellipsis"),
    ("\u{00A0}", "non-breaking space"),
    ("\u{201C}", "left double quotation mark"),
    ("\u{201D}", "right double quotation mark"),
    ("\u{2018}", "left single quotation mark"),
    ("\u{2019}", "right single quotation mark"),
    ("\u{2192}", "rightwards arrow"),
    ("\u{2190}", "leftwards arrow"),
    ("\u{2191}", "upwards arrow"),
    ("\u{2193}", "downwards arrow"),
    ("\u{21D2}", "rightwards double arrow"),
    ("\u{2713}", "check mark"),
    ("\u{2717}", "ballot x"),
    ("\u{2022}", "bullet"),
    ("\u{00B7}", "middle dot"),
    ("\u{00B0}", "degree sign"),
    ("\u{00B1}", "plus minus sign"),
    ("\u{00D7}", "multiplication sign"),
    ("\u{00F7}", "division sign"),
    ("\u{2260}", "not equal to"),
    ("\u{2264}", "less than or equal to"),
    ("\u{2265}", "greater than or equal to"),
    ("\u{221E}", "infinity"),
    ("\u{03BB}", "greek small letter lambda"),
    ("\u{03BC}", "greek small letter mu"),
    ("\u{03C0}", "greek small letter pi"),
    ("\u{2500}", "box drawings light horizontal"),
    ("\u{2502}", "box drawings light vertical"),
    ("\u{250C}", "box drawings light down and right"),
    ("\u{2510}", "box drawings light down and left"),
    ("\u{2514}", "box drawings light up and right"),
    ("\u{2518}", "box drawings light up and left"),
    ("\u{2588}", "full block"),
    ("\u{00A9}", "copyright sign"),
    ("\u{2122}", "trade mark sign"),
    ("\u{20AC}", "euro sign"),
    ("\u{00A3}", "pound sign"),
    ("\u{00A5}", "yen sign"),
];

/// Everything on offer, recents first.
pub fn choices() -> Vec<Choice> {
    let mut choices: Vec<Choice> = remembered()
        .into_iter()
        .map(|recent| Choice {
            glyph: recent.glyph,
            name: Cow::Owned(recent.name),
            group: Group::Recent,
        })
        .collect();

    for emoji in emojis::iter() {
        choices.push(Choice {
            glyph: emoji.as_str().to_string(),
            name: Cow::Borrowed(emoji.name()),
            group: Group::Emoji,
        });
        for shortcode in emoji.shortcodes() {
            choices.push(Choice {
                glyph: emoji.as_str().to_string(),
                name: Cow::Borrowed(shortcode),
                group: Group::Shortcode,
            });
        }
    }
    for (glyph, name) in PUNCTUATION {
        choices.push(Choice {
            glyph: glyph.to_string(),
            name: Cow::Borrowed(name),
            group: Group::Punctuation,
        });
    }
    choices
}

/// Whatever has been picked before, best first.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Recent {
    pub glyph: String,
    pub name: String,
    pub frecency: frecency::Frecency,
}

fn remembered_path() -> Option<std::path::PathBuf> {
    Some(
        dirs_next::home_dir()?
            .join(".unterm")
            .join("recent-characters.json"),
    )
}

/// What has been picked before, most useful first.
pub fn remembered() -> Vec<Recent> {
    let Some(path) = remembered_path() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut recents: Vec<Recent> = serde_json::from_str(&text).unwrap_or_default();
    recents.sort_by(|a, b| {
        b.frecency
            .score()
            .partial_cmp(&a.frecency.score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    recents
}

/// Remember a pick.
///
/// Frecency rather than recency: somebody who reaches for one arrow every day
/// and picked a flag once last month wants the arrow first, and a plain
/// most-recent list puts the flag there.
pub fn remember(glyph: &str, name: &str) {
    let mut recents = remembered();
    match recents.iter_mut().find(|recent| recent.glyph == glyph) {
        Some(recent) => recent.frecency.register_access(),
        None => {
            let mut frecency = frecency::Frecency::new();
            frecency.register_access();
            recents.push(Recent {
                glyph: glyph.to_string(),
                name: name.to_string(),
                frecency,
            });
        }
    }
    // Bounded, so a file nobody looks at cannot grow without limit.
    recents.truncate(64);
    let Some(path) = remembered_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(&recents) {
        let _ = std::fs::write(path, text);
    }
}

/// The offers that match `query`, best first.
///
/// Whole words in order, anywhere in the name. `smiling face` finds every
/// smiling face; `sm fa` finds them too, which is what makes a picker faster
/// than scrolling.
pub fn matching(choices: &[Choice], query: &str, limit: usize) -> Vec<Choice> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        // Recents first, then the rest as they were built. Everything with no
        // query is thousands of rows; only the top of it is ever seen.
        return choices.iter().take(limit).cloned().collect();
    }
    let words: Vec<&str> = query.split_whitespace().collect();

    let mut scored: Vec<(i32, usize, &Choice)> = choices
        .iter()
        .enumerate()
        .filter_map(|(index, choice)| {
            score(&choice.name.to_lowercase(), &words).map(|score| (score, index, choice))
        })
        .collect();
    // Best score first; ties keep the order they were built in, which puts
    // recents above everything they tie with.
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, _, choice)| choice.clone())
        .collect()
}

/// How well a name matches every word of a query, or `None` if it does not.
fn score(name: &str, words: &[&str]) -> Option<i32> {
    let mut total = 0;
    let mut at = 0usize;
    for word in words {
        let found = name.get(at..)?.find(word)? + at;
        // A word that starts a word in the name beats one buried in the middle:
        // `face` should find "grinning face" before "facepalm".
        let starts_word = found == 0
            || name[..found]
                .chars()
                .next_back()
                .map(|ch| !ch.is_alphanumeric())
                .unwrap_or(true);
        total += if starts_word { 8 } else { 2 };
        at = found + word.len();
    }
    // An exact name wins outright, and a short name beats a long one that
    // merely contains the same words.
    if words.len() == 1 && name == words[0] {
        total += 32;
    }
    Some(total - (name.len() as i32) / 8)
}

// Named for what it tests rather than `tests`, because cargo's filter is a
// plain substring and `select::tests::` would otherwise pick these up too.
#[cfg(test)]
mod picker_tests {
    use super::*;

    fn sample() -> Vec<Choice> {
        [
            ("\u{1F600}", "grinning face", Group::Emoji),
            ("\u{1F926}", "person facepalming", Group::Emoji),
            ("\u{1F680}", "rocket", Group::Emoji),
            ("\u{1F680}", "rocket", Group::Shortcode),
            ("\u{2014}", "em dash", Group::Punctuation),
            ("\u{2192}", "rightwards arrow", Group::Punctuation),
        ]
        .into_iter()
        .map(|(glyph, name, group)| Choice {
            glyph: glyph.to_string(),
            name: Cow::Borrowed(name),
            group,
        })
        .collect()
    }

    fn names(found: &[Choice]) -> Vec<&str> {
        found.iter().map(|choice| choice.name.as_ref()).collect()
    }

    #[test]
    fn a_name_finds_its_character() {
        let found = matching(&sample(), "rocket", 10);
        assert_eq!(found.first().map(|c| c.glyph.as_str()), Some("\u{1F680}"));
    }

    /// Words in order, anywhere in the name. That is what makes a picker
    /// faster than scrolling: `gr fa` should reach "grinning face".
    #[test]
    fn separate_words_match_in_order() {
        let found = matching(&sample(), "gr fa", 10);
        assert_eq!(names(&found), vec!["grinning face"]);
    }

    /// And in order means in order: `face grinning` is not `grinning face`.
    #[test]
    fn words_out_of_order_do_not_match() {
        assert!(matching(&sample(), "face grinning", 10).is_empty());
    }

    /// A word at the start of a word beats one buried in the middle.
    #[test]
    fn a_word_boundary_beats_the_middle_of_a_word() {
        let found = matching(&sample(), "face", 10);
        assert_eq!(
            names(&found).first(),
            Some(&"grinning face"),
            "{:?}",
            names(&found)
        );
    }

    /// Nothing matching is nothing offered, rather than everything.
    #[test]
    fn a_query_that_matches_nothing_offers_nothing() {
        assert!(matching(&sample(), "zzzznothing", 10).is_empty());
    }

    /// An empty query shows the top of the list rather than all of it: the
    /// whole list is thousands of rows and only the top is ever seen.
    #[test]
    fn an_empty_query_shows_the_top_of_the_list() {
        let found = matching(&sample(), "", 3);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].name, "grinning face");
    }

    #[test]
    fn the_limit_is_respected() {
        for limit in [1, 2, 5, 100] {
            assert!(matching(&sample(), "", limit).len() <= limit);
            assert!(matching(&sample(), "o", limit).len() <= limit);
        }
    }

    /// The punctuation is the point for a terminal: an em dash and a
    /// non-breaking space have no keys, and that is why anybody opens this.
    #[test]
    fn the_characters_with_no_keys_are_all_there() {
        let all = choices();
        for wanted in ["\u{2014}", "\u{00A0}", "\u{2192}", "\u{2500}", "\u{20AC}"] {
            assert!(
                all.iter().any(|choice| choice.glyph == wanted),
                "{wanted:?} is not offered"
            );
        }
    }

    /// And the emoji are, by name and by shortcode.
    #[test]
    fn emoji_are_offered_by_name_and_by_shortcode() {
        let all = choices();
        let rocket: Vec<&Choice> = all
            .iter()
            .filter(|choice| choice.glyph == "\u{1F680}")
            .collect();
        assert!(
            rocket.iter().any(|choice| choice.group == Group::Emoji),
            "no rocket by name"
        );
        assert!(
            rocket.iter().any(|choice| choice.group == Group::Shortcode),
            "no rocket by shortcode"
        );
    }

    /// Every offer types something. A row that inserts nothing is a row that
    /// looks like it did not work.
    #[test]
    fn every_offer_types_something() {
        for choice in choices().iter().take(2000) {
            assert!(!choice.glyph.is_empty(), "{:?} types nothing", choice.name);
            assert!(!choice.name.trim().is_empty(), "an offer with no name");
        }
    }

    /// Frecency, not recency: something reached for daily outranks something
    /// picked once, however recently.
    #[test]
    fn something_used_often_outranks_something_used_once() {
        let mut often = frecency::Frecency::new();
        for _ in 0..20 {
            often.register_access();
        }
        let mut once = frecency::Frecency::new();
        once.register_access();
        assert!(
            often.score() > once.score(),
            "{} is not above {}",
            often.score(),
            once.score()
        );
    }
}
