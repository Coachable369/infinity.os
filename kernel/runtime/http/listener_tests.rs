use super::*;
use crate::device::EthernetQueue;
use core::cell::Cell;

struct Policy { listen: Cell<bool>, accept: Cell<bool>, io: Cell<bool> }
impl Authority for Policy {
    // ------------------------=
    // FUNC: allows
    // DESC: Authorizes test stream IO independently of listener and acceptance grants.
    // ------------------=
    fn allows(&self, grant: u64, _: Protocol, remote: Endpoint) -> bool {
        self.io.get() && grant == 7 && remote.address[..3] == [10, 0, 0]
    }
}
impl ListenAuthority for Policy {
    // ------------------------=
    // FUNC: allows_listen
    // DESC: Grants only the test service's explicit local listener endpoint.
    // ------------------=
    fn allows_listen(&self, grant: u64, local: Endpoint) -> bool {
        self.listen.get() && grant == 7 && local == Endpoint { address: [10, 0, 0, 2], port: 22 }
    }
    // ------------------------=
    // FUNC: allows_accept
    // DESC: Independently authorizes acceptance of the known test client.
    // ------------------=
    fn allows_accept(&self, grant: u64, local: Endpoint, remote: Endpoint) -> bool {
        self.accept.get() && self.allows_listen(grant, local) && remote.address == [10, 0, 0, 1]
    }
}
// ------------------------=
// FUNC: transfer
// DESC: Exchanges real Ethernet frames while retaining unaccepted queue entries.
// ------------------=
fn transfer(a: &mut EthernetQueue, b: &mut EthernetQueue) {
    while let Some(frame) = a.pending() {
        if !b.ingest(frame) { break; }
        a.transmitted();
    }
}
// ------------------------=
// FUNC: time
// DESC: Supplies deterministic milliseconds for handshake and deadline assertions.
// ------------------=
fn time(ms: i64) -> Instant { Instant::from_millis(ms) }
// ------------------------=
// FUNC: inbound_accept_is_bounded_revocable_and_reusable
// DESC: Verifies actual TCP handshake, endpoint-confined IO, independent acceptance grants, deadlines, and stale handle rejection.
// ------------------=
#[test]
fn inbound_accept_is_bounded_revocable_and_reusable() {
    let policy = Policy { listen: Cell::new(true), accept: Cell::new(true), io: Cell::new(true) };
    let (mut a, mut b) = (EthernetQueue::new(), EthernetQueue::new());
    let (mut sa, mut sb) = ([SocketStorage::EMPTY], [SocketStorage::EMPTY]);
    let (mut ar, mut at, mut br, mut bt) = ([0; 256], [0; 256], [0; 256], [0; 256]);
    let mut client = Reactor::<1>::new(&mut a, [2,0,0,0,0,1], [10,0,0,1], 24, None, 1, time(0), &mut sa).unwrap();
    let mut server = Reactor::<1>::new(&mut b, [2,0,0,0,0,2], [10,0,0,2], 24, None, 2, time(0), &mut sb).unwrap();
    client.add_tcp(&mut ar, &mut at).unwrap();
    server.add_tcp(&mut br, &mut bt).unwrap();
    assert_eq!(server.listen(&policy, 7, 23, time(0), time(1000)), Err(Error::Denied));
    let listener = server.listen(&policy, 7, 22, time(0), time(1000)).unwrap();
    assert_eq!(server.listen(&policy, 7, 22, time(0), time(1000)), Err(Error::Capacity));
    assert_eq!(server.accept(listener, &policy, time(0), time(2000)), Err(Error::WouldBlock));
    let peer = client.open(&policy, 7, Protocol::Tcp, Endpoint { address: [10,0,0,2], port: 22 }, 50000, time(0), time(2000)).unwrap();
    let view = ListenerAuthority { policy: &policy, address: [10,0,0,2] };
    for ms in 0..100 {
        client.poll(&mut a, &policy, time(ms)); transfer(&mut a, &mut b);
        server.poll(&mut b, &view, time(ms)); transfer(&mut b, &mut a);
    }
    let (accepted, remote) = server.accept(listener, &policy, time(100), time(2000)).unwrap();
    assert_eq!(remote, Endpoint { address: [10,0,0,1], port: 50000 });
    assert_eq!(server.accept(listener, &policy, time(100), time(2000)), Err(Error::InvalidHandle));
    assert_eq!(client.send(peer, &policy, time(100), &[4; 512]), Ok(256));
    for ms in 100..200 {
        client.poll(&mut a, &policy, time(ms)); transfer(&mut a, &mut b);
        server.poll(&mut b, &view, time(ms)); transfer(&mut b, &mut a);
    }
    let mut bytes = [0; 256];
    assert_eq!(server.receive(accepted, &policy, time(200), &mut bytes), Ok(256));
    assert_eq!(bytes, [4; 256]);
    policy.io.set(false);
    assert_eq!(server.send(accepted, &policy, time(200), &[9]), Err(Error::Denied));
    server.release(accepted).unwrap();
    assert_eq!(server.receive(accepted, &policy, time(200), &mut bytes), Err(Error::InvalidHandle));
    policy.io.set(true);
    let next = server.listen(&policy, 7, 22, time(200), time(201)).unwrap();
    assert_eq!(server.accept(next, &policy, time(201), time(2000)), Err(Error::Timeout));
    server.release_listener(next).unwrap();
    let next = server.listen(&policy, 7, 22, time(202), time(2000)).unwrap();
    client.release(peer).unwrap();
    client.open(&policy, 7, Protocol::Tcp, Endpoint { address: [10,0,0,2], port: 22 }, 50001, time(202), time(2000)).unwrap();
    for ms in 202..302 {
        client.poll(&mut a, &policy, time(ms)); transfer(&mut a, &mut b);
        server.poll(&mut b, &view, time(ms)); transfer(&mut b, &mut a);
    }
    policy.accept.set(false);
    assert_eq!(server.accept(next, &policy, time(302), time(2000)), Err(Error::Denied));
    server.release_listener(next).unwrap();
    policy.listen.set(false);
    assert_eq!(server.listen(&policy, 7, 22, time(303), time(2000)), Err(Error::Denied));
}
