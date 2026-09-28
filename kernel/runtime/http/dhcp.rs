//! Bounded native DHCP client. The caller owns policy, NIC access and lease publication.
use crate::device::EthernetQueue;
use smoltcp::{iface::{Config, Interface, SocketSet, SocketStorage, SocketHandle}, socket::dhcpv4,
    time::Instant, wire::EthernetAddress};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lease {
    pub address: [u8; 4],
    pub prefix: u8,
    pub gateway: Option<[u8; 4]>,
    pub dns: [Option<[u8; 4]>; 2],
}

#[cfg(test)]
mod tests {
    use super::*;
    use smoltcp::wire::*;
    use std::{vec, vec::Vec};

    // ------------------------=
    // FUNC: reply
    // DESC: Encodes a checksummed DHCP server reply from the client's real transaction and hardware identity.
    // ------------------=
    fn reply(request: &[u8], kind: u8, wrong_id: bool) -> Vec<u8> {
        let eth = EthernetFrame::new_checked(request).unwrap();
        let ip = Ipv4Packet::new_checked(eth.payload()).unwrap();
        let udp = UdpPacket::new_checked(ip.payload()).unwrap();
        let mut payload = udp.payload()[..240].to_vec();
        payload[0] = 2;
        if wrong_id { payload[4] ^= 1; }
        payload[16..20].copy_from_slice(&[10,0,2,15]);
        payload[20..24].copy_from_slice(&[10,0,2,2]);
        payload.extend_from_slice(&[53,1,kind,54,4,10,0,2,2,1,4,255,255,255,0,
            3,4,10,0,2,2,6,4,10,0,2,3,51,4,0,0,0,120,255]);
        let mut bytes = vec![0;14+20+8+payload.len()];
        let mut frame = EthernetFrame::new_unchecked(&mut bytes[..]);
        frame.set_src_addr(EthernetAddress([2,0,0,0,0,2]));
        frame.set_dst_addr(EthernetAddress::BROADCAST);
        frame.set_ethertype(EthernetProtocol::Ipv4);
        let repr = Ipv4Repr { src_addr: Ipv4Address::new(10,0,2,2),
            dst_addr: Ipv4Address::BROADCAST, next_header: IpProtocol::Udp,
            payload_len:8+payload.len(), hop_limit:64 };
        let checksums = smoltcp::phy::ChecksumCapabilities::default();
        let mut ip = Ipv4Packet::new_unchecked(frame.payload_mut());
        repr.emit(&mut ip, &checksums);
        UdpRepr { src_port:67,dst_port:68 }.emit(&mut UdpPacket::new_unchecked(ip.payload_mut()),
            &repr.src_addr.into(), &repr.dst_addr.into(), payload.len(),
            |out|out.copy_from_slice(&payload), &checksums);
        bytes
    }

    #[test]
    // ------------------------=
    // FUNC: discovers_validates_configures_and_expires
    // DESC: Exercises real Ethernet discovery, rejects a mismatched offer, publishes only an ACK and expires its lease.
    // ------------------=
    fn discovers_validates_configures_and_expires() {
        let mut storage = [SocketStorage::EMPTY];
        let mut client = Client::new([2,0,0,0,0,1],7,Instant::from_millis(0), &mut storage);
        assert_eq!(client.poll(Instant::from_millis(0)),Some(None));
        let discover = client.frames.pending().unwrap().to_vec();
        client.frames.transmitted();
        assert!(client.frames.ingest(&reply(&discover,2,true)));
        assert_eq!(client.poll(Instant::from_millis(1)),None);
        assert!(client.frames.pending().is_none());
        assert!(client.frames.ingest(&reply(&discover,2,false)));
        assert_eq!(client.poll(Instant::from_millis(2)),None);
        let request = client.frames.pending().unwrap().to_vec();
        client.frames.transmitted();
        assert!(client.frames.ingest(&reply(&request,5,false)));
        assert_eq!(client.poll(Instant::from_millis(3)),Some(Some(Lease {
            address:[10,0,2,15],prefix:24,gateway:Some([10,0,2,2]),dns:[Some([10,0,2,3]),None] })));
        assert_eq!(client.poll(Instant::from_millis(4)),None);
        assert_eq!(client.poll(Instant::from_millis(120004)),Some(None));
    }
}
pub struct Client<'a> {
    interface: Interface,
    sockets: SocketSet<'a>,
    handle: SocketHandle,
    pub frames: EthernetQueue,
}
impl<'a> Client<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an allocation-free DHCP-only interface with caller-supplied transaction entropy.
    // ------------------=
    pub fn new(mac: [u8; 6], seed: u64, now: Instant, storage: &'a mut [SocketStorage<'a>]) -> Self {
        let mut frames = EthernetQueue::new();
        let mut config = Config::new(EthernetAddress(mac).into());
        config.random_seed = seed;
        let mut sockets = SocketSet::new(storage);
        let handle = sockets.add(dhcpv4::Socket::new());
        Self { interface: Interface::new(config, &mut frames, now), sockets, handle, frames }
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Advances discovery, renewal and expiry without blocking; reports only changed lease state.
    // ------------------=
    pub fn poll(&mut self, now: Instant) -> Option<Option<Lease>> {
        self.interface.poll(now, &mut self.frames, &mut self.sockets);
        let changed = self.sockets.get_mut::<dhcpv4::Socket>(self.handle).poll().map(|event| match event {
            dhcpv4::Event::Deconfigured => None,
            dhcpv4::Event::Configured(config) => Some(Lease {
                address: config.address.address().octets(), prefix: config.address.prefix_len(),
                gateway: config.router.map(|ip| ip.octets()),
                dns: [config.dns_servers.first().map(|ip| ip.octets()),
                    config.dns_servers.get(1).map(|ip| ip.octets())],
            }),
        });
        if let Some(lease) = changed {
            self.interface.update_ip_addrs(|ips| {
                ips.clear();
                if let Some(lease) = lease {
                    let _ = ips.push(smoltcp::wire::IpCidr::new(smoltcp::wire::Ipv4Address::from(lease.address).into(), lease.prefix));
                }
            });
        }
        changed
    }
}
