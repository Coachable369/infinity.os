//! Bounded source delivery shared by native generation and its editor consumer.
pub const MAX_FLUSH_BYTES: usize = 256;
pub const FLUSH_INTERVAL_NS: u64 = 75_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stream {
    last_flush: Option<u64>,
    pub delivered: usize,
    pub flushes: usize,
}

impl Stream {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty delivery cursor without allocating or retaining another source copy.
    // ------------------=
    pub const fn new() -> Self {
        Self { last_flush: None, delivered: 0, flushes: 0 }
    }

    // ------------------------=
    // FUNC: ready
    // DESC: Selects at most one small source slice; the final flush remains bounded and cannot skip pending bytes.
    // ------------------=
    pub fn ready(&self, source: &[u8], now: Option<u64>, complete: bool) -> Option<core::ops::Range<usize>> {
        if self.delivered >= source.len() { return None; }
        if !complete && self.last_flush.zip(now).is_some_and(|(last, now)| now.saturating_sub(last) < FLUSH_INTERVAL_NS) {
            return None;
        }
        let end = (self.delivered + MAX_FLUSH_BYTES).min(source.len());
        // Prefer a completed line when that still makes useful bounded progress.
        let end = source[self.delivered..end].iter().rposition(|byte| *byte == b'\n')
            .filter(|offset| *offset >= 32).map_or(end, |offset| self.delivered + offset + 1);
        Some(self.delivered..end)
    }

    // ------------------------=
    // FUNC: committed
    // DESC: Advances only after the editor has accepted this contiguous slice.
    // ------------------=
    pub fn committed(&mut self, end: usize, now: Option<u64>) -> bool {
        if end <= self.delivered || end - self.delivered > MAX_FLUSH_BYTES { return false; }
        self.delivered = end;
        self.last_flush = now;
        self.flushes += 1;
        true
    }
    // ------------------------=
    // FUNC: replaced
    // DESC: Records one fully accepted atomic existing-source transaction, not a streamed partial append.
    // ------------------=
    pub fn replaced(&mut self, length: usize, now: Option<u64>) {
        self.delivered = length;
        self.last_flush = now;
        self.flushes += 1;
    }
}
