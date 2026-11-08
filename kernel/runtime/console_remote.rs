//! Bounded authenticated SSH-to-Console mailbox; transport never runs application dispatch.
use super::{console_execution::Outcome,identity::StableId};
use core::sync::atomic::{AtomicBool,Ordering};
pub const CHANNELS:usize=4;
pub const INPUT:usize=infinity_enterprise_core::INPUT;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Binding {pub slot:u8,pub user:StableId,pub session:StableId}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum JobKind {Command,Input,Eof,Interrupt,Closed}
#[derive(Clone,Copy)]
pub struct Job {pub binding:Binding,pub sequence:u64,pub kind:JobKind,pub input:[u8;INPUT],pub length:usize,pub interactive:bool,pub columns:u16,pub rows:u16}
pub struct Reply {pub sequence:u64,pub outcome:Outcome,pub more:bool,pub secret:bool,pub private:bool,pub raw:bool,pub read_input:bool,pub clear_history:bool,pub close:bool}
struct Slot {binding:Option<Binding>,job:Option<Job>,replies:[Option<Reply>;2],sequence:u64,cancel_before:u64,closed:bool,columns:u16,rows:u16}
impl Slot {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an unbound channel with no authority, pending input, or response bytes.
    // ------------------=
    const fn new()->Self {Self {binding:None,job:None,replies:[None,None],sequence:0,cancel_before:0,closed:false,columns:80,rows:24}}
}
pub struct Mailbox {slots:[Slot;CHANNELS],cursor:usize}
impl Mailbox {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates fixed channel and response capacities without using the desktop session.
    // ------------------=
    pub const fn new()->Self {Self {slots:[const {Slot::new()};CHANNELS],cursor:0}}
    // ------------------------=
    // FUNC: bind
    // DESC: Binds an empty channel only to the independently authenticated nonzero SSH identity.
    // ------------------=
    pub fn bind(&mut self,binding:Binding)->bool {
        if binding.user.is_zero()||binding.session.is_zero(){return false;}
        let Some(slot)=self.slots.get_mut(binding.slot as usize)else{return false;};
        if slot.closed{return false;}
        if let Some(current)=slot.binding{return current==binding;}
        slot.binding=Some(binding);true
    }
    // ------------------------=
    // FUNC: active
    // DESC: Rejects disconnected and reused channel identities before dispatch or asynchronous delivery.
    // ------------------=
    pub fn active(&self,binding:Binding)->bool {self.slots.get(binding.slot as usize).is_some_and(|slot|!slot.closed&&slot.binding==Some(binding))}
    // ------------------------=
    // FUNC: submit
    // DESC: Queues one complete input line and returns its monotonically increasing channel sequence.
    // ------------------=
    pub fn submit(&mut self,binding:Binding,input:&[u8],interactive:bool)->Option<u64> {
        self.submit_kind(binding,input,interactive,JobKind::Command)
    }
    // ------------------------=
    // FUNC: input
    // DESC: Queues raw terminal bytes for the same session's interactive application controller.
    // ------------------=
    pub fn input(&mut self,binding:Binding,input:&[u8],interactive:bool)->Option<u64> {
        self.submit_kind(binding,input,interactive,JobKind::Input)
    }
    // ------------------------=
    // FUNC: eof
    // DESC: Queues drained channel input EOF after all accepted lines without cancelling native work or its output.
    // ------------------=
    pub fn eof(&mut self,binding:Binding,interactive:bool)->Option<u64>{self.submit_kind(binding,b"",interactive,JobKind::Eof)}
    // ------------------------=
    // FUNC: resize
    // DESC: Retains the channel's actual terminal dimensions for each subsequent command or raw input event.
    // ------------------=
    pub fn resize(&mut self,binding:Binding,columns:u16,rows:u16){
        if !self.active(binding)||columns==0||rows==0||columns>1000||rows>1000{return;}
        let slot=&mut self.slots[binding.slot as usize];slot.columns=columns;slot.rows=rows;
    }
    // ------------------------=
    // FUNC: submit_kind
    // DESC: Shares input bounds, sequence ownership and backpressure between command lines and raw terminal events.
    // ------------------=
    fn submit_kind(&mut self,binding:Binding,input:&[u8],interactive:bool,kind:JobKind)->Option<u64> {
        if !self.active(binding)||input.len()>INPUT{return None;}
        let slot=&mut self.slots[binding.slot as usize];if slot.job.is_some(){return None;}
        slot.sequence=slot.sequence.saturating_add(1);
        let mut bytes=[0;INPUT];bytes[..input.len()].copy_from_slice(input);
        slot.job=Some(Job{binding,sequence:slot.sequence,kind,input:bytes,length:input.len(),interactive,columns:slot.columns,rows:slot.rows});Some(slot.sequence)
    }
    // ------------------------=
    // FUNC: interrupt
    // DESC: Revokes earlier replies and replaces only this channel's queued input with an explicit cancellation event.
    // ------------------=
    pub fn interrupt(&mut self,binding:Binding,interactive:bool) {
        if !self.active(binding){return;}
        let slot=&mut self.slots[binding.slot as usize];slot.sequence=slot.sequence.saturating_add(1);
        slot.cancel_before=slot.sequence;slot.replies=[None,None];
        slot.job=Some(Job{binding,sequence:slot.sequence,kind:JobKind::Interrupt,input:[0;INPUT],length:0,interactive,columns:slot.columns,rows:slot.rows});
    }
    // ------------------------=
    // FUNC: retire
    // DESC: Erases channel buffers and retains a close event until the Console releases its session-owned work.
    // ------------------=
    pub fn retire(&mut self,binding:Binding) {
        if !self.active(binding){return;}
        let slot=&mut self.slots[binding.slot as usize];slot.closed=true;slot.replies=[None,None];
        slot.job=Some(Job{binding,sequence:slot.sequence,kind:JobKind::Closed,input:[0;INPUT],length:0,interactive:false,columns:slot.columns,rows:slot.rows});
    }
    // ------------------------=
    // FUNC: take_job
    // DESC: Fairly transfers one command or lifecycle event to the serialized Console loop outside runtime and network borrows.
    // ------------------=
    pub fn take_job(&mut self)->Option<Job> {
        self.take_ready_job([true;CHANNELS])
    }
    // ------------------------=
    // FUNC: take_ready_job
    // DESC: Retains command/input jobs under per-shell backpressure while always delivering interrupt and close cleanup fairly.
    // ------------------=
    pub fn take_ready_job(&mut self,ready:[bool;CHANNELS])->Option<Job> {
        for _ in 0..CHANNELS {let index=self.cursor;self.cursor=(self.cursor+1)%CHANNELS;
            if self.slots[index].job.as_ref().is_some_and(|job|matches!(job.kind,JobKind::Command|JobKind::Input|JobKind::Eof)&&!ready[index]){continue;}
            if let Some(job)=self.slots[index].job.take(){if job.kind==JobKind::Closed{self.slots[index]=Slot::new();}return Some(job);}}
        None
    }
    // ------------------------=
    // FUNC: reply
    // DESC: Accepts bounded ordered output only for the original live channel; returns the owned response on backpressure or revocation.
    // ------------------=
    pub fn reply(&mut self,binding:Binding,reply:Reply)->Result<(),Reply> {
        if !self.active(binding){return Err(reply);}
        let slot=&mut self.slots[binding.slot as usize];
        if reply.sequence<slot.cancel_before||reply.sequence>slot.sequence{return Err(reply);}
        let Some(target)=slot.replies.iter_mut().find(|target|target.is_none())else{return Err(reply);};
        *target=Some(reply);Ok(())
    }
    // ------------------------=
    // FUNC: take_reply
    // DESC: Transfers one response to the SSH terminal only after its previous wire output has drained.
    // ------------------=
    pub fn take_reply(&mut self,binding:Binding)->Option<Reply> {
        if !self.active(binding){return None;}
        let slot=&mut self.slots[binding.slot as usize];let reply=slot.replies[0].take();
        slot.replies[0]=slot.replies[1].take();reply
    }
}
static LOCK:AtomicBool=AtomicBool::new(false);
static mut MAILBOX:Mailbox=Mailbox::new();
struct Guard;
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases the short mailbox critical section before any external command handler can run.
    // ------------------=
    fn drop(&mut self){LOCK.store(false,Ordering::Release);}
}
// ------------------------=
// FUNC: access
// DESC: Serializes bounded mailbox mutation without waiting or retaining an application callback.
// ------------------=
fn access<T>(call:impl FnOnce(&mut Mailbox)->T)->Option<T>{if LOCK.swap(true,Ordering::Acquire){return None;}let _guard=Guard;Some(call(unsafe{&mut *(&raw mut MAILBOX)}))}
// ------------------------=
// FUNC: bind
// DESC: Publishes an authenticated transport binding without borrowing Console state.
// ------------------=
pub fn bind(binding:Binding)->bool{access(|mailbox|mailbox.bind(binding)).unwrap_or(false)}
// ------------------------=
// FUNC: active
// DESC: Checks whether a retained headless Console still owns its channel binding.
// ------------------=
pub fn active(binding:Binding)->bool{access(|mailbox|mailbox.active(binding)).unwrap_or(false)}
// ------------------------=
// FUNC: submit
// DESC: Transfers one complete remote input line to the shared native command dispatcher.
// ------------------=
pub fn submit(binding:Binding,input:&[u8],interactive:bool)->Option<u64>{access(|mailbox|mailbox.submit(binding,input,interactive)).flatten()}
// ------------------------=
// FUNC: input
// DESC: Transfers a bounded raw input burst without interpreting application key sequences in the transport.
// ------------------=
pub fn input(binding:Binding,input:&[u8],interactive:bool)->Option<u64>{access(|mailbox|mailbox.input(binding,input,interactive)).flatten()}
// ------------------------=
// FUNC: eof
// DESC: Transfers input EOF only after SSH has returned every buffered byte to its owning native Console.
// ------------------=
pub fn eof(binding:Binding,interactive:bool)->Option<u64>{access(|mailbox|mailbox.eof(binding,interactive)).flatten()}
// ------------------------=
// FUNC: resize
// DESC: Updates a live channel's dimensions without changing its identity or queued input.
// ------------------=
pub fn resize(binding:Binding,columns:u16,rows:u16){let _=access(|mailbox|mailbox.resize(binding,columns,rows));}
// ------------------------=
// FUNC: interrupt
// DESC: Queues cancellation for this remote session without touching the local Console.
// ------------------=
pub fn interrupt(binding:Binding,interactive:bool){let _=access(|mailbox|mailbox.interrupt(binding,interactive));}
// ------------------------=
// FUNC: retire
// DESC: Queues revocation cleanup for a disconnected remote channel.
// ------------------=
pub fn retire(binding:Binding){let _=access(|mailbox|mailbox.retire(binding));}
// ------------------------=
// FUNC: take_job
// DESC: Provides the next headless job to the native serialized input loop.
// ------------------=
pub fn take_job()->Option<Job>{access(Mailbox::take_job).flatten()}
// ------------------------=
// FUNC: take_ready_job
// DESC: Leaves blocked shell input in its channel slot while giving revocation and interruption priority over output backpressure.
// ------------------=
pub fn take_ready_job(ready:[bool;CHANNELS])->Option<Job>{access(|mailbox|mailbox.take_ready_job(ready)).flatten()}
// ------------------------=
// FUNC: reply
// DESC: Keeps ownership of pending output with the caller if channel backpressure prevents delivery.
// ------------------=
pub fn reply(binding:Binding,value:Reply)->Result<(),Reply>{let mut value=Some(value);access(|mailbox|mailbox.reply(binding,value.take().unwrap())).unwrap_or_else(||Err(value.unwrap()))}
// ------------------------=
// FUNC: take_reply
// DESC: Lets the transport drain one native result with its existing SSH window accounting.
// ------------------=
pub fn take_reply(binding:Binding)->Option<Reply>{access(|mailbox|mailbox.take_reply(binding)).flatten()}
