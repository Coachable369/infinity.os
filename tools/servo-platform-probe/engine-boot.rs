#![no_std]
#![no_main]
extern crate std;
#[cfg(infinity_page_probe)]
#[path = "engine-page.rs"] mod engine_page;
#[cfg(infinity_page_probe)]
#[path = "../../sdk/infinity-browser-core/zoom.rs"] mod zoom_steps;
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
    ".balign 2048", "engine_vectors:", ".rept 16", "b engine_fault_entry", ".space 124", ".endr"
);
// ------------------------=
// FUNC: engine_fault_entry
// DESC: Saves every general register before Rust executes, then switches to an independent diagnostic stack.
// ------------------=
core::arch::global_asm!(r#"
.section .bss.engine_fault,"aw",%nobits
.balign 16
engine_fault_registers:
    .space 288
engine_fault_stack:
    .space 16384
.section .text.engine_fault_entry,"ax"
engine_fault_entry:
    msr tpidr_el1, x0
    adrp x0, engine_fault_registers
    add x0, x0, :lo12:engine_fault_registers
    stp x1, x2, [x0, #8]
    stp x3, x4, [x0, #24]
    stp x5, x6, [x0, #40]
    stp x7, x8, [x0, #56]
    stp x9, x10, [x0, #72]
    stp x11, x12, [x0, #88]
    stp x13, x14, [x0, #104]
    stp x15, x16, [x0, #120]
    stp x17, x18, [x0, #136]
    stp x19, x20, [x0, #152]
    stp x21, x22, [x0, #168]
    stp x23, x24, [x0, #184]
    stp x25, x26, [x0, #200]
    stp x27, x28, [x0, #216]
    stp x29, x30, [x0, #232]
    mrs x1, tpidr_el1
    str x1, [x0]
    mov x1, sp
    mrs x2, elr_el1
    stp x1, x2, [x0, #248]
    mrs x1, esr_el1
    mrs x2, far_el1
    stp x1, x2, [x0, #264]
    mrs x1, spsr_el1
    str x1, [x0, #280]
    adrp x1, engine_fault_stack
    add x1, x1, :lo12:engine_fault_stack
    add x1, x1, #4, lsl #12
    mov sp, x1
    b engine_fault
"#);
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
// FUNC: record_stack_diagnostic
// DESC: Encodes one worker's stack evidence as fixed numeric records without allocating or scheduling.
// ------------------=
fn record_stack_diagnostic(slot:usize, snapshot:infinity_servo_runtime_primitives::native::StackDiagnostic) {
    // 16 phases per slot: id, low, high, flags, untouched, used, bottom changes,
    // bottom bytes checked, four packed name words, saved SP, saved LR, LR present.
    let base = 0x800 + slot as u64 * 16;
    let valid = snapshot.flags & 2 != 0;
    let unknown = u64::MAX;
    for (field, value) in [
        snapshot.id, snapshot.low as u64, snapshot.high as u64, snapshot.flags,
        if valid { snapshot.untouched as u64 } else { unknown },
        if valid { (snapshot.high - snapshot.low - snapshot.untouched) as u64 } else { unknown },
        if valid { snapshot.bottom_changed as u64 } else { unknown },
        if valid { (snapshot.high - snapshot.low).min(64) as u64 } else { 0 },
    ].into_iter().enumerate() { record(2, base + field as u64, value); }
    for (index, chunk) in snapshot.name.chunks_exact(8).enumerate() {
        let mut word = [0u8; 8]; word.copy_from_slice(chunk);
        record(2, base + 8 + index as u64, u64::from_le_bytes(word));
    }
    record(2, base + 12, snapshot.saved_sp as u64);
    record(2, base + 13, snapshot.saved_lr.map_or(unknown, |lr| lr as u64));
    record(2, base + 14, u64::from(snapshot.saved_lr.is_some()));
}
// ------------------------=
// FUNC: engine_fault
// DESC: Emits the saved register frame and a bounded mapped worker-stack window before reporting failure.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn engine_fault(frame:*const u64)->! {
    for index in 0..36 {record(2,0x100+index as u64,frame.add(index).read());}
    let mut low=0usize;let mut high=0usize;
    if infinity_servo_runtime_primitives::native::infinity_std_stack_bounds(&mut low,&mut high)==0 {
        record(2,0x124,low as u64);record(2,0x125,high as u64);
        record(2,0x126,infinity_servo_runtime_primitives::native::infinity_std_thread_id());
    }
    let stack=frame.add(31).read();
    if option_env!("INFINITY_BROWSER_STACK_DIAGNOSTICS") == Some("1") {
        record(2,0x130,1); // Stack diagnostic schema version.
        record(2,0x132,infinity_servo_runtime_primitives::native::STACK_DIAGNOSTIC_PATTERN as u64);
        let count=Runtime::diagnostic_stacks(0x8000_0000,0xa000_0000,record_stack_diagnostic);
        record(2,0x131,count as u64);
    }
    // Never follow an arbitrary fault destination; even below-SP reads must stay
    // inside both the actual current worker lease and this fixture's heap grant.
    if low>=0x8000_0000 && low<high && high<=0xa000_0000
        && stack>=low as u64 && stack<=high as u64 && stack&7==0 {
        if option_env!("INFINITY_BROWSER_STACK_DIAGNOSTICS") == Some("1") {
            let count=((stack-low as u64)/8).min(32);
            let start=stack-count*8;
            record(2,0x127,start);record(2,0x128,count);
            for index in 0..count {
                record(2,0x140+index,((start+index*8) as *const u64).read_volatile());
            }
        }
        let count=((high as u64-stack)/8).min(128);
        record(2,0x129,count);
        for index in 0..count as usize {
            record(2,0x180+index as u64,(stack as *const u64).add(index).read_volatile());
        }
    }
    record(2,15,frame.add(30).read());
    finish(1,frame.add(32).read(),frame.add(33).read())
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
        if option_env!("INFINITY_BROWSER_FAULT_CAPTURE_TEST")==Some("1") {
            unsafe { core::arch::asm!(
                "mov x9, #7", "lsl x9, x9, #32", "mov x10, #0x1234", "str x10, [x9]",
                out("x9") _,out("x10") _);
            }
            finish(1,0x610,0);
        }
        extern "C" { static __init_start:usize;static __init_end:usize;fn infinity_sqlite_initialize()->i32; }
        unsafe {
            let mut entry=core::ptr::addr_of!(__init_start);
            while entry<core::ptr::addr_of!(__init_end) {
                let initialize:extern "C" fn()=core::mem::transmute(entry.read());initialize();entry=entry.add(1);
            }
            let status=infinity_sqlite_initialize();if status!=0 { finish(1,4,status as u64); }
        }
        record(2,4,0);
        #[path = "../../sdk/infinity-browser-servo/preferences.rs"]
        mod web_preferences;
        let engine=servo::ServoBuilder::default().preferences(web_preferences::web_preferences()).build();
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
