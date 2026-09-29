#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "../kernel/ui/mod.rs"] mod ui;
mod storage;
#[path = "../kernel/storage/spatial_path.rs"] mod spatial_path;
use std::{fs::{File, OpenOptions}, io::{Read, Seek, SeekFrom, Write}};
use storage::{BlockDevice, object::{ObjectStore, STORE_RELATIVE_LBA}};
use runtime::identity::{IdentitySystem, OnboardingState, VoiceActivation, IDENTITY_STATE_BYTES};

// ------------------------=
// FUNC: output_text
// DESC: Supplies the host harness diagnostic sink.
// ------------------=
fn output_text(_: &[u8]) {}

struct Disk(File, u64);
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Exposes bounded sparse test media geometry.
    // ------------------=
    fn block_count(&self) -> u64 { self.1 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads actual file-backed sectors across independent mounts.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8;512]) -> bool {
        lba < self.1 && self.0.seek(SeekFrom::Start(lba*512)).is_ok() && self.0.read_exact(out).is_ok()
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes actual bounded file-backed sectors.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8;512]) -> bool {
        lba < self.1 && self.0.seek(SeekFrom::Start(lba*512)).is_ok() && self.0.write_all(bytes).is_ok()
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Flushes durable test media rather than retaining only an in-memory fixture.
    // ------------------=
    fn flush(&mut self) -> bool { self.0.sync_all().is_ok() }
}

// ------------------------=
// FUNC: main
// DESC: Completes setup, checkpoints repeated settings, cold-reopens media, and verifies setup and microphone opt-out survive.
// ------------------=
fn main() {
    std::fs::create_dir_all("build/tests").unwrap();
    let path = format!("build/tests/settings-reboot-{}.img", std::process::id());
    let sectors = STORE_RELATIVE_LBA + 32768;
    let file = OpenOptions::new().create_new(true).read(true).write(true).open(&path).unwrap();
    file.set_len(sectors*512).unwrap();
    let mut store = ObjectStore::format(Disk(file, sectors), 0, sectors, [0x43;16]).unwrap();
    let mut identity = IdentitySystem::new();
    identity.begin_onboarding().unwrap();
    identity.create_machine(b"RebootTest", 3, 1, 1).unwrap();
    let user = identity.create_user(b"test", b"Test", 2).unwrap();
    identity.create_password(user.id, b"fixture-only-password", 3).unwrap();
    identity.complete_onboarding().unwrap();
    assert_eq!(identity.voice_profile(user.id).unwrap().activation, VoiceActivation::Continuous);
    let key = b"/system/identity/state";
    let id = store.resolve(key).unwrap();
    let mut exhausted = false;
    for _ in 0..64 {
        if store.write(id, &identity.encode()) == Err(storage::object::ObjectError::InsufficientCapacity) {
            exhausted = true;
            break;
        }
    }
    assert!(exhausted, "Reproduce the previous append-only settings failure");
    // More than the entire global version table: a settings checkpoint must
    // not consume a new permanent history slot on every UI edit.
    for i in 0..200 {
        identity.update_speech_output(user.id, user.id, i % 2 == 0).unwrap();
        identity.update_user_theme(user.id,user.id,b"saved-theme").unwrap();
        store.checkpoint(key, &identity.encode()).unwrap();
    }
    drop(store);
    let file = OpenOptions::new().read(true).open(&path).unwrap();
    let mut read_only = ObjectStore::mount(Disk(file,sectors),0).unwrap();
    identity.update_speech_output(user.id,user.id,true).unwrap();
    assert!(read_only.checkpoint(key,&identity.encode()).is_err());
    drop(read_only);
    let file = OpenOptions::new().read(true).write(true).open(&path).unwrap();
    let mut store = ObjectStore::mount(Disk(file,sectors),0).unwrap();
    let id = store.resolve(key).unwrap();
    let mut bytes = [0u8;IDENTITY_STATE_BYTES];
    assert_eq!(store.read(id,None,&mut bytes).unwrap(),bytes.len());
    let mut restored = IdentitySystem::decode(&bytes).unwrap();
    assert_eq!(restored.onboarding_state(), OnboardingState::Complete);
    assert!(!restored.ai_profile(user.id).unwrap().speech_output_enabled);
    assert_eq!(restored.user_profile(user.id).unwrap().theme.as_bytes(),b"saved-theme");
    assert!(restored.voice_profile(user.id).unwrap().enabled);
    restored.create_session(user.id,b"fixture-only-password",4).unwrap();
    restored.update_voice_profile(user.id,user.id,false,VoiceActivation::Disabled).unwrap();
    store.checkpoint(key,&restored.encode()).unwrap();
    drop(store);
    let file = OpenOptions::new().read(true).write(true).open(&path).unwrap();
    let mut store = ObjectStore::mount(Disk(file,sectors),0).unwrap();
    let id = store.resolve(key).unwrap();
    store.read(id,None,&mut bytes).unwrap();
    let restored = IdentitySystem::decode(&bytes).unwrap();
    assert_eq!(restored.onboarding_state(),OnboardingState::Complete);
    assert!(!restored.voice_profile(user.id).unwrap().enabled);
    assert_eq!(restored.voice_profile(user.id).unwrap().activation,VoiceActivation::Disabled);
    let mut legacy = bytes;
    let user_offset = 160;
    legacy[user_offset + 158] &= !1;
    legacy[user_offset + 159] = 0;
    let mut checksum = 0x811c9dc5u32;
    for byte in &legacy[..legacy.len() - 4] {
        checksum ^= u32::from(*byte);
        checksum = checksum.wrapping_mul(0x01000193);
    }
    let checksum_at = legacy.len() - 4;
    legacy[checksum_at..].copy_from_slice(&checksum.to_le_bytes());
    let migrated = IdentitySystem::decode(&legacy).unwrap();
    assert!(migrated.voice_profile(user.id).unwrap().enabled);
    assert_eq!(migrated.voice_profile(user.id).unwrap().activation,VoiceActivation::Continuous);
    drop(store);
    std::fs::remove_file(path).unwrap();
    println!("200 durable checkpoints and two independent media remounts passed");
}
