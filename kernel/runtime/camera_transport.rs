//! Lifecycle orchestration, not a hardware driver. A backend must acknowledge
//! endpoint quiescence before reuse; no firmware pointers cross this boundary.
use super::{Camera, Error};
use crate::runtime::{capability::CapabilityManager, execution::SecurityIdentity,
    iop::{IopMessage, OperationId}, native_memory::{PageTable, check_user_range, copy_to_user}};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Transfer { Pending, Frame(usize), Disconnected, Failed }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum State { Idle, Streaming, Reconnecting, Stopping, Faulted }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Attach { Absent, Pending, Ready }
pub trait Driver {
    // ------------------------=
    // FUNC: start
    // DESC: Performs one bounded discovery or negotiation step; errors require shutdown before reuse.
    // ------------------=
    fn start(&mut self) -> Result<Attach,()>;
    // ------------------------=
    // FUNC: poll
    // DESC: Advances a time-bounded transfer into owned scratch; never retains the supplied slice.
    // ------------------=
    fn poll(&mut self, scratch: &mut [u8]) -> Transfer;
    // ------------------------=
    // FUNC: stop
    // DESC: Returns true only after DMA, endpoint activity and stale completions are quiesced.
    // ------------------=
    fn stop(&mut self) -> Result<bool,()>;
}
pub struct Transport<'a,D:Driver> {
    camera: Camera<'a>, driver:D, state:State, attempts:u8,
    stop_attempts:u8, start_steps:u8, reconnect_after_stop:bool, next_ms:u64,
}
impl<'a,D:Driver> Transport<'a,D> {
    // ------------------------=
    // FUNC: new
    // DESC: Constructs an idle single-camera transport over caller-owned storage and an explicit backend.
    // ------------------=
    pub fn new(storage:&'a mut [u8],driver:D)->Result<Self,Error> {
        Ok(Self{camera:Camera::new(storage)?,driver,state:State::Idle,attempts:0,
            stop_attempts:0,start_steps:0,reconnect_after_stop:false,next_ms:0})
    }
    // ------------------------=
    // FUNC: state
    // DESC: Exposes structured lifecycle state without exposing device addresses.
    // ------------------=
    pub fn state(&self)->State {self.state}
    // ------------------------=
    // FUNC: cancel
    // DESC: Immediately invalidates delivery and schedules hardware quiescence, including during reconnect.
    // ------------------=
    pub fn cancel(&mut self) {
        self.camera.clear(); self.reconnect_after_stop=false;
        if self.state!=State::Idle && self.state!=State::Faulted {
            if self.state!=State::Stopping {self.stop_attempts=0;}
            self.state=State::Stopping;
        }
    }
    // ------------------------=
    // FUNC: dispatch
    // DESC: Routes authenticated IOP requests; read arguments are sequence, offset, destination, count, metadata pointer and reserved.
    // ------------------=
    pub fn dispatch(&mut self,message:&IopMessage,caller:SecurityIdentity,caps:&CapabilityManager,
        now_seconds:u64,args:[u64;6],tables:&mut impl PageTable)->Result<usize,Error> {
        match message.header.operation_type_id {
            op if op==OperationId::CameraCaptureStart as u32 => {
                if args!=[0;6] {return Err(Error::Invalid);}
                if self.state!=State::Idle {return Err(Error::Busy);}
                self.camera.start(message,caller,caps,now_seconds)?;
                self.attempts=0;self.start_steps=0;self.next_ms=0;self.state=State::Reconnecting;Ok(0)
            }
            op if op==OperationId::CameraCaptureStop as u32 => {
                if args!=[0;6] {return Err(Error::Invalid);}
                self.camera.stop(message,caller)?;self.cancel();Ok(0)
            }
            op if op==OperationId::CameraFrameRead as u32 => {
                Camera::header(message,caller,OperationId::CameraFrameRead)?;
                if args[5]!=0 {return Err(Error::Invalid);}
                if message.header.deadline<=now_seconds {return Err(Error::Expired);}
                if args[3]==0 {
                    if args[..4]!=[0;4] {return Err(Error::Invalid);}
                    check_user_range(tables,args[4],16,true).map_err(|_|Error::Memory)?;
                    let (sequence,length)=self.camera.metadata(caller,message.header.capability_ref,caps,now_seconds)?;
                    let mut bytes=[0;16];bytes[..8].copy_from_slice(&sequence.to_le_bytes());
                    bytes[8..].copy_from_slice(&(length as u64).to_le_bytes());
                    copy_to_user(tables,args[4],&bytes).map_err(|_|Error::Memory)?;Ok(16)
                } else {
                    if args[4]!=0 {return Err(Error::Invalid);}
                    self.camera.read_to_user(message,caller,caps,now_seconds,args[0],
                        usize::try_from(args[1]).map_err(|_|Error::Invalid)?,args[2],
                        usize::try_from(args[3]).map_err(|_|Error::Invalid)?,tables)
                }
            }
            _=>Err(Error::Invalid),
        }
    }
    // ------------------------=
    // FUNC: tick
    // DESC: Executes at most one bounded backend call per visit; retries discovery three times and shutdown eight times.
    // ------------------=
    pub fn tick(&mut self,caps:&CapabilityManager,now_seconds:u64,now_ms:u64,scratch:&mut [u8]) {
        if matches!(self.state,State::Streaming|State::Reconnecting)
            && self.camera.authorized(caps,now_seconds).is_err() {self.cancel();}
        match self.state {
            State::Reconnecting if now_ms>=self.next_ms => {
                if self.attempts>=3 {self.cancel();return;}
                self.start_steps+=1;
                if self.start_steps>72 {self.cancel();return;}
                match self.driver.start() {
                    Ok(Attach::Ready)=>{self.attempts+=1;self.start_steps=0;self.state=State::Streaming;},
                    Ok(Attach::Pending)=>{},
                    Ok(Attach::Absent)=>{self.attempts+=1;self.start_steps=0;self.next_ms=now_ms.saturating_add(250);},
                    Err(())=>{self.attempts+=1;self.start_steps=0;self.reconnect_after_stop=true;self.stop_attempts=0;self.state=State::Stopping;}
                }
            }
            State::Streaming => {
                if scratch.len()!=super::MAX_FRAME_BYTES {self.cancel();return;}
                match self.driver.poll(scratch) {
                    Transfer::Pending=>{},
                    Transfer::Frame(length)=>{
                        if length>scratch.len() || self.camera.publish(&scratch[..length],caps,now_seconds).is_err() {self.cancel();}
                    }
                    Transfer::Disconnected=>{
                        // Drop stale bytes while preserving the lease only for bounded reconnect.
                        self.camera.storage[..self.camera.length].fill(0);self.camera.length=0;self.camera.pinned=false;
                        self.reconnect_after_stop=true;self.stop_attempts=0;self.state=State::Stopping;
                    }
                    Transfer::Failed=>self.cancel(),
                }
                scratch.fill(0);
            }
            State::Stopping=>{
                self.stop_attempts+=1;
                match self.driver.stop() {
                    Ok(true)=>{
                        self.state=if self.reconnect_after_stop && self.camera.authorized(caps,now_seconds).is_ok() {
                            State::Reconnecting
                        } else {self.camera.clear();State::Idle};
                        self.next_ms=now_ms.saturating_add(250);
                    }
                    result=>if result.is_err() || self.stop_attempts>=8 {
                        self.camera.clear();self.state=State::Faulted;
                    },
                }
            }
            _=>{},
        }
    }
}
