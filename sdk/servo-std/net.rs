//! Native TCP and bounded IPv4 resolver facades. Listener/datagram APIs remain
//! explicitly unsupported. Network authority belongs to the installed providers.
use crate::{fmt, io::{self, BorrowedCursor, IoSlice, IoSliceMut}, net::{SocketAddr, Shutdown, ToSocketAddrs},
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
#[path = "infinity_unsupported.rs"]
mod unsupported_net;
pub use unsupported_net::{TcpListener, UdpSocket};
#[derive(Debug)]
pub struct LookupHost(Option<SocketAddr>);
impl Iterator for LookupHost {
    type Item = SocketAddr;
    // ------------------------=
    // FUNC: next
    // DESC: Consumes the resolver's single bounded IPv4 endpoint.
    // ------------------=
    fn next(&mut self) -> Option<SocketAddr> { self.0.take() }
}
struct DnsQuery(u64);
impl Drop for DnsQuery {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases native query state on every lookup exit path.
    // ------------------=
    fn drop(&mut self) { unsafe { infinity_dns_cancel(self.0); } }
}
// ------------------------=
// FUNC: lookup_host
// DESC: Resolves through a granted native service with a five-second bound and cooperative waits.
// ------------------=
pub fn lookup_host(host: &str, port: u16) -> io::Result<LookupHost> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let query = loop {
        let mut id = 0;
        let status = unsafe { infinity_dns_begin(host.as_ptr(), host.len(), &mut id) };
        if status == 0 { break DnsQuery(id); }
        if status != 11 { return Err(io::Error::from_raw_os_error(status)); }
        if Instant::now() >= deadline { return Err(io::ErrorKind::TimedOut.into()); }
        crate::thread::sleep(Duration::from_millis(1));
    };
    let mut address = [0; 4];
    loop {
        let status = unsafe { infinity_dns_poll(query.0, address.as_mut_ptr()) };
        if status == 0 { return Ok(LookupHost(Some(SocketAddr::from((address, port))))); }
        if status != 11 { return Err(io::Error::from_raw_os_error(status)); }
        if Instant::now() >= deadline { return Err(io::ErrorKind::TimedOut.into()); }
        crate::thread::sleep(Duration::from_millis(1));
    }
}
#[repr(C)]
#[derive(Default)]
struct Status { flags: u32, local: [u8;4], remote: [u8;4], local_port: u16, remote_port: u16 }
unsafe extern "C" {
    fn infinity_dns_begin(bytes: *const u8, length: usize, output: *mut u64) -> i32;
    fn infinity_dns_poll(id: u64, output: *mut u8) -> i32;
    fn infinity_dns_cancel(id: u64);
    fn infinity_tcp_open(address: *const u8, port: u16, handle: *mut u64) -> i32;
    fn infinity_tcp_status(handle: u64, out: *mut Status) -> i32;
    fn infinity_tcp_read(handle: u64, bytes: *mut u8, length: usize, peek: bool, count: *mut usize) -> i32;
    fn infinity_tcp_write(handle: u64, bytes: *const u8, length: usize, count: *mut usize) -> i32;
    fn infinity_tcp_shutdown(handle: u64, mode: u32) -> i32;
    fn infinity_tcp_close(handle: u64);
}
struct Inner { handle: u64, nonblocking: AtomicBool, timeouts: Mutex<(Option<Duration>, Option<Duration>)> }
impl Drop for Inner {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases the service handle only after every duplicate stream has been dropped.
    // ------------------=
    fn drop(&mut self) { unsafe { infinity_tcp_close(self.handle); } }
}
pub struct TcpStream(Arc<Inner>);
// ------------------------=
// FUNC: result
// DESC: Converts the native adapter protocol without relying on ambient errno state.
// ------------------=
fn result(code: i32) -> io::Result<()> { if code == 0 { Ok(()) } else { Err(io::Error::from_raw_os_error(code)) } }
impl TcpStream {
    // ------------------------=
    // FUNC: nonblocking_connect
    // DESC: Creates the native stream without waiting for handshake, for readiness-driven consumers.
    // ------------------=
    pub fn nonblocking_connect(addr: &SocketAddr) -> io::Result<Self> {
        let stream = Self::start(addr)?; stream.set_nonblocking(true)?; Ok(stream)
    }
    // ------------------------=
    // FUNC: readiness
    // DESC: Returns readiness bits from the service without allocating or blocking.
    // ------------------=
    pub fn readiness(&self) -> io::Result<(bool, bool)> {
        let state = self.state()?; Ok((state.flags & 2 != 0, state.flags & 4 != 0))
    }
    // ------------------------=
    // FUNC: start
    // DESC: Begins a nonblocking native connection; IPv6 fails explicitly until supported.
    // ------------------=
    fn start(addr: &SocketAddr) -> io::Result<Self> {
        let SocketAddr::V4(addr) = addr else { return crate::sys::unsupported(); };
        let mut handle = 0;
        result(unsafe { infinity_tcp_open(addr.ip().octets().as_ptr(), addr.port(), &mut handle) })?;
        Ok(Self(Arc::new(Inner { handle, nonblocking: AtomicBool::new(false), timeouts: Mutex::new((None, None)) })))
    }
    // ------------------------=
    // FUNC: state
    // DESC: Reads actual native TCP state including permission revocation and transaction expiry.
    // ------------------=
    fn state(&self) -> io::Result<Status> {
        let mut out = Status::default(); result(unsafe { infinity_tcp_status(self.0.handle, &mut out) })?; Ok(out)
    }
    // ------------------------=
    // FUNC: wait_connected
    // DESC: Cooperatively waits for a real handshake without holding service or std locks.
    // ------------------=
    fn wait_connected(&self, timeout: Option<Duration>) -> io::Result<()> {
        let end = timeout.and_then(|d| Instant::now().checked_add(d));
        loop {
            if self.state()?.flags & 1 != 0 { return Ok(()); }
            if end.is_some_and(|end| Instant::now() >= end) { return Err(io::Error::from(io::ErrorKind::TimedOut)); }
            crate::thread::sleep(Duration::from_millis(1));
        }
    }
    // ------------------------=
    // FUNC: connect
    // DESC: Resolves through std and tries endpoints with native service-enforced deadlines.
    // ------------------=
    pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        super::each_addr(addr, |addr| { let stream = Self::start(addr)?; stream.wait_connected(None)?; Ok(stream) })
    }
    // ------------------------=
    // FUNC: connect_timeout
    // DESC: Enforces a caller-supplied connect deadline in addition to the service grant deadline.
    // ------------------=
    pub fn connect_timeout(addr: &SocketAddr, timeout: Duration) -> io::Result<Self> {
        if timeout.is_zero() { return Err(io::Error::from(io::ErrorKind::InvalidInput)); }
        let stream = Self::start(addr)?; stream.wait_connected(Some(timeout))?; Ok(stream)
    }
    // ------------------------=
    // FUNC: perform
    // DESC: Retries only WouldBlock outside callback borrows, yielding to native tasks and device pumping.
    // ------------------=
    fn perform<T>(&self, write: bool, mut operation: impl FnMut() -> io::Result<T>) -> io::Result<T> {
        let timeouts = *self.0.timeouts.lock().unwrap();
        let timeout = if write { timeouts.1 } else { timeouts.0 };
        let end = timeout.and_then(|d| Instant::now().checked_add(d));
        loop {
            match operation() {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock && !self.0.nonblocking.load(Ordering::Relaxed) => {}
                value => return value,
            }
            if end.is_some_and(|end| Instant::now() >= end) { return Err(io::Error::from(io::ErrorKind::TimedOut)); }
            crate::thread::sleep(Duration::from_millis(1));
        }
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Reads or peeks caller storage through the native stream service.
    // ------------------=
    fn receive(&self, bytes: &mut [u8], peek: bool) -> io::Result<usize> {
        self.perform(false, || { let mut count = 0;
            result(unsafe { infinity_tcp_read(self.0.handle, bytes.as_mut_ptr(), bytes.len(), peek, &mut count) })?; Ok(count) })
    }
    // ------------------------=
    // FUNC: read
    // DESC: Consumes available stream bytes or cooperatively waits according to the configured mode.
    // ------------------=
    pub fn read(&self, bytes: &mut [u8]) -> io::Result<usize> { self.receive(bytes, false) }
    // ------------------------=
    // FUNC: peek
    // DESC: Reads without consuming the native receive queue.
    // ------------------=
    pub fn peek(&self, bytes: &mut [u8]) -> io::Result<usize> { self.receive(bytes, true) }
    // ------------------------=
    // FUNC: read_buf
    // DESC: Initializes only the bounded cursor space before a native receive and advances by actual progress.
    // ------------------=
    pub fn read_buf(&self, mut buf: BorrowedCursor<'_, u8>) -> io::Result<()> {
        let count = self.read(buf.ensure_init())?; unsafe { buf.advance(count); } Ok(())
    }
    // ------------------------=
    // FUNC: read_vectored
    // DESC: Preserves partial-read semantics using the first nonempty scatter buffer.
    // ------------------=
    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.read(bufs.iter_mut().find(|b| !b.is_empty()).map_or(&mut [], |b| &mut **b))
    }
    // ------------------------=
    // FUNC: is_read_vectored
    // DESC: Reports the scalar implementation honestly to callers.
    // ------------------=
    pub fn is_read_vectored(&self) -> bool { false }
    // ------------------------=
    // FUNC: write
    // DESC: Enqueues native transmit capacity with partial-write semantics and cooperative backpressure.
    // ------------------=
    pub fn write(&self, bytes: &[u8]) -> io::Result<usize> {
        self.perform(true, || { let mut count = 0;
            result(unsafe { infinity_tcp_write(self.0.handle, bytes.as_ptr(), bytes.len(), &mut count) })?; Ok(count) })
    }
    // ------------------------=
    // FUNC: write_vectored
    // DESC: Writes the first nonempty gather slice without allocating a concatenated copy.
    // ------------------=
    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.write(bufs.iter().find(|b| !b.is_empty()).map_or(&[], |b| &**b))
    }
    // ------------------------=
    // FUNC: is_write_vectored
    // DESC: Reports the scalar implementation honestly to callers.
    // ------------------=
    pub fn is_write_vectored(&self) -> bool { false }
    // ------------------------=
    // FUNC: socket_addr
    // DESC: Returns the service-assigned local endpoint.
    // ------------------=
    pub fn socket_addr(&self) -> io::Result<SocketAddr> { let s = self.state()?; Ok((s.local, s.local_port).into()) }
    // ------------------------=
    // FUNC: peer_addr
    // DESC: Returns the actual connected endpoint and rejects an incomplete handshake.
    // ------------------=
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        let s = self.state()?; if s.flags & 1 == 0 { return Err(io::Error::from(io::ErrorKind::NotConnected)); }
        Ok((s.remote, s.remote_port).into())
    }
    // ------------------------=
    // FUNC: duplicate
    // DESC: Shares one authorized native channel and its socket options without duplicating network authority.
    // ------------------=
    pub fn duplicate(&self) -> io::Result<Self> { Ok(Self(self.0.clone())) }
    // ------------------------=
    // FUNC: shutdown
    // DESC: Routes shutdown to the native service instead of pretending unsupported modes succeeded.
    // ------------------=
    pub fn shutdown(&self, mode: Shutdown) -> io::Result<()> {
        result(unsafe { infinity_tcp_shutdown(self.0.handle, match mode { Shutdown::Read => 0, Shutdown::Write => 1, Shutdown::Both => 2 }) })
    }
    // ------------------------=
    // FUNC: set_nonblocking
    // DESC: Controls cooperative waiting across every clone of this channel.
    // ------------------=
    pub fn set_nonblocking(&self, value: bool) -> io::Result<()> { self.0.nonblocking.store(value, Ordering::Relaxed); Ok(()) }
    // ------------------------=
    // FUNC: set_read_timeout
    // DESC: Sets an optional nonzero per-read deadline without extending service authority.
    // ------------------=
    pub fn set_read_timeout(&self, value: Option<Duration>) -> io::Result<()> {
        if value.is_some_and(|d| d.is_zero()) { return Err(io::Error::from(io::ErrorKind::InvalidInput)); }
        self.0.timeouts.lock().unwrap().0 = value; Ok(())
    }
    // ------------------------=
    // FUNC: set_write_timeout
    // DESC: Sets an optional nonzero per-write deadline without extending service authority.
    // ------------------=
    pub fn set_write_timeout(&self, value: Option<Duration>) -> io::Result<()> {
        if value.is_some_and(|d| d.is_zero()) { return Err(io::Error::from(io::ErrorKind::InvalidInput)); }
        self.0.timeouts.lock().unwrap().1 = value; Ok(())
    }
    // ------------------------=
    // FUNC: read_timeout
    // DESC: Returns the shared configured read timeout.
    // ------------------=
    pub fn read_timeout(&self) -> io::Result<Option<Duration>> { Ok(self.0.timeouts.lock().unwrap().0) }
    // ------------------------=
    // FUNC: write_timeout
    // DESC: Returns the shared configured write timeout.
    // ------------------=
    pub fn write_timeout(&self) -> io::Result<Option<Duration>> { Ok(self.0.timeouts.lock().unwrap().1) }
    // ------------------------=
    // FUNC: take_error
    // DESC: Exposes terminal channel errors; they remain terminal until channel release.
    // ------------------=
    pub fn take_error(&self) -> io::Result<Option<io::Error>> { Ok(self.state().err()) }
}
macro_rules! unsupported_option {
    ($set:ident, $get:ident, $type:ty) => {
        // ------------------------=
        // FUNC: setter
        // DESC: Rejects unimplemented socket options rather than pretending to apply them.
        // ------------------=
        pub fn $set(&self, _: $type) -> io::Result<()> { crate::sys::unsupported() }
        // ------------------------=
        // FUNC: getter
        // DESC: Rejects unimplemented socket options rather than inventing a value.
        // ------------------=
        pub fn $get(&self) -> io::Result<$type> { crate::sys::unsupported() }
    }
}
impl TcpStream {
    unsupported_option!(set_linger, linger, Option<Duration>);
    unsupported_option!(set_keepalive, keepalive, bool);
    unsupported_option!(set_nodelay, nodelay, bool);
    unsupported_option!(set_ttl, ttl, u32);
}
impl fmt::Debug for TcpStream {
    // ------------------------=
    // FUNC: fmt
    // DESC: Describes a stream without exposing service handles or invoking network IO.
    // ------------------=
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("InfinityTcpStream") }
}
