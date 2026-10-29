//! Bounded source delivery shared by native generation and its editor consumer.
pub const MAX_FLUSH_BYTES: usize = 256;
pub const FLUSH_INTERVAL_NS: u64 = 16_000_000;
pub const MAX_CONTINUATIONS: usize = 3;
pub const CONTINUATION_HEADER: &[u8] = b"CONTINUE_SOURCE";
const SOURCE_HEADER: &[u8] = b"REPLACE_COMPILE\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeError { Protocol, Capacity, Repeated }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Merge { pub length: usize, prefix_bytes: usize, header_bytes: usize }
impl Merge {
    // ------------------------=
    // FUNC: output_end
    // DESC: Converts a combined-frame boundary back to the current inference output without exposing prior prefix offsets to the service.
    // ------------------=
    pub fn output_end(self, end: usize) -> Option<usize> {
        end.checked_sub(self.prefix_bytes)?.checked_add(self.header_bytes)
    }
}

// ------------------------=
// FUNC: merge_continuation
// DESC: Combines one exact immutable source prefix with a continuation-only response; it never trims source whitespace or accepts a replayed whole prefix.
// ------------------=
pub fn merge_continuation(base: &[u8], output: &[u8], merged: &mut [u8]) -> Result<Option<Merge>, MergeError> {
    let trimmed = output.trim_ascii_start();
    let leading = output.len() - trimmed.len();
    let Some(newline) = trimmed.iter().position(|byte| *byte == b'\n') else {
        return if CONTINUATION_HEADER.starts_with(trimmed.trim_ascii_end())
            || trimmed.trim_ascii_end() == CONTINUATION_HEADER { Ok(None) } else { Err(MergeError::Protocol) };
    };
    if trimmed[..newline].trim_ascii_end() != CONTINUATION_HEADER { return Err(MergeError::Protocol); }
    let header_bytes = leading + newline + 1;
    let suffix = &output[header_bytes..];
    let replay = base.len().min(64);
    if replay >= 32 && suffix[..suffix.len().min(replay)] == base[..suffix.len().min(replay)] {
        // A potential restart is not source progress yet. Withhold these bytes
        // until they diverge, or reject the repeated prefix before any of it
        // enters the editor's append-only preview.
        if suffix.len() < replay { return Ok(None); }
        return Err(MergeError::Repeated);
    }
    let prefix_bytes = SOURCE_HEADER.len().checked_add(base.len()).ok_or(MergeError::Capacity)?;
    let length = prefix_bytes.checked_add(suffix.len()).filter(|length| *length <= merged.len()).ok_or(MergeError::Capacity)?;
    merged[..SOURCE_HEADER.len()].copy_from_slice(SOURCE_HEADER);
    merged[SOURCE_HEADER.len()..prefix_bytes].copy_from_slice(base);
    merged[prefix_bytes..length].copy_from_slice(suffix);
    Ok(Some(Merge { length, prefix_bytes, header_bytes }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stream {
    last_flush: Option<u64>,
    pub delivered: usize,
    pub flushes: usize,
    pub continuation_base: usize,
    pub continuations: usize,
}

impl Stream {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty delivery cursor without allocating or retaining another source copy.
    // ------------------=
    pub const fn new() -> Self {
        Self { last_flush: None, delivered: 0, flushes: 0, continuation_base: 0, continuations: 0 }
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
    // ------------------------=
    // FUNC: continue_from
    // DESC: Records a strictly advancing bounded source checkpoint without resetting editor delivery or permitting duplicate restarts.
    // ------------------=
    pub fn continue_from(&mut self, length: usize) -> bool {
        if self.continuations >= MAX_CONTINUATIONS || length <= self.continuation_base || length < self.delivered { return false; }
        self.continuation_base = length; self.continuations += 1; true
    }
}
