#[path = "../wire-trust-fixture.rs"]
mod fixture;
use crate::node::types::TrustState;
use fixture::Fixture;
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
                if f.execute_remote { f.iop.execute_remote_node(&mut f.nodes, *now); }
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
// FUNC: offline_negotiation
// DESC: Verifies bounded peer loss during verification and final handshake clears pending authority and never establishes a session.
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
