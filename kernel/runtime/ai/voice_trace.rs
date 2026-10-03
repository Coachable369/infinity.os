//! Bounded, prompt-free timing records shared by native conversation workers.

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
        if value == 0 { break; }
    }
    let count = digits.len() - first;
    record[at..at + count].copy_from_slice(&digits[first..]);
    at += count;
    record[at] = b'\n';
    unsafe { crate::output::write(&record[..at + 1]); }
}
