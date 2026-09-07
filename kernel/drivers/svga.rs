//! Native VMware SVGA II FIFO transport. No application framebuffer access.
//! Register/command ABI: VMware SVGA II, as implemented by QEMU hw/display/vmware_vga.c.

pub trait Bus {
    // ------------------------=
    // FUNC: read_register
    // DESC: Reads one hardware register through the platform transport.
    // ------------------=
    fn read_register(&mut self, index: u32) -> u32;
    // ------------------------=
    // FUNC: write_register
    // DESC: Writes one hardware register through the platform transport.
    // ------------------=
    fn write_register(&mut self, index: u32, value: u32);
    // ------------------------=
    // FUNC: read_fifo
    // DESC: Reads one hardware FIFO word.
    // ------------------=
    fn read_fifo(&mut self, word: u32) -> u32;
    // ------------------------=
    // FUNC: write_fifo
    // DESC: Writes one hardware FIFO word.
    // ------------------=
    fn write_fifo(&mut self, word: u32, value: u32);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scanout {
    pub address: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: u32,
}

pub struct Svga {
    width: u32,
    height: u32,
    min: u32,
    max: u32,
}

impl Svga {
    // ------------------------=
    // FUNC: attach
    // DESC: Validates firmware scanout against native registers and takes ownership of the command FIFO.
    // ------------------=
    pub fn attach(bus: &mut impl Bus, scanout: Scanout, fifo_bytes: u32) -> Option<Self> {
        if bus.read_register(0) != 0x90000002
            || bus.read_register(1) != 1
            || bus.read_register(2) != scanout.width
            || bus.read_register(3) != scanout.height
            || bus.read_register(7) != 32
            || bus.read_register(8) != 0
            || bus.read_register(12) as u64 != scanout.stride as u64 * 4
            || bus.read_register(13) as u64 + bus.read_register(14) as u64 != scanout.address
            || bus.read_register(22) != 0
            || scanout.width == 0
            || scanout.height == 0
            || scanout.stride < scanout.width
            || scanout.format > 1
        {
            return None;
        }
        let (red, blue) = if scanout.format == 0 {
            (0xff, 0xff0000)
        } else {
            (0xff0000, 0xff)
        };
        if bus.read_register(9) != red
            || bus.read_register(10) != 0xff00
            || bus.read_register(11) != blue
        {
            return None;
        }
        let min = if bus.read_register(17) & (1 << 15) != 0 {
            bus.read_register(30).checked_mul(4)?.max(16)
        } else {
            16
        };
        if fifo_bytes & 3 != 0 || min & 3 != 0 || min.checked_add(10240)? >= fifo_bytes {
            return None;
        }
        bus.write_register(20, 0);
        bus.write_fifo(0, min);
        bus.write_fifo(1, fifo_bytes);
        bus.write_fifo(2, min);
        bus.write_fifo(3, min);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        bus.write_register(20, 1);
        Some(Self {
            width: scanout.width,
            height: scanout.height,
            min,
            max: fifo_bytes,
        })
    }

    // ------------------------=
    // FUNC: update
    // DESC: Queues one clipped native UPDATE atomically, without waiting on the host or overwriting unread commands.
    // ------------------=
    pub fn update(&self, bus: &mut impl Bus, x: u32, y: u32, width: u32, height: u32) -> bool {
        let width = width.min(self.width.saturating_sub(x));
        let height = height.min(self.height.saturating_sub(y));
        if width == 0 || height == 0 {
            return true;
        }
        let mut next = bus.read_fifo(2);
        let stop = bus.read_fifo(3);
        if next < self.min
            || next >= self.max
            || stop < self.min
            || stop >= self.max
            || next & 3 != 0
            || stop & 3 != 0
        {
            return false;
        }
        let free = if next >= stop {
            self.max - next + stop - self.min
        } else {
            stop - next
        };
        if free <= 20 {
            return false;
        }
        for value in [1, x, y, width, height] {
            bus.write_fifo(next / 4, value);
            next += 4;
            if next == self.max {
                next = self.min;
            }
        }
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        bus.write_fifo(2, next);
        true
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Kicks host FIFO consumption without a BUSY polling loop.
    // ------------------=
    pub fn flush(&self, bus: &mut impl Bus) {
        bus.write_register(21, 1);
    }
}
