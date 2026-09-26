"""Keep native engine clocks and object authority separate from host OS APIs."""
import importlib.util
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Stages explicit native monotonic time and rejects unsupported filesystem URL conversions.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    servo = root / "build/servo-port-audit"
    spec = importlib.util.spec_from_file_location("native", Path(__file__).with_name("prepare-mio.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    relative = "components/url/lib.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = helper.native_body(text, 'pub fn to_file_path(&self)', 'Err(UrlError::ToFilePath)')
    text = helper.native_body(text, 'pub fn from_file_path<P:', 'let _ = path; Err(UrlError::FromFilePath)')
    (servo / relative).write_text(text)

    # The beta owns session storage in memory. Do not manufacture a Unix temp
    # directory before selecting the upstream memory implementations.
    for component, handle, engine, thread_body in (
        ("client_storage", "ClientStorageThreadHandle", "SqliteEngine::memory().expect(\"Native session registry initialization failed\")",
         "ClientStorageThread::new(sender_clone, generic_receiver, engine).start();"),
        ("cache_storage", "CacheStorageThreadHandle", "MemCacheStorageEngine { name_to_cache_map: Default::default() }",
         "let mut worker = CacheStorageThread::new(sender_clone, generic_receiver, engine); worker.start();"),
    ):
        relative = "components/storage/" + component + ".rs"
        text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
        signature = "fn new(config_dir: Option<PathBuf>, temporary_storage: bool) -> " + handle
        body = '''assert!(config_dir.is_none(), "Native persistent profile storage is not available");
            let _ = temporary_storage;
            let (generic_sender, generic_receiver) = generic_channel::channel().unwrap();
            let sender_clone = generic_sender.clone();
            thread::Builder::new().name("NativeSessionStorage".to_owned()).spawn(move || {
                let engine = ''' + engine + ''';
                ''' + thread_body + '''
            }).expect("Native session storage thread unavailable");
            ''' + handle + '''::new(generic_sender)'''
        text = helper.native_body(text, signature, body)
        (servo / relative).write_text(text)

    relative = "components/net/disk_cache.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = helper.native_body(text, "fn storage_dir()", "None")
    (servo / relative).write_text(text)

    relative = "components/script/dom/navigator/navigatorinfo.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text += '''
#[cfg(all(target_os = "none", infinity_native))]
#[expect(non_snake_case)]
// ------------------------=
// FUNC: Platform
// DESC: Identifies the native platform consistently in windows and web workers.
// ------------------=
pub(crate) fn Platform() -> DOMString {
    DOMString::from_static("InfinityOS")
}
'''
    (servo / relative).write_text(text)
    relative = "components/shared/base/cross_process_instant.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text += '''
#[cfg(all(target_os = "none", infinity_native))]
mod platform {
    #[expect(unsafe_code)]
    unsafe extern "C" {
        fn infinity_std_clock(clock: u32, seconds: *mut u64, nanos: *mut u32) -> i32;
    }

    // ------------------------=
    // FUNC: now
    // DESC: Reads the native monotonic epoch shared by all engine workers; never substitutes wall time.
    // ------------------=
    #[expect(unsafe_code)]
    pub(super) fn now() -> u64 {
        let mut seconds = 0;
        let mut nanos = 0;
        // SAFETY: Both output pointers are live local values; the provider validates its owner.
        let result = unsafe { infinity_std_clock(0, &mut seconds, &mut nanos) };
        assert_eq!(result, 0, "native monotonic clock unavailable");
        seconds.checked_mul(1_000_000_000).and_then(|n| n.checked_add(nanos as u64))
            .expect("native monotonic clock overflow")
    }
}
'''
    (servo / relative).write_text(text)


if __name__ == "__main__":
    main()
