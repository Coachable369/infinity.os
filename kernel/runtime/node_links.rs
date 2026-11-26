//! Reconciles configured and bounded LAN candidate endpoints; identity trust remains explicitly approved.
use super::{
    capability::{CapabilityManager, CapabilityType},
    execution::SecurityIdentity,
    network::{
        policy::{EndpointSelector, NetworkPolicyRule},
        types::*,
        NetworkRuntime,
    },
    node::{
        link_config::{LinkConfiguration, MAX_CONFIGURED_LINKS},
        transport::NodeTransport,
        NodeRuntime,
    },
};

#[derive(Clone, Copy)]
struct Applied {
    configuration: LinkConfiguration,
    connection: u32,
    connect: u64,
    policies: [PolicyId; 2],
}
pub struct NodeLinks {
    tick: Option<u64>,
    next_beacon: u64,
    nearby: [Option<(LinkConfiguration,u64)>; MAX_CONFIGURED_LINKS],
    applied: [Option<Applied>; MAX_CONFIGURED_LINKS],
    pub errors: [Option<NetworkError>; MAX_CONFIGURED_LINKS],
}
impl NodeLinks {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no network authority; configured or discovery candidates may create a transport link, never trust.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            tick: None,
            next_beacon: 0,
            nearby: [None; MAX_CONFIGURED_LINKS],
            applied: [None; MAX_CONFIGURED_LINKS],
            errors: [None; MAX_CONFIGURED_LINKS],
        }
    }

    // ------------------------=
    // FUNC: advertise_nearby
    // DESC: Sends a small bounded LAN invitation only when the active network profile permits local discovery; grants no trust.
    // ------------------=
    pub fn advertise_nearby(&mut self, network: &mut NetworkRuntime, address: [u8;4], now: u64) {
        if address==[0;4] || now<self.next_beacon || !network.profiles.active().map(|p|p.local_discovery_enabled && p.interfaces_enabled).unwrap_or(false) { return; }
        self.next_beacon=now.saturating_add(5);
        let _=network.wire.send([255;4],[255;4],47777,47777,b"INFINITY-FABRIC-1",now);
    }

    // ------------------------=
    // FUNC: receive_nearby
    // DESC: Treats local-subnet invitations as temporary endpoints only; signed challenge exchange still authenticates every discovered identity.
    // ------------------=
    pub fn receive_nearby(&mut self, network: &NetworkRuntime, nodes: &NodeRuntime,
        packet: &super::network::wire::Datagram, address: [u8;4], now: u64) -> bool {
        if packet.destination_port!=47777 { return false; }
        if packet.source_port!=47777 || packet.length!=17 || &packet.bytes[..17]!=b"INFINITY-FABRIC-1"
            || address==[0;4] || packet.source==address || packet.source[0]==0 || packet.source[0]==127 || packet.source[0]>=224
            || !network.profiles.active().map(|p|p.local_discovery_enabled && p.interfaces_enabled).unwrap_or(false) { return true; }
        let prefix=(0..network.interfaces.address_count()).filter_map(|i|network.interfaces.address_nth(i))
            .find(|a|a.interface_id==2 && a.address==IpAddress::V4(address)).map(|a|a.prefix_length);
        let Some(prefix)=prefix.filter(|p|*p>0 && *p<=32) else { return true; };
        let mask=u32::MAX << (32-prefix);
        let remote=u32::from_be_bytes(packet.source); let local=u32::from_be_bytes(address);
        if remote & mask != local & mask || (prefix<31 && (remote & !mask==!mask || remote & !mask==0)) { return true; }
        if nodes.configured_links().iter().flatten().any(|c|c.remote==packet.source) { return true; }
        let index=self.nearby.iter().position(|v|v.map(|(c,_)|c.remote==packet.source).unwrap_or(false))
            .or_else(|| (0..MAX_CONFIGURED_LINKS).find(|i| nodes.configured_links()[*i].is_none()
                && self.nearby[*i].map(|(_,expires)|now>=expires).unwrap_or(true)));
        if let Some(index)=index {
            self.nearby[index]=Some((LinkConfiguration { local:address, remote:packet.source,
                local_port:47778, remote_port:47778, revision:1 },now.saturating_add(30)));
        }
        true
    }

    // ------------------------=
    // FUNC: reconcile
    // DESC: Applies or removes explicit endpoint intent once per second, cleans failed attempts, and retries after offline recovery without granting trust.
    // ------------------=
    pub fn reconcile(
        &mut self,
        nodes: &mut NodeRuntime,
        transport: &mut NodeTransport,
        network: &mut NetworkRuntime,
        caps: &mut CapabilityManager,
        owner: SecurityIdentity,
        now: u64,
    ) {
        if self.tick == Some(now) {
            return;
        }
        self.tick = Some(now);
        for index in 0..MAX_CONFIGURED_LINKS {
            let discovery_enabled=network.profiles.active().map(|p| p.local_discovery_enabled && p.interfaces_enabled).unwrap_or(false);
            if !discovery_enabled || self.nearby[index].map(|(_,expires)|now>=expires).unwrap_or(false) { self.nearby[index]=None; }
            let desired = nodes.configured_links()[index].or(self.nearby[index].map(|(config,_)|config));
            if let Some(applied) = self.applied[index] {
                if desired != Some(applied.configuration)
                    || network
                        .inspect_connection(applied.connection, owner, false)
                        .map(|c| c.state != ConnectionState::Open)
                        .unwrap_or(true)
                {
                    transport.release(nodes, network, caps, applied.connection, owner, now);
                    let _ = caps.retire_leaf(applied.connect, owner);
                    for policy in applied.policies {
                        let _ = network.policy.delete(policy);
                    }
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
// DESC: Installs an exact candidate address/port datagram scope, preserving stronger policy denies and rolling back partial resource allocation.
// ------------------=
fn provision(
    configuration: LinkConfiguration,
    transport: &mut NodeTransport,
    network: &mut NetworkRuntime,
    caps: &mut CapabilityManager,
    owner: SecurityIdentity,
    now: u64,
) -> Result<Applied, NetworkError> {
    let local = Endpoint {
        address: IpAddress::V4(configuration.local),
        port: configuration.local_port,
    };
    let remote = Endpoint {
        address: IpAddress::V4(configuration.remote),
        port: configuration.remote_port,
    };
    let mut policies = [0; 2];
    for (index, direction) in [Direction::Inbound, Direction::Outbound]
        .into_iter()
        .enumerate()
    {
        match network.policy.create(NetworkPolicyRule {
            id: 0,
            subject: Subject::Context(owner),
            direction,
            interface_id: Some(2),
            local: EndpointSelector {
                network: Some(local.address),
                prefix_length: 32,
                port: Some(local.port),
                protocol: Some(TransportProtocol::Datagram),
                local_only: false,
            },
            remote: EndpointSelector {
                network: Some(remote.address),
                prefix_length: 32,
                port: Some(remote.port),
                protocol: Some(TransportProtocol::Datagram),
                local_only: false,
            },
            action: PolicyAction::Allow,
            priority: 1000,
            logging: LoggingMode::Decisions,
            enabled: true,
            expires_at: None,
            policy_source: 0x4e4c4e4b,
        }) {
            Ok(id) => policies[index] = id,
            Err(error) => {
                for id in policies {
                    if id != 0 {
                        let _ = network.policy.delete(id);
                    }
                }
                return Err(error);
            }
        }
    }
    let connect = match caps.grant(
        CapabilityType::NetworkConnect,
        0,
        1,
        0,
        owner,
        owner,
        None,
        0,
    ) {
        Ok(cap) => cap,
        Err(_) => {
            for id in policies {
                let _ = network.policy.delete(id);
            }
            return Err(NetworkError::ResourceLimitExceeded);
        }
    };
    match transport.provision(network, caps, owner, connect, local, remote, now) {
        Ok(connection) => Ok(Applied {
            configuration,
            connection,
            connect,
            policies,
        }),
        Err(error) => {
            let _ = caps.retire_leaf(connect, owner);
            for id in policies {
                let _ = network.policy.delete(id);
            }
            Err(error)
        }
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    // ------------------------=
    // FUNC: invitations_are_bounded_local_and_never_grant_trust
    // DESC: Exercises LAN eligibility, duplicate handling, capacity, and profile opt-out with typed packet state.
    // ------------------=
    #[test]
    fn invitations_are_bounded_local_and_never_grant_trust() {
        let mut network=NetworkRuntime::new(); network.initialize().unwrap();
        let mut interface=*network.interfaces.interface(1).unwrap(); interface.id=2;
        network.interfaces.add_interface(interface).unwrap();
        let address=[192,168,10,1];
        network.interfaces.add_address(2,IpAddress::V4(address),24,AddressScope::Private,AddressSource::Static,None,None).unwrap();
        network.wire.configure([2,0,0,0,0,1],address);
        let mut receiver=super::super::network::wire::WireNetwork::new();
        receiver.configure([2,0,0,0,0,2],[192,168,10,2]);
        let nodes=NodeRuntime::new(); let mut links=NodeLinks::new();
        links.advertise_nearby(&mut network,address,1);
        let frame=*network.wire.peek_transmit().unwrap();
        receiver.ingest(&frame.bytes[..frame.length],1).unwrap();
        let received=receiver.receive_datagram().unwrap();
        assert_eq!(received.source,address);
        assert_eq!(received.destination_port,47777);
        assert_eq!(&received.bytes[..received.length],b"INFINITY-FABRIC-1");
        network.wire.complete_transmit();
        links.advertise_nearby(&mut network,address,2);
        assert!(network.wire.peek_transmit().is_none());
        let mut packet=super::super::network::wire::Datagram {
            source:[192,168,10,2],destination:[255;4],source_port:47777,destination_port:47777,
            bytes:[0;super::super::network::wire::DATAGRAM_BYTES],length:17 };
        packet.bytes[..17].copy_from_slice(b"INFINITY-FABRIC-1");
        assert!(links.receive_nearby(&network,&nodes,&packet,address,10));
        assert_eq!(links.nearby.iter().flatten().count(),1);
        assert_eq!(links.nearby[0].unwrap().1,40);
        links.receive_nearby(&network,&nodes,&packet,address,12);
        assert_eq!(links.nearby.iter().flatten().count(),1);
        assert_eq!(links.nearby[0].unwrap().1,42);
        for source in [address,[192,168,11,2],[192,168,10,255],[192,168,10,0]] {
            packet.source=source; links.receive_nearby(&network,&nodes,&packet,address,13);
            assert_eq!(links.nearby.iter().flatten().count(),1);
        }
        for host in 3..12 { packet.source=[192,168,10,host]; links.receive_nearby(&network,&nodes,&packet,address,14); }
        assert_eq!(links.nearby.iter().flatten().count(),MAX_CONFIGURED_LINKS);
        assert_eq!(nodes.discovered_nodes().iter().flatten().count(),0);
        packet.source=[192,168,10,15]; links.receive_nearby(&network,&nodes,&packet,address,50);
        assert_eq!(links.nearby[0].unwrap().0.remote,packet.source);
        network.activate_profile(2).unwrap();
        let mut restricted=NodeLinks::new(); restricted.receive_nearby(&network,&nodes,&packet,address,51);
        assert!(restricted.nearby.iter().all(Option::is_none));
    }
}
