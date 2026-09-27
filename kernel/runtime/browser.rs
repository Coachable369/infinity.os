//! Native browser supervisor. BSP owns UI and permission decisions; one leased
//! AP owns Servo and its private heap. No engine code runs in a paint callback.
use core::{ffi::c_void,sync::atomic::{AtomicPtr,AtomicU32,AtomicU64,Ordering}};
use infinity_browser_core::{worker as abi,mailbox::Mailbox,frames::Frames};
use crate::http_transport::{rand_chacha::ChaCha20Rng,rand_core::{RngCore,SeedableRng}};
use super::{execution::SecurityIdentity,capability::CapabilityId};
const HEAP_BYTES:usize=512*1024*1024;
// A page-aligned 512 MiB grant always contains a 256 MiB aligned buddy arena.
// It occupies loader-owned BSS, not installer payload bytes or the framebuffer.
#[repr(C,align(4096))]
struct Heap([u8;HEAP_BYTES]);
static mut HEAP:Heap=Heap([0;HEAP_BYTES]);
static STATE:AtomicU32=AtomicU32::new(0);
static FAILURE:AtomicU32=AtomicU32::new(0);
static GENERATION:AtomicU64=AtomicU64::new(0);
static PEAK:AtomicU64=AtomicU64::new(0);
static FRAME_REVISION:AtomicU64=AtomicU64::new(0);
static BOOT:AtomicPtr<crate::boot_info::BootInfo>=AtomicPtr::new(core::ptr::null_mut());
static OWNER:[AtomicU64;2]=[AtomicU64::new(0),AtomicU64::new(0)];
static COMMANDS:Mailbox<abi::Command,32>=Mailbox::new();
pub static FRAMES:Frames<16384000>=Frames::new();
static EVENTS:Mailbox<Event,32>=Mailbox::new();
static mut RNG:Option<ChaCha20Rng>=None;
static mut UTC:u64=0;
static mut EPOCH_NS:u64=0;
#[derive(Clone,Copy)]
pub struct Event {pub kind:u32,pub value:u32,pub length:usize,pub text:[u8;2048]}
static mut HOST:abi::Host=abi::Host {version:abi::VERSION,size:core::mem::size_of::<abi::Host>() as u32,
    context:core::ptr::null_mut(),heap:core::ptr::null_mut(),heap_length:HEAP_BYTES,
    cpu,monotonic,utc,entropy,idle,command,frame,event,begin,poll,cancel,fatal};

// ------------------------=
// FUNC: initialize
// DESC: Retains loader-owned boot services for later authenticated launch without starting an engine at boot.
// ------------------=
pub fn initialize(info:&'static crate::boot_info::BootInfo) {
    let _=BOOT.compare_exchange(core::ptr::null_mut(),core::ptr::from_ref(info).cast_mut(),Ordering::Release,Ordering::Relaxed);
}
// ------------------------=
// FUNC: owned_by
// DESC: Prevents an authenticated session from submitting work to another session's engine.
// ------------------=
fn owned_by(owner:SecurityIdentity)->bool {
    OWNER[0].load(Ordering::Acquire)==u64::from_le_bytes(owner.0[..8].try_into().unwrap())
        && OWNER[1].load(Ordering::Acquire)==u64::from_le_bytes(owner.0[8..].try_into().unwrap())
}
// ------------------------=
// FUNC: start
// DESC: Grants an isolated engine CPU and heap only after native time, entropy and explicit network authority exist.
// ------------------=
/// BSP only. BootInfo remains loader-owned; no firmware operation runs on the AP.
pub unsafe fn start(owner:SecurityIdentity,caps:[CapabilityId;4])->bool {
    if owner.0==[0;16] {return false;}
    if STATE.load(Ordering::Acquire)!=0 {return matches!(STATE.load(Ordering::Acquire),1|2) && owned_by(owner);}
    let Some(info)=BOOT.load(Ordering::Acquire).as_ref() else{return false;};
    if info.firmware_entropy_valid!=1 {return false;}
    let Some(seconds)=crate::console::certificate_time(info.firmware_runtime_services) else{return false;};
    if !crate::drivers::browser_network::configure(owner,caps) {return false;}
    use sha2::{Digest,Sha256};
    let mut hash=Sha256::new();hash.update(b"InfinityOS native browser RNG v1");hash.update(info.firmware_entropy);
    RNG=Some(ChaCha20Rng::from_seed(hash.finalize().into()));
    UTC=seconds;EPOCH_NS=super::ai::qwen::workers::clock_ns();
    HOST.heap=core::ptr::addr_of_mut!(HEAP.0).cast();
    OWNER[0].store(u64::from_le_bytes(owner.0[..8].try_into().unwrap()),Ordering::Release);
    OWNER[1].store(u64::from_le_bytes(owner.0[8..].try_into().unwrap()),Ordering::Release);
    STATE.store(1,Ordering::Release);
    if !super::ai::qwen::workers::background(worker) {STATE.store(0,Ordering::Release);return false;}
    true
}
// ------------------------=
// FUNC: submit
// DESC: Sends native input with explicit backpressure; callers retain events for retry instead of losing key releases.
// ------------------=
pub fn submit(owner:SecurityIdentity,value:abi::Command)->Result<(),abi::Command> {
    if !matches!(STATE.load(Ordering::Acquire),1|2) || !owned_by(owner) {return Err(value);}
    COMMANDS.try_send(value).map_err(|error|match error {
        infinity_browser_core::mailbox::SendError::Busy(v)|infinity_browser_core::mailbox::SendError::Full(v)=>v})
}
// ------------------------=
// FUNC: take_event
// DESC: Copies one pending metadata update without waiting for the engine CPU.
// ------------------=
pub fn take_event()->Option<Event> {EVENTS.try_take().ok().flatten()}
// ------------------------=
// FUNC: status
// DESC: Reports supervisor state, structured failure, frame generation and peak engine reservation.
// ------------------=
pub fn status()->(u32,u32,u64,u64) {(STATE.load(Ordering::Acquire),FAILURE.load(Ordering::Acquire),
    GENERATION.load(Ordering::Acquire),PEAK.load(Ordering::Acquire))}

#[derive(Clone,Copy)]
pub struct Presentation {
    pub address:[u8;2048],pub address_length:usize,
    pub edit:[u8;2048],pub edit_length:usize,pub caret:usize,pub address_focused:bool,pub caret_visible:bool,
    pub title:[u8;256],pub title_length:usize,
    pub loading:bool,pub input_busy:bool,pub history:u32,pub error:u32,pub revision:u64,
}
static mut PRESENTATION:Presentation=Presentation{address:[0;2048],address_length:0,title:[0;256],
    edit:[0;2048],edit_length:0,caret:0,address_focused:false,caret_visible:true,
    title_length:0,loading:false,input_busy:false,history:0,error:0,revision:0};
static mut LAST_FRAME_REVISION:u64=0;

// ------------------------=
// FUNC: presentation
// DESC: Copies BSP-owned engine metadata for the native shell; no engine calls occur during paint.
// ------------------=
pub fn presentation()->Presentation {unsafe {PRESENTATION}}

// ------------------------=
// FUNC: focus_address
// DESC: Keeps an address draft independent of asynchronous engine location updates.
// ------------------=
pub fn focus_address(focused:bool) {unsafe {
    let view=&mut *(&raw mut PRESENTATION);
    if focused && !view.address_focused {
        view.edit=view.address;view.edit_length=view.address_length;view.caret=view.edit_length;
    }
    if focused!=view.address_focused {view.address_focused=focused;view.revision=view.revision.wrapping_add(1);}
}}

// ------------------------=
// FUNC: edit_address
// DESC: Applies native bounded insertion, deletion and caret navigation to the address draft.
// ------------------=
pub fn edit_address(key:crate::ui::text_input::TextEditKey) {unsafe {
    use crate::ui::text_input::{self as text,TextEditKey as K};
    let view=&mut *(&raw mut PRESENTATION);
    if !view.address_focused {return;}
    let changed=match key {
        K::Character(c)=>text::insert_ascii(&mut view.edit,&mut view.edit_length,&mut view.caret,c),
        K::Backspace=>text::backspace(&mut view.edit,&mut view.edit_length,&mut view.caret),
        K::Delete=>text::delete(&mut view.edit,&mut view.edit_length,&mut view.caret),
        K::Left=>text::move_caret(&mut view.caret,view.edit_length,-1),
        K::Right=>text::move_caret(&mut view.caret,view.edit_length,1),
        K::Home=>text::move_caret(&mut view.caret,view.edit_length,-2),
        K::End=>text::move_caret(&mut view.caret,view.edit_length,2),
    };
    if changed {view.revision=view.revision.wrapping_add(1);}
}}

// ------------------------=
// FUNC: input_pressure
// DESC: Makes rejected whole gestures visible without fabricating a successful key delivery.
// ------------------=
pub fn input_pressure(full:bool) {unsafe {
    let view=&mut *(&raw mut PRESENTATION);
    if full!=view.input_busy {view.input_busy=full;view.revision=view.revision.wrapping_add(1);}
}}

// ------------------------=
// FUNC: poll_presentation
// DESC: Drains a bounded metadata batch and coalesces frame revisions without waiting on the engine worker.
// ------------------=
pub fn poll_presentation()->bool {
    unsafe {
        let view=&mut *(&raw mut PRESENTATION);
        let frame=FRAME_REVISION.load(Ordering::Acquire);
        let mut changed=frame!=LAST_FRAME_REVISION;
        let caret_visible=(super::ai::qwen::workers::clock_ns()/500_000_000)%2==0;
        if view.address_focused && caret_visible!=view.caret_visible {view.caret_visible=caret_visible;changed=true;}
        LAST_FRAME_REVISION=frame;
        for _ in 0..32 {
            let Some(event)=take_event() else {break;};
            match event.kind {
                abi::EVENT_OPEN=>{view.loading=false;view.error=0;view.history=0;},
                abi::EVENT_CLOSED=>{view.loading=false;view.history=0;},
                abi::EVENT_LOAD=>view.loading=event.value==0,
                abi::EVENT_HISTORY=>view.history=event.value&3,
                abi::EVENT_ERROR=>{view.error=event.value+1;view.loading=false;},
                abi::EVENT_ADDRESS=>{view.address_length=event.length.min(view.address.len());
                    view.address[..view.address_length].copy_from_slice(&event.text[..view.address_length]);},
                abi::EVENT_TITLE=>{view.title_length=event.length.min(view.title.len());
                    view.title[..view.title_length].copy_from_slice(&event.text[..view.title_length]);},
                _=>continue,
            }
            changed=true;
        }
        let error=FAILURE.load(Ordering::Acquire);
        if error!=view.error && error!=0 {view.error=error;view.loading=false;changed=true;}
        if changed {view.revision=view.revision.wrapping_add(1);}
        changed
    }
}
// ------------------------=
// FUNC: worker
// DESC: Enters the privately linked component on its sole native owner CPU.
// ------------------=
unsafe fn worker() {
    extern "C" {fn infinity_browser_private_infinity_browser_run(host:*mut abi::Host)->u32;}
    STATE.store(2,Ordering::Release);
    let result=infinity_browser_private_infinity_browser_run(core::ptr::addr_of_mut!(HOST));
    FAILURE.store(result,Ordering::Release);STATE.store(if result==0{4}else{3},Ordering::Release);
}
// ------------------------=
// FUNC: cpu
// DESC: Returns hardware CPU identity through the small target-specific adapter.
// ------------------=
unsafe extern "C" fn cpu(_: *mut c_void)->u64 {
    #[cfg(target_arch="aarch64")]
    {let value:u64;core::arch::asm!("mrs {}, mpidr_el1",out(reg)value,options(nomem,nostack));return value&0xff00ffffff;}
    #[cfg(target_arch="x86_64")]
    {let max=core::arch::x86_64::__cpuid(0).eax;
        if max>=11 {let leaf=core::arch::x86_64::__cpuid_count(11,0);if leaf.ebx!=0{return leaf.edx as u64;}}
        return (core::arch::x86_64::__cpuid(1).ebx>>24) as u64;}
}
// ------------------------=
// FUNC: monotonic
// DESC: Reads the native calibrated counter without service or UI locks.
// ------------------=
unsafe extern "C" fn monotonic(_: *mut c_void)->u64 {super::ai::qwen::workers::clock_ns()}
// ------------------------=
// FUNC: utc
// DESC: Advances verified BSP wall time using the monotonic counter without AP firmware calls.
// ------------------=
unsafe extern "C" fn utc(_: *mut c_void,seconds:*mut u64,nanos:*mut u32)->u32 {
    let elapsed=monotonic(core::ptr::null_mut()).saturating_sub(EPOCH_NS);
    seconds.write(UTC+elapsed/1_000_000_000);nanos.write((elapsed%1_000_000_000) as u32);1
}
// ------------------------=
// FUNC: entropy
// DESC: Supplies domain-separated cryptographic entropy owned exclusively by this engine worker.
// ------------------=
unsafe extern "C" fn entropy(_: *mut c_void,bytes:*mut u8,length:usize)->u32 {
    if length==0 {return 1;}
    if bytes.is_null() {return 0;}
    let Some(rng)=(&mut *(&raw mut RNG)).as_mut() else{return 0;};
    rng.fill_bytes(core::slice::from_raw_parts_mut(bytes,length));1
}
// ------------------------=
// FUNC: idle
// DESC: Leaves scheduling to the component without acquiring desktop services.
// ------------------=
unsafe extern "C" fn idle(_: *mut c_void) {core::hint::spin_loop();}
// ------------------------=
// FUNC: command
// DESC: Transfers one complete command and advances window generation only when the engine consumes Open.
// ------------------=
unsafe extern "C" fn command(_: *mut c_void,out:*mut abi::Command)->u32 {
    let Ok(Some(value))=COMMANDS.try_take() else{return 0;};
    if value.kind==abi::OPEN {GENERATION.fetch_add(1,Ordering::AcqRel);}
    out.write(value);1
}
// ------------------------=
// FUNC: frame
// DESC: Publishes owned RGBA pixels, never a global framebuffer pointer.
// ------------------=
unsafe extern "C" fn frame(_: *mut c_void,width:u32,height:u32,bytes:*const u8,length:usize) {
    if FRAMES.publish(GENERATION.load(Ordering::Acquire),width,height,core::slice::from_raw_parts(bytes,length)).is_ok() {
        FRAME_REVISION.fetch_add(1,Ordering::Release);
    }
}
// ------------------------=
// FUNC: event
// DESC: Publishes bounded shell metadata and retains failures independently of queue capacity.
// ------------------=
unsafe extern "C" fn event(_: *mut c_void,kind:u32,value:u32,text:*const u8,length:usize) {
    if kind==abi::EVENT_MEMORY {PEAK.store(value as u64,Ordering::Release);return;}
    if kind==abi::EVENT_ERROR {FAILURE.store(value+1,Ordering::Release);}
    if kind==abi::EVENT_DIAGNOSTIC || length>2048 {return;}
    let mut message=Event{kind,value,length,text:[0;2048]};
    if length>0 {message.text[..length].copy_from_slice(core::slice::from_raw_parts(text,length));}
    let _=EVENTS.try_send(message);
}
// ------------------------=
// FUNC: begin
// DESC: Requests network work only through the BSP capability-governed bridge.
// ------------------=
unsafe extern "C" fn begin(_: *mut c_void,url:*const u8,length:usize)->u64 {
    crate::drivers::browser_network::begin(core::slice::from_raw_parts(url,length))
}
// ------------------------=
// FUNC: poll
// DESC: Lends bridge-owned response data using the common integer-only ABI layout.
// ------------------=
unsafe extern "C" fn poll(_: *mut c_void,id:u64,out:*mut abi::Response)->u32 {
    // Both repr(C) Response types include the exact same versioned source file.
    crate::drivers::browser_network::poll(id,&mut *out.cast())
}
// ------------------------=
// FUNC: cancel
// DESC: Releases a bridge handle after the engine has finished copying its response.
// ------------------=
unsafe extern "C" fn cancel(_: *mut c_void,id:u64) {crate::drivers::browser_network::cancel(id);}
// ------------------------=
// FUNC: fatal
// DESC: Quarantines a failed engine CPU; the desktop can still display failure and close its native window.
// ------------------=
unsafe extern "C" fn fatal(_: *mut c_void,code:u32)->! {
    crate::drivers::browser_network::cancel_all();
    FAILURE.store(code,Ordering::Release);STATE.store(3,Ordering::Release);
    loop {core::hint::spin_loop();}
}
