//! Inbound extension of the shared bounded reactor; no new TCP implementation.
use super::*;

#[cfg(test)]
#[path = "listener_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listener(Handle);

pub trait ListenAuthority: Authority {
    // ------------------------=
    // FUNC: allows_listen
    // DESC: Requires an explicit listener grant for the configured local address and port.
    // ------------------=
    fn allows_listen(&self, grant: u64, local: Endpoint) -> bool;
    // ------------------------=
    // FUNC: allows_accept
    // DESC: Requires acceptance authority for the exact established peer before exposing data.
    // ------------------=
    fn allows_accept(&self, grant: u64, local: Endpoint, remote: Endpoint) -> bool;
}

/// A shared service may own listeners alongside existing outbound handles.
/// Poll this view rather than Reactor::poll whenever listeners are present.
pub struct ListenerAuthority<'p, P: ListenAuthority> {
    pub policy: &'p P,
    pub address: [u8; 4],
}
impl<P: ListenAuthority> Authority for ListenerAuthority<'_, P> {
    // ------------------------=
    // FUNC: allows
    // DESC: Rechecks listener grants while a peer is unknown and normal IO grants after acceptance.
    // ------------------=
    fn allows(&self, grant: u64, protocol: Protocol, remote: Endpoint) -> bool {
        if protocol == Protocol::Tcp && remote.address == [0; 4] {
            self.policy.allows_listen(grant, Endpoint { address: self.address, port: remote.port })
        } else { self.policy.allows(grant, protocol, remote) }
    }
}

impl<'a, const N: usize> Reactor<'a, N> {
    // ------------------------=
    // FUNC: listen
    // DESC: Arms one preallocated TCP backlog slot; callers explicitly bound backlog count and SYN lifetime.
    // ------------------=
    pub fn listen(&mut self, authority: &impl ListenAuthority, grant: u64, port: u16,
        now: Instant, deadline: Instant) -> Result<Listener, Error> {
        if port == 0 { return Err(Error::Configuration); }
        if deadline <= now { return Err(Error::Timeout); }
        if !authority.allows_listen(grant, Endpoint { address: self.address, port }) { return Err(Error::Denied); }
        // An outbound socket on this port cannot become an inbound listener.
        if self.slots.iter().flatten().any(|s| s.live && s.local_port == port
            && s.protocol == Protocol::Tcp && (!s.inbound || s.grant != grant)) {
            return Err(Error::Configuration);
        }
        let index = self.slots.iter().position(|s| s.is_some_and(|s|
            !s.live && s.protocol == Protocol::Tcp && s.generation != u64::MAX)).ok_or(Error::Capacity)?;
        let slot = self.slots[index].as_mut().unwrap();
        let socket = self.sockets.get_mut::<tcp::Socket>(slot.socket);
        socket.abort();
        socket.listen((Ipv4Address::from(self.address), port)).map_err(|_| Error::Configuration)?;
        slot.generation += 1;
        slot.live = true;
        slot.inbound = true;
        slot.grant = grant;
        // No valid outbound endpoint has a zero address. The port still binds policy.
        slot.remote = Endpoint { address: [0; 4], port };
        slot.local_port = port;
        slot.deadline = deadline;
        slot.failure = None;
        slot.fin = false;
        slot.write_closed = false;
        Ok(Listener(Handle { index, generation: slot.generation }))
    }

    // ------------------------=
    // FUNC: accept
    // DESC: Converts a completed backlog handshake into an endpoint-confined generation handle after all capability checks.
    // ------------------=
    pub fn accept(&mut self, listener: Listener, authority: &impl ListenAuthority,
        now: Instant, deadline: Instant) -> Result<(Handle, Endpoint), Error> {
        let handle = listener.0;
        let slot = self.slot(handle)?;
        if slot.protocol != Protocol::Tcp || slot.remote.address != [0; 4] { return Err(Error::InvalidHandle); }
        let policy = ListenerAuthority { policy: authority, address: self.address };
        self.check(handle, &policy, now)?;
        if deadline <= now { self.fail(handle.index, Error::Timeout); return Err(Error::Timeout); }
        let socket = self.sockets.get::<tcp::Socket>(slot.socket);
        if !matches!(socket.state(), tcp::State::Established | tcp::State::CloseWait) { return Err(Error::WouldBlock); }
        let peer = socket.remote_endpoint().ok_or(Error::Disconnected)?;
        let smoltcp::wire::IpAddress::Ipv4(address) = peer.addr;
        let remote = Endpoint { address: address.octets(), port: peer.port };
        if remote.address == [0; 4] || remote.address[0] >= 224 || remote.port == 0 {
            self.fail(handle.index, Error::Denied);
            return Err(Error::Denied);
        }
        if !authority.allows_accept(slot.grant, Endpoint { address: self.address, port: slot.local_port }, remote)
            || !authority.allows(slot.grant, Protocol::Tcp, remote) {
            self.fail(handle.index, Error::Denied);
            return Err(Error::Denied);
        }
        let slot = self.slots[handle.index].as_mut().unwrap();
        slot.remote = remote;
        slot.deadline = deadline;
        Ok((handle, remote))
    }

    // ------------------------=
    // FUNC: release_listener
    // DESC: Cancels incomplete handshakes and returns their bounded TCP storage without exposing an IO handle.
    // ------------------=
    pub fn release_listener(&mut self, listener: Listener) -> Result<(), Error> {
        let slot = self.slot(listener.0)?;
        if slot.remote.address != [0; 4] { return Err(Error::InvalidHandle); }
        self.release(listener.0)
    }

    // ------------------------=
    // FUNC: renew_idle_listener
    // DESC: Renews only an idle listening lease; SYN-received and established peers retain their original hard deadline.
    // ------------------=
    pub fn renew_idle_listener(&mut self, listener: Listener, authority: &impl ListenAuthority,
        now: Instant, deadline: Instant) -> Result<bool, Error> {
        let slot = self.slot(listener.0)?;
        if slot.protocol != Protocol::Tcp || slot.remote.address != [0; 4] { return Err(Error::InvalidHandle); }
        let policy = ListenerAuthority { policy: authority, address: self.address };
        self.check(listener.0, &policy, now)?;
        if deadline <= now { return Err(Error::Timeout); }
        if self.sockets.get::<tcp::Socket>(slot.socket).state() != tcp::State::Listen { return Ok(false); }
        self.slots[listener.0.index].as_mut().unwrap().deadline = deadline;
        Ok(true)
    }
}
