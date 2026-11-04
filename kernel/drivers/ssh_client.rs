//! Independent native SSH client actor, sharing the NIC without occupying browser sockets.
use core::{cell::RefCell, sync::atomic::{AtomicBool, Ordering}};
use crate::http_transport::{device::EthernetQueue, sockets::{self, Endpoint}, smoltcp::{iface::SocketStorage, time::Instant}};
use crate::runtime::{identity::{StableId, SessionOrigin, SessionState}, execution::SecurityIdentity,
    capability::CapabilityType, network::types as net};
use infinity_ssh::{client::{Connection, State}, Failure, PACKET_BUFFER};

const PORT: u16 = 65535;
const TYPES: [CapabilityType; 3] = [CapabilityType::NetworkConnect, CapabilityType::NetworkSend, CapabilityType::NetworkReceive];
static BUSY: AtomicBool = AtomicBool::new(false);
static mut ACTIVE: Option<Actor> = None;
static mut RESOLVING: Option<Resolution> = None;
/// Public non-secret diagnostic ABI: magic, version, state, RX bytes, TX bytes, session, reserved, reserved.
#[no_mangle]
pub static mut INFINITY_SSH_CLIENT_SNAPSHOT: [u64; 8] = [0x494e46535348434c, 1, 0, 0, 0, 0, 0, 0];
struct Resolution { session: StableId, ticket: super::https::Ticket, caps: [u64; 4], user: [u8; 128], length: usize, port: u16, deadline: u64 }
static mut STORAGE: [SocketStorage<'static>; 1] = [SocketStorage::EMPTY];
static mut TCP_IN: [u8; 8192] = [0; 8192];
static mut TCP_OUT: [u8; 8192] = [0; 8192];
static mut SSH_IN: [u8; PACKET_BUFFER] = [0; PACKET_BUFFER];
static mut SSH_OUT: [u8; PACKET_BUFFER] = [0; PACKET_BUFFER];
struct Actor {
    session: StableId, caps: [u64; 3], peer: Endpoint, link: super::ssh::Link,
    reactor: sockets::Reactor<'static, 1>, device: EthernetQueue, socket: sockets::Handle,
    ssh: Connection<'static>,
}
struct Guard;
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases actor ownership without blocking other services.
    // ------------------=
    fn drop(&mut self) { BUSY.store(false, Ordering::Release); }
}
// ------------------------=
// FUNC: lock
// DESC: Takes nonblocking ownership of the outbound SSH actor.
// ------------------=
fn lock() -> Option<Guard> { (!BUSY.swap(true, Ordering::Acquire)).then_some(Guard) }
struct Policy<'a> { runtime: RefCell<&'a mut crate::runtime::InfinityRuntime>, session: StableId,
    caps: [u64; 3], peer: Endpoint, link: super::ssh::Link, now: u64 }
impl sockets::Authority for Policy<'_> {
    // ------------------------=
    // FUNC: allows
    // DESC: Revalidates local-session authority, exact peer, capabilities and network policy for every operation.
    // ------------------=
    fn allows(&self, grant: u64, protocol: sockets::Protocol, peer: Endpoint) -> bool {
        if grant != self.caps[0] || protocol != sockets::Protocol::Tcp || peer != self.peer { return false; }
        let mut runtime = self.runtime.borrow_mut();
        if !local_session(&runtime.identity, self.session) || super::ssh::link(&runtime, self.now) != Some(self.link) { return false; }
        let owner = SecurityIdentity(self.session.0);
        if !self.caps.iter().zip(TYPES).all(|(id, kind)| runtime.capabilities.validate(*id, owner, kind, 0, 1, 0, self.now).is_ok()) { return false; }
        let local = net::Endpoint { address: net::IpAddress::V4(self.link.address), port: PORT };
        let remote = net::Endpoint { address: net::IpAddress::V4(peer.address), port: peer.port };
        [net::Direction::Inbound, net::Direction::Outbound].iter().all(|direction|
            matches!(runtime.network.policy.evaluate(net::Subject::Context(owner), *direction, Some(2), local, remote,
                net::TransportProtocol::Stream, self.now).action, net::PolicyAction::Allow | net::PolicyAction::AuditOnly))
    }
}
// ------------------------=
// FUNC: local_session
// DESC: Denies remote query sessions and locked or logged-out callers access to outbound connections.
// ------------------=
fn local_session(identity: &crate::runtime::identity::IdentitySystem, session: StableId) -> bool {
    (0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i| identity.session_nth(i))
        .any(|s| s.id == session && s.origin == SessionOrigin::Local && s.state == SessionState::Active
            && identity.console_session_authorized(s.id, s.user))
}
// ------------------------=
// FUNC: begin_host
// DESC: Resolves names through the existing native DNS service before opening the independent SSH socket.
// ------------------=
pub fn begin_host(session: StableId, host: &str, port: u16, user: &str, now: u64) -> Result<(), Failure> {
    if let Ok(address) = host.parse::<core::net::Ipv4Addr>() { return begin(session, Endpoint { address: address.octets(), port }, user, now); }
    let _guard = lock().ok_or(Failure::Capacity)?;
    if user.is_empty() || user.len() > 128 { return Err(Failure::Configuration); }
    unsafe {
        if (&*(&raw const ACTIVE)).is_some() || (&*(&raw const RESOLVING)).is_some() { return Err(Failure::Capacity); }
        let owner = SecurityIdentity(session.0);
        let mut caps = [0; 4];
        let granted = crate::runtime::with_runtime(|runtime| {
            if !local_session(&runtime.identity, session) { return Err(Failure::PermissionDenied); }
            for (i, kind) in [TYPES[0], TYPES[1], TYPES[2], CapabilityType::NetworkResolve].iter().enumerate() {
                match runtime.capabilities.grant(*kind, 0, 1, 0, owner, owner, None, 0) {
                    Ok(id) => caps[i] = id,
                    Err(_) => { for id in caps { if id != 0 { let _ = runtime.capabilities.retire_leaf(id, owner); } } return Err(Failure::Capacity); }
                }
            }
            Ok(())
        }).unwrap_or(Err(Failure::NetworkUnavailable));
        granted?;
        match super::https::socket_begin(owner, caps, host.as_bytes(), None, port, true) {
            Ok(ticket) => {
                let mut name = [0; 128]; name[..user.len()].copy_from_slice(user.as_bytes());
                RESOLVING = Some(Resolution { session, ticket, caps, user: name, length: user.len(), port, deadline: now.saturating_add(30_000) });
                Ok(())
            }
            Err(_) => { crate::runtime::with_runtime(|runtime| { for id in caps { let _ = runtime.capabilities.retire_leaf(id, owner); } }); Err(Failure::NetworkUnavailable) }
        }
    }
}
// ------------------------=
// FUNC: finish_resolution
// DESC: Releases the exact temporary DNS transaction and its capabilities without cancelling another service's request.
// ------------------=
unsafe fn finish_resolution() -> Option<Resolution> {
    let request = (&mut *(&raw mut RESOLVING)).take()?;
    let owner = SecurityIdentity(request.session.0);
    let _ = super::https::cancel_browser(owner, request.ticket);
    let _ = super::https::take_browser(owner, request.ticket);
    crate::runtime::with_runtime(|runtime| { for id in request.caps { let _ = runtime.capabilities.retire_leaf(id, owner); } });
    Some(request)
}
// ------------------------=
// FUNC: begin
// DESC: Opens an explicit user-requested connection with bounded independent socket storage.
// ------------------=
pub fn begin(session: StableId, peer: Endpoint, user: &str, now: u64) -> Result<(), Failure> {
    let _guard = lock().ok_or(Failure::Capacity)?;
    unsafe {
        if (&*(&raw const ACTIVE)).is_some() || (&*(&raw const RESOLVING)).is_some() { return Err(Failure::Capacity); }
        let mut seed = [0; 8]; if !super::ssh_entropy::fill(&mut seed) { return Err(Failure::EntropyUnavailable); }
        crate::runtime::with_runtime(|runtime| {
            if !local_session(&runtime.identity, session) { return Err(Failure::PermissionDenied); }
            let link = super::ssh::link(runtime, now / 1000).ok_or(Failure::NetworkUnavailable)?;
            let owner = SecurityIdentity(session.0);
            let mut caps = [0; 3];
            let result = (|| {
                for (i, kind) in TYPES.iter().enumerate() {
                    caps[i] = runtime.capabilities.grant(*kind, 0, 1, 0, owner, owner, None, 0).map_err(|_| Failure::Capacity)?;
                }
                let mut device = EthernetQueue::new();
                STORAGE = [SocketStorage::EMPTY];
                let clock = Instant::from_millis(now as i64);
                let mut reactor = sockets::Reactor::new(&mut device, link.mac, link.address, link.prefix, link.gateway,
                    u64::from_le_bytes(seed), clock, &mut *(&raw mut STORAGE)).map_err(|_| Failure::Configuration)?;
                reactor.add_tcp(&mut *(&raw mut TCP_IN), &mut *(&raw mut TCP_OUT)).map_err(|_| Failure::Capacity)?;
                let policy = Policy { runtime: RefCell::new(runtime), session, caps, peer, link, now: now / 1000 };
                let socket = reactor.open(&policy, caps[0], sockets::Protocol::Tcp, peer, PORT, clock,
                    Instant::from_millis(now.saturating_add(3_600_000) as i64)).map_err(|_| Failure::PermissionDenied)?;
                let ssh = Connection::new(&mut *(&raw mut SSH_IN), &mut *(&raw mut SSH_OUT), user, now)?;
                ACTIVE = Some(Actor { session, caps, peer, link, reactor, device, socket, ssh });
                INFINITY_SSH_CLIENT_SNAPSHOT = [0x494e46535348434c, 1, 1, 0, 0, session.short(), 0, 0];
                Ok(())
            })();
            if result.is_err() { for id in caps { if id != 0 { let _ = runtime.capabilities.retire_leaf(id, owner); } } }
            result
        }).unwrap_or(Err(Failure::NetworkUnavailable))
    }
}
// ------------------------=
// FUNC: clear
// DESC: Drops crypto state before erasing packet storage and retiring only this connection's grants.
// ------------------=
unsafe fn clear(runtime: &mut crate::runtime::InfinityRuntime) {
    if let Some(actor) = (&mut *(&raw mut ACTIVE)).take() {
        for id in actor.caps { let _ = runtime.capabilities.retire_leaf(id, SecurityIdentity(actor.session.0)); }
        drop(actor);
    }
    use zeroize::Zeroize;
    (&mut *(&raw mut SSH_IN)).zeroize(); (&mut *(&raw mut SSH_OUT)).zeroize();
    (&mut *(&raw mut TCP_IN)).zeroize(); (&mut *(&raw mut TCP_OUT)).zeroize();
    INFINITY_SSH_CLIENT_SNAPSHOT[2] = 0;
}
// ------------------------=
// FUNC: cancel
// DESC: Cancels only the local caller's connection and erases retained packet material.
// ------------------=
pub fn cancel(session: StableId) {
    let Some(_guard) = lock() else { return; };
    unsafe {
        if (&*(&raw const RESOLVING)).as_ref().is_some_and(|a| a.session == session) { finish_resolution(); }
        if (&*(&raw const ACTIVE)).as_ref().is_some_and(|a| a.session == session) {
        crate::runtime::with_runtime(|runtime| clear(runtime));
    } }
}
// ------------------------=
// FUNC: access
// DESC: Rechecks caller identity before each terminal operation or authentication response.
// ------------------=
fn access<T>(session: StableId, action: impl FnOnce(&mut Connection<'static>) -> Result<T, Failure>) -> Result<T, Failure> {
    let _guard = lock().ok_or(Failure::Capacity)?;
    if !crate::runtime::with_runtime(|r| local_session(&r.identity, session)).unwrap_or(false) { return Err(Failure::PermissionDenied); }
    unsafe { let actor = (&mut *(&raw mut ACTIVE)).as_mut().filter(|a| a.session == session).ok_or(Failure::Disconnected)?;
        action(&mut actor.ssh) }
}
// ------------------------=
// FUNC: state
// DESC: Reads progress for the owning terminal.
// ------------------=
pub fn state(session: StableId) -> Option<State> {
    { let _guard = lock()?;
        unsafe { if (&*(&raw const RESOLVING)).as_ref().is_some_and(|a| a.session == session) { return Some(State::Connecting); } }
    }
    access(session, |ssh| Ok(ssh.state())).ok()
}
// ------------------------=
// FUNC: confirm_host
// DESC: Forwards explicit fingerprint approval to the pending protocol decision.
// ------------------=
pub fn confirm_host(session: StableId, key: [u8; 32], now: u64) -> Result<(), Failure> { access(session, |ssh| ssh.confirm_host(key, now)) }
// ------------------------=
// FUNC: password
// DESC: Submits a private password without retaining it in the actor or a log.
// ------------------=
pub fn password(session: StableId, value: &str, now: u64) -> Result<(), Failure> { access(session, |ssh| ssh.password(value, now)) }
// ------------------------=
// FUNC: read
// DESC: Drains bounded remote output for the owning terminal.
// ------------------=
pub fn read(session: StableId, bytes: &mut [u8], stderr: bool) -> Result<usize, Failure> {
    access(session, |ssh| { let n = ssh.read(bytes, stderr)?;
        unsafe { INFINITY_SSH_CLIENT_SNAPSHOT[3] = INFINITY_SSH_CLIENT_SNAPSHOT[3].saturating_add(n as u64); } Ok(n) })
}
// ------------------------=
// FUNC: write
// DESC: Sends terminal input through authenticated channel backpressure.
// ------------------=
pub fn write(session: StableId, bytes: &[u8]) -> Result<usize, Failure> {
    access(session, |ssh| { let n = ssh.write(bytes)?;
        unsafe { INFINITY_SSH_CLIENT_SNAPSHOT[4] = INFINITY_SSH_CLIENT_SNAPSHOT[4].saturating_add(n as u64); } Ok(n) })
}
// ------------------------=
// FUNC: ingest
// DESC: Accepts only ARP and the dedicated outbound tuple, preserving unrelated native connections.
// ------------------=
pub(super) fn ingest(frame: &[u8]) {
    let Some(_guard) = lock() else { return; };
    unsafe { if let Some(actor) = (&mut *(&raw mut ACTIVE)).as_mut() {
        if frame.len() >= 14 && (frame[12..14] == [8, 6]
            || super::ssh::tcp_endpoint(frame, actor.link.address, PORT, true) == Some(actor.peer)) { actor.device.ingest(frame); }
    } }
}
// ------------------------=
// FUNC: poll
// DESC: Advances bounded TCP and SSH work without blocking the UI or browser actor.
// ------------------=
pub(super) fn poll(nic: &mut super::e1000::E1000, now: u64) {
    let Some(guard) = lock() else { return; };
    unsafe {
        if let Some(request) = (&*(&raw const RESOLVING)).as_ref() {
            let result = super::https::socket_result(SecurityIdentity(request.session.0), request.ticket);
            match result {
                Ok(result) if result.state == 7 => {
                    let request = finish_resolution().unwrap(); drop(guard);
                    let user = core::str::from_utf8(&request.user[..request.length]).unwrap_or("");
                    let _ = begin(request.session, Endpoint { address: result.address, port: request.port }, user, now);
                    return;
                }
                Err(super::https::Failure::Busy) => return,
                Err(_) => { finish_resolution(); return; }
                Ok(_) if now >= request.deadline => { finish_resolution(); return; }
                _ => return,
            }
        }
    }
    unsafe { crate::runtime::with_runtime(|runtime| {
        let Some(actor) = (&mut *(&raw mut ACTIVE)).as_mut() else { return; };
        let result = {
            let policy = Policy { runtime: RefCell::new(&mut *runtime), session: actor.session, caps: actor.caps,
                peer: actor.peer, link: actor.link, now: now / 1000 };
            drive(actor, &policy, nic, now)
        };
        if result.is_err() { clear(runtime); }
    }); }
}
// ------------------------=
// FUNC: drive
// DESC: Preserves partial TCP input/output and rejects stale authority before sending queued frames.
// ------------------=
fn drive(actor: &mut Actor, policy: &Policy<'_>, nic: &mut super::e1000::E1000, now: u64) -> Result<(), Failure> {
    use sockets::Authority;
    if !policy.allows(actor.caps[0], sockets::Protocol::Tcp, actor.peer) { return Err(Failure::Revoked); }
    let clock = Instant::from_millis(now as i64);
    actor.reactor.poll(&mut actor.device, policy, clock);
    let mut bytes = [0; 1024];
    match actor.reactor.peek(actor.socket, policy, clock, &mut bytes) {
        Ok(0) => return Err(Failure::Disconnected),
        Ok(n) => { let used = actor.ssh.input(&bytes[..n])?;
            if used > 0 { actor.reactor.receive(actor.socket, policy, clock, &mut bytes[..used]).map_err(|_| Failure::Disconnected)?; } }
        Err(sockets::Error::WouldBlock) => (), Err(_) => return Err(Failure::Disconnected),
    }
    let state = actor.ssh.progress(now)?;
    unsafe { INFINITY_SSH_CLIENT_SNAPSHOT[2] = match state {
        State::Connecting => 1, State::HostKey(_) => 2, State::Password => 3,
        State::Opening => 4, State::Connected => 5, State::Exited(_) => 6, State::Failed(_) => 7,
    }; }
    if !actor.ssh.output().is_empty() {
        match actor.reactor.send(actor.socket, policy, clock, actor.ssh.output()) {
            Ok(n) => actor.ssh.consume_output(n)?, Err(sockets::Error::WouldBlock) => (), Err(_) => return Err(Failure::Disconnected),
        }
    }
    for _ in 0..4 {
        let Some(frame) = actor.device.pending() else { break; };
        if !nic.transmit(frame) { break; } actor.device.transmitted();
    }
    Ok(())
}
