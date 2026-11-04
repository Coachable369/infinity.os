//! Native SSH actor. Only the shared NIC pump owns its bounded packet queues.
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::http_transport::{device::EthernetQueue, sockets::{self, listener::{Listener, ListenAuthority, ListenerAuthority}},
    smoltcp::{iface::SocketStorage, time::Instant}};
use crate::runtime::{capability::{CapabilityManager, CapabilityType}, execution::SecurityIdentity,
    identity::{IdentitySystem, StableId, SessionState, SESSION_IDENTITY_MANAGE}, network::{policy::{PolicyEngine, NetworkPolicyRule, EndpointSelector}, types as net}};
use infinity_ssh::{Configuration, Diagnostic, Failure, HostKey, IdentityAuthority, Principal, Transport, MAX_CONNECTIONS, PACKET_BUFFER};

const OWNER: SecurityIdentity = SecurityIdentity(crate::storage::object::SSH_SERVICE_OWNER.0);
const GRANT: u64 = 0x53534831;
const SOCKET_BYTES: usize = 4096;
const TYPES: [CapabilityType; 4] = [CapabilityType::NetworkListen, CapabilityType::NetworkAccept, CapabilityType::NetworkSend, CapabilityType::NetworkReceive];
static BUSY: AtomicBool = AtomicBool::new(false);
static mut INITIALIZED: bool = false;
static mut BUFFERS_DIRTY: bool = false;
static mut NEXT_START: u64 = 0;
static mut CONFIG: Configuration = Configuration { enabled: false, port: 22, max_connections: 2 };
static mut HOST: Option<HostKey> = None;
static mut STATE: Option<State> = None;
static mut GRANTS: [u64; 4] = [0; 4];
static mut RULES: [Option<net::PolicyId>; 2] = [None; 2];
static mut STORAGE: [SocketStorage<'static>; MAX_CONNECTIONS] = [const { SocketStorage::EMPTY }; MAX_CONNECTIONS];
static mut TCP_RX: [[u8; SOCKET_BYTES]; MAX_CONNECTIONS] = [[0; SOCKET_BYTES]; MAX_CONNECTIONS];
static mut TCP_TX: [[u8; SOCKET_BYTES]; MAX_CONNECTIONS] = [[0; SOCKET_BYTES]; MAX_CONNECTIONS];
static mut SSH_RX: [[u8; PACKET_BUFFER]; MAX_CONNECTIONS] = [[0; PACKET_BUFFER]; MAX_CONNECTIONS];
static mut SSH_TX: [[u8; PACKET_BUFFER]; MAX_CONNECTIONS] = [[0; PACKET_BUFFER]; MAX_CONNECTIONS];
static mut STATUS: Status = Status { enabled: false, listening: false, port: 22, active: 0, accepted: 0,
    authenticated: 0, denied: 0, closed: 0, last_failure: None, peak_protocol_ns: 0, protocol_steps: 0 };
/// Public diagnostic ABI v1: magic, version, sequence, enabled, listening, port,
/// active, accepted, authenticated, denied, closed, failure, quota, grants,
/// backlog slots, peak protocol nanoseconds, protocol steps, reserved, reserved,
/// matching sequence. Contains no packet or credential material.
#[no_mangle]
pub static mut INFINITY_SSH_DIAGNOSTIC_SNAPSHOT: [u64; 20] = [0; 20];

#[derive(Clone, Copy, Debug)]
pub struct Status {
    pub enabled: bool, pub listening: bool, pub port: u16, pub active: u8,
    pub accepted: u64, pub authenticated: u64, pub denied: u64, pub closed: u64,
    pub last_failure: Option<Failure>,
    pub peak_protocol_ns: u64, pub protocol_steps: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Link { address: [u8; 4], prefix: u8, mac: [u8; 6], gateway: Option<[u8; 4]>, generation: u64 }
struct Session { tcp: sockets::Handle, ssh: Transport<'static>, native: Option<crate::runtime::identity::Session>,
    terminal: crate::runtime::console_terminal::Terminal, failure: Option<Failure>, completed: u64, last_operation: u64 }
#[no_mangle]
pub static mut INFINITY_SSH_CONSOLE_SNAPSHOT: [u64; 48] = [0; 48];
static mut COMMANDS: u64 = 0;
static mut INTERRUPTS: u64 = 0;
static mut COMMAND_DENIALS: u64 = 0;
static mut PEAK_EXECUTION_NS: u64 = 0;
static mut RETIRING: [Option<(StableId, StableId)>; MAX_CONNECTIONS] = [None; MAX_CONNECTIONS];
struct State {
    reactor: sockets::Reactor<'static, MAX_CONNECTIONS>, device: EthernetQueue,
    listeners: [Option<Listener>; MAX_CONNECTIONS], sessions: [Option<Session>; MAX_CONNECTIONS],
    link: Link, cursor: usize,
}
struct Guard;
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases actor ownership on all return paths without a blocking lock.
    // ------------------=
    fn drop(&mut self) {
        unsafe { publish_status(); }
        BUSY.store(false, Ordering::Release);
    }
}
// ------------------------=
// FUNC: publish_status
// DESC: Publishes a coherent content-free service snapshot for installed-system diagnostics and resource accounting.
// ------------------=
unsafe fn publish_status() {
    let out = &raw mut INFINITY_SSH_DIAGNOSTIC_SNAPSHOT;
    let sequence = (*out)[2].wrapping_add(2) & !1;
    core::ptr::write_volatile(core::ptr::addr_of_mut!((*out)[2]), sequence | 1);
    core::sync::atomic::fence(Ordering::Release);
    let values = [0x494e465353483031, 1, sequence | 1, STATUS.enabled as u64,
        STATUS.listening as u64, STATUS.port as u64, STATUS.active as u64, STATUS.accepted,
        STATUS.authenticated, STATUS.denied, STATUS.closed,
        STATUS.last_failure.map(|failure| failure as u64 + 1).unwrap_or(0), CONFIG.max_connections as u64,
        (&*(&raw const GRANTS)).iter().filter(|id| **id != 0).count() as u64,
        (&*(&raw const STATE)).as_ref().map(|state| state.listeners.iter().flatten().count() as u64).unwrap_or(0),
        STATUS.peak_protocol_ns, STATUS.protocol_steps, 0, 0, sequence];
    for index in 0..20 {
        if index != 2 { core::ptr::write_volatile(core::ptr::addr_of_mut!((*out)[index]), values[index]); }
    }
    core::sync::atomic::fence(Ordering::Release);
    core::ptr::write_volatile(core::ptr::addr_of_mut!((*out)[2]), sequence);
    let console = &raw mut INFINITY_SSH_CONSOLE_SNAPSHOT;
    core::ptr::write_volatile(core::ptr::addr_of_mut!((*console)[2]), sequence | 1);
    core::sync::atomic::fence(Ordering::Release);
    let mut words = [0; 48];
    words[..7].copy_from_slice(&[0x494e465353484332, 1, sequence | 1, COMMANDS, INTERRUPTS, COMMAND_DENIALS, PEAK_EXECUTION_NS]);
    if let Some(state) = (&*(&raw const STATE)).as_ref() {
        for (index, session) in state.sessions.iter().enumerate() {
            if let Some(session) = session {
                if let Some(native) = session.native {
                    let terminal = &session.terminal;
                    words[7] += (!terminal.finished) as u64;
                    let flags = terminal.started as u64 | (terminal.interactive as u64) << 1 | (terminal.pty as u64) << 2
                        | (terminal.pending as u64) << 3 | (terminal.finished as u64) << 4;
                    words[8 + index*9..17 + index*9].copy_from_slice(&[native.id.short(), native.user.short(),
                        terminal.columns as u64, terminal.rows as u64, session.completed, terminal.status as u64,
                        flags, session.last_operation, terminal.history_count() as u64]);
                }
            }
        }
    }
    words[47] = sequence;
    for index in 0..48 { if index != 2 { core::ptr::write_volatile(core::ptr::addr_of_mut!((*console)[index]), words[index]); } }
    core::sync::atomic::fence(Ordering::Release);
    core::ptr::write_volatile(core::ptr::addr_of_mut!((*console)[2]), sequence);
}
// ------------------------=
// FUNC: lock
// DESC: Serializes configuration, ingress and cooperative polling without spinning.
// ------------------=
fn lock() -> Option<Guard> { (!BUSY.swap(true, Ordering::Acquire)).then_some(Guard) }

struct Policy<'a> {
    caps: &'a CapabilityManager, identity: &'a IdentitySystem, network: RefCell<&'a mut PolicyEngine>,
    grants: [u64; 4], local: sockets::Endpoint, now: u64,
}
impl Policy<'_> {
    // ------------------------=
    // FUNC: valid
    // DESC: Validates all service-owned, nondelegable listener and IO capabilities.
    // ------------------=
    fn valid(&self) -> bool {
        self.grants.iter().zip(TYPES).all(|(id, kind)| self.caps.validate(*id, OWNER, kind, self.local.port as u64, 1, 0, self.now).is_ok())
    }
}
impl sockets::Authority for Policy<'_> {
    // ------------------------=
    // FUNC: allows
    // DESC: Rechecks both native policy directions and revocable service authority before TCP IO or egress.
    // ------------------=
    fn allows(&self, grant: u64, protocol: sockets::Protocol, remote: sockets::Endpoint) -> bool {
        if grant != GRANT || protocol != sockets::Protocol::Tcp || !self.valid() { return false; }
        let local = net::Endpoint { address: net::IpAddress::V4(self.local.address), port: self.local.port };
        let remote = net::Endpoint { address: net::IpAddress::V4(remote.address), port: remote.port };
        [net::Direction::Inbound, net::Direction::Outbound].iter().all(|direction|
            matches!(self.network.borrow_mut().evaluate(net::Subject::Context(OWNER), *direction, Some(2), local, remote,
                net::TransportProtocol::Stream, self.now).action, net::PolicyAction::Allow | net::PolicyAction::AuditOnly))
    }
}
impl ListenAuthority for Policy<'_> {
    // ------------------------=
    // FUNC: allows_listen
    // DESC: Confines listening to the current configured address and port.
    // ------------------=
    fn allows_listen(&self, grant: u64, local: sockets::Endpoint) -> bool { grant == GRANT && local == self.local && self.valid() }
    // ------------------------=
    // FUNC: allows_accept
    // DESC: Confines accepted peers with the same capability and policy checks as stream IO.
    // ------------------=
    fn allows_accept(&self, grant: u64, local: sockets::Endpoint, remote: sockets::Endpoint) -> bool {
        self.allows_listen(grant, local) && sockets::Authority::allows(self, grant, sockets::Protocol::Tcp, remote)
    }
}
impl IdentityAuthority for Policy<'_> {
    // ------------------------=
    // FUNC: authorized_key
    // DESC: Resolves offers through IdentitySystem without creating local or administrative sessions.
    // ------------------=
    fn authorized_key(&self, name: &[u8], key: &[u8; 32]) -> Option<Principal> {
        self.identity.ssh_key_owner(name, key).map(|user| Principal { user: user.0, public_key: *key })
    }
    // ------------------------=
    // FUNC: still_authorized
    // DESC: Immediately reflects disabled accounts and revoked credentials in established SSH transports.
    // ------------------=
    fn still_authorized(&self, principal: Principal) -> bool { self.identity.ssh_key_active(StableId(principal.user), &principal.public_key) }
}

// ------------------------=
// FUNC: status
// DESC: Returns structured operational counters without private keys, credentials or untrusted packet text.
// ------------------=
pub fn status() -> Option<Status> { let _guard = lock()?; Some(unsafe { STATUS }) }

// ------------------------=
// FUNC: host_fingerprint
// DESC: Exposes only the standard public host fingerprint for local verification against OpenSSH.
// ------------------=
pub fn host_fingerprint() -> Option<[u8; 50]> {
    let _guard = lock()?;
    match unsafe { (&*(&raw const HOST)).as_ref()? } {
        HostKey::Ed25519(key) => Some(infinity_ssh::management::fingerprint(&key.verifying_key().to_bytes())),
        _ => None,
    }
}

// ------------------------=
// FUNC: manage_key
// DESC: Persists local account-key administration through IdentitySystem; authentication remains a separate protocol operation.
// ------------------=
pub fn manage_key(session: StableId, name: &[u8], key: [u8; 32], revoke: bool) -> Result<(), crate::runtime::identity::IdentityError> {
    use crate::runtime::identity::IdentityError;
    crate::runtime::with_runtime(|runtime| {
        let user = runtime.identity.user_by_handle(name).ok_or(IdentityError::NotFound)?;
        let now = crate::ui::performance::monotonic_ns().unwrap_or(0) / 1_000_000_000;
        runtime.identity.manage_ssh_key_durable(session, user.id, key, revoke, now, |bytes| {
            crate::storage::identity_state_commit(bytes).map(|_| ()).map_err(|error| match error {
                crate::storage::object::ObjectError::CommitUncertain => IdentityError::CommitUncertain,
                _ => IdentityError::InvalidState,
            })
        }).map(|_| ())
    }).unwrap_or(Err(IdentityError::InvalidState))
}

// ------------------------=
// FUNC: initialize
// DESC: Starts the configured native service only after entropy and installed object storage are available.
// ------------------=
pub(super) fn initialize(info: &crate::boot_info::BootInfo) {
    let Some(_guard) = lock() else { return; };
    unsafe {
        if INITIALIZED { return; }
        INITIALIZED = true;
    }
    let entropy = super::ssh_entropy::initialize(&info.firmware_entropy, info.firmware_entropy_valid == 1);
    match crate::storage::ssh::load_configuration() {
        Ok(config) => unsafe {
            CONFIG = config;
            STATUS.enabled = config.enabled;
            STATUS.port = config.port;
            if config.enabled {
                if !entropy { STATUS.last_failure = Some(Failure::EntropyUnavailable); return; }
                if let Err(failure) = prepare(config) { STATUS.last_failure = Some(failure); }
            }
        },
        Err(failure) => unsafe { STATUS.last_failure = Some(failure); },
    }
}

// ------------------------=
// FUNC: configure
// DESC: Authorizes local administration, persists configuration and then starts or stops the transport-only service.
// ------------------=
pub fn configure(session: StableId, config: Configuration) -> Result<(), Failure> {
    let config = config.validate()?;
    let _guard = lock().ok_or(Failure::Capacity)?;
    let allowed = crate::runtime::with_runtime(|runtime| runtime.identity.accounts(session).is_ok()
        && (0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i| runtime.identity.session_nth(i))
        .any(|s| s.id == session && s.state == SessionState::Active && s.capabilities & SESSION_IDENTITY_MANAGE != 0
            && runtime.identity.is_administrator(s.user))).unwrap_or(false);
    if !allowed { return Err(Failure::PermissionDenied); }
    // Persist first: an uncertain configuration commit must never activate a listener.
    if let Err(failure) = crate::storage::ssh::save_configuration(config) {
        unsafe { stop(); STATUS.enabled = false; STATUS.last_failure = Some(failure); }
        return Err(failure);
    }
    unsafe {
        stop();
        CONFIG = config;
        NEXT_START = 0;
        STATUS.enabled = config.enabled;
        STATUS.port = config.port;
        STATUS.last_failure = None;
        if config.enabled {
            if let Err(failure) = prepare(config) { STATUS.last_failure = Some(failure); return Err(failure); }
        }
    }
    Ok(())
}

// ------------------------=
// FUNC: prepare
// DESC: Loads a protected persistent host key and acquires exactly four native network capabilities and two scoped rules.
// ------------------=
unsafe fn prepare(config: Configuration) -> Result<(), Failure> {
    let mut probe = [0; 1];
    if !super::ssh_entropy::fill(&mut probe) { return Err(Failure::EntropyUnavailable); }
    let key = infinity_ssh::hostkey::load_or_create(&mut crate::storage::ssh::HostStore, super::ssh_entropy::fill)?;
    let result = crate::runtime::with_runtime(|runtime| {
        for (index, kind) in TYPES.iter().enumerate() {
            GRANTS[index] = runtime.capabilities.grant(*kind, config.port as u64, 1, 0, OWNER, OWNER, None, 0)
                .map_err(|_| Failure::Capacity)?;
        }
        for (index, direction) in [net::Direction::Inbound, net::Direction::Outbound].iter().enumerate() {
            RULES[index] = Some(runtime.network.policy.create(NetworkPolicyRule {
                id: 0, subject: net::Subject::Context(OWNER), direction: *direction, interface_id: Some(2),
                local: EndpointSelector { port: Some(config.port), protocol: Some(net::TransportProtocol::Stream), ..EndpointSelector::any() },
                remote: EndpointSelector::any(), action: net::PolicyAction::Allow, priority: 500,
                logging: net::LoggingMode::Denials, enabled: true, expires_at: None, policy_source: GRANT,
            }).map_err(|_| Failure::Capacity)?);
        }
        Ok(())
    }).unwrap_or(Err(Failure::NetworkUnavailable));
    if result.is_err() { stop(); return result; }
    HOST = Some(key);
    Ok(())
}

// ------------------------=
// FUNC: clear_connections
// DESC: Drops cryptographic state before zeroing reusable packet buffers and releasing all network slots.
// ------------------=
unsafe fn clear_connections() {
    if (&*(&raw const STATE)).is_none() && !BUFFERS_DIRTY { return; }
    if let Some(state) = (&mut *(&raw mut STATE)).as_mut() {
        for session in state.sessions.iter_mut().flatten() { retire_session(session); }
    }
    STATE = None;
    STORAGE = [const { SocketStorage::EMPTY }; MAX_CONNECTIONS];
    STATUS.listening = false;
    STATUS.closed = STATUS.closed.saturating_add(STATUS.active as u64);
    STATUS.active = 0;
    use zeroize::Zeroize;
    (&mut *(&raw mut TCP_RX)).zeroize(); (&mut *(&raw mut TCP_TX)).zeroize();
    (&mut *(&raw mut SSH_RX)).zeroize(); (&mut *(&raw mut SSH_TX)).zeroize();
    BUFFERS_DIRTY = false;
}
// ------------------------=
// FUNC: stop
// DESC: Removes only SSH-owned rules and nondelegated capabilities, preserving every other network consumer.
// ------------------=
unsafe fn stop() {
    clear_connections();
    HOST = None;
    crate::runtime::with_runtime(|runtime| {
        for id in &mut *(&raw mut GRANTS) {
            if *id != 0 { let _ = runtime.capabilities.retire_leaf(*id, OWNER); *id = 0; }
        }
        for id in &mut *(&raw mut RULES) {
            if let Some(id) = id.take() { let _ = runtime.network.policy.delete(id); }
        }
        drain_retiring(runtime);
    });
}

// ------------------------=
// FUNC: link
// DESC: Requires a live configured NIC and preferred unexpired address; profile changes invalidate all prior sockets.
// ------------------=
fn link(runtime: &crate::runtime::InfinityRuntime, now: u64) -> Option<Link> {
    let network = &runtime.network;
    if !network.profiles.active()?.interfaces_enabled { return None; }
    let interface = network.interfaces.interface(2)?;
    if !interface.enabled || interface.device.link_state != net::LinkState::Up { return None; }
    let address = (0..network.interfaces.address_count()).filter_map(|i| network.interfaces.address_nth(i))
        .find(|a| a.interface_id == 2 && a.state == net::AddressState::Preferred && a.valid_until.is_none_or(|end| now < end)
            && matches!(a.address, net::IpAddress::V4(_)))?;
    let net::IpAddress::V4(ip) = address.address else { return None; };
    let gateway = (0..network.interfaces.route_count()).filter_map(|i| network.interfaces.route_nth(i))
        .filter(|r| r.interface_id == 2 && r.destination == net::IpAddress::V4([0; 4]) && r.prefix_length == 0
            && r.state == net::RouteState::Active && r.policy_scope.is_none())
        .min_by_key(|r| (r.metric, r.id)).and_then(|r| match r.next_hop { Some(net::IpAddress::V4(ip)) => Some(ip), _ => None });
    Some(Link { address: ip, prefix: address.prefix_length, mac: interface.device.hardware_address?, gateway, generation: network.profiles.generation() })
}

// ------------------------=
// FUNC: start_link
// DESC: Lends disjoint static TCP buffers to the shared reactor only after destroying any previous owner.
// ------------------=
unsafe fn start_link(link: Link, config: Configuration, now: u64) -> Result<(), Failure> {
    clear_connections();
    let mut seed = [0; 8];
    if !super::ssh_entropy::fill(&mut seed) { return Err(Failure::EntropyUnavailable); }
    let mut device = EthernetQueue::new();
    STORAGE = [const { SocketStorage::EMPTY }; MAX_CONNECTIONS];
    BUFFERS_DIRTY = true;
    let mut reactor = sockets::Reactor::new(&mut device, link.mac, link.address, link.prefix, link.gateway,
        u64::from_le_bytes(seed), Instant::from_millis(now as i64), &mut *(&raw mut STORAGE)).map_err(|_| Failure::Configuration)?;
    for (rx, tx) in (&mut *(&raw mut TCP_RX)).iter_mut().zip((&mut *(&raw mut TCP_TX)).iter_mut()).take(config.max_connections as usize) {
        reactor.add_tcp(rx, tx).map_err(|_| Failure::Capacity)?;
    }
    STATE = Some(State { reactor, device, listeners: [None; MAX_CONNECTIONS], sessions: [const { None }; MAX_CONNECTIONS], link, cursor: 0 });
    Ok(())
}

// ------------------------=
// FUNC: ingest
// DESC: Copies only ARP and TCP for this listener, so the SSH stack cannot reset unrelated outbound connections.
// ------------------=
pub(super) fn ingest(frame: &[u8]) {
    let Some(_guard) = lock() else { return; };
    unsafe {
        let Some(state) = (&mut *(&raw mut STATE)).as_mut() else { return; };
        if frame.len() < 14 { return; }
        let arp = frame[12..14] == [8, 6];
        let tcp = tcp_endpoint(frame, state.link.address, CONFIG.port, true).is_some();
        if arp || tcp { let _ = state.device.ingest(frame); }
    }
}

// ------------------------=
// FUNC: tcp_endpoint
// DESC: Classifies bounded unfragmented IPv4 TCP frames; smoltcp subsequently verifies lengths and checksums.
// ------------------=
fn tcp_endpoint(frame: &[u8], address: [u8; 4], port: u16, inbound: bool) -> Option<sockets::Endpoint> {
    if frame.len() < 54 || frame[12..14] != [8, 0] || frame[14] >> 4 != 4 || frame[23] != 6
        || u16::from_be_bytes([frame[20], frame[21]]) & 0x3fff != 0 { return None; }
    let header = (frame[14] as usize & 15) * 4;
    if header < 20 || frame.len() < 14 + header + 20 { return None; }
    let (local_ip, remote_ip, local_port, remote_port) = if inbound { (30, 26, 16 + header, 14 + header) }
        else { (26, 30, 14 + header, 16 + header) };
    if frame[local_ip..local_ip + 4] != address || u16::from_be_bytes([frame[local_port], frame[local_port + 1]]) != port { return None; }
    Some(sockets::Endpoint { address: frame[remote_ip..remote_ip + 4].try_into().ok()?,
        port: u16::from_be_bytes([frame[remote_port], frame[remote_port + 1]]) })
}

// ------------------------=
// FUNC: poll
// DESC: Performs bounded NIC work and at most one cryptographic protocol event per tick, preserving other network and UI work.
// ------------------=
pub(super) fn poll(nic: &mut super::e1000::E1000, now: u64) {
    let Some(_guard) = lock() else { return; };
    unsafe {
        if !CONFIG.enabled || (&*(&raw const HOST)).is_none() { return; }
        crate::runtime::with_runtime(|runtime| {
            let Some(current) = link(runtime, now / 1000) else { clear_connections(); return; };
            if (&*(&raw const STATE)).as_ref().is_none_or(|s| s.link != current) {
                if now < NEXT_START { return; }
                if let Err(failure) = start_link(current, CONFIG, now) {
                    STATUS.last_failure = Some(failure);
                    NEXT_START = now.saturating_add(1000);
                    clear_connections();
                    return;
                }
            }
            let policy = Policy { caps: &runtime.capabilities, identity: &runtime.identity, network: RefCell::new(&mut runtime.network.policy),
                grants: GRANTS, local: sockets::Endpoint { address: current.address, port: CONFIG.port }, now: now / 1000 };
            if !policy.valid() { clear_connections(); STATUS.last_failure = Some(Failure::PermissionDenied); return; }
            let state = (&mut *(&raw mut STATE)).as_mut().unwrap();
            let clock = Instant::from_millis(now as i64);
            for listener in state.listeners.iter().flatten() {
                let _ = state.reactor.renew_idle_listener(*listener, &policy, clock,
                    Instant::from_millis(now.saturating_add(5000) as i64));
            }
            // Flush closed sockets before a subsequent listen reuses their TCP state.
            state.reactor.poll(&mut state.device, &ListenerAuthority { policy: &policy, address: current.address }, clock);
            for index in 0..CONFIG.max_connections as usize {
                if state.listeners[index].is_none() && state.sessions[index].is_none() {
                    state.listeners[index] = state.reactor.listen(&policy, GRANT, CONFIG.port, clock,
                        Instant::from_millis(now.saturating_add(5000) as i64)).ok();
                }
            }
            for index in 0..CONFIG.max_connections as usize {
                if let Some(listener) = state.listeners[index] {
                    match state.reactor.accept(listener, &policy, clock, Instant::from_millis(now.saturating_add(330_000) as i64)) {
                        Ok((tcp, _)) => {
                            state.listeners[index] = None;
                            let ssh = Transport::new(&mut (&mut *(&raw mut SSH_RX))[index], &mut (&mut *(&raw mut SSH_TX))[index], now);
                            match ssh {
                                Ok(ssh) => { state.sessions[index] = Some(Session { tcp, ssh, native: None, terminal: crate::runtime::console_terminal::Terminal::new(), failure: None, completed: 0, last_operation: 0 }); STATUS.active += 1; STATUS.accepted = STATUS.accepted.saturating_add(1); },
                                Err(failure) => { let _ = state.reactor.release(tcp); STATUS.last_failure = Some(failure); },
                            }
                        }
                        Err(sockets::Error::WouldBlock) => (),
                        Err(error) => {
                            if error == sockets::Error::Denied {
                                STATUS.denied = STATUS.denied.saturating_add(1);
                                STATUS.last_failure = Some(Failure::PermissionDenied);
                            }
                            let _ = state.reactor.release_listener(listener);
                            state.listeners[index] = None;
                        },
                    }
                }
            }
            let index = state.cursor;
            state.cursor = (state.cursor + 1) % CONFIG.max_connections as usize;
            if let Some(session) = state.sessions[index].as_mut() {
                let started = crate::ui::performance::monotonic_ns();
                let result = drive(session, &mut state.reactor, &policy, (&*(&raw const HOST)).as_ref().unwrap(), now);
                if let (Some(started), Some(ended)) = (started, crate::ui::performance::monotonic_ns()) {
                    STATUS.peak_protocol_ns = STATUS.peak_protocol_ns.max(ended.saturating_sub(started));
                    STATUS.protocol_steps = STATUS.protocol_steps.saturating_add(1);
                }
                if let Err(failure) = result {
                    if failure == Failure::InvalidSignature { STATUS.denied = STATUS.denied.saturating_add(1); }
                    let tcp = session.tcp;
                    retire_session(session);
                    state.sessions[index] = None;
                    let _ = state.reactor.release(tcp);
                    use zeroize::Zeroize;
                    (&mut *(&raw mut SSH_RX))[index].zeroize(); (&mut *(&raw mut SSH_TX))[index].zeroize();
                    STATUS.active = STATUS.active.saturating_sub(1);
                    STATUS.closed = STATUS.closed.saturating_add(1);
                    STATUS.last_failure = Some(failure);
                }
            }
            STATUS.listening = state.listeners.iter().any(Option::is_some) || STATUS.active > 0;
            for _ in 0..4 {
                let Some(frame) = state.device.pending() else { break; };
                let allowed = if frame.len() >= 14 && frame[12..14] == [8, 6] { policy.valid() }
                    else { tcp_endpoint(frame, current.address, CONFIG.port, false)
                        .is_some_and(|peer| sockets::Authority::allows(&policy, GRANT, sockets::Protocol::Tcp, peer)) };
                if allowed && !nic.transmit(frame) { break; }
                state.device.transmitted();
            }
        });
        service_console(now);
    }
}

// ------------------------=
// FUNC: drive
// DESC: Preserves TCP and SSH backpressure, reports typed authentication outcomes, and never executes remote work.
// ------------------=
fn drive(session: &mut Session, reactor: &mut sockets::Reactor<'static, MAX_CONNECTIONS>, policy: &Policy<'_>, host: &HostKey, now: u64) -> Result<(), Failure> {
    if let Some(failure) = session.failure { return Err(failure); }
    let clock = Instant::from_millis(now as i64);
    let mut bytes = [0; 1024];
    match reactor.peek(session.tcp, policy, clock, &mut bytes) {
        Ok(0) => return Err(Failure::Disconnected),
        Ok(length) => {
            let used = session.ssh.input(&bytes[..length], policy, now)?;
            if used > 0 { reactor.receive(session.tcp, policy, clock, &mut bytes[..used]).map_err(|_| Failure::Disconnected)?; }
        }
        Err(sockets::Error::WouldBlock) => (),
        Err(_) => return Err(Failure::Disconnected),
    }
    match session.ssh.progress(policy, host, now)? {
        Diagnostic::Authenticated => unsafe { STATUS.authenticated = STATUS.authenticated.saturating_add(1); },
        Diagnostic::AuthenticationRejected | Diagnostic::ChannelDenied => unsafe { STATUS.denied = STATUS.denied.saturating_add(1); },
        _ => (),
    }
    let output = session.ssh.output(policy, now)?;
    if !output.is_empty() {
        match reactor.send(session.tcp, policy, clock, output) {
            Ok(length) => session.ssh.consume_output(length)?,
            Err(sockets::Error::WouldBlock) => (),
            Err(_) => return Err(Failure::Disconnected),
        }
    }
    Ok(())
}

// ------------------------=
// FUNC: retire_session
// DESC: Defers native identity cleanup until the network policy borrow is released, without nested Runtime access.
// ------------------=
unsafe fn retire_session(session: &mut Session) {
    if let Some(native) = session.native.take() {
        if let Some(slot) = (&mut *(&raw mut RETIRING)).iter_mut().find(|slot| slot.is_none()) { *slot = Some((native.id, native.user)); }
    }
}
// ------------------------=
// FUNC: drain_retiring
// DESC: Revokes only the terminated SSH identities, leaving local Console and other connections intact.
// ------------------=
unsafe fn drain_retiring(runtime: &mut crate::runtime::InfinityRuntime) {
    for slot in &mut *(&raw mut RETIRING) {
        if let Some((id, user)) = slot.take() { let _ = runtime.identity.end_session(id, user); }
    }
}
impl IdentityAuthority for IdentitySystem {
    // ------------------------=
    // FUNC: authorized_key
    // DESC: Uses the same native credential lookup as the transport policy.
    // ------------------=
    fn authorized_key(&self, name: &[u8], key: &[u8; 32]) -> Option<Principal> {
        self.ssh_key_owner(name, key).map(|user| Principal { user: user.0, public_key: *key })
    }
    // ------------------------=
    // FUNC: still_authorized
    // DESC: Revalidates key and account state before channel data crosses the service boundary.
    // ------------------=
    fn still_authorized(&self, principal: Principal) -> bool { self.ssh_key_active(StableId(principal.user), &principal.public_key) }
}
// ------------------------=
// FUNC: service_console
// DESC: Executes at most one bounded native query per NIC tick, outside the protocol policy borrow and without any desktop callbacks.
// ------------------=
unsafe fn service_console(now: u64) {
    crate::runtime::with_runtime(|runtime| {
        drain_retiring(runtime);
        let Some(state) = (&mut *(&raw mut STATE)).as_mut() else { return; };
        let index = (state.cursor + CONFIG.max_connections as usize - 1) % CONFIG.max_connections as usize;
        let Some(session) = state.sessions[index].as_mut() else { return; };
        let result = console_step(session, runtime, now);
        if let Err(failure) = result { session.failure = Some(failure); }
    });
}
// ------------------------=
// FUNC: console_step
// DESC: Owns one channel's provisioning, input, completion, cancellation and output backpressure through the shared Console dispatcher.
// ------------------=
fn console_step(session: &mut Session, runtime: &mut crate::runtime::InfinityRuntime, now: u64) -> Result<(), Failure> {
    use infinity_ssh::ConsoleRequest;
    use crate::runtime::console_execution::{self, Outcome, Diagnostic as ConsoleDiagnostic};
    let Some(principal) = session.ssh.principal() else { return Ok(()); };
    if session.native.is_none() {
        session.native = Some(runtime.identity.create_ssh_session(StableId(principal.user), principal.public_key, now / 1000)
            .map_err(|_| Failure::PermissionDenied)?);
        session.ssh.authorize_console()?;
    }
    let native = session.native.unwrap();
    if session.terminal.finished { return Ok(()); }
    if !runtime.identity.console_session_authorized(native.id, native.user) { return Err(Failure::Revoked); }
    let terminal = &mut session.terminal;
    match session.ssh.take_console_request() {
        Some(ConsoleRequest::Pty { columns, rows }) => { if !terminal.resize(columns, rows) { return Err(Failure::Protocol); } terminal.pty = true; }
        Some(ConsoleRequest::Resize { columns, rows }) => { if !terminal.resize(columns, rows) { return Err(Failure::Protocol); } }
        Some(ConsoleRequest::Interrupt) => { terminal.interrupt(); unsafe { INTERRUPTS = INTERRUPTS.saturating_add(1); } },
        Some(ConsoleRequest::Shell) => { if !terminal.start(None, now) { return Err(Failure::Protocol); } }
        Some(ConsoleRequest::Exec { bytes, length }) => { if !terminal.start(Some(&bytes[..length]), now) { return Err(Failure::Protocol); } }
        None => (),
    }
    for _ in 0..64 {
        if !terminal.can_input() { break; }
        let mut byte = [0];
        if session.ssh.read_console(&mut byte, &runtime.identity, now)? == 0 { break; }
        terminal.input_byte(byte[0], now);
        if byte[0] == 3 { unsafe { INTERRUPTS = INTERRUPTS.saturating_add(1); } }
    }
    if terminal.started && terminal.interactive && session.ssh.console_eof() {
        if terminal.length > 0 && !terminal.pending && terminal.outcome.is_none() { terminal.pending = true; terminal.deadline = now.saturating_add(10_000); }
        terminal.closing = true;
    }
    if terminal.deadline != 0 && now >= terminal.deadline {
        terminal.deadline = 0; terminal.wire_offset = 0; terminal.wire_length = 0;
        terminal.complete(Outcome::failure(ConsoleDiagnostic::Deadline)); terminal.closing = true;
    }
    if terminal.pending {
        let started = crate::ui::performance::monotonic_ns();
        let result = console_execution::execute(runtime, native.user, native.id, &terminal.input[..terminal.length], now / 1000);
        session.completed = session.completed.saturating_add(1);
        session.last_operation = result.operation.map(|op| op.machine_id() as u64).unwrap_or(0);
        unsafe {
            COMMANDS = COMMANDS.saturating_add(1);
            if result.diagnostic == ConsoleDiagnostic::Denied { COMMAND_DENIALS = COMMAND_DENIALS.saturating_add(1); }
            if let (Some(start), Some(end)) = (started, crate::ui::performance::monotonic_ns()) { PEAK_EXECUTION_NS = PEAK_EXECUTION_NS.max(end.saturating_sub(start)); }
        }
        terminal.complete(result);
    }
    if !terminal.echo().is_empty() {
        let n = session.ssh.write_console(terminal.echo(), false, &runtime.identity, now)?;
        terminal.consume_echo(n); return Ok(());
    }
    if terminal.outcome.is_some() {
        terminal.stage_output();
        if terminal.wire_offset < terminal.wire_length {
            let n = session.ssh.write_console(&terminal.wire[terminal.wire_offset..terminal.wire_length], terminal.wire_stderr, &runtime.identity, now)?;
            terminal.wire_offset += n; return Ok(());
        }
        terminal.output_complete();
    }
    if terminal.closing && terminal.echo().is_empty() {
        if session.ssh.finish_console(terminal.status)? {
            runtime.identity.end_session(native.id, native.user).map_err(|_| Failure::PermissionDenied)?;
            // The execution authority is gone, but SSH must still flush exit-status and channel close.
            terminal.finished = true;
        }
    }
    Ok(())
}
