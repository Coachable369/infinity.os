//! NATIVE NIC ENGINE TEST ONLY. Not an installed System Generation or M9 acceptance.
#![no_std]
#[path = "../../kernel/drivers/e1000.rs"] mod e1000;
#[path = "../../kernel/core/memory.rs"] mod memory;
mod runtime;
mod fixture;

// ------------------------=
// FUNC: exit
// DESC: Returns a binary fixture outcome through QEMU's debug-exit device.
// ------------------=
fn exit(value: u32) -> ! { unsafe { core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") value); } loop { core::hint::spin_loop(); } }
#[panic_handler]
// ------------------------=
// FUNC: panic
// DESC: Reports a kernel fixture failure without interpreting diagnostic prose.
// ------------------=
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { exit(0x12) }
#[no_mangle]
// ------------------------=
// FUNC: memcpy
// DESC: Supplies the production freestanding memory-copy primitive to the probe.
// ------------------=
pub unsafe extern "C" fn memcpy(destination: *mut core::ffi::c_void, source: *const core::ffi::c_void, size: usize) -> *mut core::ffi::c_void { memory::copy(destination.cast(), source.cast(), size); destination }
#[no_mangle]
// ------------------------=
// FUNC: memset
// DESC: Supplies a volatile bounded fill to the freestanding fixture.
// ------------------=
pub unsafe extern "C" fn memset(destination: *mut core::ffi::c_void, value: i32, size: usize) -> *mut core::ffi::c_void { for i in 0..size { destination.cast::<u8>().add(i).write_volatile(value as u8); } destination }
#[no_mangle]
// ------------------------=
// FUNC: memcmp
// DESC: Supplies bounded byte comparison for checked packet parsing.
// ------------------=
pub unsafe extern "C" fn memcmp(a: *const core::ffi::c_void, b: *const core::ffi::c_void, size: usize) -> i32 { for i in 0..size { let delta = a.cast::<u8>().add(i).read() as i32 - b.cast::<u8>().add(i).read() as i32; if delta != 0 { return delta; } } 0 }

#[no_mangle]
// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Exercises two independently executing NIC engines using real ARP and bidirectional UDP on the QEMU link.
// ------------------=
pub extern "C" fn infinity_kernel_entry(_: *const u8) -> ! {
    let Some(mut nic) = (unsafe { e1000::E1000::initialize() }) else { exit(0x13); };
    let Some(start) = nic.reference_clock_ns() else { exit(0x14); };
    let host = nic.mac[5];
    if host != 1 && host != 2 { exit(0x15); }
    let own = [10, 42, 0, host]; let peer = [10, 42, 0, 3-host];
    let fixture::Fixture { mut network, capabilities, owner, connection, send, receive } = fixture::configured(nic.mac, own, peer);
    let mut frame = [0u8; e1000::MAX_FRAME];
    let mut requests = 0; let mut replies = 0; let mut next_send = 0;
    loop {
        let now = nic.reference_clock_ns().unwrap().saturating_sub(start);
        if now >= 8_000_000_000 {
            let (rx, tx, _) = nic.statistics();
            exit(if requests >= 3 && replies >= 3 && rx >= 6 && tx >= 6 { 0x10 } else { 0x11 });
        }
        if now >= next_send {
            next_send = now.saturating_add(100_000_000);
            let _ = network.send_datagram(owner, send, connection, &[1, host, 0x91, 0x29], now / 1_000_000_000, 8, &capabilities);
        }
        for _ in 0..4 { let Some(size) = nic.receive(&mut frame) else { break; }; if size != 0 { let _ = network.wire.ingest(&frame[..size], now / 1_000_000_000); } }
        for _ in 0..4 {
            let Some(packet) = network.wire.receive_datagram() else { break; };
            if packet.source != peer || packet.destination != own || packet.source_port != 49152 || packet.destination_port != 49152 || packet.length != 4 || packet.bytes[1] != 3-host || packet.bytes[2..4] != [0x91, 0x29] { exit(0x16); }
            network.connections.deliver_datagram(packet, &mut network.policy, &capabilities, now / 1_000_000_000).unwrap();
            let packet = network.receive_datagram(owner, receive, connection, now / 1_000_000_000, &capabilities).unwrap();
            if packet.bytes[0] == 1 { requests += 1; let _ = network.send_datagram(owner, send, connection, &[2, host, 0x91, 0x29], now / 1_000_000_000, 8, &capabilities); }
            else if packet.bytes[0] == 2 { replies += 1; }
            else { exit(0x17); }
        }
        for _ in 0..4 {
            let Some(packet) = network.wire.peek_transmit() else { break; };
            if !nic.transmit(&packet.bytes[..packet.length]) { break; }
            network.wire.complete_transmit();
        }
        core::hint::spin_loop();
    }
}
