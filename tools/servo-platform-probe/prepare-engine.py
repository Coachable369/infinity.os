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
