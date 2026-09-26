//! Allocation-free compact-voice output conversion shared by native HDA and its hardware probe.
#[path = "speech_filter.rs"]
mod speech_filter;
// ------------------------=
// FUNC: fill
// DESC: Converts bounded 16-kHz mono speech to negotiated HDA stereo, padding only the final DMA block with silence.
// ------------------=
pub fn fill(input: &[i16], output: &mut [i16], start: usize, rate: u32) -> bool {
    fill_rate(input, output, start, 16_000, rate)
}

// ------------------------=
// FUNC: fill_rate
// DESC: Band-limits 16/24-kHz speech during upsampling, preserving pitch, duration and chunk phase without image-frequency harshness.
// ------------------=
pub fn fill_rate(input: &[i16], output: &mut [i16], start: usize, source_rate: u32, rate: u32) -> bool {
    if !matches!(source_rate, 16_000 | 24_000) || !matches!(rate, 44_100 | 48_000)
        || output.len() % 2 != 0 || input.len() > source_rate as usize * 30 || start > 1_500_000 { return false; }
    for (i, pair) in output.chunks_exact_mut(2).enumerate() {
        let phase = (start + i) as u64 * source_rate as u64;
        let index = (phase / rate as u64) as usize;
        if index >= input.len() { pair.fill(0); continue; }
        let fraction = ((phase % rate as u64) * 64 / rate as u64) as usize;
        let mut sum = 0i64;
        for (tap, coefficient) in speech_filter::PHASES[fraction].iter().enumerate() {
            let position = index as isize + tap as isize - 7;
            let sample = if position < 0 { 0 } else { input.get(position as usize).copied().unwrap_or(0) };
            sum += sample as i64 * *coefficient as i64;
        }
        pair.fill(((sum + 16384) >> 15).clamp(i16::MIN as i64, i16::MAX as i64) as i16);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: amplitude
    // DESC: Measures a waveform's frequency component away from finite-buffer boundaries.
    // ------------------=
    fn amplitude(samples: &[i16], rate: u32, frequency: f64) -> f64 {
        let mut re = 0.0; let mut im = 0.0;
        let mono = samples.len()/2;
        for i in 96..mono-96 {
            let phase = 2.0*std::f64::consts::PI*frequency*i as f64/rate as f64;
            re += samples[i*2] as f64*phase.cos(); im += samples[i*2] as f64*phase.sin();
        }
        2.0*(re*re+im*im).sqrt()/(mono-192) as f64
    }
    // ------------------------=
    // FUNC: suppresses_resampling_images_without_losing_speech_band
    // DESC: Measures actual PCM spectral energy against linear upsampling at both supported source and output rates.
    // ------------------=
    #[test]
    fn suppresses_resampling_images_without_losing_speech_band() {
        for source in [16000,24000] { for rate in [44100,48000] {
            let input: Vec<i16> = (0..source/10).map(|i|
                (10000.0*(2.0*std::f64::consts::PI*6000.0*i as f64/source as f64).sin()) as i16).collect();
            let mut output = vec![0; (rate/10*2) as usize];
            let mut linear = output.clone();
            assert!(fill_rate(&input,&mut output,0,source,rate));
            for (i,pair) in linear.chunks_exact_mut(2).enumerate() {
                let phase=i as u64*source as u64; let index=(phase/rate as u64) as usize;
                let a=input[index] as i64;let b=input.get(index+1).copied().unwrap_or(0) as i64;
                pair.fill((a+(b-a)*(phase%rate as u64) as i64/rate as i64) as i16);
            }
            assert!(amplitude(&output,rate,6000.0)>8000.0);
            let image=(source-6000) as f64;
            assert!(amplitude(&output,rate,image)<amplitude(&linear,rate,image)*0.15);
        }}
        for row in speech_filter::PHASES { assert_eq!(row.iter().map(|v|*v as i32).sum::<i32>(),32768); }
    }
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
