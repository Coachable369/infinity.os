//! Host-only measurement of bounded production transport, not installed storage throughput.
#[path = "../wire-trust-fixture.rs"]
mod fixture;
use fixture::Fixture;
// ------------------------=
// FUNC: step
// DESC: Executes one millisecond opportunity with four-frame NIC bounds and one production poll per node.
// ------------------=
fn step(a: &mut Fixture, b: &mut Fixture, ms: &mut u64) {
    let now = *ms / 1000;
    carry(a, b, now);
    carry(b, a, now);
    for peer in [&mut *a, &mut *b] {
        for _ in 0..4 {
            let Some(p) = peer.network.wire.receive_datagram() else {
                break;
            };
            peer.network
                .connections
                .deliver_datagram(p, &mut peer.network.policy, &peer.capabilities, now)
                .unwrap();
        }
        peer.transport
            .poll(&mut peer.nodes, &mut peer.network, &peer.capabilities, now);
    }
    *ms += 1;
}
// ------------------------=
// FUNC: carry
// DESC: Moves no more than four real Ethernet frames in one directed fixture opportunity.
// ------------------=
fn carry(from: &mut Fixture, to: &mut Fixture, now: u64) {
    for _ in 0..4 {
        let Some(f) = from.network.wire.peek_transmit().copied() else {
            break;
        };
        from.network.wire.complete_transmit();
        let _ = to.network.wire.ingest(&f.bytes[..f.length], now);
    }
}
// ------------------------=
// FUNC: advance
// DESC: Advances only the fixture clock, retaining actual production second-resolution transport gates.
// ------------------=
fn advance(a: &mut Fixture, b: &mut Fixture, ms: &mut u64, seconds: u64) {
    for _ in 0..seconds * 1000 {
        step(a, b, ms)
    }
}
// ------------------------=
// FUNC: measure_established_data_cadence
// DESC: Measures successful authenticated payload delivery with a 1ms caller without modifying runtime budgets.
// ------------------=
#[test]
fn measure_established_data_cadence() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(measure_case)
        .unwrap()
        .join()
        .unwrap();
}
// ------------------------=
// FUNC: established
// DESC: Establishes independent signed trust and an authenticated session through the production transport.
// ------------------=
fn established() -> (Fixture, Fixture, u64) {
    let mut a = fixture::configured([2, 0, 0, 0, 0, 1], [0x91; 32]);
    let mut b = fixture::configured([2, 0, 0, 0, 0, 2], [0x92; 32]);
    let mut ms = 0;
    advance(&mut a, &mut b, &mut ms, 8);
    let aid = a.nodes.local_id().unwrap();
    let bid = b.nodes.local_id().unwrap();
    let link = a.transport.inspect(a.connection, a.owner).unwrap();
    let tx = a
        .transport
        .trust
        .begin(&mut a.nodes, link, 0, false, ms / 1000)
        .unwrap();
    advance(&mut a, &mut b, &mut ms, 16);
    let code = a.transport.trust.verification(aid, bid).unwrap().code;
    a.transport
        .trust
        .confirm(&mut a.nodes, tx, code, true, ms / 1000)
        .unwrap();
    b.transport
        .trust
        .confirm(&mut b.nodes, tx, code, true, ms / 1000)
        .unwrap();
    advance(&mut a, &mut b, &mut ms, 6);
    a.transport
        .trust
        .begin(&mut a.nodes, link, 0, true, ms / 1000)
        .unwrap();
    advance(&mut a, &mut b, &mut ms, 16);
    assert!(a.transport.trust.session(bid).is_some());
    (a, b, ms)
}
// ------------------------=
// FUNC: measure_case
// DESC: Counts exact ordered encrypted payloads over ten virtual seconds after real session establishment.
// ------------------=
fn measure_case() {
    let (mut a, mut b, mut ms) = established();
    let bid = b.nodes.local_id().unwrap();
    let start = ms;
    let mut admitted = 0u64;
    let mut received = 0u64;
    while ms < start + 10_000 {
        let mut payload = [0; 64];
        payload[..8].copy_from_slice(&admitted.to_le_bytes());
        if a.transport
            .trust
            .send_data(&mut a.nodes, bid, &payload, false, ms / 1000)
            .is_ok()
        {
            admitted += 1;
        }
        step(&mut a, &mut b, &mut ms);
        if let Some(data) = b.transport.trust.receive_data() {
            assert_eq!(&data.bytes[..8], &received.to_le_bytes());
            received += 1;
        }
    }
    assert!(received > 10);
    assert!(received <= 160);
    println!("MEASURE production transport: polls=10000 virtual_ms=10000 admitted={admitted} delivered={received} payload_bytes={}",received*64);
}
// ------------------------=
// FUNC: inject
// DESC: Admits an attacker-controlled datagram through real endpoint policy and transport validation without mutating session state.
// ------------------=
fn inject(b: &mut Fixture, packet: crate::runtime::network::types::Packet, now: u64) {
    let datagram = crate::runtime::network::wire::Datagram {
        source: [10, 42, 0, 1],
        destination: [10, 42, 0, 2],
        source_port: 49152,
        destination_port: 49152,
        bytes: packet.bytes,
        length: packet.length as usize,
    };
    b.network
        .connections
        .deliver_datagram(datagram, &mut b.network.policy, &b.capabilities, now)
        .unwrap();
    b.transport
        .poll(&mut b.nodes, &mut b.network, &b.capabilities, now);
}
// ------------------------=
// FUNC: authenticated_budget_rejects_forgery_and_unknown_sessions
// DESC: Runs bounded adversarial transport checks with explicit fixture stack storage.
// ------------------=
#[test]
fn authenticated_budget_rejects_forgery_and_unknown_sessions() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(adversarial_case)
        .unwrap()
        .join()
        .unwrap();
}
// ------------------------=
// FUNC: adversarial_case
// DESC: Forged tags consume the DATA budget, unknown sessions consume the control budget, and valid delivery recovers without accepting replay.
// ------------------=
fn adversarial_case() {
    use crate::runtime::node::types::NodeError;
    let (mut a, mut b, ms) = established();
    let now = ms / 1000 + 1;
    let bid = b.nodes.local_id().unwrap();
    a.transport
        .trust
        .send_data(&mut a.nodes, bid, &[0x39; 64], false, now)
        .unwrap();
    let valid = a.transport.trust.outgoing(a.connection, now).unwrap();
    a.transport.trust.sent(a.connection);
    let mut forged = valid;
    forged.bytes[valid.length as usize - 1] ^= 1;
    for _ in 0..16 {
        let before = b.transport.rejected_packets;
        inject(&mut b, forged, now);
        assert_eq!(b.transport.rejected_packets, before + 1);
        assert_ne!(b.transport.last_error, Some(NodeError::ResourceLimit));
        assert!(b.transport.trust.receive_data().is_none());
    }
    inject(&mut b, forged, now);
    assert_eq!(b.transport.last_error, Some(NodeError::ResourceLimit));
    let mut unknown = valid;
    unknown.bytes[80] ^= 1;
    inject(&mut b, unknown, now + 1);
    assert_ne!(b.transport.last_error, Some(NodeError::ResourceLimit));
    inject(&mut b, unknown, now + 1);
    assert_eq!(b.transport.last_error, Some(NodeError::ResourceLimit));
    assert!(b.transport.trust.receive_data().is_none());
    inject(&mut b, valid, now + 2);
    let delivered = b.transport.trust.receive_data().unwrap();
    assert_eq!(&delivered.bytes[..64], &[0x39; 64]);
    inject(&mut b, valid, now + 2);
    assert!(b.transport.trust.receive_data().is_none());
    assert_eq!(b.transport.last_error, Some(NodeError::ReplayDetected));
}
// ------------------------=
// FUNC: backpressure_preserves_unsent_authenticated_payload
// DESC: Runs queue-pressure behavior using the production bounded network and transport queues.
// ------------------=
#[test]
fn backpressure_preserves_unsent_authenticated_payload() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(backpressure_case)
        .unwrap()
        .join()
        .unwrap();
}
// ------------------------=
// FUNC: backpressure_case
// DESC: Sustains a full transmit queue beyond retry intervals and verifies exact delivery once capacity returns.
// ------------------=
fn backpressure_case() {
    use crate::runtime::network::wire::WireError;
    let (mut a, mut b, mut ms) = established();
    let now = ms / 1000;
    let bid = b.nodes.local_id().unwrap();
    while a.network.wire.peek_transmit().is_some() {
        a.network.wire.complete_transmit();
    }
    for _ in 0..8 {
        a.network
            .wire
            .send([10, 42, 0, 2], [10, 42, 0, 2], 49152, 49152, &[0], now)
            .unwrap();
    }
    assert_eq!(
        a.network
            .wire
            .send([10, 42, 0, 2], [10, 42, 0, 2], 49152, 49152, &[0], now),
        Err(WireError::QueueFull)
    );
    a.transport
        .trust
        .send_data(&mut a.nodes, bid, &[0x75; 64], false, now)
        .unwrap();
    for offset in 0..20_000 {
        a.transport.poll(
            &mut a.nodes,
            &mut a.network,
            &a.capabilities,
            now + offset / 1000,
        );
    }
    assert!(a
        .transport
        .trust
        .send_data(&mut a.nodes, bid, &[0x11], false, now + 20)
        .is_err());
    while a.network.wire.peek_transmit().is_some() {
        a.network.wire.complete_transmit();
    }
    ms += 20_000;
    advance(&mut a, &mut b, &mut ms, 2);
    let data = b.transport.trust.receive_data().unwrap();
    assert_eq!(&data.bytes[..64], &[0x75; 64]);
    assert!(b.transport.trust.receive_data().is_none());
}
// ------------------------=
// FUNC: confirmation_survives_transmit_backpressure
// DESC: Reproduces accepted local consent while both NIC queues remain full beyond the old retransmission budget.
// ------------------=
#[test]
fn confirmation_survives_transmit_backpressure() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(confirmation_backpressure_case)
        .unwrap()
        .join()
        .unwrap();
}
// ------------------------=
// FUNC: confirmation_backpressure_case
// DESC: Requires both protected confirmations to reach the peer once network capacity returns; no trust or deadline bypass is used.
// ------------------=
fn confirmation_backpressure_case() {
    let mut a = fixture::configured([2, 0, 0, 0, 0, 1], [0xa1; 32]);
    let mut b = fixture::configured([2, 0, 0, 0, 0, 2], [0xa2; 32]);
    let mut ms = 0;
    advance(&mut a, &mut b, &mut ms, 8);
    let aid = a.nodes.local_id().unwrap();
    let bid = b.nodes.local_id().unwrap();
    let link = a.transport.peer_link(bid).unwrap();
    let tx = a
        .transport
        .trust
        .begin(&mut a.nodes, link, 0, false, ms / 1000)
        .unwrap();
    advance(&mut a, &mut b, &mut ms, 16);
    let code = a.transport.trust.verification(aid, bid).unwrap().code;
    for (p, destination) in [(&mut a, [10, 42, 0, 2]), (&mut b, [10, 42, 0, 1])] {
        while p.network.wire.peek_transmit().is_some() {
            p.network.wire.complete_transmit();
        }
        for _ in 0..8 {
            p.network
                .wire
                .send(destination, destination, 49152, 49152, &[0], ms / 1000)
                .unwrap();
        }
        p.transport
            .trust
            .confirm(&mut p.nodes, tx, code, true, ms / 1000)
            .unwrap();
    }
    for offset in 0..20_000 {
        for p in [&mut a, &mut b] {
            p.transport.poll(
                &mut p.nodes,
                &mut p.network,
                &p.capabilities,
                (ms + offset) / 1000,
            );
        }
    }
    for p in [&mut a, &mut b] {
        while p.network.wire.peek_transmit().is_some() {
            p.network.wire.complete_transmit();
        }
    }
    ms += 20_000;
    advance(&mut a, &mut b, &mut ms, 10);
    for (p, id) in [(&a, bid), (&b, aid)] {
        assert_eq!(
            p.nodes
                .discovered_nodes()
                .iter()
                .flatten()
                .find(|n| n.id == id)
                .unwrap()
                .trust,
            crate::runtime::node::types::TrustState::Trusted
        );
    }
}
// ------------------------=
// FUNC: four_link_data_fairness
// DESC: Verifies all four production link slots progress under simultaneous authenticated traffic.
// ------------------=
#[test]
fn four_link_data_fairness() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(fairness_case)
        .unwrap()
        .join()
        .unwrap();
}
// ------------------------=
// FUNC: mesh_step
// DESC: Pumps five independent network instances with at most four NIC frames and one round-robin transport operation each.
// ------------------=
fn mesh_step(peers: &mut [Fixture; 5], ms: &mut u64) {
    let now = *ms / 1000;
    for sender in 0..5 {
        for _ in 0..4 {
            let Some(frame) = peers[sender].network.wire.peek_transmit().copied() else {
                break;
            };
            peers[sender].network.wire.complete_transmit();
            for recipient in 0..5 {
                if recipient != sender {
                    let _ = peers[recipient]
                        .network
                        .wire
                        .ingest(&frame.bytes[..frame.length], now);
                }
            }
        }
    }
    for peer in peers {
        for _ in 0..4 {
            let Some(p) = peer.network.wire.receive_datagram() else {
                break;
            };
            peer.network
                .connections
                .deliver_datagram(p, &mut peer.network.policy, &peer.capabilities, now)
                .unwrap();
        }
        peer.transport
            .poll(&mut peer.nodes, &mut peer.network, &peer.capabilities, now);
    }
    *ms += 1;
}
// ------------------------=
// FUNC: mesh_advance
// DESC: Advances bounded host scheduler opportunities for real discovery and protected pairing.
// ------------------=
fn mesh_advance(peers: &mut [Fixture; 5], ms: &mut u64, seconds: u64) {
    for _ in 0..seconds * 1000 {
        mesh_step(peers, ms);
    }
}
// ------------------------=
// FUNC: fairness_case
// DESC: Establishes four independent authorized sessions and checks exact per-link ordered payloads without starvation or exceeding budgets.
// ------------------=
fn fairness_case() {
    let mut peers = core::array::from_fn(|index| {
        let number = index as u8 + 1;
        let others: Vec<u8> = (1..=5).filter(|peer| *peer != number).collect();
        fixture::configured_peers([2, 0, 0, 0, 0, number], [number + 80; 32], &others)
    });
    let mut ms = 0;
    mesh_advance(&mut peers, &mut ms, 30);
    let ids = core::array::from_fn::<_, 5, _>(|i| peers[i].nodes.local_id().unwrap());
    for target in 1..5 {
        let link = peers[0].transport.peer_link(ids[target]).unwrap();
        let a = &mut peers[0];
        let tx = a
            .transport
            .trust
            .begin(&mut a.nodes, link, 0, false, ms / 1000)
            .unwrap();
        mesh_advance(&mut peers, &mut ms, 16);
        let code = peers[0]
            .transport
            .trust
            .verification(ids[0], ids[target])
            .unwrap()
            .code;
        for i in [0, target] {
            let p = &mut peers[i];
            p.transport
                .trust
                .confirm(&mut p.nodes, tx, code, true, ms / 1000)
                .unwrap();
        }
        mesh_advance(&mut peers, &mut ms, 6);
        let a = &mut peers[0];
        a.transport
            .trust
            .begin(&mut a.nodes, link, 0, true, ms / 1000)
            .unwrap();
        mesh_advance(&mut peers, &mut ms, 16);
        assert!(peers[0].transport.trust.session(ids[target]).is_some());
    }
    let mut sent = [0u64; 5];
    let mut received = [0u64; 5];
    let start = ms;
    while ms < start + 10_000 {
        for target in 1..5 {
            let a = &mut peers[0];
            let mut bytes = [target as u8; 64];
            bytes[..8].copy_from_slice(&sent[target].to_le_bytes());
            if a.transport
                .trust
                .send_data(&mut a.nodes, ids[target], &bytes, false, ms / 1000)
                .is_ok()
            {
                sent[target] += 1;
            }
        }
        mesh_step(&mut peers, &mut ms);
        for target in 1..5 {
            if let Some(data) = peers[target].transport.trust.receive_data() {
                assert_eq!(&data.bytes[..8], &received[target].to_le_bytes());
                assert_eq!(&data.bytes[8..64], &[target as u8; 56]);
                received[target] += 1;
            }
        }
    }
    for n in &received[1..] {
        assert!(*n > 10 && *n <= 160, "{received:?}");
    }
    assert!(received[1..].iter().max().unwrap() - received[1..].iter().min().unwrap() <= 1);
}
