#[path = "../kernel/arch/aarch64/output_platform.rs"]
mod output_platform;
#[path = "../kernel/arch/output_lock.rs"]
mod output_lock;

// ------------------------=
// FUNC: main
// DESC: Verifies AArch64 trace routing preserves disabled, QEMU, and VirtualBox platform behavior.
// ------------------=
fn main() {
    assert_eq!(output_platform::serial_base(0), None);
    assert_eq!(output_platform::serial_base(output_platform::SERIAL_ENABLED),
        Some(output_platform::QEMU_PL011_BASE));
    assert_eq!(output_platform::serial_base(
        output_platform::SERIAL_ENABLED | output_platform::VIRTUALBOX_UART),
        Some(output_platform::VIRTUALBOX_PL011_BASE));

    let records = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    std::thread::scope(|scope| {
        for writer in 0..8u8 {
            let records = records.clone();
            scope.spawn(move || {
                for _ in 0..256 {
                    output_lock::serialized(|| {
                        records.lock().unwrap().push(writer);
                        std::thread::yield_now();
                        records.lock().unwrap().push(writer);
                    });
                }
            });
        }
    });
    let records = records.lock().unwrap();
    assert_eq!(records.len(), 8 * 256 * 2);
    assert!(records.chunks_exact(2).all(|record| record[0] == record[1]));
}
