//! Single owner-thread engine session. The host transports commands and copies
//! dirty RGBA frames into its own surface; Servo never receives a framebuffer.
use std::{cell::{Cell, RefCell}, rc::Rc, string::String};
use servo::{RenderingContext, Servo, SoftwareRenderingContext, WebView, WebViewBuilder};
use super::resources::{Provider, Resources};
#[path="../infinity-browser-core/tab_order.rs"]
mod tab_order;

const MAX_DIMENSION: u32 = 2048;
const MAX_PIXELS: u64 = 2560 * 1600;

struct Delegate<P: Provider> {
    resources: Rc<Resources<P>>,
    dirty: Rc<Cell<bool>>,
    complete: Rc<Cell<bool>>,
    crashed: Rc<Cell<bool>>,
    ready: Rc<Cell<bool>>,
}

impl<P: Provider> servo::WebViewDelegate for Delegate<P> {
    // ------------------------=
    // FUNC: load_web_resource
    // DESC: Keeps every document and subresource behind the native provider.
    // ------------------=
    fn load_web_resource(&self, _: WebView, load: servo::WebResourceLoad) { self.resources.submit(load); }
    // ------------------------=
    // FUNC: notify_new_frame_ready
    // DESC: Coalesces paint notifications without painting reentrantly.
    // ------------------=
    fn notify_new_frame_ready(&self, _: WebView) { self.dirty.set(true); }
    // ------------------------=
    // FUNC: notify_load_status_changed
    // DESC: Exposes actual load completion rather than command submission.
    // ------------------=
    fn notify_load_status_changed(&self, _: WebView, status: servo::LoadStatus) {
        self.complete.set(status == servo::LoadStatus::Complete);
        if status == servo::LoadStatus::Complete { self.ready.set(true); }
    }
    // ------------------------=
    // FUNC: notify_traversal_complete
    // DESC: Finishes cached history traversal even when no new document load event is emitted.
    // ------------------=
    fn notify_traversal_complete(&self, _: WebView, _: servo::TraversalId) {
        self.complete.set(true);
    }
    // ------------------------=
    // FUNC: notify_crashed
    // DESC: Records failure without publishing untrusted engine diagnostics to UI.
    // ------------------=
    fn notify_crashed(&self, _: WebView, _: String, _: Option<String>) { self.crashed.set(true); }
}

pub struct Session<P: Provider> {
    view: WebView,
    context: Rc<SoftwareRenderingContext>,
    resources: Rc<Resources<P>>,
    dirty: Rc<Cell<bool>>,
    complete: Rc<Cell<bool>>,
    crashed: Rc<Cell<bool>>,
    size: (u32, u32),
    ready: Rc<Cell<bool>>,
    pending: RefCell<Option<servo::ServoUrl>>,
}

// ------------------------=
// FUNC: valid_size
// DESC: Bounds viewport allocations before handing dimensions to the engine.
// ------------------=
fn valid_size(width: u32, height: u32) -> bool {
    width > 0 && height > 0 && width <= MAX_DIMENSION && height <= MAX_DIMENSION
        && u64::from(width) * u64::from(height) <= MAX_PIXELS
}

impl<P: Provider + 'static> Session<P> {
    // ------------------------=
    // FUNC: find
    // DESC: Searches rendered document text, selects a real DOM range and reports an asynchronous bounded match count.
    // ------------------=
    pub fn find(&self,text:&str,index:u32,callback:impl FnOnce(u32)+'static) {
        use std::fmt::Write;
        let mut encoded=String::from("\"");
        for unit in text.encode_utf16() {let _=write!(encoded,"\\u{:04x}",unit);}
        encoded.push('"');
        let script=std::format!("{}({}, {})",include_str!("find.js"),encoded,index);
        self.view.evaluate_javascript(script,move |result|callback(match result {
            Ok(servo::JSValue::Number(n)) if n>=0.0 && n<=u32::MAX as f64=>n as u32,
            _=>u32::MAX,
        }));
    }
    // ------------------------=
    // FUNC: new
    // DESC: Opens one software-backed context with no ambient network authority.
    // ------------------=
    pub fn new(engine: &Servo, provider: P, clock: fn() -> u64, width: u32, height: u32) -> Result<Self, ()> {
        Self::create(engine, provider, clock, width, height, true)
    }
    // ------------------------=
    // FUNC: create
    // DESC: Creates an isolated view queue; grouped tabs leave the profile-level global delegate unchanged.
    // ------------------=
    fn create(engine: &Servo, provider: P, clock: fn() -> u64, width: u32, height: u32, global: bool) -> Result<Self, ()> {
        if !valid_size(width, height) { return Err(()); }
        let context = Rc::new(SoftwareRenderingContext::new((width, height).into()).map_err(|_| ())?);
        context.make_current().map_err(|_| ())?;
        let resources = Rc::new(Resources::new(provider, clock));
        if global { engine.set_delegate(resources.clone()); }
        let dirty = Rc::new(Cell::new(false));
        let complete = Rc::new(Cell::new(false));
        let crashed = Rc::new(Cell::new(false));
        let ready = Rc::new(Cell::new(false));
        let view = WebViewBuilder::new(engine, context.clone()).delegate(Rc::new(Delegate {
            resources: resources.clone(), dirty: dirty.clone(), complete: complete.clone(), crashed: crashed.clone(),
            ready: ready.clone(),
        })).build();
        view.show();
        Ok(Self { view, context, resources, dirty, complete, crashed, size: (width, height),
            ready, pending: RefCell::new(None) })
    }
    // ------------------------=
    // FUNC: navigate
    // DESC: Rejects privileged schemes and cancels superseded service requests.
    // ------------------=
    pub fn navigate(&self, address: &str) -> Result<(), ()> {
        if address.len() > 2048 { return Err(()); }
        let url = servo::ServoUrl::parse(address).map_err(|_| ())?;
        if !matches!(url.scheme(), "https" | "http") || !url.username().is_empty() || url.password().is_some() { return Err(()); }
        if self.resources.failed_document() && !self.complete.get() {
            *self.pending.borrow_mut()=Some(url);return Ok(());
        }
        self.resources.cancel_all();
        self.complete.set(false);
        if self.ready.get() { self.view.load(url.into_url()); }
        else { *self.pending.borrow_mut() = Some(url); }
        Ok(())
    }
    // ------------------------=
    // FUNC: resize
    // DESC: Changes native viewport and layout without allowing unbounded surfaces.
    // ------------------=
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), ()> {
        if !valid_size(width, height) { return Err(()); }
        if self.size != (width, height) {
            self.size = (width, height);
            self.view.resize((width, height).into());
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: input
    // DESC: Delivers viewport-local input on the engine owner thread.
    // ------------------=
    pub fn input(&self, event: servo::InputEvent) { self.view.notify_input_event(event); }
    // ------------------------=
    // FUNC: scroll
    // DESC: Delivers native scrolling through the same engine input path.
    // ------------------=
    pub fn scroll(&self, scroll: servo::Scroll, point: servo::WebViewPoint) { self.view.notify_scroll_event(scroll, point); }
    // ------------------------=
    // FUNC: reload
    // DESC: Cancels old resource operations before restarting the current load.
    // ------------------=
    pub fn reload(&self) { self.resources.cancel_all(); self.complete.set(false); self.view.reload(); }
    // ------------------------=
    // FUNC: history
    // DESC: Traverses one entry through Servo's real document history.
    // ------------------=
    pub fn history(&self, forward: bool) {
        if if forward {!self.view.can_go_forward()} else {!self.view.can_go_back()} {return;}
        self.resources.cancel_all();
        self.complete.set(false);
        if forward { self.view.go_forward(1); } else { self.view.go_back(1); }
    }
    // ------------------------=
    // FUNC: history_available
    // DESC: Exposes actual engine traversal availability for native navigation controls.
    // ------------------=
    pub fn history_available(&self)->u32 {
        self.view.can_go_back() as u32 | ((self.view.can_go_forward() as u32)<<1)
    }
    // ------------------------=
    // FUNC: pump
    // DESC: Advances services and paints only ready frames into a bounded private surface.
    // ------------------=
    pub fn pump(&self, engine: &Servo, mut frame: impl FnMut(u32, u32, &[u8])) -> Result<bool, ()> {
        engine.spin_event_loop();
        self.pump_frame(true, &mut frame)
    }
    // ------------------------=
    // FUNC: pump_frame
    // DESC: Advances a tab's requests without repainting hidden tabs or consuming their pending dirty state.
    // ------------------=
    fn pump_frame(&self, visible: bool, mut frame: impl FnMut(u32, u32, &[u8])) -> Result<bool, ()> {
        if self.ready.get() && (!self.resources.failed_document() || self.complete.get()) {
            if let Some(url) = self.pending.borrow_mut().take() {
                self.resources.cancel_all();
                self.complete.set(false);
                self.view.load(url.into_url());
            }
        }
        self.resources.pump();
        if self.crashed.get() { return Err(()); }
        if !visible || !self.dirty.replace(false) { return Ok(false); }
        self.view.paint();
        let image = self.context.read_to_image(servo::DeviceIntRect::new((0, 0).into(),
            (self.size.0 as i32, self.size.1 as i32).into())).ok_or(())?;
        frame(image.width(), image.height(), image.as_raw());
        Ok(true)
    }
    // ------------------------=
    // FUNC: complete
    // DESC: Returns engine-observed load completion for the native loading indicator.
    // ------------------=
    pub fn complete(&self) -> bool { self.complete.get() }
    // ------------------------=
    // FUNC: failed
    // DESC: Reports this document's native request failure without leaking a background tab failure into the foreground UI.
    // ------------------=
    pub fn failed(&self) -> bool { self.resources.failed_document() }
    // ------------------------=
    // FUNC: address
    // DESC: Returns the engine's actual location after redirects and history changes.
    // ------------------=
    pub fn address(&self) -> Option<String> { self.view.url().map(|url| url.into()) }
    // ------------------------=
    // FUNC: title
    // DESC: Returns document metadata for the native shell rather than engine-owned chrome.
    // ------------------=
    pub fn title(&self) -> Option<String> { self.view.page_title() }
    // ------------------------=
    // FUNC: visible
    // DESC: Changes actual engine visibility/focus and forces one fresh foreground paint after a tab switch.
    // ------------------=
    fn visible(&self, visible: bool) {
        self.view.set_focused(visible);
        if visible { self.view.show(); self.dirty.set(true); } else { self.view.hide(); }
    }
}

const TAB_LIMIT: usize = 8;
struct Tab<P: Provider> { id: u32, session: Session<P> }
/// One normal-profile engine group. Separate profiles/private sessions must not
/// share this group until Servo storage partitioning is implemented.
pub struct TabSessions<P: Provider> {
    global: Rc<Resources<P>>,
    tabs: std::vec::Vec<Tab<P>>,
    active: u32,
    next: u32,
    size: (u32, u32),
    clock: fn() -> u64,
}
impl<P: Provider + 'static> TabSessions<P> {
    // ------------------------=
    // FUNC: new
    // DESC: Installs a stable global delegate once, so creating/closing a tab cannot steal another tab's resource routing.
    // ------------------=
    pub fn new(engine: &Servo, provider: P, clock: fn() -> u64, width: u32, height: u32) -> Result<Self, ()> {
        if !valid_size(width, height) { return Err(()); }
        let global = Rc::new(Resources::new(provider, clock));
        engine.set_delegate(global.clone());
        Ok(Self { global, tabs: std::vec::Vec::with_capacity(TAB_LIMIT), active: 0, next: 1,
            size: (width, height), clock })
    }
    // ------------------------=
    // FUNC: create
    // DESC: Adds one real WebView with independent history and resource cancellation, bounded by the native tab budget.
    // ------------------=
    pub fn create(&mut self, engine: &Servo, provider: P, foreground: bool) -> Result<u32, ()> {
        if self.tabs.len() == TAB_LIMIT { return Err(()); }
        let next = self.next.checked_add(1).ok_or(())?;
        let session = Session::create(engine, provider, self.clock, self.size.0, self.size.1, false)?;
        session.visible(false);
        let id = self.next;
        self.next = next;
        let at=tab_order::adjacent_index(self.tabs.iter().map(|tab|tab.id),self.active);
        self.tabs.insert(at,Tab { id, session });
        if foreground || self.active == 0 { self.select(id)?; }
        Ok(id)
    }
    // ------------------------=
    // FUNC: select
    // DESC: Switches focus only after validating the requested identity, preserving other tabs' documents and history.
    // ------------------=
    pub fn select(&mut self, id: u32) -> Result<(), ()> {
        if !self.tabs.iter().any(|tab| tab.id == id) { return Err(()); }
        if self.active != id {
            for tab in &self.tabs { tab.session.visible(tab.id == id); }
            self.active = id;
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: active
    // DESC: Returns the real focused WebView identity; zero means no tabs remain.
    // ------------------=
    pub fn active(&self) -> u32 { self.active }
    // ------------------------=
    // FUNC: current
    // DESC: Lends only the selected page for native keyboard and navigation dispatch.
    // ------------------=
    pub fn current(&mut self) -> Option<&mut Session<P>> {
        self.tabs.iter_mut().find(|tab| tab.id == self.active).map(|tab| &mut tab.session)
    }
    // ------------------------=
    // FUNC: close
    // DESC: Cancels and drops exactly one WebView, then focuses its surviving neighbor if necessary.
    // ------------------=
    pub fn close(&mut self, id: u32) -> Result<(), ()> {
        let at = self.tabs.iter().position(|tab| tab.id == id).ok_or(())?;
        self.tabs.remove(at);
        if self.active == id {
            self.active = 0;
            if !self.tabs.is_empty() { self.select(self.tabs[at.min(self.tabs.len() - 1)].id)?; }
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: resize
    // DESC: Keeps hidden page viewports consistent without painting them on each foreground update.
    // ------------------=
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), ()> {
        if !valid_size(width, height) { return Err(()); }
        for tab in &mut self.tabs { tab.session.resize(width, height)?; }
        self.size = (width, height);
        Ok(())
    }
    // ------------------------=
    // FUNC: pump
    // DESC: Spins Servo once, services bounded tab queues, and paints only the selected page.
    // ------------------=
    pub fn pump(&self, engine: &Servo, mut frame: impl FnMut(u32, u32, &[u8])) -> Result<bool, ()> {
        engine.spin_event_loop();
        self.global.pump();
        let mut painted = false;
        for tab in &self.tabs {
            match tab.session.pump_frame(tab.id == self.active, &mut frame) {
                Ok(value) => painted |= value,
                Err(()) if tab.id == self.active => return Err(()),
                Err(()) => {},
            }
        }
        Ok(painted)
    }
}
impl<P: Provider> Drop for TabSessions<P> {
    // ------------------------=
    // FUNC: drop
    // DESC: Revokes global requests when the whole normal-profile group closes, not when an individual tab closes.
    // ------------------=
    fn drop(&mut self) { self.global.close(); }
}

impl<P: Provider> Drop for Session<P> {
    // ------------------------=
    // FUNC: drop
    // DESC: Revokes outstanding network work before the context tears down.
    // ------------------=
    fn drop(&mut self) { self.resources.close(); }
}
