//! Native readiness/control selector. Native services publish readiness through
//! NativeSource; upstream Mio TCP/UDP wrappers still require a separate port.
use crate::{Token, Interest, Registry};
use std::{fmt, io, sync::{Arc, Mutex, Condvar}, time::{Duration, Instant}};
#[path = "infinity_selector.rs"]
mod core_selector;
/// Readiness reported by an authorized native service, not a socket capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Readiness {
    /// A read can complete, fail, or return EOF without blocking.
    pub readable: bool,
    /// A write can complete or fail without blocking.
    pub writable: bool,
}
const NONE: Readiness = Readiness { readable: false, writable: false };

#[derive(Clone, Copy, Debug)]
pub struct Event { token: Token, ready: Readiness }
pub type Events = Vec<Event>;
struct State { waker: bool, core: core_selector::Selector<32>,
    sockets: [Option<(core_selector::Registration, Arc<std::net::TcpStream>)>; 32] }
struct Shared { state: Mutex<State>, changed: Condvar }
pub struct Selector { shared: Arc<Shared> }
impl fmt::Debug for Selector {
    // ------------------------=
    // FUNC: fmt
    // DESC: Describes the native selector without taking locks during formatting.
    // ------------------=
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("InfinitySelector") }
}
impl Selector {
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded control-event storage backed by the native std wait provider.
    // ------------------=
    pub fn new() -> io::Result<Self> {
        Ok(Self { shared: Arc::new(Shared { state: Mutex::new(State { waker: false, core: core_selector::Selector::new(), sockets: core::array::from_fn(|_| None) }), changed: Condvar::new() }) })
    }
    // ------------------------=
    // FUNC: try_clone
    // DESC: Shares exactly the same event domain with a cloned registry.
    // ------------------=
    pub fn try_clone(&self) -> io::Result<Self> { Ok(Self { shared: self.shared.clone() }) }
    // ------------------------=
    // FUNC: select
    // DESC: Waits for a coalesced control event or deadline and preserves events when output capacity is zero.
    // ------------------=
    pub fn select(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<()> {
        events.clear();
        if events.capacity() == 0 { return Ok(()); }
        let deadline = timeout.and_then(|duration| Instant::now().checked_add(duration));
        let mut state = self.shared.state.lock().map_err(|_| io::Error::other("native selector poisoned"))?;
        loop {
            let State { core, sockets, .. } = &mut *state;
            let mut has_sockets = false;
            for (id, socket) in sockets.iter().flatten() {
                has_sockets = true;
                core.observe(*id, socket_readiness(socket)).map_err(error)?;
            }
            let mut batch = [core_selector::Event { token: 0, ready: NONE }; 32];
            let count = state.core.drain(&mut batch[..events.capacity().min(32)]);
            if count != 0 {
                for e in &batch[..count] { events.push(Event { token: Token(e.token), ready: e.ready }); }
                return Ok(());
            }
            if let Some(deadline) = deadline {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() { return Ok(()); }
                let wait = if has_sockets { remaining.min(Duration::from_millis(1)) } else { remaining };
                let (next, _) = self.shared.changed.wait_timeout(state, wait).map_err(|_| io::Error::other("native selector poisoned"))?;
                state = next;
            } else if has_sockets {
                let (next, _) = self.shared.changed.wait_timeout(state, Duration::from_millis(1)).map_err(|_| io::Error::other("native selector poisoned"))?;
                state = next;
            } else {
                state = self.shared.changed.wait(state).map_err(|_| io::Error::other("native selector poisoned"))?;
            }
        }
    }
}
pub struct Waker { shared: Arc<Shared>, registration: core_selector::Registration }
impl fmt::Debug for Waker {
    // ------------------------=
    // FUNC: fmt
    // DESC: Formats a control source without taking its selector lock.
    // ------------------=
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("InfinityWaker") }
}
impl Waker {
    // ------------------------=
    // FUNC: new
    // DESC: Enforces Mio's one-control-waker-per-selector contract in release builds too.
    // ------------------=
    pub fn new(selector: &Selector, token: Token) -> io::Result<Self> {
        let mut state = selector.shared.state.lock().map_err(|_| io::Error::other("native selector poisoned"))?;
        if state.waker { return Err(io::Error::new(io::ErrorKind::AlreadyExists, "native control waker exists")); }
        let registration = state.core.register(token.0, Readiness { readable: true, writable: false }, NONE).map_err(error)?;
        state.waker = true;
        Ok(Self { shared: selector.shared.clone(), registration })
    }
    // ------------------------=
    // FUNC: wake
    // DESC: Coalesces notifications under the same lock used by compare-and-wait.
    // ------------------=
    pub fn wake(&self) -> io::Result<()> {
        let mut state = self.shared.state.lock().map_err(|_| io::Error::other("native selector poisoned"))?;
        state.core.wake(self.registration).map_err(error)?;
        self.shared.changed.notify_one(); Ok(())
    }
}
impl Drop for Waker {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases the control source and discards its pending event before token reuse.
    // ------------------=
    fn drop(&mut self) { if let Ok(mut state) = self.shared.state.lock() { state.waker = false; let _ = state.core.deregister(self.registration); } }
}
// ------------------------=
// FUNC: error
// DESC: Converts bounded registration failures into explicit IO errors without forging successful registration.
// ------------------=
fn error(value: core_selector::Error) -> io::Error {
    io::Error::new(match value { core_selector::Error::Full => io::ErrorKind::OutOfMemory, _ => io::ErrorKind::InvalidInput }, "native readiness registration rejected")
}
/// Readiness bridge for a native service. Ownership alone grants no network access.
pub struct NativeSource { binding: Option<(Arc<Shared>, core_selector::Registration)>, ready: Readiness,
    socket: Option<Arc<std::net::TcpStream>> }
impl fmt::Debug for NativeSource {
    // ------------------------=
    // FUNC: fmt
    // DESC: Exposes registration state without leaking native resource identity.
    // ------------------=
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_struct("NativeSource").field("registered", &self.binding.is_some()).finish() }
}
impl NativeSource {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a service readiness source without allocating a socket or authority.
    // ------------------=
    /// Starts unregistered with the supplied service state.
    pub fn new(ready: Readiness) -> Self { Self { binding: None, ready, socket: None } }
    // ------------------------=
    // FUNC: tcp
    // DESC: Associates readiness with an owned stream clone, not a forgeable raw descriptor.
    // ------------------=
    pub(crate) fn tcp(socket: std::net::TcpStream) -> Self {
        Self { binding: None, ready: socket_readiness(&socket), socket: Some(Arc::new(socket)) }
    }
    // ------------------------=
    // FUNC: observe
    // DESC: Publishes transitions including not-ready after draining IO so future edges are not lost.
    // ------------------=
    /// The native service must report not-ready after draining an operation.
    pub fn observe(&mut self, ready: Readiness) -> io::Result<()> {
        if ready == self.ready && self.socket.is_none() { return Ok(()); }
        self.ready = ready;
        if let Some((shared, id)) = &self.binding {
            let mut state = shared.state.lock().map_err(|_| io::Error::other("native selector poisoned"))?;
            state.core.observe(*id, ready).map_err(error)?;
            shared.changed.notify_one();
        }
        Ok(())
    }
}
impl crate::event::Source for NativeSource {
    // ------------------------=
    // FUNC: register
    // DESC: Registers native readiness under one selector and preserves its initial readiness snapshot.
    // ------------------=
    fn register(&mut self, registry: &Registry, token: Token, interest: Interest) -> io::Result<()> {
        if interest.is_priority() || interest.is_aio() || interest.is_lio() { return Err(io::Error::from(io::ErrorKind::Unsupported)); }
        if self.binding.is_some() { return Err(io::Error::from(io::ErrorKind::AlreadyExists)); }
        let shared = &registry.selector().shared;
        let mut state = shared.state.lock().map_err(|_| io::Error::other("native selector poisoned"))?;
        let id = state.core.register(token.0, Readiness { readable: interest.is_readable(), writable: interest.is_writable() }, self.ready).map_err(error)?;
        if let Some(socket) = &self.socket {
            let Some(slot) = state.sockets.iter_mut().find(|slot| slot.is_none()) else {
                state.core.deregister(id).map_err(error)?;
                return Err(io::Error::from(io::ErrorKind::OutOfMemory));
            };
            *slot = Some((id, socket.clone()));
        }
        self.binding = Some((shared.clone(), id)); shared.changed.notify_one(); Ok(())
    }
    // ------------------------=
    // FUNC: reregister
    // DESC: Replaces interests/token only within the existing selector and rearms current readiness.
    // ------------------=
    fn reregister(&mut self, registry: &Registry, token: Token, interest: Interest) -> io::Result<()> {
        if interest.is_priority() || interest.is_aio() || interest.is_lio() { return Err(io::Error::from(io::ErrorKind::Unsupported)); }
        let Some((shared, id)) = &self.binding else { return Err(io::Error::from(io::ErrorKind::NotFound)); };
        if !Arc::ptr_eq(shared, &registry.selector().shared) { return Err(io::Error::from(io::ErrorKind::InvalidInput)); }
        let mut state = shared.state.lock().map_err(|_| io::Error::other("native selector poisoned"))?;
        state.core.reregister(*id, token.0, Readiness { readable: interest.is_readable(), writable: interest.is_writable() }, self.ready).map_err(error)?;
        shared.changed.notify_one(); Ok(())
    }
    // ------------------------=
    // FUNC: deregister
    // DESC: Removes pending events before releasing the registration and rejects a foreign registry.
    // ------------------=
    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        let Some((shared, id)) = &self.binding else { return Err(io::Error::from(io::ErrorKind::NotFound)); };
        if !Arc::ptr_eq(shared, &registry.selector().shared) { return Err(io::Error::from(io::ErrorKind::InvalidInput)); }
        let mut state = shared.state.lock().map_err(|_| io::Error::other("native selector poisoned"))?;
        state.core.deregister(*id).map_err(error)?;
        for socket in &mut state.sockets { if socket.as_ref().is_some_and(|(found,_)| found == id) { *socket = None; } }
        drop(state);
        self.binding = None; Ok(())
    }
}
impl Drop for NativeSource {
    // ------------------------=
    // FUNC: drop
    // DESC: Withdraws pending source events even if its service closes without explicit deregistration.
    // ------------------=
    fn drop(&mut self) { if let Some((shared, id)) = &self.binding { if let Ok(mut state) = shared.state.lock() {
        let _ = state.core.deregister(*id);
        for socket in &mut state.sockets { if socket.as_ref().is_some_and(|(found,_)| found == id) { *socket = None; } }
    } } }
}
// ------------------------=
// FUNC: socket_readiness
// DESC: Converts real TCP readiness and terminal failures into selector completion edges.
// ------------------=
pub(crate) fn socket_readiness(socket: &std::net::TcpStream) -> Readiness {
    let (readable, writable) = std::os::infinity_net::readiness(socket).unwrap_or((true, true));
    Readiness { readable, writable }
}
pub mod event {
    use super::*;
    // ------------------------=
    // FUNC: token
    // DESC: Returns the exact registered application token.
    // ------------------=
    pub fn token(event: &Event) -> Token { event.token }
    // ------------------------=
    // FUNC: is_readable
    // DESC: Reports readable readiness from the native service or control wake.
    // ------------------=
    pub fn is_readable(e: &Event) -> bool { e.ready.readable }
    // ------------------------=
    // FUNC: is_writable
    // DESC: Reports writable readiness only when the native service supplied it.
    // ------------------=
    pub fn is_writable(e: &Event) -> bool { e.ready.writable }
    // ------------------------=
    // FUNC: is_error
    // DESC: Successful control notifications have no socket error flag.
    // ------------------=
    pub fn is_error(_: &Event) -> bool { false }
    // ------------------------=
    // FUNC: is_read_closed
    // DESC: Control events do not represent TCP EOF.
    // ------------------=
    pub fn is_read_closed(_: &Event) -> bool { false }
    // ------------------------=
    // FUNC: is_write_closed
    // DESC: Control events do not represent TCP half-close.
    // ------------------=
    pub fn is_write_closed(_: &Event) -> bool { false }
    // ------------------------=
    // FUNC: is_priority
    // DESC: Control notifications have no out-of-band data.
    // ------------------=
    pub fn is_priority(_: &Event) -> bool { false }
    // ------------------------=
    // FUNC: is_aio
    // DESC: POSIX AIO is not used by this native backend.
    // ------------------=
    pub fn is_aio(_: &Event) -> bool { false }
    // ------------------------=
    // FUNC: is_lio
    // DESC: POSIX list IO is not used by this native backend.
    // ------------------=
    pub fn is_lio(_: &Event) -> bool { false }
    // ------------------------=
    // FUNC: debug_details
    // DESC: Provides bounded event diagnostics without influencing acceptance.
    // ------------------=
    pub fn debug_details(f: &mut fmt::Formatter<'_>, event: &Event) -> fmt::Result { write!(f, "native control {:?}", event.token) }
}
