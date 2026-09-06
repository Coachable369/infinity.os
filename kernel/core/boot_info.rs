pub const BOOT_MAGIC: u64 = 0x494e_4642_4f4f_5430;
pub const BOOT_VERSION: u32 = 5;
#[cfg(target_arch = "x86")]
pub const ARCH_X86: u32 = 1;
#[cfg(target_arch = "x86_64")]
pub const ARCH_X86_64: u32 = 2;
#[cfg(target_arch = "aarch64")]
pub const ARCH_AARCH64: u32 = 3;

#[repr(C)]
pub struct BootInfo {
    pub magic: u64,
    pub version: u32,
    pub architecture: u32,
    pub memory_map_address: u64,
    pub memory_map_size: u64,
    pub memory_descriptor_size: u64,
    pub firmware_revision: u64,
    pub boot_flags: u64,
    pub framebuffer_address: u64,
    pub framebuffer_size: u64,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_stride: u32,
    pub framebuffer_format: u32,
    pub firmware_input: u64,
    pub firmware_pointer: u64,
    pub firmware_runtime_services: u64,
    pub firmware_network: u64,
    pub network_device_count: u32,
    pub network_link_state: u32,
    pub network_mtu: u32,
    pub network_capabilities: u32,
    pub network_mac_length: u32,
    pub network_reserved: u32,
    pub network_mac: [u8; 32],
}

const _: () = assert!(core::mem::size_of::<BootInfo>() == 176);
