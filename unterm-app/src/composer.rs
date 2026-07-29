//! A queue of prompts, fed to an agent one at a time.
//!
//! Running an agent CLI is mostly waiting: you give it something, it works,
//! it comes back, you give it the next thing. The waiting is what the composer
//! removes -- write the whole batch while you are thinking about it, and each
//! one goes in as the pane comes free.
//!
//! Two rules make it safe to leave running, and both are here rather than in
//! the event loop so they can be checked. Nothing is sent while the pane is
//! busy, because a prompt typed into a working agent is a prompt typed into
//! whatever it is showing. And nothing is sent twice: a queue that resends on
//! a redraw would fire the same prompt sixty times a second.

/// Prompts waiting to go in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Composer {
    queued: Vec<String>,
    /// What is being typed now, before Enter puts it in the queue.
    pub typing: String,
    /// True once a prompt has gone in and the pane has not gone idle since.
    ///
    /// An agent takes a moment to start working, and a pane that is still
    /// idle in that moment would take the next prompt on top of the first.
    sent_and_waiting: bool,
}

impl Composer {
    /// Put what is being typed into the queue.
    ///
    /// Blank lines are dropped rather than queued: Enter on an empty line is
    /// how anyone finishes a list, and a blank prompt sent to an agent is a
    /// bare newline that answers whatever it happened to be asking.
    pub fn commit(&mut self) {
        let prompt = self.typing.trim().to_string();
        self.typing.clear();
        if !prompt.is_empty() {
            self.queued.push(prompt);
        }
    }

    pub fn queued(&self) -> &[String] {
        &self.queued
    }

    pub fn is_empty(&self) -> bool {
        self.queued.is_empty() && self.typing.trim().is_empty()
    }

    /// Take the next prompt if the pane is ready for one.
    ///
    /// `idle` is the pane's own answer to "is anything running". Nothing goes
    /// in while something is, and nothing goes in until the pane has been idle
    /// since the last one -- an agent that has not started working yet still
    /// looks idle, and two prompts on one line is one prompt with the other
    /// stuck to it.
    pub fn take_next(&mut self, idle: bool) -> Option<String> {
        if !idle {
            // It started working: the next idle moment is a real one.
            self.sent_and_waiting = false;
            return None;
        }
        if self.sent_and_waiting {
            return None;
        }
        if self.queued.is_empty() {
            return None;
        }
        self.sent_and_waiting = true;
        Some(self.queued.remove(0))
    }

    /// Drop the queue, keeping what is being typed.
    pub fn clear(&mut self) {
        self.queued.clear();
        self.sent_and_waiting = false;
    }
}

/// Whether a line is an agent asking permission to go on.
///
/// Deliberately narrow. Auto-advancing past a question the agent actually
/// needed answered differently is worse than stopping: the prompts that match
/// are the ones offering a yes as the obvious answer, and anything else waits
/// for a person. A pattern that matched loosely would answer "delete these
/// files? [y/N]" with a yes.
pub fn is_confirmation(line: &str) -> bool {
    let line = line.trim().to_lowercase();
    if line.is_empty() {
        return false;
    }
    // The bracketed choice at the end is the tell, and the capitalised
    // default in the original tells us which way it leans.
    const AFFIRMATIVE: &[&str] = &["[y/n]", "(y/n)", "[yes/no]", "y/n?", "[Y/n]"];
    AFFIRMATIVE
        .iter()
        .any(|shape| line.ends_with(&shape.to_lowercase()))
        && !line.contains("delete")
        && !line.contains("remove")
        && !line.contains("overwrite")
        && !line.contains("force")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(prompts: &[&str]) -> Composer {
        let mut composer = Composer::default();
        for prompt in prompts {
            composer.typing = prompt.to_string();
            composer.commit();
        }
        composer
    }

    #[test]
    fn prompts_queue_in_the_order_they_were_written() {
        let composer = with(&["first", "second"]);
        assert_eq!(composer.queued(), ["first", "second"]);
    }

    /// Enter on an empty line is how anyone finishes a list. Queuing that
    /// blank sends a bare newline, which answers whatever the agent was
    /// asking at the time.
    #[test]
    fn a_blank_line_is_not_a_prompt() {
        let mut composer = with(&["real"]);
        composer.typing = "   ".to_string();
        composer.commit();
        assert_eq!(composer.queued(), ["real"]);
    }

    #[test]
    fn a_prompt_keeps_its_words_but_loses_its_edges() {
        let composer = with(&["  fix the flaky test  "]);
        assert_eq!(composer.queued(), ["fix the flaky test"]);
    }

    /// A prompt typed into a working agent goes into whatever it is showing.
    #[test]
    fn nothing_is_sent_while_the_pane_is_busy() {
        let mut composer = with(&["first"]);
        assert_eq!(composer.take_next(false), None);
        assert_eq!(composer.queued().len(), 1, "and it stays queued");
    }

    #[test]
    fn an_idle_pane_gets_the_first_prompt() {
        let mut composer = with(&["first", "second"]);
        assert_eq!(composer.take_next(true).as_deref(), Some("first"));
        assert_eq!(composer.queued(), ["second"]);
    }

    /// The whole queue would go in on one line otherwise. An agent takes a
    /// moment to start working, and the pane is still idle in that moment --
    /// so idleness alone is not permission to send the next one.
    #[test]
    fn the_next_prompt_waits_for_the_pane_to_actually_start() {
        let mut composer = with(&["first", "second"]);
        assert_eq!(composer.take_next(true).as_deref(), Some("first"));
        assert_eq!(composer.take_next(true), None, "still idle, still waiting");
        assert_eq!(composer.take_next(true), None);

        // It started working, then came back.
        assert_eq!(composer.take_next(false), None);
        assert_eq!(composer.take_next(true).as_deref(), Some("second"));
    }

    #[test]
    fn an_empty_queue_sends_nothing_however_idle_the_pane_is() {
        let mut composer = Composer::default();
        assert_eq!(composer.take_next(true), None);
    }

    #[test]
    fn clearing_drops_the_queue_and_keeps_what_is_being_typed() {
        let mut composer = with(&["first", "second"]);
        composer.typing = "half written".to_string();
        composer.clear();
        assert!(composer.queued().is_empty());
        assert_eq!(composer.typing, "half written");
    }

    /// And clearing releases the wait, so a fresh queue is not held back by
    /// a prompt sent before it.
    #[test]
    fn clearing_lets_the_next_queue_start_immediately() {
        let mut composer = with(&["first"]);
        composer.take_next(true);
        composer.clear();
        composer.typing = "new".to_string();
        composer.commit();
        assert_eq!(composer.take_next(true).as_deref(), Some("new"));
    }

    #[test]
    fn a_composer_with_nothing_in_it_knows_so() {
        let mut composer = Composer::default();
        assert!(composer.is_empty());
        composer.typing = "x".to_string();
        assert!(!composer.is_empty());
    }

    #[test]
    fn an_ordinary_yes_no_question_is_a_confirmation() {
        assert!(is_confirmation("Proceed with the change? [y/n]"));
        assert!(is_confirmation("Apply this patch? (y/n)"));
        assert!(is_confirmation("Continue? [yes/no]"));
    }

    /// The narrowness is the point. Answering "delete these files? [y/n]"
    /// with a yes because the shape matched is the failure this guards.
    #[test]
    fn a_destructive_question_is_never_answered_automatically() {
        assert!(!is_confirmation("Delete 12 files? [y/n]"));
        assert!(!is_confirmation("Remove the branch? [y/n]"));
        assert!(!is_confirmation("Overwrite config.toml? [y/n]"));
        assert!(!is_confirmation("Force push to main? [y/n]"));
    }

    #[test]
    fn ordinary_output_is_not_a_question() {
        assert!(!is_confirmation("Compiling unterm-app v0.1.0"));
        assert!(!is_confirmation(""));
        assert!(!is_confirmation("what do you think?"));
    }
}
