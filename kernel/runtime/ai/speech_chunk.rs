//! Bounded sentence buffering for cumulative, visible assistant text.
// ------------------------=
// FUNC: next
// DESC: Releases a complete sentence or a bounded word-aligned chunk, preserving low first-audio latency and longer lookahead prosody.
// ------------------=
pub fn next(bytes: &[u8], complete: bool, initial: bool) -> usize {
    // The native bridge safely divides this outer phrase into 44-byte model
    // segments and concatenates their PCM before one DMA submission. Keep the
    // whole sentence resident so segment boundaries cannot stop the device.
    const INITIAL_LIMIT: usize = 160;
    const PROVIDER_LIMIT: usize = 160;
    const FIRST_WORD_WINDOW: usize = 24;
    let phrase_limit = if initial { INITIAL_LIMIT } else { PROVIDER_LIMIT };
    let scan_limit = bytes.len().min(phrase_limit);
    for i in 0..scan_limit {
        if matches!(bytes[i], b'.' | b'!' | b'?')
            && (bytes.get(i + 1).is_some_and(u8::is_ascii_whitespace) || (complete && i + 1 == bytes.len())) {
            return i + 1;
        }
    }
    // The first audible phrase must follow visible text by only a few words.
    // Waiting for sentence punctuation made a long native response appear to
    // be queued for close to a minute even though tokens were already visible.
    if initial && !complete && bytes.len() >= FIRST_WORD_WINDOW {
        if let Some(offset) = bytes[FIRST_WORD_WINDOW..scan_limit]
            .iter()
            .position(u8::is_ascii_whitespace)
        {
            return FIRST_WORD_WINDOW + offset + 1;
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
        assert_eq!(super::next(&text[..23],false,true),0);
        assert_eq!(super::next(&text[..29],false,true),25);
        assert_eq!(super::next(&text[..38],false,true),25);
    }

    // ------------------------=
    // FUNC: lookahead_stays_within_one_native_graph
    // DESC: Verifies queued followup speech is ready after one native graph while the first phrase retains its lower-latency bound.
    // ------------------=
    #[test]
    fn lookahead_stays_within_one_native_graph() {
        let sentence=b"The queued followup phrase stays together, so playback does not stop and restart every few words.";
        let followup=super::next(sentence,true,false);
        assert_eq!(followup,sentence.len());
        let first=super::next(sentence,true,true);
        assert_eq!(first,sentence.len());
    }

    // ------------------------=
    // FUNC: streaming_chunks_match_native_graph_boundaries
    // DESC: Ensures followup text publishes one native graph at a time while initial speech trails visible text by only a few words.
    // ------------------=
    #[test]
    fn streaming_chunks_match_native_graph_boundaries() {
        let text=b"One two three four five six seven eight nine ten eleven twelve thirteen fourteen.";
        assert_eq!(super::next(text,false,true),28);
        let complete=super::next(text,true,true);
        assert_eq!(complete,text.len());
    }
}
