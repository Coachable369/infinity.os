#![no_std]
#![allow(dead_code)]
#[path = "../../../kernel/drivers/hda.rs"] mod hda;
static mut DMA: hda::Dma = hda::Dma::new();
static mut SPEECH: [i16; 240000] = [0; 240000];
static mut PCM: [i16; hda::SAMPLES] = [0; hda::SAMPLES];
unsafe extern "C" {
    fn infinity_flite_synthesize(text: *const u8, length: usize, pcm: *mut i16, capacity: usize,
        frames: *mut usize, peak: *mut usize, cancel: usize, limit: usize) -> i32;
    fn infinity_flite_clean() -> i32;
}
core::arch::global_asm!(".section .text.entry", ".global _start", "_start:",
    "ldr x0, =0x48000000", "mov sp, x0", "mov x0, #(3 << 20)",
    "msr cpacr_el1, x0", "adr x0, exception_vectors", "msr vbar_el1, x0", "isb", "bl probe", "b .",
    ".balign 2048", "exception_vectors:", ".rept 16", ".balign 128", "b exception", ".endr");
// ------------------------=
// FUNC: exception
// DESC: Emits architectural fault registers as binary evidence before shutting down the test guest.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn exception() -> ! {
    let esr:u64;let elr:u64;let far:u64;
    core::arch::asm!("mrs {}, esr_el1",out(reg)esr);
    core::arch::asm!("mrs {}, elr_el1",out(reg)elr);
    core::arch::asm!("mrs {}, far_el1",out(reg)far);
    report(&[0,esr,elr,far,0,0]);finish(0)
}
// ------------------------=
// FUNC: ticks
// DESC: Reads the guest architectural timer for synthesis and DMA deadline measurements.
// ------------------=
fn ticks() -> u64 { let n; unsafe {core::arch::asm!("mrs {}, cntvct_el0",out(reg)n);} n }
// ------------------------=
// FUNC: finish
// DESC: Reports assertion failures as machine-readable guest exit status.
// ------------------=
fn finish(code: u64) -> ! {unsafe{if code!=0{report(&[0,code,0,0,0,0]);}core::arch::asm!("hvc #0",in("x0")0x84000008u64);}loop{core::hint::spin_loop();}}
// ------------------------=
// FUNC: panic
// DESC: Turns failed guest assertions into a failing VM process rather than text-based evidence.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {finish(99)}
// ------------------------=
// FUNC: fill
// DESC: Converts 8-kHz speech to the HDA output rate using bounded interpolation and silence padding.
// ------------------=
unsafe fn fill(output: &mut [i16], start: usize, frames: usize, rate: u32) {
    assert!(speech_pcm::fill(&(&*(&raw const SPEECH))[..frames], output, start, rate));
}
#[path = "../../../kernel/drivers/speech_pcm.rs"]
mod speech_pcm;
// ------------------------=
// FUNC: report
// DESC: Writes binary metrics to the test UART, never performing guest speech on the host.
// ------------------=
unsafe fn report(values: &[u64]) {
    for value in values {for byte in value.to_le_bytes(){
        while core::ptr::read_volatile(0x09000018 as *const u32)&32!=0 {core::hint::spin_loop();}
        core::ptr::write_volatile(0x09000000 as *mut u32,byte as u32);
    }}
}
// ------------------------=
// FUNC: probe
// DESC: Synthesizes real text without any OS runtime and streams the result through native HDA DMA.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn probe() -> ! {
    use core::ptr::{read_volatile,write_volatile};
    let text=b"Hello. I am the native voice of Infinity OS. How can I help you?";
    let frequency:u64;core::arch::asm!("mrs {}, cntfrq_el0",out(reg)frequency);
    let mut frames=0;let mut peak=0;let started=ticks();
    assert_eq!(infinity_flite_synthesize(text.as_ptr(),text.len(),(&raw mut SPEECH).cast(),240000,&mut frames,&mut peak,0,8*1024*1024),0);
    let synthesis=ticks()-started;
    assert!(frames>8000&&frames<240000);assert!(peak<8*1024*1024);
    assert_eq!(infinity_flite_clean(),1);
    let mut base=0;
    for slot in 0..32usize {let cfg=(0x4010000000usize+(slot<<15)) as *mut u32;if read_volatile(cfg)==0x26688086 {write_volatile(cfg.add(4),0x10000000);write_volatile((cfg as usize+4) as *mut u16,6);base=0x10000000;}}
    assert_ne!(base,0);
    let mut device=hda::Hda::initialize(base,&raw mut DMA).unwrap();
    static mut RESIDENT: [i16; 48_000*2*32] = [0;48_000*2*32];
    let count = (frames*device.sample_rate as usize+7999)/8000*2;
    fill(&mut (&mut *(&raw mut RESIDENT))[..count],0,frames,device.sample_rate);
    device.start_resident(&(&*(&raw const RESIDENT))[..device.sample_rate as usize*2*32]).unwrap();
    let output_stream = 0x80 + ((read_volatile(base as *const u16) >> 8) as usize & 15) * 0x20;
    assert_eq!(read_volatile((base + output_stream + 0x0c) as *const u16), 1);
    let mut polls=0;
    let began=ticks();
    while (device.position().unwrap() as usize)<count*2 {
        let before=ticks();
        // Deliberately miss several former refill deadlines, like a slow desktop frame.
        while ticks()-before<frequency*120/1000 { core::hint::spin_loop(); }
        assert!(ticks()-began<frequency*10);
        polls+=1;
    }
    device.stop();
    assert!(polls>20);
    report(&[1,frames as u64,peak as u64,synthesis,frequency,polls]);
    finish(0)
}
// ------------------------=
// FUNC: memset
// DESC: Supplies freestanding compiler memory initialization.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memset(p:*mut core::ffi::c_void,v:i32,n:usize)->*mut core::ffi::c_void {for i in 0..n{core::ptr::write_volatile(p.cast::<u8>().add(i),v as u8);}p}
// ------------------------=
// FUNC: memcpy
// DESC: Supplies freestanding compiler byte copies.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memcpy(p:*mut core::ffi::c_void,s:*const core::ffi::c_void,n:usize)->*mut core::ffi::c_void {for i in 0..n{core::ptr::write_volatile(p.cast::<u8>().add(i),core::ptr::read_volatile(s.cast::<u8>().add(i)));}p}
