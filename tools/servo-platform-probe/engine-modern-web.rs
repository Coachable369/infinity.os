use super::*;

// ------------------------=
// FUNC: verify_parking
// DESC: Forces actual parking_lot contention, handoff and timeout while the native executor must run the lock owner.
// ------------------=
fn verify_parking()->bool {
    use std::sync::Arc;
    let value=Arc::new(parking_lot::Mutex::new(0u32));
    let guard=value.lock();
    let worker_value=value.clone();
    let worker=std::thread::spawn(move || {let mut value=worker_value.lock();*value=42;});
    std::thread::yield_now();
    drop(guard);
    if worker.join().is_err() || *value.lock()!=42 {return false;}
    let guard=value.lock();
    let worker_value=value.clone();
    let worker=std::thread::spawn(move || worker_value.try_lock_for(Duration::from_millis(5)).is_none());
    let timed_out=worker.join().unwrap_or(false);
    drop(guard);
    super::super::record(2,63,u64::from(timed_out));
    timed_out
}

// ------------------------=
// FUNC: verify
// DESC: Exercises local FontFace loading, variable glyph metrics and observer-triggered image decoding in native Servo.
// ------------------=
pub fn verify(engine: &Servo) -> bool {
    if !verify_parking() {return false;}
    let Ok(context)=SoftwareRenderingContext::new((640,480).into()) else {return false;};
    let repaint=Rc::new(Cell::new(false)); let loaded=Rc::new(Cell::new(0));
    let view=WebViewBuilder::new(engine,Rc::new(context)).delegate(Rc::new(PageDelegate {
        repaint:repaint.clone(),loaded:loaded.clone(),input:Rc::new(Cell::new(0)),
    })).url("data:text/html,<!doctype html><body></body>".parse().unwrap()).build();
    view.show();view.set_focused(true);
    if !spin_until(engine,&view,&repaint,&loaded) {return false;}
    if !javascript_true(engine,&view,&repaint,r#"
        document.body.innerHTML='<img id="lazy" width="40" height="40">';
        window.seen=false;window.fontLoaded=false;window.variableLoaded=false;
        const face=new FontFace('NativeProbe','local("Fira Sans")');
        document.fonts.add(face);face.load().then(()=>{window.fontLoaded=face.status==='loaded'});
        const variable=new FontFace('VariableProbe','local("EB Garamond")',{weight:'400 800'});
        document.fonts.add(variable);variable.load().then(()=>{
            window.widths=[400,800].map(weight=>{
                const span=document.createElement('span');span.textContent='Modern typography';
                span.style.cssText='display:inline-block;white-space:nowrap;font:30px VariableProbe;font-variation-settings:"wght" '+weight;
                document.body.appendChild(span);return span.getBoundingClientRect().width;
            });window.variableLoaded=Math.abs(widths[0]-widths[1])>0.1;
        });
        window.observer=new IntersectionObserver(entries=>{
            if(entries.some(e=>e.isIntersecting)) {
                window.seen=true;
                lazy.src='data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect width="40" height="40" fill="blue"/></svg>';
                observer.disconnect();
            }
        });observer.observe(lazy);true
    "#) {return false;}
    let deadline=Instant::now()+Duration::from_secs(5);
    let mut ready=false;
    while Instant::now()<deadline {
        if javascript_true(engine,&view,&repaint,"seen && fontLoaded && variableLoaded && lazy.complete && lazy.naturalWidth===40") {ready=true;break;}
        std::thread::sleep(Duration::from_millis(5));
    }
    if !ready {
        let done=Rc::new(Cell::new(0));let result=done.clone();
        view.evaluate_javascript("JSON.stringify({seen,fontLoaded,variableLoaded,widths:window.widths,image:[lazy.complete,lazy.naturalWidth]})",move |value| {
            if let Ok(servo::JSValue::String(value))=value {super::super::diagnostic(7,&value);}
            result.set(1);
        });
        spin_until(engine,&view,&repaint,&done);
        return false;
    }
    super::super::record(2,62,1);
    drop(view);
    drain_close(engine);
    true
}
