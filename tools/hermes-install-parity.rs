use sha2::{Digest, Sha256};
use std::io::Read;
// ------------------------=
// FUNC: main
// DESC: Hashes extracted optional Hermes shards and compares packaged license bytes with source artifacts.
// ------------------=
fn main() {
    let image = std::env::args().nth(1).expect("ESP image");
    let mut hash = Sha256::new();
    let mut total = 0usize;
    for part in 0..4 {
        let mut child = std::process::Command::new("mtype")
            .args([
                "-i",
                &image,
                &format!("::/EFI/INFINITY/PAYLOAD/P4-{part:03}.BIN"),
            ])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut stream = child.stdout.take().unwrap();
        let mut buffer = [0u8; 65536];
        let mut length = 0;
        loop {
            let n = stream.read(&mut buffer).unwrap();
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
            length += n;
        }
        assert!(child.wait().unwrap().success());
        assert_eq!(length, if part < 3 { 536870912 } else { 408761152 });
        total += length;
    }
    assert_eq!(total, 2019373888);
    assert_eq!(
        hash.finalize().as_slice(),
        [
            0x91, 0x77, 0x6f, 0xe0, 0xf6, 0xcd, 0x74, 0x83, 0xd9, 0xd5, 0xe0, 0x61, 0x62, 0xfd,
            0xd1, 0xf8, 0xf0, 0x26, 0x2c, 0x15, 0xce, 0xd2, 0x69, 0x79, 0x1b, 0x4d, 0x96, 0xa6,
            0x55, 0xe8, 0xa5, 0xa2
        ]
    );
    for (name, source) in [
        (
            "HERMES-LICENSE.txt",
            "docs/licenses/Hermes-Llama-3.2-LICENSE.txt",
        ),
        ("HERMES-NOTICE.txt", "docs/licenses/Hermes-NOTICE.txt"),
    ] {
        let result = std::process::Command::new("mtype")
            .args(["-i", &image, &format!("::/EFI/INFINITY/PAYLOAD/{name}")])
            .output()
            .unwrap();
        assert!(result.status.success());
        assert_eq!(result.stdout, std::fs::read(source).unwrap());
    }
}
