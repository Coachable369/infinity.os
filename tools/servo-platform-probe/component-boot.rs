#![no_std]
#![no_main]
#[path="../../sdk/infinity-browser-core/worker.rs"] mod abi;
use abi as worker;
#[path="../../sdk/infinity-browser-core/input_queue.rs"] mod input_queue;
#[path="../../sdk/infinity-browser-core/pointer.rs"] mod pointer;
#[path="../../sdk/infinity-browser-core/frames.rs"] mod frames;
#[path="guest/entropy_probe.rs"] mod entropy_probe;
use core::ffi::c_void;
core::arch::global_asm!(include_str!("guest/aarch64-memory.S"));
core::arch::global_asm!(
    ".section .text.entry", ".global _start", "_start:",
    "ldr x0, =0x7ff00000", "mov sp, x0", "mov x0, #(3 << 20)",
    "msr cpacr_el1, x0", "adr x0, component_vectors", "msr vbar_el1, x0", "isb",
    "bl probe_memory", "bl component_boot", "b .",
    ".balign 2048", "component_vectors:", ".rept 16", "b component_fault", ".space 124", ".endr"
);
static mut STEP:u32=0;
static mut FRAMES:u32=0;
static mut RELEASES:u32=0;
static mut NEXT_ID:u64=0;
static mut REDIRECT_ID:u64=0;
static mut HISTORY:u32=0;
static mut LOCATION:u32=0;
static mut LOAD_STARTS:u32=0;
static mut INPUT:input_queue::Queue<16>=input_queue::Queue::new();
static PIXELS:frames::Frames<81920>=frames::Frames::new();
static HTML:&[u8]=b"<!doctype html><html style='background:red;min-height:100vh'><script>let down=false,clicked=false;document.documentElement.style.background='rgb(12,34,56)';document.addEventListener('click',e=>{if(e.clientX===40&&e.clientY===40)clicked=true});document.addEventListener('keydown',e=>{if(e.key==='K'&&e.shiftKey&&e.ctrlKey&&!e.altKey&&!e.metaKey&&e.repeat)down=true});document.addEventListener('keyup',e=>{if(clicked&&down&&e.key==='K'&&e.shiftKey&&e.ctrlKey&&!e.repeat)document.documentElement.style.background='rgb(34,56,78)'})</script></html>";
static HEADERS:&[u8]=b"content-type: text/html\r\n";
static mut HOST:abi::Host=abi::Host {
    version:abi::VERSION,size:core::mem::size_of::<abi::Host>() as u32,context:core::ptr::null_mut(),
    heap:0x80000000 as *mut u8,heap_length:256*1024*1024,
    cpu,monotonic,utc,entropy,idle,command,frame,event,begin,poll,cancel,fatal,
};
// ------------------------=
// FUNC: record
// DESC: Emits structured fixture results without a host runtime.
// ------------------=
fn record(status:u64,detail:u64) { unsafe { for value in [9,status,detail] {for byte in value.to_le_bytes() {
    while (0x09000018 as *const u32).read_volatile()&32!=0 {}
    (0x09000000 as *mut u32).write_volatile(byte as u32);
}}}}
// ------------------------=
// FUNC: finish
// DESC: Powers off the disposable fixture after recording its actual outcome.
// ------------------=
fn finish(status:u64,detail:u64)->! {
    record(status,detail);
    unsafe { core::arch::asm!("hvc #0",in("x0") 0x84000008u64,options(noreturn)); }
}
// ------------------------=
// FUNC: panic
// DESC: Makes fixture panic an explicit failed acceptance result.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo)->! { finish(1,100) }
// ------------------------=
// FUNC: component_fault
// DESC: Records the exact guest fault address for native component diagnostics.
// ------------------=
#[no_mangle]
extern "C" fn component_fault()->! {
    let pc:u64;unsafe {core::arch::asm!("mrs {}, elr_el1",out(reg) pc);} finish(1,pc)
}
// ------------------------=
// FUNC: cpu
// DESC: Returns the real owner CPU identity to the isolated runtime.
// ------------------=
unsafe extern "C" fn cpu(_: *mut c_void)->u64 {let id:u64;core::arch::asm!("mrs {}, mpidr_el1",out(reg) id);id&0xff00ffffff}
// ------------------------=
// FUNC: monotonic
// DESC: Converts architectural ticks with the actual timer frequency.
// ------------------=
unsafe extern "C" fn monotonic(_: *mut c_void)->u64 {
    let ticks:u64;let frequency:u64;
    core::arch::asm!("isb","mrs {}, cntvct_el0","mrs {}, cntfrq_el0",out(reg) ticks,out(reg) frequency);
    (ticks as u128*1_000_000_000/frequency as u128) as u64
}
// ------------------------=
// FUNC: utc
// DESC: Supplies native RTC time through the same production ABI.
// ------------------=
unsafe extern "C" fn utc(_: *mut c_void,seconds:*mut u64,nanos:*mut u32)->u32 {
    seconds.write((0x09010000 as *const u32).read_volatile() as u64);nanos.write(0);1
}
// ------------------------=
// FUNC: entropy
// DESC: Supplies actual guest CPU entropy rather than deterministic test bytes.
// ------------------=
unsafe extern "C" fn entropy(_: *mut c_void,bytes:*mut u8,length:usize)->u32 {
    entropy_probe::fill(core::slice::from_raw_parts_mut(bytes,length)) as u32
}
// ------------------------=
// FUNC: idle
// DESC: Leaves cooperative scheduling to the component without blocking callbacks.
// ------------------=
unsafe extern "C" fn idle(_: *mut c_void) {core::hint::spin_loop();}
// ------------------------=
// FUNC: command
// DESC: Drives open, immediate navigation, resize, close and reopen through the real ABI.
// ------------------=
unsafe extern "C" fn command(_: *mut c_void,out:*mut abi::Command)->u32 {
    if (&mut *(&raw mut INPUT)).drain(1,|value|{out.write(value);true})==1 {return 1;}
    if let Some(frame)=PIXELS.acquire(1) {
        let (width,height)=frame.size();
        let color=if STEP==10 {[34,56,78,255]}else{[12,34,56,255]};
        if frame.bytes().chunks_exact(4).all(|pixel|pixel==color) {
            if STEP==17 && LOCATION==3 {STEP=18;}
            let next=match (STEP,width,height) {(2,128,128)=>3,(4,160,96)=>5,(8,128,128)=>9,(10,128,128)=>11,_=>STEP};
            if next!=STEP {STEP=next;FRAMES+=1;record(2,STEP as u64);}
        }
    }
    let mut value=abi::Command::empty();
    match STEP {
        0|6=>{value.kind=abi::OPEN;value.a=128;value.b=128;STEP+=1;},
        1|7=>{value.kind=abi::NAVIGATE;let url:&[u8]=if STEP==1 {b"https://fixture.test/redirect"}else{b"https://fixture.test/"};value.text[..url.len()].copy_from_slice(url);value.length=url.len() as u32;STEP+=1;},
        3=>{value.kind=abi::RESIZE;value.a=160;value.b=96;STEP=4;},
        5=>{value.kind=abi::CLOSE;STEP=6;},
        9=>{value.kind=abi::KEY;value.flags=abi::KEY_DOWN|abi::KEY_REPEAT;value.a='K' as u32;
            let mut pointer=pointer::Pointer::new();
            if !pointer.update(&mut *(&raw mut INPUT),40,40,1)
                || !pointer.update(&mut *(&raw mut INPUT),40,40,0) {finish(1,105);}
            value.b=abi::MOD_SHIFT|abi::MOD_CONTROL;
            let mut release=value;release.flags=0;
            if !(&mut *(&raw mut INPUT)).push(&[value,release]) {finish(1,104);}
            STEP=10;
            return (&mut *(&raw mut INPUT)).drain(1,|value|{out.write(value);true}) as u32;
        },
        11=>{value.kind=abi::NAVIGATE;let url=b"https://fixture.test/#second";
            value.text[..url.len()].copy_from_slice(url);value.length=url.len() as u32;STEP=12;},
        12 if LOCATION==2 && HISTORY&1!=0=>{value.kind=abi::BACK;STEP=13;},
        13 if LOCATION==1 && HISTORY&2!=0=>{value.kind=abi::FORWARD;STEP=14;},
        14 if LOCATION==2 && HISTORY&1!=0=>{value.kind=abi::NAVIGATE;
            let url=b"https://fixture.test/denied";value.text[..url.len()].copy_from_slice(url);
            value.length=url.len() as u32;STEP=15;},
        16=>{value.kind=abi::NAVIGATE;let url=b"https://fixture.test/recovery";
            value.text[..url.len()].copy_from_slice(url);value.length=url.len() as u32;STEP=17;},
        18=>{value.kind=abi::SHUTDOWN;STEP=19;},
        _=>return 0,
    }
    record(11,((STEP as u64)<<32)|value.kind as u64);
    out.write(value);1
}
// ------------------------=
// FUNC: frame
// DESC: Copies borrowed engine pixels into owned bounded compositor handoff storage.
// ------------------=
unsafe extern "C" fn frame(_: *mut c_void,width:u32,height:u32,bytes:*const u8,length:usize) {
    if length!=width as usize*height as usize*4 {finish(1,101);}
    let pixels=core::slice::from_raw_parts(bytes,length);
    if PIXELS.publish(1,width,height,pixels).is_err() {finish(1,102);}
}
// ------------------------=
// FUNC: event
// DESC: Treats component errors as failures, not successful UI status strings.
// ------------------=
unsafe extern "C" fn event(_: *mut c_void,kind:u32,value:u32,text:*const u8,length:usize) {
    if kind==abi::EVENT_ERROR {
        record(10,((STEP as u64)<<32)|value as u64);
        if STEP==15 && value==3 {STEP=16;} else {finish(1,200+value as u64);}
    }
    if kind==abi::EVENT_MEMORY {record(5,value as u64);}
    if kind==abi::EVENT_LOAD && value==0 {
        LOAD_STARTS|=match STEP {2=>1,8=>2,12=>4,_=>0};
    }
    if kind==abi::EVENT_HISTORY {
        if value>3 {finish(1,103);}
        HISTORY=value;
        record(6,((STEP as u64)<<32)|value as u64);
    }
    if kind==abi::EVENT_ADDRESS {
        LOCATION=match core::slice::from_raw_parts(text,length) {
            b"https://fixture.test/"=>1,b"https://fixture.test/#second"=>2,
            b"https://fixture.test/recovery"=>3,_=>0,
        };
        record(7,((STEP as u64)<<32)|LOCATION as u64);
    }
    if kind==abi::EVENT_DIAGNOSTIC {
        record(3,value as u64);
        for chunk in core::slice::from_raw_parts(text,length.min(1024)).chunks(8) {
            let mut bytes=[0;8];bytes[..chunk.len()].copy_from_slice(chunk);record(4,u64::from_le_bytes(bytes));
        }
    }
}
// ------------------------=
// FUNC: begin
// DESC: Grants only the injected fixture URL; this test makes no network-proof claim.
// ------------------=
unsafe extern "C" fn begin(_: *mut c_void,url:*const u8,length:usize)->u64 {
    record(8,((STEP as u64)<<32)|length as u64);
    if core::slice::from_raw_parts(url,length)==b"https://fixture.test/redirect" {
        if REDIRECT_ID!=0 {return 0;}
        NEXT_ID+=1;REDIRECT_ID=NEXT_ID;return NEXT_ID;
    }
    if !matches!(core::slice::from_raw_parts(url,length),b"https://fixture.test/"|b"https://fixture.test/recovery") {return 0;}
    NEXT_ID+=1;NEXT_ID
}
// ------------------------=
// FUNC: poll
// DESC: Returns bounded fixture bytes for real Servo HTML, JS and raster execution.
// ------------------=
unsafe extern "C" fn poll(_: *mut c_void,id:u64,out:*mut abi::Response)->u32 {
    if id==REDIRECT_ID {
        let headers=b"location: http://fixture.test/\r\nstrict-transport-security: max-age=3600\r\ncontent-type: text/html\r\n";
        out.write(abi::Response{status:301,headers:headers.as_ptr(),headers_length:headers.len(),body:HTML.as_ptr(),body_length:0});return 1;
    }
    out.write(abi::Response{status:200,headers:HEADERS.as_ptr(),headers_length:HEADERS.len(),body:HTML.as_ptr(),body_length:HTML.len()});1
}
// ------------------------=
// FUNC: cancel
// DESC: Counts released native response handles across window lifecycles.
// ------------------=
unsafe extern "C" fn cancel(_: *mut c_void,_:u64) {RELEASES+=1;}
// ------------------------=
// FUNC: fatal
// DESC: Converts unrecoverable component failure to a structured fixture failure.
// ------------------=
unsafe extern "C" fn fatal(_: *mut c_void,code:u32)->! {finish(1,300+code as u64)}
// ------------------------=
// FUNC: component_boot
// DESC: Executes the actual privately linked engine through its production C ABI.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn component_boot()->! {
    extern "C" {fn infinity_browser_private_infinity_browser_run(host:*mut abi::Host)->u32;}
    HOST.version=0;
    if infinity_browser_private_infinity_browser_run(core::ptr::addr_of_mut!(HOST))!=1 {finish(1,401);}
    HOST.version=abi::VERSION;
    let result=infinity_browser_private_infinity_browser_run(core::ptr::addr_of_mut!(HOST));
    if result!=0 || STEP!=19 || FRAMES!=4 || LOAD_STARTS!=7 || REDIRECT_ID==0 || RELEASES<5 || u64::from(RELEASES)!=NEXT_ID {finish(1,400+result as u64);}
    finish(0,4)
}
