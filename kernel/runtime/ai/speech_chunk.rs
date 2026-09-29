//! Bounded sentence buffering for cumulative, visible assistant text.
// ------------------------=
// FUNC: next
// DESC: Releases a complete sentence or a bounded word-aligned chunk, preserving low first-audio latency and longer lookahead prosody.
// ------------------=
pub fn next(bytes: &[u8], complete: bool, initial: bool) -> usize {
    const INITIAL_LIMIT: usize = 44;
    const PROVIDER_LIMIT: usize = 160;
    let phrase_limit = if initial { INITIAL_LIMIT } else { PROVIDER_LIMIT };
    let scan_limit = bytes.len().min(phrase_limit);
    for i in 0..scan_limit {
        if matches!(bytes[i], b'.' | b'!' | b'?')
            && (bytes.get(i + 1).is_some_and(u8::is_ascii_whitespace) || (complete && i + 1 == bytes.len())) {
            return i + 1;
        }
    }
    if complete && bytes.len() <= phrase_limit { return bytes.len(); }
    if bytes.len() >= phrase_limit {
        if let Some(i)=bytes[..phrase_limit].iter().rposition(u8::is_ascii_whitespace) {return i+1;}
        // An indivisible oversized word must reach the provider's explicit
        // rejection rather than leaving the conversation waiting forever.
        if complete || bytes.len() >= PROVIDER_LIMIT {return bytes.len().min(PROVIDER_LIMIT);}
    }
    0
}

#[cfg(test)]
mod tests {
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
            spoken.extend_from_slice(&text[at..at+n]); at += n;
        }
        assert_eq!(spoken, text);
        assert_eq!(super::next(b"unfinished", false, true), 0);
        assert_eq!(super::next(b"1.25", false, true), 0);
        assert_eq!(super::next(&[b'a'; 170], false, false), 160);
    }

    // ------------------------=
    // FUNC: publishes_one_native_graph_without_waiting_for_sentence_end
    // DESC: Verifies early word-aligned dispatch and lossless draining across native phrase boundaries.
    // ------------------=
    #[test]
    fn publishes_one_native_graph_without_waiting_for_sentence_end() {
        let text=b"This response has enough words to begin speaking before the entire sentence has been generated.";
        let mut at=0;
        while at<text.len() {
            let count=super::next(&text[at..],true,at==0);
            assert!(count>0 && count<=160);
            assert!(at+count==text.len() || text[at+count-1].is_ascii_whitespace());
            at+=count;
        }
        assert!(super::next(&text[..45],false,true)>0);
    }

    // ------------------------=
    // FUNC: lookahead_keeps_followup_sentence_together
    // DESC: Verifies queued followup speech preserves sentence prosody while the first phrase retains its early bound.
    // ------------------=
    #[test]
    fn lookahead_keeps_followup_sentence_together() {
        let sentence=b"The queued followup phrase stays together, so playback does not stop and restart every few words.";
        assert_eq!(super::next(sentence,true,false),sentence.len());
        let first=super::next(sentence,true,true);
        assert!(first>0 && first<=44 && first<sentence.len());
    }
}
