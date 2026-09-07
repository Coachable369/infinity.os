//! Runs the production FIFO driver against QEMU's actual VMware SVGA device model.
#[path = "../kernel/drivers/svga.rs"]
mod svga;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use svga::{Bus, Scanout, Svga};
struct Device {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Drop for Device {
    // ------------------------=
    // FUNC: drop
    // DESC: Stops only the test-owned, diskless QEMU process even when an assertion fails.
    // ------------------=
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Device {
    // ------------------------=
    // FUNC: command
    // DESC: Executes a qtest protocol transaction and decodes its numeric result.
    // ------------------=
    fn command(&mut self, command: String) -> u32 {
        writeln!(self.input, "{command}").unwrap();
        self.input.flush().unwrap();
        let mut response = String::new();
        self.output.read_line(&mut response).unwrap();
        let mut words = response.split_whitespace();
        assert_eq!(words.next(), Some("OK"), "{response}");
        words
            .next()
            .map(|value| u32::from_str_radix(value.trim_start_matches("0x"), 16).unwrap())
            .unwrap_or(0)
    }
    // ------------------------=
    // FUNC: pci_write
    // DESC: Configures only the diskless test device's PCI BARs and decode bits.
    // ------------------=
    fn pci_write(&mut self, offset: u32, value: u32) {
        self.command(format!("outl 0xcf8 {:#x}", 0x80001000u32 + offset));
        self.command(format!("outl 0xcfc {value:#x}"));
    }
}
impl Bus for Device {
    // ------------------------=
    // FUNC: read_register
    // DESC: Reads the actual emulated SVGA register through I/O ports.
    // ------------------=
    fn read_register(&mut self, index: u32) -> u32 {
        self.command(format!("outl 0xc000 {index:#x}"));
        self.command("inl 0xc001".into())
    }
    // ------------------------=
    // FUNC: write_register
    // DESC: Writes the actual emulated SVGA indexed register.
    // ------------------=
    fn write_register(&mut self, index: u32, value: u32) {
        self.command(format!("outl 0xc000 {index:#x}"));
        self.command(format!("outl 0xc001 {value:#x}"));
    }
    // ------------------------=
    // FUNC: read_fifo
    // DESC: Reads actual emulated PCI FIFO memory.
    // ------------------=
    fn read_fifo(&mut self, word: u32) -> u32 {
        self.command(format!("readl {:#x}", 0xf0000000u32 + word * 4))
    }
    // ------------------------=
    // FUNC: write_fifo
    // DESC: Writes actual emulated PCI FIFO memory.
    // ------------------=
    fn write_fifo(&mut self, word: u32, value: u32) {
        self.command(format!("writel {:#x} {value:#x}", 0xf0000000u32 + word * 4));
    }
}
// ------------------------=
// FUNC: main
// DESC: Verifies device identity and actual host consumption of the kernel driver's bounded update commands.
// ------------------=
fn main() {
    let mut child = Command::new("qemu-system-x86_64")
        .args([
            "-machine",
            "q35",
            "-nodefaults",
            "-device",
            "vmware-svga,addr=02.0",
            "-display",
            "none",
            "-S",
            "-qtest",
            "stdio",
            "-qtest-log",
            "/dev/null",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let input = child.stdin.take().unwrap();
    let output = BufReader::new(child.stdout.take().unwrap());
    let mut device = Device {
        child,
        input,
        output,
    };
    device.command("outl 0xcf8 0x80001000".into());
    assert_eq!(device.command("inl 0xcfc".into()), 0x040515ad);
    device.pci_write(16, 0xc001);
    device.pci_write(20, 0xe0000000);
    device.pci_write(24, 0xf0000000);
    device.pci_write(4, 3);
    for (register, value) in [(0, 0x90000002), (2, 800), (3, 600), (7, 32), (1, 1)] {
        device.write_register(register, value);
    }
    let scanout = Scanout {
        address: device.read_register(13) as u64 + device.read_register(14) as u64,
        width: device.read_register(2),
        height: device.read_register(3),
        stride: device.read_register(12) / 4,
        format: 1,
    };
    let fifo_size = device.read_register(19);
    let driver = Svga::attach(&mut device, scanout, fifo_size).expect("native device attach");
    let mut wrapped = false;
    let mut previous = device.read_fifo(2);
    for index in 0..fifo_size / 20 + 32 {
        assert!(driver.update(&mut device, index % 700, index % 500, 16, 16));
        driver.flush(&mut device);
        let next = device.read_fifo(2);
        wrapped |= next < previous;
        previous = next;
        assert_eq!(next, device.read_fifo(3));
        assert_eq!(device.read_register(22), 0);
    }
    assert!(wrapped);
}
