//! Native display-driver dispatch. Firmware scanout remains the safe fallback.

#[cfg(target_arch="aarch64")]
static GOP_SWITCH: core::sync::atomic::AtomicUsize=core::sync::atomic::AtomicUsize::new(0);
// ------------------------=
// FUNC: change_mode
// DESC: Requests a hardware mode and returns its actual scanout; never calls firmware services after exit.
// ------------------=
pub fn change_mode(width:u32,height:u32)->Option<(usize,usize,u32)> {
    #[cfg(target_arch="aarch64")] {
        let address=GOP_SWITCH.load(core::sync::atomic::Ordering::Acquire);
        if address!=0 {
            let change:unsafe extern "efiapi" fn(u32,u32,*mut u64)->u32=unsafe{core::mem::transmute(address)};
            let mut result=[0u64;3];
            if unsafe{change(width,height,result.as_mut_ptr())}==1 {return Some((result[0]as usize,result[1]as usize,result[2]as u32));}
        }
    }
    #[cfg(target_arch="x86_64")] {return pci_svga::change_mode(width,height);}
    #[allow(unreachable_code)] None
}
#[cfg(target_arch = "x86_64")]
mod pci_svga {
    use super::super::svga::{Bus, Scanout, Svga};
    use crate::boot_info::BootInfo;

    // QEMU standard VGA exposes flat 16-bit DISPI registers at BAR2 + 0x500.
    // Bind only the adapter whose linear framebuffer is the active boot scanout.
    static mut DISPI: Option<(usize,usize,u32)> = None;

    // ------------------------=
    // FUNC: dispi_mode
    // DESC: Switches a firmware-enabled standard VGA scanout and rolls back if its registers reject the geometry.
    // ------------------=
    unsafe fn dispi_mode(w:u32,h:u32)->Option<(usize,usize,u32)> {
        let (registers,frame,format)=DISPI?;
        let base=registers as *mut u16;
        let read=|i:usize|core::ptr::read_volatile(base.add(i));
        let write=|i:usize,v:u16|core::ptr::write_volatile(base.add(i),v);
        let pixels=u64::from(w)*u64::from(h);
        if w==0 || h==0 || w>u16::MAX as u32 || h>u16::MAX as u32 || pixels>3840*2160 ||
            pixels*4>u64::from(read(10))*65536 || frame as u64+pixels*4>1u64<<32 {return None;}
        let old=[read(1),read(2),read(3),read(4),read(6),read(8),read(9)];
        write(4,0);write(1,w as u16);write(2,h as u16);write(3,32);write(4,old[3]|0x41);
        write(6,w as u16);write(8,0);write(9,0);
        if read(1)!=w as u16 || read(2)!=h as u16 || read(3)!=32 || read(6)!=w as u16 ||
            read(8)!=0 || read(9)!=0 || u32::from(read(7))<h {
            write(4,0);write(1,old[0]);write(2,old[1]);write(3,old[2]);write(4,old[3]);
            write(6,old[4]);write(8,old[5]);write(9,old[6]);return None;
        }
        Some((frame,w as usize,format))
    }

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
                        if config(address,0)==0x11111234 && config(address,8)>>24==3 && config(address,4)&2!=0 {
                            let fb=config(address,16);let regs=config(address,24);
                            if fb&7==0 && u64::from(fb&!15)==info.framebuffer_address && regs&7==0 && regs&!15!=0 {
                                let base=((regs&!15)as usize+0x500)as *const u16;
                                let read=|i|core::ptr::read_volatile(base.add(i));
                                if (0xb0c0..=0xb0c5).contains(&read(0)) && read(4)&0x41==0x41 && read(3)==32 &&
                                    u32::from(read(1))==info.framebuffer_width && u32::from(read(2))==info.framebuffer_height {
                                    DISPI=Some((base as usize,info.framebuffer_address as usize,info.framebuffer_format));
                                }
                            }
                            continue;
                        }
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
    // FUNC: change_mode
    // DESC: Switches the bound SVGA II adapter after checking limits, VRAM and framebuffer layout.
    // ------------------=
    pub(super) fn change_mode(w:u32,h:u32)->Option<(usize,usize,u32)> {
        let Some((bus,driver))=(unsafe{(&mut *(&raw mut ACTIVE)).as_mut()}) else { return unsafe{dispi_mode(w,h)}; };
        if w==0 || h==0 || w>bus.read_register(4) || h>bus.read_register(5) {return None;}
        let prev=[bus.read_register(2),bus.read_register(3)];
        let size=bus.read_register(15) as u64;
        if u64::from(w)*u64::from(h)*4>size {return None;}
        bus.write_register(2,w);bus.write_register(3,h);bus.write_register(1,1);
        let pitch=bus.read_register(12);let offset=bus.read_register(14);
        let addr=u64::from(bus.read_register(13))+u64::from(offset);
        if bus.read_register(2)!=w || bus.read_register(3)!=h || pitch%4!=0 || pitch/4<w ||
            u64::from(pitch)*u64::from(h)+u64::from(offset)>size ||
            u64::from(pitch/4)*u64::from(h)>3840*2160 || addr==0 || addr%4!=0 || addr+u64::from(pitch)*u64::from(h)>(1u64<<32) {
            bus.write_register(2,prev[0]);bus.write_register(3,prev[1]);bus.write_register(1,1);return None;
        }
        driver.dimensions(w,h);
        Some((addr as usize,pitch as usize/4,(bus.read_register(9)!=255) as u32))
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
    #[cfg(target_arch="aarch64")]
    GOP_SWITCH.store(_info.display_exchange as usize,core::sync::atomic::Ordering::Release);
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
