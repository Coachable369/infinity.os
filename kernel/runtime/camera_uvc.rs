//! Shared UVC MJPEG framing; the only platform-specific operation is Exchange.
use super::camera::transport::{Attach,Driver,Transfer};
pub type Exchange=unsafe extern "efiapi" fn(u64,*mut u8,*mut u64)->i64;
pub struct Uvc<'a> {
    exchange:Exchange,phase:u8,probe:[u8;26],payload:&'a mut [u8;65536],
    frame:&'a mut [u8],length:usize,limit:usize,fid:u8,synchronized:bool,discarding:bool,
}
impl<'a> Uvc<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Binds a trusted native USB adapter to fixed scratch without allocating or exporting pointers.
    // ------------------=
    pub fn new(exchange:Exchange,payload:&'a mut [u8;65536],frame:&'a mut [u8])->Self {
        Self{exchange,phase:0,probe:[0;26],payload,frame,length:0,limit:0,fid:0,synchronized:false,discarding:false}
    }
    // ------------------------=
    // FUNC: reset
    // DESC: Wipes private frame assembly when a stream stops or is replaced.
    // ------------------=
    fn reset(&mut self) {
        self.frame.fill(0);self.payload.fill(0);self.length=0;self.synchronized=false;self.discarding=false;
    }
}
impl Driver for Uvc<'_> {
    // ------------------------=
    // FUNC: start
    // DESC: Negotiates one control operation per scheduler visit for the allowlisted 640x480 five-fps MJPEG camera.
    // ------------------=
    fn start(&mut self)->Result<Attach,()> {
        if self.phase==0 {
            let mut size=0;
            match unsafe{(self.exchange)(0,core::ptr::null_mut(),&mut size)} {
                0=>{self.reset();self.probe=[0;26];self.probe[..8].copy_from_slice(&[1,0,1,5,0x80,0x84,0x1e,0]);self.phase=1;},
                1=>return Ok(Attach::Absent),2=>return Ok(Attach::Pending),_=>return Err(()),
            }
            return Ok(Attach::Pending);
        }
        if self.phase>3 {return Err(());}
        let mut size=26;
        if unsafe{(self.exchange)(self.phase as u64,self.probe.as_mut_ptr(),&mut size)}!=0 || size!=26 {return Err(());}
        if self.phase==2 {
            let frame=u32::from_le_bytes(self.probe[18..22].try_into().unwrap()) as usize;
            let payload=u32::from_le_bytes(self.probe[22..26].try_into().unwrap()) as usize;
            if self.probe[2]!=1 || self.probe[3]!=5 || u32::from_le_bytes(self.probe[4..8].try_into().unwrap())!=2_000_000
                || frame==0 || frame>self.frame.len() || !(2..=65536).contains(&payload) {return Err(());}
            self.limit=payload;
        }
        self.phase+=1;Ok(if self.phase==4 {Attach::Ready}else{Attach::Pending})
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Reads one bounded bulk packet and assembles only complete noninterrupted JPEG frames.
    // ------------------=
    fn poll(&mut self,scratch:&mut [u8])->Transfer {
        if self.phase!=4 {return Transfer::Failed;}
        let mut size=self.limit as u64;
        let status=unsafe{(self.exchange)(4,self.payload.as_mut_ptr(),&mut size)};
        if status==1 {self.length=0;self.discarding=self.synchronized;return Transfer::Pending;}
        if status==-3 {self.reset();self.phase=0;return Transfer::Disconnected;}
        if status!=0 {return Transfer::Failed;}
        let n=size as usize;
        if n<2 || n>self.limit || self.payload[0]<2 || self.payload[0] as usize>n {return Transfer::Failed;}
        let fid=self.payload[1]&1;
        if !self.synchronized || fid!=self.fid {self.length=0;self.fid=fid;self.synchronized=true;self.discarding=false;}
        if self.payload[1]&0x40!=0 {self.frame[..self.length].fill(0);self.length=0;self.discarding=true;return Transfer::Pending;}
        if self.discarding {return Transfer::Pending;}
        let header=self.payload[0] as usize;let count=n-header;
        if count>self.frame.len()-self.length {return Transfer::Failed;}
        self.frame[self.length..self.length+count].copy_from_slice(&self.payload[header..n]);self.length+=count;
        if self.payload[1]&2==0 {return Transfer::Pending;}
        let length=self.length;
        if length<4 || length>scratch.len() || self.frame[..2]!=[0xff,0xd8] || self.frame[length-2..length]!=[0xff,0xd9] {
            self.frame[..length].fill(0);self.length=0;self.discarding=true;return Transfer::Pending;
        }
        scratch[..length].copy_from_slice(&self.frame[..length]);
        self.frame[..length].fill(0);self.length=0;self.discarding=true;
        Transfer::Frame(length)
    }
    // ------------------------=
    // FUNC: stop
    // DESC: Stops synchronous bulk requests and acknowledges transport quiescence only after the endpoint shutdown request succeeds.
    // ------------------=
    fn stop(&mut self)->Result<bool,()> {
        let mut size=0;
        let result=unsafe{(self.exchange)(5,core::ptr::null_mut(),&mut size)};
        self.reset();
        // A missing protocol also proves no outstanding synchronous request
        // remains. This is not a claim about the physical camera's power state.
        if result!=0 && result != -3 {return Err(());}
        self.phase=0;Ok(true)
    }
}
