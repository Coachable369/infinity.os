//! Identical immutable CPython and extension bytes are embedded in live and installed kernels.
#[cfg(target_arch = "aarch64")]
pub static PYTHON: &[u8] = include_bytes!("../../builds/toolchains/aarch64/python.elf");
#[cfg(target_arch = "x86_64")]
pub static PYTHON: &[u8] = include_bytes!("../../builds/toolchains/x86_64/python.elf");
#[cfg(target_arch = "aarch64")]
pub static EXAMPLE_EXTENSION: &[u8] = include_bytes!("../../builds/toolchains/aarch64/infinity_native_test.so");
#[cfg(target_arch = "x86_64")]
pub static EXAMPLE_EXTENSION: &[u8] = include_bytes!("../../builds/toolchains/x86_64/infinity_native_test.so");
#[cfg(target_arch = "aarch64")]
pub static LICENSE: &[u8] = include_bytes!("../../builds/toolchains/aarch64/python-license.txt");
#[cfg(target_arch = "x86_64")]
pub static LICENSE: &[u8] = include_bytes!("../../builds/toolchains/x86_64/python-license.txt");
