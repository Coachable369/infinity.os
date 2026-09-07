use crate::boot_info::BootInfo;
use crate::drivers::device::DeviceIdentity;

#[derive(Clone, Copy)]
pub struct SystemSnapshot {
    pub architecture: &'static [u8],
    pub boot_mode: &'static [u8],
    pub memory_map_size: u64,
    pub memory_descriptor_size: u64,
    pub firmware_runtime_services: u64,
    pub framebuffer_width: usize,
    pub framebuffer_height: usize,
    pub devices: [DeviceIdentity; 3],
    pub live_profile: bool,
}

impl SystemSnapshot {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub fn new(info: &BootInfo, devices: [DeviceIdentity; 3]) -> Self {
        Self {
            architecture: architecture_name(),
            boot_mode: if cfg!(target_arch = "x86") {
                b"BIOS"
            } else {
                b"UEFI"
            },
            memory_map_size: info.memory_map_size,
            memory_descriptor_size: info.memory_descriptor_size,
            firmware_runtime_services: info.firmware_runtime_services,
            framebuffer_width: info.framebuffer_width as usize,
            framebuffer_height: info.framebuffer_height as usize,
            devices,
            live_profile: cfg!(target_arch = "x86") || info.boot_flags & 32 == 0,
        }
    }

    // ------------------------=
    // FUNC: ready_device_count
    // DESC: Implements the ready device count operation.
    // ------------------=
    pub fn ready_device_count(&self) -> usize {
        self.devices
            .iter()
            .filter(|device| device.state.is_ready())
            .count()
    }

    // ------------------------=
    // FUNC: memory_descriptor_count
    // DESC: Implements the memory descriptor count operation.
    // ------------------=
    pub fn memory_descriptor_count(&self) -> u64 {
        if self.memory_descriptor_size == 0 {
            0
        } else {
            self.memory_map_size / self.memory_descriptor_size
        }
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: architecture_name
// DESC: Implements the architecture name operation.
// ------------------=
const fn architecture_name() -> &'static [u8] {
    b"x86"
}
#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: architecture_name
// DESC: Implements the architecture name operation.
// ------------------=
const fn architecture_name() -> &'static [u8] {
    b"x86_64"
}
#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: architecture_name
// DESC: Implements the architecture name operation.
// ------------------=
const fn architecture_name() -> &'static [u8] {
    b"AArch64"
}
