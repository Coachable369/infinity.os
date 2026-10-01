//! Bounded, allocation-free wake-word parsing shared by native voice targets.

use crate::runtime::identity::WakeWord;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Match {
    Absent,
    WakeOnly,
    Command { start: usize },
}

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
// FUNC: classify
// DESC: Requires the configured phrase at the start and returns the bounded command suffix.
// ------------------=
pub fn classify(transcript: &[u8], wake_word: WakeWord) -> Match {
    let mut at = 0;
    while at < transcript.len() && is_separator(transcript[at]) {
        at += 1;
    }
    let phrase = wake_word.phrase();
    if transcript.len().saturating_sub(at) < phrase.len() {
        return Match::Absent;
    }
    for (offset, expected) in phrase.iter().enumerate() {
        if !equal_folded(transcript[at + offset], *expected) {
            return Match::Absent;
        }
    }
    at += phrase.len();
    if at < transcript.len() && transcript[at].is_ascii_alphanumeric() {
        return Match::Absent;
    }
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
        assert_eq!(classify(b"infinityOS", WakeWord::Infinity), Match::Absent);
    }
}
