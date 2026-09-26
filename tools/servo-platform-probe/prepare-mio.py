"""Stage pinned Mio native control polling without changing registry-cache sources."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tomllib
import argparse

# ------------------------=
# FUNC: main
# DESC: Authenticates and stages the pinned dependency, then adds only the explicit native selector backend.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", action="store_true")
    options = parser.parse_args()
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    package = next(p for p in tomllib.loads(original)["package"] if p["name"] == "mio")
    if package["version"] != "1.2.3":
        raise SystemExit("Unreviewed Mio version")
    archives = list((root / "build/servo-cargo-home/registry/cache").glob("*/mio-1.2.3.crate"))
    if len(archives) != 1 or hashlib.sha256(archives[0].read_bytes()).hexdigest() != package["checksum"]:
        raise SystemExit("Mio source checksum mismatch")
    destination = root / "build/servo-native-deps/mio-1.2.3"
    with tarfile.open(archives[0]) as archive:
        if any(not m.name.startswith("mio-1.2.3/") or not (m.isfile() or m.isdir()) for m in archive.getmembers()):
            raise SystemExit("Unexpected Mio archive member")
        archive.extractall(destination.parent, filter="data")
    module = destination / "src/sys/mod.rs"
    source = module.read_text().replace("    macro_rules! debug_detail {",
        '    #[cfg(not(all(target_os = "none", infinity_native)))]\n    macro_rules! debug_detail {', 1)
    module.write_text(source + '''
#[cfg(all(target_os = "none", infinity_native, feature = "os-poll"))]
mod infinity;
#[cfg(all(target_os = "none", infinity_native, feature = "os-poll"))]
pub use infinity::*;
''')
    shutil.copyfile(root / "sdk/servo-std/mio-poll.rs", destination / "src/sys/infinity.rs")
    selector = (root / "kernel/runtime/http/selector.rs").read_text()
    selector = selector.replace("use crate::transport::Readiness;", "use super::Readiness;", 1)
    (destination / "src/sys/infinity_selector.rs").write_text(selector)
    library = destination / "src/lib.rs"
    library.write_text(library.read_text() + '''
/// Native service readiness bridge; does not grant network authority.
#[cfg(all(target_os = "none", infinity_native, feature = "os-poll"))]
pub mod infinity { pub use crate::sys::{NativeSource, Readiness}; }
''')
    if options.engine:
        lock = servo / "Cargo.lock"
        entry = ('name = "mio"\nversion = "1.2.3"\nsource = "' + package["source"] +
                 '"\nchecksum = "' + package["checksum"] + '"\n')
        text = lock.read_text()
        if text.count(entry) > 1:
            raise SystemExit("Ambiguous Mio lock entry")
        if entry in text:
            lock.write_text(text.replace(entry, 'name = "mio"\nversion = "1.2.3"\n'))
    print(destination)

if __name__ == "__main__":
    main()
