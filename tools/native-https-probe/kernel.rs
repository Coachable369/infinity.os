//! Bare-metal HTTPS mechanism proof, not installed-system acceptance.
#![no_std]
#[path = "../../kernel/core/boot_info.rs"]
mod boot_info;
#[path = "../../kernel/drivers/e1000.rs"]
mod e1000;
#[path = "../../kernel/core/memory.rs"]
mod memory;
use core::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use infinity_http::{
    client::{self, Configuration, Destination, Link},
    https::Buffers,
    smoltcp::time::Instant,
};
use rand_core::SeedableRng;
static mut READ: [u8; 16640] = [0; 16640];
static mut WRITE: [u8; 4096] = [0; 4096];
static mut REQUEST: [u8; 512] = [0; 512];
static mut RESPONSE: [u8; 4096] = [0; 4096];
// ------------------------=
// FUNC: exit
// DESC: Reports structured guest success or failure without a log-string oracle.
// ------------------=
fn exit(code: u32) -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") code);
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let status = [0x20026u64, if code == 0x10 { 0 } else { code as u64 }];
        core::arch::asm!("hlt #0xf000", in("x0") 0x20u64, in("x1") status.as_ptr(), options(nostack));
    }
    loop {
        core::hint::spin_loop();
    }
}
#[panic_handler]
// ------------------------=
// FUNC: panic
// DESC: Makes every assertion failure observable through the guest exit status.
// ------------------=
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    exit(0x12)
}
#[no_mangle]
// ------------------------=
// FUNC: memcpy
// DESC: Supplies the production freestanding byte copy.
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
// FUNC: memmove
// DESC: Supplies the production overlapping byte copy.
// ------------------=
pub unsafe extern "C" fn memmove(
    d: *mut core::ffi::c_void,
    s: *const core::ffi::c_void,
    n: usize,
) -> *mut core::ffi::c_void {
    memory::move_bytes(d.cast(), s.cast(), n);
    d
}
#[no_mangle]
// ------------------------=
// FUNC: memset
// DESC: Supplies the production byte fill.
// ------------------=
pub unsafe extern "C" fn memset(
    d: *mut core::ffi::c_void,
    value: i32,
    n: usize,
) -> *mut core::ffi::c_void {
    memory::fill(d.cast(), value as u8, n);
    d
}
#[no_mangle]
// ------------------------=
// FUNC: memcmp
// DESC: Compares protocol and decrypted response bytes.
// ------------------=
pub unsafe extern "C" fn memcmp(
    a: *const core::ffi::c_void,
    b: *const core::ffi::c_void,
    n: usize,
) -> i32 {
    for i in 0..n {
        let d = a.cast::<u8>().add(i).read() as i32 - b.cast::<u8>().add(i).read() as i32;
        if d != 0 {
            return d;
        }
    }
    0
}
// ------------------------=
// FUNC: noop
// DESC: Uses a single-threaded probe poller rather than scheduling host work.
// ------------------=
unsafe fn noop(_: *const ()) {}
// ------------------------=
// FUNC: clone_waker
// DESC: Clones the stateless probe waker without allocating.
// ------------------=
unsafe fn clone_waker(_: *const ()) -> RawWaker {
    RawWaker::new(core::ptr::null(), &VTABLE)
}
static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker, noop, noop, noop);
impl Link for e1000::E1000 {
    // ------------------------=
    // FUNC: register_waker
    // DESC: Uses the explicit bare-metal probe poller; production adapters must arrange timer and NIC wakes.
    // ------------------=
    fn register_waker(&mut self, _: &Waker) {}
    // ------------------------=
    // FUNC: now
    // DESC: Reads the native adapter clock for the shared client operation.
    // ------------------=
    fn now(&self) -> Instant {
        Instant::from_millis((self.reference_clock_ns().unwrap() / 1_000_000) as i64)
    }
    // ------------------------=
    // FUNC: allowed
    // DESC: Confines this test-only adapter to the controlled remote server.
    // ------------------=
    fn allowed(&mut self, destination: [u8; 4], port: u16) -> bool {
        destination == [10, 0, 2, 2] && port == env!("HTTPS_TEST_PORT").parse::<u16>().unwrap()
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Receives one actual NIC frame for the shared client.
    // ------------------=
    fn receive(&mut self, frame: &mut [u8; 1514]) -> Option<usize> {
        e1000::E1000::receive(self, frame)
    }
    // ------------------------=
    // FUNC: transmit
    // DESC: Preserves actual descriptor backpressure in the shared client.
    // ------------------=
    fn transmit(&mut self, frame: &[u8]) -> bool {
        e1000::E1000::transmit(self, frame)
    }
}
#[no_mangle]
// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Performs an authenticated HTTPS request using only guest TCP, TLS, crypto and native NIC code.
// ------------------=
pub extern "C" fn infinity_kernel_entry(_info: *const u8) -> ! {
    #[cfg(target_arch = "x86_64")]
    let nic = unsafe { e1000::E1000::initialize() }.unwrap();
    #[cfg(target_arch = "aarch64")]
    let nic = unsafe {
        let info = &*(_info as *const boot_info::BootInfo);
        assert_eq!(info.magic, boot_info::BOOT_MAGIC);
        assert_eq!(info.version, boot_info::BOOT_VERSION);
        assert_eq!(info.network_reserved, 4);
        e1000::E1000::initialize_ecam(info.firmware_network).unwrap()
    };
    let config = Configuration {
        mac: nic.mac,
        address: [10, 0, 2, 15],
        prefix: 24,
        gateway: Some([10, 0, 2, 2]),
        dns_server: [10, 0, 2, 3],
        local_port: 49153,
        deadline: nic.now() + infinity_http::smoltcp::time::Duration::from_secs(30),
    };
    let port = env!("HTTPS_TEST_PORT").parse::<u16>().unwrap();
    let root =
        rustls_pki_types::CertificateDer::from(include_bytes!(env!("HTTPS_TEST_ROOT")).as_slice());
    let roots = [webpki::anchor_from_trusted_cert(&root).unwrap()];
    let now = env!("HTTPS_TEST_TIME").parse::<u64>().unwrap();
    let result = {
        // Deterministic entropy is confined to this test kernel, never used by installed services.
        let mut future = core::pin::pin!(client::get(
            nic,
            config,
            Destination::Address([10, 0, 2, 2]),
            rand_chacha::ChaCha20Rng::from_seed([19; 32]),
            &roots,
            now,
            "localhost",
            port,
            "/",
            Buffers {
                read_record: unsafe { &mut *(&raw mut READ) },
                write_record: unsafe { &mut *(&raw mut WRITE) },
                request: unsafe { &mut *(&raw mut REQUEST) },
                response: unsafe { &mut *(&raw mut RESPONSE) }
            }
        ));
        let waker = unsafe { Waker::from_raw(clone_waker(core::ptr::null())) };
        let mut context = Context::from_waker(&waker);
        loop {
            if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
                break result.unwrap();
            }
        }
    };
    assert_eq!(result.status, 200);
    assert_eq!(result.body_bytes, 11);
    assert_eq!(unsafe { &(&*(&raw const RESPONSE))[..11] }, b"hello world");
    exit(0x10)
}
