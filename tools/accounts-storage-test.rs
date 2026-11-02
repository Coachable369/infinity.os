#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "../kernel/storage/spatial_path.rs"]
mod spatial_path;
mod storage;
#[path = "../kernel/ui/mod.rs"]
mod ui;

use runtime::identity::{
    AccountRequest, AccountResult, IdentityError, IdentitySystem, StableId, UserIdentity,
    IDENTITY_STATE_BYTES,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};
use storage::{
    object::{
        ObjectError, ObjectId, ObjectMetadata, ObjectQueryRequest, ObjectStore, ObjectType, Space,
        STORE_RELATIVE_LBA,
    },
    BlockDevice,
};

const IDENTITY_PATH: &[u8] = b"/system/identity/state";
const PASSWORD: &[u8] = b"storage-fixture-password";
const SECTORS: u64 = STORE_RELATIVE_LBA + 32768;
type Image = BTreeMap<u64, [u8; 512]>;

// ------------------------=
// FUNC: output_text
// DESC: Supplies the full runtime host harness diagnostic sink.
// ------------------=
fn output_text(_: &[u8]) {}

#[derive(Clone)]
struct Disk {
    image: Rc<RefCell<Image>>,
    durable: Rc<RefCell<Image>>,
    writes: Rc<Cell<usize>>,
    fail_after: Rc<Cell<Option<usize>>>,
    flushes: Rc<Cell<usize>>,
    fail_flush_at: Rc<Cell<Option<usize>>>,
    discard_unflushed: Rc<Cell<bool>>,
}

impl Disk {
    // ------------------------=
    // FUNC: from_image
    // DESC: Opens an independent sector device with no ObjectStore or identity cache.
    // ------------------=
    fn from_image(image: Image) -> Self {
        Self {
            durable: Rc::new(RefCell::new(image.clone())),
            image: Rc::new(RefCell::new(image)),
            writes: Rc::new(Cell::new(0)),
            fail_after: Rc::new(Cell::new(None)),
            flushes: Rc::new(Cell::new(0)),
            fail_flush_at: Rc::new(Cell::new(None)),
            discard_unflushed: Rc::new(Cell::new(false)),
        }
    }

    // ------------------------=
    // FUNC: arm
    // DESC: Counts actual successful sector writes and optionally fails all later writes.
    // ------------------=
    fn arm(&self, fail_after: Option<usize>) {
        self.writes.set(0);
        self.fail_after.set(fail_after);
        self.flushes.set(0);
        self.fail_flush_at.set(None);
        self.discard_unflushed.set(false);
        *self.durable.borrow_mut() = self.image();
    }

    // ------------------------=
    // FUNC: arm_flush
    // DESC: Fails a selected flush with either persisted or lost writes since the last successful flush.
    // ------------------=
    fn arm_flush(&self, ordinal: usize, discard_unflushed: bool) {
        assert!((1..=2).contains(&ordinal));
        self.arm(None);
        self.fail_flush_at.set(Some(ordinal));
        self.discard_unflushed.set(discard_unflushed);
    }

    // ------------------------=
    // FUNC: image
    // DESC: Captures only persisted sector bytes for an independent cold mount.
    // ------------------=
    fn image(&self) -> Image {
        self.image.borrow().clone()
    }
}

impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Reports bounded media without allocating the unused kernel reservation.
    // ------------------=
    fn block_count(&self) -> u64 {
        SECTORS
    }

    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads persisted sectors and returns zeroes for unwritten sparse sectors.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        if lba >= SECTORS {
            return false;
        }
        *out = self.image.borrow().get(&lba).copied().unwrap_or([0; 512]);
        true
    }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Injects a physical write failure before changing the selected sector.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        if lba >= SECTORS
            || self
                .fail_after
                .get()
                .is_some_and(|limit| self.writes.get() >= limit)
        {
            return false;
        }
        self.image.borrow_mut().insert(lba, *bytes);
        self.writes.set(self.writes.get() + 1);
        true
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Models successful persistence and both legal media outcomes of a failed flush.
    // ------------------=
    fn flush(&mut self) -> bool {
        self.flushes.set(self.flushes.get() + 1);
        if self.fail_flush_at.get() == Some(self.flushes.get()) {
            if self.discard_unflushed.get() {
                *self.image.borrow_mut() = self.durable.borrow().clone();
            }
            return false;
        }
        *self.durable.borrow_mut() = self.image();
        true
    }
}

#[derive(PartialEq, Eq)]
struct Snapshot {
    generation: u64,
    allocated: u32,
    namespace: Vec<(Vec<u8>, ObjectId)>,
    objects: Vec<(ObjectMetadata, Vec<(u32, bool)>)>,
    identity: Vec<u8>,
}

// ------------------------=
// FUNC: read_identity
// DESC: Reads the exact serialized identity object through the production storage API.
// ------------------=
fn read_identity(store: &mut ObjectStore<Disk>) -> Vec<u8> {
    let id = store.resolve(IDENTITY_PATH).unwrap();
    let mut bytes = vec![0; IDENTITY_STATE_BYTES];
    assert_eq!(
        store.read_identity_state(id, &mut bytes).unwrap(),
        bytes.len()
    );
    bytes
}

// ------------------------=
// FUNC: snapshot
// DESC: Captures namespace, object ownership, history, allocation and identity bytes as rollback evidence.
// ------------------=
fn snapshot(store: &mut ObjectStore<Disk>) -> Snapshot {
    let mut namespace = Vec::new();
    while let Some((path, id)) = store.namespace_entry(namespace.len()) {
        namespace.push((path.to_vec(), id));
    }
    let query = ObjectQueryRequest {
        kind: None,
        space: None,
        include_tombstones: true,
    };
    let mut objects = Vec::new();
    while let Some(object) = store.query_nth(query, objects.len()) {
        let id = object.object.id;
        let history = (0..store.history_count(id))
            .map(|index| store.history_version_nth(id, index).unwrap())
            .collect();
        objects.push((store.metadata(id).unwrap(), history));
    }
    Snapshot {
        generation: store.generation(),
        allocated: store.usage_blocks(),
        namespace,
        objects,
        identity: read_identity(store),
    }
}

// ------------------------=
// FUNC: persist_request
// DESC: Couples real authorized account mutations to atomic home and identity persistence.
// ------------------=
fn persist_request(
    store: &mut ObjectStore<Disk>,
    identity: &mut IdentitySystem,
    session: StableId,
    request: AccountRequest<'_>,
) -> Result<AccountResult, IdentityError> {
    persist_observed(store, identity, session, request).0
}

// ------------------------=
// FUNC: persist_observed
// DESC: Captures the typed storage failure while using the production durable identity callback contract.
// ------------------=
fn persist_observed(
    store: &mut ObjectStore<Disk>,
    identity: &mut IdentitySystem,
    session: StableId,
    request: AccountRequest<'_>,
) -> (Result<AccountResult, IdentityError>, Option<ObjectError>) {
    let creating = matches!(&request, AccountRequest::CreateUser { .. });
    let mut storage_error = None;
    let result = identity.manage_accounts_durable(session, request, |encoded, result| {
        let home = match &result {
            AccountResult::User(user) if creating => {
                Some((ObjectId(user.id.0), user.display_name.as_bytes()))
            }
            _ => None,
        };
        let checkpoint = store.checkpoint_identity(encoded, home);
        storage_error = checkpoint.err();
        checkpoint.map(|_| ()).map_err(|error| match error {
            ObjectError::NameConflict => IdentityError::Conflict,
            ObjectError::CommitUncertain => IdentityError::CommitUncertain,
            _ => IdentityError::InvalidState,
        })
    });
    (result, storage_error)
}

// ------------------------=
// FUNC: create_request
// DESC: Supplies a synthetic account through the production password and authorization flow.
// ------------------=
fn create_request<'a>(handle: &'a [u8], display_name: &'a [u8]) -> AccountRequest<'a> {
    AccountRequest::CreateUser {
        handle,
        display_name,
        password: PASSWORD,
        now: 10,
    }
}

// ------------------------=
// FUNC: user_result
// DESC: Requires the typed user result instead of interpreting command output.
// ------------------=
fn user_result(result: AccountResult) -> UserIdentity {
    match result {
        AccountResult::User(user) => user,
        _ => panic!("expected user result"),
    }
}

// ------------------------=
// FUNC: fixture
// DESC: Formats native storage and durably provisions an authenticated administrator.
// ------------------=
fn fixture() -> (Disk, IdentitySystem, StableId) {
    let disk = Disk::from_image(Image::new());
    let mut store = ObjectStore::format(disk.clone(), 0, SECTORS, [0x73; 16]).unwrap();
    let mut identity = IdentitySystem::new();
    identity.begin_onboarding().unwrap();
    identity.create_machine(b"AccountStorage", 3, 1, 1).unwrap();
    let admin = identity.create_user(b"admin", b"Storage Admin", 2).unwrap();
    identity.create_password(admin.id, PASSWORD, 3).unwrap();
    identity.complete_onboarding().unwrap();
    let session = identity.create_session(admin.id, PASSWORD, 4).unwrap();
    store
        .checkpoint_identity(
            &identity.encode(),
            Some((ObjectId(admin.id.0), admin.display_name.as_bytes())),
        )
        .unwrap();
    assert_home(&store, b"/home/Storage.Admin", admin.id);
    assert_eq!(read_identity(&mut store), identity.encode());
    drop(store);
    (disk, identity, session.id)
}

// ------------------------=
// FUNC: child_path
// DESC: Constructs explicit test namespace paths without using production home allocation logic.
// ------------------=
fn child_path(parent: &[u8], child: &[u8]) -> Vec<u8> {
    [parent, b"/", child].concat()
}

// ------------------------=
// FUNC: suffixed_home
// DESC: Expresses an expected normalized prefix and the complete stable identity suffix.
// ------------------=
fn suffixed_home(normalized_prefix: &[u8], owner: StableId) -> Vec<u8> {
    let mut path = [b"/home/".as_slice(), normalized_prefix, b"."].concat();
    for byte in owner.0 {
        path.extend_from_slice(format!("{byte:02x}").as_bytes());
    }
    path
}

// ------------------------=
// FUNC: assert_home
// DESC: Requires personal namespace directories with exact immutable ownership at all three paths.
// ------------------=
fn assert_home(store: &ObjectStore<Disk>, home: &[u8], owner: StableId) {
    for path in [
        home.to_vec(),
        child_path(home, b"documents"),
        child_path(home, b"downloads"),
    ] {
        let metadata = store.metadata(store.resolve(&path).unwrap()).unwrap();
        assert_eq!(metadata.owner, ObjectId(owner.0));
        assert_eq!(metadata.space, Space::Personal);
        assert_eq!(metadata.kind, ObjectType::NamespaceNode);
    }
}

// ------------------------=
// FUNC: assert_file
// DESC: Checks persisted file identity, ownership and binary contents without rendered text.
// ------------------=
fn assert_file(
    store: &mut ObjectStore<Disk>,
    path: &[u8],
    id: ObjectId,
    owner: StableId,
    content: &[u8],
) {
    assert_eq!(store.resolve(path), Ok(id));
    assert_eq!(store.metadata(id).unwrap().owner, ObjectId(owner.0));
    let mut bytes = vec![0; content.len() + 1];
    let length = store.read(id, None, &mut bytes).unwrap();
    assert_eq!(&bytes[..length], content);
}

// ------------------------=
// FUNC: assert_uncertain_latched
// DESC: Restores healthy media and proves ambiguous commits block writes, account access and authentication until remount.
// ------------------=
fn assert_uncertain_latched(
    disk: &Disk,
    store: &mut ObjectStore<Disk>,
    identity: &mut IdentitySystem,
    session: StableId,
) {
    let before = snapshot(store);
    let encoded = identity.encode();
    let media = disk.image();
    let admin = identity.user_by_handle(b"admin").unwrap();
    assert!(identity.session_by_short(session.short()).is_none());
    assert!(matches!(
        identity.accounts(session),
        Err(IdentityError::CommitUncertain)
    ));
    assert_eq!(
        identity.authenticate(admin.id, PASSWORD, 20),
        Err(IdentityError::CommitUncertain)
    );
    assert_eq!(
        identity.create_session(admin.id, PASSWORD, 20),
        Err(IdentityError::CommitUncertain)
    );
    disk.arm(None);
    for _ in 0..2 {
        assert_eq!(
            store.checkpoint_identity(&encoded, None),
            Err(ObjectError::CommitUncertain)
        );
        assert_eq!(
            store.create_owned_attached(
                b"blocked.bin",
                ObjectType::Blob,
                Space::Personal,
                b"\x00\xff",
                b"/home/Storage.Admin/documents/blocked.bin",
                ObjectId(admin.id.0),
            ),
            Err(ObjectError::CommitUncertain)
        );
        assert_eq!(
            persist_observed(
                store,
                identity,
                session,
                create_request(b"member", b"New Member")
            ),
            (Err(IdentityError::CommitUncertain), None)
        );
    }
    assert_eq!(disk.writes.get(), 0);
    assert_eq!(disk.flushes.get(), 0);
    assert_eq!(identity.encode(), encoded);
    assert!(disk.image() == media);
    assert!(snapshot(store) == before);
}

// ------------------------=
// FUNC: recover_creation
// DESC: Cold-loads a complete old or new account generation and proves authentication and durable writes recover together.
// ------------------=
fn recover_creation(image: Image, expected: &Snapshot, member: Option<UserIdentity>) {
    let disk = Disk::from_image(image);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert!(snapshot(&mut store) == *expected);
    let mut identity = IdentitySystem::decode(&read_identity(&mut store)).unwrap();
    assert_eq!(identity.user_by_handle(b"member"), member);
    if let Some(user) = member {
        assert_home(&store, b"/home/New.Member", user.id);
    } else {
        for path in [
            b"/home/New.Member".as_slice(),
            b"/home/New.Member/documents",
            b"/home/New.Member/downloads",
        ] {
            assert_eq!(store.resolve(path), Err(ObjectError::NamespaceNotFound));
        }
    }
    let admin = identity.user_by_handle(b"admin").unwrap();
    let session = identity.create_session(admin.id, PASSWORD, 20).unwrap().id;
    assert!(identity.accounts(session).is_ok());
    let request = match member {
        Some(user) => AccountRequest::RenameUser {
            user: user.id,
            handle: b"member",
            display_name: b"Recovered Member",
        },
        None => create_request(b"member", b"New Member"),
    };
    let recovered =
        user_result(persist_request(&mut store, &mut identity, session, request).unwrap());
    assert_home(&store, b"/home/New.Member", recovered.id);
    assert_eq!(read_identity(&mut store), identity.encode());
    let after = snapshot(&mut store);
    drop(store);
    let mut store = ObjectStore::mount(Disk::from_image(disk.image()), 0).unwrap();
    assert!(snapshot(&mut store) == after);
    assert_home(&store, b"/home/New.Member", recovered.id);
    assert_eq!(
        IdentitySystem::decode(&read_identity(&mut store))
            .unwrap()
            .user_by_handle(b"member"),
        Some(recovered)
    );
}

// ------------------------=
// FUNC: write_failure_rollback
// DESC: Fails every creation write boundary, requiring remount before retrying an uncertain root write.
// ------------------=
fn write_failure_rollback() {
    let (disk, original, session) = fixture();
    let baseline = disk.image();
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let before = snapshot(&mut store);
    let mut identity = original.clone();
    disk.arm(None);
    let expected = user_result(
        persist_request(
            &mut store,
            &mut identity,
            session,
            create_request(b"member", b"New Member"),
        )
        .unwrap(),
    );
    let writes = disk.writes.get();
    assert!(writes > 1);
    assert_home(&store, b"/home/New.Member", expected.id);
    let successful = snapshot(&mut store);
    drop(store);

    for fail_after in 0..writes {
        let disk = Disk::from_image(baseline.clone());
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let mut identity = original.clone();
        disk.arm(Some(fail_after));
        let uncertain = fail_after + 1 == writes;
        let expected_error = if uncertain {
            (IdentityError::CommitUncertain, ObjectError::CommitUncertain)
        } else {
            (IdentityError::InvalidState, ObjectError::TransactionFailed)
        };
        assert_eq!(
            persist_observed(
                &mut store,
                &mut identity,
                session,
                create_request(b"member", b"New Member")
            ),
            (Err(expected_error.0), Some(expected_error.1)),
            "sector write boundary {fail_after}"
        );
        assert_eq!(disk.writes.get(), fail_after);
        assert!(
            snapshot(&mut store) == before,
            "in-memory rollback at write {fail_after}"
        );
        assert_eq!(identity.encode(), original.encode());
        if uncertain {
            assert_uncertain_latched(&disk, &mut store, &mut identity, session);
            drop(store);
            recover_creation(disk.image(), &before, None);
            continue;
        }
        assert_eq!(
            identity.session_by_short(session.short()),
            original.session_by_short(session.short())
        );
        drop(store);

        let reopened = Disk::from_image(disk.image());
        let mut store = ObjectStore::mount(reopened, 0).unwrap();
        assert!(
            snapshot(&mut store) == before,
            "cold-mount rollback at write {fail_after}"
        );
        assert!(IdentitySystem::decode(&read_identity(&mut store))
            .unwrap()
            .user_by_handle(b"member")
            .is_none());
        let retried = user_result(
            persist_request(
                &mut store,
                &mut identity,
                session,
                create_request(b"member", b"New Member"),
            )
            .unwrap(),
        );
        assert_eq!(retried, expected);
        assert!(
            snapshot(&mut store) == successful,
            "retry after write {fail_after}"
        );
    }
}

// ------------------------=
// FUNC: flush_failure_recovery
// DESC: Separates safe bank-flush rollback from ambiguous final-flush outcomes and checks exact cold-mount generations.
// ------------------=
fn flush_failure_recovery() {
    let (disk, original, session) = fixture();
    let baseline = disk.image();
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let before = snapshot(&mut store);
    let mut identity = original.clone();
    disk.arm(None);
    let member = user_result(
        persist_request(
            &mut store,
            &mut identity,
            session,
            create_request(b"member", b"New Member"),
        )
        .unwrap(),
    );
    assert_eq!(disk.flushes.get(), 2);
    let successful = snapshot(&mut store);
    drop(store);

    for ordinal in 1..=2 {
        for discard_unflushed in [false, true] {
            let disk = Disk::from_image(baseline.clone());
            let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
            let mut identity = original.clone();
            disk.arm_flush(ordinal, discard_unflushed);
            let expected_error = if ordinal == 1 {
                (IdentityError::InvalidState, ObjectError::TransactionFailed)
            } else {
                (IdentityError::CommitUncertain, ObjectError::CommitUncertain)
            };
            assert_eq!(
                persist_observed(
                    &mut store,
                    &mut identity,
                    session,
                    create_request(b"member", b"New Member")
                ),
                (Err(expected_error.0), Some(expected_error.1)),
                "flush {ordinal}, discard unflushed {discard_unflushed}"
            );
            assert_eq!(disk.flushes.get(), ordinal);
            assert!(snapshot(&mut store) == before);
            assert_eq!(identity.encode(), original.encode());
            if ordinal == 1 {
                assert_eq!(
                    identity.session_by_short(session.short()),
                    original.session_by_short(session.short())
                );
                assert!(identity.accounts(session).is_ok());
                let mut cold = ObjectStore::mount(Disk::from_image(disk.image()), 0).unwrap();
                assert!(snapshot(&mut cold) == before);
                drop(cold);
                disk.arm(None);
                assert_eq!(
                    user_result(
                        persist_request(
                            &mut store,
                            &mut identity,
                            session,
                            create_request(b"member", b"New Member")
                        )
                        .unwrap()
                    ),
                    member
                );
                assert!(snapshot(&mut store) == successful);
                drop(store);
                recover_creation(disk.image(), &successful, Some(member));
            } else {
                assert_uncertain_latched(&disk, &mut store, &mut identity, session);
                drop(store);
                if discard_unflushed {
                    recover_creation(disk.image(), &before, None);
                } else {
                    recover_creation(disk.image(), &successful, Some(member));
                }
            }
        }
    }
}

// ------------------------=
// FUNC: duplicate_names_and_conflicts
// DESC: Separates duplicate UTF-8 display names and rejects an occupied identity-suffixed home atomically.
// ------------------=
fn duplicate_names_and_conflicts() {
    let (disk, mut identity, session) = fixture();
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let name = [b"A".as_slice(), &[0xc3, 0xa9].repeat(12)].concat();
    let first = user_result(
        persist_request(
            &mut store,
            &mut identity,
            session,
            create_request(b"first", &name),
        )
        .unwrap(),
    );
    let second = user_result(
        persist_request(
            &mut store,
            &mut identity,
            session,
            create_request(b"second", &name),
        )
        .unwrap(),
    );
    let first_home = [b"/home/".as_slice(), &name].concat();
    // The 24-byte boundary splits the twelfth two-byte character; retain 23 bytes.
    let prefix = [b"A".as_slice(), &[0xc3, 0xa9].repeat(11)].concat();
    let second_home = suffixed_home(&prefix, second.id);
    assert_ne!(first.id, second.id);
    assert_home(&store, &first_home, first.id);
    assert_home(&store, &second_home, second.id);
    drop(store);
    let disk = Disk::from_image(disk.image());
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert_home(&store, &first_home, first.id);
    assert_home(&store, &second_home, second.id);
    let decoded = IdentitySystem::decode(&read_identity(&mut store)).unwrap();
    assert_eq!(decoded.user(first.id), Some(first));
    assert_eq!(decoded.user(second.id), Some(second));

    let mut preview = identity.clone();
    let candidate = user_result(
        preview
            .manage_accounts(session, create_request(b"collision", b"Storage Admin"))
            .unwrap(),
    );
    let occupied = suffixed_home(b"Storage.Admin", candidate.id);
    let admin = identity.user_by_handle(b"admin").unwrap();
    store
        .create_owned_attached(
            b"occupied",
            ObjectType::NamespaceNode,
            Space::Personal,
            b"",
            &occupied,
            ObjectId(admin.id.0),
        )
        .unwrap();
    let before = snapshot(&mut store);
    let before_memory = identity.encode();
    let before_media = disk.image();
    disk.arm(None);
    assert_eq!(
        store.checkpoint_identity(
            &preview.encode(),
            Some((ObjectId(candidate.id.0), candidate.display_name.as_bytes()))
        ),
        Err(ObjectError::NameConflict)
    );
    assert_eq!(
        persist_request(
            &mut store,
            &mut identity,
            session,
            create_request(b"collision", b"Storage Admin")
        ),
        Err(IdentityError::Conflict)
    );
    assert_eq!(disk.writes.get(), 0);
    assert_eq!(identity.encode(), before_memory);
    assert!(disk.image() == before_media);
    assert!(snapshot(&mut store) == before);
    drop(store);
    let mut store = ObjectStore::mount(Disk::from_image(disk.image()), 0).unwrap();
    assert!(snapshot(&mut store) == before);
}

// ------------------------=
// FUNC: updates_deletion_and_recreation
// DESC: Persists bounded account updates and deletion while keeping retained files isolated from a recreated account.
// ------------------=
fn updates_deletion_and_recreation() {
    let (disk, mut identity, session) = fixture();
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let member = user_result(
        persist_request(
            &mut store,
            &mut identity,
            session,
            create_request(b"member", b"New Member"),
        )
        .unwrap(),
    );
    let home = b"/home/New.Member";
    let paths = [
        child_path(home, b"documents/retained.bin"),
        child_path(home, b"downloads/retained.bin"),
    ];
    let contents: [&[u8]; 2] = [
        b"\x00\xffprivate-document\x80",
        b"\xfe\x00private-download\x01",
    ];
    let mut files = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        files.push(
            store
                .create_owned_attached(
                    b"retained.bin",
                    ObjectType::Blob,
                    Space::Personal,
                    contents[index],
                    path,
                    ObjectId(member.id.0),
                )
                .unwrap(),
        );
    }
    let group = match persist_request(
        &mut store,
        &mut identity,
        session,
        AccountRequest::CreateGroup {
            name: b"team",
            display_name: b"Team",
            now: 11,
        },
    )
    .unwrap()
    {
        AccountResult::Group(group) => group,
        _ => panic!("expected group result"),
    };
    persist_request(
        &mut store,
        &mut identity,
        session,
        AccountRequest::AddMember {
            group: group.id,
            user: member.id,
        },
    )
    .unwrap();
    let identity_id = store.resolve(IDENTITY_PATH).unwrap();
    let initial_version = store.current_version(identity_id).unwrap();
    let allocated = store.usage_blocks();
    let namespace = snapshot(&mut store).namespace;
    for index in 0..200u32 {
        let display = format!("Member {index}");
        persist_request(
            &mut store,
            &mut identity,
            session,
            AccountRequest::RenameUser {
                user: member.id,
                handle: b"member",
                display_name: display.as_bytes(),
            },
        )
        .unwrap();
        assert_eq!(
            store.current_version(identity_id).unwrap(),
            initial_version + index + 1
        );
        assert_eq!(store.history_count(identity_id), 1);
        assert_eq!(store.usage_blocks(), allocated);
    }
    assert_eq!(snapshot(&mut store).namespace, namespace);
    drop(store);
    let disk = Disk::from_image(disk.image());
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    identity = IdentitySystem::decode(&read_identity(&mut store)).unwrap();
    assert_eq!(
        identity.user(member.id).unwrap().display_name.as_bytes(),
        b"Member 199"
    );
    assert!(identity.is_group_member(group.id, member.id));
    assert_eq!(store.history_count(identity_id), 1);
    let admin = identity.user_by_handle(b"admin").unwrap();
    let session = identity.create_session(admin.id, PASSWORD, 20).unwrap().id;
    persist_request(
        &mut store,
        &mut identity,
        session,
        AccountRequest::RenameUser {
            user: member.id,
            handle: b"member",
            display_name: b"New Member",
        },
    )
    .unwrap();
    let deleted = persist_request(
        &mut store,
        &mut identity,
        session,
        AccountRequest::DeleteUser { user: member.id },
    )
    .unwrap();
    assert_eq!(
        deleted,
        AccountResult::DeletedUser {
            user: member.id,
            preserved_personal_space: member.personal_space_ref,
        }
    );
    assert_eq!(snapshot(&mut store).namespace, namespace);
    drop(store);

    let disk = Disk::from_image(disk.image());
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    identity = IdentitySystem::decode(&read_identity(&mut store)).unwrap();
    assert!(identity.user(member.id).is_none());
    assert!(identity.personal_space(member.id).is_none());
    assert!(!identity.has_active_credential(member.id));
    assert!(!identity.is_group_member(group.id, member.id));
    assert_home(&store, home, member.id);
    for index in 0..2 {
        assert_file(
            &mut store,
            &paths[index],
            files[index],
            member.id,
            contents[index],
        );
    }
    let session = identity.create_session(admin.id, PASSWORD, 30).unwrap().id;
    let replacement = user_result(
        persist_request(
            &mut store,
            &mut identity,
            session,
            create_request(b"member", b"New Member"),
        )
        .unwrap(),
    );
    assert_ne!(replacement.id, member.id);
    assert_ne!(replacement.personal_space_ref, member.personal_space_ref);
    assert!(!identity.is_group_member(group.id, replacement.id));
    let replacement_home = suffixed_home(b"New.Member", replacement.id);
    assert_home(&store, &replacement_home, replacement.id);
    drop(store);

    let mut store = ObjectStore::mount(Disk::from_image(disk.image()), 0).unwrap();
    let mut restored = IdentitySystem::decode(&read_identity(&mut store)).unwrap();
    assert_eq!(restored.user_by_handle(b"member"), Some(replacement));
    assert!(restored.user(member.id).is_none());
    assert_home(&store, home, member.id);
    assert_home(&store, &replacement_home, replacement.id);
    restored
        .create_session(replacement.id, PASSWORD, 40)
        .unwrap();
    for index in 0..2 {
        assert_file(
            &mut store,
            &paths[index],
            files[index],
            member.id,
            contents[index],
        );
        let new_file = child_path(
            &replacement_home,
            if index == 0 {
                b"documents/retained.bin"
            } else {
                b"downloads/retained.bin"
            },
        );
        assert_eq!(
            store.resolve(&new_file),
            Err(ObjectError::NamespaceNotFound)
        );
    }
}

// ------------------------=
// FUNC: main
// DESC: Exercises account storage atomicity, ownership, durable history and retained-data isolation.
// ------------------=
fn main() {
    write_failure_rollback();
    flush_failure_recovery();
    duplicate_names_and_conflicts();
    updates_deletion_and_recreation();
    println!("Account storage behavioral checks passed");
}
