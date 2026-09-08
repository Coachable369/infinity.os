//! Reconciles persisted, explicitly approved peer endpoints into bounded runtime-owned network authority.
use super::{capability::{CapabilityManager, CapabilityType}, execution::SecurityIdentity, network::{NetworkRuntime, policy::{EndpointSelector, NetworkPolicyRule}, types::*}, node::{NodeRuntime, transport::NodeTransport, link_config::{LinkConfiguration, MAX_CONFIGURED_LINKS}}};

#[derive(Clone, Copy)]
struct Applied { configuration: LinkConfiguration, connection: u32, connect: u64, policies: [PolicyId; 2] }
pub struct NodeLinks {
    tick: Option<u64>, applied: [Option<Applied>; MAX_CONFIGURED_LINKS],
    pub errors: [Option<NetworkError>; MAX_CONFIGURED_LINKS],
}
impl NodeLinks {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no network authority; only durable endpoint intent may create an applied link.
    // ------------------=
    pub const fn new() -> Self { Self { tick: None, applied: [None; MAX_CONFIGURED_LINKS], errors: [None; MAX_CONFIGURED_LINKS] } }

    // ------------------------=
    // FUNC: reconcile
    // DESC: Applies or removes explicit endpoint intent once per second, cleans failed attempts, and retries after offline recovery without granting trust.
    // ------------------=
    pub fn reconcile(&mut self, nodes: &mut NodeRuntime, transport: &mut NodeTransport, network: &mut NetworkRuntime, caps: &mut CapabilityManager, owner: SecurityIdentity, now: u64) {
        if self.tick == Some(now) { return; } self.tick = Some(now);
        for index in 0..MAX_CONFIGURED_LINKS {
            let desired = nodes.configured_links()[index];
            if let Some(applied) = self.applied[index] {
                if desired != Some(applied.configuration) || network.inspect_connection(applied.connection, owner, false).map(|c| c.state != ConnectionState::Open).unwrap_or(true) {
                    transport.release(nodes, network, caps, applied.connection, owner, now);
                    let _ = caps.retire_leaf(applied.connect, owner);
                    for policy in applied.policies { let _ = network.policy.delete(policy); }
                    self.applied[index] = None;
                }
            }
            if self.applied[index].is_none() {
                self.errors[index] = None;
                if let Some(configuration) = desired {
                    match provision(configuration, transport, network, caps, owner, now) {
                        Ok(applied) => self.applied[index] = Some(applied),
                        Err(error) => self.errors[index] = Some(error),
                    }
                }
            }
        }
    }
}

// ------------------------=
// FUNC: provision
// DESC: Installs only the operator's exact address/port datagram scope, preserving stronger policy denies and rolling back partial resource allocation.
// ------------------=
fn provision(configuration: LinkConfiguration, transport: &mut NodeTransport, network: &mut NetworkRuntime, caps: &mut CapabilityManager, owner: SecurityIdentity, now: u64) -> Result<Applied, NetworkError> {
    let local = Endpoint { address: IpAddress::V4(configuration.local), port: configuration.local_port };
    let remote = Endpoint { address: IpAddress::V4(configuration.remote), port: configuration.remote_port };
    let mut policies = [0; 2];
    for (index, direction) in [Direction::Inbound, Direction::Outbound].into_iter().enumerate() {
        match network.policy.create(NetworkPolicyRule { id: 0, subject: Subject::Context(owner), direction, interface_id: Some(2), local: EndpointSelector { network: Some(local.address), prefix_length: 32, port: Some(local.port), protocol: Some(TransportProtocol::Datagram), local_only: false }, remote: EndpointSelector { network: Some(remote.address), prefix_length: 32, port: Some(remote.port), protocol: Some(TransportProtocol::Datagram), local_only: false }, action: PolicyAction::Allow, priority: 1000, logging: LoggingMode::Decisions, enabled: true, expires_at: None, policy_source: 0x4e4c4e4b }) {
            Ok(id) => policies[index] = id,
            Err(error) => { for id in policies { if id != 0 { let _ = network.policy.delete(id); } } return Err(error); }
        }
    }
    let connect = match caps.grant(CapabilityType::NetworkConnect, 0, 1, 0, owner, owner, None, 0) {
        Ok(cap) => cap,
        Err(_) => { for id in policies { let _ = network.policy.delete(id); } return Err(NetworkError::ResourceLimitExceeded); }
    };
    match transport.provision(network, caps, owner, connect, local, remote, now) {
        Ok(connection) => Ok(Applied { configuration, connection, connect, policies }),
        Err(error) => { let _ = caps.retire_leaf(connect, owner); for id in policies { let _ = network.policy.delete(id); } Err(error) }
    }
}
