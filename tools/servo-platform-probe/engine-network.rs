//! Real external HTTPS fixture: native e1000, DNS, TCP, TLS and Servo rendering.
use super::{native_https, resources, ResourceDelegate};
use std::{cell::{Cell, RefCell}, rc::Rc, time::{Duration, Instant}};
use infinity_browser_native_network::{client::{Configuration, Link}, smoltcp::time::Instant as NetworkTime};
use core::sync::atomic::{AtomicUsize, Ordering};
use servo::RenderingContext;
static HTTP_STATUS: AtomicUsize=AtomicUsize::new(0);
static BODY_BYTES: AtomicUsize=AtomicUsize::new(0);
#[path = "../../kernel/drivers/e1000.rs"]
mod e1000;

struct Factory { nic: Rc<RefCell<e1000::E1000>>, port: u16 }
struct Connection { nic: Rc<RefCell<e1000::E1000>> }
impl Link for Connection {
    // ------------------------=
    // FUNC: now
    // DESC: Supplies the native architectural monotonic timer.
    // ------------------=
    fn now(&self) -> NetworkTime { NetworkTime::from_micros((super::super::monotonic()/1000) as i64) }
    // ------------------------=
    // FUNC: allowed
    // DESC: Grants the configured resolver separately from HTTPS; installed capability enforcement is not claimed.
    // ------------------=
    fn allowed(&mut self, address: [u8;4], port: u16) -> bool {
        port == 443 || (address == [10,0,2,3] && port == 53)
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Receives actual guest NIC frames without host client networking.
    // ------------------=
    fn receive(&mut self, frame: &mut [u8;1514]) -> Option<usize> { self.nic.borrow_mut().receive(frame) }
    // ------------------------=
    // FUNC: transmit
    // DESC: Preserves the native descriptor ring's backpressure.
    // ------------------=
    fn transmit(&mut self, frame: &[u8]) -> bool { self.nic.borrow_mut().transmit(frame) }
    // ------------------------=
    // FUNC: register_waker
    // DESC: The fixture explicitly polls at bounded one-millisecond intervals.
    // ------------------=
    fn register_waker(&mut self, _waker: &core::task::Waker) {}
}
impl native_https::Factory for Factory {
    type Connection = Connection;
    // ------------------------=
    // FUNC: started_request
    // DESC: Associates pending and rejected fixture requests with their resource for diagnosis.
    // ------------------=
    fn started_request(url:&str) {super::super::diagnostic(7,&std::format!("BEGIN {url}\n"));}
    // ------------------------=
    // FUNC: failed_request
    // DESC: Retains one terminal native transport diagnostic for a failed fixture.
    // ------------------=
    fn failed_request(url: &str, error: &infinity_browser_native_network::client::Error) {
        super::super::diagnostic(7,&std::format!("{error:?}: {url}\n"));
    }
    // ------------------------=
    // FUNC: completed
    // DESC: Records actual authenticated HTTP completion independently of Servo's error-page load events.
    // ------------------=
    fn completed(status:u16,body_bytes:usize) {
        if status==200 { HTTP_STATUS.store(200,Ordering::Relaxed); }
        BODY_BYTES.fetch_add(body_bytes,Ordering::Relaxed);
    }
    // ------------------------=
    // FUNC: completed_request
    // DESC: Records resource completion diagnostics separately from DOM acceptance.
    // ------------------=
    fn completed_request(url:&str,status:u16,body_bytes:usize) {
        Self::completed(status,body_bytes);
        super::super::diagnostic(7,&std::format!("HTTP {status}, {body_bytes} bytes: {url}\n"));
    }
    // ------------------------=
    // FUNC: authorize
    // DESC: Grants only the fixture's named public test origin with genuine entropy and RTC time.
    // ------------------=
    fn authorize(&mut self, host: &str, port: u16) -> Result<(Connection, Configuration, u64, [u8;32]), ()> {
        if !(matches!(host, "www.google.com" | "www.gstatic.com" | "www.googleusercontent.com" | "en.wikipedia.org" | "upload.wikimedia.org" | "example.com")
            || matches!(host,"cdnjs.cloudflare.com"|"fonts.googleapis.com"|"fonts.gstatic.com")
            || ["coachable.online", "duckduckgo.com", "yahoo.com", "yimg.com"].iter()
                .any(|domain|host==*domain || host.strip_suffix(domain).is_some_and(|prefix|prefix.ends_with('.'))))
            || port != 443 { return Err(()); }
        let mut seed=[0;32];
        if !super::super::entropy_probe::fill(&mut seed) { return Err(()); }
        self.port=self.port.checked_add(1).ok_or(())?;
        let link=Connection {nic:self.nic.clone()};
        let config=Configuration { mac:self.nic.borrow().mac, address:[10,0,2,15],prefix:24,
            gateway:Some([10,0,2,2]),dns_server:[10,0,2,3],local_port:self.port,
            deadline:link.now()+infinity_browser_native_network::smoltcp::time::Duration::from_secs(30) };
        Ok((link,config,super::super::utc().ok_or(())?.0,seed))
    }
}

// ------------------------=
// FUNC: wait_for_callback
// DESC: Keeps native subresource requests moving while awaiting DOM or screenshot callbacks.
// ------------------=
fn wait_for_callback(engine:&servo::Servo, view:&servo::WebView, repaint:&Cell<bool>,
    resources:&resources::Resources<native_https::Https<Factory>>, done:&Cell<u8>)->bool {
    let deadline=Instant::now()+Duration::from_secs(10);
    while done.get()==0 && Instant::now()<deadline {
        engine.spin_event_loop();resources.pump();
        if repaint.replace(false) {view.paint();}
        std::thread::sleep(Duration::from_millis(1));
    }
    done.get()==1
}

// ------------------------=
// FUNC: verify
// DESC: Loads a public HTTPS document through guest TLS and asserts the real parsed DOM.
// ------------------=
pub fn verify(engine: &servo::Servo) -> bool {
    // Disposable QEMU virt fixture with highmem=off and the NIC explicitly at slot 1.
    // Production uses the firmware-discovered function; these addresses are never packaged.
    let nic=unsafe {
        let config=0x3f008000usize as *mut u32;
        if config.read_volatile()!=0x100e8086 { return false; }
        config.add(4).write_volatile(0x10000000);
        e1000::E1000::initialize_ecam(config as u64)
    };
    let Some(nic)=nic else { return false; };
    let nic=Rc::new(RefCell::new(nic));
    let resources=Rc::new(resources::Resources::new(native_https::Https::new(Factory {
        nic:nic.clone(),port:49152 }),super::super::monotonic));
    engine.set_delegate(resources.clone());
    let repaint=Rc::new(Cell::new(false));let loaded=Rc::new(Cell::new(0));
    let Ok(context)=servo::SoftwareRenderingContext::new((800,600).into()) else { return false; };
    let context=Rc::new(context);
    let view=servo::WebViewBuilder::new(engine,context.clone())
        .delegate(Rc::new(ResourceDelegate {resources:resources.clone(),repaint:repaint.clone(),loaded:loaded.clone()}))
        .url(option_env!("INFINITY_BROWSER_PROBE_URL").unwrap_or("https://www.google.com/").parse().unwrap()).build();
    let timeout=option_env!("INFINITY_BROWSER_LOAD_TIMEOUT").and_then(|value|value.parse::<u64>().ok()).unwrap_or(60).clamp(30,180);
    view.show();let deadline=Instant::now()+Duration::from_secs(timeout);
    while loaded.get()==0 && Instant::now()<deadline {
        engine.spin_event_loop();resources.pump();
        if repaint.replace(false) {view.paint();}
        std::thread::sleep(Duration::from_millis(1));
    }
    super::super::record(2,30,HTTP_STATUS.load(Ordering::Relaxed) as u64);
    super::super::record(2,31,BODY_BYTES.load(Ordering::Relaxed) as u64);
    super::super::record(2,32,loaded.get() as u64);
    let asserted=Rc::new(Cell::new(0));let assertion=asserted.clone();
    view.evaluate_javascript(option_env!("INFINITY_BROWSER_PROBE_ASSERT").unwrap_or("location.protocol==='https:' && location.hostname==='www.google.com' && Array.from(document.images).some(i=>i.complete && i.naturalWidth>100)"),move |value| {
        assertion.set(if matches!(value,Ok(servo::JSValue::Boolean(true))) {1}else{2});
    });
    let dom_matches=wait_for_callback(engine,&view,&repaint,&resources,&asserted);
    super::super::record(2,34,dom_matches as u64);
    {
    let inspected=Rc::new(Cell::new(0));let inspection=inspected.clone();
    view.evaluate_javascript("JSON.stringify({ready:document.readyState,body:document.body&&{text:document.body.innerText.slice(0,300),height:document.body.getBoundingClientRect().height,display:getComputedStyle(document.body).display,visibility:getComputedStyle(document.body).visibility,opacity:getComputedStyle(document.body).opacity},top:Array.from(document.elementsFromPoint(100,100)).slice(0,5).map(e=>({tag:e.tagName,id:e.id,cls:e.className,background:getComputedStyle(e).backgroundColor,opacity:getComputedStyle(e).opacity})),sheets:document.styleSheets.length,images:Array.from(document.images).slice(0,6).map(i=>({complete:i.complete,width:i.naturalWidth}))})",move |value| {
        if let Ok(servo::JSValue::String(value))=value {super::super::diagnostic(7,&value);}
        inspection.set(1);
    });
    wait_for_callback(engine,&view,&repaint,&resources,&inspected);
    }
    let mut passed=HTTP_STATUS.load(Ordering::Relaxed)==200 && BODY_BYTES.load(Ordering::Relaxed)>0
        && dom_matches;
    {
        view.paint();
        let image=context.read_to_image(servo::DeviceIntRect::new((0,0).into(),(800,600).into()));
        let pixels=image.is_some_and(|image| {
                // UART proof is a half-resolution thumbnail of real rendered pixels;
                // serializing full-size pixels can itself exhaust the test wall timeout.
                let mut thumbnail=std::vec::Vec::with_capacity(400*300*4);
                for y in (0..image.height()).step_by(2) {for x in (0..image.width()).step_by(2) {
                    thumbnail.extend_from_slice(&image.get_pixel(x,y).0);
                }}
                super::super::record(2,36,((400u64)<<32)|300u64);
                for (index,chunk) in thumbnail.chunks(8).enumerate() {
                    let mut bytes=[0;8];bytes[..chunk.len()].copy_from_slice(chunk);
                    super::super::record(5,index as u64,u64::from_le_bytes(bytes));
                }
                image.width()==800 && image.height()==600
                    && image.pixels().any(|pixel| pixel.0!=image.get_pixel(0,0).0)
            });
        passed=pixels && passed;
    }
    resources.close();
    let (rx,tx,drops)=nic.borrow_mut().statistics();
    super::super::record(2,17,rx);super::super::record(2,18,tx);super::super::record(2,19,drops);
    super::super::record(2,16,passed as u64);
    passed
}
