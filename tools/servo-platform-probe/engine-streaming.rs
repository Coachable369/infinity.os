use super::{resources, session, Servo};
use std::{cell::Cell, rc::Rc, string::ToString, time::{Duration, Instant}};

struct Provider { stage: u8, finish: Rc<Cell<bool>>, cancelled: Rc<Cell<u32>>, observed: Rc<Cell<u32>> }
impl resources::Provider for Provider {
    // ------------------------=
    // FUNC: begin
    // DESC: Admits only the controlled progressive document.
    // ------------------=
    fn begin(&mut self, url:&str)->Result<u64,()> {
        if url!="https://stream.test/" {return Err(());}
        self.stage=0;Ok(1)
    }
    // ------------------------=
    // FUNC: poll_stream
    // DESC: Withholds document completion until the test has observed its real early pixels.
    // ------------------=
    fn poll_stream(&mut self,_:u64)->Result<Option<resources::Event>,()> {
        let event=match self.stage {
            0=>resources::Event::Head(200,std::vec![("Content-Type".into(),"text/html".into())]),
            1=>{
                // Cross Servo's 1445-byte MIME sniff boundary while withholding EOF.
                let mut prefix=b"<!doctype html><html><head></head><body style='margin:0;background:white'><div style='width:160px;height:100px;background:rgb(240,20,20)'>Early</div>".to_vec();
                prefix.resize(2048,b' ');resources::Event::Data(prefix)
            },
            2 if self.finish.get()=>resources::Event::Data(b"<div style='width:160px;height:80px;background:rgb(20,20,240)'>Late</div></body></html>".to_vec()),
            2=>return Ok(None),
            3=>resources::Event::Done,
            _=>return Err(()),
        };
        self.stage+=1;self.observed.set(self.stage as u32);Ok(Some(event))
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Counts native resource release after the stream terminates.
    // ------------------=
    fn cancel(&mut self,_:u64) {self.cancelled.set(self.cancelled.get()+1);}
}

// ------------------------=
// FUNC: verify
// DESC: Requires visible document pixels before EOF, then later pixels and completion through the production session.
// ------------------=
pub fn verify(engine:&Servo)->bool {
    let finish=Rc::new(Cell::new(false));let cancelled=Rc::new(Cell::new(0));
    let observed=Rc::new(Cell::new(0));
    let Ok(session)=session::Session::new(engine,Provider{stage:0,finish:finish.clone(),cancelled:cancelled.clone(),observed:observed.clone()},
        super::super::monotonic,256,192) else {return false;};
    if session.navigate("https://stream.test/").is_err() {return false;}
    let deadline=Instant::now()+Duration::from_secs(15);
    let mut early=false;let mut frames=0;let mut red=0;
    while !early && Instant::now()<deadline {
        if session.pump(engine,|width,height,bytes| {
            frames+=1;red=bytes.chunks_exact(4).filter(|p|p[0]>200 && p[1]<80 && p[2]<80).count();early=red>5000;
            if early && option_env!("INFINITY_BROWSER_STREAM_ONLY")==Some("1") {
                super::super::record(2,36,((width as u64)<<32)|height as u64);
                for (index,chunk) in bytes.chunks(8).enumerate() {
                    let mut packed=[0;8];packed[..chunk.len()].copy_from_slice(chunk);
                    super::super::record(5,index as u64,u64::from_le_bytes(packed));
                }
            }
        }).is_err() {return false;}
        std::thread::sleep(Duration::from_millis(1));
    }
    super::super::record(2,47,observed.get() as u64);
    super::super::record(2,48,frames);
    super::super::record(2,49,red as u64);
    super::super::record(2,50,session.complete() as u64 | ((session.failed() as u64)<<1) | ((cancelled.get() as u64)<<8));
    if !early || session.complete() || cancelled.get()!=0 {return false;}
    super::super::record(2,41,1);
    finish.set(true);
    let deadline=Instant::now()+Duration::from_secs(15);let mut late=false;
    while (!late || !session.complete()) && Instant::now()<deadline {
        if session.pump(engine,|_,_,bytes| {
            late=bytes.chunks_exact(4).filter(|p|p[2]>200 && p[0]<80 && p[1]<80).count()>5000;
        }).is_err() {return false;}
        std::thread::sleep(Duration::from_millis(1));
    }
    let passed=late && session.complete() && cancelled.get()==1;
    let passed=passed && verify_gzip(engine);
    super::super::record(2,41,if passed {2}else{0});passed
}

struct CompressedProvider { stage:u8, body:std::vec::Vec<u8>, streamed:bool }
impl resources::Provider for CompressedProvider {
    // ------------------------=
    // FUNC: begin
    // DESC: Restricts compressed-response coverage to the fixture origin.
    // ------------------=
    fn begin(&mut self,url:&str)->Result<u64,()> {
        if url!="https://gzip.test/" {return Err(());}self.stage=0;Ok(1)
    }
    // ------------------------=
    // FUNC: poll_stream
    // DESC: Exercises both complete and chunked compressed native response paths.
    // ------------------=
    fn poll_stream(&mut self,_:u64)->Result<Option<resources::Event>,()> {
        let headers=std::vec![("Content-Type".into(),"text/html".into()),("Content-Encoding".into(),"gzip".into()),("Content-Length".into(),self.body.len().to_string())];
        let event=if !self.streamed && self.stage==0 {
            resources::Event::Complete(resources::Response{status:200,headers,body:self.body.clone()})
        } else {match self.stage {
            0=>resources::Event::Head(200,headers),
            1=>resources::Event::Data(self.body[..self.body.len()/2].to_vec()),
            2=>resources::Event::Data(self.body[self.body.len()/2..].to_vec()),
            3=>resources::Event::Done,
            _=>return Err(()),
        }};
        self.stage+=1;Ok(Some(event))
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Fixture has no external resources to retain.
    // ------------------=
    fn cancel(&mut self,_:u64) {}
}

// ------------------------=
// FUNC: verify_gzip
// DESC: Requires real Servo CSS pixels and completed loads from both compressed response adapters.
// ------------------=
fn verify_gzip(engine:&Servo)->bool {
    use std::io::Write;
    for streamed in [false,true] {
        let mut encoder=flate2::write::GzEncoder::new(std::vec::Vec::new(),flate2::Compression::default());
        if encoder.write_all(b"<!doctype html><html><head><style>body{margin:0;background:rgb(20,220,30)}</style></head><body>Compressed native page</body></html>").is_err() {return false;}
        let Ok(body)=encoder.finish() else {return false;};
        let Ok(session)=session::Session::new(engine,CompressedProvider{stage:0,body,streamed},super::super::monotonic,256,192) else {return false;};
        if session.navigate("https://gzip.test/").is_err() {return false;}
        let deadline=Instant::now()+Duration::from_secs(15);let mut green=false;
        while (!green || !session.complete()) && Instant::now()<deadline {
            if session.pump(engine,|_,_,bytes| {
                green=bytes.chunks_exact(4).filter(|p|p[1]>180 && p[0]<80 && p[2]<80).count()>30000;
            }).is_err() {return false;}
            std::thread::sleep(Duration::from_millis(1));
        }
        if !green || !session.complete() || session.failed() {return false;}
    }
    true
}
