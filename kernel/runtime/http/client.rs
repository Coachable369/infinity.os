//! Cooperative DNS/TCP/TLS request orchestration. No host sockets or executor.
use crate::{
    async_stream::Session,
    device::EthernetQueue,
    https,
    transport::{Error as TransportError, Transport},
};
use core::{
    future::{poll_fn, Future},
    task::Poll,
};
use smoltcp::{iface::SocketStorage, socket::tcp::State, time::Instant};

/// The service owner supplies NIC queues, monotonic time and current authority.
/// `allowed` must also reject revoked capabilities or changed interface configuration.
pub trait Link {
    // ------------------------=
    // FUNC: now
    // DESC: Supplies trusted monotonic time for transaction deadlines.
    // ------------------=
    fn now(&self) -> Instant;
    // ------------------------=
    // FUNC: allowed
    // DESC: Revalidates the current owner's capability, endpoint policy and network configuration.
    // ------------------=
    fn allowed(&mut self, destination: [u8; 4], port: u16) -> bool;
    // ------------------------=
    // FUNC: receive
    // DESC: Takes at most one frame from the service's bounded ingress queue without waiting.
    // ------------------=
    fn receive(&mut self, frame: &mut [u8; 1514]) -> Option<usize>;
    // ------------------------=
    // FUNC: transmit
    // DESC: Attempts one NIC submission and returns false on backpressure.
    // ------------------=
    fn transmit(&mut self, frame: &[u8]) -> bool;
    // ------------------------=
    // FUNC: register_waker
    // DESC: Arranges a wake on NIC progress or the next bounded network timer tick, never immediately rescheduling a spin loop.
    // ------------------=
    fn register_waker(&mut self, waker: &core::task::Waker);
}

#[derive(Clone, Copy)]
pub struct Configuration {
    pub mac: [u8; 6],
    pub address: [u8; 4],
    pub prefix: u8,
    pub gateway: Option<[u8; 4]>,
    pub dns_server: [u8; 4],
    pub local_port: u16,
    pub deadline: Instant,
}

#[derive(Clone, Copy)]
pub enum Destination {
    /// Resolve the TLS hostname through the configured DNS server.
    Resolve,
    /// An address already obtained by the owning resolver; still subject to policy.
    Address([u8; 4]),
}

#[derive(Debug)]
pub enum Error {
    Transport(TransportError),
    Https(https::Error),
    Denied,
    Timeout,
}

// ------------------------=
// FUNC: exchange
// DESC: Rechecks current authorization and deadline before moving at most four frames each way; retains unaccepted TX frames.
// ------------------=
fn exchange(
    link: &mut impl Link,
    queue: &mut EthernetQueue,
    destination: [u8; 4],
    port: u16,
    deadline: Instant,
) -> Result<Instant, Error> {
    let now = link.now();
    if now >= deadline {
        return Err(Error::Timeout);
    }
    if !link.allowed(destination, port) {
        return Err(Error::Denied);
    }
    let mut frame = [0; 1514];
    for _ in 0..4 {
        let Some(length) = link.receive(&mut frame) else {
            break;
        };
        if length > frame.len() {
            return Err(Error::Transport(TransportError::Configuration));
        }
        if length != 0 {
            let _ = queue.ingest(&frame[..length]);
        }
    }
    for _ in 0..4 {
        let Some(frame) = queue.pending() else {
            break;
        };
        if !link.transmit(frame) {
            break;
        }
        queue.transmitted();
    }
    Ok(now)
}

// ------------------------=
// FUNC: get
// DESC: Runs one cancellable bounded DNS/TCP/HTTPS transaction; the caller polls the future outside rendering and supplies genuine entropy and trusted time.
// ------------------=
pub async fn get<L: Link, R: rand_core::CryptoRngCore>(
    mut link: L,
    config: Configuration,
    destination: Destination,
    mut rng: R,
    roots: &[rustls_pki_types::TrustAnchor<'_>],
    unix_seconds: u64,
    host: &str,
    port: u16,
    path: &str,
    buffers: https::Buffers<'_>,
) -> Result<https::Response, Error> {
    let mut queue = EthernetQueue::new();
    let mut storage = [SocketStorage::EMPTY, SocketStorage::EMPTY];
    let mut queries = [None];
    let mut rx = [0; 16384];
    let mut tx = [0; 16384];
    let now = link.now();
    if now >= config.deadline {
        return Err(Error::Timeout);
    }
    let mut transport = Transport::new(
        &mut queue,
        config.mac,
        config.address,
        config.prefix,
        config.gateway,
        rng.next_u64(),
        now,
        &mut storage,
        &mut rx,
        &mut tx,
    )
    .map_err(Error::Transport)?;
    let address = match destination {
        Destination::Address(address) => address,
        Destination::Resolve => {
            if !link.allowed(config.dns_server, 53) {
                return Err(Error::Denied);
            }
            transport
                .enable_dns(config.dns_server, &mut queries)
                .map_err(Error::Transport)?;
            transport
                .resolve(host, now, config.deadline)
                .map_err(Error::Transport)?;
            poll_fn(|cx| {
                let now = match exchange(
                    &mut link,
                    &mut queue,
                    config.dns_server,
                    53,
                    config.deadline,
                ) {
                    Ok(now) => now,
                    Err(error) => {
                        transport.cancel();
                        return Poll::Ready(Err(error));
                    }
                };
                transport.poll(&mut queue, now);
                match transport.resolved_address() {
                    Ok(address) => Poll::Ready(Ok(address)),
                    Err(TransportError::WouldBlock) => {
                        link.register_waker(cx.waker());
                        Poll::Pending
                    }
                    Err(error) => Poll::Ready(Err(Error::Transport(error))),
                }
            })
            .await?
        }
    };
    // DNS and application traffic have separate authority. Discard residual DNS
    // retransmits/ARP frames before changing the endpoint checked by exchange.
    // The transport retains its neighbor cache, not these pending NIC frames.
    queue = EthernetQueue::new();
    if !link.allowed(address, port) {
        return Err(Error::Denied);
    }
    transport
        .connect(
            address,
            port,
            config.local_port,
            link.now(),
            config.deadline,
        )
        .map_err(Error::Transport)?;
    poll_fn(|cx| {
        let now = match exchange(&mut link, &mut queue, address, port, config.deadline) {
            Ok(now) => now,
            Err(error) => {
                transport.cancel();
                return Poll::Ready(Err(error));
            }
        };
        transport.poll(&mut queue, now);
        match transport.state() {
            State::Established => Poll::Ready(Ok(())),
            State::Closed => Poll::Ready(Err(Error::Transport(TransportError::Disconnected))),
            _ => {
                link.register_waker(cx.waker());
                Poll::Pending
            }
        }
    })
    .await?;
    let mut session = Session::new(transport);
    let stream = session.stream();
    let handle = stream.session();
    let mut request = core::pin::pin!(https::get(
        stream,
        rng,
        roots,
        unix_seconds,
        host,
        path,
        buffers
    ));
    let result = poll_fn(|cx| {
        let now = match exchange(&mut link, &mut queue, address, port, config.deadline) {
            Ok(now) => now,
            Err(error) => {
                handle.cancel();
                return Poll::Ready(Err(error));
            }
        };
        handle.poll(&mut queue, now);
        match request.as_mut().poll(cx) {
            Poll::Ready(result) => Poll::Ready(result.map_err(Error::Https)),
            Poll::Pending => {
                link.register_waker(cx.waker());
                Poll::Pending
            }
        }
    })
    .await;
    // A request owns its TCP session exclusively; do not keep live sockets after completion.
    handle.cancel();
    result
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
