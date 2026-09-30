//! Real external HTTPS fixture: native e1000, DNS, TCP, TLS and Servo rendering.
use super::{native_https, resources, ResourceDelegate};
use std::{cell::{Cell, RefCell}, rc::Rc, time::{Duration, Instant}};
use infinity_browser_native_network::{client::{Configuration, Link}, smoltcp::time::Instant as NetworkTime};
use core::sync::atomic::{AtomicUsize, Ordering};
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
    // FUNC: failed
    // DESC: Retains one terminal native transport diagnostic for a failed fixture.
    // ------------------=
    fn failed(error: &infinity_browser_native_network::client::Error) {
        super::super::diagnostic(7,&std::format!("{error:?}"));
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
    // FUNC: authorize
    // DESC: Grants only the fixture's named public test origin with genuine entropy and RTC time.
    // ------------------=
    fn authorize(&mut self, host: &str, port: u16) -> Result<(Connection, Configuration, u64, [u8;32]), ()> {
        if host != "www.google.com" || port != 443 { return Err(()); }
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
    let view=servo::WebViewBuilder::new(engine,Rc::new(context))
        .delegate(Rc::new(ResourceDelegate {resources:resources.clone(),repaint:repaint.clone(),loaded:loaded.clone()}))
        .url("https://www.google.com/".parse().unwrap()).build();
    view.show();let deadline=Instant::now()+Duration::from_secs(35);
    while loaded.get()==0 && Instant::now()<deadline {
        engine.spin_event_loop();resources.pump();
        if repaint.replace(false) {view.paint();}
        std::thread::sleep(Duration::from_millis(1));
    }
    super::super::record(2,30,HTTP_STATUS.load(Ordering::Relaxed) as u64);
    super::super::record(2,31,BODY_BYTES.load(Ordering::Relaxed) as u64);
    super::super::record(2,32,loaded.get() as u64);
    let mut passed=HTTP_STATUS.load(Ordering::Relaxed)==200 && BODY_BYTES.load(Ordering::Relaxed)>0
        && loaded.get()==1 && super::javascript_true(engine,&view,&repaint,
        "location.protocol==='https:' && location.hostname==='www.google.com' && Array.from(document.images).some(i=>i.complete && i.naturalWidth>100)");
    if passed {
        let pixels=Rc::new(Cell::new(0));let result=pixels.clone();
        view.take_screenshot(None,move |image| {
            result.set(if image.is_ok_and(|image| image.width()==800 && image.height()==600
                && image.pixels().any(|pixel| pixel.0!=image.get_pixel(0,0).0)) {1} else {2});
        });
        passed=super::spin_until(engine,&view,&repaint,&pixels);
    }
    resources.close();
    let (rx,tx,drops)=nic.borrow_mut().statistics();
    super::super::record(2,17,rx);super::super::record(2,18,tx);super::super::record(2,19,drops);
    super::super::record(2,16,passed as u64);
    passed
}
