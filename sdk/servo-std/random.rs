unsafe extern "C" { fn infinity_std_entropy(bytes: *mut u8, length: usize) -> i32; }
// ------------------------=
// FUNC: fill_bytes
// DESC: Requires a complete native entropy fill; never substitutes deterministic bytes.
// ------------------=
pub fn fill_bytes(bytes: &mut [u8]) {
    if !bytes.is_empty() && unsafe { infinity_std_entropy(bytes.as_mut_ptr(), bytes.len()) } != 0 {
        panic!("native entropy unavailable");
    }
}
