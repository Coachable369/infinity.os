use super::*;
use crate::device::EthernetQueue;
use core::cell::Cell;
struct Policy(Cell<bool>);
impl Authority for Policy {
    // ------------------------=
    // FUNC: allows
    // DESC: Grants only the test authority identity and local test subnet while enabled.
    // ------------------=
    fn allows(&self, grant: u64, _: Protocol, remote: Endpoint) -> bool {
        self.0.get() && grant == 7 && remote.address[..3] == [10, 0, 0]
    }
}
// ------------------------=
// FUNC: transfer
// DESC: Transfers actual Ethernet packets between two native stacks without host sockets.
// ------------------=
fn transfer(from: &mut EthernetQueue, to: &mut EthernetQueue) {
    while let Some(bytes) = from.pending() {
        if !to.ingest(bytes) { break; }
        from.transmitted();
    }
}
// ------------------------=
// FUNC: time
// DESC: Converts deterministic milliseconds to the native monotonic transport clock.
// ------------------=
fn time(ms: i64) -> Instant { Instant::from_millis(ms) }
// ------------------------=
// FUNC: endpoint
// DESC: Constructs an explicit test peer endpoint.
// ------------------=
fn endpoint(last: u8, port: u16) -> Endpoint { Endpoint { address: [10, 0, 0, last], port } }
// ------------------------=
// FUNC: tcp_concurrency_backpressure_lifecycle_and_authority
// DESC: Handshakes two streams on one interface, verifies partial IO, FIN, revocation, expiry and slot reuse.
// ------------------=
#[test]
fn tcp_concurrency_backpressure_lifecycle_and_authority() {
    let (mut a, mut b) = (EthernetQueue::new(), EthernetQueue::new());
    let (mut sa, mut sb) = ([SocketStorage::EMPTY, SocketStorage::EMPTY], [SocketStorage::EMPTY, SocketStorage::EMPTY]);
    let (mut ar1, mut at1, mut ar2, mut at2, mut br1, mut bt1, mut br2, mut bt2) =
        ([0; 256], [0; 256], [0; 256], [0; 256], [0; 256], [0; 256], [0; 256], [0; 256]);
    let mut client = Reactor::<2>::new(&mut a, [2,0,0,0,0,1], [10,0,0,1], 24, None, 1, time(0), &mut sa).unwrap();
    let mut server = Reactor::<2>::new(&mut b, [2,0,0,0,0,2], [10,0,0,2], 24, None, 2, time(0), &mut sb).unwrap();
    client.add_tcp(&mut ar1, &mut at1).unwrap(); client.add_tcp(&mut ar2, &mut at2).unwrap();
    server.add_tcp(&mut br1, &mut bt1).unwrap(); server.add_tcp(&mut br2, &mut bt2).unwrap();
    // Peer listens at the wire layer; no browser authority to listen is implied.
    let peers = [server.slots[0].unwrap().socket, server.slots[1].unwrap().socket];
    for (i, peer) in peers.iter().enumerate() { server.sockets.get_mut::<tcp::Socket>(*peer).listen(8000 + i as u16).unwrap(); }
    let policy = Policy(Cell::new(true));
    assert_eq!(client.open(&policy, 8, Protocol::Tcp, endpoint(2,8000), 50000, time(0), time(10000)), Err(Error::Denied));
    let h1 = client.open(&policy, 7, Protocol::Tcp, endpoint(2,8000), 50000, time(0), time(10000)).unwrap();
    assert_eq!(client.open(&policy, 7, Protocol::Tcp, endpoint(2,8001), 50000, time(0), time(10000)), Err(Error::Configuration));
    let h2 = client.open(&policy, 7, Protocol::Tcp, endpoint(2,8001), 50001, time(0), time(10000)).unwrap();
    assert_eq!(client.open(&policy, 7, Protocol::Tcp, endpoint(2,8001), 50002, time(0), time(10000)), Err(Error::Capacity));
    assert_eq!(client.send(h1, &policy, time(0), &[1]), Err(Error::WouldBlock));
    for tick in 0..100 {
        client.poll(&mut a, &policy, time(tick)); transfer(&mut a, &mut b);
        server.poll(&mut b, &policy, time(tick)); transfer(&mut b, &mut a);
    }
    assert_eq!(client.readiness(h1), Ok(Readiness { readable: false, writable: true }));
    assert_eq!(client.send(h1, &policy, time(100), &[42;512]), Ok(256));
    assert_eq!(client.send(h1, &policy, time(100), &[1]), Err(Error::WouldBlock));
    assert!(!client.readiness(h1).unwrap().writable);
    assert_eq!(client.send(h2, &policy, time(100), &[9,8,7]), Ok(3));
    for tick in 100..200 {
        client.poll(&mut a, &policy, time(tick)); transfer(&mut a, &mut b);
        server.poll(&mut b, &policy, time(tick)); transfer(&mut b, &mut a);
    }
    let mut bytes = [0;512];
    assert_eq!(server.sockets.get_mut::<tcp::Socket>(peers[0]).recv_slice(&mut bytes), Ok(256));
    assert_eq!(&bytes[..256], &[42;256]);
    assert_eq!(server.sockets.get_mut::<tcp::Socket>(peers[1]).recv_slice(&mut bytes), Ok(3));
    assert_eq!(&bytes[..3], &[9,8,7]);
    server.sockets.get_mut::<tcp::Socket>(peers[0]).send_slice(&[4,5,6]).unwrap();
    server.sockets.get_mut::<tcp::Socket>(peers[0]).close();
    for tick in 200..300 {
        client.poll(&mut a, &policy, time(tick)); transfer(&mut a, &mut b);
        server.poll(&mut b, &policy, time(tick)); transfer(&mut b, &mut a);
    }
    assert!(client.readiness(h1).unwrap().readable);
    assert_eq!(client.receive(h1, &policy, time(300), &mut bytes), Ok(3));
    assert_eq!(&bytes[..3], &[4,5,6]);
    assert_eq!(client.receive(h1, &policy, time(300), &mut bytes), Ok(0));
    client.shutdown_write(h1, &policy, time(300)).unwrap();
    assert_eq!(client.send(h1, &policy, time(300), &[1]), Err(Error::Disconnected));
    policy.0.set(false);
    assert_eq!(client.receive(h2, &policy, time(300), &mut bytes), Err(Error::Denied));
    assert_eq!(client.readiness(h2), Ok(Readiness { readable: true, writable: true }));
    client.release(h1).unwrap();
    policy.0.set(true);
    let next = client.open(&policy, 7, Protocol::Tcp, endpoint(2,8000), 50002, time(300), time(301)).unwrap();
    assert_ne!(next, h1);
    assert_eq!(client.send(h1, &policy, time(300), &[1]), Err(Error::InvalidHandle));
    assert_eq!(client.receive(next, &policy, time(300), &mut bytes), Err(Error::WouldBlock));
    client.poll(&mut a, &policy, time(301));
    assert_eq!(client.send(next, &policy, time(301), &[1]), Err(Error::Timeout));
    client.release(next).unwrap();
    assert_eq!(client.release(next), Err(Error::InvalidHandle));
}
// ------------------------=
// FUNC: udp_packets_are_atomic_bounded_and_peer_confined
// DESC: Exercises datagrams through real wire packets, oversized payloads, wrong peers, cancellation and reuse.
// ------------------=
#[test]
fn udp_packets_are_atomic_bounded_and_peer_confined() {
    let (mut a, mut b) = (EthernetQueue::new(), EthernetQueue::new());
    let (mut sa, mut sb) = ([SocketStorage::EMPTY], [SocketStorage::EMPTY, SocketStorage::EMPTY]);
    let (mut ar, mut at, mut br, mut bt, mut cr, mut ct) = ([0;64], [0;64], [0;64], [0;64], [0;64], [0;64]);
    let (mut amr, mut amt, mut bmr, mut bmt, mut cmr, mut cmt) =
        ([udp::PacketMetadata::EMPTY;2], [udp::PacketMetadata::EMPTY;2], [udp::PacketMetadata::EMPTY;2],
         [udp::PacketMetadata::EMPTY;2], [udp::PacketMetadata::EMPTY;2], [udp::PacketMetadata::EMPTY;2]);
    let mut client = Reactor::<1>::new(&mut a, [2,0,0,0,0,1], [10,0,0,1], 24, None, 1, time(0), &mut sa).unwrap();
    let mut server = Reactor::<2>::new(&mut b, [2,0,0,0,0,2], [10,0,0,2], 24, None, 2, time(0), &mut sb).unwrap();
    client.add_udp(&mut amr, &mut ar, &mut amt, &mut at).unwrap();
    server.add_udp(&mut bmr, &mut br, &mut bmt, &mut bt).unwrap();
    server.add_udp(&mut cmr, &mut cr, &mut cmt, &mut ct).unwrap();
    let policy = Policy(Cell::new(true));
    let c = client.open(&policy, 7, Protocol::Udp, endpoint(2,53), 50000, time(0), time(10000)).unwrap();
    let s = server.open(&policy, 7, Protocol::Udp, endpoint(1,50000), 53, time(0), time(10000)).unwrap();
    let wrong = server.open(&policy, 7, Protocol::Udp, endpoint(1,50000), 99, time(0), time(10000)).unwrap();
    assert_eq!(client.send(c, &policy, time(0), &[1;65]), Err(Error::MessageTooLarge));
    assert_eq!(client.send(c, &policy, time(0), &[1;64]), Ok(64));
    assert_eq!(client.send(c, &policy, time(0), &[2]), Err(Error::WouldBlock));
    for tick in 0..100 {
        client.poll(&mut a, &policy, time(tick)); transfer(&mut a, &mut b);
        server.poll(&mut b, &policy, time(tick)); transfer(&mut b, &mut a);
    }
    let mut bytes = [0;64];
    assert_eq!(server.receive(s, &policy, time(100), &mut bytes), Ok(64));
    assert_eq!(bytes, [1;64]);
    server.send(wrong, &policy, time(100), &[88,88]).unwrap();
    server.send(s, &policy, time(100), &[9,8,7]).unwrap();
    for tick in 100..200 {
        client.poll(&mut a, &policy, time(tick)); transfer(&mut a, &mut b);
        server.poll(&mut b, &policy, time(tick)); transfer(&mut b, &mut a);
    }
    assert_eq!(client.receive(c, &policy, time(200), &mut bytes), Ok(3));
    assert_eq!(&bytes[..3], &[9,8,7]);
    assert_eq!(client.receive(c, &policy, time(200), &mut bytes), Err(Error::WouldBlock));
    client.cancel(c).unwrap();
    assert_eq!(client.send(c, &policy, time(200), &[]), Err(Error::Cancelled));
    client.release(c).unwrap();
    let next = client.open(&policy, 7, Protocol::Udp, endpoint(2,53), 50000, time(200), time(10000)).unwrap();
    assert_eq!(client.receive(next, &policy, time(200), &mut bytes), Err(Error::WouldBlock));
    assert_eq!(client.readiness(c), Err(Error::InvalidHandle));
    client.send(next, &policy, time(200), &[77]).unwrap();
    policy.0.set(false);
    client.poll(&mut a, &policy, time(201));
    assert!(a.pending().is_none());
    assert_eq!(client.send(next, &policy, time(201), &[1]), Err(Error::Denied));
}
