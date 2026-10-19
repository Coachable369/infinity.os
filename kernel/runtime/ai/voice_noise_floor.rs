//! Bounded startup noise measurement; retains energy only, never microphone PCM.
const FRAME: usize = 320;
const FRAMES: usize = 25;

pub struct NoiseFloor {
    samples: usize,
    frames: usize,
    energy: u64,
    energies: [u32; FRAMES],
    threshold: u16,
    continuation_threshold: u16,
}
impl NoiseFloor {
    // ------------------------=
    // FUNC: new
    // DESC: Starts a half-second 16-kHz calibration with the established minimum speech threshold.
    // ------------------=
    pub const fn new() -> Self {
        Self { samples: 0, frames: 0, energy: 0, energies: [0; FRAMES], threshold: 300, continuation_threshold: 300 }
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
    // FUNC: continuation_threshold
    // DESC: Keeps confirmed quieter speech above measured ambient without lowering the stricter new-utterance gate.
    // ------------------=
    pub const fn continuation_threshold(&self) -> u16 {
        self.continuation_threshold
    }
    // ------------------------=
    // FUNC: push
    // DESC: Uses the greater of whole-window and settled-tail medians so startup gain ramping cannot underestimate ambient noise.
    // ------------------=
    pub fn push(&mut self, pcm: &[i16]) -> usize {
        let mut consumed = 0;
        for &sample in pcm {
            if self.ready() { break; }
            consumed += 1;
            self.samples += 1;
            self.energy += (i64::from(sample) * i64::from(sample)) as u64;
            if self.samples == FRAME {
                self.energies[self.frames] = (self.energy / FRAME as u64) as u32;
                self.samples = 0;
                self.energy = 0;
                self.frames += 1;
                if self.ready() {
                    // Capture devices can ramp their gain throughout the first
                    // half second. Earlier quiet frames are not the settled
                    // room level. A tail median still rejects isolated clicks.
                    let mut tail = [0u32; 9];
                    tail.copy_from_slice(&self.energies[FRAMES - 9..]);
                    tail.sort_unstable();
                    self.energies.sort_unstable();
                    let baseline = u64::from(self.energies[FRAMES / 2].max(tail[4]));
                    let mut low = 0u64;
                    let mut high = 32769u64;
                    while high - low > 1 {
                        let middle = (low + high) / 2;
                        if middle * middle <= baseline { low = middle; } else { high = middle; }
                    }
                    self.threshold = (low * 2).clamp(300, 3000) as u16;
                    self.continuation_threshold = (low * 3 / 2).max(150).min(u64::from(self.threshold)) as u16;
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
            assert!(core::mem::size_of::<NoiseFloor>() <= 144);
        }
        let mut floor = NoiseFloor::new();
        floor.push(&[0; 7999]);
        assert!(!floor.ready());
        assert_eq!(floor.push(&[0; 100]), 1);
        assert!(floor.ready());
    }

    // ------------------------=
    // FUNC: startup_zeros_and_clicks_do_not_override_ambient
    // DESC: Checks a minority of cold zero frames and clipped impulses at every chunk boundary against sustained ambient sound.
    // ------------------=
    #[test]
    fn startup_zeros_and_clicks_do_not_override_ambient() {
        let mut pcm = std::vec![0; FRAME * 5];
        pcm.extend_from_slice(&[500; FRAME * 17]);
        pcm.extend_from_slice(&[32767; FRAME * 3]);
        for size in [1, 17, 320, 701, 4800, 8000] {
            let mut floor = NoiseFloor::new();
            for chunk in pcm.chunks(size) { floor.push(chunk); }
            assert!(floor.ready());
            assert_eq!(floor.threshold(), 1000);
        }
    }
    // ------------------------=
    // FUNC: settled_gain_is_not_diluted_by_cold_frames
    // DESC: Verifies a late capture gain ramp uses its settled level without adding readiness delay or retaining PCM.
    // ------------------=
    #[test]
    fn settled_gain_is_not_diluted_by_cold_frames() {
        for size in [1, 17, 320, 701, 8000] {
            let mut pcm = std::vec![0; FRAME * 15];
            pcm.extend_from_slice(&[400; FRAME * 10]);
            let mut floor = NoiseFloor::new();
            for chunk in pcm.chunks(size) { floor.push(chunk); }
            assert!(floor.ready());
            assert_eq!(floor.threshold(), 800);
        }
    }

    // ------------------------=
    // FUNC: continuation_gate_stays_above_calibrated_ambient
    // DESC: Verifies quieter continuation remains separated from actual noise and cannot exceed the onset gate.
    // ------------------=
    #[test]
    fn continuation_gate_stays_above_calibrated_ambient() {
        for (ambient, onset, continuation) in [(0, 300, 150), (100, 300, 150),
            (400, 800, 600), (700, 1400, 1050), (32767, 3000, 3000)] {
            let mut floor = NoiseFloor::new();
            assert_eq!(floor.continuation_threshold(), 300);
            floor.push(&[ambient; FRAME * FRAMES]);
            assert!(floor.ready());
            assert_eq!(floor.threshold(), onset);
            assert_eq!(floor.continuation_threshold(), continuation);
            assert!(floor.continuation_threshold() <= floor.threshold());
            if ambient < 3000 { assert!(floor.continuation_threshold() > ambient as u16); }
        }
    }
}
