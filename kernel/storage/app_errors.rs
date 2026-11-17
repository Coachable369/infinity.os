//! Bounded per-application JUL-style error journals under /Logs.
use super::object::ObjectError;
use crate::runtime::console_execution::Bytes;
use core::fmt::{Arguments, Write};

// ------------------------=
// FUNC: record
// DESC: Appends one supervisor-generated event using a traversal-safe app basename and UTC timestamp.
// ------------------=
pub(crate) fn record(app: &[u8], utc: Option<u64>, phase: &str, detail: Arguments<'_>) -> Result<(), ObjectError> {
    let basename = app.rsplit(|b| *b == b'/').next().unwrap_or_default();
    let mut path = Bytes::<64>::new();
    path.write(b"/Logs/");
    let stem = basename.strip_suffix(b".py").unwrap_or(basename);
    for &byte in stem.iter().take(28) {
        path.write(&[if byte.is_ascii_alphanumeric() || byte == b'-' { byte } else { b'_' }]);
    }
    if stem.is_empty() { path.write(b"application"); }
    path.write(b".log");
    let mut entry = Bytes::<768>::new();
    if let Some(t) = utc.and_then(|seconds| crate::runtime::ai::clock::local_time(seconds, 0)) {
        let _ = write!(entry, "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", t.year,t.month,t.day,t.hour,t.minute,t.second);
    } else { entry.write(b"UTC-unavailable"); }
    let _ = writeln!(entry, " infinity.application {phase}\nSEVERE: {detail}");
    super::with_store(|store| store.append_app_error(path.bytes(), entry.bytes()))
}
