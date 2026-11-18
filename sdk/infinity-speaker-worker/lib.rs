#![no_std]
use infinity_speaker_core as s;
use infinity_face_core::inference::Model;
const BASE:usize=0x200000000;
const LIMIT:usize=128*1024*1024;
// ------------------------=
// FUNC: call
// DESC: Invokes only the isolated identity worker ABI through native target instructions.
// ------------------=
fn call(op:u64,a:[u64;6])->i64{unsafe{
    #[cfg(target_arch="aarch64")]
    {let mut r=a[0];core::arch::asm!("svc #0",in("x8")op,inout("x0")r,in("x1")a[1],in("x2")a[2],in("x3")a[3],in("x4")a[4],in("x5")a[5],options(nostack));r as i64}
    #[cfg(target_arch="x86_64")]
    {let mut r=op;core::arch::asm!("int 0x80",inout("rax")r,in("rdi")a[0],in("rsi")a[1],in("rdx")a[2],in("rcx")a[3],in("r8")a[4],in("r9")a[5],options(nostack));r as i64}
}}
// ------------------------=
// FUNC: exit
// DESC: Terminates without providing a session or authentication result to applications.
// ------------------=
fn exit(code:u64)->!{call(0x1000,[code,0,0,0,0,0]);loop{call(0x1001,[0;6]);}}
// ------------------------=
// FUNC: panic
// DESC: Fails closed on unexpected worker faults.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>)->!{exit(70)}
struct Heap{used:usize}
impl Heap{
    // ------------------------=
    // FUNC: bytes
    // DESC: Reserves aligned disjoint slices within the kernel-mapped private heap.
    // ------------------=
    unsafe fn bytes(&mut self,n:usize)->Result<&'static mut[u8],u64>{
        let start=(self.used+63)&!63;let end=start.checked_add(n).ok_or(71u64)?;
        if end>LIMIT{return Err(71);}
        self.used=end;Ok(core::slice::from_raw_parts_mut((BASE+start) as *mut u8,n))
    }
    // ------------------------=
    // FUNC: load
    // DESC: Copies one OS-selected immutable model or captured sample through bounded private syscall chunks.
    // ------------------=
    unsafe fn load(&mut self,id:u64)->Result<&'static mut[u8],u64>{
        let n=call(0x1280,[id,0,0,0,0,0]);if n<=0||n>48*1024*1024{return Err(72);}
        let out=self.bytes(n as usize)?;
        for offset in (0..out.len()).step_by(65536){
            let n=(out.len()-offset).min(65536);
            if call(0x1280,[id,offset as u64,out.as_mut_ptr().add(offset) as u64,n as u64,0,0])!=n as i64{return Err(72);}
        }
        Ok(out)
    }
}
// ------------------------=
// FUNC: run
// DESC: Generates a speaker embedding in isolated memory; only the kernel may compare it to an enrolled identity.
// ------------------=
unsafe fn run()->Result<(),u64>{
    let mut heap=Heap{used:0};let model=heap.load(1)?;let head=heap.load(2)?;let raw=heap.load(3)?;
    if raw.len()!=s::SAMPLES*2{return Err(73);}
    let pcm=core::slice::from_raw_parts(raw.as_ptr() as *const i16,s::SAMPLES);
    let input:&mut[f32;s::FEATURES]=&mut *(heap.bytes(s::FEATURES*4)?.as_mut_ptr() as *mut _);
    s::features(pcm,input).map_err(|e|if e==s::Error::Clipped{78u64}else{74u64})?;
    let m=Model::parse(model).map_err(|_|75u64)?;
    let arena=core::slice::from_raw_parts_mut(heap.bytes(m.arena_len*4)?.as_mut_ptr() as *mut f32,m.arena_len);
    let trunk:&mut[f32;s::TRUNK_OUTPUT]=&mut *(heap.bytes(s::TRUNK_OUTPUT*4)?.as_mut_ptr() as *mut _);
    let mut output=[0f32;s::DIM];
    s::embed(model,head,input,arena,trunk,&mut output,||call(0x1281,[0;6])==1).map_err(|_|76u64)?;
    let result=call(0x1282,[output.as_ptr() as u64,(s::DIM*4) as u64,0,0,0,0]);
    for v in &mut output{core::ptr::write_volatile(v,0.0);}
    if result!=s::DIM as i64*4{return Err(77);}
    Ok(())
}
// ------------------------=
// FUNC: infinity_app_entry
// DESC: Starts the fixed model worker and reports failures only through process exit status.
// ------------------=
#[no_mangle]
pub extern "C" fn infinity_app_entry()->!{exit(unsafe{run()}.err().unwrap_or(0))}
