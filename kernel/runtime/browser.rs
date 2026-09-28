//! Native browser supervisor. BSP owns UI and permission decisions; one leased
//! AP owns Servo and its private heap. No engine code runs in a paint callback.
use core::{ffi::c_void,sync::atomic::{AtomicPtr,AtomicU32,AtomicU64,Ordering}};
use infinity_browser_core::{worker as abi,mailbox::Mailbox,frames::Frames};
use crate::http_transport::{rand_chacha::ChaCha20Rng,rand_core::{RngCore,SeedableRng}};
use super::{execution::SecurityIdentity,capability::CapabilityId};
#[path="browser_entropy.rs"]
mod browser_entropy;
static RANDOM_SEEDS:browser_entropy::Seeds=browser_entropy::Seeds::new();
const HEAP_BYTES:usize=512*1024*1024;
// A page-aligned 512 MiB grant always contains a 256 MiB aligned buddy arena.
// It occupies loader-owned BSS, not installer payload bytes or the framebuffer.
#[repr(C,align(4096))]
struct Heap([u8;HEAP_BYTES]);
static mut HEAP:Heap=Heap([0;HEAP_BYTES]);
static STATE:AtomicU32=AtomicU32::new(0);
static FAILURE:AtomicU32=AtomicU32::new(0);
static GENERATION:AtomicU64=AtomicU64::new(0);
// Stable read-only diagnostic identity: do not rely on optimizer-private symbols.
#[no_mangle]
pub static INFINITY_BROWSER_PEAK:AtomicU64=AtomicU64::new(0);
#[no_mangle]
pub static INFINITY_BROWSER_FAILED_ALLOCATION:AtomicU32=AtomicU32::new(0);
#[no_mangle]
pub static INFINITY_BROWSER_LOCATION_HASH:AtomicU64=AtomicU64::new(0);
#[no_mangle]
pub static INFINITY_BROWSER_DOWNLOAD_STATE:AtomicU32=AtomicU32::new(0);
static FRAME_REVISION:AtomicU64=AtomicU64::new(0);
static ACTIVE_TAB:AtomicU32=AtomicU32::new(0);
static LOAD_REVISION:AtomicU64=AtomicU64::new(0);
#[no_mangle]
pub static INFINITY_BROWSER_LOADING:AtomicU32=AtomicU32::new(0);
#[no_mangle]
pub static INFINITY_BROWSER_PAGE_ERROR:AtomicU32=AtomicU32::new(0);
#[no_mangle]
pub static INFINITY_BROWSER_HISTORY:AtomicU32=AtomicU32::new(0);
static BOOT:AtomicPtr<crate::boot_info::BootInfo>=AtomicPtr::new(core::ptr::null_mut());
static OWNER:[AtomicU64;2]=[AtomicU64::new(0),AtomicU64::new(0)];
static COMMANDS:Mailbox<abi::Command,32>=Mailbox::new();
pub static FRAMES:Frames<16384000>=Frames::new();
static EVENTS:Mailbox<Event,32>=Mailbox::new();
static mut DIAGNOSTIC:([u8;2048],usize)=([0;2048],0);
static DOWNLOADS:Mailbox<Download,1>=Mailbox::new();
#[derive(Clone,Copy)]
pub struct Download {pub generation:u64,pub name:[u8;63],pub name_length:usize,pub media_type:[u8;127],
    pub type_length:usize,pub bytes:[u8;16384],pub length:usize}
static mut RNG:Option<ChaCha20Rng>=None;
static mut UTC:u64=0;
static mut EPOCH_NS:u64=0;
#[derive(Clone,Copy)]
pub struct Event {pub kind:u32,pub value:u32,pub length:usize,pub text:[u8;2048]}
static mut HOST:abi::Host=abi::Host {version:abi::VERSION,size:core::mem::size_of::<abi::Host>() as u32,
    context:core::ptr::null_mut(),heap:core::ptr::null_mut(),heap_length:HEAP_BYTES,
    cpu,monotonic,utc,entropy,idle,command,frame,event,begin,poll,cancel,download,fatal};

// ------------------------=
// FUNC: take_download
// DESC: Transfers a pending attachment only to its authenticated native browser owner for save consent.
// ------------------=
pub fn take_download(owner:SecurityIdentity)->Option<Download> {
    if !owned_by(owner) {return None;}
    DOWNLOADS.try_take().ok().flatten().filter(|value|value.generation==GENERATION.load(Ordering::Acquire))
}

// ------------------------=
// FUNC: download
// DESC: Validates and copies one bounded attachment without touching disk or accepting a website-selected path.
// ------------------=
unsafe extern "C" fn download(_: *mut c_void,name:*const u8,name_length:usize,
    media_type:*const u8,type_length:usize,body:*const u8,length:usize)->u32 {
    if name.is_null() || media_type.is_null() || body.is_null() || name_length>63 || type_length>127 || length>16384 {return 0;}
    let name=core::slice::from_raw_parts(name,name_length);
    let media_type=core::slice::from_raw_parts(media_type,type_length);
    let (Ok(name_text),Ok(type_text))=(core::str::from_utf8(name),core::str::from_utf8(media_type)) else {return 0;};
    if infinity_browser_core::download::validate_metadata(name_text,type_text).is_err() {return 0;}
    let mut value=Download{generation:GENERATION.load(Ordering::Acquire),name:[0;63],name_length,media_type:[0;127],type_length,bytes:[0;16384],length};
    value.name[..name_length].copy_from_slice(name);value.media_type[..type_length].copy_from_slice(media_type);
    value.bytes[..length].copy_from_slice(core::slice::from_raw_parts(body,length));
    DOWNLOADS.try_send(value).is_ok() as u32
}

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
pub unsafe fn start(owner:SecurityIdentity,caps:[CapabilityId;4])->Result<(),infinity_browser_core::startup::Error> {
    use infinity_browser_core::startup::{self,Error};
    if owner.0==[0;16] {return Err(Error::Session);}
    if STATE.load(Ordering::Acquire)!=0 {
        if !matches!(STATE.load(Ordering::Acquire),1|2) {return Err(Error::Worker);}
        return if owned_by(owner) && crate::drivers::browser_network::renew(owner,caps) {Ok(())}else{Err(Error::Network)};
    }
    let info=BOOT.load(Ordering::Acquire).as_ref().ok_or(Error::Boot)?;
    let seconds=startup::prerequisites(info.firmware_entropy_valid==1,
        crate::console::certificate_time(info.firmware_runtime_services))?;
    if !crate::drivers::browser_network::configure(owner,caps) {return Err(Error::Network);}
    let seed=RANDOM_SEEDS.derive(&info.firmware_entropy,&owner.0).ok_or(Error::Seed)?;
    RNG=Some(ChaCha20Rng::from_seed(seed));
    UTC=seconds;EPOCH_NS=super::ai::qwen::workers::clock_ns();
    HOST.heap=core::ptr::addr_of_mut!(HEAP.0).cast();
    OWNER[0].store(u64::from_le_bytes(owner.0[..8].try_into().unwrap()),Ordering::Release);
    OWNER[1].store(u64::from_le_bytes(owner.0[8..].try_into().unwrap()),Ordering::Release);
    STATE.store(1,Ordering::Release);
    if !super::ai::qwen::workers::background(worker) {STATE.store(0,Ordering::Release);return Err(Error::Worker);}
    Ok(())
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
    GENERATION.load(Ordering::Acquire),INFINITY_BROWSER_PEAK.load(Ordering::Acquire))}

// ------------------------=
// FUNC: diagnostic
// DESC: Copies bounded engine failure detail retained by the desktop event consumer.
// ------------------=
pub fn diagnostic()->([u8;2048],usize) {unsafe {DIAGNOSTIC}}

#[derive(Clone,Copy)]
pub struct Presentation {
    pub hovered_tab:u32,pub hovered_close:bool,
    pub frame_revision:u64,
    pub tabs:[TabPresentation;8],pub tab_count:usize,pub active_tab:u32,
    pub permission:u8,
    pub download_name:[u8;63],pub download_length:usize,pub download_state:u8,pub download_revision:u64,
    pub address:[u8;2048],pub address_length:usize,
    pub edit:[u8;2048],pub edit_length:usize,pub caret:usize,pub address_focused:bool,pub caret_visible:bool,pub address_selected:bool,
    pub title:[u8;256],pub title_length:usize,
    pub loading:bool,pub input_busy:bool,pub history:u32,pub error:u32,pub revision:u64,
}
#[derive(Clone,Copy)]
pub struct TabPresentation {pub id:u32,pub title:[u8;256],pub length:usize}
const EMPTY_TAB:TabPresentation=TabPresentation{id:0,title:[0;256],length:0};
static mut PRESENTATION:Presentation=Presentation{hovered_tab:0,hovered_close:false,frame_revision:0,address:[0;2048],address_length:0,title:[0;256],
    tabs:[EMPTY_TAB;8],tab_count:0,active_tab:0,
    permission:0,
    download_name:[0;63],download_length:0,download_state:0,download_revision:0,
    edit:[0;2048],edit_length:0,caret:0,address_focused:false,caret_visible:true,address_selected:false,
    title_length:0,loading:false,input_busy:false,history:0,error:0,revision:0};
static mut LAST_FRAME_REVISION:u64=0;
static mut LAST_VIEW_REVISION:u64=0;

// ------------------------=
// FUNC: hover_tab
// DESC: Invalidates native chrome only when the tab or close hover target changes.
// ------------------=
pub fn hover_tab(id:u32,close:bool) {unsafe {
    let view=&mut *(&raw mut PRESENTATION);
    if (view.hovered_tab,view.hovered_close)!=(id,close) {
        view.hovered_tab=id;view.hovered_close=close;view.revision=view.revision.wrapping_add(1);
    }
}}

impl Presentation {
    // ------------------------=
    // FUNC: page_key
    // DESC: Separates engine/content changes from native address, caret, title and tab-label updates.
    // ------------------=
    pub fn page_key(&self)->infinity_browser_core::damage::PageKey {
        infinity_browser_core::damage::PageKey {frame:self.frame_revision,tab:self.active_tab,
            error:self.error,permission:self.permission,download:self.download_state,
            download_content:self.download_revision,
            loading:self.loading,busy:self.input_busy}
    }
}

// ------------------------=
// FUNC: presentation
// DESC: Copies BSP-owned engine metadata for the native shell; no engine calls occur during paint.
// ------------------=
pub fn presentation()->Presentation {unsafe {PRESENTATION}}
// ------------------------=
// FUNC: launch_presentation
// DESC: Retains the requested address and exposes startup failure inside the browser instead of an invisible Console.
// ------------------=
pub fn launch_presentation(url:&[u8],error:Option<infinity_browser_core::startup::Error>) {unsafe {
    let view=&mut *(&raw mut PRESENTATION);
    view.address_length=url.len().min(view.address.len());
    view.address[..view.address_length].copy_from_slice(&url[..view.address_length]);
    view.error=error.map_or(0,|error|error as u32);
    view.loading=error.is_none();
    view.revision=view.revision.wrapping_add(1);
}}
// ------------------------=
// FUNC: frame_generation
// DESC: Separates tab surfaces within the existing window lifetime so a switch never displays another tab's pixels.
// ------------------=
pub fn frame_generation()->u64 {(GENERATION.load(Ordering::Acquire)<<32)|u64::from(ACTIVE_TAB.load(Ordering::Acquire))}

// ------------------------=
// FUNC: permission_presentation
// DESC: Presents explicit policy-governed network consent independently from engine startup.
// ------------------=
pub fn permission_presentation(state:u8) {unsafe {
    PRESENTATION.permission=state;PRESENTATION.revision=PRESENTATION.revision.wrapping_add(1);
}}

// ------------------------=
// FUNC: download_presentation
// DESC: Publishes native consent or save result without accepting untrusted UI markup.
// ------------------=
pub fn download_presentation(name:&[u8],state:u8) {unsafe {
    let view=&mut *(&raw mut PRESENTATION);
    view.download_length=name.len().min(63);view.download_name[..view.download_length].copy_from_slice(&name[..view.download_length]);
    view.download_state=state;view.loading=false;view.revision=view.revision.wrapping_add(1);
    view.download_revision=view.download_revision.wrapping_add(1);
    INFINITY_BROWSER_DOWNLOAD_STATE.store(state as u32,Ordering::Release);
}}

// ------------------------=
// FUNC: focus_address
// DESC: Keeps an address draft independent of asynchronous engine location updates.
// ------------------=
pub fn focus_address(focused:bool) {unsafe {
    let view=&mut *(&raw mut PRESENTATION);
    if !focused {view.address_selected=false;}
    if focused && !view.address_focused {
        view.edit=view.address;view.edit_length=view.address_length;view.caret=view.edit_length;view.address_selected=false;
    }
    if focused!=view.address_focused {view.address_focused=focused;view.revision=view.revision.wrapping_add(1);}
}}

// ------------------------=
// FUNC: select_address
// DESC: Focuses the native omnibox and selects its entire draft for standard shortcut replacement.
// ------------------=
pub fn select_address() {
    focus_address(true);
    unsafe {
        let view=&mut *(&raw mut PRESENTATION);
        view.address_selected=view.edit_length!=0;
        view.caret=view.edit_length;
        view.revision=view.revision.wrapping_add(1);
    }
}

// ------------------------=
// FUNC: edit_address
// DESC: Applies native bounded insertion, deletion and caret navigation to the address draft.
// ------------------=
pub fn edit_address(key:crate::ui::text_input::TextEditKey) {unsafe {
    use crate::ui::text_input::{self as text,TextEditKey as K};
    let view=&mut *(&raw mut PRESENTATION);
    if !view.address_focused {return;}
    if view.address_selected {
        match key {
            K::Character(c) if c.is_ascii_graphic() || c==b' ' => {view.edit_length=0;view.caret=0;},
            K::Backspace|K::Delete => {
                view.edit_length=0;view.caret=0;view.address_selected=false;
                view.revision=view.revision.wrapping_add(1);return;
            },
            K::Left|K::Home|K::Right|K::End => {
                view.caret=if matches!(key,K::Left|K::Home) {0}else{view.edit_length};
                view.address_selected=false;view.revision=view.revision.wrapping_add(1);return;
            },
            _=>return,
        }
        view.address_selected=false;
    }
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
        view.frame_revision=frame;
        let mut changed=frame!=LAST_FRAME_REVISION || view.revision!=LAST_VIEW_REVISION;
        let caret_visible=(super::ai::qwen::workers::clock_ns()/500_000_000)%2==0;
        if view.address_focused && caret_visible!=view.caret_visible {view.caret_visible=caret_visible;changed=true;}
        LAST_FRAME_REVISION=frame;
        for _ in 0..32 {
            let Some(event)=take_event() else {break;};
            match event.kind {
                abi::EVENT_DIAGNOSTIC=>{
                    let (bytes,length)=&mut *(&raw mut DIAGNOSTIC);
                    let count=event.length.min(bytes.len().saturating_sub(*length+1));
                    bytes[*length..*length+count].copy_from_slice(&event.text[..count]);
                    *length+=count;
                    if *length<bytes.len() {bytes[*length]=b'\n';*length+=1;}
                    continue;
                },
                abi::EVENT_OPEN=>{view.loading=false;view.error=0;view.history=0;},
                abi::EVENT_CLOSED=>{view.loading=false;view.history=0;view.tabs=[EMPTY_TAB;8];view.tab_count=0;view.active_tab=0;},
                abi::EVENT_TAB_CREATED=>{if view.tab_count<8 && !view.tabs.iter().any(|tab|tab.id==event.value) {
                    let at=infinity_browser_core::tabs::adjacent_index(view.tabs[..view.tab_count].iter().map(|tab|tab.id),view.active_tab);
                    view.tabs.copy_within(at..view.tab_count,at+1);
                    view.tabs[at]=TabPresentation{id:event.value,..EMPTY_TAB};view.tab_count+=1;
                }},
                abi::EVENT_TAB_SELECTED=>{view.active_tab=event.value;view.error=0;view.address_length=0;view.title_length=0;
                    view.history=0;view.address_focused=false;},
                abi::EVENT_TAB_CLOSED=>{if let Some(at)=view.tabs[..view.tab_count].iter().position(|tab|tab.id==event.value) {
                    view.tabs.copy_within(at+1..view.tab_count,at);view.tab_count-=1;view.tabs[view.tab_count]=EMPTY_TAB;
                }},
                abi::EVENT_LOAD=>{view.loading=event.value==0;if view.loading {view.error=0;}
                    LOAD_REVISION.fetch_add(1,Ordering::Release);},
                abi::EVENT_HISTORY=>view.history=event.value&3,
                abi::EVENT_ERROR=>{view.error=event.value+1;view.loading=false;},
                abi::EVENT_ADDRESS=>{view.address_length=event.length.min(view.address.len());
                    view.address[..view.address_length].copy_from_slice(&event.text[..view.address_length]);
                    let hash=view.address[..view.address_length].iter().fold(0xcbf29ce484222325u64,
                        |hash,byte|(hash^(*byte as u64)).wrapping_mul(0x100000001b3));
                    INFINITY_BROWSER_LOCATION_HASH.store(hash,Ordering::Release);},
                abi::EVENT_TITLE=>{view.title_length=event.length.min(view.title.len());
                    view.title[..view.title_length].copy_from_slice(&event.text[..view.title_length]);
                    if let Some(tab)=view.tabs.iter_mut().find(|tab|tab.id==view.active_tab) {
                        tab.length=view.title_length;tab.title=view.title;
                    }},
                _=>continue,
            }
            changed=true;
        }
        let error=FAILURE.load(Ordering::Acquire);
        if error!=view.error && error!=0 {view.error=error;view.loading=false;changed=true;}
        INFINITY_BROWSER_LOADING.store(view.loading as u32,Ordering::Release);
        INFINITY_BROWSER_PAGE_ERROR.store(view.error,Ordering::Release);
        INFINITY_BROWSER_HISTORY.store(view.history,Ordering::Release);
        if changed {view.revision=view.revision.wrapping_add(1);}
        LAST_VIEW_REVISION=view.revision;
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
    if value.kind==abi::OPEN {GENERATION.fetch_add(1,Ordering::AcqRel);ACTIVE_TAB.store(0,Ordering::Release);}
    out.write(value);1
}
// ------------------------=
// FUNC: frame
// DESC: Publishes owned RGBA pixels, never a global framebuffer pointer.
// ------------------=
unsafe extern "C" fn frame(_: *mut c_void,width:u32,height:u32,bytes:*const u8,length:usize) {
    if FRAMES.publish(frame_generation(),width,height,core::slice::from_raw_parts(bytes,length)).is_ok() {
        FRAME_REVISION.fetch_add(1,Ordering::Release);
    }
}
// ------------------------=
// FUNC: event
// DESC: Publishes bounded shell metadata and retains failures independently of queue capacity.
// ------------------=
unsafe extern "C" fn event(_: *mut c_void,kind:u32,value:u32,text:*const u8,length:usize) {
    if kind==abi::EVENT_TAB_SELECTED {ACTIVE_TAB.store(value,Ordering::Release);}
    if kind==abi::EVENT_MEMORY {INFINITY_BROWSER_PEAK.store(value as u64,Ordering::Release);return;}
    if kind==abi::EVENT_ALLOCATION_FAILURE {INFINITY_BROWSER_FAILED_ALLOCATION.store(value,Ordering::Release);return;}
    if kind==abi::EVENT_ERROR && value!=3 {FAILURE.store(value+1,Ordering::Release);}
    if length>2048 {return;}
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
