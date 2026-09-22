//! Observable inference timings; no logging or allocation in the worker path.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub phase_ns: [u64; 16],
    pub phase_calls: [u64; 16],
}
impl Profile {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes fixed-size per-operation counters without allocating.
    // ------------------=
    pub const fn new() -> Self {
        Self { phase_ns: [0; 16], phase_calls: [0; 16] }
    }
    // ------------------------=
    // FUNC: record
    // DESC: Accumulates BSP execution time for one bounded transformer phase.
    // ------------------=
    pub fn record(&mut self, phase: usize, elapsed: u64) {
        if phase < 16 {
            self.phase_ns[phase] = self.phase_ns[phase].saturating_add(elapsed);
            self.phase_calls[phase] = self.phase_calls[phase].saturating_add(1);
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Metrics {
    pub load_ns: u64,
    pub first_token_ns: u64,
    pub total_response_ns: u64,
    pub prefill_work_ns: u64,
    pub decode_work_ns: u64,
    pub max_slice_ns: u64,
    pub max_pump_ns: u64,
    pub reused_tokens: usize,
    pub prefill_tokens: usize,
    start_ns: Option<u64>,
    first_seen: bool,
}
impl Metrics {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes allocation-free timing state for a service instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            load_ns: 0,
            first_token_ns: 0,
            total_response_ns: 0,
            prefill_work_ns: 0,
            decode_work_ns: 0,
            max_slice_ns: 0,
            max_pump_ns: 0,
            reused_tokens: 0,
            prefill_tokens: 0,
            start_ns: None,
            first_seen: false,
        }
    }
    // ------------------------=
    // FUNC: begin
    // DESC: Resets per-request timing while retaining model-load measurements.
    // ------------------=
    pub fn begin(&mut self, now: Option<u64>, reused: usize, prefill: usize) {
        *self = Self {
            load_ns: self.load_ns,
            start_ns: now,
            reused_tokens: reused,
            prefill_tokens: prefill,
            ..Self::new()
        };
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Records end-to-end request time including final stop-token evaluation.
    // ------------------=
    pub fn finish(&mut self, now: Option<u64>) {
        self.total_response_ns = self.start_ns.zip(now).map_or(0, |(a, b)| b.saturating_sub(a));
    }
    // ------------------------=
    // FUNC: slice
    // DESC: Separates compute time from elapsed first-token latency and tracks worst slice length.
    // ------------------=
    pub fn slice(&mut self, start: Option<u64>, end: Option<u64>, emitted: bool) {
        if let Some((a, b)) = start.zip(end) {
            let elapsed = b.saturating_sub(a);
            self.max_slice_ns = self.max_slice_ns.max(elapsed);
            if self.first_seen {
                self.decode_work_ns += elapsed;
            } else {
                self.prefill_work_ns += elapsed;
            }
        }
        if emitted && !self.first_seen {
            self.first_token_ns = self
                .start_ns
                .zip(end)
                .map_or(0, |(a, b)| b.saturating_sub(a));
            self.first_seen = true;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: profile_is_bounded_and_resets
    // DESC: Verifies phase accumulation, bounds rejection and independent per-request reset.
    // ------------------=
    #[test]
    fn profile_is_bounded_and_resets() {
        let mut p = Profile::new();
        p.record(5, 10);
        p.record(5, 20);
        p.record(16, 999);
        assert_eq!((p.phase_ns[5], p.phase_calls[5]), (30, 2));
        assert_eq!(p.phase_ns.iter().sum::<u64>(), 30);
        p = Profile::new();
        assert_eq!(p.phase_ns, [0; 16]);
        assert_eq!(p.phase_calls, [0; 16]);
    }
    // ------------------------=
    // FUNC: separates_compute_and_wait
    // DESC: Checks timing phases, reused-token reporting, restart, and unavailable clocks.
    // ------------------=
    #[test]
    fn separates_compute_and_wait() {
        let mut m = Metrics::new();
        m.load_ns = 40;
        m.begin(Some(100), 12, 3);
        m.slice(Some(110), Some(120), false);
        m.slice(Some(150), Some(170), true);
        m.slice(Some(200), Some(205), true);
        assert_eq!(
            (
                m.first_token_ns,
                m.prefill_work_ns,
                m.decode_work_ns,
                m.max_slice_ns
            ),
            (70, 30, 5, 20)
        );
        assert_eq!((m.reused_tokens, m.prefill_tokens), (12, 3));
        m.finish(Some(220));
        assert_eq!(m.total_response_ns, 120);
        m.begin(None, 0, 8);
        m.slice(None, None, true);
        assert_eq!((m.load_ns, m.first_token_ns, m.max_slice_ns), (40, 0, 0));
        assert_eq!(m.total_response_ns, 0);
    }
}
