//! Bounded multi-path echo subtraction before the existing microphone VAD.
//! This is not a replacement for a hardware acoustic echo canceller.

// ------------------------=
// FUNC: correlation_score
// DESC: Scores one bounded echo lag with subsampling while keeping full-range PCM arithmetic inside i64.
// ------------------=
fn correlation_score(frame: &[i16], reference: &[i16], offset: usize, stride: usize) -> i64 {
    let mut dot = 0i64;
    let mut power = 0i64;
    for (mic, speaker) in frame
        .iter()
        .step_by(stride)
        .zip(reference[offset..].iter().step_by(stride))
    {
        let mic = (i64::from(*mic)) >> 4;
        let speaker = (i64::from(*speaker)) >> 4;
        dot += mic * speaker;
        power += speaker * speaker;
    }
    dot * dot / power.max(1)
}

// ------------------------=
// FUNC: best_offset
// DESC: Finds an acoustic echo lag with a fixed coarse-to-fine budget instead of scanning every lag at full resolution.
// ------------------=
fn best_offset(frame: &[i16], reference: &[i16]) -> (usize, i64) {
    const COARSE_SAMPLE_STEP: usize = 4;
    let last = reference.len() - frame.len();
    let mut best = (0usize, 0i64);
    for offset in 0..=last {
        let score = correlation_score(frame, reference, offset, COARSE_SAMPLE_STEP);
        if score > best.1 { best = (offset, score); }
    }
    let coarse = best.0;
    best.1 = 0;
    for offset in coarse.saturating_sub(3)..=(coarse + 3).min(last) {
        let score = correlation_score(frame, reference, offset, 1);
        if score > best.1 { best = (offset, score); }
    }
    best
}

// ------------------------=
// FUNC: nearby_offset
// DESC: Tracks a previously discovered acoustic path across consecutive microphone frames with constant bounded work.
// ------------------=
fn nearby_offset(frame: &[i16], reference: &[i16], expected: usize) -> (usize, i64) {
    let last = reference.len() - frame.len();
    let mut best = (expected.min(last), 0i64);
    for offset in expected.saturating_sub(4)..=(expected + 4).min(last) {
        let score = correlation_score(frame, reference, offset, 1);
        if score > best.1 { best = (offset, score); }
    }
    best
}

// ------------------------=
// FUNC: subtract
// DESC: Removes up to four correlated speaker paths, preserving independent near-end speech for interruption.
// ------------------=
pub fn subtract(mic: &mut [i16], reference: &[i16]) {
    const MAX_FRAMES: usize = 15;
    let mut original = [0i64; MAX_FRAMES];
    for (index, frame) in mic.chunks(320).enumerate().take(MAX_FRAMES) {
        original[index] = frame.iter().map(|&value| { let value = i64::from(value) >> 4; value * value }).sum();
    }
    for _ in 0..4 {
        let mut path: Option<(usize, usize)> = None;
        for (index, frame) in mic.chunks_mut(320).enumerate().take(MAX_FRAMES) {
            if frame.len() < 80 || reference.len() < frame.len() { continue; }
            // Twelve-bit samples keep the scored full-frame correlation in i64.
            let energy = frame.iter().map(|&value| { let value = i64::from(value) >> 4; value * value }).sum::<i64>();
            if energy == 0 { continue; }
            let best = if let Some((first_frame, first_offset)) = path {
                nearby_offset(frame, reference, first_offset.saturating_add((index - first_frame) * 320))
            } else {
                best_offset(frame, reference)
            };
            // Do not fit an unrelated human voice to arbitrary playback noise.
            if best.1 < energy / 8 { continue; }
            path.get_or_insert((index, best.0));
            let mut dot = 0i64;
            let mut power = 0i64;
            for (sample, echo) in frame.iter().zip(&reference[best.0..]) {
                dot += i64::from(*sample) * i64::from(*echo);
                power += i64::from(*echo) * i64::from(*echo);
            }
            if power == 0 { continue; }
            let gain = ((dot * (1 << 20)) / power).clamp(-(2 << 20), 2 << 20);
            for (sample, echo) in frame.iter_mut().zip(&reference[best.0..]) {
                *sample = (i64::from(*sample) - ((gain * i64::from(*echo)) >> 20))
                    .clamp(-32768, 32767) as i16;
            }
        }
    }
    for (index, frame) in mic.chunks_mut(320).enumerate().take(MAX_FRAMES) {
        let remaining = frame.iter().map(|&value| { let value = i64::from(value) >> 4; value * value }).sum::<i64>();
        // Only suppress tiny residuals after playback explained at least 97%
        // of the input. Uncorrelated near-end speech remains available to VAD.
        if remaining < original[index] / 32 { frame.fill(0); }
    }
}

#[cfg(test)]
mod tests {
    // ------------------------=
    // FUNC: reflected_playback_does_not_become_a_new_utterance
    // DESC: Exercises a direct speaker path plus two delayed room reflections and independent near-end speech.
    // ------------------=
    #[test]
    fn reflected_playback_does_not_become_a_new_utterance() {
        let mut seed=19u32;
        let reference:Vec<i16>=(0..9600).map(|_| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);((seed>>16) as i16)/3}).collect();
        let echo:Vec<i16>=(0..320).map(|i| reference[6400+i]/2+reference[6100+i]/3+reference[5600+i]/5).collect();
        let mut cleaned=echo.clone();
        super::subtract(&mut cleaned,&reference);
        let energy=cleaned.iter().map(|&v|i64::from(v).pow(2)).sum::<i64>()/320;
        assert!(energy<300*300,"residual playback can trigger VAD: {energy}");
        let human:Vec<i16>=(0..320).map(|i|if i%23<11 {2400}else{-2400}).collect();
        let mut mixed:Vec<i16>=echo.iter().zip(&human).map(|(a,b)|a+b).collect();
        super::subtract(&mut mixed,&reference);
        let error=mixed.iter().zip(&human).map(|(a,b)|(i64::from(*a)-i64::from(*b)).pow(2)).sum::<i64>()/320;
        assert!(error<2400*2400/5,"near-end voice was damaged: {error}");
    }
    // ------------------------=
    // FUNC: echo_and_double_talk
    // DESC: Verifies delayed attenuated speaker rejection while retaining independently generated near-end speech.
    // ------------------=
    #[test]
    fn echo_and_double_talk() {
        let mut seed=7u32;
        let reference:Vec<i16>=(0..1600).map(|_| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);((seed>>16) as i16)/4}).collect();
        let mut echo:Vec<i16>=reference[397..717].iter().map(|v|v/2).collect();
        super::subtract(&mut echo,&reference);
        assert!(echo.iter().all(|v|v.unsigned_abs()<=1));
        let human:Vec<i16>=(0..320).map(|i|if i%23<11 {1500}else{-1500}).collect();
        let mut mixed:Vec<i16>=reference[397..717].iter().zip(&human).map(|(e,h)|e/2+h).collect();
        super::subtract(&mut mixed,&reference);
        let error=mixed.iter().zip(&human).map(|(a,b)|(*a as f64-*b as f64).powi(2)).sum::<f64>();
        assert!(error/320.0<1500.0*1500.0*0.1);
        let mut unchanged=human.clone();super::subtract(&mut unchanged,&[]);assert_eq!(unchanged,human);
        let mut unchanged=human.clone();super::subtract(&mut unchanged,&reference);assert_eq!(unchanged,human);
        // Full-range samples at the production reference size must not overflow
        // correlation or fixed-point subtraction in debug or optimized builds.
        let reference=vec![i16::MIN;9600];let mut full=vec![i16::MIN;4800];
        super::subtract(&mut full,&reference);assert!(full.iter().all(|&v|v==0));
    }

    // ------------------------=
    // FUNC: production_batch_tracks_echo_without_erasing_interruption
    // DESC: Exercises a full 300-ms microphone read, constant echo lag tracking, and near-end speech in a later frame.
    // ------------------=
    #[test]
    fn production_batch_tracks_echo_without_erasing_interruption() {
        let mut seed=29u32;
        let reference:Vec<i16>=(0..9600).map(|_| {
            seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);
            ((seed>>16) as i16)/4
        }).collect();
        let human:Vec<i16>=(0..320).map(|i|if i%19<9 {2200}else{-2200}).collect();
        let mut mic=Vec::with_capacity(4800);
        for frame in 0..15 {
            for index in 0..320 {
                let echo=reference[2400+frame*320+index]/2;
                mic.push(if frame==7 {echo+human[index]} else {echo});
            }
        }
        super::subtract(&mut mic,&reference);
        for (frame,samples) in mic.chunks(320).enumerate() {
            let energy=samples.iter().map(|&value|i64::from(value).pow(2)).sum::<i64>()/320;
            if frame==7 {
                let error=samples.iter().zip(&human).map(|(actual,expected)|
                    (i64::from(*actual)-i64::from(*expected)).pow(2)).sum::<i64>()/320;
                assert!(error<2200*2200/5,"near-end interruption was damaged: {error}");
            } else {
                assert!(energy<300*300,"tracked echo reached VAD in frame {frame}: {energy}");
            }
        }
    }
}
