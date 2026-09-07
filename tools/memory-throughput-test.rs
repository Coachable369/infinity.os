#[path = "../kernel/core/memory.rs"]
mod memory;
use std::{hint::black_box, time::Instant};

// ------------------------=
// FUNC: main
// DESC: Checks kernel memory routines against byte-accurate references and measures framebuffer-size copies.
// ------------------=
fn main() {
    for length in 0..256 {
        for source in 0..16 {
            for destination in 0..16 {
                let mut bytes = std::array::from_fn::<_, 512, _>(|index| index as u8);
                let mut expected = bytes;
                expected.copy_within(source..source + length, destination);
                unsafe {
                    memory::move_bytes(
                        bytes.as_mut_ptr().add(destination),
                        bytes.as_ptr().add(source),
                        length,
                    );
                }
                assert_eq!(bytes, expected);
                expected[destination..destination + length].fill(173);
                unsafe {
                    memory::fill(bytes.as_mut_ptr().add(destination), 173, length);
                }
                assert_eq!(bytes, expected);
                let input = [42u8; 512];
                unsafe {
                    memory::copy(
                        bytes.as_mut_ptr().add(destination),
                        input.as_ptr().add(source),
                        length,
                    );
                }
                expected[destination..destination + length].fill(42);
                assert_eq!(bytes, expected);
            }
        }
    }
    let input = vec![123u64; 2560 * 1600 / 2];
    let mut output = vec![0u64; input.len()];
    let mut durations = [0u128; 60];
    for duration in &mut durations {
        let start = Instant::now();
        unsafe {
            memory::copy(
                black_box(output.as_mut_ptr().cast()),
                black_box(input.as_ptr().cast()),
                input.len() * 8,
            );
        }
        *duration = start.elapsed().as_nanos();
        assert_eq!(output, input);
    }
    durations.sort_unstable();
    println!("{{\"fixture\":\"kernel_framebuffer_memcpy\",\"bytes\":{},\"samples\":60,\"average_ns\":{},\"p95_ns\":{}}}", input.len()*8, durations.iter().sum::<u128>()/60, durations[56]);
}
