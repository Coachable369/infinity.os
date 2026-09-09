use super::{resources::*, placement::*};
use crate::runtime::node::{NodeRuntime, types::NodeId};
use super::replica::*;
use sha2::{Digest, Sha256};

// ------------------------=
// FUNC: manifest_counts_only_verified_independent_current_versions
// DESC: Exercises current-version availability, stale isolation, same-node failure domains and canonical bounded decoding.
// ------------------=
#[test]
fn manifest_counts_only_verified_independent_current_versions() {
    use super::manifest::*;
    let hash: [u8; 32] = Sha256::digest([7; 32]).into();
    let mut manifest = Manifest { object: [1; 16], version: 1, length: 32, hash,
        policy: StorageClass::Critical, minimum_available: 1, generation: 1, authority: NodeId([2; 32]), authority_generation: 1,
        chunks: [None; MAX_CHUNKS], placements: [None; MAX_PLACEMENTS], healing: None };
    manifest.chunks[0] = Some(Chunk { content: [3; 16], bytes: 32, hash });
    for index in 0..3 {
        manifest.placements[index] = Some(Placement { node: NodeId([index as u8 + 4; 32]),
            resource: ResourceId([index as u8 + 4; 16]), device: [index as u8 + 4; 16], generation: 1,
            version: 1, hash, state: PlacementState::Verified });
    }
    assert_eq!(manifest.validate(), Ok(()));
    assert_eq!(manifest.availability(), Availability::Healthy);
    manifest.placements[2].as_mut().unwrap().state = PlacementState::Offline;
    assert_eq!(manifest.availability(), Availability::Degraded);
    manifest.placements[2].as_mut().unwrap().state = PlacementState::Verified;
    manifest.placements[2].as_mut().unwrap().node = NodeId([4; 32]);
    assert_eq!(manifest.availability(), Availability::Degraded);
    manifest.placements[2].as_mut().unwrap().version = 2;
    assert_eq!(manifest.validate(), Err(ManifestError::Stale));
    manifest.placements[2].as_mut().unwrap().state = PlacementState::Stale;
    assert_eq!(manifest.validate(), Ok(()));
    let mut bytes = [0; MANIFEST_BYTES]; manifest.encode(&mut bytes).unwrap();
    assert_eq!(Manifest::decode(&bytes), Ok(manifest));
    for length in 0..MANIFEST_BYTES { assert!(Manifest::decode(&bytes[..length]).is_err()); }
    bytes[74] = 1; assert_eq!(Manifest::decode(&bytes), Err(ManifestError::Invalid));
    let mut next = manifest; next.generation += 1; next.chunks[0].as_mut().unwrap().content = [8; 16];
    assert_eq!(manifest.successor(&next), Err(ManifestError::Conflict));
}

// ------------------------=
// FUNC: resource
// DESC: Creates explicitly labeled host-fixture storage observations, not runtime-discovered hardware.
// ------------------=
fn resource(id: u8, owner: NodeId) -> Resource {
    Resource { id: ResourceId([id; 16]), owner, kind: ResourceKind::Storage, device: [id; 16],
        capacity: 4096, available: 2048, reserved: 0, health: Health::Healthy, online: true,
        capabilities: 1, generation: 1, sequence: 1, expires: 100 }
}

// ------------------------=
// FUNC: resource_loss_notifications_are_bounded_and_generation_fenced
// DESC: Exercises expiry and disconnect transitions, delivery backpressure, duplicate loss suppression and recovery without losing resource identity or reservations.
// ------------------=
#[test]
fn resource_loss_notifications_are_bounded_and_generation_fenced() {
    let mut directory = Directory::new();
    let peer = NodeId([7; 32]);
    for id in 1..=32 { directory.apply(resource(id, peer), 1).unwrap(); }
    directory.reserve(ResourceId([1; 16]), 1, 512, 2).unwrap();
    directory.expire(100);
    let first = directory.offline_notice().unwrap();
    assert!(!first.online);
    assert_eq!(directory.entries().iter().flatten().count(), 32);
    assert_eq!(directory.usable(0, 100), 0);
    for _ in 0..100 {
        directory.expire(101);
        directory.mark_peer_offline(peer);
        assert_eq!(directory.offline_notice(), Some(first));
    }
    let mut delivered = 0;
    while let Some(value) = directory.offline_notice() {
        directory.acknowledge_offline(value); delivered += 1;
        assert!(delivered <= 32);
    }
    assert_eq!(delivered, 32);
    directory.expire(102); assert_eq!(directory.offline_notice(), None);
    let mut recovered = resource(1, peer); recovered.sequence = 2; recovered.expires = 200;
    directory.apply(recovered, 103).unwrap();
    assert_eq!(directory.usable(0, 103), 1536);
    directory.mark_peer_offline(peer);
    directory.acknowledge_offline(first);
    assert_eq!(directory.offline_notice().unwrap().sequence, 2);
    recovered.sequence = 3;
    directory.apply(recovered, 104).unwrap();
    assert_eq!(directory.offline_notice(), None);
    assert_eq!(directory.usable(0, 104), 1536);
}

// ------------------------=
// FUNC: authority_is_not_discovery
// DESC: Exercises real native session/grant validation, exact ownership, scope, revocation and lease failure.
// ------------------=
#[test]
fn authority_is_not_discovery() {
    let mut nodes = NodeRuntime::new(); let mut peer = NodeRuntime::new();
    nodes.initialize(&[11; 32], true).unwrap();
    let id = peer.initialize(&[12; 32], true).unwrap();
    nodes.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
    let mut directory = Directory::new();
    let value = resource(1, id);
    assert_eq!(directory.advertise(&nodes, 0, 0, 7, value, 2), Err(ResourceError::AccessDenied));
    let pair = nodes.begin_pairing(id, 2).unwrap();
    nodes.confirm_pairing(pair.id, pair.verification_code, true, 3, 1).unwrap();
    let peer_public = x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from([14; 32])).to_bytes();
    let session = nodes.open_session(id, &[13; 32], &peer_public, b"resource-admission-host-fixture", 4, 1).unwrap();
    let grant = nodes.grant_remote(id, ADVERTISE_OPERATION, 7, 1, 50, 4, 1).unwrap();
    assert_eq!(directory.advertise(&nodes, session, grant, 8, value, 5), Err(ResourceError::AccessDenied));
    let mut wrong_owner = value; wrong_owner.owner = nodes.local_id().unwrap();
    assert_eq!(directory.advertise(&nodes, session, grant, 7, wrong_owner, 5), Err(ResourceError::AccessDenied));
    assert_eq!(directory.advertise(&nodes, session, grant, 7, value, 5), Ok(true));
    assert_eq!(directory.advertise(&nodes, session, grant, 7, value, 6), Ok(false));
    assert_eq!(directory.advertise(&nodes, session, grant, 7, value, 50), Err(ResourceError::AccessDenied));
    nodes.revoke_remote(grant, 7, 1).unwrap();
    assert_eq!(directory.advertise(&nodes, session, grant, 7, value, 8), Err(ResourceError::AccessDenied));
    assert_eq!(directory.entries().iter().flatten().count(), 1);
}

// ------------------------=
// FUNC: versions_capacity_and_independent_nodes
// DESC: Verifies deterministic independent placement, reservations, stale/conflicting updates, expiry and generation fencing.
// ------------------=
#[test]
fn versions_capacity_and_independent_nodes() {
    let a = NodeId([1; 32]); let b = NodeId([2; 32]);
    let mut directory = Directory::new();
    for value in [resource(3, b), resource(2, a), resource(1, a)] { directory.apply(value, 1).unwrap(); }
    assert_eq!(select(&directory, &[], &[], 1024, 2).unwrap().id, ResourceId([1; 16]));
    assert_eq!(select(&directory, &[a], &[], 1024, 2).unwrap().owner, b);
    assert_eq!(select(&directory, &[a, b], &[], 1024, 2), None);
    directory.reserve(ResourceId([1; 16]), 1, 2048, 2).unwrap();
    assert_eq!(directory.reserve(ResourceId([1; 16]), 1, 1, 2), Err(ResourceError::Capacity));
    assert_eq!(select(&directory, &[], &[], 1, 2).unwrap().id, ResourceId([2; 16]));
    let mut updated = resource(1, a); updated.generation = 2;
    assert_eq!(directory.apply(updated, 3), Err(ResourceError::Conflict));
    directory.release(updated.id, 1, 2048).unwrap();
    directory.apply(updated, 3).unwrap();
    assert_eq!(directory.apply(resource(1, a), 4), Err(ResourceError::Stale));
    updated.available = 2000;
    assert_eq!(directory.apply(updated, 4), Err(ResourceError::Conflict));
    assert_eq!(select(&directory, &[], &[], 1, 100), None);
    assert_eq!(availability(StorageClass::Protected, 1), Availability::Degraded);
    assert_eq!(availability(StorageClass::Protected, 2), Availability::Healthy);
    assert_eq!(availability(StorageClass::Critical, 2), Availability::Degraded);
    assert_eq!(availability(StorageClass::Temporary, 0), Availability::Offline);
}

// ------------------------=
// FUNC: bounded_inventory
// DESC: Rejects floods, forged capacity, unhealthy and non-storage resources without discarding stable identity.
// ------------------=
#[test]
fn bounded_inventory() {
    let mut directory = Directory::new(); let owner = NodeId([1; 32]);
    for id in 1..=MAX_RESOURCES { directory.apply(resource(id as u8, owner), 1).unwrap(); }
    assert_eq!(directory.apply(resource(99, owner), 1), Err(ResourceError::Full));
    let mut invalid = resource(1, owner); invalid.available = invalid.capacity + 1;
    assert_eq!(directory.apply(invalid, 1), Err(ResourceError::Invalid));
    for kind in [ResourceKind::Compute, ResourceKind::Memory, ResourceKind::Accelerator] {
        let mut isolated = Directory::new(); let mut value = resource(1, owner); value.kind = kind;
        isolated.apply(value, 1).unwrap(); assert_eq!(select(&isolated, &[], &[], 1, 2), None);
    }
    assert_eq!(directory.entries().iter().flatten().count(), MAX_RESOURCES);
}

struct MemoryReplica { bytes: Vec<u8>, checkpoint: Option<Checkpoint>, published: bool, fail_commit: bool, online: bool, generation: u64 }
impl ReplicaStore for MemoryReplica {
    // ------------------------=
    // FUNC: write_staging
    // DESC: Writes host-fixture staging bytes while enforcing the same generation and availability boundary as a native adapter.
    // ------------------=
    fn write_staging(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &[u8]) -> Result<(), ReplicaError> {
        if !self.online { return Err(ReplicaError::Storage); }
        if descriptor.generation != self.generation { return Err(ReplicaError::Stale); }
        self.bytes[offset as usize..offset as usize + bytes.len()].copy_from_slice(bytes); Ok(())
    }
    // ------------------------=
    // FUNC: read_staging
    // DESC: Reads fixture storage, including deliberately injected corruption and node loss.
    // ------------------=
    fn read_staging(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &mut [u8]) -> Result<(), ReplicaError> {
        if !self.online { return Err(ReplicaError::Storage); }
        if descriptor.generation != self.generation { return Err(ReplicaError::Stale); }
        bytes.copy_from_slice(&self.bytes[offset as usize..offset as usize + bytes.len()]); Ok(())
    }
    // ------------------------=
    // FUNC: checkpoint
    // DESC: Simulates an atomic durable-root commit with an injectable failure before publication.
    // ------------------=
    fn checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), ReplicaError> {
        if self.fail_commit || !self.online { return Err(ReplicaError::Storage); }
        self.checkpoint = Some(*checkpoint); Ok(())
    }
    // ------------------------=
    // FUNC: publish_verified
    // DESC: Makes availability visible only after the fixture's successful atomic durable checkpoint.
    // ------------------=
    fn publish_verified(&mut self, checkpoint: &Checkpoint) -> Result<(), ReplicaError> {
        self.checkpoint(checkpoint)?; self.published = true; Ok(())
    }
}

// ------------------------=
// FUNC: transfer_fixture
// DESC: Provides explicitly synthetic storage with a real SHA-256 immutable-content descriptor.
// ------------------=
fn transfer_fixture() -> (MemoryReplica, ReplicaDescriptor, Vec<u8>) {
    let bytes: Vec<u8> = (0..2500).map(|n| (n % 251) as u8).collect();
    let descriptor = ReplicaDescriptor { job: 1, object: [1; 16], version: 3,
        resource: ResourceId([2; 16]), generation: 7, bytes: bytes.len() as u64,
        hash: Sha256::digest(&bytes).into() };
    (MemoryReplica { bytes: vec![0; bytes.len()], checkpoint: None, published: false,
        fail_commit: false, online: true, generation: 7 }, descriptor, bytes)
}

// ------------------------=
// FUNC: durable_replica_resume_and_verified_publication
// DESC: Tests chunks, duplicate/conflicting retries, gaps, commit failure, offline recovery and full verification after restart.
// ------------------=
#[test]
fn durable_replica_resume_and_verified_publication() {
    let (mut store, descriptor, bytes) = transfer_fixture();
    let mut transfer = Transfer::begin(&mut store, descriptor).unwrap();
    assert_eq!(transfer.verify_tick(&mut store), Err(ReplicaError::Incomplete));
    assert_eq!(transfer.receive(&mut store, 2, &bytes[..1024]), Err(ReplicaError::Incomplete));
    transfer.receive(&mut store, 0, &bytes[..1024]).unwrap();
    transfer.receive(&mut store, 0, &bytes[..1024]).unwrap();
    assert_eq!(transfer.receive(&mut store, 0, &[99; 1024]), Err(ReplicaError::Conflict));
    store.fail_commit = true;
    assert_eq!(transfer.receive(&mut store, 1024, &bytes[1024..2048]), Err(ReplicaError::Storage));
    assert_eq!(transfer.inspect().copied, 1024);
    store.fail_commit = false;
    let mut resumed = Transfer::resume(store.checkpoint.unwrap()).unwrap();
    resumed.receive(&mut store, 1024, &bytes[1024..2048]).unwrap();
    store.online = false;
    assert_eq!(resumed.receive(&mut store, 2048, &bytes[2048..]), Err(ReplicaError::Storage));
    store.online = true;
    resumed.receive(&mut store, 2048, &bytes[2048..]).unwrap();
    assert!(!store.published);
    assert_eq!(resumed.verify_tick(&mut store), Ok(ReplicaState::Verifying));
    let mut rebooted = Transfer::resume(store.checkpoint.unwrap()).unwrap();
    assert_eq!(rebooted.verify_tick(&mut store), Ok(ReplicaState::Verifying));
    assert_eq!(rebooted.verify_tick(&mut store), Ok(ReplicaState::Verifying));
    store.fail_commit = true;
    assert_eq!(rebooted.verify_tick(&mut store), Err(ReplicaError::Storage));
    assert!(!store.published);
    store.fail_commit = false;
    assert_eq!(rebooted.verify_tick(&mut store), Ok(ReplicaState::Available));
    assert_eq!(rebooted.verify_tick(&mut store), Ok(ReplicaState::Available));
    assert!(store.published); assert_eq!(store.bytes, bytes);
    assert_eq!(rebooted.inspect().descriptor.object, descriptor.object);
}

// ------------------------=
// FUNC: corrupt_and_replaced_replicas_never_publish
// DESC: Rejects corrupted content and replacement device generations rather than trusting successful transport.
// ------------------=
#[test]
fn corrupt_and_replaced_replicas_never_publish() {
    let (mut store, descriptor, bytes) = transfer_fixture();
    let mut transfer = Transfer::begin(&mut store, descriptor).unwrap();
    for (index, chunk) in bytes.chunks(TRANSFER_CHUNK).enumerate() { transfer.receive(&mut store, (index * TRANSFER_CHUNK) as u64, chunk).unwrap(); }
    store.bytes[1500] ^= 1;
    assert_eq!(transfer.verify_tick(&mut store), Ok(ReplicaState::Verifying));
    assert_eq!(transfer.verify_tick(&mut store), Ok(ReplicaState::Verifying));
    assert_eq!(transfer.verify_tick(&mut store), Err(ReplicaError::Integrity));
    assert!(!store.published); assert_eq!(transfer.inspect().state, ReplicaState::Failed);
    let (mut store, descriptor, bytes) = transfer_fixture();
    let mut transfer = Transfer::begin(&mut store, descriptor).unwrap(); store.generation += 1;
    assert_eq!(transfer.receive(&mut store, 0, &bytes[..1024]), Err(ReplicaError::Stale));
    assert_eq!(transfer.inspect().copied, 0); assert!(!store.published);
}
