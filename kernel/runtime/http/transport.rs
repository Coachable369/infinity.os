use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage},
    phy::Device,
    socket::tcp::{Socket, SocketBuffer, State},
    time::{Duration, Instant},
    wire::{EthernetAddress, IpCidr, Ipv4Address},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Configuration,
    Busy,
    Disconnected,
    Timeout,
    Cancelled,
    WouldBlock,
}

/// One outbound stream, with caller-owned storage and a hard transaction deadline.
pub struct Transport<'a> {
    interface: Interface,
    sockets: SocketSet<'a>,
    handle: SocketHandle,
    deadline: Option<Instant>,
    failure: Option<Error>,
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;

impl<'a> Transport<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates one Ethernet IPv4 stream using exclusively caller-owned bounded socket buffers.
    // ------------------=
    pub fn new(
        device: &mut impl Device,
        mac: [u8; 6],
        address: [u8; 4],
        prefix: u8,
        gateway: Option<[u8; 4]>,
        random_seed: u64,
        now: Instant,
        storage: &'a mut [SocketStorage<'a>; 1],
        rx: &'a mut [u8],
        tx: &'a mut [u8],
    ) -> Result<Self, Error> {
        if prefix > 32
            || address == [0; 4]
            || rx.is_empty()
            || tx.is_empty()
            || mac == [0; 6]
            || mac[0] & 1 != 0
            || device.capabilities().medium != smoltcp::phy::Medium::Ethernet
        {
            return Err(Error::Configuration);
        }
        let mut config = Config::new(EthernetAddress(mac).into());
        config.random_seed = random_seed;
        let mut interface = Interface::new(config, device, now);
        interface.update_ip_addrs(|ips| {
            let _ = ips.push(IpCidr::new(Ipv4Address::from(address).into(), prefix));
        });
        if let Some(gateway) = gateway {
            interface
                .routes_mut()
                .add_default_ipv4_route(Ipv4Address::from(gateway))
                .map_err(|_| Error::Configuration)?;
        }
        let mut sockets = SocketSet::new(&mut storage[..]);
        let mut socket = Socket::new(SocketBuffer::new(rx), SocketBuffer::new(tx));
        socket.set_timeout(Some(Duration::from_secs(15)));
        socket.set_nagle_enabled(false);
        let handle = sockets.add(socket);
        Ok(Self {
            interface,
            sockets,
            handle,
            deadline: None,
            failure: None,
        })
    }

    // ------------------------=
    // FUNC: connect
    // DESC: Begins an already-authorized outbound connection without waiting for ARP or a TCP handshake.
    // ------------------=
    pub fn connect(
        &mut self,
        destination: [u8; 4],
        port: u16,
        local_port: u16,
        now: Instant,
        deadline: Instant,
    ) -> Result<(), Error> {
        if deadline <= now {
            return Err(Error::Timeout);
        }
        if destination == [0; 4] || destination[0] >= 224 || port == 0 || local_port == 0 {
            return Err(Error::Configuration);
        }
        let socket = self.sockets.get_mut::<Socket>(self.handle);
        if socket.is_open() {
            return Err(Error::Busy);
        }
        socket
            .connect(
                self.interface.context(),
                (Ipv4Address::from(destination), port),
                local_port,
            )
            .map_err(|_| Error::Configuration)?;
        self.deadline = Some(deadline);
        self.failure = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: poll
    // DESC: Processes at most four ingress packets and one bounded socket's egress without blocking desktop input.
    // ------------------=
    pub fn poll(&mut self, device: &mut impl Device, now: Instant) {
        if self.deadline.map(|end| now >= end).unwrap_or(false) {
            self.sockets.get_mut::<Socket>(self.handle).abort();
            self.deadline = None;
            self.failure = Some(Error::Timeout);
        }
        for _ in 0..4 {
            self.interface
                .poll_ingress_single(now, device, &mut self.sockets);
        }
        self.interface.poll_egress(now, device, &mut self.sockets);
    }

    // ------------------------=
    // FUNC: state
    // DESC: Reports actual handshake and close state rather than treating enqueue as a connected stream.
    // ------------------=
    pub fn state(&self) -> State {
        self.sockets.get::<Socket>(self.handle).state()
    }

    // ------------------------=
    // FUNC: send
    // DESC: Enqueues only available capacity and reports partial progress or backpressure to the caller.
    // ------------------=
    pub fn send(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        let socket = self.sockets.get_mut::<Socket>(self.handle);
        if !socket.may_send() {
            return Err(Error::Disconnected);
        }
        if !socket.can_send() {
            return Err(Error::WouldBlock);
        }
        socket.send_slice(bytes).map_err(|_| Error::Disconnected)
    }

    // ------------------------=
    // FUNC: receive
    // DESC: Copies available ordered bytes; distinguishes EOF from a temporarily empty receive buffer.
    // ------------------=
    pub fn receive(&mut self, bytes: &mut [u8]) -> Result<usize, Error> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if bytes.is_empty() {
            return Ok(0);
        }
        let socket = self.sockets.get_mut::<Socket>(self.handle);
        if socket.can_recv() {
            return socket.recv_slice(bytes).map_err(|_| Error::Disconnected);
        }
        if socket.may_recv() {
            Err(Error::WouldBlock)
        } else {
            Ok(0)
        }
    }

    // ------------------------=
    // FUNC: close
    // DESC: Starts graceful FIN delivery after queued output without waiting for the peer.
    // ------------------=
    pub fn close(&mut self) {
        self.sockets.get_mut::<Socket>(self.handle).close();
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Aborts immediately for user cancellation or capability revocation; subsequent IO fails closed.
    // ------------------=
    pub fn cancel(&mut self) {
        self.sockets.get_mut::<Socket>(self.handle).abort();
        self.deadline = None;
        self.failure = Some(Error::Cancelled);
    }
}
