use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};
// ------------------------=
// FUNC: split
// DESC: Streams a large logical payload into FAT-safe shards and returns verified manifest metadata.
// ------------------=
fn split(source: &str, directory: &str, slot: u32) -> (u64, u32, [u8; 32]) {
    fs::create_dir_all(directory).unwrap();
    let mut source = File::open(source).unwrap();
    let mut buffer = vec![0; 1024 * 1024];
    let mut hash = Sha256::new();
    let mut crc = !0u32;
    let mut total = 0u64;
    let mut part = 0;
    loop {
        let path = Path::new(directory).join(format!("P{slot}-{part:03}.BIN"));
        let mut target = None;
        for _ in 0..512 {
            let count = source.read(&mut buffer).unwrap();
            if count == 0 {
                return (total, !crc, hash.finalize().into());
            }
            // read_exact-style filling preserves shard boundaries even after short OS reads.
            let mut count = count;
            while count < buffer.len() {
                let n = source.read(&mut buffer[count..]).unwrap();
                if n == 0 {
                    break;
                }
                count += n;
            }
            if target.is_none() {
                target = Some(File::create(&path).unwrap());
            }
            target
                .as_mut()
                .unwrap()
                .write_all(&buffer[..count])
                .unwrap();
            hash.update(&buffer[..count]);
            for byte in &buffer[..count] {
                crc ^= *byte as u32;
                for _ in 0..8 {
                    crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
                }
            }
            total += count as u64;
        }
        part += 1;
        assert!(part < 1000);
    }
}
// ------------------------=
// FUNC: main
// DESC: Builds immutable shard manifests without embedding large payloads in executable code.
// ------------------=
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args[1] == "hermes" {
        let (length, _, digest) = split(&args[2], &args[3], 4);
        assert_eq!(length, 2_019_373_888);
        assert_eq!(
            digest,
            [
                0x91, 0x77, 0x6f, 0xe0, 0xf6, 0xcd, 0x74, 0x83, 0xd9, 0xd5, 0xe0, 0x61, 0x62, 0xfd,
                0xd1, 0xf8, 0xf0, 0x26, 0x2c, 0x15, 0xce, 0xd2, 0x69, 0x79, 0x1b, 0x4d, 0x96, 0xa6,
                0x55, 0xe8, 0xa5, 0xa2
            ]
        );
    } else if args[1] == "ministral" {
        let (length, _, digest) = split(&args[2], &args[3], 3);
        assert_eq!(length, 2_147_023_008);
        assert_eq!(
            digest,
            [
                0x9e, 0xd1, 0x50, 0xd4, 0x36, 0x7e, 0x68, 0xdf, 0x0a, 0xc8, 0xe1, 0x54, 0x0f, 0x6d,
                0xdc, 0x65, 0xb4, 0x2d, 0x0e, 0xe2, 0x63, 0x78, 0x32, 0x9d, 0x1e, 0xcb, 0xca, 0x60,
                0xf9, 0x3f, 0xc5, 0xf8
            ]
        );
    } else {
        assert_eq!(args[1], "install");
        let mut manifest = File::create(&args[5]).unwrap();
        for (slot, name, path) in [(0, "ESP_IMAGE", &args[2]), (1, "KERNEL_IMAGE", &args[3])] {
            let (length, crc, digest) = split(path, &args[4], slot);
            writeln!(manifest,"const {name}: Image = Image {{ bytes: &[], length: {length}, crc: {crc}, slot: {slot}, sha256: {digest:?} }};").unwrap();
        }
    }
}
