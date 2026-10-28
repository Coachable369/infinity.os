//! Seeds test inputs and inspects packages on a stopped disposable installation through ObjectStore.
#![allow(dead_code)]
#[path = "../storage.rs"] mod storage;
#[path = "../../kernel/storage/spatial_path.rs"] mod spatial_path;
#[path = "../../sdk/infinity-browser-core/favorites.rs"] mod favorites;
#[path = "../../kernel/runtime/font_preferences.rs"] mod font_preferences;
#[path = "../../kernel/runtime/native_python_package_limits.rs"] mod package_limits;
use std::{fs::{File, OpenOptions}, io::{Read, Seek, SeekFrom, Write}, path::Path};
use storage::{BlockDevice, object::{ObjectStore, ObjectId, ObjectType, ObjectQueryRequest, Space}};

struct Disk { file: File, sectors: u64, writable: bool }
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Returns the exact disposable raw-disk sector count.
    // ------------------=
    fn block_count(&self) -> u64 { self.sectors }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one bounded disk sector through the native storage adapter.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        lba < self.sectors && self.file.seek(SeekFrom::Start(lba * 512)).is_ok() && self.file.read_exact(out).is_ok()
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Allows writes only during explicit fixture seeding, never while inspecting package evidence.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        self.writable && lba < self.sectors && self.file.seek(SeekFrom::Start(lba * 512)).is_ok() && self.file.write_all(bytes).is_ok()
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Flushes fixture transactions before the VM can reboot its disk.
    // ------------------=
    fn flush(&mut self) -> bool { self.file.sync_all().is_ok() }
}

// ------------------------=
// FUNC: fixture_path
// DESC: Restricts every test input and output to this repository's controlled build directories.
// ------------------=
fn fixture_path(path: &str) -> std::path::PathBuf {
    let root = std::env::current_dir().unwrap().canonicalize().unwrap();
    assert!(root.join("build-kit.toml").is_file());
    let path = Path::new(path).canonicalize().unwrap();
    assert!(path.starts_with(root.join("build")) || path.starts_with(root.join("builds")));
    path
}

// ------------------------=
// FUNC: first_owner
// DESC: Reads only the first active synthetic user's public identifier from the checked native identity envelope.
// ------------------=
fn first_owner(store: &mut ObjectStore<Disk>) -> ObjectId {
    let identity = store.resolve(b"/system/identity/state").unwrap();
    let length = store.metadata(identity).unwrap().logical_size as usize;
    assert!([4096, 5632, 10768].contains(&length));
    let mut bytes = vec![0; length];
    assert_eq!(store.read(identity, None, &mut bytes).unwrap(), length);
    assert_eq!(&bytes[..8], b"INFIDN1\0");
    assert!((1..=3).contains(&u16::from_le_bytes(bytes[8..10].try_into().unwrap())));
    assert_eq!(u16::from_le_bytes(bytes[10..12].try_into().unwrap()) as usize, length);
    let checksum = bytes[..length - 4].iter().fold(0x811c9dc5u32, |sum, byte| (sum ^ *byte as u32).wrapping_mul(0x01000193));
    assert_eq!(u32::from_le_bytes(bytes[length - 4..].try_into().unwrap()), checksum);
    let offset = (0..8).map(|index| 160 + index * 160).find(|at| bytes[*at] != 0 && bytes[*at + 1] == 1).unwrap();
    ObjectId(bytes[offset + 2..offset + 18].try_into().unwrap())
}

// ------------------------=
// FUNC: seed_inputs
// DESC: Reuses bounded private fixture slots across phases, reserving archive capacity and proving package bytes remain unchanged.
// ------------------=
fn seed_inputs(store: &mut ObjectStore<Disk>, directory: &Path, owner: ObjectId, phase: &str, architecture: &str) {
    let native = match architecture {
        "aarch64" => "infinity_test_native-1.0.0-cp312-cp312-infinityos_aarch64.whl",
        "x86_64" => "infinity_test_native-1.0.0-cp312-cp312-infinityos_x86_64.whl",
        _ => panic!("Unsupported fixture architecture"),
    };
    let names = match phase {
        "install" => ["infinity_test_base-1.0.0-py3-none-any.whl", "infinity_test_app-1.0.0-py3-none-any.whl", "import.py", "metadata.py"],
        "reject" => ["infinity_test_platform-1.0.0-cp313-cp313-win_amd64.whl", "infinity_test_escape-1.0.0-py3-none-any.whl", "import.py", native],
        "upgrade" => ["infinity_test_base-1.0.0-py3-none-any.whl", "infinity_test_app-1.1.0-py3-none-any.whl", "upgraded.py", "absent.py"],
        "online" => ["online.py", "online-absent.py", "online-denied.py", "online-metadata.py"],
        "tls" => ["stdlib-tls.py", "stdlib-tls-denied.py", "stdlib-tls-hostname.py", "stdlib-sqlite.py"],
        "tk" => ["tk-acceptance.py", "tk-import.py", "tk-window-cycle.py", "tk-widget-contract.py"],
        "data" => ["data-acceptance.py", "data-isolation.py", "tk-import.py", "tk-widget-contract.py"],
        "bundled" => ["bundled-tests.py", "infinity_test_base-1.0.0-py3-none-any.whl",
            "infinity_test_app-1.0.0-py3-none-any.whl", "import.py"],
        "bundled-import" => ["bundled-tests.py", "import.py",
            "infinity_test_app-1.0.0-py3-none-any.whl", "import.py"],
        "libraries" => ["libraries-http.py", "libraries-render.py", "libraries-metadata.py", "libraries-absent.py"],
        "source" => ["infinity_source_probe-1.0.0.zip", "infinity_source_failure-1.0.0.zip", "source-import.py", "source-absent.py"],
        "native" => [if architecture == "aarch64" { "infinity_native_package-1.0.0-cp313-cp313-infinityos_aarch64.whl" }
            else { "infinity_native_package-1.0.0-cp313-cp313-infinityos_x86_64.whl" }, "native-import.py",
            "infinity_invalid_native-1.0.0-py3-none-any.whl", "metadata.py"],
        _ => panic!("Unsupported fixture phase"),
    };
    // Reuse the no-longer-needed dependency wheel slot for the import script
    // after installation; keep one free namespace entry for the owned archive.
    let names = if matches!(phase, "bundled" | "bundled-import") { &names[..3] } else { &names[..] };
    let mut previous = vec![0; package_limits::ARCHIVE_BYTES];
    let (revision, length) = store.read_python_packages(owner, &mut previous).unwrap();
    previous.truncate(length);
    for (index, name) in names.iter().enumerate() {
        let bytes = std::fs::read(directory.join(name)).unwrap();
        assert!(bytes.len() <= 64 * 1024);
        let label = format!("pip-fixture-{index}");
        let existing = (0..256).filter_map(|i| store.query_nth(ObjectQueryRequest {
            kind: Some(ObjectType::Text), space: Some(Space::Personal), include_tombstones: false,
        }, i)).find(|item| &item.display_name[..item.display_name_length as usize] == label.as_bytes());
        let path = if matches!(phase, "tk" | "data") { format!("/home/default/documents/{phase}/{name}") }
            else { format!("/home/default/documents/{name}") };
        let object = if let Some(existing) = existing {
            let id = existing.object.id;
            assert_eq!(store.metadata(id).unwrap().owner, owner);
            assert_eq!(store.namespace_refs(id), 1);
            let old_path = store.namespace_ref_nth(id, 0).unwrap().to_vec();
            assert!(old_path.starts_with(b"/home/default/documents/"));
            if old_path != path.as_bytes() { store.move_entry(&old_path, path.as_bytes()).unwrap(); }
            store.replace_state(id, &bytes).unwrap();
            id
        } else {
            assert!(matches!(phase, "install" | "online" | "tls" | "tk" | "bundled"), "Later phases may only reuse fixture objects");
            let used = (0..storage::object::NAMESPACE_CAPACITY).filter(|i| store.namespace_entry(*i).is_some()).count();
            assert!(used + (names.len() - index) + usize::from(revision.is_none()) <= storage::object::NAMESPACE_CAPACITY,
                "Fixture capacity: used={used}, requested={}, archive={}, capacity={}",
                names.len() - index, usize::from(revision.is_none()), storage::object::NAMESPACE_CAPACITY);
            store.create_owned_attached(label.as_bytes(), ObjectType::Text, Space::Personal, &bytes, path.as_bytes(), owner).unwrap()
        };
        let mut actual = vec![0; bytes.len()];
        assert_eq!(store.read(object, None, &mut actual).unwrap(), bytes.len());
        assert_eq!(actual, bytes);
    }
    let mut after = vec![0; package_limits::ARCHIVE_BYTES];
    let (after_revision, after_length) = store.read_python_packages(owner, &mut after).unwrap();
    assert_eq!(revision, after_revision);
    assert_eq!(previous, after[..after_length]);
    println!("{{\"seeded_inputs\":{},\"reused_phase\":\"{phase}\",\"package_state_changed\":false}}", names.len());
}

// ------------------------=
// FUNC: main
// DESC: Seeds only immutable wheel/script fixtures or exports a user's committed package archive from an explicitly stopped private test disk.
// ------------------=
fn main() {
    assert_eq!(std::env::var("INFINITY_BUILD_KIT_ACTIVE").as_deref(), Ok("1"));
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() == 4 || args.len() == 6);
    assert!(matches!(args[1].as_str(), "seed" | "inspect"));
    assert_eq!(args.len(), if args[1] == "seed" { 6 } else { 4 });
    let path = fixture_path(&args[2]);
    assert_eq!(path.extension().unwrap(), "raw");
    let directory = fixture_path(&args[3]);
    assert!(directory.is_dir());
    let writable = args[1] == "seed";
    let file = OpenOptions::new().read(true).write(writable).open(path).unwrap();
    let sectors = file.metadata().unwrap().len() / 512;
    let mut disk = Disk { file, sectors, writable };
    let mut entries = [0; 512]; assert!(disk.read_sector(2, &mut entries));
    let partition = u64::from_le_bytes(entries[160..168].try_into().unwrap());
    assert!(partition > 0 && partition < sectors);
    let mut store = ObjectStore::mount(disk, partition).unwrap();
    let owner = first_owner(&mut store);
    if writable {
        seed_inputs(&mut store, &directory, owner, &args[4], &args[5]);
    } else {
        let mut bytes = vec![0; package_limits::ARCHIVE_BYTES];
        let (revision, length) = store.read_python_packages(owner, &mut bytes).unwrap();
        std::fs::write(directory.join("packages.bin"), &bytes[..length]).unwrap();
        let foreign = ObjectId([0xa5; 16]); assert_ne!(foreign, owner);
        let (foreign_revision, foreign_length) = store.read_python_packages(foreign, &mut bytes).unwrap();
        assert!(foreign_revision.is_none() && foreign_length == 0);
        println!("{{\"archive_bytes\":{length},\"version\":{},\"foreign_owner_empty\":true,\"read_only\":true}}",
                 revision.map_or(0, |value| value.version));
    }
}
