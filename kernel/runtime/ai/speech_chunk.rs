//! Bounded phrase buffering for cumulative, visible assistant text.
const PROVIDER_LIMIT: usize = 160;

// ------------------------=
// FUNC: sentence_period
// DESC: Keeps common abbreviations and initials inside their sentence instead of manufacturing a spoken pause.
// ------------------=
fn sentence_period(bytes: &[u8], at: usize) -> bool {
    let start = bytes[..at]
        .iter()
        .rposition(u8::is_ascii_whitespace)
        .map_or(0, |i| i + 1);
    let word = &bytes[start..at];
    if word.len() == 1 && word[0].is_ascii_alphabetic() {
        return false;
    }
    ![b"dr".as_slice(), b"mr", b"mrs", b"ms", b"prof", b"sr", b"jr", b"st", b"vs", b"etc", b"e.g", b"i.e"]
        .iter()
        .any(|abbreviation| word.eq_ignore_ascii_case(abbreviation))
}

// ------------------------=
// FUNC: next
// DESC: Releases natural sentence or clause boundaries, falling back to a whole-word split only at the native provider's hard bound.
// ------------------=
pub fn next(bytes: &[u8], complete: bool, initial: bool) -> usize {
    // Coalesce available short sentences before dispatch. Sending "Hello!"
    // separately from the rest of a short answer needlessly runs two graphs.
    // Each bounded span still becomes resident before hardware playback.
    // Do not turn every few streamed words into a separate native graph. A
    // first clause can start early; followups prefer longer coherent phrases.
    // Unpunctuated streams still dispatch by 160 available bytes, so buffering
    // cannot wait for an entire long response. This is a text bound, not a
    // promise about model generation or synthesis wall-clock latency.
    // A complete short opening sentence is speakable now; do not hold it
    // behind the following sentence merely to reach a character quota.
    let min_sentence = if initial { 1 } else { 24 };
    let min_clause = if initial { 40 } else { 80 };
    let scan_limit = bytes.len().min(PROVIDER_LIMIT);
    if complete && bytes.len() <= PROVIDER_LIMIT {
        return bytes.len();
    }
    let mut sentence_end = 0;
    let mut clause_end = 0;
    for i in 0..scan_limit {
        if !bytes.get(i + 1).is_some_and(u8::is_ascii_whitespace)
            && !(complete && i + 1 == bytes.len())
        {
            continue;
        }
        if matches!(bytes[i], b'!' | b'?') || (bytes[i] == b'.' && sentence_period(bytes, i)) {
            sentence_end = i + 1;
        } else if matches!(bytes[i], b',' | b';' | b':') {
            clause_end = i + 1;
        }
    }
    if sentence_end >= min_sentence {
        return sentence_end;
    }
    if clause_end >= min_clause {
        return clause_end;
    }
    if bytes.len() >= PROVIDER_LIMIT {
        // A shorter genuine boundary is preferable to splitting the next
        // sentence merely because its generation reached the provider limit.
        if sentence_end != 0 || clause_end != 0 {
            return sentence_end.max(clause_end);
        }
        if let Some(i) = bytes[..PROVIDER_LIMIT]
            .iter()
            .rposition(u8::is_ascii_whitespace)
        {
            return i + 1;
        }
        // An indivisible oversized word must reach the provider's explicit
        // rejection rather than leaving the conversation waiting forever.
        return PROVIDER_LIMIT;
    }
    0
}

#[cfg(test)]
mod tests {
    // ------------------------=
    // FUNC: short_opening_sentence_starts_without_waiting_for_followup
    // DESC: Releases a complete greeting but never an unfinished word or abbreviation.
    // ------------------=
    #[test]
    fn short_opening_sentence_starts_without_waiting_for_followup() {
        assert_eq!(super::next(b"Hello! I", false, true), 6);
        assert_eq!(super::next(b"Hello", false, true), 0);
        assert_eq!(super::next(b"Dr. Smith", false, true), 0);
        assert_eq!(super::next(b"Hello! I", false, false), 0);
    }
    // ------------------------=
    // FUNC: cumulative_text_has_no_repeated_or_partial_sentence
    // DESC: Verifies stream buffering, final flushing, length bounds and exact once-only consumption.
    // ------------------=
    #[test]
    fn cumulative_text_has_no_repeated_or_partial_sentence() {
        let text = b"Hello there. This is a fluid response! And a final fragment";
        let mut at = 0;
        let mut spoken = Vec::new();
        for end in 1..=text.len() {
            let n = super::next(&text[at..end], end == text.len(), at == 0);
            spoken.extend_from_slice(&text[at..at + n]);
            at += n;
        }
        assert_eq!(spoken, text);
        assert_eq!(super::next(b"unfinished", false, true), 0);
        assert_eq!(super::next(b"1.25", false, true), 0);
        assert_eq!(super::next(&[b'a'; 170], false, false), 160);
    }

    // ------------------------=
    // FUNC: streaming_waits_for_a_natural_boundary
    // DESC: Verifies that slow token delivery does not manufacture pauses after the former 24-byte or 80-byte thresholds.
    // ------------------=
    #[test]
    fn streaming_waits_for_a_natural_boundary() {
        let text=b"This response has enough words to begin speaking before the entire sentence has been generated.";
        for available in 0..=text.len() {
            assert_eq!(super::next(&text[..available], false, true), 0);
        }
        assert_eq!(super::next(text, true, true), text.len());
    }

    // ------------------------=
    // FUNC: lookahead_stays_within_one_native_graph
    // DESC: Verifies bounded first dispatch and lossless final followup batching.
    // ------------------=
    #[test]
    fn lookahead_stays_within_one_native_graph() {
        let sentence=b"The queued followup phrase stays together, so playback does not stop and restart every few words.";
        let followup = super::next(sentence, true, false);
        assert_eq!(followup, sentence.len());
        let first = super::next(sentence, true, true);
        assert_eq!(first, sentence.len());
    }

    // ------------------------=
    // FUNC: unpunctuated_streams_remain_bounded
    // DESC: Prevents indefinite buffering without inventing small fragments or dropping bytes at the provider limit.
    // ------------------=
    #[test]
    fn unpunctuated_streams_remain_bounded() {
        let text = b"word ".repeat(80);
        for initial in [false, true] {
            for available in 0..160 {
                assert_eq!(super::next(&text[..available], false, initial), 0);
            }
            assert_eq!(super::next(&text[..160], false, initial), 160);
        }
        let mut at = 0;
        let mut output = Vec::new();
        while at < text.len() {
            let count = super::next(&text[at..], true, at == 0);
            assert!(count > 0 && count <= 160);
            output.extend_from_slice(&text[at..at + count]);
            at += count;
        }
        assert_eq!(output, text);
    }

    // ------------------------=
    // FUNC: short_sentences_share_one_synthesis_request
    // DESC: Coalesces already completed replies but starts a streamed greeting without waiting for the next sentence.
    // ------------------=
    #[test]
    fn short_sentences_share_one_synthesis_request() {
        let text = b"Hello! How can I help you today?";
        for available in 1..7 {
            assert_eq!(super::next(&text[..available], false, true), 0);
        }
        for available in 7..29 {
            assert_eq!(super::next(&text[..available], false, true), 6);
        }
        assert_eq!(super::next(text, true, true), text.len());
        let followup = b"Yes. It is ready. You can use it now.";
        assert_eq!(super::next(followup, true, false), followup.len());
        assert_eq!(super::next(b"Hi!", true, true), 3);
    }

    // ------------------------=
    // FUNC: clauses_start_before_a_long_reply_finishes
    // DESC: Checks natural early dispatch, fast lookahead coalescing and abbreviation-safe sentence boundaries through actual text inputs.
    // ------------------=
    #[test]
    fn clauses_start_before_a_long_reply_finishes() {
        let clause = b"The current configuration has been saved safely,";
        let mut text = clause.to_vec();
        text.extend_from_slice(b" and the remaining work continues in the background.");
        assert_eq!(super::next(&text[..clause.len() + 1], false, true), clause.len());
        assert_eq!(super::next(&text[..clause.len() + 1], false, false), 0);
        assert_eq!(super::next(&text, true, true), text.len());
        for abbreviation in [b"Dr.".as_slice(), b"Prof.", b"e.g.", b"A."] {
            let mut text = b"Please send the requested summary to ".to_vec();
            text.extend_from_slice(abbreviation);
            text.extend_from_slice(b" Smith");
            assert_eq!(super::next(&text, false, true), 0);
        }
        let sentence = b"The current configuration has been saved safely.";
        let mut text = sentence.to_vec();
        text.extend_from_slice(b" More details are still being generated");
        assert_eq!(super::next(&text, false, true), sentence.len());
        assert_eq!(super::next(b"The value is 1234567890.25 and still increasing", false, true), 0);
    }
}
