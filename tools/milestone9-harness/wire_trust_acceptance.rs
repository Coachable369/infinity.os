#[path = "../wire-trust-fixture.rs"]
mod fixture;
use crate::node::types::TrustState;
use fixture::Fixture;
#[cfg(test)]
#[path="simultaneous_reconnect.rs"]
mod simultaneous_reconnect;

// ------------------------=
// FUNC: advance_four
// DESC: Exercises the real millisecond scheduler, bounded four-frame NIC pump and native datagram admission on four independent runtime instances.
// ------------------=
fn advance_four(peers: &mut [Fixture; 4], millisecond: &mut u64, seconds: u64, confirms: &mut [u64; 4]) {
    for _ in 0..seconds * 1000 {
        let now = *millisecond / 1000;
        for sender in 0..4 {
            for _ in 0..4 {
                let Some(frame) = peers[sender].network.wire.peek_transmit().copied() else { break; };
                peers[sender].network.wire.complete_transmit();
                if frame.length > 50 && &frame.bytes[42..50] == b"IN9A0001" && frame.bytes[50] == 5 {
                    confirms[sender] += 1;
                }
                for recipient in 0..4 {
                    if recipient != sender {
                        let _ = peers[recipient].network.wire.ingest(&frame.bytes[..frame.length], now);
                    }
                }
            }
        }
        for peer in peers.iter_mut() {
            for _ in 0..4 {
                let Some(packet) = peer.network.wire.receive_datagram() else { break; };
                peer.network.connections.deliver_datagram(packet, &mut peer.network.policy, &peer.capabilities, now).unwrap();
            }
            peer.transport.poll(&mut peer.nodes, &mut peer.network, &peer.capabilities, now);
        }
        *millisecond += 1;
    }
}

// ------------------------=
// FUNC: four_node_confirmations_with_real_poll_cadence
// DESC: Verifies paired consent crosses three busy endpoints at the actual millisecond cadence without extending discovery or ceremony leases.
// ------------------=
#[test]
fn four_node_confirmations_with_real_poll_cadence() {
    std::thread::Builder::new()
        .name("four-node-transport-fixture".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(four_node_confirmations_case)
        .expect("bounded fixture worker")
        .join()
        .expect("four-node transport behavior");
}

// ------------------------=
// FUNC: four_node_confirmations_case
// DESC: Holds four large isolated runtime fixtures on an explicitly bounded host-only worker stack, never the native kernel stack.
// ------------------=
fn four_node_confirmations_case() {
    let mut peers = core::array::from_fn(|index| {
        let number = index as u8 + 1;
        let others: Vec<u8> = (1..=4).filter(|peer| *peer != number).collect();
        fixture::configured_peers([2, 0, 0, 0, 0, number], [number + 64; 32], &others)
    });
    let mut millis = 0;
    let mut confirms = [0; 4];
    advance_four(&mut peers, &mut millis, 30, &mut confirms);
    for peer in &peers {
        assert_eq!(peer.nodes.discovered_nodes().iter().flatten().count(), 3);
    }
    let aid = peers[0].nodes.local_id().unwrap();
    let bid = peers[1].nodes.local_id().unwrap();
    let link = peers[0].transport.inspect(peers[0].connection, peers[0].owner).unwrap();
    let a = &mut peers[0];
    a.transport.trust.begin(&mut a.nodes, link, 0, false, millis / 1000).unwrap();
    advance_four(&mut peers, &mut millis, 16, &mut confirms);
    let av = peers[0].transport.trust.verification(aid, bid).unwrap();
    let bv = peers[1].transport.trust.verification(bid, aid).unwrap();
    assert_eq!(av.fingerprint, bv.fingerprint);
    assert_eq!(av.transaction, bv.transaction);
    advance_four(&mut peers, &mut millis, 26, &mut confirms);
    let a = &mut peers[0];
    a.transport.trust.confirm(&mut a.nodes, av.transaction, av.code, true, millis / 1000).unwrap();
    advance_four(&mut peers, &mut millis, 7, &mut confirms);
    let b = &mut peers[1];
    b.transport.trust.confirm(&mut b.nodes, bv.transaction, bv.code, true, millis / 1000).unwrap();
    advance_four(&mut peers, &mut millis, 30, &mut confirms);
    assert!(confirms[0] > 0 && confirms[1] > 0, "signed confirmation transmission counts: {:?}", confirms);
    for (index, target) in [(0, bid), (1, aid)] {
        let peer = &peers[index];
        assert_eq!(peer.nodes.discovered_nodes().iter().flatten().find(|node| node.id == target).unwrap().trust,
                   TrustState::Trusted, "wire confirmations={:?}, lifecycle={:?}, transport={:?}",
                   confirms, peer.transport.trust.lifecycle(target), peer.transport.last_error);
    }
}

// ------------------------=
// FUNC: established_session_is_not_a_discovery_lease
// DESC: Suppresses discovery after establishment, then verifies authenticated data, exact session expiry, and immediate trust revocation independently.
// ------------------=
#[test]
fn established_session_is_not_a_discovery_lease() {
    let mut a = fixture::configured([2, 0, 0, 0, 0, 1], [0x91; 32]);
    let mut b = fixture::configured([2, 0, 0, 0, 0, 2], [0x92; 32]);
    let mut now = 0;
    advance(&mut a, &mut b, &mut now, 8);
    let aid = a.nodes.local_id().unwrap(); let bid = b.nodes.local_id().unwrap();
    let link = a.transport.inspect(a.connection, a.owner).unwrap();
    let tx = a.transport.trust.begin(&mut a.nodes, link, 0, false, now).unwrap();
    advance(&mut a, &mut b, &mut now, 16);
    let code = a.transport.trust.verification(aid, bid).unwrap().code;
    a.transport.trust.confirm(&mut a.nodes, tx, code, true, now).unwrap();
    b.transport.trust.confirm(&mut b.nodes, tx, code, true, now).unwrap();
    advance(&mut a, &mut b, &mut now, 6);
    a.transport.trust.begin(&mut a.nodes, link, 0, true, now).unwrap();
    advance(&mut a, &mut b, &mut now, 16);
    let handle = a.transport.trust.session(bid).unwrap();
    let expiry = a.nodes.sessions().iter().flatten().find(|s| s.id == handle).unwrap().expires_at;
    now += crate::node::DISCOVERY_LEASE_TICKS + 1;
    a.nodes.sweep(now); b.nodes.sweep(now);
    a.transport.trust.tick(&mut a.nodes, now); b.transport.trust.tick(&mut b.nodes, now);
    assert_eq!(a.transport.trust.session(bid), Some(handle));
    let mut expired_nodes = a.nodes.clone(); let mut expired_wire = a.transport.trust.clone();
    expired_wire.tick(&mut expired_nodes, expiry);
    assert_eq!(expired_wire.session(bid), None);
    let mut revoked_nodes = a.nodes.clone(); let mut revoked_wire = a.transport.trust.clone();
    revoked_nodes.revoke_trust(bid, now, 1).unwrap();
    assert!(revoked_wire.send_data(&mut revoked_nodes, bid, &[7], false, now).is_err());
    a.transport.trust.send_data(&mut a.nodes, bid, &[1, 7, 9], false, now).unwrap();
    let packet = a.transport.trust.outgoing(a.connection, now).unwrap();
    let peer_link = b.transport.inspect(b.connection, b.owner).unwrap();
    b.transport.trust.ingest(&mut b.nodes, peer_link, &packet.bytes[..packet.length as usize], now).unwrap();
    assert_eq!(b.transport.trust.receive_data().unwrap().bytes[..3], [1, 7, 9]);
}

// ------------------------=
// FUNC: verified_pairing_outlives_discovery_hint
// DESC: Drops ordinary discovery traffic beyond its liveness lease while preserving an authenticated unexpired pairing, then requires signed dual consent and rejects actual expiry or revocation.
// ------------------=
#[test]
fn verified_pairing_outlives_discovery_hint() {
    for revoke in [false, true] {
        let mut a = fixture::configured([2, 0, 0, 0, 0, 1], [0x71; 32]);
        let mut b = fixture::configured([2, 0, 0, 0, 0, 2], [0x72; 32]);
        let mut now = 0;
        advance(&mut a, &mut b, &mut now, 8);
        let aid = a.nodes.local_id().unwrap();
        let bid = b.nodes.local_id().unwrap();
        let link = a.transport.inspect(a.connection, a.owner).unwrap();
        a.transport.trust.begin(&mut a.nodes, link, 0, false, now).unwrap();
        advance(&mut a, &mut b, &mut now, 16);
        let av = a.transport.trust.verification(aid, bid).unwrap();
        let bv = b.transport.trust.verification(bid, aid).unwrap();
        let mut disconnected_nodes = a.nodes.clone();
        let mut disconnected_wire = a.transport.trust.clone();
        disconnected_wire.disconnect(&mut disconnected_nodes, a.connection, now);
        assert!(disconnected_wire.verification(aid, bid).is_none());
        assert_ne!(disconnected_nodes.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);
        a.transport.trust.confirm(&mut a.nodes, av.transaction, av.code, true, now).unwrap();
        advance(&mut a, &mut b, &mut now, 6);
        for _ in 0..40 {
            for f in [&mut a, &mut b] {
                f.transport.poll(&mut f.nodes, &mut f.network, &f.capabilities, now);
                while f.network.wire.peek_transmit().is_some() { f.network.wire.complete_transmit(); }
            }
            now += 1;
        }
        assert!(now < av.expires && now < bv.expires);
        assert_eq!(a.transport.trust.verification(aid, bid).unwrap().transaction, av.transaction);
        assert_eq!(b.transport.trust.verification(bid, aid).unwrap().transaction, bv.transaction);
        if revoke {
            b.nodes.revoke_trust(aid, now, now).unwrap();
            assert_eq!(b.transport.trust.confirm(&mut b.nodes, bv.transaction, bv.code, true, now), Err(crate::node::types::NodeError::Blocked));
        } else {
            b.transport.trust.confirm(&mut b.nodes, bv.transaction, bv.code, true, now).unwrap();
            advance(&mut a, &mut b, &mut now, 12);
            assert_eq!(a.nodes.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);
            assert_eq!(b.nodes.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);
            assert_eq!(a.transport.trust.confirm(&mut a.nodes, av.transaction, av.code, true, av.expires), Err(crate::node::types::NodeError::PairingExpired));
        }
    }
}
#[path = "remote_iop_acceptance.rs"]
mod remote_iop_acceptance;

// ------------------------=
// FUNC: transfer
// DESC: Transfers actual framed Ethernet bytes between independent host mechanism fixtures.
// ------------------=
fn transfer(a: &mut Fixture, b: &mut Fixture, now: u64) {
    for _ in 0..8 {
        let Some(frame) = a.network.wire.peek_transmit().copied() else {
            break;
        };
        a.network.wire.complete_transmit();
        b.network
            .wire
            .ingest(&frame.bytes[..frame.length], now)
            .unwrap();
    }
    while let Some(packet) = b.network.wire.receive_datagram() {
        b.network
            .connections
            .deliver_datagram(packet, &mut b.network.policy, &b.capabilities, now)
            .unwrap();
    }
}
// ------------------------=
// FUNC: advance
// DESC: Advances bounded cooperative node/network pumps without direct cross-runtime protocol calls.
// ------------------=
fn advance(a: &mut Fixture, b: &mut Fixture, now: &mut u64, count: u64) {
    for _ in 0..count {
        for _ in 0..4 {
            a.transport
                .poll(&mut a.nodes, &mut a.network, &a.capabilities, *now);
            b.transport
                .poll(&mut b.nodes, &mut b.network, &b.capabilities, *now);
            for f in [&mut *a, &mut *b] {
                f.iop.poll_remote_node(&f.capabilities, &mut f.nodes, &mut f.transport.trust, *now);
                if f.execute_remote { f.iop.execute_remote_node_durable(&mut f.nodes, *now, &mut |_| true); }
                f.iop.poll_membership(&mut f.nodes, &mut f.transport.trust, *now, |_| true);
            }
            transfer(a, b, *now);
            transfer(b, a, *now);
        }
        *now += 1;
    }
}
// ------------------------=
// FUNC: run
// DESC: Exercises explicit independent verification, consent, fresh handshake and duplex data using production wire mechanisms in HOST scope.
// ------------------=
pub fn run() {
    offline_negotiation(false);
    offline_negotiation(true);
    let mut a = fixture::configured([2, 0, 0, 0, 0, 1], [0x71; 32]);
    let mut b = fixture::configured([2, 0, 0, 0, 0, 2], [0x72; 32]);
    let mut now = 0;
    advance(&mut a, &mut b, &mut now, 8);
    let aid = a.nodes.local_id().unwrap();
    let bid = b.nodes.local_id().unwrap();
    assert_ne!(aid, bid);
    let link = a.transport.inspect(a.connection, a.owner).unwrap();
    assert_eq!(link.peer, Some(bid));
    let cancelled = a
        .transport
        .trust
        .begin(&mut a.nodes, link, 0, false, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 16);
    a.transport
        .trust
        .cancel(&mut a.nodes, cancelled, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 4);
    assert!(a.transport.trust.verification(aid, bid).is_none());
    assert!(b.transport.trust.verification(bid, aid).is_none());
    let tx = a
        .transport
        .trust
        .begin(&mut a.nodes, link, 0, false, now)
        .unwrap();
    assert_ne!(tx, cancelled);
    assert!(a
        .transport
        .trust
        .confirm(&mut a.nodes, cancelled, 0, true, now)
        .is_err());
    advance(&mut a, &mut b, &mut now, 16);
    let av = a.transport.trust.verification(aid, bid).unwrap();
    let bv = b.transport.trust.verification(bid, aid).unwrap();
    assert_eq!(av.transaction, tx);
    assert_eq!(av.fingerprint, bv.fingerprint);
    assert_eq!(av.code, bv.code);
    assert!(a.nodes.sessions().iter().all(Option::is_none));
    assert!(b.nodes.sessions().iter().all(Option::is_none));
    assert!(a
        .transport
        .trust
        .confirm(&mut a.nodes, tx, av.code, false, now)
        .is_err());
    assert!(a
        .transport
        .trust
        .confirm(&mut a.nodes, tx, (av.code + 1) % 1_000_000, true, now)
        .is_err());
    a.transport
        .trust
        .confirm(&mut a.nodes, tx, av.code, true, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 4);
    assert_ne!(
        a.nodes
            .discovered_nodes()
            .iter()
            .flatten()
            .find(|n| n.id == bid)
            .unwrap()
            .trust,
        TrustState::Trusted
    );
    b.transport
        .trust
        .confirm(&mut b.nodes, tx, bv.code, true, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 6);
    assert_eq!(
        a.nodes
            .discovered_nodes()
            .iter()
            .flatten()
            .find(|n| n.id == bid)
            .unwrap()
            .trust,
        TrustState::Trusted
    );
    let link = a.transport.inspect(a.connection, a.owner).unwrap();
    a.transport
        .trust
        .begin(&mut a.nodes, link, 0, true, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 16);
    let ah = a.transport.trust.session(bid).unwrap();
    let bh = b.transport.trust.session(aid).unwrap();
    remote_iop_acceptance::run(&mut a, &mut b, &mut now);
    let ar = a
        .nodes
        .sessions()
        .iter()
        .flatten()
        .find(|s| s.id == ah)
        .unwrap()
        .protocol_reference;
    assert!(a
        .transport
        .trust
        .send_data(&mut a.nodes, bid, &[0; 193], false, now)
        .is_err());
    assert_eq!(
        ar,
        b.nodes
            .sessions()
            .iter()
            .flatten()
            .find(|s| s.id == bh)
            .unwrap()
            .protocol_reference
    );
    a.transport
        .trust
        .send_data(&mut a.nodes, bid, &[1, 2, 3], false, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 4);
    assert_eq!(
        b.transport.trust.receive_data().unwrap().bytes[..3],
        [1, 2, 3]
    );
    b.transport
        .trust
        .send_data(&mut b.nodes, aid, &[4, 5], false, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 4);
    assert_eq!(a.transport.trust.receive_data().unwrap().bytes[..2], [4, 5]);
    a.transport
        .trust
        .send_data(&mut a.nodes, bid, &[], true, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 4);
    assert!(a.transport.trust.session(bid).is_none());
    assert!(b.transport.trust.session(aid).is_none());
    let link = a.transport.inspect(a.connection, a.owner).unwrap();
    a.transport
        .trust
        .begin(&mut a.nodes, link, 0, true, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 16);
    let fresh = a.transport.trust.session(bid).unwrap();
    assert_ne!(fresh, ah);
    assert_ne!(
        a.nodes
            .sessions()
            .iter()
            .flatten()
            .find(|s| s.id == fresh)
            .unwrap()
            .protocol_reference,
        ar
    );
    restart_pairing_receipts(&mut a, &mut b, &mut now, ar);
    membership_wire(&mut a, &mut b, &mut now);
    a.network.connections.bind_native_address(None);
    for _ in 0..4 {
        a.transport
            .poll(&mut a.nodes, &mut a.network, &a.capabilities, now);
    }
    assert!(a.transport.trust.session(bid).is_none());
    assert!(a
        .nodes
        .sessions()
        .iter()
        .flatten()
        .all(|s| s.state == crate::node::types::SessionState::Closed));
}

// ------------------------=
// FUNC: membership_wire
// DESC: Exercises durable consent through local IOP and synchronized Join/Leave over authenticated Ethernet, with no direct cross-node mutation.
// ------------------=
fn membership_wire(a: &mut Fixture, b: &mut Fixture, now: &mut u64) {
    use crate::iop::{OperationId, IopMessage};
    use crate::capability::CapabilityType;
    use crate::execution::SecurityIdentity;
    let aid = a.nodes.local_id().unwrap(); let bid = b.nodes.local_id().unwrap();
    for (f, peer) in [(&mut *a, bid), (&mut *b, aid)] {
        let mut policy = crate::node_request(peer, OperationId::NodePolicyUpdate);
        policy.flags = 1; policy.value = 1;
        f.nodes.commit_control(OperationId::NodePolicyUpdate, policy, *now, 1, 1, |_| true).unwrap();
        let service = SecurityIdentity([0x9d; 16]);
        f.iop.ensure_owned_endpoint(0xda01, service).unwrap(); f.iop.ensure_owned_endpoint(0xda02, f.owner).unwrap();
        let cap = f.capabilities.grant(CapabilityType::ServiceCall, OperationId::NodeJoin.machine_id() as u64, 1, 0, service, f.owner, Some(*now + 100), 0).unwrap();
        let request = crate::node_request(peer, OperationId::NodeJoin);
        let id = f.iop.next_node_request().unwrap();
        f.iop.send(0xda01, IopMessage::request(OperationId::NodeJoin, id, f.owner, cap, *now + 30, id, &request.encode()).unwrap(), &f.capabilities, *now).unwrap();
        f.iop.dispatch_node_transaction(&f.capabilities, &mut f.nodes, OperationId::NodeJoin, 0xda01, 0xda02, service, *now, |nodes, request, correlation, causation| nodes.commit_domain_intent(OperationId::NodeJoin, request, *now, correlation, causation, |_| true)).unwrap();
        f.iop.receive(0xda02, *now).unwrap(); f.capabilities.retire_leaf(cap, service).unwrap();
        assert!(f.nodes.mesh_members().iter().flatten().all(|member| !member.enabled));
    }
    advance(a, b, now, 24);
    for f in [&mut *a, &mut *b] { assert_eq!(f.nodes.mesh_members().iter().flatten().filter(|member| member.enabled).count(), 1); assert!(f.nodes.domain_proposal(0, *now).is_none()); }
    let before = a.nodes.encode_state().unwrap();
    let mut restored = crate::node::NodeRuntime::new(); restored.restore_state(&before).unwrap();
    assert_eq!(restored.mesh_members().iter().flatten().filter(|member| member.enabled).count(), 1);
    a.nodes.commit_domain_intent(OperationId::NodeLeave, crate::node_request(bid, OperationId::NodeLeave), *now, 99, 100, |_| true).unwrap();
    advance(a, b, now, 24);
    for f in [&mut *a, &mut *b] { assert!(f.nodes.mesh_members().iter().flatten().all(|member| !member.enabled)); assert!(f.nodes.domain_proposal(0, *now).is_none()); }
}

// ------------------------=
// FUNC: restart_pairing_receipts
// DESC: Reconstructs both HOST service instances from persisted objects and boot-fresh entropy, then proves fresh wire-session establishment.
// ------------------=
fn restart_pairing_receipts(a: &mut Fixture, b: &mut Fixture, now: &mut u64, previous: [u8; 16]) {
    let aid = a.nodes.local_id().unwrap();
    let bid = b.nodes.local_id().unwrap();
    let astate = a.nodes.encode_state().unwrap();
    let bstate = b.nodes.encode_state().unwrap();
    let receipt = a.nodes.paired_digest(bid).unwrap();
    assert_eq!(b.nodes.paired_digest(aid), Some(receipt));
    *a = fixture::configured([2, 0, 0, 0, 0, 1], [0xa1; 32]);
    *b = fixture::configured([2, 0, 0, 0, 0, 2], [0xa2; 32]);
    assert_eq!(a.nodes.restore_state(&astate).unwrap(), aid);
    assert_eq!(b.nodes.restore_state(&bstate).unwrap(), bid);
    assert!(a.nodes.sessions().iter().all(Option::is_none));
    assert!(b.nodes.sessions().iter().all(Option::is_none));
    advance(a, b, now, 8);
    let link = a.transport.inspect(a.connection, a.owner).unwrap();
    a.transport.trust.begin(&mut a.nodes, link, 0, true, *now).unwrap();
    advance(a, b, now, 16);
    let handle = a.transport.trust.session(bid).unwrap();
    assert!(b.transport.trust.session(aid).is_some());
    let current = a.nodes.sessions().iter().flatten().find(|session| session.id == handle).unwrap().protocol_reference;
    assert_ne!(previous, current);
    a.transport.trust.send_data(&mut a.nodes, bid, &[7, 8, 9], false, *now).unwrap();
    advance(a, b, now, 4);
    assert_eq!(&b.transport.trust.receive_data().unwrap().bytes[..3], &[7, 8, 9]);
}

// ------------------------=
// FUNC: offline_negotiation
// DESC: Verifies silence cannot grant authority: verified pairing waits only for its own deadline, while fresh session negotiation still closes on discovery loss.
// ------------------=
fn offline_negotiation(handshake: bool) {
    let mut a = fixture::configured([2, 0, 0, 0, 0, 1], [0x81; 32]);
    let mut b = fixture::configured([2, 0, 0, 0, 0, 2], [0x82; 32]);
    let mut now = 0;
    advance(&mut a, &mut b, &mut now, 8);
    let aid = a.nodes.local_id().unwrap();
    let bid = b.nodes.local_id().unwrap();
    let link = a.transport.inspect(a.connection, a.owner).unwrap();
    let tx = a
        .transport
        .trust
        .begin(&mut a.nodes, link, 0, false, now)
        .unwrap();
    advance(&mut a, &mut b, &mut now, 16);
    if handshake {
        let av = a.transport.trust.verification(aid, bid).unwrap();
        let bv = b.transport.trust.verification(bid, aid).unwrap();
        assert_eq!(av.fingerprint, bv.fingerprint);
        a.transport
            .trust
            .confirm(&mut a.nodes, tx, av.code, true, now)
            .unwrap();
        b.transport
            .trust
            .confirm(&mut b.nodes, tx, bv.code, true, now)
            .unwrap();
        advance(&mut a, &mut b, &mut now, 6);
        a.transport
            .trust
            .begin(&mut a.nodes, link, 0, true, now)
            .unwrap();
    }
    for delta in 0..40 {
        for _ in 0..4 {
            a.transport
                .poll(&mut a.nodes, &mut a.network, &a.capabilities, now + delta);
        }
    }
    if !handshake {
        let waiting = a.transport.trust.verification(aid, bid).unwrap();
        assert_eq!(waiting.transaction, tx);
        assert_eq!(a.nodes.discovered_nodes()[0].unwrap().trust, TrustState::PairingPending);
        assert!(a.transport.trust.session(bid).is_none());
        a.transport.poll(&mut a.nodes, &mut a.network, &a.capabilities, waiting.expires);
    }
    assert!(a.transport.trust.verification(aid, bid).is_none());
    assert!(a.transport.trust.session(bid).is_none());
    assert_eq!(
        a.nodes
            .discovered_nodes()
            .iter()
            .flatten()
            .find(|p| p.id == bid)
            .unwrap()
            .trust,
        if handshake {
            TrustState::Trusted
        } else {
            TrustState::Untrusted
        }
    );
    assert!(a
        .nodes
        .audit_records()
        .iter()
        .flatten()
        .any(|record| record.event_type == 0xda04));
}
