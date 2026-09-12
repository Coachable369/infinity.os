//! Trusted application bootstrap probe, NOT a native compiler or isolation test.
#![no_std]
#[path = "../storage.rs"] mod storage;
#[path = "../../kernel/runtime/native_c_image.rs"] mod image;
#[path = "../../kernel/runtime/native_c_io.rs"] mod object_io;
#[path = "../../kernel/core/memory.rs"] mod memory;
use storage::{BlockDevice, object::{ObjectStore, ObjectType, Space, STORE_RELATIVE_LBA}};

const SECTORS: usize = 4096;
static mut DISK: [[u8; 512]; SECTORS] = [[0; 512]; SECTORS];
#[repr(align(4096))]
struct Arena([u8; image::MAX_IMAGE]);
static mut ARENA: Arena = Arena([0; image::MAX_IMAGE]);
static mut OBJECT_BYTES: [u8; 16384] = [0; 16384];
static mut IO: object_io::ObjectIo<'static, 2> = object_io::ObjectIo::<2>::documents();
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
    objects: *const ObjectApi,
}
struct Capture { bytes: [u8; 128], length: usize, store: *mut ObjectStore<ObjectDisk> }

#[repr(C)]
struct ObjectApi {
    open: extern "C" fn(*mut Capture, *const u8, usize, u32) -> i32,
    read: extern "C" fn(*mut Capture, i32, *mut u8, usize) -> isize,
    write: extern "C" fn(*mut Capture, i32, *const u8, usize) -> isize,
    seek: extern "C" fn(*mut Capture, i32, i64, u32) -> i64,
    flush: extern "C" fn(*mut Capture, i32) -> i32,
    close: extern "C" fn(*mut Capture, i32) -> i32,
}

// These callbacks are only for the trusted fixture; application-pointer
// validation and address-space isolation remain prerequisites for deployment.
// ------------------------=
// FUNC: object_open
// DESC: Resolves a trusted C path through the session's explicit namespace grants.
// ------------------=
extern "C" fn object_open(context: *mut Capture, path: *const u8, length: usize, flags: u32) -> i32 {
    if context.is_null() || path.is_null() || length > 95 { return -1; }
    unsafe { (*(&raw mut IO)).open(&mut *(*context).store, core::slice::from_raw_parts(path, length), flags)
        .unwrap_or_else(|e| -(e as i32)) }
}

// ------------------------=
// FUNC: object_read
// DESC: Reads an opened object snapshot into a trusted app's bounded buffer.
// ------------------=
extern "C" fn object_read(_: *mut Capture, fd: i32, out: *mut u8, length: usize) -> isize {
    if out.is_null() || length > 16384 { return -1; }
    unsafe { (*(&raw mut IO)).read(fd, core::slice::from_raw_parts_mut(out, length))
        .map(|n| n as isize).unwrap_or_else(|e| -(e as isize)) }
}

// ------------------------=
// FUNC: object_write
// DESC: Buffers C output in an ObjectStore stream without touching a filesystem.
// ------------------=
extern "C" fn object_write(_: *mut Capture, fd: i32, bytes: *const u8, length: usize) -> isize {
    if bytes.is_null() || length > 16384 { return -1; }
    unsafe { (*(&raw mut IO)).write(fd, core::slice::from_raw_parts(bytes, length))
        .map(|n| n as isize).unwrap_or_else(|e| -(e as isize)) }
}

// ------------------------=
// FUNC: object_seek
// DESC: Applies checked seek arithmetic to the object descriptor.
// ------------------=
extern "C" fn object_seek(_: *mut Capture, fd: i32, offset: i64, origin: u32) -> i64 {
    unsafe { (*(&raw mut IO)).seek(fd, offset, origin).map(|n| n as i64).unwrap_or_else(|e| -(e as i64)) }
}

// ------------------------=
// FUNC: object_flush
// DESC: Publishes a dirty version using production object transactions.
// ------------------=
extern "C" fn object_flush(context: *mut Capture, fd: i32) -> i32 {
    if context.is_null() { return -1; }
    unsafe { (*(&raw mut IO)).flush(&mut *(*context).store, fd).map(|_| 0).unwrap_or_else(|e| -(e as i32)) }
}

// ------------------------=
// FUNC: object_close
// DESC: Closes a stream and reports publication errors to the C runtime.
// ------------------=
extern "C" fn object_close(context: *mut Capture, fd: i32) -> i32 {
    if context.is_null() { return -1; }
    unsafe { (*(&raw mut IO)).close(&mut *(*context).store, fd).map(|_| 0).unwrap_or_else(|e| -(e as i32)) }
}

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
    let mut output = Capture { bytes: [0;128], length: 0, store: &mut store };
    let mut api = AppApi { version: 2, size: core::mem::size_of::<AppApi>() as u32,
        context: &mut output, write_stdout: capture, objects: core::ptr::null() };
    // Deliberately only a trusted development fixture; no hardware isolation is asserted.
    let entry: extern "C" fn(*const AppApi) -> i32 = unsafe { core::mem::transmute(arena.as_ptr().add(loaded.entry)) };
    assert_eq!(entry(&api), 0);
    assert_eq!(output.length, 14);
    assert_eq!(storage::object::crc32(&output.bytes[..output.length]), storage::object::crc32(b"Hello, world!\n"));
    output.length = 0;
    api.version = 3;
    assert_eq!(entry(&api), 126);
    assert_eq!(output.length, 0);
    let object_api = ObjectApi { open: object_open, read: object_read, write: object_write,
        seek: object_seek, flush: object_flush, close: object_close };
    api.version = 2;
    api.objects = &object_api;
    let io_program = include_bytes!("../../build/native-c/io-x86_64.elf");
    // Reuse the existing executable object, retaining its stable identity.
    store.write(app_id, io_program).unwrap();
    let size = store.read(app_id, None, bytes).unwrap();
    let loaded = image::Image::load(&bytes[..size], 62, arena).unwrap();
    let entry: extern "C" fn(*const AppApi) -> i32 = unsafe { core::mem::transmute(arena.as_ptr().add(loaded.entry)) };
    assert_eq!(entry(&api), 0);
    drop(store);
    let mut store = ObjectStore::mount(ObjectDisk, 0).unwrap();
    let result = store.resolve(b"/home/default/documents/native-output.txt").unwrap();
    assert_eq!(store.metadata(result).unwrap().kind, ObjectType::Text);
    let size = store.read(result, None, bytes).unwrap();
    assert_eq!(&bytes[..size], b"Object-backed C Streams\n!");
    let sync_program = include_bytes!("../../build/native-c/sync-x86_64.elf");
    let loaded = image::Image::load(sync_program, 62, arena).unwrap();
    let entry: extern "C" fn(*const AppApi) -> i32 = unsafe { core::mem::transmute(arena.as_ptr().add(loaded.entry)) };
    assert_eq!(entry(&api), 0);
    finish(0x10)
}
