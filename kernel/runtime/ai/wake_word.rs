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
    b"infin",
    b"finity",
];
const COMPUTER_ALIASES: &[&[u8]] = &[b"computer", b"compute her"];

// ------------------------=
// FUNC: is_separator
// DESC: Recognizes punctuation and whitespace allowed between a wake phrase and its command.
// ------------------=
const fn is_separator(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(byte, b',' | b'.' | b':' | b';' | b'!' | b'?' | b'-' | b'\'' | b'"')
}

// ------------------------=
// FUNC: separator_length
// DESC: Recognizes complete UTF-8 spacing and spoken punctuation without treating arbitrary non-ASCII suffixes as wake boundaries.
// ------------------=
fn separator_length(transcript: &[u8], at: usize) -> usize {
    match transcript.get(at..) {
        Some([byte, ..]) if is_separator(*byte) => 1,
        Some([0xc2, 0x85 | 0xa0, ..]) => 2,
        Some([0xe2, 0x80, byte, ..])
            if matches!(*byte, 0x80..=0x8a | 0x90..=0x95 | 0x98..=0x9f | 0xa6 | 0xa8 | 0xa9 | 0xaf) => 3,
        Some([0xe2, 0x81, 0x9f, ..]) | Some([0xe3, 0x80, 0x80, ..])
            | Some([0xe2, 0x88, 0x92, ..]) => 3,
        _ => 0,
    }
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
    // A clipped wake token must end here; accepting any non-alphanumeric byte
    // would also authorize names such as `infin_extra` or `finityé`.
    if end < transcript.len() && separator_length(transcript, end) == 0 {
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
// FUNC: skip_separators
// DESC: Advances across only the reviewed punctuation and whitespace accepted around a wake phrase.
// ------------------=
fn skip_separators(transcript: &[u8], mut at: usize) -> usize {
    loop {
        let length = separator_length(transcript, at);
        if length == 0 { return at; }
        at += length;
    }
}

// ------------------------=
// FUNC: match_wake
// DESC: Resolves one configured wake spelling at an exact bounded transcript position.
// ------------------=
fn match_wake(transcript: &[u8], at: usize, wake_word: WakeWord) -> Option<usize> {
    aliases(wake_word)
        .iter()
        .find_map(|alias| match_alias(transcript, at, alias))
}

// ------------------------=
// FUNC: match_address
// DESC: Accepts an optional Hey only when followed by an exact configured wake phrase, including repeated addressed prefixes.
// ------------------=
fn match_address(transcript: &[u8], mut at: usize, wake_word: WakeWord) -> Option<usize> {
    if transcript
        .get(at..at.saturating_add(3))
        .is_some_and(|prefix| {
            prefix
                .iter()
                .zip(b"hey")
                .all(|(left, right)| equal_folded(*left, *right))
                && separator_length(transcript, at + 3) != 0
        })
    {
        at = skip_separators(transcript, at + 3);
    }
    match_wake(transcript, at, wake_word)
}

// ------------------------=
// FUNC: classify
// DESC: Requires the configured phrase at the start and returns the exact command suffix after all repeated wake prefixes.
// ------------------=
pub fn classify(transcript: &[u8], wake_word: WakeWord) -> Match {
    let at = skip_separators(transcript, 0);
    let Some(mut at) = match_address(transcript, at, wake_word) else {
        return Match::Absent;
    };
    loop {
        at = skip_separators(transcript, at);
        let Some(next) = match_address(transcript, at, wake_word) else {
            break;
        };
        at = next;
    }
    if at == transcript.len() {
        Match::WakeOnly
    } else {
        Match::Command { start: at }
    }
}

// ------------------------=
// FUNC: deduplicated_command_end
// DESC: Removes only exact repeated wake-addressed command copies emitted inside one recognizer result.
// ------------------=
pub fn deduplicated_command_end(
    transcript: &[u8],
    wake_word: WakeWord,
    command_start: usize,
) -> usize {
    let mut repeated = [0usize; 4];
    let mut count = 0;
    let mut at = command_start;
    while at < transcript.len() && count < repeated.len() {
        if separator_length(transcript, at) != 0 {
            at = skip_separators(transcript, at);
            if let Some(end) = match_address(transcript, at, wake_word) {
                repeated[count] = at;
                count += 1;
                at = end;
            }
        } else {
            at += 1;
        }
    }
    if count == 0 {
        return transcript.len();
    }
    let first_end = trim_command_end(transcript, command_start, repeated[0]);
    let first = &transcript[command_start..first_end];
    if first.is_empty() {
        return transcript.len();
    }
    for index in 0..count {
        let marker = repeated[index];
        let Match::Command { start } = classify(&transcript[marker..], wake_word) else {
            return transcript.len();
        };
        let segment_start = marker + start;
        let segment_limit = if index + 1 < count {
            repeated[index + 1]
        } else {
            transcript.len()
        };
        let segment_end = trim_command_end(transcript, segment_start, segment_limit);
        if !equal_command(first, &transcript[segment_start..segment_end]) {
            return transcript.len();
        }
    }
    first_end
}

// ------------------------=
// FUNC: trim_command_end
// DESC: Trims separators from one candidate command boundary without changing command content.
// ------------------=
fn trim_command_end(transcript: &[u8], start: usize, end: usize) -> usize {
    let mut at = start;
    let mut trimmed = start;
    while at < end {
        let separator = separator_length(&transcript[..end], at);
        if separator != 0 {
            at += separator;
        } else {
            at += 1;
            trimmed = at;
        }
    }
    trimmed
}

// ------------------------=
// FUNC: equal_command
// DESC: Compares two complete recognized command copies case-insensitively while retaining punctuation boundaries.
// ------------------=
fn equal_command(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| equal_folded(*left, *right))
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
        assert_eq!(
            classify(b"Infinite, open settings", WakeWord::Infinity),
            Match::Command { start: 10 }
        );
        assert_eq!(classify(b"in finity", WakeWord::Infinity), Match::WakeOnly);
        assert_eq!(
            classify(b"InfinityOS: help", WakeWord::Infinity),
            Match::Command { start: 12 }
        );
        assert_eq!(
            classify(b"Hey, Infinity, help", WakeWord::Infinity),
            Match::Command { start: 15 }
        );
        assert_eq!(
            classify(b"compute her, help", WakeWord::Computer),
            Match::Command { start: 13 }
        );
    }

    // ------------------------=
    // FUNC: collapses_repeated_wake_prefixes_and_addressed_command_copies
    // DESC: Ensures recognizer repetition cannot leak wake words or duplicate an otherwise identical prompt.
    // ------------------=
    #[test]
    fn collapses_repeated_wake_prefixes_and_addressed_command_copies() {
        let repeated_wake = b"Infinity, Infinity, Infinity, open settings";
        let Match::Command { start } = classify(repeated_wake, WakeWord::Infinity) else {
            panic!("repeated wake phrase was not admitted");
        };
        assert_eq!(&repeated_wake[start..], b"open settings");

        for transcript in [
            b"Infinity, open settings. Infinity, open settings.".as_slice(),
            b"Infinity, open settings Infinity, open settings Infinity, open settings".as_slice(),
        ] {
            let Match::Command { start } = classify(transcript, WakeWord::Infinity) else {
                panic!("repeated addressed command was not admitted");
            };
            let end = super::deduplicated_command_end(transcript, WakeWord::Infinity, start);
            assert_eq!(&transcript[start..end], b"open settings");
        }
    }

    // ------------------------=
    // FUNC: preserves_intentional_wake_references_and_distinct_following_text
    // DESC: Proves deduplication requires complete identical addressed commands rather than merely seeing the wake word again.
    // ------------------=
    #[test]
    fn preserves_intentional_wake_references_and_distinct_following_text() {
        for transcript in [
            b"Infinity, explain the Infinity operating system".as_slice(),
            b"Infinity, open settings. Infinity, then explain why".as_slice(),
        ] {
            let Match::Command { start } = classify(transcript, WakeWord::Infinity) else {
                panic!("valid addressed command was not admitted");
            };
            assert_eq!(
                super::deduplicated_command_end(transcript, WakeWord::Infinity, start),
                transcript.len()
            );
        }
    }

    // ------------------------=
    // FUNC: accepts_clipped_wake_tokens_without_changing_the_command
    // DESC: Verifies reviewed clipped spellings, casing, optional hey and punctuation preserve the exact UTF-8 command suffix.
    // ------------------=
    #[test]
    fn accepts_clipped_wake_tokens_without_changing_the_command() {
        for wake in ["Infin", "finity", "Infinity", "INFIN", "FINity"] {
            for prefix in ["", "  ", "Hey, ", "HEY! "] {
                for separator in [" ", ", ", ": ", ". ", "! ", "? ", "- ", "; ", "\t"] {
                    let command = "open café; keep Infinity in this command!";
                    let addressed = std::format!("{prefix}{wake}{separator}{command}");
                    let Match::Command { start } =
                        classify(addressed.as_bytes(), WakeWord::Infinity)
                    else {
                        panic!("reviewed wake token did not admit a command");
                    };
                    assert_eq!(&addressed.as_bytes()[start..], command.as_bytes());
                    let wake_only = std::format!("{prefix}{wake}{separator}");
                    assert_eq!(
                        classify(wake_only.as_bytes(), WakeWord::Infinity),
                        Match::WakeOnly
                    );
                }
            }
        }
    }

    // ------------------------=
    // FUNC: clipped_wake_tokens_do_not_authorize_substrings_or_other_selections
    // DESC: Rejects ambient references, incomplete shorter fragments, attached suffixes and every Infinity alias when Computer is selected.
    // ------------------=
    #[test]
    fn clipped_wake_tokens_do_not_authorize_substrings_or_other_selections() {
        for transcript in [
            "please Infin help",
            "say finity",
            "definity help",
            "affinity help",
            "Infinityx help",
            "infinityosx help",
            "Infinx help",
            "finityful help",
            "Infin_extra help",
            "finity/help",
            "finityé help",
            "Infin123 help",
            "infi help",
            "in help",
            "finite help",
            "heyInfin help",
            "hey affinity help",
        ] {
            assert_eq!(
                classify(transcript.as_bytes(), WakeWord::Infinity),
                Match::Absent
            );
        }
        for alias in super::INFINITY_ALIASES {
            assert_eq!(classify(alias, WakeWord::Computer), Match::Absent);
        }
        for alias in super::COMPUTER_ALIASES {
            assert_eq!(classify(alias, WakeWord::Infinity), Match::Absent);
            assert_eq!(classify(alias, WakeWord::Computer), Match::WakeOnly);
        }
    }

    // ------------------------=
    // FUNC: unicode_boundaries_and_repeated_addresses_preserve_command_bytes
    // DESC: Accepts exact wake tokens around complete Unicode punctuation and removes repeated Hey prefixes without rewriting the command.
    // ------------------=
    #[test]
    fn unicode_boundaries_and_repeated_addresses_preserve_command_bytes() {
        for separator in ["\u{a0}", "\u{2009}", "\u{202f}", "—", "…", "“", "”", "\"", "'"] {
            let input = std::format!("{separator}Hey{separator}Infinity{separator}HEY, finity{separator}open settings");
            let Match::Command { start } = classify(input.as_bytes(), WakeWord::Infinity) else {
                panic!("exact repeated wake address rejected");
            };
            assert_eq!(&input.as_bytes()[start..], b"open settings");
            assert_eq!(classify(std::format!("Infinity{separator}Hey, Infinity{separator}").as_bytes(),
                WakeWord::Infinity), Match::WakeOnly);
        }
        let input = "Infinity—open settings… Hey, Infinity—open settings…";
        let Match::Command { start } = classify(input.as_bytes(), WakeWord::Infinity) else {
            panic!("exact wake address rejected");
        };
        let end = super::deduplicated_command_end(input.as_bytes(), WakeWord::Infinity, start);
        assert_eq!(&input.as_bytes()[start..end], b"open settings");
        let input = b"Infinity, hey there, keep this whole command";
        let Match::Command { start } = classify(input, WakeWord::Infinity) else {
            panic!("exact wake address rejected");
        };
        assert_eq!(&input[start..], b"hey there, keep this whole command");
    }

    // ------------------------=
    // FUNC: unicode_wake_boundaries_remain_fail_closed
    // DESC: Rejects attached letters, invisible joiners, malformed UTF-8 prefixes and the unselected wake phrase rather than matching arbitrary Unicode.
    // ------------------=
    #[test]
    fn unicode_wake_boundaries_remain_fail_closed() {
        for input in ["Infinityé help", "Infinity\u{200b}help", "Infinity\u{200d}help",
            "Infinity/help", "Infinity_extra help", "Infinity‖help", "Hey—affinity help",
            "they said—Infinity help", "HeyInfinity help"] {
            assert_eq!(classify(input.as_bytes(), WakeWord::Infinity), Match::Absent);
        }
        for input in [b"Infinity\xe2\x80".as_slice(), b"Infinity\xc2",
            b"\xe2\x80Infinity help", b"Infinity\xffhelp"] {
            assert_eq!(classify(input, WakeWord::Infinity), Match::Absent);
        }
        assert_eq!(classify("“Infinity”—help".as_bytes(), WakeWord::Computer), Match::Absent);
        assert_eq!(classify("Hey—Computer—help".as_bytes(), WakeWord::Infinity), Match::Absent);
    }
}
