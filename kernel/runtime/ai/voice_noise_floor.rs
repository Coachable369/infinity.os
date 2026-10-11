//! Bounded startup noise measurement; retains energy only, never microphone PCM.
const FRAME: usize = 320;
const FRAMES: usize = 25;

pub struct NoiseFloor {
    samples: usize,
    frames: usize,
    energy: u64,
    minimum: u64,
    threshold: u16,
}
impl NoiseFloor {
    // ------------------------=
    // FUNC: new
    // DESC: Starts a half-second 16-kHz calibration with the established minimum speech threshold.
    // ------------------=
    pub const fn new() -> Self {
        Self { samples: 0, frames: 0, energy: 0, minimum: u64::MAX, threshold: 300 }
    }
    // ------------------------=
    // FUNC: ready
    // DESC: Reports completion from actual captured samples, not elapsed wall time.
    // ------------------=
    pub const fn ready(&self) -> bool { self.frames == FRAMES }
    // ------------------------=
    // FUNC: threshold
    // DESC: Returns a bounded speech floor with six decibels of separation from measured ambient sound.
    // ------------------=
    pub const fn threshold(&self) -> u16 { self.threshold }
    // ------------------------=
    // FUNC: push
    // DESC: Measures quietest complete frames so isolated clicks cannot inflate the calibrated threshold.
    // ------------------=
    pub fn push(&mut self, pcm: &[i16]) -> usize {
        let mut consumed = 0;
        for &sample in pcm {
            if self.ready() { break; }
            consumed += 1;
            self.samples += 1;
            self.energy += (i64::from(sample) * i64::from(sample)) as u64;
            if self.samples == FRAME {
                self.minimum = self.minimum.min(self.energy / FRAME as u64);
                self.samples = 0;
                self.energy = 0;
                self.frames += 1;
                if self.ready() {
                    let mut low = 0u64;
                    let mut high = 32769u64;
                    while high - low > 1 {
                        let middle = (low + high) / 2;
                        if middle * middle <= self.minimum { low = middle; } else { high = middle; }
                    }
                    self.threshold = (low * 2).clamp(300, 3000) as u16;
                }
            }
        }
        consumed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: bounded_sample_based_calibration
    // DESC: Verifies chunk independence, measured noise rejection, impulse resistance and threshold bounds.
    // ------------------=
    #[test]
    fn bounded_sample_based_calibration() {
        for (ambient, expected) in [(0, 300), (500, 1000), (700, 1400), (32767, 3000)] {
            let mut floor = NoiseFloor::new();
            assert_eq!(floor.push(&[32767; FRAME]), FRAME);
            for _ in 0..24 { floor.push(&[ambient; FRAME]); }
            assert!(floor.ready());
            assert_eq!(floor.threshold(), expected);
            assert_eq!(floor.push(&[0; FRAME]), 0);
            let mut chunked = NoiseFloor::new();
            for _ in 0..8000 { chunked.push(&[ambient]); }
            assert_eq!(chunked.threshold(), expected);
            assert!(core::mem::size_of::<NoiseFloor>() <= 48);
        }
        let mut floor = NoiseFloor::new();
        floor.push(&[0; 7999]);
        assert!(!floor.ready());
        assert_eq!(floor.push(&[0; 100]), 1);
        assert!(floor.ready());
    }
}
