//! Find a character by its name and type it.
//!
//! A terminal is the one place where the operating system's own emoji picker
//! is no use: it types into a text field, and there is no text field here.
//! So the terminal has to have its own.
//!
//! The full 0.57.4 catalogue: every emoji by name and by shortcode, the whole
//! Unicode name table, and the Nerd Font glyphs -- the box-drawing characters,
//! the em dash nobody can type, the `cod_` and `fa_` icons a prompt is built
//! from. The rows come in thirteen groups, the same thirteen the old kernel
//! had, and Ctrl+R turns the wheel to the next one. Picking one writes it at
//! the prompt.
//!
//! What is picked is remembered and floats to the top, ranked by frecency
//! rather than by recency: somebody who uses one arrow every day and picked a
//! flag once last month wants the arrow first, and a plain most-recent list
//! puts the flag there.

use nucleo_matcher::pattern::Pattern;
use nucleo_matcher::Utf32Str;
use rayon::prelude::*;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

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

impl Choice {
    /// The glyph spelled out: `U+2014`, or `U+1F44D U+1F3FD` for a sequence.
    ///
    /// Shown beside the name, and matched too: `2014` finds the em dash by
    /// number, which is how anybody who knows the number looks for it.
    pub fn codepoints(&self) -> String {
        let mut spelled = String::new();
        for ch in self.glyph.chars() {
            if !spelled.is_empty() {
                spelled.push(' ');
            }
            spelled.push_str(&format!("U+{:X}", ch as u32));
        }
        spelled
    }
}

/// Which page of the picker an offer belongs to.
///
/// The same thirteen groups 0.57.4 had, in the same order: the nine emoji
/// groups the `emojis` crate defines, the Nerd Font glyphs, the Unicode name
/// table, the `:rocket:` shortcodes, and whatever has been picked before.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// Picked before. Ranked by frecency and offered first.
    RecentlyUsed,
    SmileysAndEmotion,
    PeopleAndBody,
    AnimalsAndNature,
    FoodAndDrink,
    TravelAndPlaces,
    Activities,
    Objects,
    Symbols,
    Flags,
    /// The `cod_`/`fa_`/`md_` icon names a prompt is built from.
    NerdFonts,
    /// The whole Unicode name table: `EM DASH`, `NO-BREAK SPACE`, all of it.
    UnicodeNames,
    /// `:rocket:` and the like.
    ShortCodes,
}

impl Group {
    /// The wheel Ctrl+R turns. The order is 0.57.4's, and it comes around.
    pub const fn next(self) -> Self {
        match self {
            Group::RecentlyUsed => Group::SmileysAndEmotion,
            Group::SmileysAndEmotion => Group::PeopleAndBody,
            Group::PeopleAndBody => Group::AnimalsAndNature,
            Group::AnimalsAndNature => Group::FoodAndDrink,
            Group::FoodAndDrink => Group::TravelAndPlaces,
            Group::TravelAndPlaces => Group::Activities,
            Group::Activities => Group::Objects,
            Group::Objects => Group::Symbols,
            Group::Symbols => Group::Flags,
            Group::Flags => Group::NerdFonts,
            Group::NerdFonts => Group::UnicodeNames,
            Group::UnicodeNames => Group::ShortCodes,
            Group::ShortCodes => Group::RecentlyUsed,
        }
    }

    /// The same wheel, turned the other way.
    pub const fn previous(self) -> Self {
        match self {
            Group::SmileysAndEmotion => Group::RecentlyUsed,
            Group::PeopleAndBody => Group::SmileysAndEmotion,
            Group::AnimalsAndNature => Group::PeopleAndBody,
            Group::FoodAndDrink => Group::AnimalsAndNature,
            Group::TravelAndPlaces => Group::FoodAndDrink,
            Group::Activities => Group::TravelAndPlaces,
            Group::Objects => Group::Activities,
            Group::Symbols => Group::Objects,
            Group::Flags => Group::Symbols,
            Group::NerdFonts => Group::Flags,
            Group::UnicodeNames => Group::NerdFonts,
            Group::ShortCodes => Group::UnicodeNames,
            Group::RecentlyUsed => Group::ShortCodes,
        }
    }

    pub fn heading(self) -> &'static str {
        match self {
            Group::RecentlyUsed => "recent",
            Group::SmileysAndEmotion => "emotion",
            Group::PeopleAndBody => "people",
            Group::AnimalsAndNature => "animals",
            Group::FoodAndDrink => "food",
            Group::TravelAndPlaces => "travel",
            Group::Activities => "activities",
            Group::Objects => "objects",
            Group::Symbols => "symbols",
            Group::Flags => "flags",
            Group::NerdFonts => "nerdfonts",
            Group::UnicodeNames => "unicode",
            Group::ShortCodes => "shortcode",
        }
    }
}

impl Default for Group {
    /// Where the picker opens when nothing has ever been picked: the smileys,
    /// as in 0.57.4.
    fn default() -> Self {
        Group::SmileysAndEmotion
    }
}

/// Where the picker opens: on what was picked before, if anything ever was.
pub fn starting_group(recents: &[Choice]) -> Group {
    if recents.is_empty() {
        Group::default()
    } else {
        Group::RecentlyUsed
    }
}

/// Everything on offer apart from the recents, built once and kept.
///
/// It is a couple of hundred thousand rows -- the Unicode name table alone is
/// most of them -- and the shell asks again on every keystroke, so building
/// the list each time would be an allocation storm for rows nobody sees. The
/// order within is 0.57.4's build order: emoji with their shortcodes, then the
/// Unicode names, then the Nerd Font glyphs. Ties in the scoring keep this
/// order, so it is part of the behaviour rather than an accident.
pub fn catalog() -> &'static [Choice] {
    static CATALOG: OnceLock<Vec<Choice>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut choices = Vec::new();
        for emoji in emojis::iter() {
            let group = match emoji.group() {
                emojis::Group::SmileysAndEmotion => Group::SmileysAndEmotion,
                emojis::Group::PeopleAndBody => Group::PeopleAndBody,
                emojis::Group::AnimalsAndNature => Group::AnimalsAndNature,
                emojis::Group::FoodAndDrink => Group::FoodAndDrink,
                emojis::Group::TravelAndPlaces => Group::TravelAndPlaces,
                emojis::Group::Activities => Group::Activities,
                emojis::Group::Objects => Group::Objects,
                emojis::Group::Symbols => Group::Symbols,
                emojis::Group::Flags => Group::Flags,
            };
            match emoji.skin_tones() {
                // Every skin tone is its own row, as in 0.57.4: a thumbs up
                // in one tone is a different character, and a picker that
                // only offers the yellow one cannot type the others.
                Some(tones) => {
                    for tone in tones {
                        choices.push(Choice {
                            glyph: tone.as_str().to_string(),
                            name: Cow::Borrowed(tone.name()),
                            group,
                        });
                    }
                }
                None => choices.push(Choice {
                    glyph: emoji.as_str().to_string(),
                    name: Cow::Borrowed(emoji.name()),
                    group,
                }),
            }
            for shortcode in emoji.shortcodes() {
                choices.push(Choice {
                    glyph: emoji.as_str().to_string(),
                    name: Cow::Borrowed(shortcode),
                    group: Group::ShortCodes,
                });
            }
        }
        for (name, value) in crate::unicode_names::NAMES {
            let Some(ch) = char::from_u32(*value) else {
                continue;
            };
            choices.push(Choice {
                glyph: ch.to_string(),
                name: Cow::Borrowed(name),
                group: Group::UnicodeNames,
            });
        }
        for (name, value) in termwiz::nerdfonts::NERD_FONT_GLYPHS {
            choices.push(Choice {
                glyph: value.to_string(),
                name: Cow::Borrowed(name),
                group: Group::NerdFonts,
            });
        }
        choices
    })
}

/// What has been picked before, as rows for the picker, best first.
pub fn recent_choices() -> Vec<Choice> {
    remembered()
        .into_iter()
        .map(|recent| Choice {
            glyph: recent.glyph,
            name: Cow::Owned(recent.name),
            group: Group::RecentlyUsed,
        })
        .collect()
}

/// Whatever has been picked before, best first.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Recent {
    pub glyph: String,
    pub name: String,
    pub frecency: frecency::Frecency,
}

fn remembered_path() -> Option<std::path::PathBuf> {
    unterm_protocol::state_path("recent-characters.json")
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

// The matcher keeps internal scratch space, so each thread that scores gets
// its own rather than sharing one behind a lock. Same shape as 0.57.4.
thread_local! {
    static MATCHER: RefCell<nucleo_matcher::Matcher> =
        RefCell::new(nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT));
}

fn fuzzy_score(pattern: &Pattern, name: &str) -> Option<u32> {
    MATCHER.with_borrow_mut(|matcher| {
        let mut buf = Vec::new();
        pattern.score(Utf32Str::new(name, &mut buf), matcher)
    })
}

/// The offers that match `query`, best first.
///
/// 0.57.4's matching, kept exactly:
///
/// - An empty query shows the open group, in the order it was built.
/// - Anything typed is a nucleo fuzzy pattern over *every* group at once:
///   the groups are pages to browse, not fences for the search.
/// - A run of hex digits also means a codepoint, so `2014` finds the em dash
///   by number -- uppercased first, so `e1` finds `U+E1` rather than
///   HENTAIGANA LETTER E-1. So does anything spelled `U+...` outright.
/// - A name that equals the query outright beats every partial match.
/// - One row per glyph: an emoji found by name and by shortcode is still one
///   character, and its best score is the one that counts.
pub fn matching(
    recents: &[Choice],
    catalog: &[Choice],
    group: Group,
    query: &str,
    limit: usize,
) -> Vec<Choice> {
    if query.is_empty() {
        return recents
            .iter()
            .chain(catalog)
            .filter(|choice| choice.group == group)
            .take(limit)
            .cloned()
            .collect();
    }

    let pattern = Pattern::parse(
        query,
        nucleo_matcher::pattern::CaseMatching::Ignore,
        nucleo_matcher::pattern::Normalization::Smart,
    );
    let wanted_codepoint = if query.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(format!("U+{}", query.to_ascii_uppercase()))
    } else if query.starts_with("U+") {
        Some(query.to_string())
    } else {
        None
    };

    // Scored in parallel because the catalogue is a couple of hundred
    // thousand rows and this runs on every keystroke.
    let entries: Vec<&Choice> = recents.iter().chain(catalog).collect();
    let scored: Vec<(usize, u32)> = entries
        .par_iter()
        .enumerate()
        .filter_map(|(index, choice)| {
            // An exact name wins outright, otherwise the order among a crowd
            // of same-scored candidates buries the one that was asked for.
            let exact = |score: u32| {
                if choice.name == query {
                    u32::MAX
                } else {
                    score
                }
            };
            let by_name = fuzzy_score(&pattern, &choice.name).map(exact);
            let score = match &wanted_codepoint {
                Some(wanted) => {
                    let codepoints = choice.codepoints();
                    if codepoints == *wanted {
                        Some(u32::MAX)
                    } else {
                        match (by_name, fuzzy_score(&pattern, &codepoints)) {
                            (Some(a), Some(b)) => Some(a.max(b)),
                            (a, b) => a.or(b),
                        }
                    }
                }
                None => by_name,
            };
            score.map(|score| (index, score))
        })
        .collect();

    // One row per glyph, keeping its best score. First one in wins ties,
    // which puts a recent above the catalogue row for the same character.
    let mut best = HashMap::<&str, (u32, usize)>::new();
    for (index, score) in scored {
        let glyph = entries[index].glyph.as_str();
        match best.get(glyph) {
            Some((held, _)) if *held >= score => {}
            _ => {
                best.insert(glyph, (score, index));
            }
        }
    }

    // Best score first; ties keep the order the list was built in, which is
    // recents, then emoji, names, and glyphs -- 0.57.4's order made stable.
    let mut rows: Vec<(u32, usize)> = best.into_values().collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    rows.into_iter()
        .take(limit)
        .map(|(_, index)| entries[index].clone())
        .collect()
}

// Named for what it tests rather than `tests`, because cargo's filter is a
// plain substring and `select::tests::` would otherwise pick these up too.
#[cfg(test)]
mod picker_tests {
    use super::*;

    fn choice(glyph: &str, name: &'static str, group: Group) -> Choice {
        Choice {
            glyph: glyph.to_string(),
            name: Cow::Borrowed(name),
            group,
        }
    }

    fn sample() -> Vec<Choice> {
        vec![
            choice("\u{1F600}", "grinning face", Group::SmileysAndEmotion),
            choice("\u{1F926}", "person facepalming", Group::PeopleAndBody),
            choice("\u{1F680}", "rocket", Group::TravelAndPlaces),
            choice("\u{1F680}", "rocket", Group::ShortCodes),
            choice("\u{2014}", "EM DASH", Group::UnicodeNames),
            choice("\u{2192}", "RIGHTWARDS ARROW", Group::UnicodeNames),
            choice("\u{eb99}", "cod_account", Group::NerdFonts),
        ]
    }

    fn names(found: &[Choice]) -> Vec<&str> {
        found.iter().map(|choice| choice.name.as_ref()).collect()
    }

    fn search(query: &str) -> Vec<Choice> {
        matching(&[], &sample(), Group::default(), query, 10)
    }

    #[test]
    fn a_name_finds_its_character() {
        let found = search("rocket");
        assert_eq!(found.first().map(|c| c.glyph.as_str()), Some("\u{1F680}"));
    }

    /// Fuzzy, as 0.57.4 was: the letters in order find the name, and case
    /// does not matter. `em dash` finds `EM DASH`.
    #[test]
    fn matching_is_fuzzy_and_ignores_case() {
        assert_eq!(names(&search("em dash")), vec!["EM DASH"]);
        assert!(names(&search("grface")).contains(&"grinning face"));
    }

    /// Hex digits are a codepoint as well as letters: `2014` finds the em
    /// dash by number, and so does `U+2014` spelled out.
    #[test]
    fn a_codepoint_finds_its_character_by_number() {
        for query in ["2014", "U+2014"] {
            assert_eq!(
                search(query).first().map(|c| c.glyph.as_str()),
                Some("\u{2014}"),
                "{query} did not find the em dash"
            );
        }
    }

    /// A name that equals the query beats every longer name containing it.
    #[test]
    fn an_exact_name_beats_a_name_that_contains_it() {
        let rows = vec![
            choice("\u{1F680}", "rocket ship somewhere", Group::Objects),
            choice("\u{2708}", "rocket", Group::TravelAndPlaces),
        ];
        let found = matching(&[], &rows, Group::default(), "rocket", 10);
        assert_eq!(found.first().map(|c| c.glyph.as_str()), Some("\u{2708}"));
    }

    /// One row per glyph: the rocket found by name and by shortcode is still
    /// one character.
    #[test]
    fn a_glyph_is_offered_once_however_many_names_match() {
        let found = search("rocket");
        let rockets = found
            .iter()
            .filter(|choice| choice.glyph == "\u{1F680}")
            .count();
        assert_eq!(rockets, 1);
    }

    /// An empty query shows the open group, not everything: the groups are
    /// the pages Ctrl+R turns through.
    #[test]
    fn an_empty_query_shows_the_open_group() {
        let found = matching(&[], &sample(), Group::UnicodeNames, "", 10);
        assert_eq!(names(&found), vec!["EM DASH", "RIGHTWARDS ARROW"]);
        let found = matching(&[], &sample(), Group::NerdFonts, "", 10);
        assert_eq!(names(&found), vec!["cod_account"]);
    }

    /// And anything typed searches every group at once: the pages are for
    /// browsing, not fences for the search.
    #[test]
    fn a_query_reaches_across_every_group() {
        let found = matching(&[], &sample(), Group::Flags, "cod account", 10);
        assert_eq!(names(&found), vec!["cod_account"]);
    }

    /// Nothing matching is nothing offered, rather than everything.
    #[test]
    fn a_query_that_matches_nothing_offers_nothing() {
        assert!(search("zzzznothing").is_empty());
    }

    #[test]
    fn the_limit_is_respected() {
        for limit in [1, 2, 5, 100] {
            assert!(matching(&[], &sample(), Group::default(), "", limit).len() <= limit);
            assert!(matching(&[], &sample(), Group::default(), "o", limit).len() <= limit);
        }
    }

    /// The wheel of groups is 0.57.4's, comes back around in thirteen turns,
    /// and turns backwards to exactly where it came from.
    #[test]
    fn the_wheel_of_groups_comes_back_around() {
        let order = [
            Group::RecentlyUsed,
            Group::SmileysAndEmotion,
            Group::PeopleAndBody,
            Group::AnimalsAndNature,
            Group::FoodAndDrink,
            Group::TravelAndPlaces,
            Group::Activities,
            Group::Objects,
            Group::Symbols,
            Group::Flags,
            Group::NerdFonts,
            Group::UnicodeNames,
            Group::ShortCodes,
        ];
        for pair in order.windows(2) {
            assert_eq!(pair[0].next(), pair[1]);
            assert_eq!(pair[1].previous(), pair[0]);
        }
        assert_eq!(Group::ShortCodes.next(), Group::RecentlyUsed);
        assert_eq!(Group::RecentlyUsed.previous(), Group::ShortCodes);
    }

    /// The picker opens on the recents when there are any, and on the
    /// smileys -- 0.57.4's default -- when nothing was ever picked.
    #[test]
    fn the_picker_opens_where_0574_did() {
        assert_eq!(starting_group(&[]), Group::default());
        let recents = [choice("\u{2014}", "EM DASH", Group::RecentlyUsed)];
        assert_eq!(starting_group(&recents), Group::RecentlyUsed);
    }

    /// Every one of the twelve catalogue groups actually has rows in it --
    /// this is the parity that was lost, so it is pinned here.
    #[test]
    fn every_group_of_the_catalogue_is_populated() {
        let catalog = catalog();
        for group in [
            Group::SmileysAndEmotion,
            Group::PeopleAndBody,
            Group::AnimalsAndNature,
            Group::FoodAndDrink,
            Group::TravelAndPlaces,
            Group::Activities,
            Group::Objects,
            Group::Symbols,
            Group::Flags,
            Group::NerdFonts,
            Group::UnicodeNames,
            Group::ShortCodes,
        ] {
            assert!(
                catalog.iter().any(|choice| choice.group == group),
                "{group:?} has no rows"
            );
        }
    }

    /// The characters with no keys are all there, out of the full name table
    /// rather than a hand-picked list: an em dash, a non-breaking space, the
    /// box drawing, the euro sign. That is why anybody opens this.
    #[test]
    fn the_characters_with_no_keys_are_all_there() {
        let catalog = catalog();
        for wanted in ["\u{2014}", "\u{00A0}", "\u{2192}", "\u{2500}", "\u{20AC}"] {
            assert!(
                catalog.iter().any(|choice| choice.glyph == wanted),
                "{wanted:?} is not offered"
            );
        }
    }

    /// Every offer types something. A row that inserts nothing is a row that
    /// looks like it did not work.
    #[test]
    fn every_offer_types_something() {
        for choice in catalog() {
            assert!(!choice.glyph.is_empty(), "{:?} types nothing", choice.name);
            assert!(!choice.name.trim().is_empty(), "an offer with no name");
        }
    }

    /// The codepoints are spelled the way 0.57.4 spelled them, because the
    /// numeric search compares against exactly this spelling.
    #[test]
    fn codepoints_are_spelled_like_0574() {
        assert_eq!(
            choice("\u{2014}", "EM DASH", Group::UnicodeNames).codepoints(),
            "U+2014"
        );
        assert_eq!(
            choice(
                "\u{1F44D}\u{1F3FD}",
                "thumbs up: medium skin tone",
                Group::PeopleAndBody
            )
            .codepoints(),
            "U+1F44D U+1F3FD"
        );
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
