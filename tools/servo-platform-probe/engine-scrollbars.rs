use super::*;

// ------------------------=
// FUNC: wait_state
// DESC: Waits for actual asynchronous DOM scroll state, not engine log text.
// ------------------=
fn wait_state(engine: &Servo, view: &WebView, repaint: &Cell<bool>, script: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if javascript_true(engine, view, repaint, script) { return true; }
        if Instant::now() >= deadline { return false; }
        std::thread::sleep(Duration::from_millis(5));
    }
}
// ------------------------=
// FUNC: pointer
// DESC: Delivers real native pointer events to Servo without invoking page handlers synthetically.
// ------------------=
fn pointer(view: &WebView, action: Option<servo::MouseButtonAction>, x: f32, y: f32) {
    let point = servo::WebViewPoint::Device((x,y).into());
    view.notify_input_event(match action {
        Some(action) => servo::InputEvent::MouseButton(servo::MouseButtonEvent::new(action, servo::MouseButton::Primary, point)),
        None => servo::InputEvent::MouseMove(servo::MouseMoveEvent::new(point)),
    });
}
// ------------------------=
// FUNC: pixels
// DESC: Verifies real content and persistent rail pixels; optionally exports the guest frame for visual inspection.
// ------------------=
fn pixels(engine: &Servo, view: &WebView, repaint: &Cell<bool>, samples: &[(u32,u32,[u8;4])], export: bool) -> bool {
    let done = Rc::new(Cell::new(0)); let result = done.clone(); let samples = samples.to_vec();
    view.take_screenshot(None, move |image| {
        let Ok(image) = image else { result.set(2); return; };
        let valid = samples.iter().all(|(x,y,color)| image.get_pixel(*x,*y).0 == *color);
        if export {
            super::super::record(2,36,((image.width() as u64)<<32)|image.height() as u64);
            for (index,chunk) in image.as_raw().chunks(8).enumerate() {
                let mut bytes=[0;8]; bytes[..chunk.len()].copy_from_slice(chunk);
                super::super::record(5,index as u64,u64::from_le_bytes(bytes));
            }
        }
        result.set(if valid {1} else {2});
    });
    spin_until(engine, view, repaint, &done)
}
// ------------------------=
// FUNC: verify
// DESC: Exercises production root rails, retained zero offsets, script/wheel updates and unaffected nested-container input.
// ------------------=
pub fn verify(engine: &Servo) -> bool {
    let Ok(context) = SoftwareRenderingContext::new((320,240).into()) else { return false; };
    if context.make_current().is_err() { return false; }
    let repaint=Rc::new(Cell::new(false)); let loaded=Rc::new(Cell::new(0));
    let view=WebViewBuilder::new(engine,Rc::new(context)).delegate(Rc::new(PageDelegate {
        repaint:repaint.clone(), loaded:loaded.clone(), input:Rc::new(Cell::new(0)),
    })).url("data:text/html,<!doctype html><body></body>".parse().unwrap()).build();
    view.show(); view.set_focused(true);
    if !spin_until(engine,&view,&repaint,&loaded) { return false; }
    if !javascript_true(engine,&view,&repaint,r#"
        document.body.style.cssText='margin:0;width:640px;height:960px;background:rgb(0,0,255)';
        document.body.innerHTML='<div style="position:absolute;left:0;top:0;width:640px;height:240px;background:red"></div><div id="nested" style="position:absolute;left:40px;top:40px;width:100px;height:100px;overflow:auto;background:white"><div style="height:400px"><button id="button">Click</button></div></div>';
        window.presses=0;window.clicked=0;document.addEventListener('mousedown',()=>window.presses++);
        button.onclick=()=>window.clicked++;scrollTo(0,0);true
    "#) { return false; }
    if !pixels(engine,&view,&repaint,&[(5,5,[255,0,0,255]),(313,10,[108,137,174,255]),(307,100,[16,27,58,255])],true) { return false; }
    super::super::record(2,61,1);
    pointer(&view,Some(servo::MouseButtonAction::Down),313.0,8.0);
    pointer(&view,None,313.0,235.0);
    if !wait_state(engine,&view,&repaint,"scrollY===720 && window.presses===0") ||
        !pixels(engine,&view,&repaint,&[(5,5,[0,0,255,255]),(313,210,[62,111,255,255])],false) { return false; }
    pointer(&view,None,313.0,8.0);
    if !wait_state(engine,&view,&repaint,"scrollY===0 && window.presses===0") ||
        !pixels(engine,&view,&repaint,&[(5,5,[255,0,0,255]),(313,10,[62,111,255,255])],false) { return false; }
    pointer(&view,Some(servo::MouseButtonAction::Up),313.0,8.0);
    pointer(&view,Some(servo::MouseButtonAction::Down),8.0,233.0);
    pointer(&view,None,305.0,233.0);
    if !wait_state(engine,&view,&repaint,"scrollX===320 && scrollY===0 && window.presses===0") { return false; }
    pointer(&view,None,8.0,233.0);
    if !wait_state(engine,&view,&repaint,"scrollX===0 && window.presses===0") { return false; }
    pointer(&view,Some(servo::MouseButtonAction::Up),8.0,233.0);
    super::super::record(2,61,2);
    pointer(&view,Some(servo::MouseButtonAction::Down),313.0,180.0);
    pointer(&view,Some(servo::MouseButtonAction::Up),313.0,180.0);
    if !wait_state(engine,&view,&repaint,"scrollY===216 && window.presses===0") { return false; }
    if !javascript_true(engine,&view,&repaint,"scrollTo(320,480);true") ||
        !wait_state(engine,&view,&repaint,"scrollX===320 && scrollY===480") ||
        !pixels(engine,&view,&repaint,&[(313,120,[134,199,255,255])],false) { return false; }
    if !javascript_true(engine,&view,&repaint,"scrollTo(0,0);true") ||
        !pixels(engine,&view,&repaint,&[(5,5,[255,0,0,255])],false) { return false; }
    super::super::record(2,61,3);
    view.notify_scroll_event(servo::Scroll::Delta(servo::WebViewVector::Device((0.0,60.0).into())),servo::WebViewPoint::Device((80.0,100.0).into()));
    if !wait_state(engine,&view,&repaint,"nested.scrollTop===60 && scrollY===0") { return false; }
    if !javascript_true(engine,&view,&repaint,"nested.scrollTop=0;true") || !pixels(engine,&view,&repaint,&[],false) { return false; }
    pointer(&view,Some(servo::MouseButtonAction::Down),60.0,50.0);
    pointer(&view,Some(servo::MouseButtonAction::Up),60.0,50.0);
    if !wait_state(engine,&view,&repaint,"window.clicked===1 && window.presses===1") { return false; }
    view.notify_scroll_event(servo::Scroll::Delta(servo::WebViewVector::Device((0.0,60.0).into())),servo::WebViewPoint::Device((250.0,100.0).into()));
    if !wait_state(engine,&view,&repaint,"scrollY===60 && nested.scrollTop===0") { return false; }
    super::super::record(2,61,4);
    if !javascript_true(engine,&view,&repaint,"document.documentElement.style.scrollbarWidth='none';true") ||
        !pixels(engine,&view,&repaint,&[(313,10,[255,0,0,255])],false) { return false; }
    if !javascript_true(engine,&view,&repaint,"document.documentElement.style.scrollbarWidth='thin';scrollTo(0,0);true") ||
        !pixels(engine,&view,&repaint,&[(313,100,[16,27,58,255]),(316,10,[108,137,174,255])],false) { return false; }
    if !javascript_true(engine,&view,&repaint,"document.documentElement.style.scrollbarWidth='auto';true") { return false; }
    view.set_page_zoom(2.0);
    if !wait_state(engine,&view,&repaint,"innerWidth===160") || !pixels(engine,&view,&repaint,&[(307,180,[16,27,58,255])],false) { return false; }
    view.set_page_zoom(1.0);view.resize((400,300).into());
    if !wait_state(engine,&view,&repaint,"innerWidth===400 && innerHeight===300") || !pixels(engine,&view,&repaint,&[(387,180,[16,27,58,255])],false) { return false; }
    if !javascript_true(engine,&view,&repaint,"document.body.innerHTML='';document.documentElement.style.overflow='hidden';document.body.style.cssText='margin:0;width:640px;height:960px;background:rgb(0,255,0)';true") ||
        !pixels(engine,&view,&repaint,&[(393,10,[0,255,0,255]),(10,293,[0,255,0,255])],false) { return false; }
    if !javascript_true(engine,&view,&repaint,"document.documentElement.style.overflow='';true") { return false; }
    if !javascript_true(engine,&view,&repaint,"document.body.innerHTML='';document.body.style.cssText='margin:0;background:rgb(0,255,0)';true") ||
        !pixels(engine,&view,&repaint,&[(393,10,[0,255,0,255]),(10,293,[0,255,0,255])],false) { return false; }
    super::super::record(2,61,5);
    drop(view);drain_close(engine);true
}
