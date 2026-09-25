//! Allocation-free compact-voice output conversion shared by native HDA and its hardware probe.
// ------------------------=
// FUNC: fill
// DESC: Converts bounded 16-kHz mono speech to negotiated HDA stereo, padding only the final DMA block with silence.
// ------------------=
pub fn fill(input: &[i16], output: &mut [i16], start: usize, rate: u32) -> bool {
    fill_rate(input, output, start, 16_000, rate)
}

// ------------------------=
// FUNC: fill_rate
// DESC: Converts bounded 16/24-kHz mono speech at its true source rate without changing pitch or duration.
// ------------------=
pub fn fill_rate(input: &[i16], output: &mut [i16], start: usize, source_rate: u32, rate: u32) -> bool {
    if !matches!(source_rate, 16_000 | 24_000) || !matches!(rate, 44_100 | 48_000)
        || output.len() % 2 != 0 || input.len() > source_rate as usize * 30 || start > 1_500_000 { return false; }
    for (i, pair) in output.chunks_exact_mut(2).enumerate() {
        let phase = (start + i) as u64 * source_rate as u64;
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
    // FUNC: neural_voice_rate_preserves_duration_and_chunk_phase
    // DESC: Verifies 24-kHz waveform sample positions, stereo duplication and chunk invariance at both HDA rates.
    // ------------------=
    #[test]
    fn neural_voice_rate_preserves_duration_and_chunk_phase() {
        let input = [0, 1000, 2000, 3000, 4000, 5000];
        for rate in [44_100, 48_000] {
            let mut whole = [0; 80];
            let mut chunks = [0; 80];
            assert!(fill_rate(&input, &mut whole, 0, 24_000, rate));
            for (i, chunk) in chunks.chunks_mut(10).enumerate() {
                assert!(fill_rate(&input, chunk, i * 5, 24_000, rate));
            }
            assert_eq!(whole, chunks);
            assert!(whole.chunks_exact(2).all(|p| p[0] == p[1]));
            assert!(whole[24..].iter().all(|s| *s == 0));
            if rate == 48_000 { assert_eq!(&whole[..8], &[0,0,500,500,1000,1000,1500,1500]); }
        }
        let mut output = [123; 4];
        assert!(!fill_rate(&input, &mut output, 0, 22_050, 48_000));
        assert_eq!(output, [123; 4]);
    }
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
