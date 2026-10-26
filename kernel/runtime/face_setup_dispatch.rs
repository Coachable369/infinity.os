//! Private dispatcher for the OS-selected setup worker. This is deliberately
//! separate from the general application syscall table and has no unlock call.
use super::{execution::SecurityIdentity, face_setup::{Setup,wire},
    native_memory::{PageTable,copy_from_user,copy_to_user}};

pub struct Dispatch<'a> {
    pub owner:SecurityIdentity,
    pub setup:&'a mut Setup,
    pub detector:&'a [u8],
    pub recognizer:&'a [u8],
}
impl Dispatch<'_> {
    // ------------------------=
    // FUNC: call
    // DESC: Checks the trusted process identity and explicit authorization on every bounded worker operation.
    // ------------------=
    pub fn call(&mut self, caller:SecurityIdentity, authorized:bool, tables:&mut impl PageTable,
        op:u64,args:[u64;6],now:u64)->i64 {
        if caller!=self.owner || !authorized {return -13;}
        if self.setup.tick(now).is_err() {return -110;}
        match op {
            wire::CLOCK_MS=>i64::try_from(now).unwrap_or(i64::MAX),
            wire::MODEL_READ=>self.model(tables,args),
            wire::CONTROL_READ=>{
                if args[1]!=core::mem::size_of::<wire::Control>() as u64 || args[2..].iter().any(|&v|v!=0) {return -22;}
                let c=self.setup.control();
                // Explicit serialization avoids exposing struct padding across the privilege boundary.
                let mut bytes=[0u8;144];
                bytes[..4].copy_from_slice(&c.version.to_le_bytes());
                bytes[4..8].copy_from_slice(&c.command.to_le_bytes());
                for (index,value) in [c.command_id,c.worker,c.lease,c.account].into_iter().enumerate() {
                    bytes[8+index*8..16+index*8].copy_from_slice(&value.to_le_bytes());
                }
                bytes[40..44].copy_from_slice(&c.name_len.to_le_bytes());
                bytes[48..112].copy_from_slice(&c.name);bytes[112..144].copy_from_slice(&c.model);
                if copy_to_user(tables,args[0],&bytes).is_err() {-14}else{144}
            }
            wire::REPORT_WRITE=>{
                if args[1]!=32 || args[2..].iter().any(|&v|v!=0) {return -22;}
                let mut b=[0u8;32];
                if copy_from_user(tables,args[0],&mut b).is_err() {return -14;}
                let word=|i:usize|u32::from_le_bytes([b[i],b[i+1],b[i+2],b[i+3]]);
                let r=wire::Report {version:word(0),phase:word(4),error:word(8),photos:word(12),
                    comparisons:word(16),reserved:word(20),command_id:u64::from_le_bytes(b[24..32].try_into().unwrap())};
                if self.setup.accept(r,now).is_err() {-22}else{32}
            }
            wire::PREVIEW_WRITE|wire::ENROLLMENT_WRITE=>{
                if args[3]==0 || args[3]>4096 || args[4]!=0 || args[5]!=0 {return -22;}
                let Ok(offset)=usize::try_from(args[1]) else {return -22;};
                let mut bytes=[0u8;4096];let count=args[3] as usize;
                if copy_from_user(tables,args[2],&mut bytes[..count]).is_err() {return -14;}
                let result=if op==wire::ENROLLMENT_WRITE {self.setup.write_enrollment(args[0],offset,&bytes[..count],now)}
                    else {self.setup.write_preview(args[0],offset,&bytes[..count],now)};
                for byte in &mut bytes {unsafe {core::ptr::write_volatile(byte,0);}}
                if result.is_err() {-22}else{count as i64}
            }
            _=>-38,
        }
    }
    // ------------------------=
    // FUNC: model
    // DESC: Copies only pinned models in overflow-safe chunks capped at 64 KiB with no caller-supplied paths.
    // ------------------=
    fn model(&self,tables:&mut impl PageTable,args:[u64;6])->i64 {
        if args[4]!=0 || args[5]!=0 {return -22;}
        let data=match args[0] {1=>self.detector,2=>self.recognizer,_=>return -2};
        if args[3]==0 {return if args[1]==0 && args[2]==0 {data.len() as i64}else{-22};}
        if args[3]>64*1024 {return -22;}
        let Ok(offset)=usize::try_from(args[1]) else {return -22;};
        let Some(end)=offset.checked_add(args[3] as usize) else {return -22;};
        let Some(bytes)=data.get(offset..end) else {return -22;};
        if copy_to_user(tables,args[2],bytes).is_err() {-14}else{bytes.len() as i64}
    }
}
