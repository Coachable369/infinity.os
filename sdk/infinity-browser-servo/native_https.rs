//! Native TLS service adapter; no host sockets, unverified certificates or blocking waits.
use std::{boxed::Box, future::Future, pin::Pin, string::String, task::{Context, Poll, Waker}, vec::Vec};
use infinity_browser_native_network as net;
use net::{client::{Configuration, Destination, Link}, rand_core::SeedableRng};
use super::resources::{Provider, Response};

pub trait Factory {
    type Connection: Link + 'static;
    // ------------------------=
    // FUNC: authorize
    // DESC: Acquires endpoint-scoped native authority plus trusted time and cryptographic entropy.
    // ------------------=
    fn authorize(&mut self, host: &str, port: u16) -> Result<(Self::Connection, Configuration, u64, [u8;32]), ()>;
    // ------------------------=
    // FUNC: failed
    // DESC: Reports one terminal transport failure to native diagnostics, never raw text to page content.
    // ------------------=
    fn failed(_error: &net::client::Error) {}
    // ------------------------=
    // FUNC: completed
    // DESC: Publishes bounded response metadata for service telemetry without exposing document contents.
    // ------------------=
    fn completed(_status: u16, _body_bytes: usize) {}
}
type Request = Pin<Box<dyn Future<Output = Result<Response, ()>>>>;
pub struct Https<F> { factory: F, next: u64, requests: Vec<(u64, Request)> }
impl<F: Factory> Https<F> {
    // ------------------------=
    // FUNC: new
    // DESC: Serializes bounded requests over the native service link to avoid competing NIC consumers.
    // ------------------=
    pub fn new(factory: F) -> Self { Self { factory, next: 0, requests: Vec::with_capacity(16) } }
}
impl<F: Factory> Provider for Https<F> {
    // ------------------------=
    // FUNC: begin
    // DESC: Authorizes an HTTPS GET and creates a cancellable transaction with bounded buffers.
    // ------------------=
    fn begin(&mut self, url: &str) -> Result<u64, ()> {
        self.begin_with_headers(url, &[])
    }
    // ------------------------=
    // FUNC: begin_with_headers
    // DESC: Preserves bounded engine metadata throughout native TLS processing.
    // ------------------=
    fn begin_with_headers(&mut self, url: &str, headers: &[u8]) -> Result<u64, ()> {
        let request_headers=headers.to_vec();
        if self.requests.len() == 16 { return Err(()); }
        let url = servo::ServoUrl::parse(url).map_err(|_| ())?.into_url();
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() { return Err(()); }
        let host = String::from(url.host_str().ok_or(())?);
        let port = url.port_or_known_default().ok_or(())?;
        let (link, config, utc, seed) = self.factory.authorize(&host, port)?;
        let mut path = String::from(url.path());
        if let Some(query) = url.query() { path.push('?'); path.push_str(query); }
        self.next = self.next.checked_add(1).ok_or(())?;
        let future = Box::pin(async move {
            let mut read = std::vec![0;16640];
            let mut write = std::vec![0;4096];
            let mut request = std::vec![0;12288];
            let mut body = std::vec![0;super::resources::MAX_BODY];
            let mut head = std::vec![0;8192];
            let result = net::client::get_with_request_headers(link, config, Destination::Resolve,
                net::rand_chacha::ChaCha20Rng::from_seed(seed), net::TLS_SERVER_ROOTS, utc,
                &host, port, &path, &request_headers, net::https::Buffers { read_record: &mut read,
                    write_record: &mut write, request: &mut request, response: &mut body }, Some(&mut head))
                .await.map_err(|error| { F::failed(&error); })?;
            let parsed = net::response::Headers::parse(&head[..result.header_bytes]).map_err(|_| ())?;
            let mut headers = Vec::new();
            for (name, value) in parsed.iter() {
                // Native client already decodes transfer framing. Never ask Servo to decode it twice.
                if name.eq_ignore_ascii_case("transfer-encoding") || name.eq_ignore_ascii_case("connection") { continue; }
                headers.push((String::from(name), String::from(core::str::from_utf8(value).map_err(|_| ())?)));
            }
            body.truncate(result.body_bytes);
            F::completed(result.status, result.body_bytes);
            Ok(Response { status: result.status, headers, body })
        });
        self.requests.push((self.next, future));
        Ok(self.next)
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Advances only the oldest native transaction once; pending never spins on the UI caller.
    // ------------------=
    fn poll(&mut self, id: u64) -> Result<Option<Response>, ()> {
        let Some((first, future)) = self.requests.first_mut() else { return Err(()); };
        if *first != id { return if self.requests.iter().any(|(key,_)| *key==id) { Ok(None) } else { Err(()) }; }
        match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Pending => Ok(None), Poll::Ready(result) => result.map(Some),
        }
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Drops the owning future and its transport buffers on completion or navigation cancellation.
    // ------------------=
    fn cancel(&mut self, id: u64) {
        if let Some(at) = self.requests.iter().position(|(key,_)| *key==id) { drop(self.requests.remove(at)); }
    }
}
