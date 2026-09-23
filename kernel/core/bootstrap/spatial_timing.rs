//! Fixed-cost animation-only timing; sorting occurs only on explicit inspection.
pub struct Timings {
    samples: [[u64; 5]; 128],
    count: usize,
    next: usize,
}
impl Timings {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes bounded allocation-free animation telemetry.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            samples: [[0; 5]; 128],
            count: 0,
            next: 0,
        }
    }
    // ------------------------=
    // FUNC: record
    // DESC: Records total, capture, paint, fade and scanout durations only from valid monotonic timestamps.
    // ------------------=
    pub fn record(&mut self, points: [Option<u64>; 5]) {
        let [Some(a), Some(b), Some(c), Some(d), Some(e)] = points else {
            return;
        };
        if a > b || b > c || c > d || d > e {
            return;
        }
        self.samples[self.next] = [e - a, b - a, c - b, d - c, e - d];
        self.next = (self.next + 1) % self.samples.len();
        self.count = (self.count + 1).min(self.samples.len());
    }
    // ------------------------=
    // FUNC: summary
    // DESC: Returns count, average, P95, worst and four phase averages in microseconds outside the paint path.
    // ------------------=
    pub fn summary(&self) -> [u64; 8] {
        let mut result = [0; 8];
        result[0] = self.count as u64;
        if self.count == 0 {
            return result;
        }
        let mut totals = [0; 128];
        for (i, sample) in self.samples[..self.count].iter().enumerate() {
            totals[i] = sample[0];
            result[1] += sample[0];
            for j in 1..5 {
                result[j + 3] += sample[j];
            }
        }
        totals[..self.count].sort_unstable();
        result[2] = totals[(self.count * 95).div_ceil(100) - 1] / 1000;
        result[3] = totals[self.count - 1] / 1000;
        for i in [1, 4, 5, 6, 7] {
            result[i] /= self.count as u64 * 1000;
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: bounded_timings_reject_invalid_and_preserve_phases
    // DESC: Verifies missing clocks, invalid ordering, ring overwrite, percentile and independent phase accounting.
    // ------------------=
    fn bounded_timings_reject_invalid_and_preserve_phases() {
        let mut t = Timings::new();
        t.record([None; 5]);
        t.record([Some(5), Some(4), Some(6), Some(7), Some(8)]);
        assert_eq!(t.summary(), [0; 8]);
        for _ in 0..256 {
            t.record([Some(0), Some(1000), Some(3000), Some(6000), Some(10000)]);
        }
        assert_eq!(t.summary(), [128, 10, 10, 10, 1, 2, 3, 4]);
    }
}
