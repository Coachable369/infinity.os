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
    if !verify_parking() || !verify_cache(engine) {return false;}
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
    let woff=include_bytes!("../../build/servo-port-audit/tests/wpt/tests/fonts/hasubi-mono/HasubiMono-Regular.woff2");
    let bytes=woff.iter().map(|b|std::format!("{b}")).collect::<Vec<_>>().join(",");
    let script=std::format!("window.woffLoaded=false;window.woffError='';const remote=new FontFace('WoffProbe',new Uint8Array([{bytes}]));document.fonts.add(remote);remote.load().then(()=>{{window.woffLoaded=remote.status==='loaded'}}).catch(e=>{{window.woffError=String(e)}});true");
    if !javascript_true(engine,&view,&repaint,&script) {return false;}
    let deadline=Instant::now()+Duration::from_secs(5);
    let mut ready=false;
    while Instant::now()<deadline {
        if javascript_true(engine,&view,&repaint,"seen && fontLoaded && variableLoaded && woffLoaded && lazy.complete && lazy.naturalWidth===40") {ready=true;break;}
        std::thread::sleep(Duration::from_millis(5));
    }
    if !ready {
        let done=Rc::new(Cell::new(0));let result=done.clone();
        view.evaluate_javascript("JSON.stringify({seen,fontLoaded,variableLoaded,woffLoaded,woffError,widths:window.widths,image:[lazy.complete,lazy.naturalWidth]})",move |value| {
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

struct CacheProvider { starts:Rc<Cell<u32>>, phase:u8, font:bool }
impl resources::Provider for CacheProvider {
    // ------------------------=
    // FUNC: begin
    // DESC: Counts observable wire requests to one deterministic cacheable origin.
    // ------------------=
    fn begin(&mut self,url:&str)->Result<u64,()> {self.font=url.ends_with(".woff2");self.phase=0;self.starts.set(self.starts.get()+1);Ok(self.starts.get() as u64)}
    // ------------------------=
    // FUNC: poll
    // DESC: Returns a bounded cacheable response with explicit cross-origin policy.
    // ------------------=
    fn poll(&mut self,_id:u64)->Result<Option<resources::Response>,()> {
        Ok(Some(resources::Response {status:200,headers:std::vec![
            ("Content-Type".into(),"text/plain".into()),
            ("Cache-Control".into(),"public, max-age=3600".into()),
            ("Access-Control-Allow-Origin".into(),"*".into())],body:b"cache payload".to_vec()}))
    }
    // ------------------------=
    // FUNC: poll_stream
    // DESC: Exercises the production header/body/completion sequence rather than complete-response shortcut.
    // ------------------=
    fn poll_stream(&mut self,id:u64)->Result<Option<resources::Event>,()> {
        self.phase+=1;
        Ok(Some(match self.phase {
            1=>resources::Event::Head(200,self.poll(id)?.unwrap().headers),
            2=>resources::Event::Data(if self.font {include_bytes!("../../build/servo-port-audit/tests/wpt/tests/fonts/hasubi-mono/HasubiMono-Regular.woff2").to_vec()} else {b"cache payload".to_vec()}),
            _=>resources::Event::Done,
        }))
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Owns no transport allocation in this deterministic fixture.
    // ------------------=
    fn cancel(&mut self,_id:u64) {}
}
// ------------------------=
// FUNC: verify_cache
// DESC: Fetches the same fresh resource twice and checks body delivery plus native request count.
// ------------------=
fn verify_cache(engine:&Servo)->bool {
    let starts=Rc::new(Cell::new(0));
    let resources=Rc::new(resources::Resources::new(CacheProvider{starts:starts.clone(),phase:0,font:false},super::super::monotonic));
    engine.set_delegate(resources.clone());
    let Ok(context)=SoftwareRenderingContext::new((320,240).into()) else {return false;};
    let repaint=Rc::new(Cell::new(false));let loaded=Rc::new(Cell::new(0));
    let view=WebViewBuilder::new(engine,Rc::new(context)).delegate(Rc::new(ResourceDelegate{resources:resources.clone(),repaint:repaint.clone(),loaded:loaded.clone()}))
        .url("data:text/html,<!doctype html><body>Cache fixture</body>".parse().unwrap()).build();
    view.show();
    if !spin_until(engine,&view,&repaint,&loaded){return false;}
    if !javascript_true(engine,&view,&repaint,"window.cacheState=0;(async()=>{try{const first=await fetch('https://cache.test/asset',{credentials:'omit'});const second=fetch('https://cache.test/asset',{credentials:'omit'});const a=await first.text();const b=await(await second).text();window.cacheState=a==='cache payload'&&b===a?1:2;}catch(e){window.cacheState=3;}})();true"){return false;}
    let deadline=Instant::now()+Duration::from_secs(5);let mut delivered=false;
    while Instant::now()<deadline {engine.spin_event_loop();resources.pump();if javascript_true(engine,&view,&repaint,"cacheState===1"){delivered=true;break;}std::thread::sleep(Duration::from_millis(1));}
    super::super::record(2,64,starts.get() as u64);
    super::super::record(2,65,u64::from(delivered));
    if !delivered || starts.get()!=1 {return false;}
    if !javascript_true(engine,&view,&repaint,r#"window.preloadFont=0;const preload=document.createElement('link');preload.rel='preload';preload.as='font';preload.crossOrigin='anonymous';preload.href='https://cache.test/font.woff2';document.head.appendChild(preload);const face=new FontFace('PreloadProbe','url(https://cache.test/font.woff2)');document.fonts.add(face);face.load().then(()=>{window.preloadFont=1}).catch(()=>{window.preloadFont=2});true"#){return false;}
    let deadline=Instant::now()+Duration::from_secs(5);let mut font_loaded=false;
    while Instant::now()<deadline {engine.spin_event_loop();resources.pump();if javascript_true(engine,&view,&repaint,"preloadFont===1"){font_loaded=true;break;}std::thread::sleep(Duration::from_millis(1));}
    super::super::record(2,66,u64::from(font_loaded));
    drop(view);drain_close(engine);
    font_loaded
}
