//! Native IO source ownership without POSIX descriptors.
use std::{fmt, io, net, ops::{Deref, DerefMut}, sync::Mutex};
use crate::{event::Source, Registry, Token, Interest, sys::{NativeSource, Readiness, socket_readiness}};
pub(crate) trait Socket {
    // ------------------------=
    // FUNC: source
    // DESC: Creates a readiness binding only for implemented native socket kinds.
    // ------------------=
    fn source(&self) -> io::Result<NativeSource>;
    // ------------------------=
    // FUNC: ready
    // DESC: Reads immediate native readiness after IO drains or returns backpressure.
    // ------------------=
    fn ready(&self) -> Readiness;
}
impl Socket for net::TcpStream {
    // ------------------------=
    // FUNC: source
    // DESC: Retains the stream while registered so handle reuse cannot target a different socket.
    // ------------------=
    fn source(&self) -> io::Result<NativeSource> { self.try_clone().map(NativeSource::tcp) }
    // ------------------------=
    // FUNC: ready
    // DESC: Reads TCP state through the explicit native std extension.
    // ------------------=
    fn ready(&self) -> Readiness { socket_readiness(self) }
}
macro_rules! unavailable {
    ($ty:ty) => { impl Socket for $ty {
        // ------------------------=
        // FUNC: source
        // DESC: Denies readiness registration for an unsupported std socket backend.
        // ------------------=
        fn source(&self) -> io::Result<NativeSource> { Err(io::Error::from(io::ErrorKind::Unsupported)) }
        // ------------------------=
        // FUNC: ready
        // DESC: Unsupported socket kinds never fabricate readiness.
        // ------------------=
        fn ready(&self) -> Readiness { Readiness { readable: false, writable: false } }
    } }
}
unavailable!(net::TcpListener);
unavailable!(net::UdpSocket);
pub struct IoSource<T> { inner: T, source: Mutex<Option<NativeSource>> }
impl<T> IoSource<T> {
    // ------------------------=
    // FUNC: new
    // DESC: Wraps an owned native stream without registering or granting new authority.
    // ------------------=
    pub fn new(inner: T) -> Self { Self { inner, source: Mutex::new(None) } }
    // ------------------------=
    // FUNC: into_inner
    // DESC: Drops the readiness registration before returning the underlying stream owner.
    // ------------------=
    pub fn into_inner(self) -> T { self.inner }
}
impl<T: Socket> IoSource<T> {
    // ------------------------=
    // FUNC: do_io
    // DESC: Executes nonblocking IO outside selector locks, then publishes the resulting readiness state.
    // ------------------=
    pub fn do_io<R>(&self, operation: impl FnOnce(&T) -> io::Result<R>) -> io::Result<R> {
        let result = operation(&self.inner);
        if let Some(source) = &mut *self.source.lock().unwrap() { source.observe(self.inner.ready())?; }
        result
    }
}
impl<T> Deref for IoSource<T> {
    type Target = T;
    // ------------------------=
    // FUNC: deref
    // DESC: Borrows the wrapped socket for metadata operations.
    // ------------------=
    fn deref(&self) -> &T { &self.inner }
}
impl<T> DerefMut for IoSource<T> {
    // ------------------------=
    // FUNC: deref_mut
    // DESC: Borrows the wrapped socket exclusively.
    // ------------------=
    fn deref_mut(&mut self) -> &mut T { &mut self.inner }
}
impl<T: fmt::Debug> fmt::Debug for IoSource<T> {
    // ------------------------=
    // FUNC: fmt
    // DESC: Formats the socket without selector locking.
    // ------------------=
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.inner.fmt(f) }
}
impl<T: Socket> Source for IoSource<T> {
    // ------------------------=
    // FUNC: register
    // DESC: Atomically retains and registers a real native readiness source.
    // ------------------=
    fn register(&mut self, registry: &Registry, token: Token, interest: Interest) -> io::Result<()> {
        let slot = self.source.get_mut().unwrap();
        if slot.is_some() { return Err(io::Error::from(io::ErrorKind::AlreadyExists)); }
        let mut source = self.inner.source()?; source.register(registry, token, interest)?; *slot = Some(source); Ok(())
    }
    // ------------------------=
    // FUNC: reregister
    // DESC: Refreshes state before replacing interest and token on the owning selector.
    // ------------------=
    fn reregister(&mut self, registry: &Registry, token: Token, interest: Interest) -> io::Result<()> {
        let source = self.source.get_mut().unwrap().as_mut().ok_or(io::ErrorKind::NotFound)?;
        source.observe(self.inner.ready())?; source.reregister(registry, token, interest)
    }
    // ------------------------=
    // FUNC: deregister
    // DESC: Removes pending events and releases the retained stream after validating registry ownership.
    // ------------------=
    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        let slot = self.source.get_mut().unwrap();
        slot.as_mut().ok_or(io::ErrorKind::NotFound)?.deregister(registry)?; *slot = None; Ok(())
    }
}
