//! Bounded discovery carriage over explicitly authorized native datagram connections.
//! A receiver-issued, boot-fresh nonce provides freshness without shared boot clocks.
use super::{map_crypto_error, DiscoveryAdvertisement, NodeCrypto, NodeRuntime, NODE_PROTOCOL_VERSION};
use super::types::{NodeError, NodeId};
use crate::runtime::{capability::{CapabilityManager, CapabilityType}, execution::SecurityIdentity};
use crate::runtime::network::{NetworkRuntime, types::{ConnectionState, Endpoint, IpAddress, NetworkError, Packet, TransportProtocol}};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

pub const MAX_LINKS: usize = 4;
const CHALLENGE: &[u8; 8] = b"INDCH001";
const RESPONSE: &[u8; 8] = b"INDRS001";
const RESPONSE_BYTES: usize = 199;

#[derive(Clone, Copy)]
pub struct LinkAuthority {
    pub owner: SecurityIdentity,
    pub connection: u32,
    pub send: u64,
    pub receive: u64,
}

#[derive(Clone, Copy)]
struct Link {
    authority: LinkAuthority,
    local: Endpoint,
    remote: Endpoint,
    peer: Option<NodeId>,
    nonce: [u8; 32],
    expires: u64,
    next_challenge: u64,
    next_send: u64,
    crypto_tick: Option<u64>,
    pending: Option<Packet>,
    pending_until: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiscoveryChange { Discovered(NodeId), Recovered(NodeId) }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LinkSnapshot {
    pub connection: u32,
    pub peer: Option<NodeId>,
    pub local: Endpoint,
    pub remote: Endpoint,
}

pub struct NodeTransport {
    seed: Option<[u8; 32]>,
    counter: u64,
    links: [Option<Link>; MAX_LINKS],
    cursor: usize,
    pub rejected_packets: u64,
    pub last_error: Option<NodeError>,
}

impl NodeTransport {
    // ------------------------=
    // FUNC: new
    // DESC: Creates no connections, authority, peer assumptions or traffic at boot.
    // ------------------=
    pub const fn new() -> Self {
        Self { seed: None, counter: 0, links: [None; MAX_LINKS], cursor: 0, rejected_packets: 0, last_error: None }
    }

    // ------------------------=
    // FUNC: initialize
    // DESC: Seeds challenges from fresh firmware entropy, never from the persisted node identity.
    // ------------------=
    pub fn initialize(&mut self, entropy: &[u8; 32], valid: bool) -> Result<(), NodeError> {
        if !valid || entropy.iter().all(|byte| *byte == 0) { return Err(NodeError::EntropyUnavailable); }
        if self.seed.is_some() { return Err(NodeError::UnsupportedState); }
        self.seed = Some(*entropy);
        Ok(())
    }

    // ------------------------=
    // FUNC: attach
    // DESC: Binds an already authorized connected datagram; registration cannot create network authority.
    // ------------------=
    pub fn attach(&mut self, authority: LinkAuthority, network: &NetworkRuntime, capabilities: &CapabilityManager, now: u64) -> Result<(), NetworkError> {
        let connection = network.inspect_connection(authority.connection, authority.owner, false)?;
        if connection.protocol != TransportProtocol::Datagram || connection.state != ConnectionState::Open { return Err(NetworkError::UnsupportedProtocol); }
        for (id, kind) in [(authority.send, CapabilityType::NetworkSend), (authority.receive, CapabilityType::NetworkReceive)] {
            capabilities.validate(id, authority.owner, kind, authority.connection as u64, 1, 0, now).map_err(|_| NetworkError::CapabilityRevoked)?;
        }
        if self.links.iter().flatten().any(|link| link.authority.connection == authority.connection) { return Err(NetworkError::AlreadyExists); }
        let slot = self.links.iter().position(Option::is_none).ok_or(NetworkError::ResourceLimitExceeded)?;
        self.links[slot] = Some(Link { authority, local: connection.local, remote: connection.remote, peer: None, nonce: [0; 32], expires: 0, next_challenge: now, next_send: now, crypto_tick: None, pending: None, pending_until: 0 });
        Ok(())
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns an owned link's actual configured endpoints and authenticated peer binding, never key material.
    // ------------------=
    pub fn inspect(&self, connection: u32, owner: SecurityIdentity) -> Option<LinkSnapshot> {
        self.links.iter().flatten().find(|link| link.authority.connection == connection && link.authority.owner == owner)
            .map(|link| LinkSnapshot { connection, peer: link.peer, local: link.local, remote: link.remote })
    }

    // ------------------------=
    // FUNC: detach
    // DESC: Removes transport state without erasing persistent peer trust or creating grants.
    // ------------------=
    pub fn detach(&mut self, connection: u32) {
        for slot in &mut self.links {
            if slot.map(|link| link.authority.connection == connection).unwrap_or(false) {
                if let Some(link) = slot.as_mut() { link.nonce.zeroize(); }
                *slot = None;
            }
        }
    }

    // ------------------------=
    // FUNC: poll
    // DESC: Services one link without waiting; performs at most one signature operation per link per second.
    // ------------------=
    pub fn poll(&mut self, nodes: &mut NodeRuntime, network: &mut NetworkRuntime, capabilities: &CapabilityManager, now: u64) -> Option<DiscoveryChange> {
        let seed = self.seed?;
        if !network.profiles.active().map(|profile| profile.local_discovery_enabled && profile.interfaces_enabled).unwrap_or(false) {
            for link in self.links.iter_mut().flatten() { link.pending = None; link.expires = 0; link.nonce.zeroize(); }
            return None;
        }
        let index = self.cursor;
        self.cursor = (self.cursor + 1) % MAX_LINKS;
        let link = self.links[index].as_mut()?;
        if now >= link.pending_until { link.pending = None; }
        let authority = link.authority;
        let mut change = None;
        if link.crypto_tick != Some(now) {
            if let Ok(packet) = network.receive_datagram(authority.owner, authority.receive, authority.connection, now, capabilities) {
                link.crypto_tick = Some(now);
                match process_packet(link, nodes, &packet.bytes[..packet.length as usize], now) {
                    Ok(result) => change = result,
                    Err(error) => {
                        self.last_error = Some(error);
                        self.rejected_packets = self.rejected_packets.saturating_add(1);
                    }
                }
            }
        }
        if now >= link.next_challenge && link.pending.is_none() {
            let Some(counter) = self.counter.checked_add(1) else { return change; };
            self.counter = counter;
            let mut hash = Sha256::new();
            hash.update(b"InfinityOS native discovery challenge v1");
            hash.update(seed);
            hash.update(counter.to_le_bytes());
            hash.update(authority.connection.to_le_bytes());
            link.nonce.copy_from_slice(&hash.finalize());
            link.expires = now.saturating_add(5);
            link.next_challenge = now.saturating_add(10);
        }
        if now >= link.next_send {
            let challenge = challenge_packet(link.nonce);
            let outgoing = link.pending.as_ref().or_else(|| if now < link.expires { Some(&challenge) } else { None });
            if let Some(packet) = outgoing {
                if network.send_datagram(authority.owner, authority.send, authority.connection, &packet.bytes[..packet.length as usize], now, now.saturating_add(1), capabilities).is_ok() { link.pending = None; }
            }
            link.next_send = now.saturating_add(1);
        }
        change
    }
}

// ------------------------=
// FUNC: challenge_packet
// DESC: Encodes one fixed-length freshness challenge without allocation.
// ------------------=
fn challenge_packet(nonce: [u8; 32]) -> Packet {
    let mut packet = Packet { bytes: [0; crate::runtime::network::types::MAX_PACKET_BYTES], length: 40 };
    packet.bytes[..8].copy_from_slice(CHALLENGE);
    packet.bytes[8..40].copy_from_slice(&nonce);
    packet
}

// ------------------------=
// FUNC: process_packet
// DESC: Authenticates a challenge-bound announcement before committing liveness, consuming each nonce once.
// ------------------=
fn process_packet(link: &mut Link, nodes: &mut NodeRuntime, bytes: &[u8], now: u64) -> Result<Option<DiscoveryChange>, NodeError> {
    if bytes.len() == 40 && &bytes[..8] == CHALLENGE {
        if link.pending.is_some() { return Err(NodeError::ResourceLimit); }
        let local = nodes.local_id.ok_or(NodeError::EntropyUnavailable)?;
        let public = nodes.crypto.public_identity().map_err(map_crypto_error)?;
        let mut packet = Packet { bytes: [0; crate::runtime::network::types::MAX_PACKET_BYTES], length: RESPONSE_BYTES as u16 };
        packet.bytes[..8].copy_from_slice(RESPONSE);
        packet.bytes[8..40].copy_from_slice(&bytes[8..40]);
        packet.bytes[40..72].copy_from_slice(&local.0);
        packet.bytes[72..104].copy_from_slice(&public);
        packet.bytes[104..106].copy_from_slice(&NODE_PROTOCOL_VERSION.to_le_bytes());
        packet.bytes[106..108].copy_from_slice(&NODE_PROTOCOL_VERSION.to_le_bytes());
        packet.bytes[116..135].copy_from_slice(&endpoint_bytes(link.local));
        let signature = nodes.crypto.sign(nodes.key_ref.ok_or(NodeError::EntropyUnavailable)?, &packet.bytes[..135]).map_err(map_crypto_error)?;
        packet.bytes[135..199].copy_from_slice(&signature);
        link.pending = Some(packet);
        link.pending_until = now.saturating_add(5);
        return Ok(None);
    }
    if bytes.len() != RESPONSE_BYTES || &bytes[..8] != RESPONSE { return Err(NodeError::InvalidAdvertisement); }
    if now >= link.expires || bytes[8..40] != link.nonce { return Err(NodeError::ReplayDetected); }
    let mut node = [0; 32]; node.copy_from_slice(&bytes[40..72]);
    let mut public = [0; 32]; public.copy_from_slice(&bytes[72..104]);
    let mut signature = [0; 64]; signature.copy_from_slice(&bytes[135..199]);
    if bytes[116..135] != endpoint_bytes(link.remote) { return Err(NodeError::IdentityMismatch); }
    if link.peer.map(|peer| peer != NodeId(node)).unwrap_or(false) { return Err(NodeError::IdentityMismatch); }
    NodeCrypto::verify(&public, &bytes[..135], &signature).map_err(map_crypto_error)?;
    let id = NodeId(node);
    let prior = nodes.discovered.iter().flatten().find(|peer| peer.id == id).copied();
    nodes.record_discovery(DiscoveryAdvertisement {
        node: id, public_key: public,
        protocol_min: u16::from_le_bytes([bytes[104], bytes[105]]),
        protocol_max: u16::from_le_bytes([bytes[106], bytes[107]]),
        service_bits: u64::from_le_bytes(bytes[108..116].try_into().map_err(|_| NodeError::InvalidAdvertisement)?),
        sequence: 0, expires_at: now.saturating_add(super::DISCOVERY_LEASE_TICKS), signature: [0; 64],
    }, now)?;
    link.peer = Some(id);
    link.expires = 0;
    link.nonce.zeroize();
    Ok(match prior {
        None => Some(DiscoveryChange::Discovered(id)),
        Some(peer) if peer.reachability != super::types::Reachability::Online => Some(DiscoveryChange::Recovered(id)),
        _ => None,
    })
}

// ------------------------=
// FUNC: endpoint_bytes
// DESC: Canonically encodes the reachable connected endpoint with explicit address family and network-order port.
// ------------------=
fn endpoint_bytes(endpoint: Endpoint) -> [u8; 19] {
    let mut bytes = [0; 19];
    match endpoint.address {
        IpAddress::V4(address) => { bytes[0] = 4; bytes[1..5].copy_from_slice(&address); }
        IpAddress::V6(address) => { bytes[0] = 6; bytes[1..17].copy_from_slice(&address); }
    }
    bytes[17..19].copy_from_slice(&endpoint.port.to_be_bytes());
    bytes
}
