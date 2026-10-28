//! Checked native FontKit transport shared by C and Python application contexts.
use super::{native_font, native_memory::{PageTable, copy_from_user, copy_to_user, check_user_range}};

// ------------------------=
// FUNC: dispatch
// DESC: Validates all user ranges before copying public font pixels; no caller receives kernel or framebuffer pointers.
// ------------------=
pub fn dispatch(tables: &mut impl PageTable, args: [u64; 6]) -> i64 {
    if args[1] != 16 || args[3] != native_font::METRIC_BYTES as u64
        || args[5] > native_font::MAX_COVERAGE as u64 { return -22; }
    let mut request_bytes = [0; 16];
    if copy_from_user(tables, args[0], &mut request_bytes).is_err() { return -14; }
    let request = core::array::from_fn(|i| u32::from_le_bytes(request_bytes[i*4..i*4+4].try_into().unwrap()));
    let mut coverage = [0; native_font::MAX_COVERAGE];
    let metrics = match native_font::glyph(request, &mut coverage[..args[5] as usize]) {
        Ok(metrics) => metrics, Err(error) => return error,
    };
    let count = metrics[9] as usize;
    let Some(metric_end) = args[2].checked_add(40) else { return -14; };
    let Some(pixel_end) = args[4].checked_add(count as u64) else { return -14; };
    if args[2] < pixel_end && args[4] < metric_end { return -22; }
    if check_user_range(tables, args[2], 40, true).is_err()
        || check_user_range(tables, args[4], count, true).is_err() { return -14; }
    let mut output = [0; 40];
    for (i, value) in metrics.iter().enumerate() { output[i*4..i*4+4].copy_from_slice(&value.to_le_bytes()); }
    if copy_to_user(tables, args[4], &coverage[..count]).is_err()
        || copy_to_user(tables, args[2], &output).is_err() { return -14; }
    0
}
