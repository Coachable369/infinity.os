//! Allocation-free compact-voice output conversion shared by native HDA and its hardware probe.
// ------------------------=
// FUNC: fill
// DESC: Converts bounded 8-kHz mono speech to negotiated HDA stereo, padding only the final DMA block with silence.
// ------------------=
pub fn fill(input: &[i16], output: &mut [i16], start: usize, rate: u32) -> bool {
    if !matches!(rate, 44_100 | 48_000) || output.len() % 2 != 0 || input.len() > 240_000 || start > 1_500_000 { return false; }
    for (i, pair) in output.chunks_exact_mut(2).enumerate() {
        let phase = (start + i) as u64 * 8000;
        let index = (phase / rate as u64) as usize;
        let fraction = (phase % rate as u64) as i64;
        let a = input.get(index).copied().unwrap_or(0) as i64;
        let b = input.get(index + 1).copied().unwrap_or(0) as i64;
        pair.fill((a + (b - a) * fraction / rate as i64) as i16);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: chunking_extrema_and_tail
    // DESC: Verifies phase continuity, identical stereo channels, extreme-amplitude arithmetic and tail silence.
    // ------------------=
    #[test]
    fn chunking_extrema_and_tail() {
        let input = [i16::MIN, i16::MAX, -16000, 16000, 0];
        for rate in [44100, 48000] {
            let mut whole = [0i16; 200];
            let mut chunked = [0i16; 200];
            assert!(fill(&input, &mut whole, 0, rate));
            for (i, chunk) in chunked.chunks_mut(14).enumerate() {
                assert!(fill(&input, chunk, i * 7, rate));
            }
            assert_eq!(whole, chunked);
            assert_eq!(whole[0], i16::MIN);
            assert!(whole.chunks_exact(2).all(|p| p[0] == p[1]));
            assert!(whole[60..].iter().all(|s| *s == 0));
        }
    }
    // ------------------------=
    // FUNC: rejects_invalid_formats_without_mutating_output
    // DESC: Rejects unsupported output configurations before exposing partial PCM.
    // ------------------=
    #[test]
    fn rejects_invalid_formats_without_mutating_output() {
        let mut output = [123; 4];
        assert!(!fill(&[1], &mut output, 0, 16000));
        assert_eq!(output, [123; 4]);
        assert!(!fill(&[1], &mut output[..3], 0, 48000));
        assert!(!fill(&[1], &mut output, 1500001, 48000));
        assert_eq!(output, [123; 4]);
    }
}
