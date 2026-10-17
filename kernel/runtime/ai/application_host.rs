//! BSP-owned adapter between isolated application syscalls and the shared action registry.
use super::application::{self, Choice, Error, Outcome, Registry, Risk, Snapshot};

#[derive(Clone, Copy)]
pub struct Context { pub instance: u64, pub owner: [u8;16], pub session: [u8;16] }
static mut REGISTRY: Registry = Registry::new();
static mut CURRENT: Option<Context> = None;

// ------------------------=
// FUNC: attach
// DESC: Binds the current isolated process to a fresh instance without an AI grant.
// ------------------=
pub fn attach(owner: [u8;16], session: [u8;16]) -> Result<Context, Error> {
    detach();
    unsafe {
        let instance = (&mut *(&raw mut REGISTRY)).register(owner,session)?;
        let context=Context{instance,owner,session}; CURRENT=Some(context); Ok(context)
    }
}
// ------------------------=
// FUNC: detach
// DESC: Revokes the terminated process instance and all outstanding requests.
// ------------------=
pub fn detach() {
    unsafe { if let Some(c)=core::ptr::addr_of_mut!(CURRENT).replace(None) { let _=(&mut *(&raw mut REGISTRY)).close(c.instance,c.owner,c.session); } }
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
    with_registry(|registry|match operation {
        0x1150=>{
            if input.len()!=24+application::STATE_BYTES {return Err(Error::Invalid);}
            let length=u32_at(input,16) as usize;
            if length>application::STATE_BYTES {return Err(Error::Invalid);}
            let flags=u32_at(input,4);
            if flags & !3 != 0 {return Err(Error::Invalid);}
            registry.begin(c.instance,c.owner,c.session,u64_at(input,8),&input[24..24+length],flags&1!=0,flags&2!=0)?; Ok(0)
        },
        0x1151=>{
            if input.len()!=192 {return Err(Error::Invalid);}
            let risk=match u32_at(input,8){0=>Risk::ReadOnly,1=>Risk::Reversible,2=>Risk::Sensitive,_=>return Err(Error::Invalid)};
            let mut action=[0;48];action.copy_from_slice(&input[16..64]);
            let mut label=[0;96];label.copy_from_slice(&input[64..160]);
            let mut arguments=[0;4];for (index,value) in arguments.iter_mut().enumerate(){*value=u64_at(input,160+index*8) as i64;}
            registry.offer(c.instance,c.owner,c.session,Choice{id:u32_at(input,4),action,label,arguments,risk})?;Ok(0)
        },
        0x1152=>{registry.commit(c.instance,c.owner,c.session)?;Ok(0)},
        0x1153=>{
            if output.len()<72 {return Err(Error::Invalid);}
            let Some(request)=registry.poll(c.instance,c.owner,c.session,now)? else{return Ok(0);};
            output[..72].fill(0);output[..4].copy_from_slice(&1u32.to_le_bytes());
            output[4..8].copy_from_slice(&request.choice.id.to_le_bytes());
            output[8..16].copy_from_slice(&request.token.to_le_bytes());
            output[16..24].copy_from_slice(&request.instance.to_le_bytes());
            output[24..32].copy_from_slice(&request.revision.to_le_bytes());
            output[32..40].copy_from_slice(&request.deadline.to_le_bytes());
            for (index,value) in request.choice.arguments.iter().enumerate(){output[40+index*8..48+index*8].copy_from_slice(&value.to_le_bytes());}
            Ok(72)
        },
        0x1154=>{
            if input.len()!=16 {return Err(Error::Invalid);}
            let outcome=match u32_at(input,4){0=>Outcome::Applied,1=>Outcome::Rejected,_=>return Err(Error::Invalid)};
            registry.finish(c.instance,c.owner,c.session,u64_at(input,8),outcome,now)?;Ok(0)
        },
        _=>Err(Error::Invalid),
    })
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
