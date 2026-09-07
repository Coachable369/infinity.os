#[path = "../kernel/drivers/svga.rs"]
mod svga;
use svga::{Bus, Scanout, Svga};
struct Device {
    registers: [u32; 64],
    fifo: Vec<u32>,
}
impl Bus for Device {
    // ------------------------=
    // FUNC: read_register
    // DESC: Reads the test device's structured register state.
    // ------------------=
    fn read_register(&mut self, index: u32) -> u32 {
        self.registers[index as usize]
    }
    // ------------------------=
    // FUNC: write_register
    // DESC: Captures native driver register writes.
    // ------------------=
    fn write_register(&mut self, index: u32, value: u32) {
        self.registers[index as usize] = value;
    }
    // ------------------------=
    // FUNC: read_fifo
    // DESC: Reads the bounded emulated FIFO allocation.
    // ------------------=
    fn read_fifo(&mut self, word: u32) -> u32 {
        self.fifo[word as usize]
    }
    // ------------------------=
    // FUNC: write_fifo
    // DESC: Captures each command word using bounds-checked memory.
    // ------------------=
    fn write_fifo(&mut self, word: u32, value: u32) {
        self.fifo[word as usize] = value;
    }
}
// ------------------------=
// FUNC: main
// DESC: Exercises real driver initialization, clipping, FIFO wrap, backpressure, and rejection without side effects.
// ------------------=
fn main() {
    let mut device = Device {
        registers: [0; 64],
        fifo: vec![0; 16384],
    };
    for (register, value) in [
        (0, 0x90000002),
        (1, 1),
        (2, 1920),
        (3, 1080),
        (7, 32),
        (9, 0xff0000),
        (10, 0xff00),
        (11, 0xff),
        (12, 7680),
        (13, 0xe0000000),
    ] {
        device.registers[register] = value;
    }
    let scanout = Scanout {
        address: 0xe0000000,
        width: 1920,
        height: 1080,
        stride: 1920,
        format: 1,
    };
    let before = device.registers;
    assert!(Svga::attach(
        &mut device,
        Scanout {
            stride: 640,
            ..scanout
        },
        65536
    )
    .is_none());
    assert_eq!(device.registers, before);
    let driver = Svga::attach(&mut device, scanout, 65536).unwrap();
    assert_eq!(device.registers[20], 1);
    assert_eq!(&device.fifo[..4], &[16, 65536, 16, 16]);
    assert!(driver.update(&mut device, 1900, 1060, 100, 100));
    assert_eq!(&device.fifo[4..9], &[1, 1900, 1060, 20, 20]);
    assert_eq!(device.fifo[2], 36);
    device.fifo[2] = 65528;
    device.fifo[3] = 64;
    assert!(driver.update(&mut device, 10, 20, 30, 40));
    assert_eq!(&device.fifo[16382..], &[1, 10]);
    assert_eq!(&device.fifo[4..7], &[20, 30, 40]);
    assert_eq!(device.fifo[2], 28);
    device.fifo[3] = 48;
    let before = device.fifo.clone();
    assert!(!driver.update(&mut device, 0, 0, 10, 10));
    assert_eq!(device.fifo, before);
    driver.flush(&mut device);
    assert_eq!(device.registers[21], 1);
    device.fifo[2] = u32::MAX;
    assert!(!driver.update(&mut device, 0, 0, 10, 10));
}
