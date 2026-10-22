//! Bounded camera service boundary. Hardware scheduling and syscall registration
//! must supply authenticated callers; this module never grants camera authority.
use super::{capability::{CapabilityManager, CapabilityType}, execution::SecurityIdentity,
    iop::{IopMessage, MessageType, OperationId, PayloadRef, IOP_VERSION},
    native_memory::{copy_to_user, PageTable}};

pub const MAX_FRAME_BYTES: usize = 1_228_800;
pub const MAX_READ_BYTES: usize = 4096;
#[path = "camera_transport.rs"]
pub mod transport;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Invalid, Denied, Expired, Busy, Unavailable, Stale, Memory }
#[derive(Clone, Copy)]
struct Session { owner: SecurityIdentity, capability: u64, deadline: u64 }
pub struct Camera<'a> {
    storage: &'a mut [u8],
    length: usize,
    sequence: u64,
    session: Option<Session>,
}
impl<'a> Camera<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves a single caller-owned frame slot, clearing previous contents without heap allocation.
    // ------------------=
    pub fn new(storage: &'a mut [u8]) -> Result<Self, Error> {
        if storage.len()!=MAX_FRAME_BYTES { return Err(Error::Invalid); }
        storage.fill(0);
        Ok(Self { storage, length: 0, sequence: 0, session: None })
    }
    // ------------------------=
    // FUNC: header
    // DESC: Rejects forged callers, unsupported wire versions, flags and indirect payloads before dispatch.
    // ------------------=
    fn header(message: &IopMessage, caller: SecurityIdentity, op: OperationId) -> Result<(), Error> {
        let h=&message.header;
        if h.protocol_version!=IOP_VERSION || h.schema_version!=1 || h.message_type!=MessageType::Request
            || h.caller_identity!=caller || h.operation_type_id!=op as u32 || h.payload_length!=0
            || h.flags!=0 || !matches!(message.payload_ref, PayloadRef::Inline) { return Err(Error::Invalid); }
        Ok(())
    }
    // ------------------------=
    // FUNC: start
    // DESC: Authorizes one explicit five-second capture lease; never replaces another active owner.
    // ------------------=
    pub fn start(&mut self, message: &IopMessage, caller: SecurityIdentity, caps: &CapabilityManager, now: u64) -> Result<(), Error> {
        Self::header(message,caller,OperationId::CameraCaptureStart)?;
        let h=&message.header;
        if h.deadline<=now || h.deadline-now>5 { return Err(Error::Expired); }
        caps.validate(h.capability_ref,caller,CapabilityType::CameraCapture,0,1,0,now).map_err(|_|Error::Denied)?;
        if self.session.is_some() && self.authorized(caps,now).is_ok() { return Err(Error::Busy); }
        self.clear();
        self.session=Some(Session {owner:caller, capability:h.capability_ref, deadline:h.deadline});
        Ok(())
    }
    // ------------------------=
    // FUNC: authorized
    // DESC: Rechecks expiration and capability revocation before every capture or delivery operation.
    // ------------------=
    pub fn authorized(&mut self, caps: &CapabilityManager, now: u64) -> Result<(), Error> {
        let s=self.session.ok_or(Error::Unavailable)?;
        let error=if now>=s.deadline {Some(Error::Expired)} else if
            caps.validate(s.capability,s.owner,CapabilityType::CameraCapture,0,1,0,now).is_err() {Some(Error::Denied)} else {None};
        if let Some(error)=error {self.clear(); return Err(error);}
        Ok(())
    }
    // ------------------------=
    // FUNC: metadata
    // DESC: Returns frame sequence and size only to the authenticated lease owner with current authority.
    // ------------------=
    pub fn metadata(&mut self, caller: SecurityIdentity, capability: u64, caps: &CapabilityManager, now: u64) -> Result<(u64,usize),Error> {
        let s=self.session.ok_or(Error::Unavailable)?;
        if caller!=s.owner || capability!=s.capability {return Err(Error::Denied);}
        self.authorized(caps,now)?;
        if self.length==0 {return Err(Error::Unavailable);}
        Ok((self.sequence,self.length))
    }
    // ------------------------=
    // FUNC: publish
    // DESC: Accepts a complete encoded frame from the trusted driver, replacing the prior slot with a new sequence.
    // ------------------=
    pub fn publish(&mut self, bytes: &[u8], caps: &CapabilityManager, now: u64) -> Result<(u64,usize), Error> {
        self.authorized(caps,now)?;
        if bytes.len()<4 || bytes.len()>MAX_FRAME_BYTES || bytes[..2]!=[0xff,0xd8] || bytes[bytes.len()-2..]!=[0xff,0xd9] {
            self.clear(); return Err(Error::Invalid);
        }
        let Some(next)=self.sequence.checked_add(1) else {self.clear(); return Err(Error::Stale);};
        self.storage[..self.length].fill(0);
        self.storage[..bytes.len()].copy_from_slice(bytes);
        self.length=bytes.len(); self.sequence=next;
        Ok((self.sequence,self.length))
    }
    // ------------------------=
    // FUNC: read_to_user
    // DESC: Copies at most one page into checked private user memory; sequence checks prevent mixing frames.
    // ------------------=
    pub fn read_to_user(&mut self, message: &IopMessage, caller: SecurityIdentity, caps: &CapabilityManager,
        now: u64, sequence: u64, offset: usize, destination: u64, count: usize, tables: &mut impl PageTable) -> Result<usize,Error> {
        Self::header(message,caller,OperationId::CameraFrameRead)?;
        let s=self.session.ok_or(Error::Unavailable)?;
        if caller!=s.owner || message.header.capability_ref!=s.capability {return Err(Error::Denied);}
        if message.header.deadline<=now || message.header.deadline>s.deadline {return Err(Error::Expired);}
        self.authorized(caps,now)?;
        if sequence!=self.sequence || self.length==0 {return Err(Error::Stale);}
        if count==0 || count>MAX_READ_BYTES || offset>self.length || count>self.length-offset {return Err(Error::Invalid);}
        copy_to_user(tables,destination,&self.storage[offset..offset+count]).map_err(|_|Error::Memory)?;
        Ok(count)
    }
    // ------------------------=
    // FUNC: stop
    // DESC: Allows an authenticated owner to discard its stream even after authority is revoked.
    // ------------------=
    pub fn stop(&mut self, message: &IopMessage, caller: SecurityIdentity) -> Result<(),Error> {
        Self::header(message,caller,OperationId::CameraCaptureStop)?;
        if self.session.map(|s|s.owner)!=Some(caller) {return Err(Error::Denied);}
        self.clear(); Ok(())
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases buffered image bytes and invalidates the lease on cancellation, device loss or deadline.
    // ------------------=
    pub fn clear(&mut self) {
        self.storage[..self.length].fill(0); self.length=0; self.session=None;
    }
}
impl Drop for Camera<'_> {
    // ------------------------=
    // FUNC: drop
    // DESC: Erases any outstanding frame when the owning service is torn down.
    // ------------------=
    fn drop(&mut self) { self.clear(); }
}
