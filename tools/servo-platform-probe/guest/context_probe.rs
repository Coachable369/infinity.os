//! Shared stack-isolation behavior; only FP register access is target-specific.
use infinity_servo_runtime_primitives::context::{Context, switch};
use infinity_servo_runtime_primitives::wait::{Waits, Outcome};
use core::sync::atomic::AtomicU32;
#[repr(align(16))]
struct Stack([u8; 32768]);
static mut STACK_A: Stack = Stack([0; 32768]);
static mut STACK_B: Stack = Stack([0; 32768]);
static mut MAIN: Context = Context::empty();
static mut A: Context = Context::empty();
static mut B: Context = Context::empty();
static mut COUNTS: [u64; 2] = [0; 2];
static WORDS: [AtomicU32; 2] = [AtomicU32::new(0), AtomicU32::new(0)];
static mut WAITS: Waits<2> = Waits::new();

// ------------------------=
// FUNC: rounding
// DESC: Reads thread-sensitive floating-point control bits for isolation assertions.
// ------------------=
unsafe fn rounding() -> u64 {
    #[cfg(target_arch = "aarch64")]
    { let value: u64; core::arch::asm!("mrs {}, fpcr", out(reg) value); (value >> 22) & 3 }
    #[cfg(target_arch = "x86_64")]
    { let mut value = 0u32; core::arch::asm!("stmxcsr [{}]", in(reg) &mut value); ((value >> 13) & 3) as u64 }
}
// ------------------------=
// FUNC: set_rounding
// DESC: Sets distinct control state on each independently suspended stack.
// ------------------=
unsafe fn set_rounding(value: u64) {
    #[cfg(target_arch = "aarch64")]
    { core::arch::asm!("msr fpcr, {}", in(reg) value << 22); }
    #[cfg(target_arch = "x86_64")]
    { let value = 0x1f80u32 | ((value as u32) << 13); core::arch::asm!("ldmxcsr [{}]", in(reg) &value); }
}
// ------------------------=
// FUNC: worker
// DESC: Retains private stack bytes and FP state across a thousand resumptions, then signals completion.
// ------------------=
unsafe fn worker(index: usize, own: *mut Context) -> ! {
    let mut canary = [0u64; 64];
    for (offset, value) in canary.iter_mut().enumerate() {
        core::ptr::write_volatile(value, (index as u64 + 1) * 1000 + offset as u64);
    }
    set_rounding(index as u64 + 1);
    let counter = core::ptr::addr_of_mut!(COUNTS).cast::<u64>().add(index);
    for step in 0..1000 {
        assert_eq!(rounding(), index as u64 + 1);
        for (offset, value) in canary.iter().enumerate() {
            assert_eq!(core::ptr::read_volatile(value), (index as u64 + 1) * 1000 + offset as u64);
        }
        counter.write_volatile(step + 1);
        assert_eq!((*core::ptr::addr_of_mut!(WAITS)).prepare(index as u64 + 1,
            &WORDS[index], 0, None, 0), Ok(None));
        switch(own, core::ptr::addr_of!(MAIN));
        assert_eq!((*core::ptr::addr_of_mut!(WAITS)).take(index as u64 + 1), Some(Outcome::Woken));
    }
    counter.write_volatile(1001);
    switch(own, core::ptr::addr_of!(MAIN));
    panic!("completed context resumed")
}
// ------------------------=
// FUNC: entry_a
// DESC: Enters the first independent worker stack.
// ------------------=
extern "C" fn entry_a() -> ! { unsafe { worker(0, core::ptr::addr_of_mut!(A)) } }
// ------------------------=
// FUNC: entry_b
// DESC: Enters the second independent worker stack.
// ------------------=
extern "C" fn entry_b() -> ! { unsafe { worker(1, core::ptr::addr_of_mut!(B)) } }
// ------------------------=
// FUNC: run
// DESC: Alternates real machine contexts and validates stack/control isolation before reporting exact resumption count.
// ------------------=
pub unsafe fn run() -> u64 {
    let a = core::ptr::addr_of_mut!(A);
    let b = core::ptr::addr_of_mut!(B);
    assert!((*a).initialize(core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(STACK_A.0).cast(), 32768), entry_a));
    assert!((*b).initialize(core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(STACK_B.0).cast(), 32768), entry_b));
    set_rounding(0);
    for step in 0..1001 {
        if step != 0 {
            assert_eq!((*core::ptr::addr_of_mut!(WAITS)).wake(&WORDS[0], false), 1);
            assert_eq!((*core::ptr::addr_of_mut!(WAITS)).wake(&WORDS[1], false), 1);
        }
        switch(core::ptr::addr_of_mut!(MAIN), a);
        assert_eq!(rounding(), 0);
        switch(core::ptr::addr_of_mut!(MAIN), b);
        assert_eq!(rounding(), 0);
        for index in 0..2 {
            assert_eq!(core::ptr::addr_of!(COUNTS).cast::<u64>().add(index).read_volatile(), step + 1);
        }
    }
    2002
}
