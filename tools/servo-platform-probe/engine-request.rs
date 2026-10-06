use super::{resources, ResourceDelegate, Servo};
use std::{cell::Cell, rc::Rc, time::{Duration, Instant}};

struct Provider { observed: Rc<Cell<u32>> }
impl resources::Provider for Provider {
    // ------------------------=
    // FUNC: begin
    // DESC: Rejects requests that bypass typed method and body delivery.
    // ------------------=
    fn begin(&mut self,_:&str)->Result<u64,()> {Err(())}
    // ------------------------=
    // FUNC: begin_request
    // DESC: Checks actual script and form payloads, preserved 307 bodies, and rewritten 303 methods.
    // ------------------=
    fn begin_request(&mut self,url:&str,method:&str,_:&[u8],body:&[u8])->Result<u64,()> {
        let binary=&[0,13,10,255,128,65];
        let (id,valid)=match url {
            "https://request.test/"=>(1,method=="GET" && body.is_empty()),
            "https://request.test/preserve"=>(2,method=="POST" && body==binary),
            "https://request.test/rewrite"=>(3,method=="POST" && body==binary),
            "https://request.test/result"=>(4,method=="GET" && body.is_empty()),
            "https://request.test/form"=>(5,method=="POST" && body==b"q=native+search"),
            _=>return Err(()),
        };
        if !valid {self.observed.set(self.observed.get()|0x8000);return Err(());}
        self.observed.set(self.observed.get()|(1<<(id-1)));Ok(id)
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Drives redirects and real DOM form submission without external service dependencies.
    // ------------------=
    fn poll(&mut self,id:u64)->Result<Option<resources::Response>,()> {
        let mut headers=std::vec![("content-type".into(),"text/html".into())];
        let status=match id {
            2=>{headers.push(("location".into(),"/rewrite".into()));307},
            3=>{headers.push(("location".into(),"/result".into()));303},
            _=>200,
        };
        let body=if id==1 {b"<!doctype html><form method=post action='/form'><input name=q value='native search'></form><script>fetch('/preserve',{method:'POST',body:new Uint8Array([0,13,10,255,128,65])}).then(r=>{if(r.ok) document.forms[0].submit()})</script>".to_vec()}else{b"<!doctype html><body>Done</body>".to_vec()};
        Ok(Some(resources::Response{status,headers,body}))
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Releases the completed in-memory fixture transaction.
    // ------------------=
    fn cancel(&mut self,_:u64) {}
}

// ------------------------=
// FUNC: verify
// DESC: Requires all method/body transitions through Servo and the production resource adapter.
// ------------------=
pub fn verify(engine:&Servo)->bool {
    let observed=Rc::new(Cell::new(0));
    let resources=Rc::new(resources::Resources::new(Provider{observed:observed.clone()},super::super::monotonic));
    engine.set_delegate(resources.clone());
    let repaint=Rc::new(Cell::new(false));let loaded=Rc::new(Cell::new(0));
    let Ok(context)=servo::SoftwareRenderingContext::new((128,128).into()) else{return false;};
    let view=servo::WebViewBuilder::new(engine,Rc::new(context))
        .delegate(Rc::new(ResourceDelegate{resources:resources.clone(),repaint:repaint.clone(),loaded}))
        .url("https://request.test/".parse().unwrap()).build();
    view.show();let deadline=Instant::now()+Duration::from_secs(15);
    while observed.get()!=31 && observed.get()&0x8000==0 && Instant::now()<deadline {
        engine.spin_event_loop();resources.pump();
        if repaint.replace(false) {view.paint();}
        std::thread::sleep(Duration::from_millis(1));
    }
    let passed=observed.get()==31;
    super::super::record(2,46,observed.get() as u64);
    resources.close();drop(view);super::drain_close(engine);passed
}
