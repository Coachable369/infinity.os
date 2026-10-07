use super::{resources,session,Servo};
use std::{cell::Cell,rc::Rc,time::{Duration,Instant}};
use core::sync::atomic::{AtomicU64,Ordering};
static OFFSET:AtomicU64=AtomicU64::new(0);

// ------------------------=
// FUNC: clock
// DESC: Advances only the adapter deadline for deterministic timeout recovery verification.
// ------------------=
fn clock()->u64 {super::super::monotonic()+OFFSET.load(Ordering::Relaxed)}

struct Provider {active:Option<(u64,u8,u8)>,serial:u64,starts:Rc<Cell<u32>>,cancels:Rc<Cell<u32>>,stage:Rc<Cell<u8>>}
impl resources::Provider for Provider {
    // ------------------------=
    // FUNC: begin
    // DESC: Rejects overlapping leases and selects a controlled failure or successful document.
    // ------------------=
    fn begin(&mut self,url:&str)->Result<u64,()> {
        if self.active.is_some() {return Err(());}
        let kind=if url.starts_with("https://recovery.test/good") {0}
            else if url=="https://recovery.test/before" {1}
            else if url=="https://recovery.test/partial" {2}
            else if url=="https://recovery.test/timeout" {3}
            else {return Err(());};
        self.serial+=1;self.active=Some((self.serial,kind,0));
        self.starts.set(self.starts.get()+1);self.stage.set(0);Ok(self.serial)
    }
    // ------------------------=
    // FUNC: poll_stream
    // DESC: Exercises header failure, authenticated partial-body failure and a stalled response without external servers.
    // ------------------=
    fn poll_stream(&mut self,id:u64)->Result<Option<resources::Event>,()> {
        let Some((current,kind,stage))=self.active.as_mut() else {return Err(());};
        if *current!=id || *kind==1 {return Err(());}
        let event=match *stage {
            0=>resources::Event::Head(200,std::vec![("content-type".into(),"text/html".into()),("cache-control".into(),"no-store".into())]),
            1=>{
                let mut body=if *kind==0 {b"<!doctype html><body style='margin:0;background:rgb(12,180,40)'>".to_vec()}
                    else {b"<!doctype html><body style='margin:0;background:rgb(180,12,40)'>".to_vec()};
                body.resize(2048,b' ');resources::Event::Data(body)
            },
            _ if *kind==2=>return Err(()),
            _ if *kind==3=>return Ok(None),
            2=>resources::Event::Done,
            _=>return Err(()),
        };
        *stage+=1;self.stage.set(*stage);Ok(Some(event))
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Requires exact lease release so leaked failed requests prevent the next document from starting.
    // ------------------=
    fn cancel(&mut self,id:u64) {
        if self.active.is_some_and(|(current,_,_)|current==id) {
            self.active=None;self.cancels.set(self.cancels.get()+1);
        }
    }
}

// ------------------------=
// FUNC: loaded
// DESC: Requires real successful-page pixels, matching location and engine completion after recovery.
// ------------------=
fn loaded(engine:&Servo,session:&session::Session<Provider>,url:&str)->bool {
    let deadline=Instant::now()+Duration::from_secs(10);let mut painted=false;
    while Instant::now()<deadline {
        if session.pump(engine,|_,_,bytes| {
            painted=bytes.chunks_exact(4).filter(|p|p[1]>150 && p[0]<60 && p[2]<80).count()>8000;
        }).is_err() {return false;}
        if painted && session.complete() && !session.failed() && session.address().as_deref()==Some(url) {return true;}
        std::thread::sleep(Duration::from_millis(1));
    }
    false
}

// ------------------------=
// FUNC: verify
// DESC: Reuses one production tab across successful loads, three failure stages and immediate replacement navigations.
// ------------------=
pub fn verify(engine:&Servo)->bool {
    let starts=Rc::new(Cell::new(0));let cancels=Rc::new(Cell::new(0));let stage=Rc::new(Cell::new(0));
    let Ok(session)=session::Session::new(engine,Provider{active:None,serial:0,starts:starts.clone(),cancels:cancels.clone(),stage:stage.clone()},clock,128,128) else{return false;};
    if session.navigate("https://recovery.test/good0").is_err() || !loaded(engine,&session,"https://recovery.test/good0") {return false;}
    for (index,path) in ["before","partial","timeout"].iter().enumerate() {
        if session.navigate(&std::format!("https://recovery.test/{path}")).is_err() {return false;}
        let deadline=Instant::now()+Duration::from_secs(10);let mut advanced=false;
        while !session.failed() && Instant::now()<deadline {
            if session.pump(engine,|_,_,_|{}).is_err() {return false;}
            if *path=="timeout" && stage.get()==2 && !advanced {OFFSET.fetch_add(121_000_000_000,Ordering::Relaxed);advanced=true;}
            std::thread::sleep(Duration::from_millis(1));
        }
        super::super::record(2,51,((index as u64)<<32)|(session.failed() as u64)|((session.complete() as u64)<<1));
        if !session.failed() {return false;}
        let next=std::format!("https://recovery.test/good{}",index+1);
        if session.navigate(&next).is_err() || !loaded(engine,&session,&next) {return false;}
        super::super::record(2,52,index as u64+1);
    }
    session.reload();
    if session.complete() || !loaded(engine,&session,"https://recovery.test/good3") {return false;}
    let passed=starts.get()==8 && cancels.get()==8;
    drop(session);super::drain_close(engine);OFFSET.store(0,Ordering::Relaxed);
    passed && renderer_recovery(engine) && failed_document_retry(engine)
}

// ------------------------=
// FUNC: failed_document_retry
// DESC: Retries an actual failed document through the production tab-group boundary before requiring any old completion event.
// ------------------=
fn failed_document_retry(engine:&Servo)->bool {
    let Ok(mut tabs)=session::TabSessions::new(engine,provider(),clock,128,128) else{return false;};
    let Ok(healthy)=tabs.create(engine,provider(),true) else{return false;};
    let good="https://recovery.test/good-neighbor";
    if tabs.current().unwrap().navigate(good).is_err() || !loaded(engine,tabs.current().unwrap(),good) {return false;}
    let Ok(failing)=tabs.create(engine,provider(),true) else{return false;};
    if tabs.current().unwrap().navigate("https://recovery.test/partial").is_err() {return false;}
    let deadline=Instant::now()+Duration::from_secs(10);
    while !tabs.current().unwrap().failed() && Instant::now()<deadline {
        if tabs.pump(engine,|_,_,_|{}).is_err() {return false;}
        std::thread::sleep(Duration::from_millis(1));
    }
    if !tabs.current().unwrap().failed() || tabs.recover_current(engine,provider()).is_err()
        || tabs.current().unwrap().failed() || tabs.active()!=failing {return false;}
    let next="https://recovery.test/good-explicit-retry";
    if tabs.current().unwrap().navigate(next).is_err() || !loaded(engine,tabs.current().unwrap(),next) {return false;}
    if tabs.select(healthy).is_err() || !loaded(engine,tabs.current().unwrap(),good) {return false;}
    drop(tabs);super::drain_close(engine);true
}

// ------------------------=
// FUNC: provider
// DESC: Creates an isolated single-lease transport for each real engine tab.
// ------------------=
fn provider()->Provider {
    Provider{active:None,serial:0,starts:Rc::new(Cell::new(0)),cancels:Rc::new(Cell::new(0)),stage:Rc::new(Cell::new(0))}
}

// ------------------------=
// FUNC: renderer_recovery
// DESC: Injects renderer failure and verifies the same tab can render again without destroying its healthy neighbor.
// ------------------=
fn renderer_recovery(engine:&Servo)->bool {
    let Ok(mut tabs)=session::TabSessions::new(engine,provider(),clock,128,128) else{return false;};
    let Ok(healthy)=tabs.create(engine,provider(),true) else{return false;};
    let first="https://recovery.test/good-neighbor";
    if tabs.current().unwrap().navigate(first).is_err() || !loaded(engine,tabs.current().unwrap(),first) {return false;}
    let Ok(failing)=tabs.create(engine,provider(),true) else{return false;};
    let second="https://recovery.test/good-before-crash";
    if tabs.current().unwrap().navigate(second).is_err() || !loaded(engine,tabs.current().unwrap(),second) {return false;}
    tabs.current().unwrap().inject_crash();
    if tabs.pump(engine,|_,_,_|{}).is_ok() {return false;}
    if tabs.select(healthy).is_err() || tabs.pump(engine,|_,_,_|{}).is_err()
        || tabs.current().unwrap().address().as_deref()!=Some(first) {return false;}
    if tabs.select(failing).is_err() || tabs.recover_current(engine,provider()).is_err() || tabs.active()!=failing {return false;}
    let next="https://recovery.test/good-after-crash";
    if tabs.current().unwrap().navigate(next).is_err() || !loaded(engine,tabs.current().unwrap(),next) {return false;}
    tabs.current().unwrap().inject_crash();
    if tabs.pump(engine,|_,_,_|{}).is_ok() || tabs.recover_current(engine,provider()).is_err() {return false;}
    tabs.current().unwrap().reload();
    if !loaded(engine,tabs.current().unwrap(),next) {return false;}
    if tabs.select(healthy).is_err() || tabs.current().unwrap().address().as_deref()!=Some(first) {return false;}
    super::super::record(2,53,1);
    drop(tabs);super::drain_close(engine);true
}
