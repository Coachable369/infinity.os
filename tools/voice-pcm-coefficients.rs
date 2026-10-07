//! Reproduce the fixed Q30 microphone filters offline, never on the input thread.
const TAPS: usize = 96;
const PHASES: usize = 160;

// ------------------------=
// FUNC: coefficients
// DESC: Generates the canonical normalized Blackman-windowed sinc coefficients in little-endian Q30.
// ------------------=
fn coefficients(rate: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(TAPS * PHASES * 4);
    let pi = core::f64::consts::PI;
    let cutoff = 6_800.0 / rate as f64;
    for phase in 0..PHASES {
        let fraction = phase as f64 / PHASES as f64;
        let mut sum = 0.0;
        let mut values = [0.0f64; TAPS];
        for tap in 0..TAPS {
            let x = tap as f64 - (TAPS - 1) as f64 / 2.0 - fraction;
            let sinc = if x.abs() < 1e-12 { 2.0 * cutoff }
                else { libm::sin(2.0 * pi * cutoff * x) / (pi * x) };
            let angle = 2.0 * pi * tap as f64 / (TAPS - 1) as f64;
            let window = 0.42 - 0.5 * libm::cos(angle) + 0.08 * libm::cos(2.0 * angle);
            values[tap] = sinc * window;
            sum += values[tap];
        }
        for value in values {
            let coefficient = libm::round(value / sum * (1u64 << 30) as f64) as i32;
            bytes.extend_from_slice(&coefficient.to_le_bytes());
        }
    }
    bytes
}

// ------------------------=
// FUNC: main
// DESC: Generates repository filter assets or verifies their exact canonical binary contents.
// ------------------=
fn main() {
    let check = std::env::args().nth(1).as_deref() == Some("--check");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/runtime/ai");
    for rate in [44_100, 48_000] {
        let path = root.join(format!("voice-pcm-{rate}.bin"));
        let bytes = coefficients(rate);
        if check {
            assert_eq!(std::fs::read(&path).unwrap(), bytes, "{}", path.display());
        } else {
            std::fs::write(path, bytes).unwrap();
        }
    }
}
