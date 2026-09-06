//! Stable, architecture-neutral network contracts. These values are the native
//! machine API; no descriptor, errno, device-file, or host socket leaks through.

use crate::runtime::execution::SecurityIdentity;

pub const MAX_INTERFACES: usize = 8;
pub const MAX_ADDRESSES: usize = 24;
pub const MAX_ROUTES: usize = 32;
pub const MAX_CONNECTIONS: usize = 8;
pub const MAX_LISTENERS: usize = 8;
pub const MAX_PACKET_BYTES: usize = 512;
pub const CONNECTION_QUEUE_DEPTH: usize = 3;
pub const MAX_POLICIES: usize = 32;
pub const MAX_PROFILES: usize = 8;
pub const MAX_RESOLVER_ENTRIES: usize = 24;
pub const MAX_DISCOVERED_SERVICES: usize = 24;

pub type InterfaceId = u32;
pub type AddressId = u32;
pub type RouteId = u32;
pub type ConnectionId = u32;
pub type ListenerId = u32;
pub type PolicyId = u32;
pub type ProfileId = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinkType { Loopback, Ethernet, Wireless, Virtual, Unknown }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinkState { Unknown, Down, Up }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OperationalState { Unavailable, Offline, Configuring, Ready, Degraded, Failed }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddressFamily { Ipv4, Ipv6 }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddressScope { Host, Link, Private, Global }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddressSource { Static, Dynamic, Slaac, LinkLocal, System, TestFixture }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddressState { Tentative, Preferred, Deprecated, Invalid }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RouteSource { Interface, Static, Dynamic, Profile, System, TestFixture }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RouteState { Active, Inactive, Rejected }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransportProtocol { Stream, Datagram }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectionState { Defined, Resolving, Connecting, Open, Closing, Closed, Failed }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SecurityState { Plain, Negotiating, VerifiedFixture, VerificationFailed, Unsupported }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectivityClass { Offline, LinkOnly, LocalNetwork, LimitedConnectivity, Routed, InternetReachableOptional, Degraded }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NetworkSetupMode { Automatic, Wired, Wireless, Offline }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Subject { System, Context(SecurityIdentity), Service(u32), Application(u64), Session(u64) }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction { Inbound, Outbound }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PolicyAction { Allow, Deny, Ask, RateLimit, AuditOnly }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoggingMode { None, Denials, Decisions }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NetworkError {
    NetworkUnavailable, InterfaceNotFound, LinkDown, AddressUnavailable, NoRoute,
    NameResolutionFailed, ResolutionTimeout, ResolverUnavailable, ConnectionRefused,
    ConnectionReset, ConnectionTimeout, ConnectionClosed, AccessDenied, PolicyDenied,
    CapabilityRevoked, InvalidEndpoint, UnsupportedProtocol, SecureHandshakeFailed,
    IdentityVerificationFailed, ResourceLimitExceeded, QueueFull, AlreadyExists,
    Conflict, UnsupportedOperation, InvalidProfile,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IpAddress { V4([u8; 4]), V6([u8; 16]) }

impl IpAddress {
    // ------------------------=
    // FUNC: family
    // DESC: Returns the address family without string parsing or host assumptions.
    // ------------------=
    pub const fn family(self) -> AddressFamily {
        match self { Self::V4(_) => AddressFamily::Ipv4, Self::V6(_) => AddressFamily::Ipv6 }
    }

    // ------------------------=
    // FUNC: matches_prefix
    // DESC: Compares two typed addresses under a validated network prefix.
    // ------------------=
    pub fn matches_prefix(self, other: Self, prefix: u8) -> bool {
        let (a, b, bits): ([u8; 16], [u8; 16], u8) = match (self, other) {
            (Self::V4(a), Self::V4(b)) => {
                let mut aa = [0; 16]; let mut bb = [0; 16];
                aa[..4].copy_from_slice(&a); bb[..4].copy_from_slice(&b); (aa, bb, 32)
            }
            (Self::V6(a), Self::V6(b)) => (a, b, 128),
            _ => return false,
        };
        if prefix > bits { return false; }
        let bytes = (prefix / 8) as usize;
        let rem = prefix % 8;
        if a[..bytes] != b[..bytes] { return false; }
        rem == 0 || (a[bytes] & (0xff << (8 - rem))) == (b[bytes] & (0xff << (8 - rem)))
    }

    // ------------------------=
    // FUNC: is_host_local
    // DESC: Identifies native host-local addresses used by the loopback transport.
    // ------------------=
    pub const fn is_host_local(self) -> bool {
        match self { Self::V4(v) => v[0] == 127, Self::V6(v) => v[15] == 1 && v[0] == 0 && v[1] == 0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkDevice {
    pub device_id: u64, pub driver_id: u32, pub link_type: LinkType,
    pub hardware_address: Option<[u8; 6]>, pub link_state: LinkState,
    pub maximum_frame_size: u16, pub can_receive: bool, pub can_transmit: bool,
    pub offload_capabilities: u32, pub operational_state: OperationalState, pub error_code: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FirmwareNetworkDevice {
    pub firmware_handle: u64,
    pub device_id: u64,
    pub hardware_address: Option<[u8; 6]>,
    pub link_state: LinkState,
    pub maximum_frame_size: u16,
    pub can_receive: bool,
    pub can_transmit: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkInterface {
    pub id: InterfaceId, pub device: NetworkDevice, pub enabled: bool,
    pub rx_packets: u64, pub tx_packets: u64, pub rx_drops: u64, pub tx_drops: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkAddress {
    pub id: AddressId, pub interface_id: InterfaceId, pub address: IpAddress,
    pub prefix_length: u8, pub scope: AddressScope, pub source: AddressSource,
    pub state: AddressState, pub valid_until: Option<u64>, pub preferred_until: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Route {
    pub id: RouteId, pub destination: IpAddress, pub prefix_length: u8,
    pub next_hop: Option<IpAddress>, pub interface_id: InterfaceId, pub metric: u32,
    pub source: RouteSource, pub state: RouteState, pub policy_scope: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Endpoint { pub address: IpAddress, pub port: u16 }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ConnectionStatistics {
    pub bytes_sent: u64, pub bytes_received: u64, pub messages_sent: u64,
    pub messages_received: u64, pub queue_drops: u64, pub opened_at: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Connection {
    pub id: ConnectionId, pub owner: SecurityIdentity, pub subject: Subject,
    pub protocol: TransportProtocol, pub local: Endpoint, pub remote: Endpoint,
    pub state: ConnectionState, pub security: SecurityState, pub capability_scope: u64,
    pub queue_limit: u8, pub statistics: ConnectionStatistics, pub future_node_identity: Option<[u8; 16]>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Packet { pub length: u16, pub bytes: [u8; MAX_PACKET_BYTES] }

impl Packet {
    // ------------------------=
    // FUNC: from_slice
    // DESC: Copies a bounded payload into a native transport packet.
    // ------------------=
    pub fn from_slice(data: &[u8]) -> Result<Self, NetworkError> {
        if data.len() > MAX_PACKET_BYTES { return Err(NetworkError::ResourceLimitExceeded); }
        let mut bytes = [0; MAX_PACKET_BYTES]; bytes[..data.len()].copy_from_slice(data);
        Ok(Self { length: data.len() as u16, bytes })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct NetworkDiagnostics {
    pub rx_packets: u64, pub tx_packets: u64, pub drops: u64, pub connection_failures: u64,
    pub resolver_failures: u64, pub queue_pressure: u64, pub policy_denials: u64,
    pub active_connections: u16, pub resolver_entries: u16, pub discovered_services: u16,
}
