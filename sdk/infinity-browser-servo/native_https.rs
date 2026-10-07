//! Native TLS service adapter; no host sockets, unverified certificates or blocking waits.
use std::{boxed::Box, future::Future, pin::Pin, string::String, task::{Context, Poll, Waker}, vec::Vec};
use infinity_browser_native_network as net;
use net::{client::{Configuration, Destination, Link}, rand_core::SeedableRng};
use super::resources::{Provider, Response, Event};
use std::{cell::RefCell, rc::Rc};

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
    // FUNC: failed_request
    // DESC: Associates fixture transport failures with their requested resource without changing authority.
    // ------------------=
    fn failed_request(_url: &str, error: &net::client::Error) { Self::failed(error); }
    // ------------------------=
    // FUNC: completed
    // DESC: Publishes bounded response metadata for service telemetry without exposing document contents.
    // ------------------=
    fn completed(_status: u16, _body_bytes: usize) {}
    // ------------------------=
    // FUNC: completed_request
    // DESC: Associates fixture completion telemetry with its resource without changing transport behavior.
    // ------------------=
    fn completed_request(_url:&str,status:u16,body_bytes:usize) {Self::completed(status,body_bytes);}
}
type Request = Pin<Box<dyn Future<Output = Result<Response, ()>>>>;
type Events = Rc<RefCell<Option<Result<Event, ()>>>>;
pub struct Https<F> { factory: F, next: u64, requests: Vec<(u64, Request, Events)> }
impl<F: Factory> Https<F> {
    // ------------------------=
    // FUNC: new
    // DESC: Serializes bounded requests over the native service link to avoid competing NIC consumers.
    // ------------------=
    pub fn new(factory: F) -> Self { Self { factory, next: 0, requests: Vec::with_capacity(16) } }
}
impl<F: Factory> Provider for Https<F> {
    // ------------------------=
    // FUNC: retry_pre_header_failures
    // DESC: Allows bounded retries for transient native transport failures before response exposure.
    // ------------------=
    fn retry_pre_header_failures(&self) -> bool { true }
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
        self.begin_request(url,"GET",headers,&[])
    }
    // ------------------------=
    // FUNC: begin_request
    // DESC: Preserves standard request methods and bounded binary bodies through native TLS.
    // ------------------=
    fn begin_request(&mut self,url:&str,method:&str,headers:&[u8],body:&[u8])->Result<u64,()> {
        net::request::validate_method(method).map_err(|_|())?;
        if body.len()>net::request::BODY_LIMIT {return Err(());}
        let method=String::from(method);let request_body=body.to_vec();
        let request_headers=headers.to_vec();
        let requested_url=String::from(url);
        if self.requests.len() == 16 { return Err(()); }
        let url = servo::ServoUrl::parse(url).map_err(|_| ())?.into_url();
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() { return Err(()); }
        let host = String::from(url.host_str().ok_or(())?);
        let port = url.port_or_known_default().ok_or(())?;
        let (link, config, utc, seed) = self.factory.authorize(&host, port)?;
        let mut path = String::from(url.path());
        if let Some(query) = url.query() { path.push('?'); path.push_str(query); }
        self.next = self.next.checked_add(1).ok_or(())?;
        let events = Rc::new(RefCell::new(None));
        let output = events.clone();
        let future = Box::pin(async move {
            let mut read = std::vec![0;16640];
            let mut write = std::vec![0;4096];
            let mut request = std::vec![0;17408];
            let mut body = std::vec![0;super::resources::MAX_BODY];
            let mut head = std::vec![0;8192];
            let result = net::client::request_streaming(link, config, Destination::Resolve,
                net::rand_chacha::ChaCha20Rng::from_seed(seed), net::TLS_SERVER_ROOTS, utc,
                &host, port, &path, &method, &request_headers, &request_body, net::https::Buffers { read_record: &mut read,
                    write_record: &mut write, request: &mut request, response: &mut body }, Some(&mut head),
                |head, bytes| {
                    if output.borrow().is_some() { return false; }
                    let event = if let Some(head) = head {
                        (|| {
                            let status = net::response::parse(head, false).map_err(|_|())?.ok_or(())?.status;
                            let parsed = net::response::Headers::parse(head).map_err(|_|())?;
                            let mut fields = Vec::new();
                            for (name,value) in parsed.iter() {
                                if name.eq_ignore_ascii_case("transfer-encoding") || name.eq_ignore_ascii_case("connection") {continue;}
                                fields.push((String::from(name), String::from(core::str::from_utf8(value).map_err(|_|())?)));
                            }
                            Ok(Event::Head(status,fields))
                        })()
                    } else { Ok(Event::Data(bytes.to_vec())) };
                    *output.borrow_mut() = Some(event); true
                })
                .await.map_err(|error| { F::failed_request(&requested_url,&error); })?;
            core::future::poll_fn(|_| if output.borrow().is_none() { Poll::Ready(()) } else { Poll::Pending }).await;
            let parsed = net::response::Headers::parse(&head[..result.header_bytes]).map_err(|_| ())?;
            let mut headers = Vec::new();
            for (name, value) in parsed.iter() {
                // Native client already decodes transfer framing. Never ask Servo to decode it twice.
                if name.eq_ignore_ascii_case("transfer-encoding") || name.eq_ignore_ascii_case("connection") { continue; }
                headers.push((String::from(name), String::from(core::str::from_utf8(value).map_err(|_| ())?)));
            }
            body.truncate(result.body_bytes);
            F::completed_request(&requested_url,result.status, result.body_bytes);
            Ok(Response { status: result.status, headers, body })
        });
        self.requests.push((self.next, future, events));
        Ok(self.next)
    }
    // ------------------------=
    // FUNC: poll_stream
    // DESC: Advances only the oldest native transaction once; pending never spins on the UI caller.
    // ------------------=
    fn poll_stream(&mut self, id: u64) -> Result<Option<Event>, ()> {
        let Some((first, future, events)) = self.requests.first_mut() else { return Err(()); };
        if *first != id { return if self.requests.iter().any(|(key,_,_)| *key==id) { Ok(None) } else { Err(()) }; }
        if let Some(event) = events.borrow_mut().take() { return event.map(Some); }
        match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Pending => events.borrow_mut().take().transpose(),
            Poll::Ready(result) => result.map(|_|Some(Event::Done)),
        }
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Drops the owning future and its transport buffers on completion or navigation cancellation.
    // ------------------=
    fn cancel(&mut self, id: u64) {
        if let Some(at) = self.requests.iter().position(|(key,_,_)| *key==id) { drop(self.requests.remove(at)); }
    }
}
