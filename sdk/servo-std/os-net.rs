//! Explicit InfinityOS networking extension; does not expose POSIX descriptors.
use crate::{io, net::{TcpStream, SocketAddr}, sys::{AsInner, FromInner}};
// ------------------------=
// FUNC: connect_nonblocking
// DESC: Starts capability-governed TCP without waiting on the calling task.
// ------------------=
#[stable(feature = "infinity_native_net", since = "1.98.0")]
pub fn connect_nonblocking(addr: &SocketAddr) -> io::Result<TcpStream> {
    crate::sys::net::TcpStream::nonblocking_connect(addr).map(TcpStream::from_inner)
}
// ------------------------=
// FUNC: readiness
// DESC: Inspects the native connection without transferring its authority or raw handle.
// ------------------=
#[stable(feature = "infinity_native_net", since = "1.98.0")]
pub fn readiness(stream: &TcpStream) -> io::Result<(bool, bool)> { stream.as_inner().readiness() }
