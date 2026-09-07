//! Explicit reference-network configuration and short-lived test authority.
use crate::runtime::{network::{NetworkRuntime, types::*, policy::{NetworkPolicyRule, EndpointSelector}}, capability::{CapabilityManager, CapabilityType}, execution::SecurityIdentity};

pub struct Fixture { pub network: NetworkRuntime, pub capabilities: CapabilityManager, pub owner: SecurityIdentity, pub connection: u32, pub send: u64, pub receive: u64 }

// ------------------------=
// FUNC: configured
// DESC: Creates explicit test-only address/routes and peer-scoped policies; never part of installed bootstrap defaults.
// ------------------=
pub fn configured(mac: [u8; 6], own: [u8; 4], peer: [u8; 4]) -> Fixture {
    let mut network = NetworkRuntime::new(); network.initialize().unwrap();
    network.register_firmware_device(FirmwareNetworkDevice { firmware_handle: 0, device_id: 0x100e8086, hardware_address: Some(mac), link_state: LinkState::Up, maximum_frame_size: 1500, can_receive: true, can_transmit: true }).unwrap();
    network.interfaces.add_address(2, IpAddress::V4(own), 24, AddressScope::Private, AddressSource::TestFixture, None, None).unwrap();
    network.interfaces.add_route(IpAddress::V4([10, 42, 0, 0]), 24, None, 2, 0, RouteSource::Static, None).unwrap();
    network.wire.configure(mac, own); network.connections.bind_native_address(Some(own));
    let owner = SecurityIdentity([0x51; 16]);
    for direction in [Direction::Inbound, Direction::Outbound] {
        network.policy.create(NetworkPolicyRule {
            id: 0, subject: Subject::Context(owner), direction, interface_id: Some(2),
            local: EndpointSelector { network: Some(IpAddress::V4(own)), prefix_length: 32, port: Some(49152), protocol: Some(TransportProtocol::Datagram), local_only: false },
            remote: EndpointSelector { network: Some(IpAddress::V4(peer)), prefix_length: 32, port: Some(49152), protocol: Some(TransportProtocol::Datagram), local_only: false },
            action: PolicyAction::Allow, priority: 0, logging: LoggingMode::Decisions, enabled: true, expires_at: Some(15), policy_source: 0x54455354,
        }).unwrap();
    }
    let mut capabilities = CapabilityManager::new();
    let connect = capabilities.grant(CapabilityType::NetworkConnect, 0, 1, 0, owner, owner, Some(15), 0).unwrap();
    let connection = network.connections.connect(owner, Subject::Context(owner), connect, Endpoint { address: IpAddress::V4(own), port: 49152 }, Endpoint { address: IpAddress::V4(peer), port: 49152 }, TransportProtocol::Datagram, 3, 0, 15, &capabilities, &mut network.policy).unwrap();
    let send = capabilities.grant(CapabilityType::NetworkSend, connection as u64, 1, 0, owner, owner, Some(15), 0).unwrap();
    let receive = capabilities.grant(CapabilityType::NetworkReceive, connection as u64, 1, 0, owner, owner, Some(15), 0).unwrap();
    Fixture { network, capabilities, owner, connection, send, receive }
}
