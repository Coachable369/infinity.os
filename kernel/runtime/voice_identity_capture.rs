//! Bounded enrollment-only microphone lifetime. No desktop audio policy lives here.
pub const LIFETIME:u64=240_000_000_000;
pub trait Backend {
 // ------------------------=
 // FUNC: open
 // DESC: Opens one authorized input stream until the supplied deadline.
 // ------------------=
 fn open(&mut self,until:u64)->bool;
 // ------------------------=
 // FUNC: renew
 // DESC: Renews the existing stream without restarting its device.
 // ------------------=
 fn renew(&mut self)->bool;
 // ------------------------=
 // FUNC: discard
 // DESC: Discards queued audio at a new recording boundary.
 // ------------------=
 fn discard(&mut self)->bool;
 // ------------------------=
 // FUNC: close
 // DESC: Releases the owned stream and capability.
 // ------------------=
 fn close(&mut self);
}
pub struct Session {active:bool,started:u64,until:u64,renew_at:u64}
impl Session {
 // ------------------------=
 // FUNC: new
 // DESC: Creates a dormant session with no microphone authority.
 // ------------------=
 pub const fn new()->Self{Self{active:false,started:0,until:0,renew_at:0}}
 // ------------------------=
 // FUNC: begin
 // DESC: Opens once or reuses a healthy session, discarding queued audio before each explicit recording.
 // ------------------=
 pub fn begin(&mut self,now:u64,b:&mut impl Backend)->bool{
  if self.active&&!self.service(now,b){return false;}
  if !self.active{
   self.started=now;self.until=now.saturating_add(LIFETIME);
   if !b.open(self.until){return false;}
   self.active=true;self.renew_at=now.saturating_add(500_000_000);
  }
  if !b.discard(){self.close(b);return false;}
  true
 }
 // ------------------------=
 // FUNC: service
 // DESC: Renews a single bounded lease without device restart; expiration and failures close it without automatic retry.
 // ------------------=
 pub fn service(&mut self,now:u64,b:&mut impl Backend)->bool{
  if !self.active{return true;}
  if now<self.started||now>=self.until {self.close(b);return false;}
  if now>=self.renew_at{
   if !b.renew(){self.close(b);return false;}
   self.renew_at=now.saturating_add(500_000_000);
  }
  true
 }
 // ------------------------=
 // FUNC: close
 // DESC: Releases microphone authority exactly once on completion, cancellation or failure.
 // ------------------=
 pub fn close(&mut self,b:&mut impl Backend){if self.active{b.close();}self.active=false;}
}
#[cfg(test)]
mod tests {
 use super::*;
 #[derive(Default)] struct Mic{opens:usize,closes:usize,discards:usize,renewals:usize,failed:bool,queued:usize}
 impl Backend for Mic{
 // ------------------------=
 // FUNC: open
 // DESC: Counts device opens in the deterministic microphone fixture.
 // ------------------=
 fn open(&mut self,_:u64)->bool{self.opens+=1;!self.failed}
 // ------------------------=
 // FUNC: renew
 // DESC: Models healthy renewal or a terminal device fault.
 // ------------------=
 fn renew(&mut self)->bool{self.renewals+=1;!self.failed}
 // ------------------------=
 // FUNC: discard
 // DESC: Removes prior-sample audio at an explicit recording boundary.
 // ------------------=
 fn discard(&mut self)->bool{self.discards+=1;self.queued=0;!self.failed}
 // ------------------------=
 // FUNC: close
 // DESC: Counts release of the fixture's single device stream.
 // ------------------=
 fn close(&mut self){self.closes+=1;}
 }
 // ------------------------=
 // FUNC: retries_reuse_one_stream_and_discard_previous_audio
 // DESC: Exercises repeated recordings and analysis intervals without growing device or lease ownership.
 // ------------------=
 #[test] fn retries_reuse_one_stream_and_discard_previous_audio(){
  let(mut s,mut m)=(Session::new(),Mic::default());
  for i in 0..100 {m.queued=2048;assert!(s.begin(i*1_000_000_000,&mut m));assert_eq!(m.queued,0);assert!(s.service(i*1_000_000_000+700_000_000,&mut m));}
  assert_eq!(m.opens,1);assert_eq!(m.closes,0);assert_eq!(m.discards,100);
  s.close(&mut m);s.close(&mut m);assert_eq!(m.closes,1);
 }
 // ------------------------=
 // FUNC: faults_and_expiry_do_not_reopen
 // DESC: Bounds idle capture and rejects renewal faults without a device restart loop.
 // ------------------=
 #[test] fn faults_and_expiry_do_not_reopen(){
  for fault in [false,true]{let(mut s,mut m)=(Session::new(),Mic::default());assert!(s.begin(10,&mut m));m.failed=fault;
   assert!(!s.service(if fault{1_000_000_000}else{LIFETIME+10},&mut m));
   for _ in 0..100{assert!(s.service(LIFETIME+11,&mut m));}
   assert_eq!((m.opens,m.closes),(1,1));}
 }
}
