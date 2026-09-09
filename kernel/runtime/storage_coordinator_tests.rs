use super::*;
use core::sync::atomic::{AtomicUsize, Ordering};
static CALLS: AtomicUsize = AtomicUsize::new(0);
// ------------------------=
// FUNC: empty_native
// DESC: Supplies explicit empty persistent state and counts bounded native invocations without fabricating network success.
// ------------------=
fn empty_native(request: NativeRequest) -> Result<NativeReply, RemoteError> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    match request {
        NativeRequest::ConfigLoad => Err(RemoteError::NotFound),
        NativeRequest::Load { .. } => Ok(NativeReply::Manifest(None)),
        NativeRequest::DeletionLoad { .. } => Ok(NativeReply::Deletion(None)),
        _ => Err(RemoteError::UnsupportedOperation),
    }
}
// ------------------------=
// FUNC: canonical_participation_keeps_exact_scopes_and_expires
// DESC: Verifies persistent policy roundtrip, duplicate peer rejection, operation-specific grants, denied defaults and lease expiry.
// ------------------=
#[test]
fn canonical_participation_keeps_exact_scopes_and_expires() {
    let mut c = Configuration::empty();
    assert_eq!(
        grant(&c, NodeId([2; 32]), Operation::TransferBegin, 1),
        Err(RemoteError::AccessDenied)
    );
    c.scope = 41;
    c.peers[0] = Some(Participation {
        peer: NodeId([2; 32]),
        grants: [11, 12, 13, 14, 15],
        expires: 70,
        advertise: 0,
        advertise_expires: 0,
        delete: 0,
        delete_expires: 0,
    });
    let encoded = c.encode().unwrap();
    let decoded = Configuration::decode(&encoded).unwrap();
    assert_eq!(decoded.scope, 41);
    assert_eq!(decoded.peers, c.peers);
    for magic in [b"INFPCF01", b"INFPCF02"] {
        let mut legacy = [0; CONFIG_BYTES];
        legacy[..32].copy_from_slice(&encoded[..32]);
        legacy[..8].copy_from_slice(magic);
        for i in 0..4 {
            legacy[32 + i * 112..32 + (i + 1) * 112]
                .copy_from_slice(&encoded[32 + i * 120..32 + i * 120 + 112]);
        }
        assert_eq!(Configuration::decode(&legacy).unwrap().peers, c.peers);
    }
    for (op, value) in [
        (Operation::TransferBegin, 11),
        (Operation::TransferChunk, 12),
        (Operation::TransferCommit, 13),
        (Operation::ReplicaInspect, 14),
        (Operation::ObjectRead, 15),
    ] {
        assert_eq!(grant(&decoded, NodeId([2; 32]), op, 69), Ok(value));
        assert_eq!(
            grant(&decoded, NodeId([2; 32]), op, 70),
            Err(RemoteError::AccessDenied)
        );
    }
    assert_eq!(
        grant(&decoded, NodeId([2; 32]), Operation::ObjectCreate, 1),
        Err(RemoteError::AccessDenied)
    );
    assert_eq!(grant(&decoded,NodeId([2;32]),Operation::ReplicaDelete,1),Err(RemoteError::AccessDenied));
    let mut retirement=decoded;retirement.peers[0].as_mut().unwrap().delete=71;retirement.peers[0].as_mut().unwrap().delete_expires=80;
    let retirement=Configuration::decode(&retirement.encode().unwrap()).unwrap();
    assert_eq!(grant(&retirement,NodeId([2;32]),Operation::ReplicaDelete,79),Ok(71));
    assert_eq!(grant(&retirement,NodeId([2;32]),Operation::ReplicaDelete,80),Err(RemoteError::AccessDenied));
    assert_eq!(grant(&retirement,NodeId([2;32]),Operation::TransferCommit,69),Ok(13));
    c.peers[1] = c.peers[0];
    assert!(c.encode().is_err());
    let mut corrupt = encoded;
    corrupt[511] = 1;
    assert!(Configuration::decode(&corrupt).is_err());
    corrupt = encoded;
    corrupt[32] = 2;
    assert!(Configuration::decode(&corrupt).is_err());
}
// ------------------------=
// FUNC: native_pump_is_bounded_and_retires_authority
// DESC: Hundreds of polls each execute at most one native operation and do not exhaust capabilities; no backend yields an honest failure.
// ------------------=
#[test]
fn native_pump_is_bounded_and_retires_authority() {
    let mut r = InfinityRuntime::new(false);
    r.define_bootstrap().unwrap();
    r.start_all(0);
    r.nodes.initialize(&[91; 32], true).unwrap();
    r.node_clock = Some(10);
    r.storage_coordinator.handler = Some(empty_native);
    CALLS.store(0, Ordering::SeqCst);
    poll(&mut r, 10);
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    assert!(r.storage_coordinator.loaded);
    r.storage_coordinator.local_refresh = false;
    for now in 11..400 {
        r.storage_coordinator.local_refresh = false;
        let before = CALLS.load(Ordering::SeqCst);
        poll(&mut r, now);
        assert!(CALLS.load(Ordering::SeqCst) - before <= 1);
        assert!(r.storage_coordinator.last_error.is_none());
    }
    r.storage_coordinator.handler = None;
    poll(&mut r, 500);
    assert_eq!(
        r.storage_coordinator.last_error,
        Some(RemoteError::ServiceUnavailable)
    );
}

// ------------------------=
// FUNC: remote_chunk_bytes_are_withheld_until_final_integrity
// DESC: Exercises the production incremental verifier with partial, corrupted, oversized and exact content while extracting only the requested range.
// ------------------=
#[test]
fn remote_chunk_bytes_are_withheld_until_final_integrity() {
    use fabric::{
        manifest::{Chunk, Placement, MAX_CHUNKS, MAX_PLACEMENTS},
        placement::StorageClass,
        resources::ResourceId,
    };
    let bytes = [7u8; 128];
    let hash = Sha256::digest(bytes).into();
    let placement = Placement {
        node: NodeId([2; 32]),
        resource: ResourceId([2; 16]),
        device: [2; 16],
        generation: 1,
        version: 1,
        hash,
        state: PlacementState::Verified,
        admission_generation: 1,
    };
    let mut manifest = Manifest {
        object: [1; 16],
        version: 1,
        length: 128,
        hash,
        policy: StorageClass::Protected,
        minimum_available: 1,
        generation: 1,
        authority: NodeId([1; 32]),
        authority_generation: 1,
        chunks: [None; MAX_CHUNKS],
        placements: [None; MAX_PLACEMENTS],
        healing: None,
    };
    manifest.chunks[0] = Some(Chunk {
        content: [1; 16],
        bytes: 128,
        hash,
    });
    manifest.placements[0] = Some(placement);
    let mut job = ReadJob {
        manifest,
        placement,
        base: 0,
        size: 128,
        position: 0,
        requested: 60,
        length: 12,
        result: [0; 64],
        digest: Sha256::new(),
        expected: hash,
        pending: None,
    };
    assert_eq!(
        absorb_read(&mut job, &[7; 65]),
        Err(RemoteError::MalformedRequest)
    );
    assert_eq!(job.position, 0);
    assert_eq!(absorb_read(&mut job, &bytes[..64]), Ok(false));
    assert_eq!(job.position, 64);
    assert_eq!(absorb_read(&mut job, &bytes[64..]), Ok(true));
    assert_eq!(job.result[..12], [7; 12]);
    job.position = 0;
    job.digest = Sha256::new();
    job.result.fill(0);
    assert_eq!(absorb_read(&mut job, &bytes[..64]), Ok(false));
    assert_eq!(
        absorb_read(&mut job, &[8; 64]),
        Err(RemoteError::InvalidState)
    );
    let mut r = InfinityRuntime::new(false);
    r.define_bootstrap().unwrap();
    r.start_all(0);
    r.nodes.initialize(&[81; 32], true).unwrap();
    for n in 2..=3 {
        let peer = NodeId([n; 32]);
        r.storage_coordinator.config.peers[(n - 2) as usize] = Some(Participation {
            peer,
            grants: [1, 2, 3, 4, 5],
            expires: 100,
            advertise: 0,
            advertise_expires: 0,
            delete: 0,
            delete_expires: 0,
        });
        let resource = fabric::resources::Resource {
            id: ResourceId([n; 16]),
            owner: peer,
            kind: fabric::resources::ResourceKind::Storage,
            device: [n; 16],
            capacity: 65536,
            available: 65536,
            reserved: 0,
            health: fabric::resources::Health::Healthy,
            online: true,
            capabilities: 1,
            generation: 1,
            sequence: 1,
            expires: 100,
        };
        r.fabric_resources.observe_local(resource, peer, 1).unwrap();
    }
    manifest.placements[1] = Some(Placement {
        node: NodeId([3; 32]),
        resource: ResourceId([3; 16]),
        device: [3; 16],
        ..placement
    });
    begin_remote_read(&mut r, manifest, 60, 12, 2).unwrap();
    assert_eq!(
        r.storage_coordinator
            .reader
            .as_ref()
            .unwrap()
            .placement
            .node,
        NodeId([2; 32])
    );
    r.storage_coordinator.reader = None;
    r.storage_coordinator.read_tried = 1;
    start_remote_read(&mut r, manifest, 60, 12, 3).unwrap();
    assert_eq!(
        r.storage_coordinator
            .reader
            .as_ref()
            .unwrap()
            .placement
            .node,
        NodeId([3; 32])
    );
    assert_eq!(r.storage_coordinator.reader.as_ref().unwrap().position, 0);
    r.storage_coordinator.reader = None;
    r.storage_coordinator.read_tried = 3;
    assert_eq!(
        start_remote_read(&mut r, manifest, 60, 12, 4),
        Err(RemoteError::NotFound)
    );
}
