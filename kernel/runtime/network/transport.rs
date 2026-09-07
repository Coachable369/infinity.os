//! Typed connection transport with bounded per-connection queues. The current
//! implemented adapter is host-local loopback; wire adapters remain separable.

use super::policy::PolicyEngine;
use super::types::*;
use crate::runtime::capability::{CapabilityId, CapabilityManager, CapabilityType};
use crate::runtime::execution::SecurityIdentity;

#[derive(Clone, Copy)]
struct PacketQueue {
    entries: [Option<Packet>; CONNECTION_QUEUE_DEPTH],
    head: usize,
    len: usize,
}

impl PacketQueue {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded packet queue.
    // ------------------=
    const fn new() -> Self {
        Self {
            entries: [None; CONNECTION_QUEUE_DEPTH],
            head: 0,
            len: 0,
        }
    }

    // ------------------------=
    // FUNC: push
    // DESC: Enqueues one packet or applies explicit backpressure at capacity.
    // ------------------=
    fn push(&mut self, packet: Packet, limit: usize) -> Result<(), NetworkError> {
        if self.len >= limit.min(CONNECTION_QUEUE_DEPTH).max(1) {
            return Err(NetworkError::QueueFull);
        }
        let at = (self.head + self.len) % CONNECTION_QUEUE_DEPTH;
        self.entries[at] = Some(packet);
        self.len += 1;
        Ok(())
    }

    // ------------------------=
    // FUNC: pop
    // DESC: Dequeues the oldest packet in deterministic order.
    // ------------------=
    fn pop(&mut self) -> Option<Packet> {
        if self.len == 0 {
            return None;
        }
        let packet = self.entries[self.head].take();
        self.head = (self.head + 1) % CONNECTION_QUEUE_DEPTH;
        self.len -= 1;
        packet
    }
}

pub struct ConnectionManager {
    connections: [Option<Connection>; MAX_CONNECTIONS],
    receive: [PacketQueue; MAX_CONNECTIONS],
    next_id: ConnectionId,
    failures: u64,
    queue_pressure: u64,
}

impl ConnectionManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded typed connection table and queue set.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            connections: [None; MAX_CONNECTIONS],
            receive: [PacketQueue::new(); MAX_CONNECTIONS],
            next_id: 1,
            failures: 0,
            queue_pressure: 0,
        }
    }

    // ------------------------=
    // FUNC: connect
    // DESC: Authorizes, routes, and opens a typed loopback connection under a deadline.
    // ------------------=
    pub fn connect(
        &mut self,
        owner: SecurityIdentity,
        subject: Subject,
        capability: CapabilityId,
        local: Endpoint,
        remote: Endpoint,
        protocol: TransportProtocol,
        queue_limit: u8,
        now: u64,
        deadline: u64,
        capabilities: &CapabilityManager,
        policies: &mut PolicyEngine,
    ) -> Result<ConnectionId, NetworkError> {
        if deadline <= now {
            self.failures = self.failures.saturating_add(1);
            return Err(NetworkError::ConnectionTimeout);
        }
        capabilities
            .validate(
                capability,
                owner,
                CapabilityType::NetworkConnect,
                0,
                1,
                0,
                now,
            )
            .map_err(|e| match e {
                crate::runtime::capability::CapabilityError::Revoked
                | crate::runtime::capability::CapabilityError::Expired => {
                    NetworkError::CapabilityRevoked
                }
                _ => NetworkError::AccessDenied,
            })?;
        if !remote.address.is_host_local() {
            self.failures = self.failures.saturating_add(1);
            return Err(NetworkError::UnsupportedOperation);
        }
        let decision = policies.evaluate(
            subject,
            Direction::Outbound,
            Some(1),
            local,
            remote,
            protocol,
            now,
        );
        if !matches!(
            decision.action,
            PolicyAction::Allow | PolicyAction::AuditOnly
        ) {
            return Err(NetworkError::PolicyDenied);
        }
        let slot = self
            .connections
            .iter()
            .position(Option::is_none)
            .ok_or(NetworkError::ResourceLimitExceeded)?;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.connections[slot] = Some(Connection {
            id,
            owner,
            subject,
            protocol,
            local,
            remote,
            state: ConnectionState::Open,
            security: SecurityState::Plain,
            capability_scope: capability,
            queue_limit: queue_limit.min(CONNECTION_QUEUE_DEPTH as u8).max(1),
            statistics: ConnectionStatistics {
                opened_at: now,
                ..ConnectionStatistics::default()
            },
            future_node_identity: None,
        });
        Ok(id)
    }

    // ------------------------=
    // FUNC: secure_fixture
    // DESC: Applies deterministic fixture identity verification without claiming production TLS assurance.
    // ------------------=
    pub fn secure_fixture(
        &mut self,
        id: ConnectionId,
        expected_identity: u64,
        observed_identity: u64,
    ) -> Result<(), NetworkError> {
        let connection = self.connection_mut(id)?;
        if expected_identity != observed_identity {
            connection.security = SecurityState::VerificationFailed;
            return Err(NetworkError::IdentityVerificationFailed);
        }
        connection.security = SecurityState::VerifiedFixture;
        Ok(())
    }

    // ------------------------=
    // FUNC: send
    // DESC: Revalidates revocable authority and enqueues bounded loopback data.
    // ------------------=
    pub fn send(
        &mut self,
        owner: SecurityIdentity,
        capability: CapabilityId,
        id: ConnectionId,
        data: &[u8],
        now: u64,
        capabilities: &CapabilityManager,
    ) -> Result<(), NetworkError> {
        capabilities
            .validate(
                capability,
                owner,
                CapabilityType::NetworkSend,
                id as u64,
                1,
                0,
                now,
            )
            .map_err(|e| match e {
                crate::runtime::capability::CapabilityError::Revoked
                | crate::runtime::capability::CapabilityError::Expired => {
                    NetworkError::CapabilityRevoked
                }
                _ => NetworkError::AccessDenied,
            })?;
        let slot = self
            .connections
            .iter()
            .position(|v| v.map(|c| c.id) == Some(id))
            .ok_or(NetworkError::ConnectionClosed)?;
        let connection = self.connections[slot].as_mut().unwrap();
        if connection.state != ConnectionState::Open {
            return Err(NetworkError::ConnectionClosed);
        }
        let packet = Packet::from_slice(data)?;
        if self.receive[slot]
            .push(packet, connection.queue_limit as usize)
            .is_err()
        {
            connection.statistics.queue_drops = connection.statistics.queue_drops.saturating_add(1);
            self.queue_pressure = self.queue_pressure.saturating_add(1);
            return Err(NetworkError::QueueFull);
        }
        connection.statistics.bytes_sent = connection
            .statistics
            .bytes_sent
            .saturating_add(data.len() as u64);
        connection.statistics.messages_sent = connection.statistics.messages_sent.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: receive
    // DESC: Revalidates authority and returns the oldest queued typed packet.
    // ------------------=
    pub fn receive(
        &mut self,
        owner: SecurityIdentity,
        capability: CapabilityId,
        id: ConnectionId,
        now: u64,
        capabilities: &CapabilityManager,
    ) -> Result<Packet, NetworkError> {
        capabilities
            .validate(
                capability,
                owner,
                CapabilityType::NetworkReceive,
                id as u64,
                1,
                0,
                now,
            )
            .map_err(|e| match e {
                crate::runtime::capability::CapabilityError::Revoked
                | crate::runtime::capability::CapabilityError::Expired => {
                    NetworkError::CapabilityRevoked
                }
                _ => NetworkError::AccessDenied,
            })?;
        let slot = self
            .connections
            .iter()
            .position(|v| v.map(|c| c.id) == Some(id))
            .ok_or(NetworkError::ConnectionClosed)?;
        let packet = self.receive[slot]
            .pop()
            .ok_or(NetworkError::NetworkUnavailable)?;
        let connection = self.connections[slot].as_mut().unwrap();
        connection.statistics.bytes_received = connection
            .statistics
            .bytes_received
            .saturating_add(packet.length as u64);
        connection.statistics.messages_received =
            connection.statistics.messages_received.saturating_add(1);
        Ok(packet)
    }

    // ------------------------=
    // FUNC: close
    // DESC: Closes a connection and reclaims its bounded queue.
    // ------------------=
    pub fn close(&mut self, id: ConnectionId) -> Result<(), NetworkError> {
        let slot = self
            .connections
            .iter()
            .position(|v| v.map(|c| c.id) == Some(id))
            .ok_or(NetworkError::ConnectionClosed)?;
        self.connections[slot] = None;
        self.receive[slot] = PacketQueue::new();
        Ok(())
    }

    // ------------------------=
    // FUNC: connection_mut
    // DESC: Resolves mutable connection state by stable identifier.
    // ------------------=
    fn connection_mut(&mut self, id: ConnectionId) -> Result<&mut Connection, NetworkError> {
        self.connections
            .iter_mut()
            .flatten()
            .find(|c| c.id == id)
            .ok_or(NetworkError::ConnectionClosed)
    }

    // ------------------------=
    // FUNC: count
    // DESC: Returns the active connection count.
    // ------------------=
    pub fn count(&self) -> usize {
        self.connections.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: nth
    // DESC: Returns one connection for authorized inspection.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<&Connection> {
        self.connections.iter().flatten().nth(index)
    }

    // ------------------------=
    // FUNC: failures
    // DESC: Returns observed connection setup failures.
    // ------------------=
    pub const fn failures(&self) -> u64 {
        self.failures
    }

    // ------------------------=
    // FUNC: queue_pressure
    // DESC: Returns observed queue-capacity pressure events.
    // ------------------=
    pub const fn queue_pressure(&self) -> u64 {
        self.queue_pressure
    }
}
