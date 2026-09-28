//! Bounded microphone conversion to speech-provider 16 kHz mono PCM.
//! Coefficients are prepared once, never during capture. No heap or host DSP.
const TAPS: usize = 96;
const PHASES: usize = 160;
const OUTPUT_RATE: u32 = 16_000;

pub struct Resampler {
    rate: u32,
    phase: u32,
    cursor: usize,
    history: [i16; TAPS],
    coefficients: [[i32; TAPS]; PHASES],
}

impl Resampler {
    // ------------------------=
    // FUNC: empty
    // DESC: Reserves fixed filter storage in an inactive state suitable for static initialization.
    // ------------------=
    pub const fn empty() -> Self {
        Self { rate: 0, phase: 0, cursor: 0, history: [0; TAPS], coefficients: [[0; TAPS]; PHASES] }
    }

    // ------------------------=
    // FUNC: configure
    // DESC: Prepares a normalized Blackman-windowed sinc anti-alias filter for supported HDA rates.
    // ------------------=
    pub fn configure(&mut self, rate: u32) -> bool {
        self.clear();
        if self.rate == rate && rate != 0 { return true; }
        self.rate = 0;
        if !matches!(rate, 16_000 | 44_100 | 48_000) { return false; }
        if rate != OUTPUT_RATE {
            let pi = core::f64::consts::PI;
            let cutoff = 6_800.0 / rate as f64;
            for phase in 0..PHASES {
                let fraction = phase as f64 / PHASES as f64;
                let mut sum = 0.0;
                let mut values = [0.0f64; TAPS];
                for tap in 0..TAPS {
                    let x = tap as f64 - (TAPS - 1) as f64 / 2.0 - fraction;
                    let sinc = if x.abs() < 1e-12 { 2.0 * cutoff }
                        else { libm::sin(2.0 * pi * cutoff * x) / (pi * x) };
                    let angle = 2.0 * pi * tap as f64 / (TAPS - 1) as f64;
                    let window = 0.42 - 0.5 * libm::cos(angle) + 0.08 * libm::cos(2.0 * angle);
                    let value = sinc * window;
                    values[tap] = value;
                    sum += value;
                }
                for tap in 0..TAPS {
                    self.coefficients[phase][tap] = libm::round(values[tap] / sum * (1u64 << 30) as f64) as i32;
                }
            }
        }
        self.rate = rate;
        true
    }

    // ------------------------=
    // FUNC: clear
    // DESC: Erases retained microphone history and phase without recalculating reusable coefficients.
    // ------------------=
    pub fn clear(&mut self) { self.history.fill(0); self.phase = 0; self.cursor = 0; }

    // ------------------------=
    // FUNC: process
    // DESC: Returns consumed and produced counts, preserving unconsumed input under output backpressure.
    // ------------------=
    pub fn process(&mut self, input: &[i16], output: &mut [i16]) -> (usize, usize) {
        if self.rate == 0 || output.is_empty() { return (0, 0); }
        if self.rate == OUTPUT_RATE {
            let n = input.len().min(output.len());
            output[..n].copy_from_slice(&input[..n]);
            return (n, n);
        }
        let mut consumed = 0;
        let mut produced = 0;
        for &sample in input {
            if produced == output.len() { break; }
            self.history[self.cursor] = sample;
            self.cursor = (self.cursor + 1) % TAPS;
            consumed += 1;
            self.phase += OUTPUT_RATE;
            if self.phase >= self.rate {
                self.phase -= self.rate;
                // Both 44.1 kHz and 48 kHz land exactly on these 160 fractional phases.
                let bank = (self.phase / 100) as usize;
                // Integer MACs avoid soft-float helper calls on the UI owner.
                // 96 signed Q30 taps at full-scale PCM remain well inside i64.
                let mut value = 0i64;
                for tap in 0..TAPS {
                    let index = (self.cursor + TAPS - 1 - tap) % TAPS;
                    value += self.history[index] as i64 * self.coefficients[bank][tap] as i64;
                }
                let rounded = if value < 0 { -((-value + (1 << 29)) >> 30) } else { (value + (1 << 29)) >> 30 };
                output[produced] = rounded.clamp(i16::MIN as i64, i16::MAX as i64) as i16;
                produced += 1;
            }
        }
        (consumed, produced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: private_history_is_erased
    // DESC: Verifies actual retained sample storage is cleared between microphone sessions.
    // ------------------=
    fn private_history_is_erased() {
        let mut r = Resampler::empty();
        assert!(r.configure(48_000));
        r.process(&[1234; 300], &mut [0; 100]);
        assert!(r.history.iter().any(|s| *s != 0));
        r.clear();
        assert!(r.history.iter().all(|s| *s == 0));
        assert_eq!((r.phase, r.cursor), (0, 0));
        assert!(!r.configure(0));
        assert_eq!(r.process(&[1; 32], &mut [0; 32]), (0, 0));
    }
}
