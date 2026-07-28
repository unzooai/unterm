//! Fuzzy matching shared by the launcher, palette, char select and dir jump.
//!
//! What remains of the selector overlay: the overlay itself existed to run a
//! Lua callback, but its matcher is what those four surfaces actually use.

use nucleo_matcher::pattern::Pattern;
use nucleo_matcher::{Matcher, Utf32Str};
use std::cell::RefCell;

thread_local! {
    static MATCHER: RefCell<Matcher> = RefCell::new(Matcher::default());
}

pub fn matcher_score(pattern: &Pattern, s: &str) -> Option<u32> {
    MATCHER.with_borrow_mut(|matcher| {
        let mut buf = vec![];
        pattern.score(Utf32Str::new(s, &mut buf), matcher)
    })
}

pub fn matcher_pattern(s: &str) -> Pattern {
    nucleo_matcher::pattern::Pattern::parse(
        s,
        nucleo_matcher::pattern::CaseMatching::Ignore,
        nucleo_matcher::pattern::Normalization::Smart,
    )
}
