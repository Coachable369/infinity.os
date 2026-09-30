use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use servo::{RenderingContext, Servo, SoftwareRenderingContext, WebView, WebViewBuilder};
#[path = "../../sdk/infinity-browser-servo/resources.rs"]
mod resources;
#[path = "../../sdk/infinity-browser-servo/session.rs"]
mod session;
#[cfg(infinity_network_probe)]
#[path = "../../sdk/infinity-browser-servo/native_https.rs"]
mod native_https;
#[cfg(infinity_network_probe)]
#[path = "engine-network.rs"]
mod network;

struct ResourceDelegate<P: resources::Provider> {
    resources: Rc<resources::Resources<P>>,
    repaint: Rc<Cell<bool>>,
    loaded: Rc<Cell<u8>>,
}
impl<P: resources::Provider> servo::WebViewDelegate for ResourceDelegate<P> {
    // ------------------------=
    // FUNC: load_web_resource
    // DESC: Routes actual Servo document and subresource loads through the shared adapter.
    // ------------------=
    fn load_web_resource(&self, _view: WebView, load: servo::WebResourceLoad) { self.resources.submit(load); }
    // ------------------------=
    // FUNC: notify_new_frame_ready
    // DESC: Defers painting until outside the engine callback.
    // ------------------=
    fn notify_new_frame_ready(&self, _view: WebView) { self.repaint.set(true); }
    // ------------------------=
    // FUNC: notify_load_status_changed
    // DESC: Observes completion of the real intercepted document load.
    // ------------------=
    fn notify_load_status_changed(&self, _view: WebView, status: servo::LoadStatus) {
        if status == servo::LoadStatus::Complete { self.loaded.set(1); }
    }
}

struct FixtureProvider { starts: Rc<Cell<u32>>, cancels: Rc<Cell<u32>>, serial: u64 }
impl resources::Provider for FixtureProvider {
    // ------------------------=
    // FUNC: begin
    // DESC: Injects deterministic resources for adapter testing, not wire-network acceptance.
    // ------------------=
    fn begin(&mut self, url: &str) -> Result<u64, ()> {
        let kind = match url {
            "https://adapter.test/" => 1,
            "https://adapter.test/style.css" => 2,
            "https://adapter.test/script.js" => 3,
            "https://adapter.test/logo.png" => 4,
            _ => { self.starts.set(self.starts.get() | 16); return Err(()); }
        };
        self.starts.set(self.starts.get() | (1 << (kind - 1)));
        self.serial += 1;
        super::record(2,24,kind);
        Ok((self.serial << 8) | kind)
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Supplies fixture bytes to real Servo parsing, stylesheet loading and JavaScript execution.
    // ------------------=
    fn poll(&mut self, id: u64) -> Result<Option<resources::Response>, ()> {
        if id & 255 == 4 {
            return Ok(Some(resources::Response {status:200, headers:std::vec![("content-type".into(),"image/png".into())],
                body:include_bytes!("../test-fixtures/browser/google-logo.png").to_vec()}));
        }
        let (media, body) = match id & 255 {
            1 => ("text/html", "<!doctype html><link rel=stylesheet href=/style.css><script src=/script.js></script><body><img id=logo src=/logo.png><img id=embedded onload=\"let b=new Blob([Uint8Array.from(atob(this.src.split(',')[1]),c=>c.charCodeAt(0))],{type:'image/png'});let i=document.createElement('img');i.id='blobimage';i.src=URL.createObjectURL(b);document.body.appendChild(i)\" src='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+ip1sAAAAASUVORK5CYII='><img src=https://denied.test/no.png onerror=\"document.body.dataset.denied='yes'\"></body>"),
            2 => ("text/css", "html{background:rgb(12,34,56)}body{margin:0}img{display:none}"),
            3 => ("application/javascript", "window.adapterResult=6*7;"),
            _ => return Err(()),
        };
        let mut bytes=body.as_bytes().to_vec();
        if id & 255 == 2 {bytes.extend_from_slice(b"/*");bytes.resize(2_600_000,b' ');bytes.extend_from_slice(b"*/");}
        Ok(Some(resources::Response { status: 200,
            headers: std::vec![("content-type".into(), media.into())], body: bytes }))
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Counts released operations so successful loads cannot leak service handles.
    // ------------------=
    fn cancel(&mut self, _id: u64) { self.cancels.set(self.cancels.get() + 1); }
}

// ------------------------=
// FUNC: verify_resources
// DESC: Proves real Servo interception, external CSS/JS processing and fail-closed resource denial.
// ------------------=
fn verify_resources(engine: &Servo) -> bool {
    let starts = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let resources = Rc::new(resources::Resources::new(FixtureProvider {
        starts: starts.clone(), cancels: cancels.clone(), serial: 0,
    }, super::monotonic));
    engine.set_delegate(resources.clone());
    let repaint = Rc::new(Cell::new(false));
    let loaded = Rc::new(Cell::new(0));
    let Ok(context) = SoftwareRenderingContext::new((128, 128).into()) else { return false; };
    let view = WebViewBuilder::new(engine, Rc::new(context))
        .delegate(Rc::new(ResourceDelegate { resources: resources.clone(), repaint: repaint.clone(), loaded: loaded.clone() }))
        .url("https://adapter.test/".parse().unwrap()).build();
    view.show();
    let deadline = Instant::now() + Duration::from_secs(10);
    while loaded.get() == 0 && Instant::now() < deadline {
        engine.spin_event_loop(); resources.pump();
        if repaint.replace(false) { view.paint(); }
        std::thread::sleep(Duration::from_millis(1));
    }
    let blob_created=javascript_true(engine,&view,&repaint,"let b=new Blob([Uint8Array.from(atob(document.getElementById('embedded').src.split(',')[1]),c=>c.charCodeAt(0))],{type:'image/png'});let i=document.createElement('img');i.id='blobimage';i.src=URL.createObjectURL(b);document.body.appendChild(i);true");
    // Image decode and dynamically inserted blob images can complete after the
    // document load event. Wait for actual decoder output, not just that event.
    let deadline=Instant::now()+Duration::from_secs(5);
    let mut decoded=false;
    while !decoded && Instant::now()<deadline {
        engine.spin_event_loop();resources.pump();
        decoded=javascript_true(engine,&view,&repaint,"document.getElementById('logo')?.naturalWidth===272 && document.getElementById('embedded')?.naturalWidth===1 && document.getElementById('blobimage')?.naturalWidth===1");
        std::thread::sleep(Duration::from_millis(10));
    }
    let passed = loaded.get() == 1 && starts.get() == 31 && cancels.get() >= 4
        && blob_created && decoded
        && javascript_true(engine, &view, &repaint, "window.adapterResult===42 && document.body.dataset.denied==='yes'")
        && pixels_match(engine, &view, &repaint, [12,34,56,255])
        && javascript_true(engine,&view,&repaint,"document.getElementById('logo').style.display='block'; true")
        && image_pixels_match(engine,&view,&repaint);
    resources.close();
    drop(view);
    drain_close(engine);
    passed
}

// ------------------------=
// FUNC: verify_session
// DESC: Exercises the production session with real resource loading, resize and dirty RGBA delivery.
// ------------------=
fn verify_session(engine: &Servo) -> bool {
    let starts = Rc::new(Cell::new(0));
    let cancels = Rc::new(Cell::new(0));
    let Ok(mut session) = session::Session::new(engine, FixtureProvider {
        starts: starts.clone(), cancels: cancels.clone(), serial: 0,
    }, super::monotonic, 128, 128) else { return false; };
    if session.navigate("file:///secret").is_ok() || session.resize(0, 128).is_ok()
        || session.resize(2560, 2560).is_ok() || session.navigate("https://adapter.test/").is_err() { return false; }
    let mut painted = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !(painted && session.complete()) {
        if session.pump(engine, |w,h,bytes| {
            painted = w == 128 && h == 128 && bytes.len() == 128*128*4
                && bytes.chunks_exact(4).all(|pixel| pixel == [12,34,56,255]);
        }).is_err() { return false; }
        std::thread::sleep(Duration::from_millis(1));
    }
    super::record(2, 21, (u64::from(painted) << 32) | u64::from(starts.get()));
    if !painted || starts.get() != 31 || session.resize(160, 96).is_err() { return false; }
    painted = false;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && !painted {
        if session.pump(engine, |w,h,bytes| {
            painted = w == 160 && h == 96 && bytes.len() == 160*96*4
                && bytes.chunks_exact(4).all(|pixel| pixel == [12,34,56,255]);
        }).is_err() { return false; }
        std::thread::sleep(Duration::from_millis(1));
    }
    let deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < deadline {
        if session.pump(engine, |_,_,_| {}).is_err() { return false; }
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut idle_frames = 0;
    for _ in 0..50 {
        if session.pump(engine, |_,_,_| idle_frames += 1).is_err() { return false; }
        std::thread::sleep(Duration::from_millis(1));
    }
    let location_matches = session.address().as_deref() == Some("https://adapter.test/");
    drop(session);
    drain_close(engine);
    super::record(2, 22, (u64::from(painted) << 32) | u64::from(cancels.get()));
    painted && cancels.get() >= 4 && idle_frames == 0 && location_matches
}

// ------------------------=
// FUNC: drain_close
// DESC: Lets asynchronous document teardown release native worker slots before opening another context.
// ------------------=
fn drain_close(engine: &Servo) {
    let deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < deadline {
        engine.spin_event_loop();
        std::thread::sleep(Duration::from_millis(1));
    }
}

struct PageDelegate { repaint: Rc<Cell<bool>>, loaded: Rc<Cell<u8>>, input: Rc<Cell<u8>> }
impl servo::WebViewDelegate for PageDelegate {
    // ------------------------=
    // FUNC: notify_new_frame_ready
    // DESC: Schedules painting outside the engine callback to avoid reentrant borrows.
    // ------------------=
    fn notify_new_frame_ready(&self, _view: WebView) { self.repaint.set(true); }
    // ------------------------=
    // FUNC: notify_load_status_changed
    // DESC: Marks real completed navigation instead of assuming a queued load has finished.
    // ------------------=
    fn notify_load_status_changed(&self, _view: WebView, status: servo::LoadStatus) {
        if status == servo::LoadStatus::Complete { self.loaded.set(1); }
    }
    // ------------------------=
    // FUNC: notify_traversal_complete
    // DESC: Observes history completion including restored documents that do not reload.
    // ------------------=
    fn notify_traversal_complete(&self, _view: WebView, _id: servo::TraversalId) { self.loaded.set(1); }
    // ------------------------=
    // FUNC: notify_input_event_handled
    // DESC: Acknowledges both halves of an input pair before querying resulting DOM state.
    // ------------------=
    fn notify_input_event_handled(&self, _view: WebView, _id: servo::InputEventId, result: servo::InputEventResult) {
        if result.contains(servo::InputEventResult::DispatchFailed) { self.input.set(2); }
        else if self.input.get() == 3 { self.input.set(1); }
        else if self.input.get() == 0 { self.input.set(3); }
    }
}

// ------------------------=
// FUNC: javascript_true
// DESC: Checks observable DOM state returned by the real engine interpreter.
// ------------------=
fn javascript_true(engine: &Servo, view: &WebView, repaint: &Cell<bool>, script: &str) -> bool {
    let done = Rc::new(Cell::new(0));
    let result = done.clone();
    view.evaluate_javascript(script, move |value| {
        result.set(if matches!(value, Ok(servo::JSValue::Boolean(true))) { 1 } else { 2 });
    });
    spin_until(engine, view, repaint, &done)
}

// ------------------------=
// FUNC: spin_until
// DESC: Pumps actual engine work with a bounded deadline and cooperative scheduler yields.
// ------------------=
fn spin_until(engine: &Servo, view: &WebView, repaint: &Cell<bool>, done: &Cell<u8>) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while matches!(done.get(), 0 | 3) && Instant::now() < deadline {
        engine.spin_event_loop();
        if repaint.replace(false) { view.paint(); }
        std::thread::sleep(Duration::from_millis(1));
    }
    super::record(2, 12, done.get() as u64);
    done.get() == 1
}

// ------------------------=
// FUNC: pixels_match
// DESC: Checks rendered pixels, not engine logs or page source, as the painting oracle.
// ------------------=
fn pixels_match(engine: &Servo, view: &WebView, repaint: &Cell<bool>, expected: [u8; 4]) -> bool {
    let done = Rc::new(Cell::new(0));
    let result = done.clone();
    view.take_screenshot(None, move |image| {
        if let Ok(ref image) = image {
            super::record(2, 13, ((image.width() as u64) << 32) | image.height() as u64);
            if let Some(pixel) = image.pixels().next() {
                super::record(2, 14, u32::from_le_bytes(pixel.0) as u64);
            }
        } else { super::record(2, 13, 0); }
        result.set(if image.is_ok_and(|image| image.width() == 128 && image.height() == 128 &&
            image.pixels().all(|pixel| pixel.0 == expected)) { 1 } else { 2 });
    });
    spin_until(engine, view, repaint, &done)
}

// ------------------------=
// FUNC: image_pixels_match
// DESC: Proves decoded network PNG pixels reach the real software-rendered surface.
// ------------------=
fn image_pixels_match(engine:&Servo,view:&WebView,repaint:&Cell<bool>)->bool {
    let done=Rc::new(Cell::new(0));
    let result=done.clone();
    view.take_screenshot(None,move |image| {
        result.set(if image.is_ok_and(|image| image.pixels().filter(|p| {
            let [r,g,b,a]=p.0; a==255 && b>180 && b>r.saturating_add(30) && g>60
        }).count()>50) {1}else{2});
    });
    spin_until(engine,view,repaint,&done)
}

// ------------------------=
// FUNC: verify
// DESC: Loads real HTML/CSS, verifies its pixels, mutates the DOM through SpiderMonkey and verifies repaint.
// ------------------=
pub fn verify(engine: &Servo) -> u64 {
    let context = match SoftwareRenderingContext::new((128, 128).into()) {
        Ok(context) => Rc::new(context),
        Err(_) => return 1,
    };
    if context.make_current().is_err() { return 2; }
    super::record(2,11,1);
    let repaint = Rc::new(Cell::new(false));
    let loaded = Rc::new(Cell::new(0));
    let input = Rc::new(Cell::new(0));
    let view = WebViewBuilder::new(engine, context)
        .delegate(Rc::new(PageDelegate { repaint: repaint.clone(), loaded: loaded.clone(), input: input.clone() }))
        .url("data:text/html,%3Chtml%20style='background:rgb(255,0,0)'%3E%3Cbody%3E%3C/body%3E%3C/html%3E".parse().unwrap())
        .build();
    super::record(2,11,2);
    view.show();
    if !pixels_match(engine, &view, &repaint, [255, 0, 0, 255]) { return 3; }
    super::record(2,11,3);
    let done = Rc::new(Cell::new(0));
    let result = done.clone();
    view.evaluate_javascript("document.documentElement.style.background='rgb(0,0,255)'; 6*7", move |value| {
        result.set(if matches!(value, Ok(servo::JSValue::Number(value)) if value == 42.0) { 1 } else { 2 });
    });
    if !spin_until(engine, &view, &repaint, &done) { return 4; }
    super::record(2,11,4);
    if !pixels_match(engine, &view, &repaint, [0, 0, 255, 255]) { return 5; }
    if !javascript_true(engine, &view, &repaint,
        "document.body.innerHTML='<input id=entry style=\"position:absolute;left:0;top:0;width:100px;height:30px\">'; true") { return 6; }
    // Wait for layout and hit-test data through the normal screenshot barrier.
    let ready = Rc::new(Cell::new(0));
    let result = ready.clone();
    view.take_screenshot(None, move |image| { result.set(if image.is_ok() { 1 } else { 2 }); });
    if !spin_until(engine, &view, &repaint, &ready) { return 7; }
    for action in [servo::MouseButtonAction::Down, servo::MouseButtonAction::Up] {
        view.notify_input_event(servo::InputEvent::MouseButton(servo::MouseButtonEvent::new(
            action, servo::MouseButton::Primary, servo::WebViewPoint::Device((20.0, 15.0).into()))));
    }
    if !spin_until(engine, &view, &repaint, &input) { super::finish(1, 9, 12); }
    if !javascript_true(engine, &view, &repaint, "document.activeElement.id === 'entry'") { return 8; }
    input.set(0);
    for state in [servo::KeyState::Down, servo::KeyState::Up] {
        view.notify_input_event(servo::InputEvent::Keyboard(servo::KeyboardEvent::from_state_and_key(
            state, servo::Key::Character("A".into()))));
    }
    if !spin_until(engine, &view, &repaint, &input) { super::finish(1, 9, 13); }
    if !javascript_true(engine, &view, &repaint, "document.getElementById('entry').value === 'A'") { super::finish(1, 9, 9); }
    super::record(2, 11, 5);
    let first_url = view.url();
    loaded.set(0);
    view.load("data:text/html,%3Chtml%20style='background:rgb(0,255,0)'%3E%3Cbody%3E%3C/body%3E%3C/html%3E".parse().unwrap());
    if !spin_until(engine, &view, &repaint, &loaded) ||
        !pixels_match(engine, &view, &repaint, [0, 255, 0, 255]) { return 10; }
    loaded.set(0);
    view.reload();
    if !spin_until(engine, &view, &repaint, &loaded) ||
        !pixels_match(engine, &view, &repaint, [0, 255, 0, 255]) { return 11; }
    super::record(2, 11, 6);
    loaded.set(0);
    view.go_back(1);
    if !spin_until(engine, &view, &repaint, &loaded) || view.url() != first_url { super::finish(1, 9, 14); }
    loaded.set(0);
    view.go_forward(1);
    if !spin_until(engine, &view, &repaint, &loaded) ||
        !pixels_match(engine, &view, &repaint, [0, 255, 0, 255]) { super::finish(1, 9, 15); }
    super::record(2, 11, 7);
    if !javascript_true(engine, &view, &repaint,
        "document.documentElement.style.cssText='background:red;scrollbar-width:none'; document.body.style.cssText='margin:0;height:512px'; document.body.innerHTML='<div style=\"position:absolute;top:256px;left:0;width:100%;height:256px;background:blue\"></div>'; true") { super::finish(1, 9, 16); }
    if !pixels_match(engine, &view, &repaint, [255, 0, 0, 255]) { super::finish(1, 9, 17); }
    view.notify_scroll_event(servo::Scroll::End, servo::WebViewPoint::Device((64.0, 64.0).into()));
    if !pixels_match(engine, &view, &repaint, [0, 0, 255, 255]) { super::finish(1, 9, 18); }
    super::record(2, 11, 8);
    drop(view);
    drain_close(engine);
    if !verify_resources(engine) { return 19; }
    if !verify_session(engine) { return 21; }
    super::record(2, 11, 9);
    #[cfg(infinity_network_probe)]
    if !network::verify(engine) { return 20; }
    0
}
