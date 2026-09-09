use super::*;
use crate::runtime::crypto::KeyRef;
// ------------------------=
// FUNC: fixture
// DESC: Creates three real signing identities with explicitly configured fixed membership.
// ------------------=
fn fixture() -> (Group, [(NodeCrypto, KeyRef); 3]) {
    let identities = core::array::from_fn(|i| {
        let mut c = NodeCrypto::new();
        let k = c.initialize(&[i as u8 + 1; 32], true).unwrap();
        (c, k)
    });
    let keys = core::array::from_fn(|i| identities[i].0.public_identity().unwrap());
    let members = keys.map(|key| {
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
            keys,
        },
        identities,
    )
}
// ------------------------=
// FUNC: signed
// DESC: Signs canonical fixture metadata with the actual designated writer key.
// ------------------=
fn signed(
    g: &Group,
    ids: &[(NodeCrypto, KeyRef); 3],
    previous: Option<Record>,
    deleted: bool,
) -> SignedRecord {
    let record = Record {
        group: g.digest(),
        object: [1; 16],
        generation: previous.map_or(1, |p| p.generation + 1),
        version: previous.map_or(1, |p| p.version + 1),
        previous: previous.map_or([0; 32], Record::digest),
        manifest: [2; 32],
        namespace: [3; 32],
        policy: if previous.is_some() { [5; 32] } else { [4; 32] },
        revocation: previous.map_or(1, |p| p.revocation + 1),
        deleted,
    };
    SignedRecord {
        record,
        signature: ids[0].0.sign(ids[0].1, &record.encode()).unwrap(),
    }
}
// ------------------------=
// FUNC: receipt
// DESC: Signs one member's phase-separated persistence receipt.
// ------------------=
fn receipt(ids: &[(NodeCrypto, KeyRef); 3], member: u8, r: Record, published: bool) -> Receipt {
    let mut p = Receipt {
        member,
        digest: r.digest(),
        published,
        signature: [0; 64],
    };
    p.signature = ids[member as usize]
        .0
        .sign(ids[member as usize].1, &p.transcript())
        .unwrap();
    p
}
// ------------------------=
// FUNC: certificate
// DESC: Builds an owner-signed certificate with independent valid staging receipts.
// ------------------=
fn certificate(ids: &[(NodeCrypto, KeyRef); 3], value: SignedRecord) -> Certificate {
    Certificate {
        value,
        prepared: [
            receipt(ids, 0, value.record, false),
            receipt(ids, 1, value.record, false),
        ],
    }
}
// ------------------------=
// FUNC: reader_grant
// DESC: Issues an actual owner-signed read-only delegation for an exact node and application principal.
// ------------------=
fn reader_grant(g: &Group, ids: &[(NodeCrypto, KeyRef); 3], record: Record) -> ReaderGrant {
    let mut grant = ReaderGrant {
        group: g.digest(),
        object: record.object,
        reader: g.members[1],
        principal: [7; 16],
        policy: record.policy,
        revocation: record.revocation,
        expires: 100,
        signature: [0; 64],
    };
    grant.signature = ids[0].0.sign(ids[0].1, &grant.transcript()).unwrap();
    grant
}
// ------------------------=
// FUNC: partial_publication_requires_read_quorum_and_writeback
// DESC: Exercises actual signatures, failed persistence, partial publication, A-offline quorum, duplicate acknowledgements and bounded write-back.
// ------------------=
#[test]
fn partial_publication_requires_read_quorum_and_writeback() {
    let (g, ids) = fixture();
    let first = signed(&g, &ids, None, false);
    let cert = certificate(&ids, first);
    let mut publication = Publication::new(&g, cert).unwrap();
    publication
        .acknowledge(&g, receipt(&ids, 0, first.record, true))
        .unwrap();
    assert_eq!(publication.committed(), Err(Error::Quorum));
    assert_eq!(
        publication.acknowledge(&g, receipt(&ids, 0, first.record, true)),
        Err(Error::Conflict)
    );
    publication
        .acknowledge(&g, receipt(&ids, 1, first.record, true))
        .unwrap();
    assert_eq!(publication.committed(), Ok(cert));
    let mut nodes = [Replica::default(); 3];
    assert_eq!(
        nodes[0].prepare(&g, first, |_| Err(Error::Persistence)),
        Err(Error::Persistence)
    );
    assert!(nodes[0].staged.is_none());
    for n in &mut nodes[..2] {
        n.prepare(&g, first, |_| Ok(())).unwrap();
    }
    nodes[1].publish(&g, cert, |_| Ok(())).unwrap();
    assert!(nodes[2].committed.is_none());
    let mut read = ReadRound::new(first.record.object);
    read.observe_authenticated(&g, g.members[1], nodes[1].committed)
        .unwrap();
    assert_eq!(read.selected(), Err(Error::Quorum));
    assert_eq!(
        read.observe_authenticated(&g, g.members[1], nodes[1].committed),
        Err(Error::Conflict)
    );
    read.observe_authenticated(&g, g.members[2], None).unwrap();
    assert_eq!(read.selected(), Ok(cert));
    let grant = reader_grant(&g, &ids, first.record);
    assert_eq!(
        read.readable(&g, &grant, g.members[1], [7; 16], 1),
        Err(Error::Quorum)
    );
    for i in [1, 2] {
        nodes[i].publish(&g, cert, |_| Ok(())).unwrap();
        read.acknowledge_writeback(&g, receipt(&ids, i as u8, first.record, true))
            .unwrap();
    }
    assert_eq!(
        read.readable(&g, &grant, g.members[1], [7; 16], 1),
        Ok(first.record)
    );
    assert_eq!(
        read.readable(&g, &grant, g.members[2], [7; 16], 1),
        Err(Error::Denied)
    );
    assert_eq!(
        read.readable(&g, &grant, g.members[1], [8; 16], 1),
        Err(Error::Denied)
    );
    assert_eq!(
        read.readable(&g, &grant, g.members[1], [7; 16], 100),
        Err(Error::Denied)
    );
    assert_eq!(
        read.acknowledge_writeback(&g, receipt(&ids, 1, first.record, true)),
        Err(Error::Conflict)
    );
    let restored = nodes[2];
    assert_eq!(restored.committed, Some(cert));
}
// ------------------------=
// FUNC: stale_replays_tombstones_and_revocation_fail_closed
// DESC: Rejects old generations, forged membership/signatures, changed canonical padding and obsolete reader-policy approval after quorum freshness.
// ------------------=
#[test]
fn stale_replays_tombstones_and_revocation_fail_closed() {
    let (g, ids) = fixture();
    let one = signed(&g, &ids, None, false);
    let two = signed(&g, &ids, Some(one.record), false);
    let mut n = Replica::default();
    n.prepare(&g, one, |_| Ok(())).unwrap();
    n.publish(&g, certificate(&ids, one), |_| Ok(())).unwrap();
    n.prepare(&g, two, |_| Ok(())).unwrap();
    n.publish(&g, certificate(&ids, two), |_| Ok(())).unwrap();
    assert_eq!(
        n.publish(&g, certificate(&ids, one), |_| Ok(())),
        Err(Error::Stale)
    );
    let mut read = ReadRound::new(one.record.object);
    assert_eq!(
        read.observe_authenticated(&g, NodeId([9; 32]), Some(certificate(&ids, two))),
        Err(Error::Denied)
    );
    read.observe_authenticated(&g, g.members[1], Some(certificate(&ids, two)))
        .unwrap();
    read.observe_authenticated(&g, g.members[2], Some(certificate(&ids, one)))
        .unwrap();
    for i in [1, 2] {
        read.acknowledge_writeback(&g, receipt(&ids, i, two.record, true))
            .unwrap();
    }
    assert_eq!(
        read.readable(
            &g,
            &reader_grant(&g, &ids, one.record),
            g.members[1],
            [7; 16],
            1
        ),
        Err(Error::Denied)
    );
    assert_eq!(
        read.readable(
            &g,
            &reader_grant(&g, &ids, two.record),
            g.members[1],
            [7; 16],
            1
        ),
        Ok(two.record)
    );
    let tombstone = signed(&g, &ids, Some(two.record), true);
    let cert = certificate(&ids, tombstone);
    let mut deleted = ReadRound::new(one.record.object);
    for i in [1, 2] {
        deleted
            .observe_authenticated(&g, g.members[i], Some(cert))
            .unwrap();
    }
    for i in [1, 2] {
        deleted
            .acknowledge_writeback(&g, receipt(&ids, i, tombstone.record, true))
            .unwrap();
    }
    assert_eq!(
        deleted.readable(
            &g,
            &reader_grant(&g, &ids, tombstone.record),
            g.members[1],
            [7; 16],
            1
        ),
        Err(Error::Deleted)
    );
    let mut corrupt = one;
    corrupt.signature[0] ^= 1;
    assert_eq!(corrupt.validate(&g), Err(Error::Signature));
    let mut duplicate = certificate(&ids, one);
    duplicate.prepared[1] = duplicate.prepared[0];
    assert_eq!(duplicate.validate(&g), Err(Error::Quorum));
    let mut wire = one.record.encode();
    wire[255] = 1;
    assert_eq!(Record::decode(&wire), Err(Error::Invalid));
    let mut bad = g;
    bad.members[2] = bad.members[1];
    assert_eq!(bad.validate(), Err(Error::Invalid));
}
// ------------------------=
// FUNC: read_round_never_mixes_epochs_or_memberships
// DESC: Different valid groups cannot contribute observations, write-back receipts or reader authorization to a round already bound to one epoch.
// ------------------=
#[test]
fn read_round_never_mixes_epochs_or_memberships() {
    let (g, ids) = fixture();
    let one = signed(&g, &ids, None, false);
    let cert = certificate(&ids, one);
    let mut other = g;
    other.epoch = 2;
    other.validate().unwrap();
    let mut read = ReadRound::new(one.record.object);
    read.observe_authenticated(&g, g.members[1], Some(cert))
        .unwrap();
    assert_eq!(
        read.observe_authenticated(&other, other.members[2], None),
        Err(Error::Denied)
    );
    assert_eq!(read.selected(), Err(Error::Quorum));
    read.observe_authenticated(&g, g.members[2], None).unwrap();
    let ack = receipt(&ids, 1, one.record, true);
    assert_eq!(read.acknowledge_writeback(&other, ack), Err(Error::Denied));
    assert_eq!(
        read.readable(
            &g,
            &reader_grant(&g, &ids, one.record),
            g.members[1],
            [7; 16],
            1
        ),
        Err(Error::Quorum)
    );
    for i in [1, 2] {
        read.acknowledge_writeback(&g, receipt(&ids, i, one.record, true))
            .unwrap();
    }
    assert_eq!(
        read.readable(
            &other,
            &reader_grant(&g, &ids, one.record),
            g.members[1],
            [7; 16],
            1
        ),
        Err(Error::Denied)
    );
    let mut swapped = g;
    swapped.members.swap(1, 2);
    swapped.keys.swap(1, 2);
    swapped.validate().unwrap();
    assert_eq!(
        read.readable(
            &swapped,
            &reader_grant(&g, &ids, one.record),
            g.members[1],
            [7; 16],
            1
        ),
        Err(Error::Denied)
    );
    assert_eq!(
        read.readable(
            &g,
            &reader_grant(&g, &ids, one.record),
            g.members[1],
            [7; 16],
            1
        ),
        Ok(one.record)
    );
}
