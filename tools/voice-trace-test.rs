//! Exercises actual numeric telemetry serialization through a bounded test sink.
#[allow(dead_code)]
#[path = "../kernel/runtime/ai/voice_trace.rs"]
mod trace;
mod output {
    pub static BYTES: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
    // ------------------------=
    // FUNC: write
    // DESC: Captures a single production serial record for structured protocol decoding.
    // ------------------=
    pub unsafe fn write(bytes: &[u8]) { BYTES.lock().unwrap().extend_from_slice(bytes); }
}
// ------------------------=
// FUNC: span_protocol_preserves_status_and_clock_validity
// DESC: Decodes numeric fields from real serialized records and verifies failed/reversed intervals remain distinguishable.
// ------------------=
#[test]
fn span_protocol_preserves_status_and_clock_validity() {
    trace::span(4, 25, 3, 100, 150, 24000, 0);
    trace::span(4, 25, 4, 200, 190, 0, 7);
    let bytes = output::BYTES.lock().unwrap();
    let lines: Vec<_> = std::str::from_utf8(&bytes).unwrap().lines().collect();
    let decode = |line: &str| line.split_once("] ").unwrap().1.split_whitespace()
        .map(|v| u64::from_str_radix(v, 16).unwrap()).collect::<Vec<_>>();
    assert_eq!(decode(lines[0]), vec![1, 4, 25, 3, 100, 150, 24000, 0, 1]);
    assert_eq!(decode(lines[1]), vec![1, 4, 25, 4, 200, 190, 0, 7, 0]);
    assert!(bytes.len() < 160);
}
