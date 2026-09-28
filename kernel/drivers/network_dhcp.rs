//! DHCP is native link configuration, not application or Internet authority.
use crate::{http_transport::{dhcp::{Client, Lease}, smoltcp::time::Instant},
    runtime::network::{NetworkRuntime, types::*}};

static mut CLIENT: Option<Client<'static>> = None;
static mut STORAGE: [crate::http_transport::smoltcp::iface::SocketStorage<'static>; 1] =
    [crate::http_transport::smoltcp::iface::SocketStorage::EMPTY];
static mut MAC: [u8; 6] = [0; 6];
static mut LEASE: Option<Lease> = None;
static mut ADDRESS: Option<AddressId> = None;
static mut ROUTES: [Option<RouteId>; 2] = [None; 2];
static mut PRIOR_DNS: [Option<IpAddress>; 2] = [None; 2];
static mut PROFILE_GENERATION: u64 = 0;

// ------------------------=
// FUNC: clear
// DESC: Removes only this client's ephemeral address, routes and unchanged DNS assignments.
// ------------------=
unsafe fn clear(network: &mut NetworkRuntime) {
    if let Some(id) = ADDRESS.take() { let _ = network.interfaces.remove_address(id); }
    for id in &mut *(&raw mut ROUTES) {
        if let Some(id) = id.take() { let _ = network.interfaces.remove_route(id); }
    }
    if let Some(lease) = LEASE.take() {
        for i in 0..2 {
            if network.resolver.server(i) == lease.dns[i].map(IpAddress::V4) {
                let _ = network.resolver.set_server(i, PRIOR_DNS[i]);
            }
        }
    }
}

// ------------------------=
// FUNC: prepare
// DESC: Starts address discovery only on an enabled automatic wired link without explicit IPv4 configuration.
// ------------------=
pub unsafe fn prepare(network: &mut NetworkRuntime, mac: [u8; 6], now: u64) {
    let enabled = network.interfaces.interface(2).is_some_and(|i|
        i.enabled && i.device.link_state == LinkState::Up)
        && network.profiles.active().is_some_and(|p| p.interfaces_enabled && p.dynamic_addressing)
        && matches!(network.setup_snapshot().selected, NetworkSetupMode::Automatic | NetworkSetupMode::Wired)
        && !(0..network.interfaces.address_count()).filter_map(|i| network.interfaces.address_nth(i))
            .any(|a| a.interface_id == 2 && matches!(a.address, IpAddress::V4(_))
                && Some(a.id) != ADDRESS);
    let generation = network.profiles.generation();
    if !enabled || MAC != mac || PROFILE_GENERATION != generation {
        clear(network); CLIENT = None; MAC = mac;
        PROFILE_GENERATION = generation;
    }
    if enabled && CLIENT.is_none() {
        // Transaction IDs are correlation tokens, not credentials. Mix the
        // observed interface identity with the native monotonic boot clock.
        let mut seed = now;
        for byte in mac { seed = seed.rotate_left(7) ^ u64::from(byte); }
        STORAGE = [crate::http_transport::smoltcp::iface::SocketStorage::EMPTY];
        CLIENT = Some(Client::new(mac, seed, Instant::from_millis(now as i64), &mut *(&raw mut STORAGE)));
    }
}

// ------------------------=
// FUNC: ingest
// DESC: Offers observed NIC packets to the bounded DHCP parser without consuming other services' packets.
// ------------------=
pub unsafe fn ingest(frame: &[u8]) {
    use crate::http_transport::smoltcp::wire::{EthernetFrame, EthernetProtocol, Ipv4Packet, IpProtocol, UdpPacket};
    let Ok(ethernet) = EthernetFrame::new_checked(frame) else { return; };
    let accepted = match ethernet.ethertype() {
        EthernetProtocol::Arp => true,
        EthernetProtocol::Ipv4 => Ipv4Packet::new_checked(ethernet.payload()).ok()
            .filter(|ip| ip.next_header() == IpProtocol::Udp)
            .and_then(|ip| UdpPacket::new_checked(ip.payload()).ok()
                .map(|udp| udp.src_port() == 67 && udp.dst_port() == 68)).unwrap_or(false),
        _ => false,
    };
    if accepted {
        if let Some(client) = (&mut *(&raw mut CLIENT)).as_mut() { client.frames.ingest(frame); }
    }
}

// ------------------------=
// FUNC: poll
// DESC: Publishes DHCP lease transitions and submits only native DHCP frames through the existing NIC.
// ------------------=
pub unsafe fn poll(network: &mut NetworkRuntime, nic: &mut crate::drivers::e1000::E1000, now: u64) {
    let change = (&mut *(&raw mut CLIENT)).as_mut().and_then(|c| c.poll(Instant::from_millis(now as i64)));
    if let Some(lease) = change {
        if lease != LEASE {
            clear(network);
            if let Some(lease) = lease {
                let address = network.interfaces.add_address(2, IpAddress::V4(lease.address),
                    lease.prefix, AddressScope::Global, AddressSource::Dynamic, None, None);
                if let Ok(id) = address {
                    ADDRESS = Some(id);
                    let mask = u32::MAX.checked_shl(32 - u32::from(lease.prefix)).unwrap_or(0);
                    let subnet = (u32::from_be_bytes(lease.address) & mask).to_be_bytes();
                    ROUTES[0] = network.interfaces.add_route(IpAddress::V4(subnet), lease.prefix,
                        None, 2, 100, RouteSource::Dynamic, None).ok();
                    if let Some(gateway) = lease.gateway.filter(|_| network.profiles.active()
                        .is_some_and(|p| p.default_route_enabled)) {
                        ROUTES[1] = network.interfaces.add_route(IpAddress::V4([0;4]), 0,
                            Some(IpAddress::V4(gateway)), 2, 100, RouteSource::Dynamic, None).ok();
                    }
                    for i in 0..2 {
                        PRIOR_DNS[i] = network.resolver.server(i);
                        if PRIOR_DNS[i].is_none() && network.profiles.active().is_some_and(|p| p.resolver_enabled) {
                            let _ = network.resolver.set_server(i, lease.dns[i].map(IpAddress::V4));
                        }
                    }
                    LEASE = Some(lease);
                }
            }
        }
    }
    if let Some(client) = (&mut *(&raw mut CLIENT)).as_mut() {
        for _ in 0..4 {
            let Some(frame) = client.frames.pending() else { break; };
            if !nic.transmit(frame) { break; }
            client.frames.transmitted();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: dhcp_respects_offline_static_and_dynamic_policy
    // DESC: Exercises actual client lifetime against profile transitions and preserves explicitly configured addresses.
    // ------------------=
    fn dhcp_respects_offline_static_and_dynamic_policy() { unsafe {
        let mut network = NetworkRuntime::new(); network.initialize().unwrap();
        let mac = [2,0,0,0,0,1];
        network.register_firmware_device(FirmwareNetworkDevice { firmware_handle:0,
            device_id:1,hardware_address:Some(mac),link_state:LinkState::Up,
            maximum_frame_size:1500,can_receive:true,can_transmit:true }).unwrap();
        prepare(&mut network,mac,0);
        assert!((&*(&raw const CLIENT)).is_some());
        network.activate_profile(3).unwrap();
        prepare(&mut network,mac,1);
        assert!((&*(&raw const CLIENT)).is_none());
        network.activate_profile(1).unwrap();
        let id = network.interfaces.add_address(2,IpAddress::V4([192,0,2,5]),24,
            AddressScope::Private,AddressSource::Static,None,None).unwrap();
        prepare(&mut network,mac,2);
        assert!((&*(&raw const CLIENT)).is_none());
        assert!((0..network.interfaces.address_count()).filter_map(|i|network.interfaces.address_nth(i))
            .any(|a|a.id==id));
        network.interfaces.remove_address(id).unwrap();
        prepare(&mut network,mac,3);
        assert!((&*(&raw const CLIENT)).is_some());
        network.activate_profile(4).unwrap();
        prepare(&mut network,mac,4);
        assert!((&*(&raw const CLIENT)).is_none());
    }}
}
