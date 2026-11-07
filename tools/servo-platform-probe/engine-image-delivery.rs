//! Paints a multi-megabyte raster via markup, script and a CSS background.
use super::*;

struct Provider { part:u32, document:bool, lazy:bool, starts:Rc<Cell<u32>> }
impl resources::Provider for Provider {
    // ------------------------=
    // FUNC: begin
    // DESC: Serves an HTML document and its three independent image URLs.
    // ------------------=
    fn begin(&mut self,url:&str)->Result<u64,()> {
        self.document=url.ends_with("/index") || url.ends_with("/lazy");self.lazy=url.ends_with("/lazy");self.part=0;
        if !self.document {self.starts.set(self.starts.get()+1);}
        Ok(1)
    }
    // ------------------------=
    // FUNC: poll_stream
    // DESC: Streams a 1024-square RGB bitmap in bounded chunks through the browser resource adapter.
    // ------------------=
    fn poll_stream(&mut self,_:u64)->Result<Option<resources::Event>,()> {
        use resources::Event;
        let step=self.part;self.part+=1;
        let event=match (self.document,step) {
            (doc,0)=>Event::Head(200,std::vec![("Content-Type".into(),if doc {"text/html"} else {"image/bmp"}.into())]),
            (true,1) if self.lazy=>Event::Data(br#"<!doctype html><style>body{margin:0;height:10000px}img{position:absolute;width:64px;height:64px}</style><img id="first" loading="LAZY" style="top:3500px" src="/first.bmp"><img id="second" loading="lazy" style="top:8000px" srcset="/second.bmp 1x">"#.to_vec()),
            (true,1)=>Event::Data(br#"<!doctype html><style>*{margin:0}img,div{position:absolute;top:0;width:64px;height:64px}div{left:128px;background:url(/background.bmp) center/cover}</style><img src="/markup.bmp"><img id="dynamic" style="left:64px"><div></div><script>dynamic.src='/script.bmp'</script>"#.to_vec()),
            (false,1)=>{
                let mut b=std::vec![0u8;54];b[..2].copy_from_slice(b"BM");
                for (pos,v) in [(2,54+3*1024*1024u32),(10,54),(14,40),(18,1024),(22,1024)]{b[pos..pos+4].copy_from_slice(&v.to_le_bytes());}
                b[26]=1;b[28]=24;Event::Data(b)
            },
            (false,2..=65)=>Event::Data([30,210,15].repeat(16384)),
            _=>Event::Done,
        };Ok(Some(event))
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: The deterministic provider does not own a socket or file handle.
    // ------------------=
    fn cancel(&mut self,_:u64){}
}
// ------------------------=
// FUNC: verify
// DESC: Requires the decoded image color at three independent painted locations and a completed navigation.
// ------------------=
pub fn verify(engine:&Servo)->bool {
    let Ok(session)=session::Session::new(engine,Provider{part:0,document:false,lazy:false,starts:Rc::new(Cell::new(0))},crate::monotonic,240,160) else{return false;};
    if session.navigate("https://image.test/index").is_err(){return false;}
    let deadline=Instant::now()+Duration::from_secs(30);let mut painted=false;
    while Instant::now()<deadline {
        if session.pump(engine,|width,_,pixels| {
            painted=[32,96,160].iter().all(|x|{
                let i=((32*width+*x)*4) as usize;pixels.get(i..i+3)==Some(&[15,210,30])
            });
        }).is_err(){return false;}
        if painted && session.complete(){return verify_lazy(engine);}
        std::thread::sleep(Duration::from_millis(1));
    }
    false
}

// ------------------------=
// FUNC: verify_lazy
// DESC: Requires zero offscreen transfers at document completion, one after scroll and one after switching to eager.
// ------------------=
fn verify_lazy(engine:&Servo)->bool {
    let starts=Rc::new(Cell::new(0));let repaint=Rc::new(Cell::new(false));let loaded=Rc::new(Cell::new(0));
    let resources=Rc::new(resources::Resources::new(Provider{part:0,document:false,lazy:false,starts:starts.clone()},crate::monotonic));
    let Ok(context)=SoftwareRenderingContext::new((240,160).into()) else{return false;};
    let view=WebViewBuilder::new(engine,Rc::new(context)).delegate(Rc::new(ResourceDelegate{resources:resources.clone(),repaint:repaint.clone(),loaded}))
        .url("https://image.test/lazy".parse().unwrap()).build();
    view.show();
    for (action,condition,count) in [
        ("true","document.readyState==='complete' && document.getElementById('first')?.loading==='lazy' && !first.complete && !second.complete",0),
        ("window.scrollTo(0,3450);true","first.complete && first.naturalWidth===1024 && !second.complete",1),
        ("second.loading='eager';true","second.complete && second.naturalWidth===1024",2)
    ] {
        if count!=0 && !javascript_true(engine,&view,&repaint,action){return false;}
        let deadline=Instant::now()+Duration::from_secs(20);
        loop {
            engine.spin_event_loop(); resources.pump();
            if repaint.replace(false){view.paint();}
            if javascript_true(engine,&view,&repaint,condition) && starts.get()==count {break;}
            if Instant::now()>=deadline {crate::record(2,80,((count as u64)<<32)|starts.get() as u64);return false;}
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    drop(view);drain_close(engine);true
}
