"""Stage a project-local Rust std overlay; never change the host toolchain."""
import os
from pathlib import Path
import shutil
import subprocess


# ------------------------=
# FUNC: main
# DESC: Copies pinned Rust sources and inserts isolated Infinity platform selection arms.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    version = subprocess.check_output(["rustc", "--version"], text=True).strip()
    if version != "rustc 1.98.0 (88d9e12ae 2026-08-18) (Homebrew)":
        raise SystemExit("Rust source overlay requires reviewed Rust 1.98.0 toolchain")
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    source = sysroot / "lib/rustlib/src/rust/library"
    destination = root / "build/servo-rust-src/library"
    for path in destination.rglob("*"):
        if path.is_file() and not path.is_symlink():
            path.chmod(path.stat().st_mode | 0o200)
    shutil.copytree(source, destination, dirs_exist_ok=True, copy_function=shutil.copyfile)
    # Opt-in only in this isolated port: do not inject feature attributes into
    # every dependency (no_std dependencies cannot resolve std's feature name).
    path = destination / "std/src/lib.rs"
    text = path.read_text()
    text = text.replace('not(restricted_std), stable',
                        'any(not(restricted_std), infinity_native), stable', 1)
    text = text.replace('    restricted_std,\n    unstable(',
                        '    all(restricted_std, not(infinity_native)),\n    unstable(', 1)
    path.write_text(text)
    system = destination / "std/src/sys"
    adapters = root / "sdk/servo-std"
    path = destination / "std/src/os/mod.rs"
    path.write_text(path.read_text() + '''
#[cfg(all(target_os = "none", infinity_native))]
#[stable(feature = "infinity_native_net", since = "1.98.0")]
#[path = "infinity_net.rs"]
pub mod infinity_net;
''')
    shutil.copyfile(adapters / "os-net.rs", destination / "std/src/os/infinity_net.rs")
    path = system / "net/connection/mod.rs"
    path.write_text(path.read_text().replace("cfg_select! {", '''cfg_select! {
    all(target_os = "none", infinity_native) => {
        mod infinity;
        pub use infinity::*;
    }
''', 1))
    shutil.copyfile(adapters / "net.rs", system / "net/connection/infinity.rs")
    unsupported = (system / "net/connection/unsupported.rs").read_text()
    unsupported = unsupported.replace("io::Result<(TcpStream, SocketAddr)>", "io::Result<(super::TcpStream, SocketAddr)>")
    (system / "net/connection/infinity_unsupported.rs").write_text(unsupported)
    for relative, exports in (
        ("alloc/mod.rs", ""),
        ("io/error/mod.rs", "pub use infinity::*;"),
        ("random/mod.rs", "pub use infinity::fill_bytes;"),
        ("thread/mod.rs", "pub use infinity::*;"),
        ("time/mod.rs", "use infinity as imp;"),
    ):
        path = system / relative
        text = path.read_text()
        arm = ('cfg_select! {\n    all(target_os = "none", infinity_native) => {\n'
               f'        mod infinity; {exports}\n    }}\n')
        path.write_text(text.replace("cfg_select! {", arm, 1))
        shutil.copyfile(adapters / (relative.split('/')[0] + ".rs"), path.parent / "infinity.rs")
    path = system / "thread_local/mod.rs"
    text = path.read_text()
    marker = "pub(crate) mod key {"
    before, after = text.split(marker, 1)
    after = after.replace("cfg_select! {", '''cfg_select! {
        all(target_os = "none", infinity_native) => {
            mod infinity;
            pub use infinity::*;
            mod racy;
            pub use racy::LazyKey;
        }
''', 1)
    path.write_text(before + marker + after)
    shutil.copyfile(adapters / "tls.rs", system / "thread_local/key/infinity.rs")
    path = system / "mod.rs"
    path.write_text(path.read_text() + '\n#[cfg(all(target_os = "none", infinity_native))]\npub mod futex;\n')
    shutil.copyfile(adapters / "futex.rs", system / "futex.rs")
    for component in ("mutex", "condvar", "once", "rwlock", "thread_parking"):
        path = system / "sync" / component / "mod.rs"
        path.write_text(path.read_text().replace('    any(\n',
            '    any(\n        all(target_os = "none", infinity_native),\n', 1))
    print(destination)


if __name__ == "__main__":
    main()
