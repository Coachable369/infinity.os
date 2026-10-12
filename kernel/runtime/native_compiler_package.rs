//! Compiler and initial C SDK are embedded in both live and installed kernels.
//! The same immutable bytes survive installation; no installer-only path exists.
#[cfg(target_arch = "aarch64")]
pub static CLANG: &[u8] = include_bytes!("../../builds/toolchains/aarch64/clang.elf");
#[cfg(target_arch = "x86_64")]
pub static CLANG: &[u8] = include_bytes!("../../builds/toolchains/x86_64/clang.elf");
#[cfg(target_arch = "aarch64")]
pub static LLD: &[u8] = include_bytes!("../../builds/toolchains/aarch64/lld.elf");
#[cfg(target_arch = "x86_64")]
pub static LLD: &[u8] = include_bytes!("../../builds/toolchains/x86_64/lld.elf");
#[cfg(target_arch = "aarch64")]
pub static CRT: &[u8] = include_bytes!("../../builds/toolchains/aarch64/application-crt.o");
#[cfg(target_arch = "x86_64")]
pub static CRT: &[u8] = include_bytes!("../../builds/toolchains/x86_64/application-crt.o");
pub static LINK_SCRIPT: &[u8] = include_bytes!("../../sdk/c/application.ld");
