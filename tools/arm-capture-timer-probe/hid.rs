//! Native HDA capture under the same firmware HID calls made by the desktop.
use super::{boot_info, bytes, finish, hda, ticks, MAGIC, PCM};
#[cfg(not(feature = "reserve-ap"))]
use super::{PHASE,ACK};
#[cfg(feature = "reserve-ap")]
use super::RECORDS;
#[cfg(not(feature = "reserve-ap"))]
use core::sync::atomic::Ordering;

// ------------------------=
// FUNC: worker_idle
// DESC: Compares CPU7 event waits with a diagnostic timer-assisted interrupt wait; never used in production.
// ------------------=
pub unsafe fn worker_idle(target: bool, phase: usize) {
    if target && (phase == 3 || phase == 5) {
        let (ctl,cval,freq): (u64,u64,u64);
        core::arch::asm!("mrs {},cntv_ctl_el0",out(reg)ctl);
        core::arch::asm!("mrs {},cntv_cval_el0",out(reg)cval);
        core::arch::asm!("mrs {},cntfrq_el0",out(reg)freq);
        let deadline = ticks()+freq/500;
        core::arch::asm!("msr cntv_cval_el0,{}", "msr cntv_ctl_el0,{}", "isb", "dsb sy", "wfi",in(reg)deadline,in(reg)1u64);
        core::arch::asm!("msr cntv_ctl_el0,{}", "msr cntv_cval_el0,{}", "isb",in(reg)ctl,in(reg)cval);
    } else { core::arch::asm!("wfe",options(nomem,nostack)); }
}

#[repr(C)]
struct Pointers {
    check_event: usize,
    absolute_count: u32,
    relative_count: u32,
    usb_mouse_count: u32,
    reserved: u32,
    absolute: [usize; 8],
    relative: [usize; 8],
    usb_mouse: [usize; 8],
    usb_endpoint: [u8; 8],
    usb_mouse_absolute: [u8; 8],
    usb_keyboard_count: u32,
    reserved2: u32,
    usb_keyboard: [usize; 8],
    usb_keyboard_endpoint: [u8; 8],
    usb_mouse_async: u32,
    usb_mouse_sequence: u32,
    usb_mouse_length: u32,
    usb_mouse_report: [u8; 8],
}

// ------------------------=
// FUNC: cancel
// DESC: Reproduces the production fallback's cancellation of the boot mouse callback.
// ------------------=
unsafe fn cancel(p: &mut Pointers) {
    for i in 0..p.usb_mouse_count.min(8) as usize {
        let usb = p.usb_mouse[i];
        let address = *((usb as *const usize).add(2));
        if address != 0 {
            let call: unsafe extern "efiapi" fn(usize,u8,u8,usize,usize,usize,usize)->u64 = core::mem::transmute(address);
            let _ = call(usb,p.usb_endpoint[i],0,0,0,0,0);
        }
    }
    core::ptr::write_volatile(&raw mut p.usb_mouse_async,0);
}

// ------------------------=
// FUNC: raw_poll
// DESC: Performs the production bounded synchronous USB interrupt report requests.
// ------------------=
unsafe fn raw_poll(p: &Pointers) -> (u64,u64) {
    let (mut calls,mut reports) = (0,0);
    for i in 0..p.usb_mouse_count.min(8) as usize {
        let usb = p.usb_mouse[i];
        let address = *((usb as *const usize).add(3));
        if address == 0 { continue; }
        let call: unsafe extern "efiapi" fn(usize,u8,*mut u8,*mut usize,usize,*mut u32)->u64 = core::mem::transmute(address);
        let budget = if p.usb_mouse_absolute[i] != 0 {16} else {4};
        for _ in 0..budget {
            let mut data = [0u8;8]; let mut len = data.len(); let mut status = 0;
            calls += 1;
            if call(usb,p.usb_endpoint[i],data.as_mut_ptr(),&mut len,1,&mut status) != 0 || len < 3 { break; }
            reports += 1;
        }
    }
    (calls,reports)
}

// ------------------------=
// FUNC: protocol_poll
// DESC: Performs the production CheckEvent/GetState calls without dispatching pointer changes.
// ------------------=
unsafe fn protocol_poll(p: &Pointers) -> u64 {
    let mut calls = 0;
    for (items,count) in [(&p.absolute,p.absolute_count),(&p.relative,p.relative_count)] {
        for &address in items.iter().take(count.min(8) as usize) {
            let words = address as *const usize;
            if p.check_event != 0 {
                let check: unsafe extern "efiapi" fn(usize)->u64 = core::mem::transmute(p.check_event);
                let _ = check(*words.add(2));
            }
            let get: unsafe extern "efiapi" fn(usize,*mut u8)->u64 = core::mem::transmute(*words.add(1));
            let mut state = [0u64;4];
            let _ = get(address,state.as_mut_ptr().cast());
            calls += 1;
        }
    }
    calls
}

// ------------------------=
// FUNC: measure
// DESC: Compares idle capture with raw HID, bounded cadence and firmware-only polling without host input.
// ------------------=
pub unsafe fn measure(info: &boot_info::BootInfo, device: &mut hda::Hda, frequency: u64) -> ! {
    #[cfg(feature = "reserve-ap")]
    reserve_measure(info,device,frequency);
    #[cfg(not(feature = "reserve-ap"))]
    {
    assert_ne!(info.firmware_pointer,0);
    let p = &mut *(info.firmware_pointer as *mut Pointers);
    assert!(p.usb_mouse_count > 0);
    let header = [MAGIC,4,device.sample_rate as u64,frequency,p.absolute_count as u64,
        p.relative_count as u64,p.usb_mouse_count as u64,p.usb_mouse_async as u64];
    for value in header { bytes(&value.to_le_bytes()); }
    for phase in 0..6 {
        PHASE.store(phase as usize,Ordering::Release);
        core::arch::asm!("sev",options(nomem,nostack));
        let wait_start = ticks();
        while ACK.load(Ordering::Acquire) != phase as usize {
            if ticks()-wait_start > frequency*2 { finish(4); }
        }
        let begin = ticks(); let mut last_data = begin; let mut last_hid = begin;
        let (mut frames,mut max_gap,mut reads,mut calls,reports,max_call) = (0,0,0,0,0,0);
        let (waits,max_wait) = (0,0);
        let start_lpib = core::ptr::read_volatile((info.boot_reserved as usize+0x84) as *const u32);
        while ticks()-begin < frequency*8 {
            let now = ticks();
            let samples = device.read_capture(&mut *(&raw mut PCM)).unwrap();
            reads += 1;
            if samples > 0 { max_gap = max_gap.max(now-last_data); last_data = now; frames += samples as u64/2; }
            if phase == 1 && now-last_hid >= frequency/1000 {
                core::arch::asm!("sev",options(nomem,nostack)); last_hid = now; calls += 1;
            }
        }
        max_gap = max_gap.max(ticks()-last_data);
        let end_lpib = core::ptr::read_volatile((info.boot_reserved as usize+0x84) as *const u32);
        let row = [phase,begin,ticks(),frames,max_gap,reads,start_lpib as u64,end_lpib as u64,calls,reports,max_call,waits,max_wait];
        for value in row { bytes(&value.to_le_bytes()); }
    }
    device.stop_capture(); finish(0)
    }
}

// ------------------------=
// FUNC: reserve_measure
// DESC: Measures uninterrupted idle HID capture with the highest physical CPU left under firmware ownership.
// ------------------=
#[cfg(feature = "reserve-ap")]
unsafe fn reserve_measure(info: &boot_info::BootInfo, device: &mut hda::Hda, frequency: u64) -> ! {
    assert_ne!(info.firmware_pointer,0);
    let p = &mut *(info.firmware_pointer as *mut Pointers);
    let version = if cfg!(feature = "production-policy") {6} else {5};
    let header = [MAGIC,version,device.sample_rate as u64,frequency,p.absolute_count as u64,
        p.relative_count as u64,p.usb_mouse_count as u64,p.usb_mouse_async as u64];
    for value in header { bytes(&value.to_le_bytes()); }
    for i in 0..8 {
        let cpu = if i < 7 {RECORDS[i][0]&255} else {u64::MAX};
        if i < 7 {assert_ne!(cpu,7);}
        bytes(&cpu.to_le_bytes());
    }
    cancel(p);
    let begin = ticks(); let mut last_data = begin; let mut last_audio = 0;
    let (mut frames,mut max_gap,mut reads,mut calls,mut reports,mut max_call) = (0,0,0,0,0,0);
    let start_lpib = core::ptr::read_volatile((info.boot_reserved as usize+0x84) as *const u32);
    while ticks()-begin < frequency*40 {
        let now = ticks();
        if now-last_audio >= frequency/1000 {
            last_audio = now;
            let samples = device.read_capture(&mut *(&raw mut PCM)).unwrap(); reads += 1;
            if samples > 0 { max_gap = max_gap.max(now-last_data); last_data = now; frames += samples as u64/2; }
        }
        let start = ticks(); let result = raw_poll(p);
        calls += result.0+protocol_poll(p); reports += result.1; max_call = max_call.max(ticks()-start);
    }
    max_gap = max_gap.max(ticks()-last_data);
    let end_lpib = core::ptr::read_volatile((info.boot_reserved as usize+0x84) as *const u32);
    let row = [0,begin,ticks(),frames,max_gap,reads,start_lpib as u64,end_lpib as u64,calls,reports,max_call,0,0];
    for value in row { bytes(&value.to_le_bytes()); }
    device.stop_capture(); finish(0)
}
