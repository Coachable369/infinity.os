//! Real native std TCP over packet-exchanging smoltcp endpoints, not host sockets.
use core::{ptr, sync::atomic::{AtomicBool, Ordering}};
use std::{boxed::Box, io::{Read, Write}, net::{TcpStream, Shutdown}, time::Duration};
use infinity_http::{device::EthernetQueue, sockets::{Reactor, Authority, Endpoint, Protocol, Handle, Error}, smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage}, socket::tcp,
    time::Instant, wire::{EthernetAddress, IpCidr, Ipv4Address},
}};
use infinity_servo_runtime_primitives::network::{self, Provider, Status};
static ALLOWED: AtomicBool = AtomicBool::new(true);
static mut SERVICE: *mut Service = ptr::null_mut();
struct Policy;
impl Authority for Policy {
    // ------------------------=
    // FUNC: allows
    // DESC: Limits fixture authority to its explicit wire peer; no Internet or host socket access exists.
    // ------------------=
    fn allows(&self, grant: u64, protocol: Protocol, endpoint: Endpoint) -> bool {
        ALLOWED.load(Ordering::Relaxed) && grant == 7 && protocol == Protocol::Tcp
            && endpoint == Endpoint { address: [10,0,0,2], port: 8000 }
    }
}
struct Service {
    client: Reactor<'static, 1>, a: EthernetQueue, b: EthernetQueue,
    server: Interface, sockets: SocketSet<'static>, peer: SocketHandle,
    channel: Option<(u64, Handle, u16)>, generation: u64,
}
// ------------------------=
// FUNC: now
// DESC: Uses the same actual guest monotonic clock as the native thread provider.
// ------------------=
fn now() -> Instant { Instant::from_micros((super::std_probe::monotonic() / 1000) as i64) }
// ------------------------=
// FUNC: error
// DESC: Maps typed native service errors to the staged std adapter protocol.
// ------------------=
fn error(e: Error) -> i32 {
    match e { Error::WouldBlock => 11, Error::Denied => 13, Error::Capacity => 12,
        Error::Timeout => 110, Error::Disconnected => 104, Error::Cancelled => 4, _ => 22 }
}
// ------------------------=
// FUNC: service
// DESC: Borrows the owner-local fixture only within a nonblocking callback, never over a stack switch.
// ------------------=
unsafe fn service() -> &'static mut Service { &mut *SERVICE }
// ------------------------=
// FUNC: channel
// DESC: Rejects stale ABI identities before passing the reactor's generation-tagged handle onward.
// ------------------=
fn channel(s: &Service, id: u64) -> Result<(Handle, u16), i32> {
    s.channel.filter(|c| c.0 == id).map(|c| (c.1,c.2)).ok_or(22)
}
// ------------------------=
// FUNC: open
// DESC: Allocates a real native connection under fixture capability policy.
// ------------------=
fn open(address: [u8;4], port: u16) -> Result<u64,i32> { unsafe {
    let s = service();
    if s.channel.is_some() { return Err(12); }
    let local_port = 50000 + s.generation as u16;
    let handle = s.client.open(&Policy, 7, Protocol::Tcp, Endpoint { address, port }, local_port,
        now(), now() + infinity_http::smoltcp::time::Duration::from_secs(10)).map_err(error)?;
    s.generation += 1;
    s.channel = Some((s.generation, handle, local_port));
    Ok(s.generation)
} }
// ------------------------=
// FUNC: status
// DESC: Returns actual handshake, readiness and local endpoint information from the native reactor.
// ------------------=
fn status(id: u64) -> Result<Status,i32> { unsafe {
    let s = service(); let (handle, port) = channel(s,id)?;
    let connected = s.client.connected(handle, &Policy, now()).map_err(error)?;
    let ready = s.client.readiness(handle).map_err(error)?;
    Ok(Status { flags: connected as u32 | ((ready.readable as u32) << 1) | ((ready.writable as u32) << 2),
        local: [10,0,0,1], remote: [10,0,0,2], local_port: port, remote_port: 8000 })
} }
// ------------------------=
// FUNC: read
// DESC: Executes real ordered stream receive or peek without consuming data on peek.
// ------------------=
fn read(id: u64, bytes: &mut [u8], peek: bool) -> Result<usize,i32> { unsafe {
    let s = service(); let (handle, _) = channel(s,id)?;
    if peek { s.client.peek(handle, &Policy, now(), bytes).map_err(error) }
    else { s.client.receive(handle, &Policy, now(), bytes).map_err(error) }
} }
// ------------------------=
// FUNC: write
// DESC: Enqueues only available native TCP buffer capacity.
// ------------------=
fn write(id: u64, bytes: &[u8]) -> Result<usize,i32> { unsafe {
    let s = service(); let (handle, _) = channel(s,id)?;
    s.client.send(handle, &Policy, now(), bytes).map_err(error)
} }
// ------------------------=
// FUNC: shutdown
// DESC: Performs supported write-half shutdown and rejects unsupported read-half semantics.
// ------------------=
fn shutdown(id: u64, mode: u32) -> Result<(),i32> { unsafe {
    let s = service(); let (handle, _) = channel(s,id)?;
    if mode != 1 { return Err(95); }
    s.client.shutdown_write(handle, &Policy, now()).map_err(error)
} }
// ------------------------=
// FUNC: close
// DESC: Releases the reactor slot exactly once when the final std stream clone is dropped.
// ------------------=
fn close(id: u64) { unsafe {
    let s = service();
    if let Ok((handle,_)) = channel(s,id) { s.client.release(handle).unwrap(); s.channel = None; }
} }
// ------------------------=
// FUNC: transfer
// DESC: Moves bounded real Ethernet frames between two freestanding stacks.
// ------------------=
fn transfer(from: &mut EthernetQueue, to: &mut EthernetQueue) {
    while let Some(bytes) = from.pending() { if !to.ingest(bytes) { break; } from.transmitted(); }
}
// ------------------------=
// FUNC: pump
// DESC: Drives actual TCP packets and a tiny echo peer outside all std and callback borrows.
// ------------------=
pub fn pump() { unsafe {
    if SERVICE.is_null() { return; }
    let s = service();
    s.client.poll(&mut s.a, &Policy, now()); transfer(&mut s.a, &mut s.b);
    for _ in 0..4 { s.server.poll_ingress_single(now(), &mut s.b, &mut s.sockets); }
    let peer = s.sockets.get_mut::<tcp::Socket>(s.peer);
    if peer.can_recv() && peer.can_send() {
        let mut bytes = [0;256];
        let count = peer.recv_slice(&mut bytes).unwrap();
        assert_eq!(peer.send_slice(&bytes[..count]), Ok(count));
    }
    if peer.state() == tcp::State::CloseWait { peer.close(); }
    s.server.poll_egress(now(), &mut s.b, &mut s.sockets); transfer(&mut s.b, &mut s.a);
} }
// ------------------------=
// FUNC: run
// DESC: Proves real std connect, clone lifetime, ordered IO, peek, timeout and revocation inside the guest.
// ------------------=
pub fn run() { unsafe {
    // An unbound service must fail, not create a host connection.
    assert_eq!(TcpStream::connect(std::net::SocketAddr::from(([10,0,0,2],8000))).unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    let mut a = EthernetQueue::new(); let mut b = EthernetQueue::new();
    let storage = Box::leak(Box::new([SocketStorage::EMPTY]));
    let mut client = Reactor::new(&mut a,[2,0,0,0,0,1],[10,0,0,1],24,None,1,now(),storage).unwrap();
    client.add_tcp(Box::leak(Box::new([0;512])),Box::leak(Box::new([0;512]))).unwrap();
    let mut config = Config::new(EthernetAddress([2,0,0,0,0,2]).into()); config.random_seed = 2;
    let mut server = Interface::new(config,&mut b,now());
    server.update_ip_addrs(|ips| { ips.push(IpCidr::new(Ipv4Address::new(10,0,0,2).into(),24)).unwrap(); });
    let mut sockets = SocketSet::new(&mut Box::leak(Box::new([SocketStorage::EMPTY]))[..]);
    let mut peer = tcp::Socket::new(tcp::SocketBuffer::new(&mut Box::leak(Box::new([0;512]))[..]),
        tcp::SocketBuffer::new(&mut Box::leak(Box::new([0;512]))[..]));
    peer.listen(8000).unwrap();
    let peer = sockets.add(peer);
    SERVICE = Box::into_raw(Box::new(Service { client,a,b,server,sockets,peer,channel:None,generation:0 }));
    let provider = Box::leak(Box::new(Provider { cpu:super::std_probe::cpu, owner:super::std_probe::cpu(),
        open,status,read,write,shutdown,close }));
    assert!(network::install(provider)); assert!(!network::install(provider));
    assert_eq!(TcpStream::connect(std::net::SocketAddr::from(([10,0,0,3],8000))).unwrap_err().kind(),std::io::ErrorKind::PermissionDenied);
    let stream = TcpStream::connect_timeout(&([10,0,0,2],8000).into(), Duration::from_secs(1)).unwrap();
    assert_eq!(stream.peer_addr().unwrap(), ([10,0,0,2],8000).into());
    assert_eq!(stream.local_addr().unwrap(), ([10,0,0,1],50000).into());
    let mut copy = stream.try_clone().unwrap(); drop(stream);
    assert!(service().channel.is_some());
    copy.write_all(&[9,8,7]).unwrap();
    let mut bytes = [0;3]; assert_eq!(copy.peek(&mut bytes).unwrap(),3); assert_eq!(bytes,[9,8,7]);
    bytes.fill(0); copy.read_exact(&mut bytes).unwrap(); assert_eq!(bytes,[9,8,7]);
    copy.set_nonblocking(true).unwrap();
    assert_eq!(copy.read(&mut bytes).unwrap_err().kind(),std::io::ErrorKind::WouldBlock);
    copy.set_nonblocking(false).unwrap(); copy.set_read_timeout(Some(Duration::from_millis(2))).unwrap();
    let before = std::time::Instant::now();
    assert_eq!(copy.read(&mut bytes).unwrap_err().kind(),std::io::ErrorKind::TimedOut);
    assert!(before.elapsed() >= Duration::from_millis(2));
    assert!(copy.set_read_timeout(Some(Duration::ZERO)).is_err());
    assert_eq!(copy.shutdown(Shutdown::Read).unwrap_err().kind(),std::io::ErrorKind::Unsupported);
    ALLOWED.store(false,Ordering::Relaxed);
    assert_eq!(copy.write(&[1]).unwrap_err().kind(),std::io::ErrorKind::PermissionDenied);
    drop(copy); assert!(service().channel.is_none());
    mio_socket();
    #[cfg(feature = "async-probe")]
    async_socket();
} }
// ------------------------=
// FUNC: mio_socket
// DESC: Executes upstream Mio TCP connect and repeated readiness-driven IO against real native TCP packets.
// ------------------=
fn mio_socket() { unsafe {
    ALLOWED.store(true, Ordering::Relaxed);
    let s = service();
    s.sockets.get_mut::<tcp::Socket>(s.peer).abort();
    s.sockets.get_mut::<tcp::Socket>(s.peer).listen(8000).unwrap();
    let mut stream = mio::net::TcpStream::connect(([10,0,0,2],8000).into()).unwrap();
    let mut poll = mio::Poll::new().unwrap();
    let mut events = mio::Events::with_capacity(8);
    poll.registry().register(&mut stream, mio::Token(45), mio::Interest::WRITABLE).unwrap();
    poll.poll(&mut events, Some(Duration::from_secs(1))).unwrap();
    assert_eq!(events.iter().count(),1);
    let event = events.iter().next().unwrap(); assert_eq!(event.token(),mio::Token(45)); assert!(event.is_writable());
    assert_eq!(stream.peer_addr().unwrap(), ([10,0,0,2],8000).into());
    assert!(stream.take_error().unwrap().is_none());
    poll.registry().reregister(&mut stream, mio::Token(46), mio::Interest::READABLE).unwrap();
    for byte in [1u8,2,3] {
        stream.write_all(&[byte]).unwrap();
        poll.poll(&mut events, Some(Duration::from_secs(1))).unwrap();
        assert_eq!(events.iter().count(),1);
        let event = events.iter().next().unwrap(); assert_eq!(event.token(),mio::Token(46)); assert!(event.is_readable());
        let mut out = [0]; assert_eq!(stream.read(&mut out).unwrap(),1); assert_eq!(out,[byte]);
        assert_eq!(stream.read(&mut out).unwrap_err().kind(),std::io::ErrorKind::WouldBlock);
        poll.poll(&mut events, Some(Duration::ZERO)).unwrap(); assert!(events.is_empty());
    }
    drop(stream); assert!(service().channel.is_none());
    poll.poll(&mut events, Some(Duration::ZERO)).unwrap(); assert!(events.is_empty());
} }
// ------------------------=
// FUNC: async_socket
// DESC: Proves pinned Tokio scheduling, timers and async TCP over native std/Mio without a host runtime.
// ------------------=
#[cfg(feature = "async-probe")]
fn async_socket() {
    unsafe {
        let s = service();
        s.sockets.get_mut::<tcp::Socket>(s.peer).abort();
        s.sockets.get_mut::<tcp::Socket>(s.peer).listen(8000).unwrap();
    }
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    runtime.block_on(async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(std::net::SocketAddr::from(([10,0,0,2],8000))).await.unwrap();
        assert_eq!(stream.peer_addr().unwrap(), ([10,0,0,2],8000).into());
        for message in [[11,12,13], [21,22,23]] {
            stream.write_all(&message).await.unwrap();
            let mut bytes = [0;3]; stream.read_exact(&mut bytes).await.unwrap(); assert_eq!(bytes,message);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let mut bytes = [0];
        assert!(tokio::time::timeout(Duration::from_millis(2), stream.read(&mut bytes)).await.is_err());
        let mut native = stream.into_std().unwrap();
        native.set_nonblocking(false).unwrap();
        native.write_all(&[31]).unwrap(); native.read_exact(&mut bytes).unwrap(); assert_eq!(bytes,[31]);
        drop(native);
    });
    unsafe { assert!(service().channel.is_none()); }
}
