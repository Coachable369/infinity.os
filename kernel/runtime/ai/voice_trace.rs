//! Bounded, prompt-free timing records shared by native conversation workers.

// ------------------------=
// FUNC: capture
// DESC: Records calibration and VAD transitions as timestamp, state, threshold, mean-square energy and peak; never retains PCM.
// ------------------=
pub fn capture(now: u64, state: u64, threshold: u16, pcm: &[i16]) {
    let energy = pcm
        .iter()
        .map(|&v| (i64::from(v) * i64::from(v)) as u64)
        .sum::<u64>()
        / pcm.len().max(1) as u64;
    let peak = pcm.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
    const PREFIX: &[u8] = b"[VOICE CAPTURE] ";
    const HEX: &[u8] = b"0123456789abcdef";
    let mut record = [0u8; PREFIX.len() + 5 * 17];
    record[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut at = PREFIX.len();
    for value in [now, state, u64::from(threshold), energy, u64::from(peak)] {
        for shift in (0..16).rev() {
            record[at] = HEX[((value >> (shift * 4)) & 15) as usize];
            at += 1;
        }
        record[at] = b' ';
        at += 1;
    }
    record[at - 1] = b'\n';
    unsafe {
        crate::output::write(&record);
    }
}
// ------------------------=
// FUNC: write
// DESC: Adds the same monotonic nanosecond clock to each voice lifecycle record without allocation or runtime locks.
// ------------------=
pub fn write(prefix: &[u8], event: &[u8], now: u64) {
    let mut record = [0u8; 128];
    let prefix_len = prefix.len().min(24);
    record[..prefix_len].copy_from_slice(&prefix[..prefix_len]);
    let event_len = event.len().min(record.len() - prefix_len - 25);
    record[prefix_len..prefix_len + event_len].copy_from_slice(&event[..event_len]);
    let mut at = prefix_len + event_len;
    record[at..at + 4].copy_from_slice(b" ns=");
    at += 4;
    let mut digits = [0u8; 20];
    let mut first = digits.len();
    let mut value = now;
    loop {
        first -= 1;
        digits[first] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let count = digits.len() - first;
    record[at..at + count].copy_from_slice(&digits[first..]);
    at += count;
    record[at] = b'\n';
    unsafe {
        crate::output::write(&record[..at + 1]);
    }
}

// ------------------------=
// FUNC: timing
// DESC: Writes one versioned numeric timing record, without transcripts, PCM, allocation or runtime locks.
// ------------------=
pub fn timing(values: [u64; 10]) {
    numeric(b"[VOICE TIMING] ", &values);
}

// ------------------------=
// FUNC: numeric
// DESC: Emits a bounded versioned numeric record in one serial write, never user content.
// ------------------=
pub fn numeric(prefix: &[u8], values: &[u64]) {
    if prefix.len() > 24 || values.len() > 24 { return; }
    const HEX: &[u8] = b"0123456789abcdef";
    let mut record = [0u8; 24 + 24 * 17];
    record[..prefix.len()].copy_from_slice(prefix);
    let mut at = prefix.len();
    for &value in values {
        let digits = ((64 - value.leading_zeros() as usize + 3) / 4).max(1);
        for shift in (0..digits).rev() {
            record[at] = HEX[((value >> (shift * 4)) & 15) as usize];
            at += 1;
        }
        record[at] = b' ';
        at += 1;
    }
    if at == prefix.len() { return; }
    record[at - 1] = b'\n';
    unsafe { crate::output::write(&record[..at]); }
}

// ------------------------=
// FUNC: span
// DESC: Records a correlated phase interval and work units; invalid clocks remain explicitly invalid.
// ------------------=
pub fn span(phase: u64, job: u64, part: u64, start: u64, end: u64, units: u64, status: u64) {
    numeric(b"[VOICE SPAN] ", &[1, phase, job, part, start, end, units, status,
        u64::from(start != 0 && end >= start)]);
}
