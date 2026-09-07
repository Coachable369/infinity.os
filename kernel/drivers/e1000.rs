//! QEMU 82540EM reference backend. DMA and MMIO are confined to this module.
//! Descriptor layout/registers follow Intel's 8254x Software Developer Manual.
use core::ptr::{addr_of_mut, read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};

const COUNT: usize = 32;
const BUFFER: usize = 2048;
pub const MAX_FRAME: usize = 1514;

#[derive(Clone, Copy)]
#[repr(C)]
struct Rx { address: u64, length: u16, checksum: u16, status: u8, errors: u8, special: u16 }
#[derive(Clone, Copy)]
#[repr(C)]
struct Tx { address: u64, length: u16, offset: u8, command: u8, status: u8, start: u8, special: u16 }
#[repr(C, align(128))]
struct Dma {
    rx: [Rx; COUNT], tx: [Tx; COUNT],
    receive: [[u8; BUFFER]; COUNT], send: [[u8; BUFFER]; COUNT],
}
static mut DMA: Dma = Dma {
    rx: [Rx { address: 0, length: 0, checksum: 0, status: 0, errors: 0, special: 0 }; COUNT],
    tx: [Tx { address: 0, length: 0, offset: 0, command: 0, status: 1, start: 0, special: 0 }; COUNT],
    receive: [[0; BUFFER]; COUNT], send: [[0; BUFFER]; COUNT],
};

pub struct E1000 { base: usize, rx: usize, tx: usize, tx_reap: usize, tx_pending: usize, rx_packets: u64, tx_packets: u64, clock_period: u32, pub mac: [u8; 6], pub drops: u64 }

// ------------------------=
// FUNC: out
// DESC: Writes a PCI configuration port in the x86 reference backend.
// ------------------=
unsafe fn out(port: u16, value: u32) { core::arch::asm!("out dx, eax", in("dx") port, in("eax") value, options(nostack)); }
// ------------------------=
// FUNC: input
// DESC: Reads a PCI configuration port in the x86 reference backend.
// ------------------=
unsafe fn input(port: u16) -> u32 { let value; core::arch::asm!("in eax, dx", in("dx") port, out("eax") value, options(nostack)); value }
// ------------------------=
// FUNC: pci
// DESC: Reads one aligned configuration dword on the root PCI bus.
// ------------------=
unsafe fn pci(device: u32, offset: u32) -> u32 { out(0xcf8, 0x80000000 | device | offset); input(0xcfc) }

impl E1000 {
    // ------------------------=
    // FUNC: read
    // DESC: Reads a register from the validated identity-mapped BAR.
    // ------------------=
    unsafe fn read(&self, offset: usize) -> u32 { read_volatile((self.base + offset) as *const u32) }
    // ------------------------=
    // FUNC: write
    // DESC: Writes a register and flushes posted writes with a status read.
    // ------------------=
    unsafe fn write(&self, offset: usize, value: u32) { write_volatile((self.base + offset) as *mut u32, value); let _ = self.read(8); }
    // ------------------------=
    // FUNC: initialize
    // DESC: Takes ownership of one QEMU root-bus 82540EM using fixed resident DMA buffers; caller guarantees unique ownership.
    // ------------------=
    pub unsafe fn initialize() -> Option<Self> {
        for function in 0..256u32 {
            let address = function << 8;
            if pci(address, 0) != 0x100e8086 { continue; }
            let bar = pci(address, 0x10);
            if bar & 1 != 0 || bar & 6 == 4 && pci(address, 0x14) != 0 { continue; }
            let base = (bar & !15) as usize;
            if base < 0x100000 || base.checked_add(0x20000)? > 0x1_0000_0000 { continue; }
            let command = pci(address, 4) & 0xffff;
            out(0xcf8, 0x80000000 | address | 4); out(0xcfc, command | 6);
            let mut nic = Self { base, rx: 0, tx: 0, tx_reap: 0, tx_pending: 0, rx_packets: 0, tx_packets: 0, clock_period: 0, mac: [0; 6], drops: 0 };
            // QEMU PC HPET, needed when TCG does not expose CPUID TSC calibration.
            let capabilities = read_volatile(0xfed00000 as *const u64);
            let period = (capabilities >> 32) as u32;
            if capabilities & 0xff != 0 && capabilities & (1 << 13) != 0 && (1..=100_000_000).contains(&period) {
                nic.clock_period = period;
                let configuration = read_volatile(0xfed00010 as *const u64);
                write_volatile(0xfed00010 as *mut u64, configuration | 1);
            }
            nic.write(0xd8, u32::MAX); // No interrupt-driven work until supported.
            nic.write(0x100, 0); nic.write(0x400, 0);
            let low = nic.read(0x5400).to_le_bytes();
            let high = nic.read(0x5404);
            if high & (1 << 31) == 0 { return None; }
            nic.mac = [low[0], low[1], low[2], low[3], high as u8, (high >> 8) as u8];
            if nic.mac == [0; 6] || nic.mac[0] & 1 != 0 { return None; }
            let dma = addr_of_mut!(DMA);
            for i in 0..COUNT {
                write_volatile(addr_of_mut!((*dma).rx[i]), Rx { address: addr_of_mut!((*dma).receive[i]) as u64, length: 0, checksum: 0, status: 0, errors: 0, special: 0 });
                write_volatile(addr_of_mut!((*dma).tx[i]), Tx { address: addr_of_mut!((*dma).send[i]) as u64, length: 0, offset: 0, command: 0, status: 1, start: 0, special: 0 });
            }
            let rx = addr_of_mut!((*dma).rx) as u64;
            let tx = addr_of_mut!((*dma).tx) as u64;
            fence(Ordering::SeqCst);
            nic.write(0x2800, rx as u32); nic.write(0x2804, (rx >> 32) as u32);
            nic.write(0x2808, (COUNT * 16) as u32); nic.write(0x2810, 0); nic.write(0x2818, (COUNT - 1) as u32);
            nic.write(0x3800, tx as u32); nic.write(0x3804, (tx >> 32) as u32);
            nic.write(0x3808, (COUNT * 16) as u32); nic.write(0x3810, 0); nic.write(0x3818, 0);
            nic.write(0, nic.read(0) | (1 << 6));
            nic.write(0x410, 10 | (8 << 10) | (6 << 20));
            nic.write(0x400, 2 | 8 | (15 << 4) | (64 << 12));
            nic.write(0x100, 2 | (1 << 15) | (1 << 26)); // 2KiB, broadcast, strip CRC.
            return Some(nic);
        }
        None
    }
    // ------------------------=
    // FUNC: link_up
    // DESC: Reads the actual link-up status bit rather than inferring connectivity.
    // ------------------=
    pub fn link_up(&self) -> bool { unsafe { self.read(8) & 2 != 0 } }
    // ------------------------=
    // FUNC: reference_clock_ns
    // DESC: Reads the validated QEMU PC 64-bit HPET counter using its reported femtosecond period.
    // ------------------=
    pub fn reference_clock_ns(&self) -> Option<u64> {
        if self.clock_period == 0 { return None; }
        let ticks = unsafe { read_volatile(0xfed000f0 as *const u64) };
        Some(((u128::from(ticks) * u128::from(self.clock_period)) / 1_000_000).min(u128::from(u64::MAX)) as u64)
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Consumes at most one completed frame and returns ownership to the NIC without waiting.
    // ------------------=
    pub fn receive(&mut self, out: &mut [u8; MAX_FRAME]) -> Option<usize> {
        unsafe {
            let dma = addr_of_mut!(DMA);
            let pointer = addr_of_mut!((*dma).rx[self.rx]);
            let descriptor = read_volatile(pointer);
            if descriptor.status & 1 == 0 { return None; }
            fence(Ordering::Acquire);
            let length = descriptor.length as usize;
            let valid = descriptor.status & 2 != 0 && descriptor.errors == 0 && (14..=MAX_FRAME).contains(&length);
            if valid {
                core::ptr::copy_nonoverlapping(addr_of_mut!((*dma).receive[self.rx]) as *const u8, out.as_mut_ptr(), length);
                self.rx_packets = self.rx_packets.saturating_add(1);
            }
            else { self.drops = self.drops.saturating_add(1); }
            write_volatile(addr_of_mut!((*pointer).status), 0);
            fence(Ordering::Release);
            self.write(0x2818, self.rx as u32);
            self.rx = (self.rx + 1) % COUNT;
            if valid { Some(length) } else { Some(0) }
        }
    }
    // ------------------------=
    // FUNC: transmit
    // DESC: Submits one frame only to a completed descriptor; false is bounded backpressure or invalid input.
    // ------------------=
    pub fn transmit(&mut self, frame: &[u8]) -> bool {
        if !(14..=MAX_FRAME).contains(&frame.len()) || !self.link_up() { return false; }
        self.statistics();
        if self.tx_pending == COUNT { return false; }
        unsafe {
            let dma = addr_of_mut!(DMA);
            let pointer = addr_of_mut!((*dma).tx[self.tx]);
            if read_volatile(addr_of_mut!((*pointer).status)) & 1 == 0 { return false; }
            fence(Ordering::Acquire);
            core::ptr::copy_nonoverlapping(frame.as_ptr(), addr_of_mut!((*dma).send[self.tx]) as *mut u8, frame.len());
            write_volatile(addr_of_mut!((*pointer).length), frame.len() as u16);
            write_volatile(addr_of_mut!((*pointer).command), 1 | 2 | 8); // EOP, IFCS, RS.
            write_volatile(addr_of_mut!((*pointer).status), 0);
            fence(Ordering::Release);
            self.tx = (self.tx + 1) % COUNT;
            self.tx_pending += 1;
            self.write(0x3818, self.tx as u32);
            true
        }
    }
    // ------------------------=
    // FUNC: statistics
    // DESC: Reaps at most four completed TX descriptors; counts hardware completion, never queued submissions.
    // ------------------=
    pub fn statistics(&mut self) -> (u64, u64, u64) {
        for _ in 0..4 {
            if self.tx_pending == 0 { break; }
            let status = unsafe { read_volatile(addr_of_mut!((*addr_of_mut!(DMA)).tx[self.tx_reap].status)) };
            if status & 1 == 0 { break; }
            fence(Ordering::Acquire);
            self.tx_reap = (self.tx_reap + 1) % COUNT;
            self.tx_pending -= 1;
            self.tx_packets = self.tx_packets.saturating_add(1);
        }
        (self.rx_packets, self.tx_packets, self.drops)
    }
}
