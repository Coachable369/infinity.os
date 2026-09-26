"""Stage authenticated getrandom versions behind the native entropy service ABI."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Preserves upstream platforms while routing native fills to a fail-closed service.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("async_staging", Path(__file__).with_name("prepare-async-net.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    packages = tomllib.loads(original)["package"]
    lock = servo / "Cargo.lock"
    locked = lock.read_text()
    for version in ("0.2.17", "0.3.4", "0.4.1"):
        directory, package = helper.stage(root, packages, "getrandom", version)
        old = version.startswith("0.2.")
        path = directory / ("src/lib.rs" if old else "src/backends.rs")
        text = path.read_text()
        marker = "cfg_if! {\n    if #[cfg("
        if text.count(marker) != 1:
            raise SystemExit("Unexpected getrandom selector")
        branch = ('#[path = "infinity.rs"] mod imp;' if old else 'mod infinity;\n        pub use infinity::*;')
        text = text.replace(marker, 'cfg_if! {\n    if #[cfg(all(target_os = "none", infinity_native))] {\n        ' + branch + '\n    } else if #[cfg(', 1)
        path.write_text(text)
        name = "getrandom_inner" if old else "fill_inner"
        backend = directory / ("src/infinity.rs" if old else "src/backends/infinity.rs")
        backend.write_text('''use crate::Error;
use core::mem::MaybeUninit;
''' + ("" if old else "pub use crate::util::{inner_u32, inner_u64};\n") + '''
// ------------------------=
// FUNC: ''' + name + '''
// DESC: Requires a complete native entropy fill; never substitutes predictable bytes.
// ------------------=
pub fn ''' + name + '''(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    unsafe extern "C" {
        fn infinity_std_entropy(bytes: *mut u8, length: usize) -> i32;
    }
    if dest.is_empty() { return Ok(()); }
    if unsafe { infinity_std_entropy(dest.as_mut_ptr().cast(), dest.len()) } == 0 {
        Ok(())
    } else {
        Err(Error::UNSUPPORTED)
    }
}
''')
        header = 'name = "getrandom"\nversion = "' + version + '"\n'
        entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
        locked = locked.replace(entry, header)
    directory, package = helper.stage(root, packages, "aws-lc-sys", "0.45.0")
    path = directory / "aws-lc/crypto/rand_extra/internal.h"
    text = path.read_text()
    marker = "#if defined(BORINGSSL_UNSAFE_DETERMINISTIC_MODE)"
    if text.count(marker) != 1:
        raise SystemExit("Unexpected AWS-LC entropy selector")
    text = text.replace(marker, "#if defined(__INFINITYOS__)\n#define OPENSSL_RAND_GETENTROPY\n#elif defined(BORINGSSL_UNSAFE_DETERMINISTIC_MODE)", 1)
    path.write_text(text)
    header = 'name = "aws-lc-sys"\nversion = "0.45.0"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    locked = locked.replace(entry, header)
    lock.write_text(locked)

if __name__ == "__main__":
    main()
