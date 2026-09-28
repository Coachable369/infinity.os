//! Bounded multi-path echo subtraction before the existing microphone VAD.
//! This is not a replacement for a hardware acoustic echo canceller.

// ------------------------=
// FUNC: subtract
// DESC: Removes up to four correlated speaker paths, preserving independent near-end speech for interruption.
// ------------------=
pub fn subtract(mic: &mut [i16], reference: &[i16]) {
    for frame in mic.chunks_mut(320) {
        if frame.len()<80 || reference.len()<frame.len() {continue;}
        let original=frame.iter().map(|&v| {let v=(v as i64)>>4;v*v}).sum::<i64>();
        for _ in 0..4 {
        // Twelve-bit samples keep even the squared full-frame correlation in
        // i64. The soft-float kernel must not emulate millions of FP operations
        // on the desktop thread while the microphone is active.
        let energy=frame.iter().map(|&v| {let v=(v as i64)>>4;v*v}).sum::<i64>();
        if energy==0 {break;}
        let mut best=(0usize,0i64);
        for offset in 0..=reference.len()-frame.len() {
            let mut dot=0i64;let mut power=0i64;
            for (a,b) in frame.iter().step_by(4).zip(reference[offset..].iter().step_by(4)) {
                let a=(*a as i64)>>4;let b=(*b as i64)>>4;
                dot+=a*b;power+=b*b;
            }
            let score=dot*dot/power.max(1);
            if score>best.1 {best=(offset,score);}
        }
        let center=best.0;best.1=0;
        for offset in center.saturating_sub(3)..=(center+3).min(reference.len()-frame.len()) {
            let mut dot=0i64;let mut power=0i64;
            for (a,b) in frame.iter().zip(&reference[offset..]) {
                let a=(*a as i64)>>4;let b=(*b as i64)>>4;
                dot+=a*b;power+=b*b;
            }
            let score=dot*dot/power.max(1);
            if score>best.1 {best=(offset,score);}
        }
        // Do not fit an unrelated human voice to arbitrary playback noise.
        if best.1<energy/8 {break;}
        let mut dot=0i64;let mut power=0i64;
        for (a,b) in frame.iter().zip(&reference[best.0..]) {
            dot+=*a as i64 * *b as i64;power+=(*b as i64)*(*b as i64);
        }
        if power==0 {break;}
        let gain=((dot*(1<<20))/power).clamp(-(2<<20),2<<20);
        for (sample,echo) in frame.iter_mut().zip(&reference[best.0..]) {
            *sample=(*sample as i64-((gain*(*echo as i64))>>20)).clamp(-32768,32767) as i16;
        }
        let remaining=frame.iter().map(|&v| {let v=(v as i64)>>4;v*v}).sum::<i64>();
        // Only suppress tiny residuals after the playback has explained at
        // least 97% of the input. Uncorrelated near-end speech is not muted.
        if remaining<original/32 {frame.fill(0);break;}
        }
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
}
