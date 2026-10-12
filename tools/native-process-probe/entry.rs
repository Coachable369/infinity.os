#![no_std]
#[cfg(target_arch = "x86_64")]
#[path = "probe.rs"] mod probe;
#[cfg(target_arch = "aarch64")]
#[path = "arm.rs"] mod probe;
