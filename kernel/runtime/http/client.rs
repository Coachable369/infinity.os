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

/// A protocol conversation borrows one real endpoint-confined native stream.
/// The owning service keeps polling the same NIC and revocation checks while
/// the conversation is suspended. No socket escapes its lifetime.
pub trait Conversation {
    type Output;
    // ------------------------=
    // FUNC: resolved
    // DESC: Allows a DNS-only consumer to return the actual resolved address before any TCP connection is attempted.
    // ------------------=
    fn resolved(&mut self, _: [u8; 4]) -> Option<Self::Output> { None }
    // ------------------------=
    // FUNC: exchange
    // DESC: Executes a protocol over the owned stream while the shared connection driver retains transport authority.
    // ------------------=
    async fn exchange<S: embedded_io_async::Read + embedded_io_async::Write>(
        &mut self, stream: S,
    ) -> Result<Self::Output, Error>;
}

pub struct ResolveAddress;
impl Conversation for ResolveAddress {
    type Output = [u8; 4];
    // ------------------------=
    // FUNC: resolved
    // DESC: Completes native getaddrinfo using the resolver's actual answer without issuing TCP traffic.
    // ------------------=
    fn resolved(&mut self, address: [u8; 4]) -> Option<Self::Output> { Some(address) }
    // ------------------------=
    // FUNC: exchange
    // DESC: Rejects accidental stream invocation because DNS-only conversations must finish before connect.
    // ------------------=
    async fn exchange<S: embedded_io_async::Read + embedded_io_async::Write>(
        &mut self, _: S,
    ) -> Result<Self::Output, Error> { Err(Error::Denied) }
}

struct HttpExchange<'a, R, P> {
    rng: Option<R>, roots: &'a [rustls_pki_types::TrustAnchor<'a>], unix_seconds: u64,
    host: &'a str, path: &'a str, method: &'a str, request_headers: &'a [u8], request_body: &'a [u8],
    buffers: Option<https::Buffers<'a>>, headers: Option<&'a mut [u8]>, progress: P,
}
impl<R: rand_core::CryptoRngCore, P: FnMut(Option<&[u8]>, &[u8]) -> bool> Conversation for HttpExchange<'_, R, P> {
    type Output = https::Response;
    // ------------------------=
    // FUNC: exchange
    // DESC: Preserves the existing HTTP TLS request implementation inside the reusable native connection lifetime.
    // ------------------=
    async fn exchange<S: embedded_io_async::Read + embedded_io_async::Write>(
        &mut self, stream: S,
    ) -> Result<Self::Output, Error> {
        https::request_streaming(stream, self.rng.take().ok_or(Error::Denied)?, self.roots,
            self.unix_seconds, self.host, self.path, self.method, self.request_headers, self.request_body,
            self.buffers.take().ok_or(Error::Denied)?, self.headers.take(), &mut self.progress)
            .await.map_err(Error::Https)
    }
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
        if !queue.can_ingest() {
            break;
        }
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
    link: L,
    config: Configuration,
    destination: Destination,
    rng: R,
    roots: &[rustls_pki_types::TrustAnchor<'_>],
    unix_seconds: u64,
    host: &str,
    port: u16,
    path: &str,
    buffers: https::Buffers<'_>,
) -> Result<https::Response, Error> {
    get_with_headers(link, config, destination, rng, roots, unix_seconds, host,
                     port, path, buffers, None).await
}

// ------------------------=
// FUNC: get_with_headers
// DESC: Runs the same capability-governed native transaction with optional bounded final-header retention.
// ------------------=
pub async fn get_with_headers<L: Link, R: rand_core::CryptoRngCore>(
    link: L, config: Configuration, destination: Destination, rng: R,
    roots: &[rustls_pki_types::TrustAnchor<'_>], unix_seconds: u64,
    host: &str, port: u16, path: &str, buffers: https::Buffers<'_>,
    headers: Option<&mut [u8]>,
) -> Result<https::Response, Error> {
    get_with_request_headers(link, config, destination, rng, roots, unix_seconds, host, port, path, &[], buffers, headers).await
}

// ------------------------=
// FUNC: get_with_request_headers
// DESC: Carries validated engine fields through the same deadline-bound native transport.
// ------------------=
pub async fn get_with_request_headers<L: Link, R: rand_core::CryptoRngCore>(
    link: L,
    config: Configuration,
    destination: Destination,
    rng: R,
    roots: &[rustls_pki_types::TrustAnchor<'_>],
    unix_seconds: u64,
    host: &str,
    port: u16,
    path: &str,
    request_headers: &[u8],
    buffers: https::Buffers<'_>,
    headers: Option<&mut [u8]>,
) -> Result<https::Response, Error> {
    get_streaming(link, config, destination, rng, roots, unix_seconds, host, port, path,
        request_headers, buffers, headers, |_, _| true).await
}

// ------------------------=
// FUNC: get_streaming
// DESC: Keeps streaming response delivery inside the same revocable endpoint authority and transaction deadline.
// ------------------=
pub async fn get_streaming<L: Link, R: rand_core::CryptoRngCore>(
    link: L, config: Configuration, destination: Destination, rng: R,
    roots: &[rustls_pki_types::TrustAnchor<'_>], unix_seconds: u64,
    host: &str, port: u16, path: &str, request_headers: &[u8],
    buffers: https::Buffers<'_>, headers: Option<&mut [u8]>,
    progress: impl FnMut(Option<&[u8]>, &[u8]) -> bool,
) -> Result<https::Response, Error> {
    request_streaming(link,config,destination,rng,roots,unix_seconds,host,port,path,"GET",
        request_headers,&[],buffers,headers,progress).await
}

// ------------------------=
// FUNC: request_streaming
// DESC: Applies native endpoint authority, cancellation, and deadlines to standard methods and bounded request bodies.
// ------------------=
pub async fn request_streaming<L:Link,R:rand_core::CryptoRngCore>(
    link:L,config:Configuration,destination:Destination,mut rng:R,
    roots:&[rustls_pki_types::TrustAnchor<'_>],unix_seconds:u64,
    host:&str,port:u16,path:&str,method:&str,request_headers:&[u8],request_body:&[u8],
    buffers:https::Buffers<'_>,headers:Option<&mut[u8]>,progress:impl FnMut(Option<&[u8]>,&[u8])->bool,
) -> Result<https::Response, Error> {
    let seed = rng.next_u64();
    connect(link, config, destination, seed, host, port, HttpExchange { rng: Some(rng), roots,
        unix_seconds, host, path, method, request_headers, request_body,
        buffers: Some(buffers), headers, progress }).await
}

// ------------------------=
// FUNC: connect
// DESC: Shares DNS, TCP establishment, NIC pumping, deadlines and revocation between HTTP and native socket protocol conversations.
// ------------------=
pub async fn connect<L: Link, C: Conversation>(
    mut link: L, config: Configuration, destination: Destination, seed: u64,
    host: &str, port: u16, mut conversation: C,
) -> Result<C::Output, Error> {
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
        seed,
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
    if let Some(result) = conversation.resolved(address) {
        transport.cancel();
        return Ok(result);
    }
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
    let mut request = core::pin::pin!(conversation.exchange(stream));
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
            Poll::Ready(result) => Poll::Ready(result),
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
