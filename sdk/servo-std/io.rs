use crate::io::ErrorKind;
unsafe extern "C" { fn infinity_std_last_error() -> i32; }
// ------------------------=
// FUNC: errno
// DESC: Reads the native execution thread's last adapter error.
// ------------------=
pub fn errno() -> i32 { unsafe { infinity_std_last_error() } }
// ------------------------=
// FUNC: is_interrupted
// DESC: Recognizes the adapter ABI interruption code.
// ------------------=
pub fn is_interrupted(code: i32) -> bool { code == 4 }
// ------------------------=
// FUNC: decode_error_kind
// DESC: Maps stable adapter codes without treating unknown failures as success.
// ------------------=
pub fn decode_error_kind(code: i32) -> ErrorKind {
    match code {
        2 => ErrorKind::NotFound, 4 => ErrorKind::Interrupted,
        11 => ErrorKind::WouldBlock, 12 => ErrorKind::OutOfMemory,
        13 => ErrorKind::PermissionDenied, 17 => ErrorKind::AlreadyExists,
        22 => ErrorKind::InvalidInput, 38 | 95 => ErrorKind::Unsupported,
        110 => ErrorKind::TimedOut, _ => ErrorKind::Other,
    }
}
// ------------------------=
// FUNC: error_string
// DESC: Formats an adapter error for internal diagnostics, not browser presentation.
// ------------------=
pub fn error_string(code: i32) -> String { format!("Infinity runtime error {code}") }
