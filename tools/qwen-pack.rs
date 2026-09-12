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
    if args[1] == "model" {
        let (length, _, digest) = split(&args[2], &args[3], 2);
        assert_eq!(length, 5_027_783_488);
        assert_eq!(
            digest,
            [
                0xd9, 0x8c, 0xdc, 0xbd, 0x03, 0xe1, 0x7c, 0xe4, 0x76, 0x81, 0x43, 0x5b, 0x51, 0x50,
                0xe3, 0x4c, 0x14, 0x17, 0xf5, 0x0b, 0x5c, 0x00, 0x19, 0xdd, 0x56, 0x0e, 0x48, 0x82,
                0xc5, 0x74, 0x57, 0x85
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
