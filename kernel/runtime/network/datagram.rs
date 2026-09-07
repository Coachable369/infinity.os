//! Native datagram operations reuse the connection/policy/route managers.
use super::{NetworkRuntime, types::*, wire::WireError};
use crate::runtime::{capability::{CapabilityManager, CapabilityId, CapabilityType}, execution::SecurityIdentity};

impl NetworkRuntime {
    // ------------------------=
    // FUNC: receive_datagram
    // DESC: Rechecks current inbound policy when dequeuing, so queued data cannot bypass a policy change.
    // ------------------=
    pub fn receive_datagram(&mut self, owner: SecurityIdentity, capability: CapabilityId, connection_id: ConnectionId, now: u64, capabilities: &CapabilityManager) -> Result<Packet, NetworkError> {
        let connection = self.inspect_connection(connection_id, owner, false)?;
        if connection.protocol != TransportProtocol::Datagram { return Err(NetworkError::UnsupportedProtocol); }
        let decision = self.policy.evaluate(connection.subject, Direction::Inbound, Some(2), connection.local, connection.remote, connection.protocol, now);
        if !matches!(decision.action, PolicyAction::Allow | PolicyAction::AuditOnly) { return Err(NetworkError::PolicyDenied); }
        self.connections.receive(owner, capability, connection_id, now, capabilities)
    }
    // ------------------------=
    // FUNC: send_datagram
    // DESC: Revalidates current authority, endpoint policy, route and deadline before bounded native UDP submission.
    // ------------------=
    pub fn send_datagram(&mut self, owner: SecurityIdentity, capability: CapabilityId, connection_id: ConnectionId, bytes: &[u8], now: u64, deadline: u64, capabilities: &CapabilityManager) -> Result<(), NetworkError> {
        if now >= deadline { return Err(NetworkError::ConnectionTimeout); }
        capabilities.validate(capability, owner, CapabilityType::NetworkSend, u64::from(connection_id), 1, 0, now).map_err(|_| NetworkError::CapabilityRevoked)?;
        let connection = self.inspect_connection(connection_id, owner, false)?;
        capabilities.validate(connection.capability_scope, owner, CapabilityType::NetworkConnect, 0, 1, 0, now).map_err(|_| NetworkError::CapabilityRevoked)?;
        if connection.owner != owner || connection.state != ConnectionState::Open { return Err(NetworkError::AccessDenied); }
        if connection.protocol != TransportProtocol::Datagram { return Err(NetworkError::UnsupportedProtocol); }
        if !self.profiles.active().map(|p| p.interfaces_enabled).unwrap_or(false) { return Err(NetworkError::NetworkUnavailable); }
        let destination = match connection.remote.address { IpAddress::V4(ip) => ip, _ => return Err(NetworkError::UnsupportedProtocol) };
        let route = self.interfaces.select_route(connection.remote.address, None)?;
        if route.interface_id != 2 { return Err(NetworkError::NoRoute); }
        let next_hop = match route.next_hop.unwrap_or(connection.remote.address) { IpAddress::V4(ip) => ip, _ => return Err(NetworkError::UnsupportedProtocol) };
        let decision = self.policy.evaluate(connection.subject, Direction::Outbound, Some(2), connection.local, connection.remote, TransportProtocol::Datagram, now);
        if !matches!(decision.action, PolicyAction::Allow | PolicyAction::AuditOnly) { return Err(NetworkError::PolicyDenied); }
        self.wire.send(destination, next_hop, connection.local.port, connection.remote.port, bytes, now).map_err(|error| match error {
            WireError::QueueFull => NetworkError::QueueFull,
            WireError::AddressUnresolved | WireError::Unconfigured => NetworkError::AddressUnavailable,
            WireError::Unsupported => NetworkError::UnsupportedProtocol,
            WireError::InvalidFrame => NetworkError::InvalidEndpoint,
        })
    }
}
