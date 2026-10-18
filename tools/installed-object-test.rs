#![allow(dead_code)]
mod storage;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use storage::object::ObjectStore;
use storage::BlockDevice;

struct FileDisk {
    file: File,
    sectors: u64,
}

impl FileDisk {
    // ------------------------=
    // FUNC: open
    // DESC: Opens a raw test-disk artifact as a read-only block device.
    // ------------------=
    fn open(path: &str) -> Self {
        let file = File::open(path).expect("open installed test disk");
        let sectors = file.metadata().expect("disk metadata").len() / 512;
        Self { file, sectors }
    }
}

impl BlockDevice for FileDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Returns the raw artifact capacity in 512-byte sectors.
    // ------------------=
    fn block_count(&self) -> u64 {
        self.sectors
    }

    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one sector from the raw installed-disk artifact.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        self.file.seek(SeekFrom::Start(lba.saturating_mul(512))).is_ok()
            && self.file.read_exact(out).is_ok()
    }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Rejects mutation because installed-object verification is read-only.
    // ------------------=
    fn write_sector(&mut self, _: u64, _: &[u8; 512]) -> bool {
        false
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Completes the read-only BlockDevice contract without mutating storage.
    // ------------------=
    fn flush(&mut self) -> bool {
        true
    }
}

// ------------------------=
// FUNC: read_u64
// DESC: Decodes one little-endian 64-bit field from an on-disk record.
// ------------------=
fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64 field"))
}

// ------------------------=
// FUNC: export_generated
// DESC: Extracts bounded generated C and native executables from the stopped private test disk through its actual read-only ObjectStore.
// ------------------=
fn export_generated<D: BlockDevice>(store: &mut ObjectStore<D>, directory: &str) {
    use std::fmt::Write;
    let root = std::env::current_dir().unwrap().canonicalize().unwrap();
    assert!(root.join("build-kit.toml").is_file());
    let destination = std::path::Path::new(directory);
    let parent = destination.parent().unwrap().canonicalize().unwrap();
    assert!(parent.starts_with(root.join("build")) || parent.starts_with(root.join("builds")));
    std::fs::create_dir(destination).expect("fresh artifact export directory");
    let mut records = Vec::new();
    for index in 0..4096 {
        let Some((path, object)) = store.namespace_entry(index) else { break; };
        let source = path.starts_with(b"/home/default/documents/app-c-") && path.ends_with(b".c");
        let executable = path.starts_with(b"/home/default/program-") && path.ends_with(b".elf");
        if !source && !executable { continue; }
        let path = path.to_vec();
        let metadata = store.metadata(object).expect("generated object metadata");
        assert!(metadata.logical_size > 0 && metadata.logical_size <= 16 * 1024 * 1024);
        assert!(records.len() < 128);
        let mut content = vec![0; metadata.logical_size as usize];
        assert_eq!(store.read(object, None, &mut content).expect("generated object bytes"), content.len());
        if executable {
            assert!(content.len() >= 64 && &content[..6] == b"\x7fELF\x02\x01");
        }
        let mut object_hex = String::new();
        let mut path_hex = String::new();
        for value in object.0 { write!(object_hex, "{value:02x}").unwrap(); }
        for value in path { write!(path_hex, "{value:02x}").unwrap(); }
        let name = format!("{}.{}", object_hex, if source { "c" } else { "elf" });
        std::fs::write(destination.join(&name), &content).expect("write extracted artifact");
        records.push(format!("{{\"object\":\"{object_hex}\",\"path_hex\":\"{path_hex}\",\"file\":\"{name}\",\"source\":{source},\"version\":{},\"bytes\":{},\"crc32\":{}}}",
                             metadata.current_version, content.len(), storage::object::crc32(&content)));
    }
    assert!(!records.is_empty(), "No generated artifact was persisted");
    let report = format!("{{\"version\":1,\"read_only_native_storage\":true,\"artifacts\":[{}]}}\n", records.join(","));
    std::fs::write(destination.join("objects.json"), report).expect("write artifact receipt");
}

// ------------------------=
// FUNC: main
// DESC: Verifies installed native runtime, networking, AI, organization, and date/time System objects.
// ------------------=
fn main() {
    let path = std::env::args().nth(1).expect("raw disk path");
    let mut disk = FileDisk::open(&path);
    let mut entries = [0u8; 512];
    assert!(disk.read_sector(2, &mut entries), "read GPT entries");
    let container_lba = read_u64(&entries, 128 + 32);
    assert!(container_lba > 0, "Infinity Container partition missing");
    let mut store = ObjectStore::mount(disk, container_lba).expect("mount native Object Store");
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.get(2).map(String::as_str) == Some("--export-generated") {
        assert_eq!(arguments.len(), 4);
        export_generated(&mut store, &arguments[3]);
        return;
    }
    let date_time = store.date_time_configuration().expect("installed date/time configuration");
    assert!(date_time.is_valid(), "installed date/time configuration invalid");
    assert!(store.runtime_bootstrap_valid(), "native runtime/AI objects invalid");
    let organization = store.resolve(b"/system/organization/schema").expect("organization schema object");
    let mut bytes = [0u8; 256];
    assert_eq!(store.read(organization, None, &mut bytes).expect("read organization schema"), 256);
    assert!(storage::organization::organization_schema_valid(&bytes));
    let mapping=store.resolve(b"/system/ai/instruction-mappings.json").expect("installed AI instruction mapping");
    let mut mapping_bytes=[0u8;16*1024];
    let mapping_length=store.read(mapping,None,&mut mapping_bytes).expect("read installed AI instruction mapping");
    assert_eq!(&mapping_bytes[..mapping_length],include_bytes!("../system/ai/instruction-mappings.json"));
    if std::env::args().any(|argument| argument == "--bounded-node-checkpoint") {
        let node = store.resolve(b"/system/security/nodes/state").unwrap();
        assert_eq!(store.history_count(node), 1);
        let mut state = [0u8; 16_384];
        assert_eq!(store.read(node, None, &mut state).unwrap(), 12_288);
    }
    if std::env::args().any(|argument| argument == "--browser-download") {
        let id=store.resolve(b"/home/default/downloads/native-browser-test.txt").unwrap();
        let length=store.read(id,None,&mut bytes).unwrap();
        assert_eq!(&bytes[..length],b"{\n  \"Content-Disposition\": [\n    \"attachment; filename=native-browser-test.txt\"\n  ],\n  \"Content-Type\": [\n    \"text/plain\"\n  ]\n}\n");
        let metadata=store.metadata(id).unwrap();
        assert_ne!(metadata.owner.0,[0;16]);
        let relation=store.relationship_nth(id,0).unwrap();
        assert_eq!(relation.kind,storage::object::RelationshipType::References);
        let length=store.read(relation.target.id,None,&mut bytes).unwrap();
        assert_eq!(&bytes[..length],b"text/plain");
        assert_eq!(store.metadata(relation.target.id).unwrap().owner,metadata.owner);
    }
    println!("PASS: installed native runtime, editable AI mapping, organization schema, and date/time settings mount and validate");
}
