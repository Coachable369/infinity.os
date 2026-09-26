//! Guest CPU entropy instructions; not a substitute for installed entropy-service integration.
// ------------------------=
// FUNC: word
// DESC: Reads a CPU-provided random word only after capability detection, with bounded retry.
// ------------------=
fn word() -> Option<u64> { unsafe {
    #[cfg(target_arch = "aarch64")]
    {
        let features: u64; core::arch::asm!("mrs {}, id_aa64isar0_el1", out(reg) features);
        if features >> 60 == 0 { return None; }
        for _ in 0..10 {
            let value: u64; let valid: u64;
            core::arch::asm!("mrs {}, S3_3_C2_C4_0", "cset {}, ne", out(reg) value, out(reg) valid);
            if valid != 0 { return Some(value); }
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if core::arch::x86_64::__cpuid(1).ecx & (1 << 30) == 0 { return None; }
        for _ in 0..10 {
            let value: u64; let valid: u8;
            core::arch::asm!("rdrand {}", "setc {}", out(reg) value, out(reg_byte) valid);
            if valid != 0 { return Some(value); }
        }
    }
    None
} }
// ------------------------=
// FUNC: fill
// DESC: Fills the entire requested buffer from actual virtual CPU entropy or reports failure.
// ------------------=
pub fn fill(bytes: &mut [u8]) -> bool {
    for chunk in bytes.chunks_mut(8) {
        let Some(value) = word() else { return false; };
        chunk.copy_from_slice(&value.to_le_bytes()[..chunk.len()]);
    }
    true
}
