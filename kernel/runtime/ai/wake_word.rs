//! Bounded, allocation-free wake-word parsing shared by native voice targets.

use crate::runtime::identity::WakeWord;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Match {
    Absent,
    WakeOnly,
    Command { start: usize },
}

const INFINITY_ALIASES: &[&[u8]] = &[
    b"infinity os",
    b"infinityos",
    b"in finity",
    b"infinite",
    b"infinity",
];
const COMPUTER_ALIASES: &[&[u8]] = &[b"computer", b"compute her"];

// ------------------------=
// FUNC: is_separator
// DESC: Recognizes punctuation and whitespace allowed between a wake phrase and its command.
// ------------------=
const fn is_separator(byte: u8) -> bool {
    byte.is_ascii_whitespace() || matches!(byte, b',' | b'.' | b':' | b';' | b'!' | b'?' | b'-')
}

// ------------------------=
// FUNC: equal_folded
// DESC: Compares one ASCII transcript byte to a wake-word byte without case sensitivity.
// ------------------=
const fn equal_folded(left: u8, right: u8) -> bool {
    left.to_ascii_lowercase() == right.to_ascii_lowercase()
}

// ------------------------=
// FUNC: match_alias
// DESC: Matches one bounded recognizer spelling and returns the first command byte after it.
// ------------------=
fn match_alias(transcript: &[u8], start: usize, alias: &[u8]) -> Option<usize> {
    if transcript.len().saturating_sub(start) < alias.len() {
        return None;
    }
    for (offset, expected) in alias.iter().enumerate() {
        if !equal_folded(transcript[start + offset], *expected) {
            return None;
        }
    }
    let end = start + alias.len();
    if end < transcript.len() && transcript[end].is_ascii_alphanumeric() {
        return None;
    }
    Some(end)
}

// ------------------------=
// FUNC: aliases
// DESC: Supplies only reviewed Whisper spellings for the configured local wake phrase.
// ------------------=
fn aliases(wake_word: WakeWord) -> &'static [&'static [u8]] {
    match wake_word {
        WakeWord::Infinity => INFINITY_ALIASES,
        WakeWord::Computer => COMPUTER_ALIASES,
    }
}

// ------------------------=
// FUNC: classify
// DESC: Requires the configured phrase at the start and returns the bounded command suffix.
// ------------------=
pub fn classify(transcript: &[u8], wake_word: WakeWord) -> Match {
    let mut at = 0;
    while at < transcript.len() && is_separator(transcript[at]) {
        at += 1;
    }
    if transcript.get(at..at.saturating_add(3)).is_some_and(|prefix| {
        prefix.iter().zip(b"hey").all(|(left, right)| equal_folded(*left, *right))
            && transcript.get(at + 3).is_some_and(|byte| is_separator(*byte))
    }) {
        at += 3;
        while at < transcript.len() && is_separator(transcript[at]) {
            at += 1;
        }
    }
    let Some(mut at) = aliases(wake_word)
        .iter()
        .find_map(|alias| match_alias(transcript, at, alias))
    else { return Match::Absent; };
    while at < transcript.len() && is_separator(transcript[at]) {
        at += 1;
    }
    if at == transcript.len() {
        Match::WakeOnly
    } else {
        Match::Command { start: at }
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, Match};
    use crate::runtime::identity::WakeWord;

    // ------------------------=
    // FUNC: matches_configured_phrase_and_command
    // DESC: Verifies case and punctuation do not alter an addressed command.
    // ------------------=
    #[test]
    fn matches_configured_phrase_and_command() {
        assert_eq!(
            classify(b"  infinity, what time is it?", WakeWord::Infinity),
            Match::Command { start: 12 }
        );
        assert_eq!(
            classify(b"COMPUTER: open settings", WakeWord::Computer),
            Match::Command { start: 10 }
        );
    }

    // ------------------------=
    // FUNC: distinguishes_wake_only_and_unaddressed_speech
    // DESC: Ensures a wake-only turn arms capture while unrelated speech remains ignored.
    // ------------------=
    #[test]
    fn distinguishes_wake_only_and_unaddressed_speech() {
        assert_eq!(classify(b"Infinity.", WakeWord::Infinity), Match::WakeOnly);
        assert_eq!(classify(b"Computer", WakeWord::Infinity), Match::Absent);
        assert_eq!(classify(b"identity", WakeWord::Infinity), Match::Absent);
    }

    // ------------------------=
    // FUNC: accepts_bounded_native_recognizer_variants
    // DESC: Keeps common tiny.en wake spellings and an optional conversational prefix usable without broad substring triggers.
    // ------------------=
    #[test]
    fn accepts_bounded_native_recognizer_variants() {
        assert_eq!(classify(b"Infinite, open settings", WakeWord::Infinity), Match::Command { start: 10 });
        assert_eq!(classify(b"in finity", WakeWord::Infinity), Match::WakeOnly);
        assert_eq!(classify(b"InfinityOS: help", WakeWord::Infinity), Match::Command { start: 12 });
        assert_eq!(classify(b"Hey, Infinity, help", WakeWord::Infinity), Match::Command { start: 15 });
        assert_eq!(classify(b"compute her, help", WakeWord::Computer), Match::Command { start: 13 });
    }
}
