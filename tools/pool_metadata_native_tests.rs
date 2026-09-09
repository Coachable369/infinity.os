use super::*;
use crate::runtime::{
    crypto::{KeyRef, NodeCrypto},
    fabric::metadata::*,
};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
// ------------------------=
// FUNC: committed_receipt_requires_exact_durable_head
// DESC: Checks read-only publication receipts reject staged, stale and unauthorized heads and survive cold mount.
// ------------------=
#[test]
fn committed_receipt_requires_exact_durable_head() {
    use crate::fabric_pool_metadata_service::Service;
    use crate::runtime::{iop::{remote::AuthenticatedStorageRequest, storage_protocol::{Operation, StorageOperationV1}}, storage_metadata::{NativeReply as R, NativeRequest as N}};
    let (g, keys) = group();
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let b = actual_bundle(&mut store, &g, &keys);
    stage_bundle(&mut store, b).unwrap();
    let mut request = AuthenticatedStorageRequest {
        local:g.members[1], peer:g.owner, session_reference:[1;16], grant:1,
        request_id:1, correlation:1, causation:1,
        payload:StorageOperationV1 { operation:Operation::PoolMetadata, object:b.manifest.object,
            authority_generation:1, manifest_generation:b.value.record.generation, object_version:1,
            offset:0, scope:0, value:7, length:32, data:[0;64] },
    };
    request.payload.data[..32].copy_from_slice(&b.value.record.digest());
    let mut service = Service::new();
    assert!(service.execute(&mut store, N::Wire { request, now:1 }).is_err());
    publish_bundle(&mut store, b.manifest.object, certificate(b.value, &keys)).unwrap();
    for cold in [false, true] {
        if cold { store = ObjectStore::mount(disk.clone(), 0).unwrap(); service = Service::new(); }
        let writes = disk.0.borrow().writes;
        let generation = store.generation();
        for _ in 0..2 {
            match service.execute(&mut store, N::Wire { request, now:1 }).unwrap() {
                R::Wire(reply) => { assert_eq!(reply.length,32); assert_eq!(reply.offset,1); assert_eq!(&reply.data[..32], &b.value.record.digest()); }
                _ => panic!("unexpected typed reply"),
            }
        }
        for case in 0..5 {
            let mut invalid = request;
            match case {
                0 => invalid.payload.data[0] ^= 1,
                1 => invalid.payload.manifest_generation += 1,
                2 => invalid.peer = NodeId([99;32]),
                3 => invalid.grant = 0,
                _ => invalid.payload.length = 31,
            }
            assert!(service.execute(&mut store, N::Wire { request:invalid, now:1 }).is_err());
        }
        assert_eq!(disk.0.borrow().writes,writes);
        assert_eq!(store.generation(),generation);
    }
}
#[path="pool_metadata_mutation_tests.rs"]
mod mutation_tests;
#[derive(Clone, Default)]
struct Disk(Rc<RefCell<Media>>);
#[derive(Clone, Default)]
struct Media {
    sectors: BTreeMap<u64, [u8; 512]>,
    writes: usize,
    reads: usize,
    cut: Option<usize>,
}
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Defines bounded sparse native test media.
    // ------------------=
    fn block_count(&self) -> u64 {
        400000
    }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads actual persisted fixture sectors.
    // ------------------=
    fn read_sector(&mut self, l: u64, o: &mut [u8; 512]) -> bool {
        let mut m = self.0.borrow_mut();
        m.reads += 1;
        *o = *m.sectors.get(&l).unwrap_or(&[0; 512]);
        true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Injects exact sector failures without mutating rejected sectors.
    // ------------------=
    fn write_sector(&mut self, l: u64, b: &[u8; 512]) -> bool {
        let mut m = self.0.borrow_mut();
        if m.cut == Some(0) {
            return false;
        }
        if let Some(n) = m.cut.as_mut() {
            *n -= 1;
        }
        m.writes += 1;
        m.sectors.insert(l, *b);
        true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Models stable writes for native commit ordering.
    // ------------------=
    fn flush(&mut self) -> bool {
        true
    }
}
// ------------------------=
// FUNC: group
// DESC: Creates explicit real signing identities without inferring membership from discovery.
// ------------------=
fn group() -> (Group, [(NodeCrypto, KeyRef); 3]) {
    let keys = core::array::from_fn(|i| {
        let mut c = NodeCrypto::new();
        let k = c.initialize(&[i as u8 + 1; 32], true).unwrap();
        (c, k)
    });
    let public = core::array::from_fn(|i| keys[i].0.public_identity().unwrap());
    let members = public.map(|key| {
        let mut h = Sha256::new();
        h.update(b"InfinityOS NodeId v1");
        h.update(key);
        NodeId(h.finalize().into())
    });
    (
        Group {
            epoch: 1,
            owner: members[0],
            members,
            keys: public,
        },
        keys,
    )
}
// ------------------------=
// FUNC: signed
// DESC: Signs an immutable metadata descriptor for one object.
// ------------------=
fn signed(g: &Group, keys: &[(NodeCrypto, KeyRef); 3], object: u8) -> SignedRecord {
    let r = Record {
        group: g.digest(),
        object: [object; 16],
        generation: 1,
        version: 1,
        previous: [0; 32],
        manifest: [3; 32],
        namespace: [4; 32],
        policy: [5; 32],
        revocation: 1,
        deleted: false,
    };
    SignedRecord {
        record: r,
        signature: keys[0].0.sign(keys[0].1, &r.encode()).unwrap(),
    }
}
// ------------------------=
// FUNC: certificate
// DESC: Produces two distinct cryptographically valid durable-staging receipts.
// ------------------=
fn certificate(value: SignedRecord, keys: &[(NodeCrypto, KeyRef); 3]) -> Certificate {
    Certificate {
        value,
        prepared: core::array::from_fn(|i| {
            let mut p = Receipt {
                member: i as u8,
                digest: value.record.digest(),
                published: false,
                signature: [0; 64],
            };
            p.signature = keys[i].0.sign(keys[i].1, &p.transcript()).unwrap();
            p
        }),
    }
}
// ------------------------=
// FUNC: persistence_callback_roundtrip_and_capacity
// DESC: Verifies native staging/publication recovery, read-only load, eight-object bounds and replay idempotency using actual signatures.
// ------------------=
#[test]
fn persistence_callback_roundtrip_and_capacity() {
    let (g, keys) = group();
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let writes = disk.0.borrow().writes;
    assert!(load(&mut store, &g, [1; 16]).unwrap().staged.is_none());
    assert_eq!(disk.0.borrow().writes, writes);
    for n in 1..=8 {
        let value = signed(&g, &keys, n);
        let mut r = load(&mut store, &g, [n; 16]).unwrap();
        let old = r;
        r.prepare(&g, value, |next| {
            persist(&mut store, &g, [n; 16], old, next)
        })
        .unwrap();
        let generation = store.generation();
        persist(&mut store, &g, [n; 16], old, r).unwrap();
        assert_eq!(store.generation(), generation);
        let old = r;
        r.publish(&g, certificate(value, &keys), |next| {
            persist(&mut store, &g, [n; 16], old, next)
        })
        .unwrap();
    }
    let ninth = Replica {
        staged: Some(signed(&g, &keys, 9)),
        committed: None,
    };
    let generation = store.generation();
    assert_eq!(
        persist(&mut store, &g, [9; 16], Replica::default(), ninth),
        Err(Error::ResourceLimit)
    );
    assert_eq!(store.generation(), generation);
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    for n in 1..=8 {
        let r = load(&mut store, &g, [n; 16]).unwrap();
        assert!(r.staged.is_none());
        assert_eq!(
            r.committed.unwrap(),
            certificate(signed(&g, &keys, n), &keys)
        );
    }
}
// ------------------------=
// FUNC: every_sector_cut_keeps_old_or_new_canonical_replica
// DESC: Cuts initial catalog creation and committed publication; no acknowledgement state advances unless the complete native transaction survives reboot.
// ------------------=
#[test]
fn every_sector_cut_keeps_old_or_new_canonical_replica() {
    let (g, keys) = group();
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let value = signed(&g, &keys, 1);
    let stage = Replica {
        staged: Some(value),
        committed: None,
    };
    let published = Replica {
        staged: None,
        committed: Some(certificate(value, &keys)),
    };
    for (old, next) in [(Replica::default(), stage), (stage, published)] {
        let baseline = disk.0.borrow().clone();
        let before = baseline.writes;
        persist(&mut store, &g, [1; 16], old, next).unwrap();
        let cost = disk.0.borrow().writes - before;
        for cut in 0..=cost {
            let d = Disk(Rc::new(RefCell::new(baseline.clone())));
            let mut s = ObjectStore::mount(d.clone(), 0).unwrap();
            d.0.borrow_mut().cut = Some(cut);
            let result = persist(&mut s, &g, [1; 16], old, next);
            assert_eq!(result.is_ok(), cut == cost);
            d.0.borrow_mut().cut = None;
            drop(s);
            let mut s = ObjectStore::mount(d, 0).unwrap();
            assert_eq!(
                replica_bytes(load(&mut s, &g, [1; 16]).unwrap()),
                replica_bytes(if cut == cost { next } else { old })
            );
        }
    }
}
// ------------------------=
// FUNC: malformed_quorum_and_stale_callback_fail_closed
// DESC: Rejects forged signatures, duplicate receipt members, mixed identities, stale CAS and noncanonical persisted fields rather than resetting durable metadata.
// ------------------=
#[test]
fn malformed_quorum_and_stale_callback_fail_closed() {
    let (g, keys) = group();
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let value = signed(&g, &keys, 1);
    let stage = Replica {
        staged: Some(value),
        committed: None,
    };
    persist(&mut store, &g, [1; 16], Replica::default(), stage).unwrap();
    let mut cert = certificate(value, &keys);
    cert.prepared[1] = cert.prepared[0];
    assert_eq!(
        persist(
            &mut store,
            &g,
            [1; 16],
            stage,
            Replica {
                staged: None,
                committed: Some(cert)
            }
        ),
        Err(Error::Quorum)
    );
    cert = certificate(value, &keys);
    cert.value.signature[0] ^= 1;
    assert_eq!(
        persist(
            &mut store,
            &g,
            [1; 16],
            stage,
            Replica {
                staged: None,
                committed: Some(cert)
            }
        ),
        Err(Error::Signature)
    );
    let published = Replica {
        staged: None,
        committed: Some(certificate(value, &keys)),
    };
    assert_eq!(
        persist(&mut store, &g, [1; 16], Replica::default(), published),
        Err(Error::Stale)
    );
    assert_eq!(
        persist(&mut store, &g, [2; 16], stage, published),
        Err(Error::Denied)
    );
    let mut other = g;
    other.epoch += 1;
    assert!(matches!(
        load(&mut store, &other, [1; 16]),
        Err(Error::Denied)
    ));
    let (mut bytes, id) = read_catalog(&mut store).unwrap();
    bytes[32 + 240 + 2] = 1;
    store.replace_state(id.unwrap(), &bytes).unwrap();
    assert!(matches!(load(&mut store, &g, [1; 16]), Err(Error::Invalid)));
}

// ------------------------=
// FUNC: actual_bundle
// DESC: Builds real native content and owner-signed canonical namespace, manifest, policy and explicit read grants.
// ------------------=
fn actual_bundle(
    store: &mut ObjectStore<Disk>,
    g: &Group,
    keys: &[(NodeCrypto, KeyRef); 3],
) -> crate::runtime::fabric::metadata_bundle::Bundle {
    use crate::runtime::fabric::{
        metadata_bundle::*, placement::StorageClass, resources::ResourceId,
    };
    store.initialize_pool_catalog().unwrap();
    let manifest = store
        .pool_create(
            g.owner,
            0,
            88,
            StorageClass::Temporary,
            b"immutable actual payload",
            g.owner,
            ResourceId([8; 16]),
            [9; 16],
            1,
        )
        .unwrap();
    let mut manifest_bytes = [0; 5376];
    manifest.encode(&mut manifest_bytes).unwrap();
    let record = Record {
        group: g.digest(),
        object: manifest.object,
        generation: 1,
        version: manifest.version,
        previous: [0; 32],
        manifest: Sha256::digest(manifest_bytes).into(),
        namespace: namespace_digest(manifest.object, b"/personal/shared").unwrap(),
        policy: policy_digest(g, &manifest, 1),
        revocation: 1,
        deleted: false,
    };
    let value = SignedRecord {
        record,
        signature: keys[0].0.sign(keys[0].1, &record.encode()).unwrap(),
    };
    let mut path = [0; 95];
    path[..16].copy_from_slice(b"/personal/shared");
    let grants = core::array::from_fn(|i| {
        let mut grant = ReaderGrant {
            group: g.digest(),
            object: manifest.object,
            reader: g.members[i],
            principal: [7; 16],
            policy: record.policy,
            revocation: 1,
            expires: 100,
            signature: [0; 64],
        };
        grant.signature = keys[0].0.sign(keys[0].1, &grant.transcript()).unwrap();
        Some(grant)
    });
    Bundle {
        group: *g,
        value,
        certificate: None,
        manifest,
        path,
        path_len: 16,
        grants,
        repair_grants: [None; 2],
    }
}
// ------------------------=
// FUNC: actual_payload_atomic_recovery_and_delegation
// DESC: Cuts every write during first stage and publish; recovered certificates always identify the complete original payload and explicit grant fences remain enforced.
// ------------------=
#[test]
fn actual_payload_atomic_recovery_and_delegation() {
    let (g, keys) = group();
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let bundle = actual_bundle(&mut store, &g, &keys);
    let mut published = bundle;
    published.certificate = Some(certificate(bundle.value, &keys));
    let encoded = published.encode().unwrap();
    assert!(published.authorize(g.members[1], [7; 16], 99).is_ok());
    assert!(published.authorize(g.members[1], [7; 16], 100).is_err());
    assert!(published.authorize(g.members[1], [8; 16], 1).is_err());
    let mut corrupt = encoded;
    corrupt[900] ^= 1;
    assert!(crate::runtime::fabric::metadata_bundle::Bundle::decode(&corrupt).is_err());
    for publish in [false, true] {
        let baseline = disk.0.borrow().clone();
        let before = baseline.writes;
        if publish {
            publish_bundle(
                &mut store,
                bundle.manifest.object,
                published.certificate.unwrap(),
            )
            .unwrap()
        } else {
            stage_bundle(&mut store, bundle).unwrap()
        }
        let cost = disk.0.borrow().writes - before;
        for cut in 0..=cost {
            let d = Disk(Rc::new(RefCell::new(baseline.clone())));
            let mut s = ObjectStore::mount(d.clone(), 0).unwrap();
            d.0.borrow_mut().cut = Some(cut);
            let result = if publish {
                publish_bundle(
                    &mut s,
                    bundle.manifest.object,
                    published.certificate.unwrap(),
                )
            } else {
                stage_bundle(&mut s, bundle)
            };
            assert_eq!(result.is_ok(), cut == cost);
            d.0.borrow_mut().cut = None;
            drop(s);
            let mut s = ObjectStore::mount(d, 0).unwrap();
            let recovered = load_bundle(&mut s, 0).unwrap();
            if publish && cut == cost {
                assert_eq!(recovered.unwrap().encode().unwrap(), encoded)
            } else {
                assert!(recovered.is_none())
            }
        }
    }
    let generation = store.generation();
    publish_bundle(
        &mut store,
        bundle.manifest.object,
        published.certificate.unwrap(),
    )
    .unwrap();
    assert_eq!(store.generation(), generation);
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    assert_eq!(
        read_bundle(&mut store, bundle.manifest.object)
            .unwrap()
            .encode()
            .unwrap(),
        encoded
    );
}

// ------------------------=
// FUNC: metadata_native_windows_and_expiring_reader
// DESC: Exercises authenticated bounded payload transfer, cross-session denial, durable publication and actual content reads with expiry after a cache hit.
// ------------------=
#[test]
fn metadata_native_windows_and_expiring_reader() {
    use crate::fabric_pool_metadata_service::Service;
    use crate::runtime::{
        iop::{
            remote::AuthenticatedStorageRequest,
            storage_protocol::{Operation, StorageOperationV1},
        },
        storage_metadata::{NativeReply as R, NativeRequest as N},
    };
    let (g, keys) = group();
    let disk = Disk::default();
    let mut owner = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let mut b = actual_bundle(&mut owner, &g, &keys);
    let recipient = Disk::default();
    let mut target =
        ObjectStore::format(recipient.clone(), 0, recipient.block_count(), [8; 16]).unwrap();
    let mut service = Service::new();
    for publish in [false, true] {
        if publish {
            b.certificate = Some(certificate(b.value, &keys));
        }
        let bytes = b.encode().unwrap();
        let mut p = StorageOperationV1 {
            operation: Operation::PoolMetadata,
            object: b.manifest.object,
            authority_generation: 1,
            manifest_generation: 1,
            object_version: 1,
            length: 32,
            offset: bytes.len() as u64,
            data: [0; 64],
            scope: 0,
            value: 0,
        };
        p.data[..32].copy_from_slice(&Sha256::digest(bytes));
        let mut r = AuthenticatedStorageRequest {
            local: g.members[1],
            peer: g.owner,
            session_reference: [1; 16],
            grant: 1,
            request_id: 1,
            correlation: 1,
            causation: 1,
            payload: p,
        };
        assert!(matches!(
            service.execute(&mut target, N::Wire { request: r, now: 1 }),
            Ok(R::Wire(_))
        ));
        for (i, chunk) in bytes.chunks(64).enumerate() {
            r.payload.value = 1;
            r.payload.offset = (i * 64) as u64;
            r.payload.length = chunk.len() as u16;
            r.payload.data.fill(0);
            r.payload.data[..chunk.len()].copy_from_slice(chunk);
            if i == 0 {
                let mut wrong = r;
                wrong.session_reference = [2; 16];
                assert!(service
                    .execute(
                        &mut target,
                        N::Wire {
                            request: wrong,
                            now: 1
                        }
                    )
                    .is_err());
            }
            assert!(service
                .execute(&mut target, N::Wire { request: r, now: 1 })
                .is_ok());
        }
        r.payload.value = if publish { 3 } else { 2 };
        r.payload.length = 0;
        match service
            .execute(&mut target, N::Wire { request: r, now: 1 })
            .unwrap()
        {
            R::Wire(p) => {
                assert_eq!(p.length, 32);
                assert_eq!(p.offset, 1);
                assert_eq!(&p.data[..32], &b.value.record.digest())
            }
            _ => panic!(),
        }
    }
    drop(target);
    let mut target = ObjectStore::mount(recipient.clone(), 0).unwrap();
    assert_eq!(
        read_bundle(&mut target, b.manifest.object)
            .unwrap()
            .encode()
            .unwrap(),
        b.encode().unwrap()
    );
    let mut request = AuthenticatedStorageRequest {
        local: g.members[1],
        peer: g.owner,
        session_reference: [1; 16],
        grant: 1,
        request_id: 2,
        correlation: 2,
        causation: 2,
        payload: StorageOperationV1 {
            operation: Operation::PoolMetadata,
            object: b.manifest.object,
            authority_generation: 1,
            manifest_generation: 1,
            object_version: 1,
            offset: 0,
            scope: 0,
            value: 4,
            length: 0,
            data: [0; 64],
        },
    };
    service
        .execute(&mut target, N::Wire { request, now: 1 })
        .unwrap();
    let reads = recipient.0.borrow().reads;
    let expected = b.encode().unwrap();
    let mut result = vec![];
    for at in (0..expected.len()).step_by(64) {
        request.payload.value = 5;
        request.payload.offset = at as u64;
        match service
            .execute(&mut target, N::Wire { request, now: 1 })
            .unwrap()
        {
            R::Wire(p) => result.extend_from_slice(&p.data[..p.length as usize]),
            _ => panic!(),
        }
    }
    assert_eq!(result, expected);
    assert_eq!(recipient.0.borrow().reads, reads);
    let mut staged = b;
    staged.certificate = None;
    stage_bundle(&mut owner, staged).unwrap();
    publish_bundle(&mut owner, b.manifest.object, b.certificate.unwrap()).unwrap();
    let mut service = Service::new();
    for now in [1, 2] {
        match service
            .execute(
                &mut owner,
                N::Read {
                    object: b.manifest.object,
                    record: b.value.record,
                    principal: [7; 16],
                    reader: g.owner,
                    now,
                    offset: 0,
                    length: 9,
                    overlay: None,
                },
            )
            .unwrap()
        {
            R::Bytes { data, length } => {
                assert_eq!(length, 9);
                assert_eq!(&data[..9], b"immutable")
            }
            _ => panic!(),
        }
    }
    assert!(service
        .execute(
            &mut owner,
            N::Read {
                object: b.manifest.object,
                record: b.value.record,
                principal: [7; 16],
                reader: g.owner,
                now: 100,
                offset: 0,
                length: 9,
                overlay: None
            }
        )
        .is_err());
}

// ------------------------=
// FUNC: repair_authorization
// DESC: Constructs explicit owner delegation, writer proposal and independently signed destination persistence evidence.
// ------------------=
fn repair_authorization(
    base: crate::runtime::fabric::metadata_bundle::Bundle,
    keys: &[(NodeCrypto, KeyRef); 3],
) -> crate::runtime::fabric::metadata_repair::RepairAuthorization {
    use crate::runtime::fabric::{
        manifest::{Placement, PlacementState},
        metadata_repair::*,
        resources::ResourceId,
    };
    let mut d = NodeCrypto::new();
    let key = d.initialize(&[17; 32], true).unwrap();
    let public = d.public_identity().unwrap();
    let mut h = Sha256::new();
    h.update(b"InfinityOS NodeId v1");
    h.update(public);
    let destination = NodeId(h.finalize().into());
    let mut grant = RepairGrant {
        group: base.group.digest(),
        anchor: base.value.record.digest(),
        writer: base.group.members[1],
        destinations: [
            destination,
            NodeId([0; 32]),
            NodeId([0; 32]),
            NodeId([0; 32]),
        ],
        expires: 100,
        signature: [0; 64],
    };
    grant.signature = keys[0].0.sign(keys[0].1, &grant.transcript()).unwrap();
    let mut manifest = base.manifest;
    manifest.generation += 1;
    manifest.placements[1] = Some(Placement {
        node: destination,
        resource: ResourceId([21; 16]),
        device: [22; 16],
        generation: 1,
        version: manifest.version,
        hash: manifest.hash,
        state: PlacementState::Verified,
        admission_generation: 2,
    });
    let mut bytes = [0; 5376];
    manifest.encode(&mut bytes).unwrap();
    let mut value = RepairRecord {
        anchor: grant.anchor,
        grant: grant.digest(),
        sequence: 1,
        previous: [0; 32],
        manifest: Sha256::digest(bytes).into(),
        signature: [0; 64],
    };
    value.signature = keys[1].0.sign(keys[1].1, &value.transcript()).unwrap();
    let mut availability = RepairAvailability {
        repair: value.digest(),
        node: destination,
        resource: [21; 16],
        device: [22; 16],
        public_key: public,
        signature: [0; 64],
    };
    availability.signature = d.sign(key, &availability.transcript()).unwrap();
    RepairAuthorization {
        anchor: base,
        repair: RepairBundle {
            grant,
            value,
            certificate: None,
            manifest,
            availability: Some(availability),
        },
    }
}
// ------------------------=
// FUNC: repair_native_atomic_head_and_recipient_signature
// DESC: Verifies separate overlay stage/publish durability, every-sector old/new recovery, destination-signature rejection and idempotent committed heads.
// ------------------=
#[test]
fn repair_native_atomic_head_and_recipient_signature() {
    use crate::runtime::{
        fabric::{metadata_repair::RepairCertificate, resources::ResourceId},
        storage_metadata_repair::{NativeReply as R, NativeRequest as N},
    };
    let (g, keys) = group();
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let mut base = actual_bundle(&mut store, &g, &keys);
    stage_bundle(&mut store, base).unwrap();
    base.certificate = Some(certificate(base.value, &keys));
    publish_bundle(&mut store, base.manifest.object, base.certificate.unwrap()).unwrap();
    let stage = repair_authorization(base, &keys);
    let mut published = stage;
    published.repair.certificate = Some(RepairCertificate {
        value: stage.repair.value,
        prepared: core::array::from_fn(|i| {
            let mut r = Receipt {
                member: i as u8,
                digest: stage.repair.value.digest(),
                published: false,
                signature: [0; 64],
            };
            r.signature = keys[i].0.sign(keys[i].1, &r.transcript()).unwrap();
            r
        }),
    });
    let mut replica =
        crate::native_fabric::service::ReplicaService::mount(&mut store, ResourceId([8; 16]), 1)
            .unwrap();
    let mut service = crate::fabric_pool_repair::Service::new();
    let mut forged = stage;
    forged.repair.availability.as_mut().unwrap().signature[0] ^= 1;
    assert!(service
        .execute(
            &mut store,
            &mut replica,
            N::Stage {
                authorization: forged,
                now: 1
            }
        )
        .is_err());
    for publish in [false, true] {
        let baseline = disk.0.borrow().clone();
        let before = baseline.writes;
        let run = |s: &mut ObjectStore<Disk>,
                   rep: &mut crate::native_fabric::service::ReplicaService,
                   svc: &mut crate::fabric_pool_repair::Service| {
            svc.execute(
                s,
                rep,
                if publish {
                    N::Publish {
                        authorization: published,
                        now: 1,
                    }
                } else {
                    N::Stage {
                        authorization: stage,
                        now: 1,
                    }
                },
            )
        };
        run(&mut store, &mut replica, &mut service).unwrap();
        let cost = disk.0.borrow().writes - before;
        for cut in 0..=cost {
            let d = Disk(Rc::new(RefCell::new(baseline.clone())));
            let mut s = ObjectStore::mount(d.clone(), 0).unwrap();
            let mut svc = crate::fabric_pool_repair::Service::new();
            d.0.borrow_mut().cut = Some(cut);
            let result = run(&mut s, &mut replica, &mut svc);
            assert_eq!(result.is_ok(), cut == cost);
            d.0.borrow_mut().cut = None;
            drop(s);
            let mut s = ObjectStore::mount(d, 0).unwrap();
            match svc
                .execute(
                    &mut s,
                    &mut replica,
                    N::Load {
                        object: base.manifest.object,
                        anchor: base.value.record.digest(),
                        now: 101,
                    },
                )
                .unwrap()
            {
                R::Overlay(head) => assert_eq!(head.is_some(), publish && cut == cost),
                _ => panic!(),
            }
        }
    }
    let generation = store.generation();
    service
        .execute(
            &mut store,
            &mut replica,
            N::Publish {
                authorization: published,
                now: 1,
            },
        )
        .unwrap();
    assert_eq!(store.generation(), generation);
}

// ------------------------=
// FUNC: delegated_repair_preserves_owner_and_verifies_bytes
// DESC: Exercises explicit writer-to-destination repair, lease revocation, immutable owner identity and bounded verification against actual recipient sectors.
// ------------------=
#[test]
fn delegated_repair_preserves_owner_and_verifies_bytes() {
    use crate::runtime::{
        fabric::resources::ResourceId,
        iop::{
            remote::AuthenticatedStorageRequest,
            storage_protocol::{Operation, StorageOperationV1},
        },
    };
    let (g, keys) = group();
    let disk = Disk::default();
    let mut owner = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let mut base = actual_bundle(&mut owner, &g, &keys);
    base.certificate = Some(certificate(base.value, &keys));
    let mut a = repair_authorization(base, &keys);
    a.repair.availability = None;
    let d = Disk::default();
    let mut recipient = ObjectStore::format(d.clone(), 0, d.block_count(), [8; 16]).unwrap();
    let mut service = crate::native_fabric::service::ReplicaService::mount(
        &mut recipient,
        ResourceId([21; 16]),
        1,
    )
    .unwrap();
    service.attach_device_identity(Some([22; 16]));
    let mut p = StorageOperationV1 {
        operation: Operation::PoolMetadata,
        object: base.manifest.object,
        authority_generation: 1,
        manifest_generation: 2,
        object_version: 1,
        offset: base.manifest.length,
        scope: 0,
        value: 20,
        length: 32,
        data: [0; 64],
    };
    p.data[..32].copy_from_slice(&base.manifest.hash);
    let mut r = AuthenticatedStorageRequest {
        local: a.repair.grant.destinations[0],
        peer: g.members[1],
        session_reference: [1; 16],
        grant: 7,
        request_id: 1,
        correlation: 1,
        causation: 1,
        payload: p,
    };
    let mut wrong = r;
    wrong.peer = g.owner;
    assert!(service
        .repair_transfer(&mut recipient, wrong, &a, 1)
        .is_err());
    assert!(service.repair_transfer(&mut recipient, r, &a, 100).is_err());
    assert_eq!(
        service
            .repair_transfer(&mut recipient, r, &a, 1)
            .unwrap()
            .value,
        0
    );
    r.payload.value = 21;
    r.payload.offset = 0;
    r.payload.length = 24;
    r.payload.data.fill(0);
    r.payload.data[..24].copy_from_slice(b"immutable actual payload");
    service.repair_transfer(&mut recipient, r, &a, 1).unwrap();
    r.payload.value = 22;
    r.payload.length = 0;
    r.payload.data.fill(0);
    let mut completed = false;
    for _ in 0..8 {
        if service
            .repair_transfer(&mut recipient, r, &a, 1)
            .unwrap()
            .value
            == 1
        {
            completed = true;
            break;
        }
    }
    assert!(completed);
    drop(recipient);
    let mut recipient = ObjectStore::mount(d, 0).unwrap();
    assert_eq!(
        crate::native_fabric::service::verify_persisted_replica(
            &mut recipient,
            g.owner.0,
            base.manifest.object,
            base.manifest.version,
            base.manifest.hash
        )
        .unwrap(),
        24
    );
    assert!(crate::native_fabric::service::verify_persisted_replica(
        &mut recipient,
        g.members[1].0,
        base.manifest.object,
        base.manifest.version,
        base.manifest.hash
    )
    .is_err());
}

// ------------------------=
// FUNC: payload_header_migration_preserves_certified_bytes
// DESC: Reads the previous bounded header without writing, then atomically links an intent under the upgraded header with identical certified payloads.
// ------------------=
#[test]
fn payload_header_migration_preserves_certified_bytes() {
    let (g, keys) = group();
    let d = Disk::default();
    let mut store = ObjectStore::format(d.clone(), 0, d.block_count(), [7; 16]).unwrap();
    let mut b = actual_bundle(&mut store, &g, &keys);
    stage_bundle(&mut store, b).unwrap();
    b.certificate = Some(certificate(b.value, &keys));
    publish_bundle(&mut store, b.manifest.object, b.certificate.unwrap()).unwrap();
    let suffix: String = b
        .manifest
        .object
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect();
    let path = format!("/system/storage/pool-quorum-payload/{suffix}");
    let id = store.resolve(path.as_bytes()).unwrap();
    let mut current = [0; 15424];
    assert_eq!(store.read(id, None, &mut current).unwrap(), 15424);
    let mut legacy = vec![0; 15392];
    legacy[..10].copy_from_slice(&current[..10]);
    legacy[..8].copy_from_slice(b"INFQPY01");
    legacy[32..].copy_from_slice(&current[64..]);
    store.replace_state(id, &legacy).unwrap();
    let writes = d.0.borrow().writes;
    assert_eq!(
        read_bundle(&mut store, b.manifest.object)
            .unwrap()
            .encode()
            .unwrap(),
        b.encode().unwrap()
    );
    assert_eq!(d.0.borrow().writes, writes);
    write_mutation_payload(&mut store, b.manifest.object, b"durable intent").unwrap();
    drop(store);
    let mut store = ObjectStore::mount(d, 0).unwrap();
    let mut intent = [0; 32];
    assert_eq!(
        read_mutation_payload(&mut store, b.manifest.object, &mut intent).unwrap(),
        14
    );
    assert_eq!(&intent[..14], b"durable intent");
    assert_eq!(
        read_bundle(&mut store, b.manifest.object)
            .unwrap()
            .encode()
            .unwrap(),
        b.encode().unwrap()
    );
}
