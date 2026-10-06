//! Versioned integer/pointer-only ABI between the native engine component and
//! the OS worker. Callbacks must be nonblocking mailbox operations. All pointers
//! remain owned by their issuer; frame/event bytes are borrowed for one call.
pub const VERSION: u32 = 4;
pub const OPEN: u32 = 1;
pub const CLOSE: u32 = 2;
pub const NAVIGATE: u32 = 3;
pub const RESIZE: u32 = 4;
pub const POINTER: u32 = 5;
pub const BUTTON: u32 = 6;
pub const KEY: u32 = 7;
pub const SCROLL: u32 = 8;
pub const BACK: u32 = 9;
pub const FORWARD: u32 = 10;
pub const RELOAD: u32 = 11;
pub const TAB_CREATE: u32 = 12;
pub const TAB_SELECT: u32 = 13;
pub const TAB_CLOSE: u32 = 14;
pub const FIND: u32 = 15;
pub const ZOOM: u32 = 16;
pub const EVENT_FIND: u32 = 13;
pub const SHUTDOWN: u32 = 255;
/// KEY flags: press versus release, named versus Unicode key, autorepeat.
pub const KEY_DOWN:u32=1;
pub const KEY_NAMED:u32=2;
pub const KEY_REPEAT:u32=4;
/// KEY b field is an engine-independent native modifier mask.
pub const MOD_SHIFT:u32=1;
pub const MOD_CONTROL:u32=2;
pub const MOD_ALT:u32=4;
pub const MOD_META:u32=8;
pub const EVENT_OPEN: u32 = 1;
pub const EVENT_CLOSED: u32 = 2;
pub const EVENT_LOAD: u32 = 3;
pub const EVENT_ADDRESS: u32 = 4;
pub const EVENT_TITLE: u32 = 5;
pub const EVENT_ERROR: u32 = 6;
/// Value is native heap high-water bytes, including allocator rounding, not total OS RAM.
pub const EVENT_MEMORY: u32 = 7;
pub const EVENT_ALLOCATION_FAILURE: u32 = 9;
/// Value bit 0 permits Back; bit 1 permits Forward, from real engine history.
pub const EVENT_HISTORY: u32 = 8;
pub const EVENT_TAB_CREATED: u32 = 10;
pub const EVENT_TAB_SELECTED: u32 = 11;
pub const EVENT_TAB_CLOSED: u32 = 12;
/// Supervisor diagnostics only; never render raw engine text as an error page.
pub const EVENT_DIAGNOSTIC: u32 = 100;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Command {
    pub clipboard_epoch: u64,
    pub kind: u32,
    pub flags: u32,
    pub x: i32,
    pub y: i32,
    pub a: u32,
    pub b: u32,
    pub length: u32,
    pub text: [u8; 2048],
}
impl Command {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates a fully initialized mailbox payload with no command or text.
    // ------------------=
    pub const fn empty() -> Self {
        Self { clipboard_epoch: 0, kind: 0, flags: 0, x: 0, y: 0, a: 0, b: 0, length: 0, text: [0; 2048] }
    }
}

/// Response buffers stay valid until cancel(id); completion always calls cancel.
#[repr(C)]
pub struct Response {
    pub status: u32,
    pub headers: *const u8,
    pub headers_length: usize,
    pub body: *const u8,
    pub body_length: usize,
}

#[repr(C)]
pub struct Host {
    pub version: u32,
    pub size: u32,
    pub context: *mut core::ffi::c_void,
    pub heap: *mut u8,
    pub heap_length: usize,
    pub cpu: unsafe extern "C" fn(*mut core::ffi::c_void) -> u64,
    pub monotonic: unsafe extern "C" fn(*mut core::ffi::c_void) -> u64,
    pub utc: unsafe extern "C" fn(*mut core::ffi::c_void, *mut u64, *mut u32) -> u32,
    pub entropy: unsafe extern "C" fn(*mut core::ffi::c_void, *mut u8, usize) -> u32,
    pub idle: unsafe extern "C" fn(*mut core::ffi::c_void),
    pub command: unsafe extern "C" fn(*mut core::ffi::c_void, *mut Command) -> u32,
    pub frame: unsafe extern "C" fn(*mut core::ffi::c_void, u32, u32, *const u8, usize),
    pub event: unsafe extern "C" fn(*mut core::ffi::c_void, u32, u32, *const u8, usize),
    pub begin: unsafe extern "C" fn(*mut core::ffi::c_void, *const u8, usize, *const u8, usize) -> u64,
    /// 0 pending; 1 complete; any other result failed.
    pub poll: unsafe extern "C" fn(*mut core::ffi::c_void, u64, *mut Response) -> u32,
    pub cancel: unsafe extern "C" fn(*mut core::ffi::c_void, u64),
    /// Offers a bounded complete attachment for explicit native save consent; zero rejects it.
    pub download: unsafe extern "C" fn(*mut core::ffi::c_void, *const u8, usize, *const u8, usize, *const u8, usize) -> u32,
    /// Zero denies; otherwise the result is the number of copied UTF-8 bytes plus one.
    pub clipboard_read: unsafe extern "C" fn(*mut core::ffi::c_void, *mut u8, usize) -> usize,
    pub clipboard_write: unsafe extern "C" fn(*mut core::ffi::c_void, *const u8, usize) -> u32,
    /// Must abandon/quarantine this worker, never return into a failed engine.
    pub fatal: unsafe extern "C" fn(*mut core::ffi::c_void, u32) -> !,
}
