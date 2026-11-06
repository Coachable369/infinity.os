#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "storage.rs"] mod storage;
#[path = "../kernel/ui/mod.rs"] mod ui;
use std::{cell::{Cell, RefCell}, collections::BTreeMap, rc::Rc};
use storage::{BlockDevice, object::{ObjectStore, ObjectType, ObjectError, Space, STORE_RELATIVE_LBA, SSH_SERVICE_OWNER}};
const SECTORS: u64 = STORE_RELATIVE_LBA + 32768;
#[derive(Clone)]
struct Disk(Rc<RefCell<BTreeMap<u64, [u8; 512]>>>, Rc<Cell<Option<usize>>>, u64);
// ------------------------=
// FUNC: output_text
// DESC: Supplies diagnostics without using text output as test evidence.
// ------------------=
fn output_text(_: &[u8]) {}
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Supplies fixed-capacity native sector storage.
    // ------------------=
    fn block_count(&self) -> u64 { self.2 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads persisted sparse sectors through the production BlockDevice contract.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        if lba >= self.2 { return false; }
        *out = self.0.borrow().get(&lba).copied().unwrap_or([0; 512]); true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Records native sector writes for cold-remount verification.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        if lba >= self.2 { return false; }
        if let Some(left) = self.1.get() {
            if left == 0 { return false; }
            self.1.set(Some(left - 1));
        }
        self.0.borrow_mut().insert(lba, *bytes); true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Completes synchronous persistence in the deterministic block device.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}
// ------------------------=
// FUNC: main
// DESC: Exercises service-private host-key persistence on the real object store, generic access denial and independent remount.
// ------------------=
fn main() {
    let disk = Disk(Rc::new(RefCell::new(BTreeMap::new())), Rc::new(Cell::new(None)), SECTORS);
    let mut store = ObjectStore::format(disk.clone(), 0, SECTORS, [0x73; 16]).unwrap();
    let path = b"/system/security/ssh/host-key";
    assert_eq!(store.resolve(path), Err(ObjectError::NamespaceNotFound));
    let bytes = [42; 74];
    let id = store.create_owned_attached(b"@ssh-host-key", ObjectType::Metadata, Space::System, &bytes, path, SSH_SERVICE_OWNER).unwrap();
    let mut out = [0; 74];
    assert_eq!(store.read(id, None, &mut out), Err(ObjectError::Unauthorized));
    assert_eq!(store.read(id, Some(1), &mut out), Err(ObjectError::Unauthorized));
    assert_eq!(store.write(id, &[1; 74]), Err(ObjectError::Unauthorized));
    assert_eq!(store.attach(b"/personal/key-alias", id), Err(ObjectError::Unauthorized));
    assert_eq!(store.copy_attached(id, b"/personal/key-copy"), Err(ObjectError::Unauthorized));
    assert_eq!(store.remove(id), Err(ObjectError::Unauthorized));
    assert!(store.create_attached(b"@ssh-host-key", ObjectType::Metadata, Space::System, &[1; 74], path).is_err());
    assert_eq!(store.read_ssh_private(id, b"@ssh-host-key", &mut out), Ok(74));
    assert_eq!(out, bytes);
    drop(store);
    let cold = Disk(Rc::new(RefCell::new(disk.0.borrow().clone())), Rc::new(Cell::new(None)), SECTORS);
    let mut store = ObjectStore::mount(cold, 0).unwrap();
    assert_eq!(store.resolve(path), Ok(id));
    assert_eq!(store.read_ssh_private(id, b"@ssh-host-key", &mut out), Ok(74));
    assert_eq!(out, bytes);
    assert_eq!(store.read(id, None, &mut out), Err(ObjectError::Unauthorized));
    assert_eq!(store.write(id, &[1; 74]), Err(ObjectError::Unauthorized));
    assert_eq!(store.read_ssh_private(id, b"identity-state", &mut out), Err(ObjectError::Unauthorized));
    let impostor = store.create_attached(b"@ssh-host-key", ObjectType::Metadata, Space::System,
        &bytes, b"/system/security/ssh/wrong-owner").unwrap();
    assert_eq!(store.read_ssh_private(impostor, b"@ssh-host-key", &mut out), Err(ObjectError::Unauthorized));
    account_key_transactions();
    enrollment_namespace_capacity();
}

// ------------------------=
// FUNC: storage_error
// DESC: Preserves ambiguous commits as fail-closed identity failures in the production-equivalent transaction harness.
// ------------------=
fn storage_error(error: ObjectError) -> runtime::identity::IdentityError {
    use runtime::identity::IdentityError;
    if error == ObjectError::CommitUncertain { IdentityError::CommitUncertain } else { IdentityError::InvalidState }
}

// ------------------------=
// FUNC: account_key_transactions
// DESC: Exercises generated account keys, registration, restricted storage, remount, creation rollback and identity isolation.
// ------------------=
fn account_key_transactions() {
    use runtime::identity::{IdentitySystem, IdentityError, AccountRequest, AccountResult, IDENTITY_STATE_BYTES};
    use storage::object::ObjectId;
    let disk = Disk(Rc::new(RefCell::new(BTreeMap::new())), Rc::new(Cell::new(None)), SECTORS);
    let mut store = ObjectStore::format(disk.clone(), 0, SECTORS, [0x74; 16]).unwrap();
    let mut identity = IdentitySystem::new();
    identity.begin_onboarding().unwrap();
    identity.create_machine(b"Enrollment", 0xaa64, 1, 1).unwrap();
    let keys = infinity_ssh::userkey::UserKeyPair::generate(|bytes| { bytes.fill(73); true }).unwrap();
    let first = identity.create_onboarding_user_durable(b"owner", b"Owner", 2, keys.public_key(), |encoded, user| {
        store.checkpoint_identity_enrolled(encoded, Some((ObjectId(user.id.0), user.display_name.as_bytes())),
            Some((keys.private_file(), keys.public_file()))).map(|_| ()).map_err(storage_error)
    }).unwrap();
    assert!(identity.ssh_key_active(first.id, &keys.public_key()));
    assert!(!identity.has_password_credential(first.id));
    identity.create_password(first.id, b"owner password", 3).unwrap();
    identity.complete_onboarding().unwrap();
    store.checkpoint_identity(&identity.encode(), None).unwrap();
    let session = identity.create_session(first.id, b"owner password", 4).unwrap();
    let private = store.resolve(b"/home/Owner/.ssh/id_ed25519").unwrap();
    let public = store.resolve(b"/home/Owner/.ssh/id_ed25519.pub").unwrap();
    let directory = store.resolve(b"/home/Owner/.ssh").unwrap();
    let mut out = [0; 512];
    for id in [private, public, directory] {
        assert_eq!(store.read(id, None, &mut out), Err(ObjectError::Unauthorized));
        assert_eq!(store.read(id, Some(1), &mut out), Err(ObjectError::Unauthorized));
        assert_eq!(store.attach(b"/personal/alias", id), Err(ObjectError::Unauthorized));
        assert_eq!(store.copy_attached(id, b"/personal/copy"), Err(ObjectError::Unauthorized));
        assert_eq!(store.write(id, b"replacement"), Err(ObjectError::Unauthorized));
        assert_eq!(store.remove(id), Err(ObjectError::Unauthorized));
    }
    assert_eq!(store.read_user_ssh_key(private, ObjectId([9; 16]), &mut out), Err(ObjectError::Unauthorized));
    let n = store.read_user_ssh_key(private, ObjectId(first.id.0), &mut out).unwrap();
    assert_eq!(&out[..n], keys.private_file());
    let second_keys = infinity_ssh::userkey::UserKeyPair::generate(|bytes| { bytes.fill(74); true }).unwrap();
    let request = || AccountRequest::CreateUser { handle: b"member", display_name: b"Member", password: b"member password", now: 5 };
    let before = identity.encode();
    assert_eq!(identity.manage_accounts_enrolled_durable(session.id, request(), Some(second_keys.public_key()), |_, _| Err(IdentityError::InvalidState)), Err(IdentityError::InvalidState));
    assert_eq!(identity.encode(), before);
    assert_eq!(identity.ssh_key_owner(b"member", &second_keys.public_key()), None);
    // A mid-transaction device failure must not expose a half-created user/home.
    disk.1.set(Some(5));
    assert!(identity.manage_accounts_enrolled_durable(session.id, request(), Some(second_keys.public_key()), |encoded, result| {
        let AccountResult::User(user) = result else { panic!() };
        store.checkpoint_identity_enrolled(encoded, Some((ObjectId(user.id.0), user.display_name.as_bytes())),
            Some((second_keys.private_file(), second_keys.public_file()))).map(|_| ()).map_err(storage_error)
    }).is_err());
    disk.1.set(None);
    assert_eq!(identity.encode(), before);
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert!(store.resolve(b"/home/Member").is_err());
    let AccountResult::User(second) = identity.manage_accounts_enrolled_durable(session.id, request(), Some(second_keys.public_key()), |encoded, result| {
        let AccountResult::User(user) = result else { panic!() };
        store.checkpoint_identity_enrolled(encoded, Some((ObjectId(user.id.0), user.display_name.as_bytes())),
            Some((second_keys.private_file(), second_keys.public_file()))).map(|_| ()).map_err(storage_error)
    }).unwrap() else { panic!() };
    assert_ne!(first.id, second.id);
    assert_ne!(keys.public_key(), second_keys.public_key());
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    let state = store.resolve(b"/system/identity/state").unwrap();
    let mut bytes = [0; IDENTITY_STATE_BYTES];
    let n = store.read_identity_state(state, &mut bytes).unwrap();
    let restored = IdentitySystem::decode(&bytes[..n]).unwrap();
    assert_eq!(restored.ssh_key_owner(b"member", &second_keys.public_key()), Some(second.id));
    assert_eq!(restored.ssh_key_owner(b"owner", &keys.public_key()), Some(first.id));
    let id = store.resolve(b"/home/Member/.ssh/id_ed25519").unwrap();
    let n = store.read_user_ssh_key(id, ObjectId(second.id.0), &mut out).unwrap();
    assert_eq!(&out[..n], second_keys.private_file());
    assert_eq!(store.read_user_ssh_key(id, ObjectId(first.id.0), &mut out), Err(ObjectError::Unauthorized));
    // Replaying account enrollment cannot overwrite existing keys or identity.
    assert_eq!(store.checkpoint_identity_enrolled(&restored.encode(), Some((ObjectId(first.id.0), b"Owner")),
        Some((second_keys.private_file(), second_keys.public_file()))), Err(ObjectError::NameConflict));
    let n = store.read_user_ssh_key(private, ObjectId(first.id.0), &mut out).unwrap();
    assert_eq!(&out[..n], keys.private_file());
}

// ------------------------=
// FUNC: enrollment_namespace_capacity
// DESC: Migrates a full v7 namespace, commits a face-sized object beyond its old limit, and checks cold readback without comparing whole disks.
// ------------------=
fn enrollment_namespace_capacity() {
    let disk=Disk(Rc::new(RefCell::new(BTreeMap::new())),Rc::new(Cell::new(None)), STORE_RELATIVE_LBA+262144);
    let mut store=ObjectStore::format(disk.clone(),0,disk.2,[0x75;16]).unwrap();
    let source=store.create_attached(b"marker",ObjectType::Text,Space::Personal,b"retained",b"/marker").unwrap();
    let mut index=0;
    while store.persistent_namespace_count()<44 {
        let path=format!("/alias{}",index);store.attach(path.as_bytes(),source).unwrap();index+=1;
    }
    drop(store);
    // Format 10 moved tables into allocated extents. A genuine v7 fixture
    // must relocate those tables; changing only the version corrupts it.
    for bank in [storage::object::BANK_A, storage::object::BANK_B] {
        let mut sectors=disk.0.borrow_mut();
        let header=sectors[&(STORE_RELATIVE_LBA+bank)];
        let extent=u32::from_le_bytes(header[40..44].try_into().unwrap()) as u64;
        let tables=STORE_RELATIVE_LBA+80+extent*8
            + if bank==storage::object::BANK_A {0} else {80};
        for i in 0..13 {
            let row=sectors[&(tables+i)];
            let offset=if i<6 {5+i} else {25+i-6};
            sectors.insert(STORE_RELATIVE_LBA+bank+offset,row);
        }
        for i in 0..8 {
            let row=sectors[&(tables+32+i)];
            let offset=if i<4 {bank+11+i} else {72+if bank==storage::object::BANK_A {0} else {4}+i-4};
            sectors.insert(STORE_RELATIVE_LBA+offset,row);
        }
        for i in 0..11 {
            let row=sectors[&(tables+48+i)];
            let offset=if i<8 {bank+15+i} else {2+if bank==storage::object::BANK_A {0} else {3}+i-8};
            sectors.insert(STORE_RELATIVE_LBA+offset,row);
        }
    }
    // Model an actual old-format checkpoint, with checksummed metadata and no v8 entries.
    for sector in disk.0.borrow_mut().values_mut() {
        if [b"INFOROOT".as_slice(),b"INFOSTAT",b"INFOOBJ2",b"INFOVER2",b"INFONSP2",b"INFOALC2",b"INFOREL2"].contains(&&sector[..8]) {
            sector[8..12].copy_from_slice(&7u32.to_le_bytes());sector[508..].fill(0);
            let sum=storage::object::crc32(sector);sector[508..].copy_from_slice(&sum.to_le_bytes());
        }
    }
    let mut store=ObjectStore::mount(disk.clone(),0).unwrap();
    assert_eq!(store.persistent_namespace_count(),44);
    let record=[0x59u8;11120];
    let face=store.create_attached(b"@face-identity",ObjectType::Metadata,Space::System,&record,b"/system/identity/face/capacity").unwrap();
    assert_eq!(store.persistent_namespace_count(),45);
    drop(store);
    let mut cold=ObjectStore::mount(disk,0).unwrap();
    let mut bytes=[0;11120];assert_eq!(cold.read(face,None,&mut bytes),Ok(record.len()));assert_eq!(bytes,record);
    assert_eq!(cold.read(source,None,&mut bytes),Ok(8));assert_eq!(&bytes[..8],b"retained");
}
