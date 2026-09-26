use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use servo::{RenderingContext, Servo, SoftwareRenderingContext, WebView, WebViewBuilder};

struct PageDelegate(Rc<Cell<bool>>);
impl servo::WebViewDelegate for PageDelegate {
    // ------------------------=
    // FUNC: notify_new_frame_ready
    // DESC: Schedules painting outside the engine callback to avoid reentrant borrows.
    // ------------------=
    fn notify_new_frame_ready(&self, _view: WebView) { self.0.set(true); }
}

// ------------------------=
// FUNC: spin_until
// DESC: Pumps actual engine work with a bounded deadline and cooperative scheduler yields.
// ------------------=
fn spin_until(engine: &Servo, view: &WebView, repaint: &Cell<bool>, done: &Cell<u8>) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while done.get() == 0 && Instant::now() < deadline {
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
    let view = WebViewBuilder::new(engine, context)
        .delegate(Rc::new(PageDelegate(repaint.clone())))
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
    0
}
