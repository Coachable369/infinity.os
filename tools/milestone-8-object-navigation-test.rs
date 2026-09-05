#![allow(dead_code)]

#[path = "../kernel/runtime/capability.rs"]
mod capability;
#[path = "../kernel/runtime/execution.rs"]
mod execution;
#[path = "../kernel/runtime/iop.rs"]
mod iop;
#[path = "../kernel/runtime/object_navigation.rs"]
mod object_navigation;
#[path = "storage.rs"]
mod storage;

use object_navigation::*;
use std::{cell::RefCell, fs, rc::Rc};
use storage::{
    object::{ObjectError, ObjectStore, ObjectType, Space},
    BlockDevice,
};

#[derive(Clone)]
struct MemoryDisk(Rc<RefCell<Vec<[u8; 512]>>>);

impl MemoryDisk {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a deterministic in-memory block device for behavioral Object Store tests.
    // ------------------=
    fn new(sectors: usize) -> Self {
        Self(Rc::new(RefCell::new(vec![[0; 512]; sectors])))
    }
}

impl BlockDevice for MemoryDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Returns the mock device capacity.
    // ------------------=
    fn block_count(&self) -> u64 {
        self.0.borrow().len() as u64
    }

    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one exact mock sector.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        let Some(sector) = self.0.borrow().get(lba as usize).copied() else {
            return false;
        };
        *out = sector;
        true
    }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes one exact mock sector.
    // ------------------=
    fn write_sector(&mut self, lba: u64, input: &[u8; 512]) -> bool {
        let mut disk = self.0.borrow_mut();
        let Some(sector) = disk.get_mut(lba as usize) else {
            return false;
        };
        *sector = *input;
        true
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Commits immediately in the deterministic mock device.
    // ------------------=
    fn flush(&mut self) -> bool {
        true
    }
}

// ------------------------=
// FUNC: navigation_context_behavior
// DESC: Verifies CurrentNamespaceRef isolation, native cd semantics, and no compatibility ownership.
// ------------------=
fn navigation_context_behavior() {
    let visible =
        |path: &[u8]| matches!(path, b"/home/alice" | b"/home/alice/demo" | b"/home" | b"/");
    let mut first = ConsoleNavigationContext::new(1, 11, b"/home/alice").unwrap();
    let second = ConsoleNavigationContext::new(2, 11, b"/home/alice").unwrap();
    assert_eq!(first.path(), b"/home/alice");
    assert_eq!(
        first.navigate(b"demo", visible).unwrap(),
        b"/home/alice/demo"
    );
    assert_eq!(second.path(), b"/home/alice");
    assert_eq!(first.navigate(b"..", visible).unwrap(), b"/home/alice");
    assert_eq!(first.navigate(b"-", visible).unwrap(), b"/home/alice/demo");
    assert_eq!(first.navigate(b"~", visible).unwrap(), b"/home/alice");
    assert!(is_native_command(b"cd"));
    assert!(is_native_command(b"idir"));
    assert!(is_native_command(b"path"));
    assert!(!is_native_command(b"pwd"));
}

// ------------------------=
// FUNC: shell_profile_behavior
// DESC: Verifies enablement, custom lifecycle, deterministic precedence, validation, and reboot persistence.
// ------------------=
fn shell_profile_behavior() {
    let mut profiles = ShellProfileService::new();
    assert_eq!(profiles.resolve(b"pwd"), Err(ProfileError::NotFound));
    assert_eq!(profiles.resolve(b"cd /home").unwrap().resolved_profile, 1);
    profiles.enable(7, b"linux", 2).unwrap();
    let linux = profiles.resolve(b"pwd").unwrap();
    assert_eq!(linux.canonical.as_bytes(), b"path");
    profiles.disable(7, b"compat.linux", 3).unwrap();
    assert_eq!(profiles.resolve(b"pwd"), Err(ProfileError::NotFound));

    let first = profiles.create(7, b"dev-one", 4).unwrap();
    profiles
        .alias_add(7, b"dev-one", b"ll", b"list", 5)
        .unwrap();
    profiles.enable(7, b"dev-one", 6).unwrap();
    let second = profiles.create(7, b"dev-two", 7).unwrap();
    profiles
        .alias_add(7, b"dev-two", b"ll", b"list tree", 8)
        .unwrap();
    profiles.enable(7, b"dev-two", 9).unwrap();
    let resolved = profiles.resolve(b"ll /home").unwrap();
    assert_eq!(resolved.resolved_profile, first.min(second));
    assert_eq!(resolved.shadowed_count, 1);
    assert_eq!(resolved.canonical.as_bytes(), b"list /home");
    assert_eq!(
        profiles.alias_add(7, b"dev-one", b"cd", b"path", 10),
        Err(ProfileError::Immutable)
    );
    assert_eq!(
        profiles.alias_add(7, b"dev-one", "bad\u{200b}".as_bytes(), b"list", 10),
        Err(ProfileError::InvalidName)
    );
    assert_eq!(
        profiles.alias_add(7, b"dev-one", b"evil", b"list; object destroy", 10),
        Err(ProfileError::InvalidTemplate)
    );
    profiles.alias_delete(7, b"dev-one", b"ll", 11).unwrap();
    assert_eq!(profiles.resolve(b"ll").unwrap().resolved_profile, second);
    assert_eq!(
        profiles.delete(7, b"infinity.native"),
        Err(ProfileError::Immutable)
    );

    let encoded = profiles.encode();
    let restored = ShellProfileService::decode(&encoded).unwrap();
    assert_eq!(
        restored.resolve(b"ll").unwrap().canonical.as_bytes(),
        b"list tree"
    );
    let mut corrupt = encoded;
    corrupt[400] ^= 0x5a;
    assert_eq!(
        ShellProfileService::decode(&corrupt).err(),
        Some(ProfileError::CorruptState)
    );
}

// ------------------------=
// FUNC: object_reference_behavior
// DESC: Verifies native create, move, copy, reference, Trash, namespace-delete, and explicit-destroy identity rules.
// ------------------=
fn object_reference_behavior() {
    let sectors = storage::object::STORE_RELATIVE_LBA as usize + 32_768;
    let disk = MemoryDisk::new(sectors);
    let mut store = ObjectStore::format(disk, 0, sectors as u64, [0x38; 16]).unwrap();
    assert!(store.runtime_bootstrap_valid());
    assert!(store.resolve(b"/system/settings/shell/profiles").is_ok());

    let namespace = store
        .create_attached(
            b"demo",
            ObjectType::NamespaceNode,
            Space::Personal,
            b"",
            b"/home/default/demo",
        )
        .unwrap();
    let source = store
        .create_attached(
            b"report",
            ObjectType::Text,
            Space::Personal,
            b"version one",
            b"/home/default/demo/report",
        )
        .unwrap();
    store
        .move_entry(b"/home/default/demo/report", b"/home/default/demo/renamed")
        .unwrap();
    assert_eq!(
        store.resolve(b"/home/default/demo/renamed").unwrap(),
        source
    );
    let copied = store
        .copy_attached(source, b"/home/default/demo/copied")
        .unwrap();
    assert_ne!(copied, source);
    store.attach(b"/shared/report", source).unwrap();
    assert_eq!(store.resolve(b"/shared/report").unwrap(), source);
    assert_eq!(store.namespace_refs(source), 2);
    store.detach(b"/shared/report").unwrap();
    assert_eq!(
        store.resolve(b"/home/default/demo/renamed").unwrap(),
        source
    );

    store
        .move_entry(
            b"/home/default/demo/renamed",
            b"/trash/home/default/demo/renamed",
        )
        .unwrap();
    assert_eq!(
        store.resolve(b"/trash/home/default/demo/renamed").unwrap(),
        source
    );
    store
        .move_entry(
            b"/trash/home/default/demo/renamed",
            b"/home/default/demo/renamed",
        )
        .unwrap();
    assert_eq!(
        store.resolve(b"/home/default/demo/renamed").unwrap(),
        source
    );
    assert_eq!(
        store.delete_namespace(b"/home/default/demo"),
        Err(ObjectError::Busy)
    );
    store.remove_path(b"/home/default/demo/renamed").unwrap();
    store.remove_path(b"/home/default/demo/copied").unwrap();
    assert_eq!(
        store.delete_namespace(b"/home/default/demo").unwrap(),
        namespace
    );
    store.collect().unwrap();

    let protected = store.resolve(b"/system/runtime").unwrap();
    assert_eq!(
        store.destroy_explicit(protected, true),
        Err(ObjectError::Unauthorized)
    );
    let disposable = store
        .create(b"disposable", ObjectType::Text, Space::Personal, b"x")
        .unwrap();
    assert_eq!(
        store.destroy_explicit(disposable, false),
        Err(ObjectError::Unauthorized)
    );
    store.destroy_explicit(disposable, true).unwrap();
    assert!(!store.object_exists(disposable));
}

// ------------------------=
// FUNC: file_navigator_behavior
// DESC: Verifies app identity, explicit navigation history, large-result virtualization, and generated icon framing.
// ------------------=
fn file_navigator_behavior() {
    assert_eq!(
        FILE_NAVIGATOR_APPLICATION_ID,
        b"app.infinity.file-navigator"
    );
    assert_eq!(FILE_NAVIGATOR_DEFAULT_SIZE, (780, 560));
    assert_eq!(FILE_NAVIGATOR_MINIMUM_SIZE, (640, 420));
    assert_eq!(FILE_NAVIGATOR_INTENTS.len(), 3);
    let mut navigator = FileNavigatorState::new(b"/home/default").unwrap();
    navigator.navigate(b"/home/default/projects").unwrap();
    navigator.back().unwrap();
    assert_eq!(navigator.active_namespace_ref.as_bytes(), b"/home/default");
    navigator.forward().unwrap();
    assert_eq!(
        navigator.active_namespace_ref.as_bytes(),
        b"/home/default/projects"
    );
    let (first, end) = FileNavigatorState::visible_range(10_000, 9_940 * 28, 560, 28);
    assert!(first >= 9_940);
    assert_eq!(end, 9_962);

    let icon = fs::read("assets/apps/file-navigator-icon-v1.png").unwrap();
    assert_eq!(&icon[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(u32::from_be_bytes(icon[16..20].try_into().unwrap()), 256);
    assert_eq!(u32::from_be_bytes(icon[20..24].try_into().unwrap()), 256);
    assert_eq!(icon[25], 6);
}

// ------------------------=
// FUNC: operation_registry_behavior
// DESC: Verifies lifecycle symmetry and all required command signatures resolve to typed IOP operations.
// ------------------=
fn operation_registry_behavior() {
    for (create, delete) in [
        (
            b"namespace create".as_slice(),
            b"namespace delete".as_slice(),
        ),
        (b"object create".as_slice(), b"object delete".as_slice()),
        (
            b"reference create".as_slice(),
            b"reference delete".as_slice(),
        ),
        (
            b"shell profile create".as_slice(),
            b"shell profile delete".as_slice(),
        ),
        (
            b"shell alias add".as_slice(),
            b"shell alias delete".as_slice(),
        ),
        (b"trash add".as_slice(), b"trash restore".as_slice()),
    ] {
        assert!(native_operation(create).is_some());
        assert!(native_operation(delete).is_some());
    }
    assert_eq!(
        native_operation(b"object copy"),
        Some(iop::OperationId::ObjectCopy)
    );
    assert_eq!(
        native_operation(b"namespace move"),
        Some(iop::OperationId::NamespaceMove)
    );
    assert_eq!(
        native_operation(b"navigator"),
        Some(iop::OperationId::ApplicationLaunch)
    );
}

// ------------------------=
// FUNC: main
// DESC: Runs the Milestone 8 behavioral acceptance suite.
// ------------------=
fn main() {
    navigation_context_behavior();
    shell_profile_behavior();
    object_reference_behavior();
    file_navigator_behavior();
    operation_registry_behavior();
    println!("PASS Milestone 8 native Object navigation, File Navigator, Shell Profiles, identity semantics, Trash, security, persistence, and bounded large-result behavior");
}
