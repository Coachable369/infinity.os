//! Owner-thread Servo resource adapter. All HTTP loads are intercepted; there is
//! no fallback to an ambient socket. The provider must enforce OS capabilities.
use std::{cell::RefCell, vec::Vec};
use servo::{WebResourceLoad, WebResourceResponse};
#[path = "../infinity-browser-core/download.rs"]
pub mod download;
#[path = "../infinity-browser-core/resource_order.rs"]
mod resource_order;

pub const MAX_BODY: usize = 4 * 1024 * 1024;
const MAX_REQUESTS: usize = 256;
const TIMEOUT_NS: u64 = 120_000_000_000;

pub struct Response {
    pub status: u16,
    pub headers: Vec<(std::string::String, std::string::String)>,
    pub body: Vec<u8>,
}
pub enum Event { Complete(Response), Head(u16, Vec<(std::string::String, std::string::String)>), Data(Vec<u8>), Done }
struct Stream {
    id: u64, deadline: u64, document: bool, size: usize,
    load: servo::InterceptedWebResourceLoad,
    attachment: Option<(download::Metadata, Vec<u8>)>,
}

pub trait Provider {
    // ------------------------=
    // FUNC: begin
    // DESC: Submits a nonblocking authorized request; rejection must grant no socket authority.
    // ------------------=
    fn begin(&mut self, url: &str) -> Result<u64, ()>;
    // ------------------------=
    // FUNC: begin_with_headers
    // DESC: Carries engine-selected request metadata; fixture providers may use the URL-only default.
    // ------------------=
    fn begin_with_headers(&mut self, url: &str, _headers: &[u8]) -> Result<u64, ()> { self.begin(url) }
    // ------------------------=
    // FUNC: begin_request
    // DESC: Carries the engine's validated method and bounded binary body without inventing request authority.
    // ------------------=
    fn begin_request(&mut self,url:&str,method:&str,headers:&[u8],body:&[u8])->Result<u64,()> {
        if method=="GET" && body.is_empty() {self.begin_with_headers(url,headers)} else {Err(())}
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Returns a bounded complete native-service response or a pending state.
    // ------------------=
    fn poll(&mut self, _id: u64) -> Result<Option<Response>, ()> { Err(()) }
    // ------------------------=
    // FUNC: poll_stream
    // DESC: Delivers one bounded transport event; complete-response fixtures retain their existing contract.
    // ------------------=
    fn poll_stream(&mut self, id: u64) -> Result<Option<Event>, ()> { self.poll(id).map(|value|value.map(Event::Complete)) }
    // ------------------------=
    // FUNC: cancel
    // DESC: Releases request resources after success, failure, revocation or supersession.
    // ------------------=
    fn cancel(&mut self, id: u64);
    // ------------------------=
    // FUNC: document_failed
    // DESC: Publishes document failure immediately without waiting for the engine's failed-navigation processing.
    // ------------------=
    fn document_failed(&mut self) {}
    // ------------------------=
    // FUNC: download
    // DESC: Offers a complete bounded attachment to native storage consent, never an engine filesystem path.
    // ------------------=
    fn download(&mut self, _metadata: &download::Metadata, _body: &[u8]) -> Result<(), ()> { Err(()) }
}

struct Pending { id: u64, deadline: u64, priority: resource_order::Priority, load: WebResourceLoad }
struct State<P> { provider: P, pending: Vec<Pending>, stream: Option<Stream>, closed: bool, failed_document: bool }
pub struct Resources<P: Provider> { state: RefCell<State<P>>, clock: fn() -> u64 }

impl<P: Provider> servo::ServoDelegate for Resources<P> {
    // ------------------------=
    // FUNC: load_web_resource
    // DESC: Applies the same boundary to worker/global loads that have no WebView delegate.
    // ------------------=
    fn load_web_resource(&self, load: WebResourceLoad) { self.submit(load); }
}

// ------------------------=
// FUNC: reject
// DESC: Explicitly cancels interception; dropping an unhandled load would allow ambient networking.
// ------------------=
fn reject(load: WebResourceLoad) {
    let response = WebResourceResponse::new(load.request().url.clone());
    load.intercept(response).cancel();
}

// ------------------------=
// FUNC: fail_load
// DESC: Notifies the shell only for document failure; optional subresources still fail closed without a page-wide error.
// ------------------=
fn fail_load<P: Provider>(state: &mut State<P>, load: WebResourceLoad) {
    if load.request().is_for_main_frame { state.failed_document=true;state.provider.document_failed(); }
    let response = WebResourceResponse::new(load.request().url.clone());
    load.intercept(response).fail();
}

impl<P: Provider> Resources<P> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded resource queue without opening any network connections.
    // ------------------=
    pub fn new(provider: P, clock: fn() -> u64) -> Self {
        Self { state: RefCell::new(State { provider, pending: Vec::with_capacity(MAX_REQUESTS), stream: None, closed: false, failed_document: false }), clock }
    }
    // ------------------------=
    // FUNC: submit
    // DESC: Intercepts main documents and subresources with identical policy and queue limits.
    // ------------------=
    pub fn submit(&self, load: WebResourceLoad) {
        let mut state = self.state.borrow_mut();
        if state.closed {reject(load);return;}
        let request = load.request();
        let url = &request.url;
        // Dropping without interception invokes Servo's own local scheme handler.
        // These schemes cannot open an ambient socket; retain Servo's origin/CSP checks.
        if matches!(url.scheme(), "data" | "blob" | "about") { return; }
        if state.pending.len() == MAX_REQUESTS || !matches!(request.method.as_str(),"GET"|"HEAD"|"POST"|"PUT"|"PATCH"|"DELETE"|"OPTIONS")
            || request.body.len()>64*1024
            || !matches!(url.scheme(), "http" | "https") || url.as_str().len() > 8192
            || !url.username().is_empty() || url.password().is_some() {
            fail_load(&mut state, load); return;
        }
        // Queue metadata, not response buffers. Start the transfer deadline only
        // when this resource reaches the head of the serialized native transport.
        let priority=resource_order::classify(request.is_for_main_frame,request.destination.as_str());
        // Keep active work and equal-priority FIFO order; unblock document
        // parsing/layout before spending the serialized transport on media.
        let at=resource_order::insertion_index(state.pending.iter().map(|pending|(pending.id!=0,pending.priority)),priority);
        state.pending.insert(at,Pending { id: 0, deadline: 0, priority, load });
    }
    // ------------------------=
    // FUNC: pump
    // DESC: Polls each queued request once without blocking the engine or desktop event loop.
    // ------------------=
    pub fn pump(&self) {
        let mut state = self.state.borrow_mut();
        let now = (self.clock)();
        if let Some(mut stream) = state.stream.take() {
            let event = if now >= stream.deadline { Err(()) } else { state.provider.poll_stream(stream.id) };
            match event {
                Ok(None) => { state.stream = Some(stream); return; },
                Ok(Some(Event::Data(bytes))) if bytes.len() <= MAX_BODY.saturating_sub(stream.size) => {
                    stream.size += bytes.len();
                    if let Some((_, body)) = &mut stream.attachment { body.extend_from_slice(&bytes); }
                    else { stream.load.send_body_data(bytes); }
                    state.stream = Some(stream); return;
                },
                Ok(Some(Event::Done)) => {
                    state.provider.cancel(stream.id);
                    if let Some((metadata, body)) = stream.attachment {
                        if state.provider.download(&metadata, &body).is_ok() { stream.load.cancel(); }
                        else {
                            if stream.document { state.failed_document = true; state.provider.document_failed(); }
                            stream.load.fail();
                        }
                    } else { stream.load.finish(); }
                },
                _ => {
                    state.provider.cancel(stream.id);
                    if stream.document { state.failed_document = true; state.provider.document_failed(); }
                    stream.load.fail();
                },
            }
        }
        let at = 0;
        while at < state.pending.len() {
            if state.pending[at].id == 0 {
                let mut url=state.pending[at].load.request().url.clone();
                url.set_fragment(None);
                let mut headers=Vec::new();
                let mut oversized=false;
                for (name,value) in &state.pending[at].load.request().headers {
                    if matches!(name.as_str(), "host"|"connection"|"transfer-encoding"|"content-length"|"accept-encoding"|"proxy-authorization"|"proxy-connection"|"upgrade"|"te"|"trailer") { continue; }
                    if headers.len().saturating_add(name.as_str().len()).saturating_add(value.as_bytes().len()).saturating_add(4)>8192 {oversized=true;break;}
                    headers.extend_from_slice(name.as_str().as_bytes());
                    headers.extend_from_slice(b": ");headers.extend_from_slice(value.as_bytes());headers.extend_from_slice(b"\r\n");
                }
                if oversized {let pending=state.pending.remove(at);fail_load(&mut state,pending.load);continue;}
                let method=state.pending[at].load.request().method.clone();
                let body=state.pending[at].load.request().body.clone();
                match state.provider.begin_request(url.as_str(),method.as_str(), &headers,&body) {
                    Ok(id) if id != 0 => {
                        state.pending[at].id=id;
                        state.pending[at].deadline=now.saturating_add(TIMEOUT_NS);
                    },
                    _ => {
                        let pending=state.pending.remove(at);
                        fail_load(&mut state,pending.load);
                        continue;
                    }
                }
            }
            let id = state.pending[at].id;
            let result = if now >= state.pending[at].deadline { Err(()) } else { state.provider.poll_stream(id) };
            if matches!(result, Ok(None)) { break; }
            let pending = state.pending.remove(at);
            if let Ok(Some(Event::Head(status, ref headers))) = result {
                let document = pending.load.request().is_for_main_frame;
                let mut response = WebResourceResponse::new(pending.load.request().url.clone());
                let mut valid = (200..=599).contains(&status) && headers.len() <= 128;
                let mut bytes = 0usize;
                for (name, value) in headers {
                    bytes = bytes.saturating_add(name.len()).saturating_add(value.len());
                    let (Ok(name), Ok(value)) = (name.parse::<http::header::HeaderName>(), value.parse()) else { valid=false; break; };
                    response.headers.append(name, value);
                }
                valid &= bytes <= 16384;
                let attachment = if document && (200..300).contains(&status) {
                    let fields: Vec<_> = headers.iter().filter(|(name,_)|name.eq_ignore_ascii_case("content-disposition")).collect();
                    if fields.len()>1 { valid=false; }
                    let media=headers.iter().find(|(name,_)|name.eq_ignore_ascii_case("content-type")).map_or("",|(_,value)|value.as_str());
                    match download::attachment(fields.first().map_or("",|(_,value)|value.as_str()),media) {
                        Ok(value)=>value.map(|metadata|(metadata, Vec::new())), Err(_)=>{valid=false;None},
                    }
                } else { None };
                if !valid { state.provider.cancel(id); fail_load(&mut state,pending.load); continue; }
                response.status_code = status.try_into().unwrap();
                state.stream = Some(Stream { id, deadline: pending.deadline, document, size: 0,
                    load: pending.load.intercept(response), attachment });
                return;
            }
            state.provider.cancel(id);
            let Ok(Some(Event::Complete(result))) = result else {
                fail_load(&mut state, pending.load); continue;
            };
            if result.body.len() > MAX_BODY || result.headers.len() > 128 || !(200..=599).contains(&result.status) {
                fail_load(&mut state, pending.load); continue;
            }
            if pending.load.request().is_for_main_frame && (200..300).contains(&result.status) {
                let dispositions: Vec<_> = result.headers.iter().filter(|(name,_)|name.eq_ignore_ascii_case("content-disposition")).collect();
                if dispositions.len()>1 {fail_load(&mut state,pending.load);continue;}
                let media_type=result.headers.iter().find(|(name,_)|name.eq_ignore_ascii_case("content-type"))
                    .map_or("",|(_,value)|value.as_str());
                let metadata=download::attachment(dispositions.first().map_or("",|(_,value)|value.as_str()),media_type);
                match metadata {
                    Ok(Some(metadata))=>{
                        if state.provider.download(&metadata,&result.body).is_ok() {reject(pending.load);}
                        else {fail_load(&mut state,pending.load);}
                        continue;
                    },
                    Err(_)=>{fail_load(&mut state,pending.load);continue;},
                    Ok(None)=>{},
                }
            }
            let mut response = WebResourceResponse::new(pending.load.request().url.clone());
            response.status_code = result.status.try_into().unwrap();
            let mut valid = true;
            let mut bytes = 0usize;
            for (name, value) in result.headers {
                bytes = bytes.saturating_add(name.len()).saturating_add(value.len());
                if bytes > 16384 { valid = false; break; }
                let (Ok(name), Ok(value)) = (name.parse::<http::header::HeaderName>(), value.parse()) else {
                    valid = false; break;
                };
                response.headers.append(name, value);
            }
            if !valid {
                fail_load(&mut state, pending.load); continue;
            }
            let mut intercepted = pending.load.intercept(response);
            intercepted.send_body_data(result.body);
            intercepted.finish();
        }
    }
    // ------------------------=
    // FUNC: failed_document
    // DESC: Allows retry to await the failed document's lifecycle completion rather than lose a replacement navigation.
    // ------------------=
    pub fn failed_document(&self) -> bool { self.state.borrow().failed_document }
    // ------------------------=
    // FUNC: cancel_all
    // DESC: Cancels superseded navigation and all subresources without closing the reusable adapter.
    // ------------------=
    pub fn cancel_all(&self) {
        let mut state = self.state.borrow_mut();
        state.failed_document=false;
        if let Some(stream) = state.stream.take() { state.provider.cancel(stream.id); stream.load.cancel(); }
        while let Some(pending) = state.pending.pop() {
            if pending.id != 0 { state.provider.cancel(pending.id); }
            reject(pending.load);
        }
    }
    // ------------------------=
    // FUNC: close
    // DESC: Rejects future requests and releases active native-service operations.
    // ------------------=
    pub fn close(&self) { self.state.borrow_mut().closed = true; self.cancel_all(); }
}

impl<P: Provider> Drop for Resources<P> {
    // ------------------------=
    // FUNC: drop
    // DESC: Ensures teardown cannot accidentally fall back to Servo networking.
    // ------------------=
    fn drop(&mut self) { self.close(); }
}
