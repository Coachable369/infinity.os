//! Replay two mono 16-kHz signed PCM fixtures through the production duplex echo filter.
//! Run via build-kit; arguments: reply.raw interruption.raw output-directory.
#[path = "../kernel/runtime/ai/voice_echo.rs"]
mod voice_echo;
// ------------------------=
// FUNC: read
// DESC: Reads immutable signed 16 kHz fixture samples.
// ------------------=
fn read(path: &str) -> Vec<i16> {
    std::fs::read(path)
        .unwrap()
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect()
}
// ------------------------=
// FUNC: main
// DESC: Measures real speech overlap through the production echo filter without claiming physical microphone equivalence.
// ------------------=
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let reply = read(&args[1]);
    let stop = read(&args[2]);
    let mut filter = voice_echo::Canceller::new();
    let length = reply.len().min(160_000);
    let mut cleaned = Vec::new();
    let mut mixed = Vec::new();
    let mut human = Vec::new();
    for at in (0..length).step_by(320) {
        let n = (length - at).min(320);
        let end = at + n;
        let mut reference = vec![0; 9600];
        let count = end.min(9600);
        reference[9600 - count..].copy_from_slice(&reply[end - count..end]);
        let mut mic: Vec<i16> = (at..end)
            .map(|i| {
                let echo = if i >= 800 {
                    i32::from(reply[i - 800]) / 2
                } else {
                    0
                } + if i >= 1440 {
                    i32::from(reply[i - 1440]) / 4
                } else {
                    0
                };
                let near = if i >= 16000 && i - 16000 < stop.len() {
                    stop[i - 16000]
                } else {
                    0
                };
                human.push(near);
                (echo + i32::from(near)).clamp(-32768, 32767) as i16
            })
            .collect();
        mixed.extend_from_slice(&mic);
        filter.process(&mut mic, &reference);
        cleaned.extend(mic);
    }
    for (name, data) in [("mixed", &mixed), ("cleaned", &cleaned), ("human", &human)] {
        std::fs::write(
            format!("{}/{name}.raw", args[3]),
            data.iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    }
    let residual: f64 = cleaned
        .iter()
        .zip(&human)
        .map(|(a, b)| (f64::from(*a) - f64::from(*b)).powi(2))
        .sum();
    let original: f64 = mixed
        .iter()
        .zip(&human)
        .map(|(a, b)| (f64::from(*a) - f64::from(*b)).powi(2))
        .sum();
    assert!(
        original > 0.0 && residual / original < 0.01,
        "synthetic overlap lost echo rejection or near-end speech"
    );
    println!(
        "samples={length} residual_error_ratio={} reduction_db={}",
        residual / original,
        10.0 * (original / residual).log10()
    );
}
