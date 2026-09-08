//! Behavioral packet tests use the actual NetworkRuntime wire and datagram paths.
#[path = "../nic-probe/fixture.rs"]
mod fixture;
use crate::node::{NodeRuntime, transport::{NodeTransport, LinkAuthority, DiscoveryChange}, types::TrustState};
use crate::network::wire::Frame;

// ------------------------=
// FUNC: deliver
// DESC: Transfers real Ethernet bytes then admits decoded UDP using current peer policy and capabilities.
// ------------------=
fn deliver(from: &mut fixture::Fixture, to: &mut fixture::Fixture, now: u64, capture: &mut Option<Frame>) {
    for _ in 0..8 {
        let Some(frame) = from.network.wire.peek_transmit().copied() else { break; };
        // Binary protocol capture, not rendered or diagnostic text acceptance.
        if frame.length == 241 { *capture = Some(frame); }
        from.network.wire.complete_transmit();
        to.network.wire.ingest(&frame.bytes[..frame.length], now).unwrap();
    }
    while let Some(packet) = to.network.wire.receive_datagram() {
        to.network.connections.deliver_datagram(packet, &mut to.network.policy, &to.capabilities, now).unwrap();
    }
}

// ------------------------=
// FUNC: authority
// DESC: References only the explicit fixture's connected-peer capabilities.
// ------------------=
fn authority(fixture: &fixture::Fixture) -> LinkAuthority {
    LinkAuthority { owner: fixture.owner, connection: fixture.connection, send: fixture.send, receive: fixture.receive }
}

// ------------------------=
// FUNC: run
// DESC: Verifies native signed discovery, independent clocks, replay denial, authority revocation and offline quiescence.
// ------------------=
pub fn run() {
    reject_invalid_announcement();
    let mut a = fixture::configured([2,0,0,0,0,1], [10,42,0,1], [10,42,0,2]);
    let mut b = fixture::configured([2,0,0,0,0,2], [10,42,0,2], [10,42,0,1]);
    let mut an = NodeRuntime::new(); let mut bn = NodeRuntime::new();
    let aid = an.initialize(&[0x31;32], true).unwrap();
    let bid = bn.initialize(&[0x32;32], true).unwrap();
    let mut at = NodeTransport::new(); let mut bt = NodeTransport::new();
    at.initialize(&[0x41;32], true).unwrap(); bt.initialize(&[0x42;32], true).unwrap();
    at.persist_discovery = engineering_discovery_writer; bt.persist_discovery = engineering_discovery_writer;
    let mut wrong = authority(&a); wrong.send = a.receive;
    assert!(at.attach(wrong, &a.network, &a.capabilities, 0).is_err());
    at.attach(authority(&a), &a.network, &a.capabilities, 0).unwrap();
    bt.attach(authority(&b), &b.network, &b.capabilities, 0).unwrap();
    assert!(at.attach(authority(&a), &a.network, &a.capabilities, 0).is_err());
    let mut a_changes = 0; let mut b_changes = 0;
    let mut captured_a = None; let mut captured_b = None;
    for tick in 0..9 {
        for _ in 0..4 {
            if let Some(change) = at.poll(&mut an, &mut a.network, &a.capabilities, tick) {
                assert_eq!(change, DiscoveryChange::Discovered(bid)); a_changes += 1;
            }
            if let Some(change) = bt.poll(&mut bn, &mut b.network, &b.capabilities, tick + 4) {
                assert_eq!(change, DiscoveryChange::Discovered(aid)); b_changes += 1;
            }
            deliver(&mut a, &mut b, tick+4, &mut captured_a);
            deliver(&mut b, &mut a, tick, &mut captured_b);
        }
    }
    assert_eq!((a_changes, b_changes), (1, 1));
    let before = an.discovered_nodes().iter().flatten().find(|peer| peer.id == bid).copied().unwrap();
    assert_eq!(before.trust, TrustState::Untrusted);
    assert_eq!(at.inspect(a.connection, a.owner).unwrap().peer, Some(bid));
    assert!(at.inspect(a.connection, crate::execution::SecurityIdentity([0;16])).is_none());
    assert!(an.sessions().iter().all(Option::is_none));
    assert!(an.remote_grants().iter().all(Option::is_none));
    let replay = captured_b.unwrap();
    a.network.wire.ingest(&replay.bytes[..replay.length], 9).unwrap();
    while let Some(packet) = a.network.wire.receive_datagram() {
        a.network.connections.deliver_datagram(packet, &mut a.network.policy, &a.capabilities, 9).unwrap();
    }
    for _ in 0..4 { assert_eq!(at.poll(&mut an, &mut a.network, &a.capabilities, 9), None); }
    assert_eq!(at.rejected_packets, 1);
    assert_eq!(an.discovered_nodes().iter().flatten().find(|peer| peer.id == bid).unwrap().last_seen, before.last_seen);
    a.capabilities.revoke(a.send).unwrap();
    for _ in 0..4 { at.poll(&mut an, &mut a.network, &a.capabilities, 10); }
    assert!(a.network.wire.peek_transmit().is_none());
    b.network.activate_profile(3).unwrap();
    for _ in 0..4 { bt.poll(&mut bn, &mut b.network, &b.capabilities, 13); }
    assert!(b.network.wire.peek_transmit().is_none());
    at.detach(a.connection);
    for _ in 0..8 { assert_eq!(at.poll(&mut an, &mut a.network, &a.capabilities, 14), None); }
}

// ------------------------=
// FUNC: pairing_keeps_discovery_live
// DESC: Keeps real discovery and pairing carriage active while two operators confirm at different times within the pairing lease.
// ------------------=
#[test]
fn pairing_keeps_discovery_live() {
    pairing_review(45, 65, false);
}

// ------------------------=
// FUNC: pairing_review_boundaries
// DESC: Exercises immediate consent, prolonged normal traffic and exact expired-consent rejection without changing production leases.
// ------------------=
#[test]
fn pairing_review_boundaries() {
    pairing_review(20, 20, false);
    pairing_review(20, 65, false);
    pairing_review(20, 120, false);
    pairing_review(20, 132, true);
}

// ------------------------=
// FUNC: pairing_review
// DESC: Runs independent native packet runtimes throughout a declared operator-review interval and checks transaction identity and trust transitions.
// ------------------=
fn pairing_review(first: u64, second: u64, expired: bool) {
    pairing_review_with_storage(first, second, expired, false);
}

// ------------------------=
// FUNC: pairing_storage_retry
// DESC: Preserves authenticated consent across durable failure without granting trust, then commits on recovery.
// ------------------=
#[test]
fn pairing_storage_retry() {
    pairing_review_with_storage(20, 25, false, true);
}

// ------------------------=
// FUNC: rejected_pairing_writer
// DESC: Injects a durable checkpoint failure without weakening other protocol boundaries.
// ------------------=
fn rejected_pairing_writer(_: &[u8; crate::node::types::NODE_STATE_BYTES]) -> bool { false }

// ------------------------=
// FUNC: pairing_review_with_storage
// DESC: Exercises normal transport and optional durable failure throughout the unchanged confirmation lease.
// ------------------=
fn pairing_review_with_storage(first: u64, second: u64, expired: bool, storage_failure: bool) {
    let mut a = fixture::configured_until([2,0,0,0,0,1], [10,42,0,1], [10,42,0,2], 200);
    let mut b = fixture::configured_until([2,0,0,0,0,2], [10,42,0,2], [10,42,0,1], 200);
    let mut an = NodeRuntime::new(); let mut bn = NodeRuntime::new();
    let aid = an.initialize(&[0x31;32], true).unwrap();
    let bid = bn.initialize(&[0x32;32], true).unwrap();
    let mut at = NodeTransport::new(); let mut bt = NodeTransport::new();
    at.initialize(&[0x41;32], true).unwrap(); bt.initialize(&[0x42;32], true).unwrap();
    at.persist_discovery = engineering_discovery_writer; bt.persist_discovery = engineering_discovery_writer;
    at.attach(authority(&a), &a.network, &a.capabilities, 0).unwrap();
    bt.attach(authority(&b), &b.network, &b.capabilities, 0).unwrap();
    let mut capture = None;
    at.trust.persist_pairing = engineering_discovery_writer;
    bt.trust.persist_pairing = engineering_discovery_writer;
    let mut original = None;
    for tick in 0..second + if storage_failure { 60 } else { 20 } {
        if storage_failure && tick == second { at.trust.persist_pairing = rejected_pairing_writer; }
        if storage_failure && tick == second + 40 {
            let before = at.trust.lifecycle(bid).unwrap();
            assert_eq!(before.3, 3);
            assert_eq!(before.1, crate::node::wire_trust::WireState::LocallyConfirmed);
            assert_ne!(an.discovered_nodes().iter().flatten().find(|p| p.id == bid).unwrap().trust, TrustState::Trusted);
            assert_eq!(at.trust.last_error, Some(crate::node::types::NodeError::StateCorrupt));
            at.trust.persist_pairing = engineering_discovery_writer;
            let view = at.trust.verification(aid, bid).unwrap();
            assert_eq!(at.trust.confirm(&mut an, view.transaction, view.code, false, tick), Err(crate::node::types::NodeError::HumanApprovalRequired));
            assert_eq!(at.trust.confirm(&mut an, view.transaction, (view.code + 1) % 1_000_000, true, tick), Err(crate::node::types::NodeError::VerificationMismatch));
            assert_eq!(at.trust.lifecycle(bid).unwrap(), before);
            at.trust.confirm(&mut an, view.transaction, view.code, true, tick).unwrap();
        }
        for _ in 0..4 {
            at.poll(&mut an, &mut a.network, &a.capabilities, tick);
            bt.poll(&mut bn, &mut b.network, &b.capabilities, tick);
            deliver(&mut a, &mut b, tick, &mut capture);
            deliver(&mut b, &mut a, tick, &mut capture);
        }
        if tick == 9 {
            let link = at.inspect(a.connection, a.owner).unwrap();
            at.trust.begin(&mut an, link, 0, false, tick).unwrap();
        }
        if tick == first || (tick == second && !expired) {
            let av = at.trust.verification(aid, bid).expect("left pairing must survive operator review");
            let bv = bt.trust.verification(bid, aid).expect("right pairing must survive operator review");
            assert_eq!(av.code, bv.code);
            assert_eq!(av.fingerprint, bv.fingerprint);
            assert_eq!(av.transaction, bv.transaction);
            if let Some(prior) = original { assert_eq!((bv.transaction, bv.code), prior); }
            else { original = Some((bv.transaction, bv.code)); }
            if tick == first { at.trust.confirm(&mut an, av.transaction, av.code, true, tick).unwrap(); }
            if tick == second { bt.trust.confirm(&mut bn, bv.transaction, bv.code, true, tick).unwrap(); }
        }
        if tick == second && expired {
            let (transaction, code) = original.unwrap();
            assert_eq!(bt.trust.confirm(&mut bn, transaction, code, true, tick), Err(crate::node::types::NodeError::PairingExpired));
            assert!(bt.trust.verification(bid, aid).is_none());
        }
    }
    if expired {
        assert!(an.discovered_nodes().iter().flatten().all(|peer| peer.trust != TrustState::Trusted));
        assert!(bn.discovered_nodes().iter().flatten().all(|peer| peer.trust != TrustState::Trusted));
        return;
    }
    assert_eq!(an.discovered_nodes().iter().flatten().find(|peer| peer.id == bid).unwrap().trust, TrustState::Trusted);
    assert_eq!(bn.discovered_nodes().iter().flatten().find(|peer| peer.id == aid).unwrap().trust, TrustState::Trusted);
}

// ------------------------=
// FUNC: engineering_discovery_writer
// DESC: Explicit HOST fixture commit boundary; production transport requires the native durable writer.
// ------------------=
fn engineering_discovery_writer(_: &[u8; crate::node::types::NODE_STATE_BYTES]) -> bool { true }

// ------------------------=
// FUNC: reject_invalid_announcement
// DESC: Verifies a live receiver nonce cannot make an invalid signature or malformed payload refresh peer state.
// ------------------=
fn reject_invalid_announcement() {
    let mut a = fixture::configured([2,0,0,0,0,1], [10,42,0,1], [10,42,0,2]);
    let mut b = fixture::configured([2,0,0,0,0,2], [10,42,0,2], [10,42,0,1]);
    let mut an = NodeRuntime::new(); let mut bn = NodeRuntime::new();
    an.initialize(&[0x31;32], true).unwrap(); bn.initialize(&[0x32;32], true).unwrap();
    let mut at = NodeTransport::new(); let mut bt = NodeTransport::new();
    at.initialize(&[0x51;32], true).unwrap(); bt.initialize(&[0x52;32], true).unwrap();
    at.persist_discovery = engineering_discovery_writer; bt.persist_discovery = engineering_discovery_writer;
    at.attach(authority(&a), &a.network, &a.capabilities, 0).unwrap();
    bt.attach(authority(&b), &b.network, &b.capabilities, 0).unwrap();
    let mut capture = None;
    for tick in 0..5 {
        for _ in 0..4 {
            at.poll(&mut an, &mut a.network, &a.capabilities, tick);
            bt.poll(&mut bn, &mut b.network, &b.capabilities, tick);
            deliver(&mut a, &mut b, tick, &mut capture);
            // Replace just the signed announcement body; NetworkRuntime constructs
            // a valid UDP checksum, so rejection must occur at node authentication.
            let mut queued = [None; 8];
            for slot in &mut queued {
                let Some(frame) = b.network.wire.peek_transmit().copied() else { break; };
                b.network.wire.complete_transmit(); *slot = Some(frame);
            }
            for frame in queued.iter().flatten() {
                if frame.length == 241 {
                    let mut payload = [0; 199]; payload.copy_from_slice(&frame.bytes[42..241]);
                    payload[198] ^= 1;
                    b.network.send_datagram(b.owner, b.send, b.connection, &payload, tick, tick+1, &b.capabilities).unwrap();
                } else { a.network.wire.ingest(&frame.bytes[..frame.length], tick).unwrap(); }
            }
            deliver(&mut b, &mut a, tick, &mut capture);
        }
    }
    assert!(at.rejected_packets > 0);
    assert_eq!(at.last_error, Some(crate::node::types::NodeError::SignatureInvalid));
    assert!(an.discovered_nodes().iter().all(Option::is_none));
    for (tick, payload) in [(5, &[0u8; 7][..]), (6, &[0u8; 512][..])] {
        while a.network.receive_datagram(a.owner, a.receive, a.connection, tick, &a.capabilities).is_ok() {}
        b.network.send_datagram(b.owner, b.send, b.connection, payload, tick, tick+1, &b.capabilities).unwrap();
        deliver(&mut b, &mut a, tick, &mut capture);
        for _ in 0..4 { at.poll(&mut an, &mut a.network, &a.capabilities, tick); }
        assert_eq!(at.last_error, Some(crate::node::types::NodeError::InvalidAdvertisement));
    }
}
