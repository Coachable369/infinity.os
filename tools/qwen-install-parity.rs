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
    let mut loaders = 0;
    for (arch, name, machine) in [("aarch64", "BOOTAA64.EFI", 0xaa64u16),
                                  ("x86_64", "BOOTX64.EFI", 0x8664u16)] {
        let boot = Command::new("mtype")
            .args(["-i", &image, &format!("::/EFI/BOOT/{name}")])
            .output().unwrap();
        if !boot.status.success() { continue; }
        loaders += 1;
        assert_eq!(boot.stdout, std::fs::read(format!("build/{arch}/{name}")).unwrap());
        let pe = u32::from_le_bytes(boot.stdout[60..64].try_into().unwrap()) as usize;
        assert_eq!(&boot.stdout[pe..pe + 4], b"PE\0\0");
        assert_eq!(u16::from_le_bytes(boot.stdout[pe + 4..pe + 6].try_into().unwrap()), machine);
    }
    assert_eq!(loaders, 1);
    let mut buffer = [0u8; 65536];
    for part in 0..10 {
        let path = format!("::/EFI/INFINITY/PAYLOAD/P2-{part:03}.BIN");
        let removed = Command::new("mtype")
            .args(["-i", &image, &path])
            .output()
            .unwrap();
        assert!(
            !removed.status.success(),
            "removed model shard is still packaged"
        );
    }
    if std::env::args().any(|value| value == "--ministral") {
        let mut digest = Sha256::new();
        let mut total = 0u64;
        for part in 0..4 {
            let path = format!("::/EFI/INFINITY/PAYLOAD/P3-{part:03}.BIN");
            let mut child = Command::new("mtype")
                .args(["-i", &image, &path])
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let mut output = child.stdout.take().unwrap();
            let mut length = 0;
            loop {
                let count = output.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                length += count as u64;
                digest.update(&buffer[..count]);
            }
            assert!(child.wait().unwrap().success());
            assert_eq!(length, if part < 3 { 536_870_912 } else { 536_410_272 });
            total += length;
        }
        assert_eq!(total, 2_147_023_008);
        assert_eq!(
            digest.finalize().as_slice(),
            [
                0x9e, 0xd1, 0x50, 0xd4, 0x36, 0x7e, 0x68, 0xdf, 0x0a, 0xc8, 0xe1, 0x54, 0x0f, 0x6d,
                0xdc, 0x65, 0xb4, 0x2d, 0x0e, 0xe2, 0x63, 0x78, 0x32, 0x9d, 0x1e, 0xcb, 0xca, 0x60,
                0xf9, 0x3f, 0xc5, 0xf8
            ]
        );
        let license = Command::new("mtype")
            .args([
                "-i",
                &image,
                "::/EFI/INFINITY/PAYLOAD/MINISTRAL-LICENSE.txt",
            ])
            .output()
            .unwrap();
        assert!(license.status.success());
        assert_eq!(
            license.stdout,
            std::fs::read("docs/licenses/Ministral-Apache-2.0.txt").unwrap()
        );
    }
}
