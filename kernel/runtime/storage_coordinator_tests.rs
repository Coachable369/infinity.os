use super::*;
use core::sync::atomic::{AtomicUsize, Ordering};
// ------------------------=
// FUNC: read_manifest_fixture
// DESC: Supplies one owned immutable manifest; any local byte read panics so remote preference cannot silently use local data.
// ------------------=
fn read_manifest_fixture(request: NativeRequest) -> Result<NativeReply, RemoteError> {
    let NativeRequest::Load { owner, .. } = request else {
        panic!("unexpected local read");
    };
    let hash = Sha256::digest([7u8; 128]).into();
    let mut m = Manifest {
        object: [1; 16],
        version: 1,
        length: 128,
        hash,
        policy: fabric::placement::StorageClass::Protected,
        minimum_available: 1,
        generation: 1,
        authority: owner,
        authority_generation: 1,
        chunks: [None; fabric::manifest::MAX_CHUNKS],
        placements: [None; fabric::manifest::MAX_PLACEMENTS],
        healing: None,
    };
    m.chunks[0] = Some(fabric::manifest::Chunk {
        content: [1; 16],
        bytes: 128,
        hash,
    });
    m.placements[0] = Some(fabric::manifest::Placement {
        node: NodeId([2; 32]),
        resource: fabric::resources::ResourceId([2; 16]),
        device: [2; 16],
        generation: 1,
        version: 1,
        hash,
        state: PlacementState::Verified,
        admission_generation: 1,
    });
    Ok(NativeReply::Manifest(Some(m)))
}
// ------------------------=
// FUNC: explicit_remote_preference_keeps_ownership_and_fences
// DESC: Verifies typed preference bounds, live caller ownership, version fences, no-source failure, explicit grants and local-read avoidance; wire integrity is covered by the incremental verifier test.
// ------------------=
#[test]
fn explicit_remote_preference_keeps_ownership_and_fences() {
    assert_eq!(read_preference(64), Ok((64, ReadSource::LocalPreferred)));
    assert_eq!(
        read_preference(REMOTE_VERIFIED | 64),
        Ok((64, ReadSource::RemoteVerified))
    );
    for value in [0, 65, REMOTE_VERIFIED, REMOTE_VERIFIED | 65, 1 << 62] {
        assert!(read_preference(value).is_err());
    }
    let mut r = InfinityRuntime::new(false);
    r.define_bootstrap().unwrap();
    r.start_all(0);
    r.nodes.initialize(&[81; 32], true).unwrap();
    r.node_clock = Some(10);
    r.identity
        .create_machine(b"read-fixture", 64, 1, 0)
        .unwrap();
    let user = r
        .identity
        .create_user(b"operator", b"Operator", 0)
        .unwrap()
        .id;
    r.identity.create_password(user, b"Fixture901", 0).unwrap();
    let session = r
        .identity
        .create_session(user, b"Fixture901", 1)
        .unwrap()
        .id;
    let other_session = r
        .identity
        .create_session(user, b"Fixture901", 2)
        .unwrap()
        .id;
    r.storage_coordinator.handler = Some(read_manifest_fixture);
    let request = StorageOperationV1 {
        operation: Operation::ObjectRead,
        object: [1; 16],
        authority_generation: 1,
        manifest_generation: 1,
        object_version: 1,
        offset: 0,
        scope: 0,
        value: REMOTE_VERIFIED | 64,
        length: 0,
        data: [0; 64],
    };
    assert_eq!(
        submit_read_from(&mut r, StableId([0; 16]), session, request),
        Err(RemoteError::AccessDenied)
    );
    r.storage_last_observation = Some(request);
    let id = submit_read_from(&mut r, user, session, request).unwrap();
    assert!(r.storage_last_observation.is_none());
    assert_eq!(r.storage_coordinator.completed_read, 0);
    assert_eq!(
        take_read_from(&mut r, user, other_session, id),
        Err(RemoteError::NotFound)
    );
    let p = r.storage_coordinator.public_read.take().unwrap();
    let p = public_read_pump(&mut r, p, 10);
    assert_eq!(p.result, Some(Err(RemoteError::NotFound)));
    r.storage_coordinator.public_read = Some(p);
    assert_eq!(
        take_read_from(&mut r, user, session, id),
        Err(RemoteError::NotFound)
    );
    assert!(r.storage_coordinator.public_read.is_none());
    assert_eq!(r.storage_coordinator.completed_read, id);
    assert_eq!(
        r.storage_coordinator.read_error,
        Some(RemoteError::NotFound)
    );
    assert!(r.storage_last_observation.is_none());
    let success_id = submit_read_from(&mut r, user, session, request).unwrap();
    let mut reply = request;
    reply.length = 3;
    reply.data[..3].copy_from_slice(&[7, 8, 9]);
    r.storage_coordinator.public_read.as_mut().unwrap().result = Some(Ok(reply));
    assert_eq!(
        take_read_from(&mut r, user, other_session, success_id),
        Err(RemoteError::NotFound)
    );
    assert_eq!(r.storage_coordinator.completed_read, 0);
    assert_eq!(
        take_read_from(&mut r, user, session, success_id),
        Ok(Some(reply))
    );
    assert_eq!(r.storage_last_observation, Some(reply));
    assert_eq!(r.storage_coordinator.completed_read, success_id);
    assert_eq!(r.storage_coordinator.read_error, None);
    let mut bad = request;
    bad.object_version = 2;
    submit_read_from(&mut r, user, session, bad).unwrap();
    let p = r.storage_coordinator.public_read.take().unwrap();
    assert_eq!(
        public_read_pump(&mut r, p, 10).result,
        Some(Err(RemoteError::Conflict))
    );
    let peer = NodeId([2; 32]);
    r.storage_coordinator.config.peers[0] = Some(Participation {
        peer,
        grants: [1, 2, 3, 4, 5],
        expires: 100,
        advertise: 0,
        advertise_expires: 0,
        delete: 0,
        delete_expires: 0,
    });
    r.fabric_resources
        .observe_local(
            fabric::resources::Resource {
                id: fabric::resources::ResourceId([2; 16]),
                owner: peer,
                kind: fabric::resources::ResourceKind::Storage,
                device: [2; 16],
                capacity: 65536,
                available: 65536,
                reserved: 0,
                health: fabric::resources::Health::Healthy,
                online: true,
                capabilities: 1,
                generation: 1,
                sequence: 1,
                expires: 200,
            },
            peer,
            1,
        )
        .unwrap();
    submit_read_from(&mut r, user, session, request).unwrap();
    let p = r.storage_coordinator.public_read.take().unwrap();
    let p = public_read_pump(&mut r, p, 10);
    assert!(matches!(p.phase, PublicReadPhase::Remote));
    assert!(p.result.is_none());
    assert_eq!(
        r.storage_coordinator
            .reader
            .as_ref()
            .unwrap()
            .placement
            .node,
        peer
    );
    r.storage_coordinator.reader = None;
    submit_read_from(&mut r, user, session, request).unwrap();
    let p = r.storage_coordinator.public_read.take().unwrap();
    assert_eq!(
        public_read_pump(&mut r, p, 100).result,
        Some(Err(RemoteError::NotFound))
    );
}
static CALLS: AtomicUsize = AtomicUsize::new(0);
// ------------------------=
// FUNC: committed_transition_delivery_recovers_after_failure
// DESC: Forces real event-authority rejection, retains committed generation and exact flags, then verifies bounded retry publishes the same structured transition.
// ------------------=
#[test]
fn committed_transition_delivery_recovers_after_failure() {
    use iop::storage_protocol::{transition::*, StorageCommit, EVENT_OBJECT_CHANGED};
    let mut r = InfinityRuntime::new(false);
    r.define_bootstrap().unwrap();
    r.start_all(0);
    let holder = r.service_identity(SERVICE_REPLICA_STORAGE).unwrap();
    let cap = r
        .capabilities
        .grant(
            CapabilityType::EventSubscribe,
            EVENT_OBJECT_CHANGED as u64,
            1,
            0,
            holder,
            holder,
            None,
            0,
        )
        .unwrap();
    let lease = r
        .events
        .subscribe(
            holder,
            cap,
            event::EventFilter {
                type_id: EVENT_OBJECT_CHANGED,
                scope: None,
            },
            100,
            event::OverflowPolicy::LatestOnly,
            1,
            &r.capabilities,
            1,
        )
        .unwrap();
    let notice = StorageCommit {
        event: EVENT_OBJECT_CHANGED,
        object: [7; 16],
        generation: 4,
        copied: 128,
        state: 1,
        correlation: 3,
        causation: 2,
        transitions: HEAL_COMPLETED | VERIFIED | AVAILABLE | HEALTHY | STALE_RECONCILED,
    };
    retain_notice(&mut r, notice);
    r.storage_event_cap[2] = Some(u64::MAX);
    flush_notice(&mut r, 1);
    assert_eq!(r.storage_coordinator.notice, Some(notice));
    assert!(r.events.receive(lease, 1).is_err());
    flush_notice(&mut r, 2);
    assert!(r.storage_coordinator.notice.is_none());
    let delivered = r.events.receive(lease, 2).unwrap();
    assert_eq!(delivered.payload_len, 40);
    assert_eq!(&delivered.payload[..40], &notice.payload());
    assert_eq!(delivered.correlation_id, 3);
    assert_eq!(delivered.causation_id, 2);
    retain_notice(&mut r,notice);
    let another=StorageCommit{object:[8;16],generation:5,transitions:CREATED,..notice};
    retain_notice(&mut r,another);
    let pending=r.storage_coordinator.notice.unwrap();
    assert_eq!(pending.object,[8;16]);
    assert_eq!(pending.transitions,CREATED|RECONSTRUCT);
    flush_notice(&mut r,3);
    let recovered=r.events.receive(lease,3).unwrap();
    assert_eq!(u32::from_le_bytes(recovered.payload[36..40].try_into().unwrap()),CREATED|RECONSTRUCT);
    let NativeReply::Manifest(Some(before))=read_manifest_fixture(NativeRequest::Load{index:0,owner:NodeId([1;32]),scope:0}).unwrap()else{panic!("fixture")};
    let mut stale=before;stale.generation+=1;stale.placements[0].as_mut().unwrap().state=PlacementState::Stale;
    assert_ne!(manifest_transitions(&before,&stale)&STALE_DETECTED,0);
    let mut reconciled=stale;reconciled.generation+=1;reconciled.placements[0].as_mut().unwrap().state=PlacementState::Verified;
    assert_eq!(manifest_transitions(&stale,&reconciled)&(VERIFIED|AVAILABLE|STALE_RECONCILED),VERIFIED|AVAILABLE|STALE_RECONCILED);
    assert_eq!(
        iop::storage_protocol::mutation_transitions(
            Operation::TransferChunk,
            &StorageOperationV1 {
                operation: Operation::TransferChunk,
                object: [1; 16],
                authority_generation: 1,
                manifest_generation: 1,
                object_version: 1,
                offset: 64,
                scope: 0,
                value: 0,
                length: 0,
                data: [0; 64]
            }
        ),
        0
    );
}
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
    assert_eq!(
        grant(&decoded, NodeId([2; 32]), Operation::ReplicaDelete, 1),
        Err(RemoteError::AccessDenied)
    );
    let mut retirement = decoded;
    retirement.peers[0].as_mut().unwrap().delete = 71;
    retirement.peers[0].as_mut().unwrap().delete_expires = 80;
    let retirement = Configuration::decode(&retirement.encode().unwrap()).unwrap();
    assert_eq!(
        grant(&retirement, NodeId([2; 32]), Operation::ReplicaDelete, 79),
        Ok(71)
    );
    assert_eq!(
        grant(&retirement, NodeId([2; 32]), Operation::ReplicaDelete, 80),
        Err(RemoteError::AccessDenied)
    );
    assert_eq!(
        grant(&retirement, NodeId([2; 32]), Operation::TransferCommit, 69),
        Ok(13)
    );
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
