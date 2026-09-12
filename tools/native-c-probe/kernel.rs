//! Trusted application bootstrap probe, NOT a native compiler or isolation test.
#![no_std]
#[path = "../storage.rs"] mod storage;
#[path = "../../kernel/runtime/native_c_image.rs"] mod image;
#[path = "../../kernel/core/memory.rs"] mod memory;
use storage::{BlockDevice, object::{ObjectStore, ObjectType, Space, STORE_RELATIVE_LBA}};

const SECTORS: usize = 4096;
static mut DISK: [[u8; 512]; SECTORS] = [[0; 512]; SECTORS];
#[repr(align(4096))]
struct Arena([u8; image::MAX_IMAGE]);
static mut ARENA: Arena = Arena([0; image::MAX_IMAGE]);
static mut OBJECT_BYTES: [u8; 16384] = [0; 16384];
struct ObjectDisk;

impl BlockDevice for ObjectDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Presents a sparse test block device without introducing a filesystem.
    // ------------------=
    fn block_count(&self) -> u64 { STORE_RELATIVE_LBA + SECTORS as u64 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads only the production ObjectStore's reserved sector region.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8;512]) -> bool {
        let Some(index) = lba.checked_sub(STORE_RELATIVE_LBA).filter(|v| *v < SECTORS as u64) else { return false; };
        unsafe { out.copy_from_slice(&(*(&raw const DISK))[index as usize]); }
        true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes production object encodings into test-owned block memory.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8;512]) -> bool {
        let Some(index) = lba.checked_sub(STORE_RELATIVE_LBA).filter(|v| *v < SECTORS as u64) else { return false; };
        unsafe { (*(&raw mut DISK))[index as usize].copy_from_slice(bytes); }
        true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Completes writes synchronously for the in-memory block fixture.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}

#[repr(C)]
struct AppApi {
    version: u32,
    size: u32,
    context: *mut Capture,
    write_stdout: extern "C" fn(*mut Capture, *const u8, usize) -> i32,
}
struct Capture { bytes: [u8; 128], length: usize }

// ------------------------=
// FUNC: capture
// DESC: Captures the trusted app's actual output through the ABI for binary result assertions.
// ------------------=
extern "C" fn capture(context: *mut Capture, bytes: *const u8, length: usize) -> i32 {
    if context.is_null() || bytes.is_null() || length > 128 { return -1; }
    // This probe invokes only the explicitly built trusted fixture, not user code.
    let output = unsafe { &mut *context };
    if length > output.bytes.len() - output.length { return -1; }
    output.bytes[output.length..output.length + length].copy_from_slice(unsafe { core::slice::from_raw_parts(bytes, length) });
    output.length += length;
    length as i32
}

// ------------------------=
// FUNC: finish
// DESC: Reports machine-readable success or failure through QEMU's debug-exit port.
// ------------------=
fn finish(code: u32) -> ! {
    unsafe { core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") code); }
    loop { core::hint::spin_loop(); }
}

#[panic_handler]
// ------------------------=
// FUNC: panic
// DESC: Makes every failed assertion observable as a non-success VM exit code.
// ------------------=
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { finish(0x12) }

#[no_mangle]
// ------------------------=
// FUNC: memcpy
// DESC: Supplies the production freestanding copy operation.
// ------------------=
pub unsafe extern "C" fn memcpy(d: *mut core::ffi::c_void, s: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void { memory::copy(d.cast(),s.cast(),n); d }
#[no_mangle]
// ------------------------=
// FUNC: memmove
// DESC: Supplies the production overlapping-copy operation.
// ------------------=
pub unsafe extern "C" fn memmove(d: *mut core::ffi::c_void, s: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void { memory::move_bytes(d.cast(),s.cast(),n); d }
#[no_mangle]
// ------------------------=
// FUNC: memset
// DESC: Supplies the production memory-fill operation.
// ------------------=
pub unsafe extern "C" fn memset(d: *mut core::ffi::c_void, c: i32, n: usize) -> *mut core::ffi::c_void { memory::fill(d.cast(),c as u8,n); d }
#[no_mangle]
// ------------------------=
// FUNC: memcmp
// DESC: Compares native runtime byte ranges for compiler-generated calls.
// ------------------=
pub unsafe extern "C" fn memcmp(a: *const core::ffi::c_void, b: *const core::ffi::c_void, n: usize) -> i32 {
    for i in 0..n { let delta = a.cast::<u8>().add(i).read() as i32 - b.cast::<u8>().add(i).read() as i32; if delta != 0 { return delta; } }
    0
}

#[no_mangle]
// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Round-trips real C source and ELF objects through ObjectStore, remounts, loads and executes the trusted ELF.
// ------------------=
pub extern "C" fn infinity_kernel_entry(_: *const u8) -> ! {
    let program = include_bytes!("../../build/native-c/hello-x86_64.elf");
    let source = include_bytes!("../../sdk/c/examples/hello.c");
    let mut store = ObjectStore::format(ObjectDisk, 0, ObjectDisk.block_count(), [0x61;16]).unwrap();
    let source_id = store.resolve(b"/home/default/documents/hello.c").unwrap();
    let app_id = store.create_attached(b"hello", ObjectType::ApplicationData, Space::Applications,
        program, b"/applications/hello").unwrap();
    drop(store);
    let mut store = ObjectStore::mount(ObjectDisk, 0).unwrap();
    assert_eq!(store.resolve(b"/home/default/documents/hello.c").unwrap(), source_id);
    assert_eq!(store.resolve(b"/applications/hello").unwrap(), app_id);
    let bytes = unsafe { &mut *(&raw mut OBJECT_BYTES) };
    let source_length = store.read(source_id, None, bytes).unwrap();
    assert_eq!(&bytes[..source_length], source);
    // Verify the source is editable using ordinary object versioning and survives remount.
    let version = store.write(source_id, b"int main(void) { return 23; }\n").unwrap();
    assert!(version > 1);
    drop(store);
    let mut store = ObjectStore::mount(ObjectDisk, 0).unwrap();
    assert_eq!(store.resolve(b"/home/default/documents/hello.c").unwrap(), source_id);
    assert_eq!(store.metadata(source_id).unwrap().current_version, version);
    let length = store.read(app_id, None, bytes).unwrap();
    assert_eq!(&bytes[..length], program);
    let arena = unsafe { &mut (*(&raw mut ARENA)).0 };
    let loaded = image::Image::load(&bytes[..length], 62, arena).unwrap();
    let mut output = Capture { bytes: [0;128], length: 0 };
    let mut api = AppApi { version: 1, size: core::mem::size_of::<AppApi>() as u32,
        context: &mut output, write_stdout: capture };
    // Deliberately only a trusted development fixture; no hardware isolation is asserted.
    let entry: extern "C" fn(*const AppApi) -> i32 = unsafe { core::mem::transmute(arena.as_ptr().add(loaded.entry)) };
    assert_eq!(entry(&api), 0);
    assert_eq!(output.length, 14);
    assert_eq!(storage::object::crc32(&output.bytes[..output.length]), storage::object::crc32(b"Hello, world!\n"));
    output.length = 0;
    api.version = 2;
    assert_eq!(entry(&api), 126);
    assert_eq!(output.length, 0);
    finish(0x10)
}
