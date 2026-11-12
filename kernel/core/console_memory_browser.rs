//! Only the immutable signed-in Infinity Memory program can invoke this private UIKit service.
use crate::runtime::{ai, console_execution::Bytes, native_memory::{copy_from_user, copy_to_user, PageTable}};
pub const PATH: &[u8] = b"/System/infinity-memory.py";

// ------------------------=
// FUNC: dispatch
// DESC: Validates the native program, bounded UTF-8 input and operation before calling AIKit with the kernel-owned session.
// ------------------=
pub fn dispatch<T: PageTable>(allowed: bool, owner:[u8;16], session:[u8;16], tables:&mut T, operation:u64, args:[u64;6]) -> Option<i64> {
    if operation != 0x12f8 { return None; }
    if !allowed { return Some(-13); }
    if args[0] > 7 || args[2] > 3 || args[4] > 512 { return Some(-22); }
    let mut input = [0;512];
    let input = &mut input[..args[4] as usize];
    if !input.is_empty() && copy_from_user(tables,args[3],input).is_err() {return Some(-14);}
    let mut output = Bytes::<4096>::new();
    let result = ai::with_ai_runtime(|ai| ai.browse_memories(owner,session,args[0],args[1],args[2] as u8,input,&mut output));
    use ai::persistent_memory::Error;
    Some(match result {
        Ok(()) if copy_to_user(tables,args[5],output.bytes()).is_ok() => output.bytes().len() as i64,
        Ok(()) => -14,
        Err(Error::Unauthorized) => -13, Err(Error::Capacity) => -28,
        Err(Error::NotFound) => -2, Err(Error::Invalid) => -22, Err(Error::Unavailable) => -95,
        Err(_) => -5,
    })
}
