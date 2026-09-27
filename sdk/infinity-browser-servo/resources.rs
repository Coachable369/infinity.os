//! Owner-thread Servo resource adapter. All HTTP loads are intercepted; there is
//! no fallback to an ambient socket. The provider must enforce OS capabilities.
use std::{cell::RefCell, vec::Vec};
use servo::{WebResourceLoad, WebResourceResponse};

pub const MAX_BODY: usize = 1024 * 1024;
const MAX_REQUESTS: usize = 16;
const TIMEOUT_NS: u64 = 30_000_000_000;

pub struct Response {
    pub status: u16,
    pub headers: Vec<(std::string::String, std::string::String)>,
    pub body: Vec<u8>,
}

pub trait Provider {
    // ------------------------=
    // FUNC: begin
    // DESC: Submits a nonblocking authorized request; rejection must grant no socket authority.
    // ------------------=
    fn begin(&mut self, url: &str) -> Result<u64, ()>;
    // ------------------------=
    // FUNC: poll
    // DESC: Returns a bounded complete native-service response or a pending state.
    // ------------------=
    fn poll(&mut self, id: u64) -> Result<Option<Response>, ()>;
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
}

struct Pending { id: u64, deadline: u64, load: WebResourceLoad }
struct State<P> { provider: P, pending: Vec<Pending>, closed: bool, failed_document: bool }
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
        Self { state: RefCell::new(State { provider, pending: Vec::with_capacity(MAX_REQUESTS), closed: false, failed_document: false }), clock }
    }
    // ------------------------=
    // FUNC: submit
    // DESC: Intercepts main documents and subresources with identical policy and queue limits.
    // ------------------=
    pub fn submit(&self, load: WebResourceLoad) {
        let mut state = self.state.borrow_mut();
        let request = load.request();
        let url = &request.url;
        if state.closed || state.pending.len() == MAX_REQUESTS || request.method.as_str() != "GET"
            || !matches!(url.scheme(), "http" | "https") || url.as_str().len() > 2048
            || !url.username().is_empty() || url.password().is_some() {
            fail_load(&mut state, load); return;
        }
        // Fragments identify document locations, never HTTP request targets.
        // Retain the original URL on `load` for Servo history and anchor handling.
        let mut network_url=url.clone();
        network_url.set_fragment(None);
        let Ok(id) = state.provider.begin(network_url.as_str()) else {
            fail_load(&mut state, load); return;
        };
        if id == 0 || state.pending.iter().any(|pending| pending.id == id) {
            state.provider.cancel(id); fail_load(&mut state, load); return;
        }
        state.pending.push(Pending { id, deadline: (self.clock)().saturating_add(TIMEOUT_NS), load });
    }
    // ------------------------=
    // FUNC: pump
    // DESC: Polls each queued request once without blocking the engine or desktop event loop.
    // ------------------=
    pub fn pump(&self) {
        let mut state = self.state.borrow_mut();
        let now = (self.clock)();
        let mut at = 0;
        while at < state.pending.len() {
            let id = state.pending[at].id;
            let result = if now >= state.pending[at].deadline { Err(()) } else { state.provider.poll(id) };
            if matches!(result, Ok(None)) { at += 1; continue; }
            let pending = state.pending.swap_remove(at);
            state.provider.cancel(id);
            let Ok(Some(result)) = result else {
                fail_load(&mut state, pending.load); continue;
            };
            if result.body.len() > MAX_BODY || result.headers.len() > 32 || !(200..=599).contains(&result.status) {
                fail_load(&mut state, pending.load); continue;
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
        while let Some(pending) = state.pending.pop() {
            state.provider.cancel(pending.id); reject(pending.load);
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
