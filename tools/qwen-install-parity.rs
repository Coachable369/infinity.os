use sha2::{Digest, Sha256};
use std::{
    io::Read,
    process::{Command, Stdio},
};
// ------------------------=
// FUNC: main
// DESC: Verifies extracted EFI and model bytes, not logs or source strings, from a built or installed ESP.
// ------------------=
fn main() {
    let image = std::env::args()
        .nth(1)
        .expect("ESP image, or disk.raw@@1048576");
    let boot = Command::new("mtype")
        .args(["-i", &image, "::/EFI/BOOT/BOOTAA64.EFI"])
        .output()
        .unwrap();
    assert!(boot.status.success());
    assert_eq!(
        boot.stdout,
        std::fs::read("build/aarch64/BOOTAA64.EFI").unwrap()
    );
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 65536];
    for part in 0..10 {
        let path = format!("::/EFI/INFINITY/PAYLOAD/P2-{part:03}.BIN");
        let mut child = Command::new("mtype")
            .args(["-i", &image, &path])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = child.stdout.take().unwrap();
        let mut length = 0;
        loop {
            let n = output.read(&mut buffer).unwrap();
            if n == 0 {
                break;
            }
            length += n as u64;
            digest.update(&buffer[..n]);
        }
        assert!(child.wait().unwrap().success());
        assert_eq!(length, if part < 9 { 536_870_912 } else { 195_945_280 });
        total += length;
    }
    assert_eq!(total, 5_027_783_488);
    assert_eq!(
        digest.finalize().as_slice(),
        [
            0xd9, 0x8c, 0xdc, 0xbd, 0x03, 0xe1, 0x7c, 0xe4, 0x76, 0x81, 0x43, 0x5b, 0x51, 0x50,
            0xe3, 0x4c, 0x14, 0x17, 0xf5, 0x0b, 0x5c, 0x00, 0x19, 0xdd, 0x56, 0x0e, 0x48, 0x82,
            0xc5, 0x74, 0x57, 0x85
        ]
    );
}
