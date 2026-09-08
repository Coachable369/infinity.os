//! Explicit engineering/operator network configuration; not an installed default policy.
use crate::runtime::{
    capability::{CapabilityManager, CapabilityType},
    execution::SecurityIdentity,
    network::{
        policy::{EndpointSelector, NetworkPolicyRule},
        types::*,
        NetworkRuntime,
    },
    node::{transport::NodeTransport, NodeRuntime},
};
pub struct Fixture {
    pub iop: crate::runtime::iop::IopRouter,
    pub execute_remote: bool,
    pub network: NetworkRuntime,
    pub capabilities: CapabilityManager,
    pub owner: SecurityIdentity,
    pub connection: u32,
    pub nodes: NodeRuntime,
    pub transport: NodeTransport,
}

// ------------------------=
// FUNC: engineering_pairing_writer
// DESC: Explicit in-memory fixture boundary; this is never an installed durability claim or production fallback.
// ------------------=
fn engineering_pairing_writer(_: &[u8; crate::runtime::node::types::NODE_STATE_BYTES]) -> bool { true }

// ------------------------=
// FUNC: configured
// DESC: Provides explicit test deployment policy, then provisions the real production connected endpoint and independent node identity.
// ------------------=
pub fn configured(mac: [u8; 6], entropy: [u8; 32]) -> Fixture {
    let own = [10, 42, 0, mac[5]];
    let peer = [10, 42, 0, 3 - mac[5]];
    let mut network = NetworkRuntime::new();
    network.initialize().unwrap();
    network
        .register_firmware_device(FirmwareNetworkDevice {
            firmware_handle: 0,
            device_id: 0x100e8086,
            hardware_address: Some(mac),
            link_state: LinkState::Up,
            maximum_frame_size: 1500,
            can_receive: true,
            can_transmit: true,
        })
        .unwrap();
    network
        .interfaces
        .add_address(
            2,
            IpAddress::V4(own),
            24,
            AddressScope::Private,
            AddressSource::Static,
            None,
            None,
        )
        .unwrap();
    network
        .interfaces
        .add_route(
            IpAddress::V4([10, 42, 0, 0]),
            24,
            None,
            2,
            0,
            RouteSource::Static,
            None,
        )
        .unwrap();
    network.wire.configure(mac, own);
    network.connections.bind_native_address(Some(own));
    let owner = SecurityIdentity([0x51; 16]);
    for direction in [Direction::Inbound, Direction::Outbound] {
        network
            .policy
            .create(NetworkPolicyRule {
                id: 0,
                subject: Subject::Context(owner),
                direction,
                interface_id: Some(2),
                local: EndpointSelector {
                    network: Some(IpAddress::V4(own)),
                    prefix_length: 32,
                    port: Some(49152),
                    protocol: Some(TransportProtocol::Datagram),
                    local_only: false,
                },
                remote: EndpointSelector {
                    network: Some(IpAddress::V4(peer)),
                    prefix_length: 32,
                    port: Some(49152),
                    protocol: Some(TransportProtocol::Datagram),
                    local_only: false,
                },
                action: PolicyAction::Allow,
                priority: 0,
                logging: LoggingMode::Decisions,
                enabled: true,
                expires_at: Some(1200),
                policy_source: 0x54455354,
            })
            .unwrap();
    }
    let mut capabilities = CapabilityManager::new();
    let connect = capabilities
        .grant(
            CapabilityType::NetworkConnect,
            0,
            1,
            0,
            owner,
            owner,
            Some(1200),
            0,
        )
        .unwrap();
    let mut transport = NodeTransport::new();
    transport.initialize(&entropy, true).unwrap();
    transport.trust.persist_pairing = engineering_pairing_writer;
    transport.persist_discovery = engineering_pairing_writer;
    let connection = transport
        .provision(
            &mut network,
            &mut capabilities,
            owner,
            connect,
            Endpoint {
                address: IpAddress::V4(own),
                port: 49152,
            },
            Endpoint {
                address: IpAddress::V4(peer),
                port: 49152,
            },
            0,
        )
        .unwrap();
    let mut nodes = NodeRuntime::new();
    nodes.initialize(&entropy, true).unwrap();
    // Engineering-only in-memory service lifecycle. Installed runtime leaves
    // mutation readiness false until durable commit and IEF integration exists.
    let mut iop = crate::runtime::iop::IopRouter::new();
    iop.remote.set_mutation_service_ready(true);
    Fixture {
        iop,
        execute_remote: true,
        network,
        capabilities,
        owner,
        connection,
        nodes,
        transport,
    }
}
