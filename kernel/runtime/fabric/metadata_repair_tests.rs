use super::*;
// ------------------------=
// FUNC: fresh_repair_read_requires_distinct_observations_and_writeback
// DESC: Exercises exact R2/W2, duplicate isolation, authentic absence, recipient forgery rejection and historical certificate survival after admission lease expiry.
// ------------------=
#[test]
fn fresh_repair_read_requires_distinct_observations_and_writeback() {
    let (g, ids, anchor, grant, base, next, r) = fixture();
    let b = RepairBundle {
        grant,
        value: r,
        certificate: Some(RepairCertificate {
            value: r,
            prepared: [
                receipt(&ids, 1, r.digest(), false),
                receipt(&ids, 2, r.digest(), false),
            ],
        }),
        manifest: next,
        availability: Some(availability(r, &next)),
    };
    let mut round = RepairReadRound::new(&g, &anchor).unwrap();
    assert_eq!(round.resolved(), Err(Error::Quorum));
    round
        .observe_authenticated(&g, &anchor, &base, g.members[1], Some(b), grant.expires + 1)
        .unwrap();
    assert_eq!(round.selected(), Err(Error::Quorum));
    assert_eq!(
        round.observe_authenticated(&g, &anchor, &base, g.members[1], Some(b), 1),
        Err(Error::Conflict)
    );
    round
        .observe_authenticated(&g, &anchor, &base, g.members[2], None, 1)
        .unwrap();
    assert_eq!(round.selected().unwrap(), Some(b));
    assert_eq!(round.resolved(), Err(Error::Quorum));
    round
        .acknowledge_writeback(&g, receipt(&ids, 1, r.digest(), true))
        .unwrap();
    assert_eq!(round.resolved(), Err(Error::Quorum));
    assert_eq!(
        round.acknowledge_writeback(&g, receipt(&ids, 1, r.digest(), true)),
        Err(Error::Conflict)
    );
    round
        .acknowledge_writeback(&g, receipt(&ids, 2, r.digest(), true))
        .unwrap();
    assert_eq!(round.resolved().unwrap(), Some(b));
    let mut absent = RepairReadRound::new(&g, &anchor).unwrap();
    absent
        .observe_authenticated(&g, &anchor, &base, g.members[1], None, 1)
        .unwrap();
    assert_eq!(absent.resolved(), Err(Error::Quorum));
    absent
        .observe_authenticated(&g, &anchor, &base, g.members[2], None, 1)
        .unwrap();
    assert_eq!(absent.resolved().unwrap(), None);
    let mut forged = b;
    forged.availability.as_mut().unwrap().signature[0] ^= 1;
    let mut isolated = RepairReadRound::new(&g, &anchor).unwrap();
    assert!(isolated
        .observe_authenticated(&g, &anchor, &base, g.members[1], Some(forged), 1)
        .is_err());
    assert_eq!(isolated.selected(), Err(Error::Quorum));
    assert!(b
        .encode(
            &g,
            &anchor,
            &base,
            grant.expires + 1,
            &mut [0; REPAIR_BUNDLE_BYTES]
        )
        .is_err());
}
// ------------------------=
// FUNC: repair_bundle_rejects_malformed_canonical_storage
// DESC: Round-trips signed payload bytes and rejects every truncated boundary, altered padding and changed manifest.
// ------------------=
#[test]
fn repair_bundle_rejects_malformed_canonical_storage() {
    let (g, ids, anchor, grant, base, next, r) = fixture();
    let bundle = RepairBundle {
        grant,
        value: r,
        certificate: Some(RepairCertificate {
            value: r,
            prepared: [
                receipt(&ids, 1, r.digest(), false),
                receipt(&ids, 2, r.digest(), false),
            ],
        }),
        manifest: next,
        availability: Some(availability(r, &next)),
    };
    let mut bytes = [0; REPAIR_BUNDLE_BYTES];
    bundle.encode(&g, &anchor, &base, 1, &mut bytes).unwrap();
    assert_eq!(
        RepairBundle::decode(&g, &anchor, &base, 1, &bytes).unwrap(),
        bundle
    );
    for size in [0, 255, 319, 543, 799, REPAIR_BUNDLE_BYTES - 1] {
        assert!(RepairBundle::decode(&g, &anchor, &base, 1, &bytes[..size]).is_err());
    }
    for at in [0, 240, 320, 464, 544, 656, 800, 840] {
        let mut invalid = bytes;
        invalid[at] ^= 1;
        assert!(RepairBundle::decode(&g, &anchor, &base, 1, &invalid).is_err());
    }
    let mut staged = bundle;
    staged.certificate = None;
    staged.encode(&g, &anchor, &base, 1, &mut bytes).unwrap();
    assert_eq!(
        RepairBundle::decode(&g, &anchor, &base, 1, &bytes).unwrap(),
        staged
    );
}
use crate::runtime::{
    crypto::KeyRef,
    fabric::{
        manifest::{Chunk, Placement, PlacementState},
        metadata::{Record, SignedRecord},
        placement::StorageClass,
        resources::ResourceId,
    },
};
type Signers = [(NodeCrypto, KeyRef); 3];
// ------------------------=
// FUNC: destination
// DESC: Constructs the independent replacement's real identity, never an owner's signing key.
// ------------------=
fn destination() -> (NodeCrypto, KeyRef, NodeId) {
    let mut c = NodeCrypto::new();
    let k = c.initialize(&[99; 32], true).unwrap();
    let mut h = Sha256::new();
    h.update(b"InfinityOS NodeId v1");
    h.update(c.public_identity().unwrap());
    (c, k, NodeId(h.finalize().into()))
}
// ------------------------=
// FUNC: availability
// DESC: Signs the exact replacement copy receipt with the independently generated destination key.
// ------------------=
fn availability(r: RepairRecord, m: &Manifest) -> RepairAvailability {
    let (c, k, node) = destination();
    let p = m.placements[1].unwrap();
    let mut a = RepairAvailability {
        repair: r.digest(),
        node,
        resource: p.resource.0,
        device: p.device,
        public_key: c.public_identity().unwrap(),
        signature: [0; 64],
    };
    a.signature = c.sign(k, &a.transcript()).unwrap();
    a
}
// ------------------------=
// FUNC: receipt
// DESC: Signs an explicit member's exact durable phase acknowledgement for the fixture.
// ------------------=
fn receipt(ids: &Signers, member: u8, digest: [u8; 32], published: bool) -> Receipt {
    let mut r = Receipt {
        member,
        digest,
        published,
        signature: [0; 64],
    };
    r.signature = ids[member as usize]
        .0
        .sign(ids[member as usize].1, &r.transcript())
        .unwrap();
    r
}
// ------------------------=
// FUNC: fixture
// DESC: Builds actual owner/member signatures and a placement-only delegated repair anchored to immutable content.
// ------------------=
fn fixture() -> (
    Group,
    Signers,
    Certificate,
    RepairGrant,
    Manifest,
    Manifest,
    RepairRecord,
) {
    let ids: Signers = core::array::from_fn(|i| {
        let mut c = NodeCrypto::new();
        let k = c.initialize(&[i as u8 + 51; 32], true).unwrap();
        (c, k)
    });
    let keys = core::array::from_fn(|i| ids[i].0.public_identity().unwrap());
    let members = keys.map(|key| {
        let mut h = Sha256::new();
        h.update(b"InfinityOS NodeId v1");
        h.update(key);
        NodeId(h.finalize().into())
    });
    let g = Group {
        epoch: 1,
        owner: members[0],
        members,
        keys,
    };
    let hash = Sha256::digest([42]).into();
    let mut base = Manifest {
        object: [1; 16],
        version: 1,
        length: 1,
        hash,
        policy: StorageClass::Critical,
        minimum_available: 1,
        generation: 1,
        authority: members[0],
        authority_generation: 1,
        chunks: [None; 64],
        placements: [None; 8],
        healing: None,
    };
    base.chunks[0] = Some(Chunk {
        content: [2; 16],
        bytes: 1,
        hash,
    });
    base.placements[0] = Some(Placement {
        node: members[1],
        resource: ResourceId([3; 16]),
        device: [4; 16],
        generation: 1,
        version: 1,
        hash,
        state: PlacementState::Verified,
        admission_generation: 1,
    });
    let mut bytes = [0; MANIFEST_BYTES];
    base.encode(&mut bytes).unwrap();
    let r = Record {
        group: g.digest(),
        object: base.object,
        generation: 1,
        version: 1,
        previous: [0; 32],
        manifest: Sha256::digest(bytes).into(),
        namespace: [7; 32],
        policy: [8; 32],
        revocation: 1,
        deleted: false,
    };
    let anchor = Certificate {
        value: SignedRecord {
            record: r,
            signature: ids[0].0.sign(ids[0].1, &r.encode()).unwrap(),
        },
        prepared: [
            receipt(&ids, 0, r.digest(), false),
            receipt(&ids, 1, r.digest(), false),
        ],
    };
    let mut grant = RepairGrant {
        group: g.digest(),
        anchor: r.digest(),
        writer: members[1],
        destinations: [destination().2, members[1], members[2], NodeId([0; 32])],
        expires: 100,
        signature: [0; 64],
    };
    grant.signature = ids[0].0.sign(ids[0].1, &grant.transcript()).unwrap();
    let mut next = base;
    next.generation = 2;
    let mut replacement = base.placements[0].unwrap();
    replacement.node = destination().2;
    replacement.resource = ResourceId([10; 16]);
    replacement.device = [11; 16];
    replacement.admission_generation = 2;
    next.placements[1] = Some(replacement);
    next.encode(&mut bytes).unwrap();
    let mut repair = RepairRecord {
        anchor: r.digest(),
        grant: grant.digest(),
        sequence: 1,
        previous: [0; 32],
        manifest: Sha256::digest(bytes).into(),
        signature: [0; 64],
    };
    repair.signature = ids[1].0.sign(ids[1].1, &repair.transcript()).unwrap();
    (g, ids, anchor, grant, base, next, repair)
}
// ------------------------=
// FUNC: owner_offline_repair_requires_both_surviving_publication_receipts
// DESC: Validates an owner-authorized placement change and rejects success before two distinct surviving members durably publish.
// ------------------=
#[test]
fn owner_offline_repair_requires_both_surviving_publication_receipts() {
    let (g, ids, anchor, grant, base, next, r) = fixture();
    r.validate(&g, &anchor, &grant, &base, &next, 1).unwrap();
    r.successor(None).unwrap();
    let cert = RepairCertificate {
        value: r,
        prepared: [
            receipt(&ids, 1, r.digest(), false),
            receipt(&ids, 2, r.digest(), false),
        ],
    };
    let mut p = RepairPublication::new(&g, &anchor, &grant, &base, &next, cert, 1).unwrap();
    assert_eq!(p.committed(), Err(Error::Quorum));
    p.acknowledge(&g, receipt(&ids, 1, r.digest(), true))
        .unwrap();
    assert_eq!(p.committed(), Err(Error::Quorum));
    assert_eq!(
        p.acknowledge(&g, receipt(&ids, 1, r.digest(), true)),
        Err(Error::Conflict)
    );
    p.acknowledge(&g, receipt(&ids, 2, r.digest(), true))
        .unwrap();
    assert_eq!(p.committed().unwrap(), cert);
}
// ------------------------=
// FUNC: delegated_repair_cannot_change_content_authority_or_policy
// DESC: Even a correctly signed delegate cannot change immutable owner data or expand destination authority.
// ------------------=
#[test]
fn delegated_repair_cannot_change_content_authority_or_policy() {
    let (g, ids, anchor, grant, base, next, r) = fixture();
    for case in 0..6 {
        let mut altered = next;
        match case {
            0 => altered.hash = [55; 32],
            1 => altered.authority = g.members[1],
            2 => altered.policy = StorageClass::Protected,
            3 => altered.version += 1,
            4 => altered.placements[1].as_mut().unwrap().node = NodeId([90; 32]),
            _ => altered.chunks[0].as_mut().unwrap().content = [92; 16],
        };
        let mut bytes = [0; MANIFEST_BYTES];
        if altered.encode(&mut bytes).is_err() {
            assert!(r.validate(&g, &anchor, &grant, &base, &altered, 1).is_err());
            continue;
        }
        let mut forged = r;
        forged.manifest = Sha256::digest(bytes).into();
        forged.signature = ids[1].0.sign(ids[1].1, &forged.transcript()).unwrap();
        assert!(forged
            .validate(&g, &anchor, &grant, &base, &altered, 1)
            .is_err());
    }
    assert_eq!(
        r.validate(&g, &anchor, &grant, &base, &next, 100),
        Err(Error::Denied)
    );
    let mut forged = grant;
    forged.destinations[0] = NodeId([90; 32]);
    assert_eq!(forged.validate(&g, &anchor, 1), Err(Error::Signature));
}
// ------------------------=
// FUNC: repair_fences_reject_stale_owner_anchor_forks_and_duplicate_quorums
// DESC: Refuses old-anchor repair authority, skipped generation, alternate predecessor and forged quorum membership.
// ------------------=
#[test]
fn repair_fences_reject_stale_owner_anchor_forks_and_duplicate_quorums() {
    let (g, ids, anchor, grant, base, next, r) = fixture();
    let mut successor = r;
    successor.sequence = 2;
    successor.previous = r.digest();
    successor.successor(Some(r)).unwrap();
    assert_eq!(r.successor(Some(successor)), Err(Error::Stale));
    successor.previous = [1; 32];
    assert_eq!(successor.successor(Some(r)), Err(Error::Stale));
    let mut changed = anchor.value.record;
    changed.generation += 1;
    changed.previous = anchor.value.record.digest();
    changed.revocation += 1;
    let newer = Certificate {
        value: SignedRecord {
            record: changed,
            signature: ids[0].0.sign(ids[0].1, &changed.encode()).unwrap(),
        },
        prepared: [
            receipt(&ids, 0, changed.digest(), false),
            receipt(&ids, 1, changed.digest(), false),
        ],
    };
    assert_eq!(grant.validate(&g, &newer, 1), Err(Error::Denied));
    let duplicate = receipt(&ids, 1, r.digest(), false);
    let cert = RepairCertificate {
        value: r,
        prepared: [duplicate, duplicate],
    };
    assert_eq!(
        cert.validate(&g, &anchor, &grant, &base, &next, 1),
        Err(Error::Quorum)
    );
}
