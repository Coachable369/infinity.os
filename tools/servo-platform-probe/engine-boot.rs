#![no_std]
#![no_main]
extern crate std;
#[cfg(infinity_page_probe)]
#[path = "engine-page.rs"] mod engine_page;
use core::mem::MaybeUninit;
use infinity_servo_runtime_primitives::native::{Runtime,Hooks};
#[path="guest/entropy_probe.rs"] mod entropy_probe;
static mut RUNTIME: MaybeUninit<Runtime> = MaybeUninit::uninit();
core::arch::global_asm!(include_str!("guest/aarch64-memory.S"));
core::arch::global_asm!(
    ".section .text.entry", ".global _start", "_start:",
    "ldr x0, =0x7ff00000", "mov sp, x0", "mov x0, #(3 << 20)",
    "msr cpacr_el1, x0", "adr x0, engine_vectors", "msr vbar_el1, x0", "isb",
    "bl probe_memory", "bl engine_boot", "b .",
    ".balign 2048", "engine_vectors:", ".rept 16", "b engine_fault", ".space 124", ".endr"
);
// ------------------------=
// FUNC: record
// DESC: Emits structured boot checkpoints through the guest UART, not a host runtime.
// ------------------=
fn record(status:u64, phase:u64, detail:u64) {
    unsafe { for value in [9,status,phase,detail] { for byte in value.to_le_bytes() {
        while (0x09000018 as *const u32).read_volatile() & 32 != 0 {}
        (0x09000000 as *mut u32).write_volatile(byte as u32);
    } } }
}
// ------------------------=
// FUNC: finish
// DESC: Records actual success or failure before powering down this disposable guest.
// ------------------=
fn finish(status:u64,phase:u64,detail:u64)->! {
    record(status,phase,detail);
    unsafe { core::arch::asm!("hvc #0",in("x0") 0x84000008u64); }
    loop { core::hint::spin_loop(); }
}
// ------------------------=
// FUNC: engine_fault
// DESC: Captures a real synchronous hardware exception as a failed boot result.
// ------------------=
#[no_mangle]
pub extern "C" fn engine_fault()->! {
    let pc:u64;let esr:u64;let caller:u64;
    unsafe { core::arch::asm!("mrs {}, elr_el1","mrs {}, esr_el1","mov {}, x30",out(reg) pc,out(reg) esr,out(reg) caller); }
    record(2, 15, caller);
    finish(1,pc,esr)
}
// ------------------------=
// FUNC: cpu
// DESC: Reads the native CPU identity used for ownership enforcement.
// ------------------=
fn cpu()->u64 { let id:u64;unsafe { core::arch::asm!("mrs {}, mpidr_el1",out(reg) id); } id & 0xff00ffffff }
// ------------------------=
// FUNC: monotonic
// DESC: Converts the architectural timer using its actual frequency.
// ------------------=
fn monotonic()->u64 {
    let ticks:u64;let frequency:u64;
    unsafe { core::arch::asm!("isb","mrs {}, cntvct_el0","mrs {}, cntfrq_el0",out(reg) ticks,out(reg) frequency); }
    (ticks as u128*1_000_000_000/frequency as u128) as u64
}
// ------------------------=
// FUNC: utc
// DESC: Reads QEMU's native PL031 RTC; no fabricated time is supplied.
// ------------------=
fn utc()->Option<(u64,u32)> { Some((unsafe { (0x09010000 as *const u32).read_volatile() } as u64,0)) }
// ------------------------=
// FUNC: pump
// DESC: Keeps the disposable owner cooperative; production event integration is not claimed.
// ------------------=
fn pump() { core::hint::spin_loop(); }
// ------------------------=
// FUNC: engine_boot
// DESC: Initializes native providers and invokes real Servo initialization on a registered worker stack.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn engine_boot()->! {
    record(2,1,0);
    let runtime=core::ptr::addr_of_mut!(RUNTIME).cast::<Runtime>();
    let heap=core::slice::from_raw_parts_mut(0x80000000 as *mut u8,512*1024*1024);
    runtime.write(Runtime::new(heap,Hooks {cpu,monotonic,utc,entropy:entropy_probe::fill,pump}).unwrap());
    if !Runtime::install(runtime) { finish(1,2,0); }
    std::panic::set_hook(std::boxed::Box::new(|info| {
        let line=info.location().map_or(0,|location| location.line());
        if let Some(location)=info.location() { diagnostic(6,location.file()); }
        if let Some(message)=info.payload().downcast_ref::<&str>() { diagnostic(7,message); }
        if let Some(message)=info.payload().downcast_ref::<std::string::String>() { diagnostic(7,message); }
        finish(1,3,line as u64);
    }));
    record(2,2,0);
    std::thread::Builder::new().stack_size(4*1024*1024).spawn(|| {
        extern "C" { static __init_start:usize;static __init_end:usize;fn infinity_sqlite_initialize()->i32; }
        unsafe {
            let mut entry=core::ptr::addr_of!(__init_start);
            while entry<core::ptr::addr_of!(__init_end) {
                let initialize:extern "C" fn()=core::mem::transmute(entry.read());initialize();entry=entry.add(1);
            }
            let status=infinity_sqlite_initialize();if status!=0 { finish(1,4,status as u64); }
        }
        record(2,4,0);
        let engine=servo::ServoBuilder::default().build();
        record(2,5,unsafe {Runtime::allocated()} as u64);
        #[cfg(infinity_swgl_probe)]
        {
            let failure = infinity_swgl_probe::verify_clear();
            if failure != 0 { finish(1,8,failure); }
            record(2,8,1);
        }
        #[cfg(infinity_page_probe)]
        {
            let failure = engine_page::verify(&engine);
            if failure != 0 { finish(1,9,failure); }
            record(2,9,1);
        }
        core::mem::forget(engine);
    }).unwrap().join().unwrap();
    finish(0,5,Runtime::allocated() as u64)
}
// ------------------------=
// FUNC: diagnostic
// DESC: Emits bounded panic context without allocation; it is diagnostic data, not a pass oracle.
// ------------------=
fn diagnostic(kind:u64,text:&str) {
    for chunk in text.as_bytes()[..text.len().min(512)].chunks(8) {
        let mut bytes=[0u8;8]; bytes[..chunk.len()].copy_from_slice(chunk);
        record(4,kind,u64::from_le_bytes(bytes));
    }
}
// ------------------------=
// FUNC: infinity_probe_fatal
// DESC: Records bounded native C failure diagnostics, then powers down the failed fixture.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_probe_fatal(line:u64,file:*const u8,condition:*const u8,caller:u64)->! {
    for (kind, pointer) in [(6,file),(7,condition)] {
        if !pointer.is_null() {
            let mut length=0;
            while length<512 && pointer.add(length).read()!=0 { length+=1; }
            if let Ok(text)=core::str::from_utf8(core::slice::from_raw_parts(pointer,length)) {
                diagnostic(kind,text);
            }
        }
    }
    record(2,10,caller);
    finish(1,10,line)
}
// ------------------------=
// FUNC: infinity_probe_diagnostic
// DESC: Emits bounded C diagnostics separately from the structured acceptance result.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_probe_diagnostic(pointer:*const u8) {
    if pointer.is_null() { return; }
    let mut length=0;
    while length<512 && pointer.add(length).read()!=0 { length+=1; }
    if let Ok(text)=core::str::from_utf8(core::slice::from_raw_parts(pointer,length)) { diagnostic(7,text); }
}
// ------------------------=
// FUNC: infinity_probe_bytes
// DESC: Reports bounded C diagnostic byte slices without requiring a terminating NUL.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_probe_bytes(pointer:*const u8,length:usize) {
    if pointer.is_null() { return; }
    if let Ok(text)=core::str::from_utf8(core::slice::from_raw_parts(pointer,length.min(512))) {
        diagnostic(7,text);
    }
}
