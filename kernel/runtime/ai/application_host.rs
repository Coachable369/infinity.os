//! BSP-owned adapter between isolated application syscalls and the shared action registry.
use super::application::{self, Choice, Error, Outcome, Parameter, ParameterKind, Registry, Risk, Schema, Snapshot};

#[derive(Clone, Copy)]
pub struct Context { pub instance: u64, pub owner: [u8;16], pub session: [u8;16] }
static mut REGISTRY: Registry = Registry::new();
static mut CURRENT: Option<Context> = None;
#[no_mangle]
pub static mut INFINITY_APP_INTERACTION_SNAPSHOT:[u64;18]=[0x494e4641494e5431,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
#[no_mangle]
pub static mut INFINITY_APP_INTERACTION_STATE:[u8;application::STATE_BYTES]=[0;application::STATE_BYTES];
#[no_mangle]
pub static mut INFINITY_APP_INTERACTION_CHOICES:[u8;application::MAX_CHOICES*416]=[0;application::MAX_CHOICES*416];
static mut DIAGNOSTIC_COUNTS:[u64;7]=[0;7];

// ------------------------=
// FUNC: publish_diagnostics
// DESC: Exports only the live process's published contract and real syscall receipts under one read-only diagnostic seqlock.
// ------------------=
fn publish_diagnostics() {
    unsafe {
        let snapshot=CURRENT.and_then(|c|(&mut *(&raw mut REGISTRY)).diagnostic_snapshot(c.instance,c.owner,c.session).ok());
        let target=core::ptr::addr_of_mut!(INFINITY_APP_INTERACTION_SNAPSHOT).cast::<u64>();
        let sequence=target.add(2).read_volatile().wrapping_add(2)&!1;
        target.add(2).write_volatile(sequence|1);
        let state=&mut *(&raw mut INFINITY_APP_INTERACTION_STATE);state.fill(0);
        let records=&mut *(&raw mut INFINITY_APP_INTERACTION_CHOICES);records.fill(0);
        let mut choice_count=0;let mut schema_count=0;
        if let Some(snapshot)=snapshot {
            state[..snapshot.length].copy_from_slice(&snapshot.state[..snapshot.length]);
            for (index,choice) in snapshot.choices.iter().flatten().enumerate() {
                let record=&mut records[index*416..(index+1)*416];
                record[..4].copy_from_slice(&1u32.to_le_bytes());
                record[4..8].copy_from_slice(&choice.id.to_le_bytes());
                record[8..12].copy_from_slice(&(choice.risk as u32).to_le_bytes());
                record[16..64].copy_from_slice(&choice.action);record[64..160].copy_from_slice(&choice.label);
                for (slot,value) in choice.arguments.iter().enumerate(){record[160+slot*8..168+slot*8].copy_from_slice(&value.to_le_bytes());}
                if let Some(schema)=snapshot.schemas.iter().flatten().find(|schema|schema.choice==choice.id) {
                    record[12..16].copy_from_slice(&1u32.to_le_bytes());schema_count+=1;
                    for (slot,parameter) in schema.parameters.iter().enumerate() {
                        let at=192+slot*48;record[at..at+24].copy_from_slice(&parameter.name);
                        record[at+24..at+28].copy_from_slice(&(parameter.kind as u32).to_le_bytes());
                        record[at+32..at+40].copy_from_slice(&parameter.minimum.to_le_bytes());
                        record[at+40..at+48].copy_from_slice(&parameter.maximum.to_le_bytes());
                    }
                    record[384..408].copy_from_slice(&schema.text_name);
                    record[408..412].copy_from_slice(&(schema.text_limit as u32).to_le_bytes());
                }
                choice_count+=1;
            }
        }
        let values=[CURRENT.map_or(0,|c|c.instance),snapshot.map_or(0,|s|s.revision),
            snapshot.map_or(0,|s|s.agent_turn as u64),snapshot.map_or(0,|s|s.terminal as u64),
            snapshot.map_or(0,|s|s.length as u64),choice_count,schema_count,
            DIAGNOSTIC_COUNTS[0],DIAGNOSTIC_COUNTS[1],DIAGNOSTIC_COUNTS[2],DIAGNOSTIC_COUNTS[3],
            DIAGNOSTIC_COUNTS[4],DIAGNOSTIC_COUNTS[5],DIAGNOSTIC_COUNTS[6]];
        for (index,value) in values.into_iter().enumerate(){target.add(index+3).write_volatile(value);}
        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
        target.add(17).write_volatile(sequence);target.add(2).write_volatile(sequence);
    }
}

// ------------------------=
// FUNC: attach
// DESC: Binds the current isolated process to a fresh instance without an AI grant.
// ------------------=
pub fn attach(owner: [u8;16], session: [u8;16]) -> Result<Context, Error> {
    detach();
    unsafe {
        let instance = (&mut *(&raw mut REGISTRY)).register(owner,session)?;
        let context=Context{instance,owner,session}; CURRENT=Some(context); DIAGNOSTIC_COUNTS=[0;7];
        publish_diagnostics();Ok(context)
    }
}
// ------------------------=
// FUNC: detach
// DESC: Revokes the terminated process instance and all outstanding requests.
// ------------------=
pub fn detach() {
    unsafe { if let Some(c)=core::ptr::addr_of_mut!(CURRENT).replace(None) { let _=(&mut *(&raw mut REGISTRY)).close(c.instance,c.owner,c.session); } }
    unsafe {DIAGNOSTIC_COUNTS=[0;7];}publish_diagnostics();
}
// ------------------------=
// FUNC: context
// DESC: Identifies the live native process for trusted console routing.
// ------------------=
pub fn context() -> Option<Context> { unsafe { CURRENT } }
// ------------------------=
// FUNC: with_registry
// DESC: Runs a BSP-side trusted operation without exposing registry references to a native application.
// ------------------=
pub fn with_registry<T>(operation: impl FnOnce(&mut Registry) -> T) -> T { unsafe { operation(&mut *(&raw mut REGISTRY)) } }
// ------------------------=
// FUNC: observe
// DESC: Retrieves the authorized current observation for an exact authenticated user.
// ------------------=
pub fn observe(owner: [u8;16]) -> Result<(Context, Snapshot), Error> {
    let c=context().ok_or(Error::Missing)?;
    if owner!=c.owner {return Err(Error::Denied);}
    with_registry(|registry|registry.observe(c.instance,c.owner,c.session)).map(|state|(c,state))
}
// ------------------------=
// FUNC: syscall
// DESC: Decodes bounded little-endian version-one packets; caller identity comes only from the isolated process.
// ------------------=
pub fn syscall(operation:u64, session:[u8;16], input:&[u8], output:&mut [u8], now:u64)->Result<usize,Error> {
    let c=context().ok_or(Error::Missing)?;
    if session!=c.session {return Err(Error::Denied);}
    if input.len()<8 || u32_at(input,0)!=application::VERSION {return Err(Error::Invalid);}
    let result=with_registry(|registry|match operation {
        0x1150=>{
            if input.len()!=24+application::STATE_BYTES {return Err(Error::Invalid);}
            let length=u32_at(input,16) as usize;
            if length>application::STATE_BYTES {return Err(Error::Invalid);}
            let flags=u32_at(input,4);
            if flags & !3 != 0 {return Err(Error::Invalid);}
            registry.begin(c.instance,c.owner,c.session,u64_at(input,8),&input[24..24+length],flags&1!=0,flags&2!=0)?; Ok(0)
        },
        0x1151 | 0x1155=>{
            if input.len()!=if operation==0x1155 {416} else {192} {return Err(Error::Invalid);}
            let risk=match u32_at(input,8){0=>Risk::ReadOnly,1=>Risk::Reversible,2=>Risk::Sensitive,_=>return Err(Error::Invalid)};
            let mut action=[0;48];action.copy_from_slice(&input[16..64]);
            let mut label=[0;96];label.copy_from_slice(&input[64..160]);
            let mut arguments=[0;4];for (index,value) in arguments.iter_mut().enumerate(){*value=u64_at(input,160+index*8) as i64;}
            let choice=Choice{id:u32_at(input,4),action,label,arguments,risk};
            if operation==0x1155 {
                let mut parameters=[Parameter{name:[0;24],kind:ParameterKind::Fixed,minimum:0,maximum:0};4];
                for (index,parameter) in parameters.iter_mut().enumerate() {
                    let at=192+index*48;
                    parameter.name.copy_from_slice(&input[at..at+24]);
                    parameter.kind=match u32_at(input,at+24) {0=>ParameterKind::Fixed,1=>ParameterKind::Integer,2=>ParameterKind::Boolean,3=>ParameterKind::Reference,_=>return Err(Error::Invalid)};
                    parameter.minimum=u64_at(input,at+32) as i64; parameter.maximum=u64_at(input,at+40) as i64;
                }
                let mut text_name=[0;24];text_name.copy_from_slice(&input[384..408]);
                registry.offer_schema(c.instance,c.owner,c.session,choice,Schema{choice:choice.id,parameters,text_name,text_limit:u32_at(input,408) as usize})?;
            } else {registry.offer(c.instance,c.owner,c.session,choice)?;}
            Ok(0)
        },
        0x1152=>{registry.commit(c.instance,c.owner,c.session)?;Ok(0)},
        0x1153 | 0x1156=>{
            let length=if operation==0x1156 {592} else {72};
            if output.len()<length {return Err(Error::Invalid);}
            let Some(request)=registry.poll_payload(c.instance,c.owner,c.session,now,operation==0x1156)? else{return Ok(0);};
            output[..length].fill(0);output[..4].copy_from_slice(&1u32.to_le_bytes());
            output[4..8].copy_from_slice(&request.choice.id.to_le_bytes());
            output[8..16].copy_from_slice(&request.token.to_le_bytes());
            output[16..24].copy_from_slice(&request.instance.to_le_bytes());
            output[24..32].copy_from_slice(&request.revision.to_le_bytes());
            output[32..40].copy_from_slice(&request.deadline.to_le_bytes());
            for (index,value) in request.choice.arguments.iter().enumerate(){output[40+index*8..48+index*8].copy_from_slice(&value.to_le_bytes());}
            if operation==0x1156 {
                output[72..76].copy_from_slice(&(request.text_length as u32).to_le_bytes());
                output[76..80].copy_from_slice(&(request.typed as u32).to_le_bytes());
                output[80..80+request.text_length].copy_from_slice(&request.text[..request.text_length]);
            }
            Ok(length)
        },
        0x1154=>{
            if input.len()!=16 {return Err(Error::Invalid);}
            let outcome=match u32_at(input,4){0=>Outcome::Applied,1=>Outcome::Rejected,_=>return Err(Error::Invalid)};
            registry.finish(c.instance,c.owner,c.session,u64_at(input,8),outcome,now)?;Ok(0)
        },
        _=>Err(Error::Invalid),
    });
    if let Ok(length)=result {
        unsafe {
            if operation==0x1152 {DIAGNOSTIC_COUNTS[0]=DIAGNOSTIC_COUNTS[0].saturating_add(1);}
            if matches!(operation,0x1153|0x1156) && length>0 {
                DIAGNOSTIC_COUNTS[1]=DIAGNOSTIC_COUNTS[1].saturating_add(1);
                DIAGNOSTIC_COUNTS[4]=u32_at(output,4) as u64;DIAGNOSTIC_COUNTS[5]=u64_at(output,8);
            }
            if operation==0x1154 {
                let outcome=u32_at(input,4) as usize;
                DIAGNOSTIC_COUNTS[2+outcome]=DIAGNOSTIC_COUNTS[2+outcome].saturating_add(1);
                DIAGNOSTIC_COUNTS[6]=outcome as u64+1;
            }
        }
        if matches!(operation,0x1152|0x1154) || matches!(operation,0x1153|0x1156) && length>0 {publish_diagnostics();}
    }
    result
}
// ------------------------=
// FUNC: u32_at
// DESC: Reads a field only after the containing packet length has been checked.
// ------------------=
fn u32_at(bytes:&[u8],offset:usize)->u32 {u32::from_le_bytes(bytes[offset..offset+4].try_into().unwrap())}
// ------------------------=
// FUNC: u64_at
// DESC: Reads an unaligned little-endian field from a validated packet.
// ------------------=
fn u64_at(bytes:&[u8],offset:usize)->u64 {u64::from_le_bytes(bytes[offset..offset+8].try_into().unwrap())}
