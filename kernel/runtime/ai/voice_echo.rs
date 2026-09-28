//! Bounded direct-path echo subtraction before the existing microphone VAD.
//! This is not a replacement for a hardware acoustic echo canceller.

// ------------------------=
// FUNC: subtract
// DESC: Finds delayed playback correlated with each microphone frame and removes only that component, preserving uncorrelated near-end speech.
// ------------------=
pub fn subtract(mic: &mut [i16], reference: &[i16]) {
    for frame in mic.chunks_mut(320) {
        if frame.len()<80 || reference.len()<frame.len() {continue;}
        // Twelve-bit samples keep even the squared full-frame correlation in
        // i64. The soft-float kernel must not emulate millions of FP operations
        // on the desktop thread while the microphone is active.
        let energy=frame.iter().map(|&v| {let v=(v as i64)>>4;v*v}).sum::<i64>();
        if energy==0 {continue;}
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
        if best.1<energy/4 {continue;}
        let mut dot=0i64;let mut power=0i64;
        for (a,b) in frame.iter().zip(&reference[best.0..]) {
            dot+=*a as i64 * *b as i64;power+=(*b as i64)*(*b as i64);
        }
        if power==0 {continue;}
        let gain=((dot*(1<<20))/power).clamp(-(2<<20),2<<20);
        for (sample,echo) in frame.iter_mut().zip(&reference[best.0..]) {
            *sample=(*sample as i64-((gain*(*echo as i64))>>20)).clamp(-32768,32767) as i16;
        }
    }
}

#[cfg(test)]
mod tests {
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
        // Full-range samples at the production reference size must not overflow
        // correlation or fixed-point subtraction in debug or optimized builds.
        let reference=vec![i16::MIN;9600];let mut full=vec![i16::MIN;4800];
        super::subtract(&mut full,&reference);assert!(full.iter().all(|&v|v==0));
    }
}
