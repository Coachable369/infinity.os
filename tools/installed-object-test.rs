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
// FUNC: profile_models
// DESC: Validates the documented identity disk envelope and returns only active users' persisted model indices, never identity or credential data.
// ------------------=
fn profile_models(bytes: &[u8]) -> Option<Vec<(u8, bool)>> {
    if bytes.len() < 4096 || &bytes[..8] != b"INFIDN1\0" { return None; }
    let version = u16::from_le_bytes(bytes[8..10].try_into().ok()?);
    // Identity versions 1/2/3 contain the same eight 160-byte user records.
    // Reject future layouts rather than interpreting unknown private fields.
    if !matches!((version, bytes.len()), (1, 4096) | (2, 5632) | (3, 10768))
        || u16::from_le_bytes(bytes[10..12].try_into().ok()?) as usize != bytes.len() { return None; }
    let checksum = bytes[..bytes.len()-4].iter().fold(0x811c9dc5u32,
        |sum, byte| (sum ^ *byte as u32).wrapping_mul(0x01000193));
    if u32::from_le_bytes(bytes[bytes.len()-4..].try_into().ok()?) != checksum { return None; }
    let mut models = Vec::new();
    for index in 0..8 {
        let at = 160 + index * 160;
        if bytes[at] == 0 || bytes[at+1] != 1 { continue; }
        let flags = bytes[at+157];
        let selected = (flags >> 2) & 15;
        models.push((if flags & 0x80 == 0 || selected > 4 { 3 } else { selected },
                     flags & 0x80 == 0 || flags & 2 != 0));
    }
    Some(models)
}

// ------------------------=
// FUNC: inspect_profile_models
// DESC: Reads persisted AI selection from a stopped private installation through the read-only native ObjectStore and emits only numeric preferences.
// ------------------=
fn inspect_profile_models<D: BlockDevice>(store: &mut ObjectStore<D>) {
    let object = store.resolve(b"/system/identity/state").expect("identity state");
    let size = store.metadata(object).unwrap().logical_size as usize;
    assert!(size <= 16 * 1024);
    let mut bytes = vec![0; size];
    assert_eq!(store.read(object, None, &mut bytes).unwrap(), size);
    let models = profile_models(&bytes).expect("valid versioned identity preferences");
    let records: Vec<_> = models.iter().map(|(index, enabled)|
        format!("{{\"chat_model_index\":{index},\"chat_enabled\":{enabled}}}")).collect();
    println!("{{\"active_user_count\":{},\"profiles\":[{}],\"read_only\":true}}", models.len(), records.join(","));
}

// ------------------------=
// FUNC: profile_model_projection_validates_envelope
// DESC: Verifies model projection, legacy defaults, inactive-user exclusion and corruption rejection without exposing or changing private fields.
// ------------------=
#[test]
fn profile_model_projection_validates_envelope() {
    for (version, size) in [(1u16, 4096usize), (2, 5632), (3, 10768)] {
        let mut bytes = vec![0u8; size];
        bytes[..8].copy_from_slice(b"INFIDN1\0");
        bytes[8..10].copy_from_slice(&version.to_le_bytes());
        bytes[10..12].copy_from_slice(&(size as u16).to_le_bytes());
        for (index, flags) in [0x8e, 0x92, 0x80, 0x00, 0xfe].into_iter().enumerate() {
            let at = 160 + index * 160;
            bytes[at] = 1; bytes[at+1] = 1; bytes[at+157] = flags;
        }
        bytes[960] = 1; bytes[961] = 2;
        let checksum = bytes[..size-4].iter().fold(0x811c9dc5u32,
            |sum, byte| (sum ^ *byte as u32).wrapping_mul(0x01000193));
        bytes[size-4..].copy_from_slice(&checksum.to_le_bytes());
        assert_eq!(profile_models(&bytes), Some(vec![(3,true),(4,true),(0,false),(3,true),(3,true)]));
        bytes[317] ^= 1;
        assert_eq!(profile_models(&bytes), None);
        assert_eq!(profile_models(&bytes[..100]), None);
    }
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
        let source = path.starts_with(b"/home/default/documents/") && (path.ends_with(b".c") || path.ends_with(b".py"));
        let extension = if path.ends_with(b".py") {"py"} else if source {"c"} else {"elf"};
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
        let name = format!("{}.{}", object_hex, extension);
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
    if arguments.get(2).map(String::as_str) == Some("--profile-only") {
        assert_eq!(arguments.len(), 3);
        inspect_profile_models(&mut store);
        return;
    }
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
