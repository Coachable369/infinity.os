"""Stage a checksum-verified native ABI correction outside Cargo's registry cache."""
import hashlib
import os
from pathlib import Path
import tarfile
import subprocess
import tomllib


# ------------------------=
# FUNC: main
# DESC: Stages basic native C ABI types in pinned libc without pretending to provide POSIX services.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    servo = root / "build/servo-port-audit"
    original_lock = subprocess.check_output(
        ["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    lock = tomllib.loads(original_lock)
    package = next(p for p in lock["package"] if p["name"] == "libc")
    if package["version"] != "0.2.189":
        raise SystemExit("Unreviewed libc ABI version")
    cache = root / "build/servo-cargo-home/registry"
    archives = list((cache / "cache").glob("*/libc-0.2.189.crate"))
    if len(archives) != 1:
        raise SystemExit("Expected exactly one pinned libc source")
    if hashlib.sha256(archives[0].read_bytes()).hexdigest() != package["checksum"]:
        raise SystemExit("libc archive checksum mismatch")
    # Extract authenticated bytes rather than trusting previously extracted cache files.
    destination = root / "build/servo-native-deps/libc-0.2.189"
    with tarfile.open(archives[0]) as archive:
        members = archive.getmembers()
        if any(not item.name.startswith("libc-0.2.189/") or
               not (item.isfile() or item.isdir()) for item in members):
            raise SystemExit("Unexpected libc archive member")
        archive.extractall(destination.parent, members=members, filter="data")
    source = destination / "src/lib.rs"
    old = '''    } else {
        // non-supported targets: empty...
    }'''
    new = '''    } else if #[cfg(all(target_os = "none", infinity_native))] {
        mod primitives;
        pub use crate::primitives::*;
        pub type size_t = usize;
        pub type ssize_t = isize;
        pub type off_t = c_long;
        pub const ENOSPC: c_int = 28;
        extern "C" {
            #[link_name = "infinity_c_malloc"]
            pub fn malloc(size: size_t) -> *mut c_void;
            #[link_name = "infinity_c_free"]
            pub fn free(pointer: *mut c_void);
            #[link_name = "infinity_c_realloc"]
            pub fn realloc(pointer: *mut c_void, size: size_t) -> *mut c_void;
            #[link_name = "infinity_std_usable_size"]
            pub fn malloc_usable_size(pointer: *mut c_void) -> size_t;
        }
    } else {
        // non-supported targets: empty...
    }'''
    text = source.read_text()
    if text.count(old) != 1:
        raise SystemExit("libc ABI patch no longer matches")
    source.write_text(text.replace(old, new))
    # The path overlay changes only this package's source identity. Preserve every
    # other upstream resolution instead of allowing cargo update to move transitive pins.
    original_entry = ('name = "libc"\nversion = "0.2.189"\nsource = "' +
                      package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n')
    if original_lock.count(original_entry) != 1:
        raise SystemExit("Unexpected libc lock entry")
    (servo / "Cargo.lock").write_text(original_lock.replace(
        original_entry, 'name = "libc"\nversion = "0.2.189"\n'))
    print(destination)


if __name__ == "__main__":
    main()
