//! Bounded sentence buffering for cumulative, visible assistant text.
// ------------------------=
// FUNC: next
// DESC: Releases a complete sentence or a bounded word-aligned chunk, never an unfinished streaming word.
// ------------------=
pub fn next(bytes: &[u8], complete: bool) -> usize {
    // Match the native graph's single-phrase bound. Larger requests make
    // Kokoro finish several graphs before publishing the first audible PCM.
    let limit = bytes.len().min(44);
    for i in 0..limit {
        if matches!(bytes[i], b'.' | b'!' | b'?')
            && (bytes.get(i + 1).is_some_and(u8::is_ascii_whitespace) || (complete && i + 1 == bytes.len())) {
            return i + 1;
        }
    }
    if complete && bytes.len() <= 44 { return bytes.len(); }
    if bytes.len() >= 44 {
        if let Some(i)=bytes[..limit].iter().rposition(u8::is_ascii_whitespace) {return i+1;}
        // An indivisible oversized word must reach the provider's explicit
        // rejection rather than leaving the conversation waiting forever.
        if complete || bytes.len()>=160 {return bytes.len().min(160);}
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
            let n = super::next(&text[at..end], end == text.len());
            spoken.extend_from_slice(&text[at..at+n]); at += n;
        }
        assert_eq!(spoken, text);
        assert_eq!(super::next(b"unfinished", false), 0);
        assert_eq!(super::next(b"1.25", false), 0);
        assert_eq!(super::next(&[b'a'; 170], false), 160);
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
            let count=super::next(&text[at..],true);
            assert!(count>0 && count<=44);
            assert!(at+count==text.len() || text[at+count-1].is_ascii_whitespace());
            at+=count;
        }
        assert!(super::next(&text[..45],false)>0);
    }
}
