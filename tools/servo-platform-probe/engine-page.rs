use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use servo::{RenderingContext, Servo, SoftwareRenderingContext, WebView, WebViewBuilder};

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
    0
}
