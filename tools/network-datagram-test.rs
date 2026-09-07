//! Behavioral authority and lifecycle tests over the production native datagram path.
#[path = "nic-probe/runtime.rs"] mod runtime;
#[path = "nic-probe/fixture.rs"] mod fixture;
use runtime::network::{types::*, wire::Datagram};

// ------------------------=
// FUNC: main
// DESC: Asserts native send/receive authorization, bounded input, lease expiry and link invalidation.
// ------------------=
fn main() {
    let own = [10, 42, 0, 1]; let peer = [10, 42, 0, 2];
    let mut f = fixture::configured([2, 0, 0, 0, 0, 1], own, peer);
    let c = *f.network.connections.nth(0).unwrap();
    assert_eq!(f.network.connections.connect(f.owner, c.subject, c.capability_scope, c.local, c.remote, c.protocol, 3, 1, 10, &f.capabilities, &mut f.network.policy), Err(NetworkError::Conflict));
    let packet = Datagram { source: peer, destination: own, source_port: 49152, destination_port: 49152, bytes: [42; 512], length: 2 };
    assert_eq!(f.network.connections.deliver_datagram(packet, &mut f.network.policy, &f.capabilities, 1), Ok(()));
    let mut malformed = packet; malformed.length = 513;
    assert_eq!(f.network.connections.deliver_datagram(malformed, &mut f.network.policy, &f.capabilities, 1), Err(NetworkError::InvalidEndpoint));
    let mut unknown = packet; unknown.source[3] = 99;
    assert_eq!(f.network.connections.deliver_datagram(unknown, &mut f.network.policy, &f.capabilities, 1), Err(NetworkError::ConnectionRefused));
    assert_eq!(f.network.send_datagram(f.owner, f.send, f.connection, &[1], 1, 1, &f.capabilities), Err(NetworkError::ConnectionTimeout));
    assert_eq!(f.network.send_datagram(f.owner, f.send, f.connection, &[1], 1, 10, &f.capabilities), Err(NetworkError::AddressUnavailable));
    assert!(f.network.wire.peek_transmit().is_some()); // ARP, never a fabricated successful send.
    let outbound = (0..f.network.policy.count()).filter_map(|i| f.network.policy.nth(i)).find(|r| r.subject == Subject::Context(f.owner) && r.direction == Direction::Outbound).unwrap().id;
    f.network.policy.delete(outbound).unwrap();
    assert_eq!(f.network.send_datagram(f.owner, f.send, f.connection, &[1], 2, 10, &f.capabilities), Err(NetworkError::PolicyDenied));
    let inbound = (0..f.network.policy.count()).filter_map(|i| f.network.policy.nth(i)).find(|r| r.subject == Subject::Context(f.owner) && r.direction == Direction::Inbound).unwrap().id;
    f.network.policy.delete(inbound).unwrap();
    assert_eq!(f.network.connections.deliver_datagram(packet, &mut f.network.policy, &f.capabilities, 2), Err(NetworkError::PolicyDenied));
    assert_eq!(f.network.receive_datagram(f.owner, f.receive, f.connection, 2, &f.capabilities), Err(NetworkError::PolicyDenied));
    f.capabilities.revoke(f.send).unwrap();
    assert_eq!(f.network.send_datagram(f.owner, f.send, f.connection, &[1], 2, 10, &f.capabilities), Err(NetworkError::CapabilityRevoked));
    let mut f = fixture::configured([2, 0, 0, 0, 0, 1], own, peer);
    assert_eq!(f.network.connections.deliver_datagram(packet, &mut f.network.policy, &f.capabilities, 15), Err(NetworkError::CapabilityRevoked));
    let connect = f.network.connections.nth(0).unwrap().capability_scope;
    assert_eq!(f.network.connections.deliver_datagram(packet, &mut f.network.policy, &f.capabilities, 2), Ok(()));
    f.capabilities.revoke(connect).unwrap();
    assert_eq!(f.network.receive_datagram(f.owner, f.receive, f.connection, 2, &f.capabilities), Err(NetworkError::CapabilityRevoked));
    assert_eq!(f.network.connections.deliver_datagram(packet, &mut f.network.policy, &f.capabilities, 2), Err(NetworkError::CapabilityRevoked));
    assert_eq!(f.network.send_datagram(f.owner, f.send, f.connection, &[1], 2, 10, &f.capabilities), Err(NetworkError::CapabilityRevoked));
    f.network.connections.bind_native_address(None);
    assert_eq!(f.network.connections.count(), 0);
    f.network.connections.bind_native_address(Some(own));
    assert_eq!(f.network.connections.count(), 0); // Link recovery must not resurrect revoked authority.
}
