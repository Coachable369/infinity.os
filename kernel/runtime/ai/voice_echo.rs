//! Bounded multi-path echo subtraction before the existing microphone VAD.
//! This is not a replacement for a hardware acoustic echo canceller.

// ------------------------=
// FUNC: correlation_score
// DESC: Scores one echo lag with subsampling and overflow-safe full-range PCM arithmetic.
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
    if let Some(square) = dot.checked_mul(dot) {
        square / power.max(1)
    } else {
        ((i128::from(dot) * i128::from(dot)) / i128::from(power.max(1))) as i64
    }
}

// ------------------------=
// FUNC: best_offset
// DESC: Finds an acoustic echo lag with a fixed coarse-to-fine budget instead of scanning every lag at full resolution.
// ------------------=
fn best_offset(frame: &[i16], reference: &[i16]) -> (usize, i64) {
    const COARSE_SAMPLE_STEP: usize = 8;
    let last = reference.len() - frame.len();
    let mut best = (0usize, 0i64);
    for offset in (0..=last).step_by(if frame.len() < 1280 { 1 } else { 8 }) {
        let score = correlation_score(
            frame,
            reference,
            offset,
            if frame.len() < 1280 {
                4
            } else {
                COARSE_SAMPLE_STEP
            },
        );
        if score > best.1 {
            best = (offset, score);
        }
    }
    let coarse = best.0;
    best.1 = 0;
    for offset in coarse.saturating_sub(8)..=(coarse + 8).min(last) {
        let score = correlation_score(frame, reference, offset, 1);
        if score > best.1 {
            best = (offset, score);
        }
    }
    best
}

// ------------------------=
// FUNC: fit
// DESC: Fits bounded speaker paths over a speech-scale window, returning delay and fixed-point gain.
// ------------------=
fn fit(mic: &mut [i16], reference: &[i16]) -> [(usize, i64); 4] {
    let mut paths = [(0, 0); 4];
    for path in &mut paths {
        let energy = mic.iter().map(|&x| (i64::from(x) >> 4).pow(2)).sum::<i64>();
        if energy == 0 || reference.len() < mic.len() {
            break;
        }
        let (offset, score) = best_offset(mic, reference);
        if score < energy / 8 {
            break;
        }
        let mut dot = 0i64;
        let mut power = 0i64;
        for (&m, &r) in mic.iter().zip(&reference[offset..]) {
            dot += i64::from(m) * i64::from(r);
            power += i64::from(r).pow(2);
        }
        if power == 0 {
            break;
        }
        let gain = ((dot * (1 << 20)) / power).clamp(-(2 << 20), 2 << 20);
        *path = (reference.len() - mic.len() - offset, gain);
        for (m, &r) in mic.iter_mut().zip(&reference[offset..]) {
            *m = (i64::from(*m) - ((gain * i64::from(r)) >> 20)).clamp(-32768, 32767) as i16;
        }
    }
    paths
}
pub struct Canceller {
    history: [i16; 4800],
    work: [i16; 4800],
    filled: usize,
    since_fit: usize,
    paths: [(usize, i64); 4],
    learned: bool,
}
impl Canceller {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves bounded private microphone history and echo path state without allocation.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            history: [0; 4800],
            work: [0; 4800],
            filled: 0,
            since_fit: 0,
            paths: [(0, 0); 4],
            learned: false,
        }
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases microphone history and learned paths at owner or reference boundaries.
    // ------------------=
    pub fn clear(&mut self) {
        self.history.fill(0);
        self.work.fill(0);
        self.filled = 0;
        self.since_fit = 0;
        self.learned = false;
        self.paths = [(0, 0); 4];
    }
    // ------------------------=
    // FUNC: process
    // DESC: Emits current samples immediately, fitting at most every 80 ms and freezing learned paths during double talk.
    // ------------------=
    pub fn process(&mut self, mic: &mut [i16], reference: &[i16]) {
        let n = mic.len();
        if n == 0 {
            return;
        }
        if n > 4800 || reference.len() < n {
            self.clear();
            return;
        }
        let keep = self
            .filled
            .min(4800 - n)
            .min(reference.len().saturating_sub(n));
        self.history.copy_within(self.filled - keep..self.filled, 0);
        self.history[keep..keep + n].copy_from_slice(mic);
        self.filled = keep + n;
        if mic.iter().all(|&sample| sample == 0) {
            return;
        }
        self.since_fit = self.since_fit.saturating_add(n);
        let refit = self.since_fit >= 1280;
        if refit {
            self.since_fit = 0;
            self.work[..self.filled].copy_from_slice(&self.history[..self.filled]);
            let original = self.work[..self.filled]
                .iter()
                .map(|&x| i64::from(x).pow(2))
                .sum::<i64>();
            let paths = fit(&mut self.work[..self.filled], reference);
            let residual = self.work[..self.filled]
                .iter()
                .map(|&x| i64::from(x).pow(2))
                .sum::<i64>();
            if original > 0 && residual < original / 16 {
                self.paths = paths;
                self.learned = true;
            }
        }
        if self.learned {
            for (i, m) in mic.iter_mut().enumerate() {
                let mut value = i64::from(*m);
                for &(lag, gain) in &self.paths {
                    if let Some(at) = reference.len().checked_sub(n + lag) {
                        value -= (gain * i64::from(reference[at + i])) >> 20;
                    }
                }
                *m = value.clamp(-32768, 32767) as i16;
            }
        } else if refit {
            mic.copy_from_slice(&self.work[keep..self.filled]);
        }
    }
}

#[cfg(test)]
// ------------------------=
// FUNC: subtract
// DESC: Exercises the bounded fitting primitive in single-window unit fixtures.
// ------------------=
pub fn subtract(mic: &mut [i16], reference: &[i16]) {
    let _ = fit(mic, reference);
}
#[cfg(test)]
mod tests {
    // ------------------------=
    // FUNC: streaming_double_talk_preserves_user_and_erases_private_state
    // DESC: Exercises incremental DMA sizes, trained echo, independent interruption, silent input, and session reset.
    // ------------------=
    #[test]
    fn streaming_double_talk_preserves_user_and_erases_private_state() {
        let mut seed = 51u32;
        let speaker: Vec<i16> = (0..32000)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 16) as i16) / 4
            })
            .collect();
        for block in [80, 320, 1600] {
            let mut filter = super::Canceller::new();
            let mut error = 0i64;
            let mut echo_energy = 0i64;
            for at in (0..speaker.len()).step_by(block) {
                let end = (at + block).min(speaker.len());
                let mut reference = vec![0; 9600];
                let count = end.min(9600);
                reference[9600 - count..].copy_from_slice(&speaker[end - count..end]);
                let human: Vec<i16> = (at..end)
                    .map(|i| {
                        if (16000..24000).contains(&i) {
                            if i % 23 < 11 {
                                2600
                            } else {
                                -2600
                            }
                        } else {
                            0
                        }
                    })
                    .collect();
                let mut mic: Vec<i16> = (at..end)
                    .zip(&human)
                    .map(|(i, &h)| {
                        let echo = if i >= 800 { speaker[i - 800] / 2 } else { 0 };
                        if at >= 9600 {
                            echo_energy += i64::from(echo).pow(2);
                        }
                        echo + h
                    })
                    .collect();
                filter.process(&mut mic, &reference);
                if at >= 9600 {
                    error += mic
                        .iter()
                        .zip(&human)
                        .map(|(&m, &h)| (i64::from(m) - i64::from(h)).pow(2))
                        .sum::<i64>();
                }
            }
            assert!(filter.learned);
            assert!(
                error < echo_energy / 100,
                "block={block} error={error} echo={echo_energy}"
            );
            let mut silent = vec![0; block];
            filter.process(&mut silent, &speaker[speaker.len() - 9600..]);
            assert!(silent.iter().all(|&v| v == 0));
            filter.clear();
            assert!(!filter.learned);
            assert_eq!(filter.filled, 0);
            assert!(filter.history.iter().chain(&filter.work).all(|&v| v == 0));
            assert_eq!(filter.paths, [(0, 0); 4]);
        }
    }
    // ------------------------=
    // FUNC: reflected_playback_does_not_become_a_new_utterance
    // DESC: Exercises a direct speaker path plus two delayed room reflections and independent near-end speech.
    // ------------------=
    #[test]
    fn reflected_playback_does_not_become_a_new_utterance() {
        let mut seed = 19u32;
        let reference: Vec<i16> = (0..9600)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 16) as i16) / 3
            })
            .collect();
        let echo: Vec<i16> = (0..320)
            .map(|i| reference[6400 + i] / 2 + reference[6100 + i] / 3 + reference[5600 + i] / 5)
            .collect();
        let mut cleaned = echo.clone();
        super::subtract(&mut cleaned, &reference);
        let energy = cleaned.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() / 320;
        assert!(
            energy < 300 * 300,
            "residual playback can trigger VAD: {energy}"
        );
        let human: Vec<i16> = (0..320)
            .map(|i| if i % 23 < 11 { 2400 } else { -2400 })
            .collect();
        let mut mixed: Vec<i16> = echo.iter().zip(&human).map(|(a, b)| a + b).collect();
        super::subtract(&mut mixed, &reference);
        let error = mixed
            .iter()
            .zip(&human)
            .map(|(a, b)| (i64::from(*a) - i64::from(*b)).pow(2))
            .sum::<i64>()
            / 320;
        assert!(
            error < 2400 * 2400 / 5,
            "near-end voice was damaged: {error}"
        );
    }
    // ------------------------=
    // FUNC: echo_and_double_talk
    // DESC: Verifies delayed attenuated speaker rejection while retaining independently generated near-end speech.
    // ------------------=
    #[test]
    fn echo_and_double_talk() {
        let mut seed = 7u32;
        let reference: Vec<i16> = (0..1600)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 16) as i16) / 4
            })
            .collect();
        let mut echo: Vec<i16> = reference[397..717].iter().map(|v| v / 2).collect();
        super::subtract(&mut echo, &reference);
        assert!(echo.iter().all(|v| v.unsigned_abs() <= 1));
        let human: Vec<i16> = (0..320)
            .map(|i| if i % 23 < 11 { 1500 } else { -1500 })
            .collect();
        let mut mixed: Vec<i16> = reference[397..717]
            .iter()
            .zip(&human)
            .map(|(e, h)| e / 2 + h)
            .collect();
        super::subtract(&mut mixed, &reference);
        let error = mixed
            .iter()
            .zip(&human)
            .map(|(a, b)| (*a as f64 - *b as f64).powi(2))
            .sum::<f64>();
        assert!(error / 320.0 < 1500.0 * 1500.0 * 0.1);
        let mut unchanged = human.clone();
        super::subtract(&mut unchanged, &[]);
        assert_eq!(unchanged, human);
        let mut unchanged = human.clone();
        super::subtract(&mut unchanged, &reference);
        assert_eq!(unchanged, human);
        // Full-range samples at the production reference size must not overflow
        // correlation or fixed-point subtraction in debug or optimized builds.
        let reference = vec![i16::MIN; 9600];
        let mut full = vec![i16::MIN; 4800];
        super::subtract(&mut full, &reference);
        assert!(full.iter().all(|&v| v == 0));
    }

    // ------------------------=
    // FUNC: production_batch_tracks_echo_without_erasing_interruption
    // DESC: Exercises a full 300-ms microphone read, constant echo lag tracking, and near-end speech in a later frame.
    // ------------------=
    #[test]
    fn production_batch_tracks_echo_without_erasing_interruption() {
        let mut seed = 29u32;
        let reference: Vec<i16> = (0..9600)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 16) as i16) / 4
            })
            .collect();
        let human: Vec<i16> = (0..320)
            .map(|i| if i % 19 < 9 { 2200 } else { -2200 })
            .collect();
        let mut mic = Vec::with_capacity(4800);
        for frame in 0..15 {
            for index in 0..320 {
                let echo = reference[2400 + frame * 320 + index] / 2;
                mic.push(if frame == 7 {
                    echo + human[index]
                } else {
                    echo
                });
            }
        }
        super::subtract(&mut mic, &reference);
        for (frame, samples) in mic.chunks(320).enumerate() {
            let energy = samples
                .iter()
                .map(|&value| i64::from(value).pow(2))
                .sum::<i64>()
                / 320;
            if frame == 7 {
                let error = samples
                    .iter()
                    .zip(&human)
                    .map(|(actual, expected)| (i64::from(*actual) - i64::from(*expected)).pow(2))
                    .sum::<i64>()
                    / 320;
                assert!(
                    error < 2200 * 2200 / 5,
                    "near-end interruption was damaged: {error}"
                );
            } else {
                assert!(
                    energy < 300 * 300,
                    "tracked echo reached VAD in frame {frame}: {energy}"
                );
            }
        }
    }
}
