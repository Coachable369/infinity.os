//! Native engineering/operator entrypoint. Never compiled into the installed OS.
#![no_std]
#[path = "../../kernel/core/boot_info.rs"]
mod boot_info;
#[path = "../../kernel/drivers/e1000.rs"]
mod e1000;
#[path = "../wire-trust-fixture.rs"]
mod fixture;
#[path = "../../kernel/core/memory.rs"]
mod memory;
mod operator;
mod runtime;
use runtime::network::types::Packet;

static mut FIXTURE: core::mem::MaybeUninit<fixture::Fixture> = core::mem::MaybeUninit::uninit();
static mut COMMITTED_STATE: [u8; runtime::node::types::NODE_STATE_BYTES] = [0; runtime::node::types::NODE_STATE_BYTES];

// ------------------------=
// FUNC: engineering_commit
// DESC: Stores the actual binary commit in explicit engineering RAM storage; this is not an installed durability claim.
// ------------------=
fn engineering_commit(bytes: &[u8; runtime::node::types::NODE_STATE_BYTES]) -> bool {
    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), (&raw mut COMMITTED_STATE).cast::<u8>(), bytes.len()); }
    true
}

#[inline(never)]
// ------------------------=
// FUNC: initialize_fixture
// DESC: Constructs the bounded engineering runtime outside the event-loop stack so durable transaction frames cannot overwrite boot page tables.
// ------------------=
unsafe fn initialize_fixture(mac: [u8; 6], entropy: [u8; 32]) -> &'static mut fixture::Fixture {
    let pointer = (&raw mut FIXTURE).cast::<fixture::Fixture>();
    pointer.write(fixture::configured(mac, entropy));
    &mut *pointer
}

// ------------------------=
// FUNC: exit
// DESC: Reports a binary engineering outcome through QEMU debug-exit.
// ------------------=
pub fn exit(value: u32) -> ! {
    unsafe {
        core::arch::asm!("out dx, eax",in("dx")0xf4u16,in("eax")value);
    }
    loop {
        core::hint::spin_loop();
    }
}
#[panic_handler]
// ------------------------=
// FUNC: panic
// DESC: Distinguishes a guest panic from a protocol rejection using binary exit status.
// ------------------=
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    exit(0x12)
}
#[no_mangle]
// ------------------------=
// FUNC: memcpy
// DESC: Supplies freestanding production memory copy.
// ------------------=
pub unsafe extern "C" fn memcpy(
    d: *mut core::ffi::c_void,
    s: *const core::ffi::c_void,
    n: usize,
) -> *mut core::ffi::c_void {
    memory::copy(d.cast(), s.cast(), n);
    d
}
#[no_mangle]
// ------------------------=
// FUNC: memset
// DESC: Supplies bounded freestanding memory fill.
// ------------------=
pub unsafe extern "C" fn memset(
    d: *mut core::ffi::c_void,
    v: i32,
    n: usize,
) -> *mut core::ffi::c_void {
    for i in 0..n {
        d.cast::<u8>().add(i).write_volatile(v as u8);
    }
    d
}
#[no_mangle]
// ------------------------=
// FUNC: memcmp
// DESC: Supplies bounded freestanding memory comparison.
// ------------------=
pub unsafe extern "C" fn memcmp(
    a: *const core::ffi::c_void,
    b: *const core::ffi::c_void,
    n: usize,
) -> i32 {
    for i in 0..n {
        let delta = a.cast::<u8>().add(i).read() as i32 - b.cast::<u8>().add(i).read() as i32;
        if delta != 0 {
            return delta;
        }
    }
    0
}
#[no_mangle]
// ------------------------=
// FUNC: bcmp
// DESC: Supplies the compiler's freestanding equality comparison alias.
// ------------------=
pub unsafe extern "C" fn bcmp(
    a: *const core::ffi::c_void,
    b: *const core::ffi::c_void,
    n: usize,
) -> i32 {
    memcmp(a, b, n)
}

#[no_mangle]
// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Runs actual NIC and production node transport with independently generated firmware identities and explicit external operator actions.
// ------------------=
pub extern "C" fn infinity_kernel_entry(info: *const boot_info::BootInfo) -> ! {
    let Some(info) = (unsafe { info.as_ref() }) else {
        exit(0x13);
    };
    if info.magic != boot_info::BOOT_MAGIC || info.firmware_entropy_valid != 1 {
        exit(0x14);
    }
    let Some(mut nic) = (unsafe { e1000::E1000::initialize() }) else {
        exit(0x15);
    };
    if nic.mac[5] != 1 && nic.mac[5] != 2 {
        exit(0x16);
    }
    let Some(start) = nic.reference_clock_ns() else {
        exit(0x17);
    };
    let f = unsafe { initialize_fixture(nic.mac, info.firmware_entropy) };
    let mut op = operator::Operator::new();
    operator::Operator::ready();
    let mut last_tx: Option<Packet> = None;
    let mut last_rx: Option<Packet> = None;
    let mut saved: Option<Packet> = None;
    let mut frame = [0u8; e1000::MAX_FRAME];
    let mut next = 0;
    loop {
        let ns = nic.reference_clock_ns().unwrap().saturating_sub(start);
        let now = ns / 1_000_000_000;
        if now > 1100 {
            exit(0x18);
        }
        if ns >= next {
            next = ns.saturating_add(1_000_000);
            for _ in 0..4 {
                let Some(length) = nic.receive(&mut frame) else {
                    break;
                };
                if length != 0 {
                    let _ = f.network.wire.ingest(&frame[..length], now);
                }
            }
            for _ in 0..4 {
                let Some(packet) = f.network.wire.receive_datagram() else {
                    break;
                };
                if packet.length >= 154 && packet.bytes[8] == 11 {
                    last_rx = Packet::from_slice(&packet.bytes[..packet.length]).ok();
                }
                let _ = f.network.connections.deliver_datagram(
                    packet,
                    &mut f.network.policy,
                    &f.capabilities,
                    now,
                );
            }
            f.transport
                .poll(&mut f.nodes, &mut f.network, &f.capabilities, now);
            f.iop.poll_remote_node(&f.capabilities, &mut f.nodes, &mut f.transport.trust, now);
            if f.execute_remote { f.iop.execute_remote_node_durable(&mut f.nodes, now, &mut engineering_commit); }
            for _ in 0..4 {
                let Some(frame) = f.network.wire.peek_transmit() else {
                    break;
                };
                if !nic.transmit(&frame.bytes[..frame.length]) {
                    break;
                }
                if frame.length >= 196 && frame.bytes[23] == 17 && frame.bytes[50] == 11 {
                    last_tx = Packet::from_slice(&frame.bytes[42..frame.length]).ok();
                }
                f.network.wire.complete_transmit();
            }
        }
        op.poll(f, now, &last_tx, &last_rx, &mut saved);
        core::hint::spin_loop();
    }
}
