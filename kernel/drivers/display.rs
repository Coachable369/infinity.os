//! Native display-driver dispatch. Firmware scanout remains the safe fallback.

#[cfg(target_arch = "x86_64")]
mod pci_svga {
    use super::super::svga::{Bus, Scanout, Svga};
    use crate::boot_info::BootInfo;

    struct Hardware {
        port: u16,
        fifo: *mut u32,
    }
    static mut ACTIVE: Option<(Hardware, Svga)> = None;

    // ------------------------=
    // FUNC: output
    // DESC: Writes a native x86 PCI or SVGA register port.
    // ------------------=
    unsafe fn output(port: u16, value: u32) {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") value, options(nostack));
    }
    // ------------------------=
    // FUNC: input
    // DESC: Reads a native x86 PCI or SVGA register port.
    // ------------------=
    unsafe fn input(port: u16) -> u32 {
        let value: u32;
        core::arch::asm!("in eax, dx", in("dx") port, out("eax") value, options(nostack));
        value
    }
    // ------------------------=
    // FUNC: config
    // DESC: Reads one aligned PCI configuration dword using configuration mechanism one.
    // ------------------=
    unsafe fn config(address: u32, offset: u32) -> u32 {
        output(0xcf8, 0x80000000 | address | offset);
        input(0xcfc)
    }
    impl Bus for Hardware {
        // ------------------------=
        // FUNC: read_register
        // DESC: Reads a VMware SVGA II indexed register.
        // ------------------=
        fn read_register(&mut self, index: u32) -> u32 {
            unsafe {
                output(self.port, index);
                input(self.port + 1)
            }
        }
        // ------------------------=
        // FUNC: write_register
        // DESC: Writes a VMware SVGA II indexed register.
        // ------------------=
        fn write_register(&mut self, index: u32, value: u32) {
            unsafe {
                output(self.port, index);
                output(self.port + 1, value);
            }
        }
        // ------------------------=
        // FUNC: read_fifo
        // DESC: Reads a validated native FIFO word using volatile MMIO.
        // ------------------=
        fn read_fifo(&mut self, word: u32) -> u32 {
            unsafe { core::ptr::read_volatile(self.fifo.add(word as usize)) }
        }
        // ------------------------=
        // FUNC: write_fifo
        // DESC: Writes a validated native FIFO word using volatile MMIO.
        // ------------------=
        fn write_fifo(&mut self, word: u32, value: u32) {
            unsafe {
                core::ptr::write_volatile(self.fifo.add(word as usize), value);
            }
        }
    }

    // ------------------------=
    // FUNC: initialize
    // DESC: Binds only matching, firmware-enabled PCI SVGA II hardware with safely mapped BARs.
    // ------------------=
    pub(super) fn initialize(info: &BootInfo) -> bool {
        unsafe {
            if (*(&raw const ACTIVE)).is_some() {
                return true;
            }
            for bus in 0..256u32 {
                for device in 0..32u32 {
                    let base = bus << 16 | device << 11;
                    if config(base, 0) == u32::MAX {
                        continue;
                    }
                    let functions = if config(base, 12) & 0x00800000 != 0 {
                        8
                    } else {
                        1
                    };
                    for function in 0..functions {
                        let address = base | function << 8;
                        if config(address, 0) != 0x040515ad || config(address, 8) >> 24 != 3 {
                            continue;
                        }
                        let bar0 = config(address, 16);
                        let bar2 = config(address, 24);
                        // Firmware must already have enabled I/O and memory decode.
                        if config(address, 4) & 3 != 3
                            || bar0 & 1 == 0
                            || bar0 & !3 == 0
                            || bar0 & !3 > 0xfffc
                            || bar2 & 7 != 0
                        {
                            continue;
                        }
                        let mut hardware = Hardware {
                            port: (bar0 & !3) as u16,
                            fifo: core::ptr::null_mut(),
                        };
                        let start = hardware.read_register(18);
                        let size = hardware.read_register(19);
                        if start == 0
                            || start != bar2 & !15
                            || start & 3 != 0
                            || size < 16384
                            || size > 16 * 1024 * 1024
                            || start as u64 + size as u64 > 0x100000000
                        {
                            continue;
                        }
                        hardware.fifo = start as usize as *mut u32;
                        let scanout = Scanout {
                            address: info.framebuffer_address,
                            width: info.framebuffer_width,
                            height: info.framebuffer_height,
                            stride: info.framebuffer_stride,
                            format: info.framebuffer_format,
                        };
                        if let Some(driver) = Svga::attach(&mut hardware, scanout, size) {
                            driver.update(&mut hardware, 0, 0, scanout.width, scanout.height);
                            driver.flush(&mut hardware);
                            *(&raw mut ACTIVE) = Some((hardware, driver));
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    // ------------------------=
    // FUNC: update
    // DESC: Sends compositor damage to the bound native SVGA driver, retaining damage on backpressure.
    // ------------------=
    pub(super) fn update(x: usize, y: usize, width: usize, height: usize) -> bool {
        unsafe {
            match &mut *(&raw mut ACTIVE) {
                Some((hardware, driver)) => {
                    driver.update(hardware, x as u32, y as u32, width as u32, height as u32)
                }
                None => true,
            }
        }
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Notifies the native display once after a compositor damage batch.
    // ------------------=
    pub(super) fn flush() {
        unsafe {
            if let Some((hardware, driver)) = &mut *(&raw mut ACTIVE) {
                driver.flush(hardware);
            }
        }
    }
}

// ------------------------=
// FUNC: initialize
// DESC: Selects a real native backend where implemented; other architectures retain firmware scanout.
// ------------------=
pub fn initialize(_info: &crate::boot_info::BootInfo) -> &'static str {
    #[cfg(target_arch = "x86_64")]
    if pci_svga::initialize(_info) {
        return "vmware-svga-ii-native";
    }
    "uefi-gop-framebuffer"
}
// ------------------------=
// FUNC: update
// DESC: Publishes one damaged region without granting applications hardware access.
// ------------------=
pub fn update(_x: usize, _y: usize, _width: usize, _height: usize) -> bool {
    #[cfg(target_arch = "x86_64")]
    return pci_svga::update(_x, _y, _width, _height);
    #[cfg(not(target_arch = "x86_64"))]
    true
}
// ------------------------=
// FUNC: flush
// DESC: Completes submission of the compositor's native display batch.
// ------------------=
pub fn flush() {
    #[cfg(target_arch = "x86_64")]
    pci_svga::flush();
}
