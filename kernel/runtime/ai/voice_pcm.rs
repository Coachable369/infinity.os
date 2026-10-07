//! Bounded microphone conversion to speech-provider 16 kHz mono PCM.
//! Fixed coefficients are embedded in both live and installed kernels. No heap or host DSP.
const TAPS: usize = 96;
const PHASES: usize = 160;
const OUTPUT_RATE: u32 = 16_000;
const COEFFICIENT_BYTES: usize = TAPS * PHASES * 4;
static FILTER_44100: &[u8; COEFFICIENT_BYTES] = include_bytes!("voice-pcm-44100.bin");
static FILTER_48000: &[u8; COEFFICIENT_BYTES] = include_bytes!("voice-pcm-48000.bin");

pub struct Resampler {
    rate: u32,
    phase: u32,
    cursor: usize,
    history: [i16; TAPS],
    coefficients: &'static [u8; COEFFICIENT_BYTES],
}

impl Resampler {
    // ------------------------=
    // FUNC: empty
    // DESC: Reserves fixed filter storage in an inactive state suitable for static initialization.
    // ------------------=
    pub const fn empty() -> Self {
        Self { rate: 0, phase: 0, cursor: 0, history: [0; TAPS], coefficients: FILTER_48000 }
    }

    // ------------------------=
    // FUNC: configure
    // DESC: Selects a precomputed anti-alias filter without blocking input on software floating-point initialization.
    // ------------------=
    pub fn configure(&mut self, rate: u32) -> bool {
        self.clear();
        self.rate = 0;
        self.coefficients = match rate {
            44_100 => FILTER_44100,
            16_000 | 48_000 => FILTER_48000,
            _ => return false,
        };
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
                    let offset = (bank * TAPS + tap) * 4;
                    let coefficient = i32::from_le_bytes(self.coefficients[offset..offset + 4].try_into().unwrap());
                    value += self.history[index] as i64 * coefficient as i64;
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
    // FUNC: cold_configuration_reuses_embedded_filters
    // DESC: Checks cold and repeated rate changes select immutable filters and reset sample state.
    // ------------------=
    fn cold_configuration_reuses_embedded_filters() {
        assert!(core::mem::size_of::<Resampler>() < 256);
        let mut r = Resampler::empty();
        for rate in [44_100, 48_000, 16_000, 48_000, 44_100] {
            assert!(r.configure(rate));
            let expected = if rate == 44_100 { FILTER_44100 } else { FILTER_48000 };
            assert!(core::ptr::eq(r.coefficients, expected));
            assert_eq!((r.phase, r.cursor), (0, 0));
            assert!(r.history.iter().all(|sample| *sample == 0));
            assert!(r.process(&[1234; 300], &mut [0; 300]).1 > 0);
        }
    }
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
