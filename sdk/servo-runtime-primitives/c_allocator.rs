//! Opt-in C allocation symbols backed by the same granted heap as Rust.
use crate::native::{infinity_c_malloc, infinity_c_free, infinity_c_realloc};

// ------------------------=
// FUNC: malloc
// DESC: Allocates C-aligned storage from the governed owner-local heap.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut u8 { infinity_c_malloc(size) }

// ------------------------=
// FUNC: free
// DESC: Releases a native allocation; null is a no-op.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn free(pointer: *mut u8) { infinity_c_free(pointer); }

// ------------------------=
// FUNC: realloc
// DESC: Preserves the original allocation and bytes if resizing fails.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn realloc(pointer: *mut u8, size: usize) -> *mut u8 {
    infinity_c_realloc(pointer, size)
}

// ------------------------=
// FUNC: calloc
// DESC: Rejects multiplication overflow and zeroes only a successfully allocated payload.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut u8 {
    let Some(length) = count.checked_mul(size) else { return core::ptr::null_mut(); };
    let pointer = infinity_c_malloc(length);
    if !pointer.is_null() { pointer.write_bytes(0, length); }
    pointer
}
