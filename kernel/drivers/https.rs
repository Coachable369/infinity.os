//! BSP-owned HTTPS actor. The network pump calls poll after releasing Runtime borrows.
use crate::http_transport::{
    self as http,
    client::{Configuration, Destination, Link},
    rand_core::{RngCore, SeedableRng},
    smoltcp::time::Instant,
    task::Task,
};
use crate::runtime::{
    capability::{CapabilityId, CapabilityType},
    execution::SecurityIdentity,
    network::types::*,
};
use core::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

const BODY: usize = 8192;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Busy,
    Invalid,
    Unavailable,
    Denied,
    Cancelled,
    Timeout,
    Resolution,
    Transport,
}
pub struct Response {
    pub status: u16,
    pub length: usize,
    pub bytes: [u8; BODY],
}
type Outcome = Result<Response, Failure>;
#[derive(Clone, Copy)]
struct Authority {
    owner: SecurityIdentity,
    connect: CapabilityId,
    send: CapabilityId,
    receive: CapabilityId,
    resolve: CapabilityId,
    generation: u64,
    address: [u8; 4],
    prefix: u8,
    mac: [u8; 6],
    dns: [u8; 4],
    gateway: Option<[u8; 4]>,
    local_port: u16,
}
struct Frame {
    bytes: [u8; 1514],
    length: usize,
}
struct Ring {
    frames: [Frame; 4],
    head: usize,
    count: usize,
}
impl Ring {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a fixed packet ring owned by the BSP HTTPS actor.
    // ------------------=
    const fn new() -> Self {
        Self {
            frames: [const {
                Frame {
                    bytes: [0; 1514],
                    length: 0,
                }
            }; 4],
            head: 0,
            count: 0,
        }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Retains a complete frame or reports backpressure without overwriting queued traffic.
    // ------------------=
    fn push(&mut self, bytes: &[u8]) -> bool {
        if self.count == 4 || !(14..=1514).contains(&bytes.len()) {
            return false;
        }
        let frame = &mut self.frames[(self.head + self.count) % 4];
        frame.bytes[..bytes.len()].copy_from_slice(bytes);
        frame.length = bytes.len();
        self.count += 1;
        true
    }
    // ------------------------=
    // FUNC: pop
    // DESC: Copies one retained ingress packet to the client.
    // ------------------=
    fn pop(&mut self, bytes: &mut [u8; 1514]) -> Option<usize> {
        if self.count == 0 {
            return None;
        }
        let frame = &self.frames[self.head];
        let length = frame.length;
        bytes[..length].copy_from_slice(&frame.bytes[..length]);
        self.head = (self.head + 1) % 4;
        self.count -= 1;
        Some(length)
    }
}
static LOCK: AtomicBool = AtomicBool::new(false);
static mut TASK: Task<Outcome, 262144> = Task::new();
static mut RNG: Option<http::rand_chacha::ChaCha20Rng> = None;
static mut CLOCK: u64 = 0;
static mut SERVICES: u64 = 0;
static mut OWNER: Option<SecurityIdentity> = None;
static mut RESULT: Option<Outcome> = None;
static mut RX: Ring = Ring::new();
static mut TX: Ring = Ring::new();
static mut AUTH: Option<Authority> = None;
static mut ENDPOINT: ([u8; 4], u16) = ([0; 4], 0);
struct Guard;
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases actor ownership after every early-return path.
    // ------------------=
    fn drop(&mut self) {
        LOCK.store(false, Ordering::Release);
    }
}
// ------------------------=
// FUNC: lock
// DESC: Rejects reentrant actor operations rather than waiting inside an input or service callback.
// ------------------=
fn lock() -> Option<Guard> {
    if LOCK.swap(true, Ordering::Acquire) {
        None
    } else {
        Some(Guard)
    }
}

// ------------------------=
// FUNC: default_gateway
// DESC: Selects the active unscoped IPv4 default route independently of the typically on-link DNS server.
// ------------------=
fn default_gateway(network: &crate::runtime::network::NetworkRuntime) -> Option<[u8; 4]> {
    (0..network.interfaces.route_count())
        .filter_map(|i| network.interfaces.route_nth(i))
        .filter(|r| {
            r.interface_id == 2
                && r.destination == IpAddress::V4([0; 4])
                && r.prefix_length == 0
                && r.state == RouteState::Active
                && r.policy_scope.is_none()
        })
        .min_by_key(|r| (r.metric, r.id))
        .and_then(|r| match r.next_hop {
            Some(IpAddress::V4(ip)) => Some(ip),
            _ => None,
        })
}

// ------------------------=
// FUNC: initialize
// DESC: Seeds a dedicated cryptographic request generator from genuine boot entropy, failing closed when unavailable.
// ------------------=
pub(super) fn initialize(info: &crate::boot_info::BootInfo) {
    let Some(_guard) = lock() else {
        return;
    };
    unsafe {
        SERVICES = info.firmware_runtime_services;
        if info.firmware_entropy_valid == 1 && RNG.is_none() {
            use sha2::{Digest, Sha256};
            let mut derivation = Sha256::new();
            derivation.update(b"InfinityOS HTTPS RNG v1\0");
            derivation.update(info.firmware_entropy);
            RNG = Some(http::rand_chacha::ChaCha20Rng::from_seed(
                derivation.finalize().into(),
            ));
        }
    }
}

// ------------------------=
// FUNC: authorized
// DESC: Rechecks caller capabilities, profile generation, link, source address, route and endpoint policy on the BSP only.
// ------------------=
fn authorized(auth: Authority, address: [u8; 4], port: u16, now: u64) -> bool {
    crate::runtime::with_runtime(|runtime| {
        if !(0..crate::runtime::identity::MAX_SESSIONS)
            .filter_map(|i| runtime.identity.session_nth(i))
            .any(|s| {
                s.id.0 == auth.owner.0 && s.state == crate::runtime::identity::SessionState::Active
            })
        {
            return false;
        }
        let network = &mut runtime.network;
        if network.resolver.server(0) != Some(IpAddress::V4(auth.dns)) {
            return false;
        }
        if default_gateway(network) != auth.gateway {
            return false;
        }
        for (id, kind) in [
            (auth.connect, CapabilityType::NetworkConnect),
            (auth.send, CapabilityType::NetworkSend),
            (auth.receive, CapabilityType::NetworkReceive),
        ] {
            if runtime
                .capabilities
                .validate(id, auth.owner, kind, 0, 1, 0, now)
                .is_err()
            {
                return false;
            }
        }
        if port == 53
            && (runtime
                .capabilities
                .validate(
                    auth.resolve,
                    auth.owner,
                    CapabilityType::NetworkResolve,
                    0,
                    1,
                    0,
                    now,
                )
                .is_err()
                || !network.resolver.enabled())
        {
            return false;
        }
        if network.profiles.generation() != auth.generation
            || !network
                .profiles
                .active()
                .map(|p| p.interfaces_enabled)
                .unwrap_or(false)
        {
            return false;
        }
        if !network
            .interfaces
            .interface(2)
            .map(|i| {
                i.enabled
                    && i.device.link_state == LinkState::Up
                    && i.device.hardware_address == Some(auth.mac)
            })
            .unwrap_or(false)
        {
            return false;
        }
        if !(0..network.interfaces.address_count())
            .filter_map(|i| network.interfaces.address_nth(i))
            .any(|a| {
                a.interface_id == 2
                    && a.address == IpAddress::V4(auth.address)
                    && a.prefix_length == auth.prefix
                    && a.state == AddressState::Preferred
                    && a.valid_until.map(|end| now < end).unwrap_or(true)
            })
        {
            return false;
        }
        let Ok(route) = network
            .interfaces
            .select_route(IpAddress::V4(address), None)
        else {
            return false;
        };
        let on_link =
            IpAddress::V4(auth.address).matches_prefix(IpAddress::V4(address), auth.prefix);
        let expected_hop = if on_link {
            None
        } else {
            auth.gateway.map(IpAddress::V4)
        };
        if route.interface_id != 2
            || route.next_hop != expected_hop
            || (!on_link && expected_hop.is_none())
        {
            return false;
        }
        for direction in [Direction::Outbound, Direction::Inbound] {
            let decision = network.policy.evaluate(
                Subject::Context(auth.owner),
                direction,
                Some(2),
                Endpoint {
                    address: IpAddress::V4(auth.address),
                    port: auth.local_port,
                },
                Endpoint {
                    address: IpAddress::V4(address),
                    port,
                },
                if port == 53 {
                    TransportProtocol::Datagram
                } else {
                    TransportProtocol::Stream
                },
                now,
            );
            if !matches!(
                decision.action,
                PolicyAction::Allow | PolicyAction::AuditOnly
            ) {
                return false;
            }
        }
        true
    })
    .unwrap_or(false)
}
struct ServiceLink(Authority);
impl Link for ServiceLink {
    // ------------------------=
    // FUNC: now
    // DESC: Reads the monotonic network-pump timestamp under actor ownership.
    // ------------------=
    fn now(&self) -> Instant {
        Instant::from_millis(unsafe { CLOCK } as i64)
    }
    // ------------------------=
    // FUNC: allowed
    // DESC: Revalidates live authority before the shared client processes traffic.
    // ------------------=
    fn allowed(&mut self, address: [u8; 4], port: u16) -> bool {
        let allowed = authorized(self.0, address, port, unsafe { CLOCK / 1000 });
        if allowed {
            unsafe {
                if ENDPOINT != (address, port) {
                    TX = Ring::new();
                }
                ENDPOINT = (address, port);
            }
        }
        allowed
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Drains only the actor's bounded copy of ingress, preserving existing UDP consumers.
    // ------------------=
    fn receive(&mut self, frame: &mut [u8; 1514]) -> Option<usize> {
        unsafe { (&mut *(&raw mut RX)).pop(frame) }
    }
    // ------------------------=
    // FUNC: transmit
    // DESC: Queues bounded egress for reauthorization immediately before NIC submission.
    // ------------------=
    fn transmit(&mut self, frame: &[u8]) -> bool {
        unsafe { (&mut *(&raw mut TX)).push(frame) }
    }
    // ------------------------=
    // FUNC: register_waker
    // DESC: Relies on the actor's one-millisecond cooperative network timer, not immediate self-wakes.
    // ------------------=
    fn register_waker(&mut self, _: &Waker) {}
}

// ------------------------=
// FUNC: get
// DESC: Queues one owner-scoped native HTTPS GET; called only on the BSP outside an outstanding runtime borrow.
// ------------------=
pub(crate) fn get(
    owner: SecurityIdentity,
    connect: CapabilityId,
    send: CapabilityId,
    receive: CapabilityId,
    resolve: CapabilityId,
    host: &str,
    path: &str,
) -> Result<(), Failure> {
    let Some(_guard) = lock() else {
        return Err(Failure::Busy);
    };
    if host.is_empty() || host.len() > 253 || path.len() > 1024 {
        return Err(Failure::Invalid);
    }
    let mut check = [0; 1536];
    http::request::get(host, path, &mut check).map_err(|_| Failure::Invalid)?;
    unsafe {
        if OWNER.is_some() {
            return Err(Failure::Busy);
        }
        let now = crate::ui::performance::monotonic_ns()
            .map(|n| n / 1_000_000)
            .unwrap_or(CLOCK);
        CLOCK = now;
        let time = crate::console::certificate_time(SERVICES).ok_or(Failure::Unavailable)?;
        let rng = (&mut *(&raw mut RNG))
            .as_mut()
            .ok_or(Failure::Unavailable)?;
        let mut seed = [0; 32];
        rng.fill_bytes(&mut seed);
        let port = 49152 + (rng.next_u32() % 16384) as u16;
        let (config, auth) = crate::runtime::with_runtime(|runtime| {
            let n = &runtime.network;
            let interface = n.interfaces.interface(2)?;
            let source = (0..n.interfaces.address_count())
                .filter_map(|i| n.interfaces.address_nth(i))
                .find(|a| {
                    a.interface_id == 2
                        && a.state == AddressState::Preferred
                        && matches!(a.address, IpAddress::V4(_))
                })?;
            let IpAddress::V4(address) = source.address else {
                return None;
            };
            let IpAddress::V4(dns) = n.resolver.server(0)? else {
                return None;
            };
            let gateway = default_gateway(n);
            Some((
                Configuration {
                    mac: interface.device.hardware_address?,
                    address,
                    prefix: source.prefix_length,
                    gateway,
                    dns_server: dns,
                    local_port: port,
                    deadline: Instant::from_millis(
                        now.saturating_add(30000).min(i64::MAX as u64) as i64
                    ),
                },
                Authority {
                    owner,
                    connect,
                    send,
                    receive,
                    resolve,
                    generation: n.profiles.generation(),
                    address,
                    prefix: source.prefix_length,
                    mac: interface.device.hardware_address?,
                    dns,
                    gateway,
                    local_port: port,
                },
            ))
        })
        .flatten()
        .ok_or(Failure::Unavailable)?;
        if !authorized(auth, config.dns_server, 53, now / 1000) {
            return Err(Failure::Denied);
        }
        let mut name = [0; 253];
        name[..host.len()].copy_from_slice(host.as_bytes());
        let name_length = host.len();
        let mut target = [0; 1024];
        target[..path.len()].copy_from_slice(path.as_bytes());
        let target_length = path.len();
        let future = async move {
            let mut read = [0; 16640];
            let mut write = [0; 4096];
            let mut request = [0; 1536];
            let mut body = [0; BODY];
            let result = http::client::get(
                ServiceLink(auth),
                config,
                Destination::Resolve,
                http::rand_chacha::ChaCha20Rng::from_seed(seed),
                http::tls::system_roots(),
                time,
                core::str::from_utf8(&name[..name_length]).unwrap(),
                443,
                core::str::from_utf8(&target[..target_length]).unwrap(),
                http::https::Buffers {
                    read_record: &mut read,
                    write_record: &mut write,
                    request: &mut request,
                    response: &mut body,
                },
            )
            .await
            .map_err(|error| match error {
                http::client::Error::Timeout
                | http::client::Error::Transport(http::transport::Error::Timeout) => {
                    Failure::Timeout
                }
                http::client::Error::Transport(http::transport::Error::ResolutionFailed) => {
                    Failure::Resolution
                }
                http::client::Error::Denied => Failure::Denied,
                _ => Failure::Transport,
            })?;
            Ok(Response {
                status: result.status,
                length: result.body_bytes,
                bytes: body,
            })
        };
        Pin::new_unchecked(&mut *(&raw mut TASK))
            .start(future)
            .map_err(|_| Failure::Busy)?;
        RX = Ring::new();
        TX = Ring::new();
        AUTH = Some(auth);
        OWNER = Some(owner);
        RESULT = None;
        Ok(())
    }
}
// ------------------------=
// FUNC: take
// DESC: Returns only the owner's complete response and releases the bounded request slot.
// ------------------=
pub(crate) fn take(owner: SecurityIdentity) -> Result<Option<Outcome>, Failure> {
    let Some(_guard) = lock() else {
        return Err(Failure::Busy);
    };
    unsafe {
        if OWNER != Some(owner) {
            return Err(Failure::Denied);
        }
        let result = (&mut *(&raw mut RESULT)).take();
        if result.is_some() {
            OWNER = None;
            AUTH = None;
        }
        Ok(result)
    }
}
// ------------------------=
// FUNC: cancel
// DESC: Cancels only the caller's request, destroys its future, and clears queued packets before reporting cancellation.
// ------------------=
pub(crate) fn cancel(owner: SecurityIdentity) -> Result<(), Failure> {
    let Some(_guard) = lock() else {
        return Err(Failure::Busy);
    };
    unsafe {
        if OWNER != Some(owner) {
            return Err(Failure::Denied);
        }
        Pin::new_unchecked(&mut *(&raw mut TASK)).cancel();
        RX = Ring::new();
        TX = Ring::new();
        RESULT = Some(Err(Failure::Cancelled));
    }
    Ok(())
}
// ------------------------=
// FUNC: ingest
// DESC: Copies bounded NIC ingress without taking ownership from existing datagram services.
// ------------------=
pub(super) fn ingest(frame: &[u8]) {
    let Some(_guard) = lock() else {
        return;
    };
    unsafe {
        if (&*(&raw const TASK)).active() {
            let _ = (&mut *(&raw mut RX)).push(frame);
        }
    }
}
// ------------------------=
// FUNC: poll
// DESC: Runs one cooperative request poll outside Runtime borrows and revalidates all queued egress immediately before native submission.
// ------------------=
pub(super) fn poll(nic: &mut super::e1000::E1000, now_ms: u64) {
    let Some(_guard) = lock() else {
        return;
    };
    unsafe {
        CLOCK = now_ms;
        if let Some(owner) = OWNER {
            let active = crate::runtime::with_runtime(|runtime| {
                (0..crate::runtime::identity::MAX_SESSIONS)
                    .filter_map(|i| runtime.identity.session_nth(i))
                    .any(|s| {
                        s.id.0 == owner.0
                            && s.state == crate::runtime::identity::SessionState::Active
                    })
            })
            .unwrap_or(false);
            if !active {
                Pin::new_unchecked(&mut *(&raw mut TASK)).cancel();
                OWNER = None;
                AUTH = None;
                RESULT = None;
                RX = Ring::new();
                TX = Ring::new();
                return;
            }
        }
        if !(&*(&raw const TASK)).active() {
            return;
        }
        let mut context = Context::from_waker(Waker::noop());
        if let Poll::Ready(result) = Pin::new_unchecked(&mut *(&raw mut TASK)).poll(&mut context) {
            RESULT = Some(result);
            RX = Ring::new();
            TX = Ring::new();
            return;
        }
        let Some(auth) = AUTH else {
            return;
        };
        if !authorized(auth, ENDPOINT.0, ENDPOINT.1, now_ms / 1000) {
            Pin::new_unchecked(&mut *(&raw mut TASK)).cancel();
            RESULT = Some(Err(Failure::Denied));
            TX = Ring::new();
            RX = Ring::new();
            return;
        }
        let tx = &mut *(&raw mut TX);
        for _ in 0..4 {
            if tx.count == 0 {
                break;
            }
            let frame = &tx.frames[tx.head];
            let on_link =
                IpAddress::V4(auth.address).matches_prefix(IpAddress::V4(ENDPOINT.0), auth.prefix);
            let next_hop = if on_link {
                ENDPOINT.0
            } else {
                auth.gateway.unwrap_or([0; 4])
            };
            let local = http::egress::local_port(
                &frame.bytes[..frame.length],
                auth.mac,
                auth.address,
                ENDPOINT.0,
                next_hop,
                ENDPOINT.1,
            );
            let permitted = match local {
                Some(0) => true, // Endpoint authority and next-hop ARP are already validated.
                Some(port) if ENDPOINT.1 == 53 || port == auth.local_port => authorized(
                    Authority {
                        local_port: port,
                        ..auth
                    },
                    ENDPOINT.0,
                    ENDPOINT.1,
                    now_ms / 1000,
                ),
                _ => false,
            };
            if !permitted {
                tx.head = (tx.head + 1) % 4;
                tx.count -= 1;
                continue;
            }
            if !nic.transmit(&frame.bytes[..frame.length]) {
                break;
            }
            tx.head = (tx.head + 1) % 4;
            tx.count -= 1;
        }
    }
}
