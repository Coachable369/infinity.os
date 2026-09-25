pub const BOOT_MAGIC: u64 = 0x494e_4642_4f4f_5430;
pub const BOOT_VERSION: u32 = 9;
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
    /// Kind 4: firmware_network is an identity-mapped PCI ECAM function address.
    /// Older loaders retain kinds 1-3; they do not enable native ARM PCI ownership.
    pub network_reserved: u32,
    pub network_mac: [u8; 32],
    pub firmware_entropy: [u8; 32],
    pub firmware_entropy_valid: u32,
    /// Optional firmware-assigned HDA MMIO BAR below 4 GiB. Old loaders supply zero.
    pub boot_reserved: u32,
    pub payload_bridge: u64,
    pub model_address: u64,
    pub model_bytes: u64,
    pub model_work_address: u64,
    pub model_work_bytes: u64,
    pub worker_bridge: u64,
    pub native_pool_address: u64,
    pub native_pool_bytes: u64,
    pub kernel_address: u64,
    pub kernel_bytes: u64,
    pub kernel_text_address: u64,
    pub kernel_text_bytes: u64,
    pub kernel_rodata_address: u64,
    pub kernel_rodata_bytes: u64,
    pub kernel_data_address: u64,
    pub kernel_data_bytes: u64,
    pub kernel_page_table: u64,
    pub kernel_stack_address: u64,
    pub kernel_stack_bytes: u64,
}

const _: () = assert!(core::mem::size_of::<BootInfo>() == 368);
