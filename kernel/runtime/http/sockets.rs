//! Bounded native socket reactor. No descriptors, ambient authority, allocation,
//! host sockets, or blocking. The service owner supplies storage, a NIC, monotonic
//! time and a capability validator. Polling and all access are serialized there.
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage},
    phy::{Device, Medium},
    socket::{tcp, udp},
    time::Instant,
    wire::{EthernetAddress, IpCidr, IpEndpoint, Ipv4Address},
};
use crate::transport::Readiness;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol { Tcp, Udp }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Endpoint { pub address: [u8; 4], pub port: u16 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle { index: usize, generation: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Configuration, Capacity, InvalidHandle, Denied, Timeout, Cancelled,
    WouldBlock, Disconnected, MessageTooLarge,
}
/// The embedding service resolves the opaque grant against its IOP capability
/// context. Rechecked on every operation and poll, including queued egress.
pub trait Authority {
    // ------------------------=
    // FUNC: allows
    // DESC: Resolves a service-owned grant for the exact protocol and remote endpoint.
    // ------------------=
    fn allows(&self, grant: u64, protocol: Protocol, remote: Endpoint) -> bool;
}
#[derive(Clone, Copy)]
struct Slot {
    socket: SocketHandle,
    protocol: Protocol,
    generation: u64,
    live: bool,
    grant: u64,
    remote: Endpoint,
    local_port: u16,
    deadline: Instant,
    failure: Option<Error>,
    fin: bool,
    write_closed: bool,
}
pub struct Reactor<'a, const N: usize> {
    interface: Interface,
    sockets: SocketSet<'a>,
    slots: [Option<Slot>; N],
    capacity: usize,
    address: [u8; 4],
}
impl<'a, const N: usize> Reactor<'a, N> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates one shared interface with caller-owned bounded socket storage.
    // ------------------=
    pub fn new(device: &mut impl Device, mac: [u8; 6], address: [u8; 4], prefix: u8,
        gateway: Option<[u8; 4]>, seed: u64, now: Instant,
        storage: &'a mut [SocketStorage<'a>]) -> Result<Self, Error> {
        if N == 0 || storage.is_empty() || prefix > 32 || address == [0; 4]
            || address[0] >= 224 || mac == [0; 6] || mac[0] & 1 != 0
            || device.capabilities().medium != Medium::Ethernet {
            return Err(Error::Configuration);
        }
        let mut config = Config::new(EthernetAddress(mac).into());
        config.random_seed = seed;
        let mut interface = Interface::new(config, device, now);
        interface.update_ip_addrs(|ips| { let _ = ips.push(IpCidr::new(Ipv4Address::from(address).into(), prefix)); });
        if let Some(gateway) = gateway {
            interface.routes_mut().add_default_ipv4_route(Ipv4Address::from(gateway))
                .map_err(|_| Error::Configuration)?;
        }
        let capacity = storage.len().min(N);
        Ok(Self { interface, sockets: SocketSet::new(storage), slots: [None; N], capacity, address })
    }
    // ------------------------=
    // FUNC: reserve
    // DESC: Registers preallocated socket storage once, without giving an application a live handle.
    // ------------------=
    fn reserve(&mut self, socket: impl smoltcp::socket::AnySocket<'a>, protocol: Protocol) -> Result<(), Error> {
        let index = self.slots[..self.capacity].iter().position(Option::is_none).ok_or(Error::Capacity)?;
        let socket = self.sockets.add(socket);
        self.slots[index] = Some(Slot { socket, protocol, generation: 0, live: false, grant: 0,
            remote: Endpoint { address: [0; 4], port: 0 }, local_port: 0,
            deadline: Instant::from_millis(0), failure: None, fin: false, write_closed: false });
        Ok(())
    }
    // ------------------------=
    // FUNC: add_tcp
    // DESC: Reserves a reusable TCP slot with bounded receive and transmit buffers.
    // ------------------=
    pub fn add_tcp(&mut self, rx: &'a mut [u8], tx: &'a mut [u8]) -> Result<(), Error> {
        if rx.is_empty() || tx.is_empty() { return Err(Error::Configuration); }
        let mut socket = tcp::Socket::new(tcp::SocketBuffer::new(rx), tcp::SocketBuffer::new(tx));
        socket.set_nagle_enabled(false);
        self.reserve(socket, Protocol::Tcp)
    }
    // ------------------------=
    // FUNC: add_udp
    // DESC: Reserves a reusable datagram slot with caller-owned packet metadata and payload rings.
    // ------------------=
    pub fn add_udp(&mut self, rx_meta: &'a mut [udp::PacketMetadata], rx: &'a mut [u8],
        tx_meta: &'a mut [udp::PacketMetadata], tx: &'a mut [u8]) -> Result<(), Error> {
        if rx_meta.is_empty() || tx_meta.is_empty() || rx.is_empty() || tx.is_empty() { return Err(Error::Configuration); }
        self.reserve(udp::Socket::new(udp::PacketBuffer::new(rx_meta, rx),
            udp::PacketBuffer::new(tx_meta, tx)), Protocol::Udp)
    }
    // ------------------------=
    // FUNC: open
    // DESC: Opens an endpoint-confined nonblocking channel only after capability and resource checks.
    // ------------------=
    pub fn open(&mut self, authority: &impl Authority, grant: u64, protocol: Protocol,
        remote: Endpoint, local_port: u16, now: Instant, deadline: Instant) -> Result<Handle, Error> {
        if remote.address == [0; 4] || remote.address[0] >= 224 || remote.port == 0 || local_port == 0 {
            return Err(Error::Configuration);
        }
        if deadline <= now { return Err(Error::Timeout); }
        if !authority.allows(grant, protocol, remote) { return Err(Error::Denied); }
        // No ambiguous local tuple demultiplexing; callers select an unused ephemeral port.
        if self.slots.iter().flatten().any(|s| s.live && s.protocol == protocol && s.local_port == local_port) {
            return Err(Error::Configuration);
        }
        let index = self.slots.iter().position(|s| s.map(|s| !s.live && s.protocol == protocol
            && s.generation != u64::MAX).unwrap_or(false)).ok_or(Error::Capacity)?;
        let slot = self.slots[index].as_mut().unwrap();
        match protocol {
            Protocol::Tcp => {
                let socket = self.sockets.get_mut::<tcp::Socket>(slot.socket);
                socket.abort();
                socket.connect(self.interface.context(), (Ipv4Address::from(remote.address), remote.port),
                    (Ipv4Address::from(self.address), local_port)).map_err(|_| Error::Configuration)?;
            }
            Protocol::Udp => {
                let socket = self.sockets.get_mut::<udp::Socket>(slot.socket);
                socket.close();
                socket.bind((Ipv4Address::from(self.address), local_port)).map_err(|_| Error::Configuration)?;
            }
        }
        slot.generation += 1;
        slot.live = true;
        slot.grant = grant;
        slot.remote = remote;
        slot.local_port = local_port;
        slot.deadline = deadline;
        slot.failure = None;
        slot.fin = false;
        slot.write_closed = false;
        Ok(Handle { index, generation: slot.generation })
    }
    // ------------------------=
    // FUNC: slot
    // DESC: Rejects stale generations before touching a reused socket or its data.
    // ------------------=
    fn slot(&self, handle: Handle) -> Result<Slot, Error> {
        self.slots.get(handle.index).copied().flatten().filter(|s| s.live && s.generation == handle.generation)
            .ok_or(Error::InvalidHandle)
    }
    // ------------------------=
    // FUNC: fail
    // DESC: Cancels network work before marking a terminal error visible to the selector.
    // ------------------=
    fn fail(&mut self, index: usize, error: Error) {
        let slot = self.slots[index].as_mut().unwrap();
        match slot.protocol {
            Protocol::Tcp => self.sockets.get_mut::<tcp::Socket>(slot.socket).abort(),
            Protocol::Udp => self.sockets.get_mut::<udp::Socket>(slot.socket).close(),
        }
        slot.failure = Some(error);
    }
    // ------------------------=
    // FUNC: check
    // DESC: Revalidates time and authority on each IO call, not just once at connection creation.
    // ------------------=
    fn check(&mut self, handle: Handle, authority: &impl Authority, now: Instant) -> Result<Slot, Error> {
        let slot = self.slot(handle)?;
        if let Some(error) = slot.failure { return Err(error); }
        let failure = if !authority.allows(slot.grant, slot.protocol, slot.remote) { Some(Error::Denied) }
            else if now >= slot.deadline { Some(Error::Timeout) } else { None };
        if let Some(error) = failure { self.fail(handle.index, error); return Err(error); }
        Ok(slot)
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Revalidates all channels before bounded ingress and shared-interface egress.
    // ------------------=
    pub fn poll(&mut self, device: &mut impl Device, authority: &impl Authority, now: Instant) {
        for index in 0..N {
            if let Some(slot) = self.slots[index].filter(|s| s.live) {
                let _ = self.check(Handle { index, generation: slot.generation }, authority, now);
            }
        }
        for _ in 0..4 { self.interface.poll_ingress_single(now, device, &mut self.sockets); }
        self.interface.poll_egress(now, device, &mut self.sockets);
        for slot in self.slots.iter_mut().flatten().filter(|s| s.live && s.protocol == Protocol::Tcp && s.failure.is_none()) {
            let state = self.sockets.get::<tcp::Socket>(slot.socket).state();
            if matches!(state, tcp::State::CloseWait | tcp::State::Closing | tcp::State::LastAck | tcp::State::TimeWait) {
                slot.fin = true;
            }
            if state == tcp::State::Closed && !slot.fin { slot.failure = Some(Error::Disconnected); }
        }
    }
    // ------------------------=
    // FUNC: readiness
    // DESC: Reports terminal completion and backpressure without consuming a byte.
    // ------------------=
    pub fn readiness(&self, handle: Handle) -> Result<Readiness, Error> {
        let slot = self.slot(handle)?;
        if slot.failure.is_some() { return Ok(Readiness { readable: true, writable: true }); }
        Ok(match slot.protocol {
            Protocol::Tcp => {
                let s = self.sockets.get::<tcp::Socket>(slot.socket);
                Readiness { readable: s.can_recv() || slot.fin, writable: s.can_send() || slot.write_closed || slot.fin }
            }
            Protocol::Udp => {
                let s = self.sockets.get::<udp::Socket>(slot.socket);
                Readiness { readable: s.can_recv(), writable: s.can_send() }
            }
        })
    }
    // ------------------------=
    // FUNC: connected
    // DESC: Reports handshake completion separately from write readiness and terminal failure.
    // ------------------=
    pub fn connected(&mut self, handle: Handle, authority: &impl Authority, now: Instant) -> Result<bool, Error> {
        let slot = self.check(handle, authority, now)?;
        Ok(match slot.protocol {
            Protocol::Tcp => matches!(self.sockets.get::<tcp::Socket>(slot.socket).state(),
                tcp::State::Established | tcp::State::CloseWait),
            Protocol::Udp => true,
        })
    }
    // ------------------------=
    // FUNC: peek
    // DESC: Copies TCP bytes without consuming receive state, preserving EOF and terminal errors.
    // ------------------=
    pub fn peek(&mut self, handle: Handle, authority: &impl Authority, now: Instant, bytes: &mut [u8]) -> Result<usize, Error> {
        let slot = self.check(handle, authority, now)?;
        if slot.protocol != Protocol::Tcp { return Err(Error::Configuration); }
        if bytes.is_empty() { return Ok(0); }
        let socket = self.sockets.get_mut::<tcp::Socket>(slot.socket);
        if socket.can_recv() { return socket.peek_slice(bytes).map_err(|_| Error::Disconnected); }
        if slot.fin { Ok(0) } else { Err(Error::WouldBlock) }
    }
    // ------------------------=
    // FUNC: send
    // DESC: Returns partial TCP progress or atomic UDP progress with endpoint confinement.
    // ------------------=
    pub fn send(&mut self, handle: Handle, authority: &impl Authority, now: Instant, bytes: &[u8]) -> Result<usize, Error> {
        let slot = self.check(handle, authority, now)?;
        match slot.protocol {
            Protocol::Tcp => {
                if slot.write_closed { return Err(Error::Disconnected); }
                let s = self.sockets.get_mut::<tcp::Socket>(slot.socket);
                if !s.may_send() { return Err(if s.state() == tcp::State::SynSent { Error::WouldBlock } else { Error::Disconnected }); }
                if bytes.is_empty() { return Ok(0); }
                if !s.can_send() { return Err(Error::WouldBlock); }
                s.send_slice(bytes).map_err(|_| Error::Disconnected)
            }
            Protocol::Udp => {
                let s = self.sockets.get_mut::<udp::Socket>(slot.socket);
                if bytes.len() > s.payload_send_capacity() { return Err(Error::MessageTooLarge); }
                s.send_slice(bytes, IpEndpoint::new(Ipv4Address::from(slot.remote.address).into(), slot.remote.port))
                    .map_err(|_| Error::WouldBlock)?;
                Ok(bytes.len())
            }
        }
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Reads TCP bytes or one authorized-peer datagram; drains wrong-peer packets with bounded work.
    // ------------------=
    pub fn receive(&mut self, handle: Handle, authority: &impl Authority, now: Instant, bytes: &mut [u8]) -> Result<usize, Error> {
        let slot = self.check(handle, authority, now)?;
        match slot.protocol {
            Protocol::Tcp => {
                if bytes.is_empty() { return Ok(0); }
                let s = self.sockets.get_mut::<tcp::Socket>(slot.socket);
                if s.can_recv() { return s.recv_slice(bytes).map_err(|_| Error::Disconnected); }
                if slot.fin { Ok(0) } else { Err(Error::WouldBlock) }
            }
            Protocol::Udp => {
                let s = self.sockets.get_mut::<udp::Socket>(slot.socket);
                for _ in 0..4 {
                    let (packet, metadata) = s.recv().map_err(|_| Error::WouldBlock)?;
                    if metadata.endpoint == IpEndpoint::new(Ipv4Address::from(slot.remote.address).into(), slot.remote.port) {
                        let length = bytes.len().min(packet.len());
                        bytes[..length].copy_from_slice(&packet[..length]);
                        return Ok(length);
                    }
                }
                Err(Error::WouldBlock)
            }
        }
    }
    // ------------------------=
    // FUNC: shutdown_write
    // DESC: Sends TCP FIN after queued data without discarding the receive half or waiting.
    // ------------------=
    pub fn shutdown_write(&mut self, handle: Handle, authority: &impl Authority, now: Instant) -> Result<(), Error> {
        let slot = self.check(handle, authority, now)?;
        if slot.protocol != Protocol::Tcp { return Err(Error::Configuration); }
        self.sockets.get_mut::<tcp::Socket>(slot.socket).close();
        self.slots[handle.index].as_mut().unwrap().write_closed = true;
        Ok(())
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Interrupts a channel and exposes cancellation to pending operations until release.
    // ------------------=
    pub fn cancel(&mut self, handle: Handle) -> Result<(), Error> {
        self.slot(handle)?;
        self.fail(handle.index, Error::Cancelled);
        Ok(())
    }
    // ------------------------=
    // FUNC: release
    // DESC: Revokes the handle immediately and returns its fixed buffers to the channel pool.
    // ------------------=
    pub fn release(&mut self, handle: Handle) -> Result<(), Error> {
        self.cancel(handle)?;
        self.slots[handle.index].as_mut().unwrap().live = false;
        Ok(())
    }
}

#[cfg(test)]
#[path = "sockets_tests.rs"]
mod tests;
