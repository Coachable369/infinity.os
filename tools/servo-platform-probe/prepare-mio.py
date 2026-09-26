"""Stage pinned Mio native control polling without changing registry-cache sources."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tomllib
import argparse

NATIVE = 'all(target_os = "none", infinity_native)'

# ------------------------=
# FUNC: native_body
# DESC: Adds an explicit native branch to one pinned upstream method while retaining other platforms unchanged.
# ------------------=
def native_body(text, signature, body):
    if text.count(signature) != 1:
        raise SystemExit("Pinned Mio method changed: " + signature)
    start = text.index("{", text.index(signature))
    depth = 1
    end = start + 1
    while depth:
        depth += (text[end] == "{") - (text[end] == "}")
        end += 1
    original = text[start + 1:end - 1]
    return (text[:start + 1] + '\n#[cfg(' + NATIVE + ')] { ' + body + ' }\n' +
        '#[cfg(not(' + NATIVE + '))] {' + original + '}\n' + text[end - 1:])

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
    shutil.copyfile(root / "sdk/servo-std/mio-io.rs", destination / "src/infinity_io.rs")
    for name, kind, variable in (("tcp/stream", "TcpStream", "stream"), ("tcp/listener", "TcpListener", "listener"), ("udp", "UdpSocket", "socket")):
        path = destination / ("src/net/" + name + ".rs")
        text = path.read_text()
        if name == "tcp/stream":
            text = text.replace("use crate::sys::tcp::{connect, new_for_addr};", '#[cfg(not(' + NATIVE + '))]\nuse crate::sys::tcp::{connect, new_for_addr};')
            text = native_body(text, "pub fn connect(addr: SocketAddr)",
                "std::os::infinity_net::connect_nonblocking(&addr).map(TcpStream::from_std)")
        elif name == "tcp/listener":
            text = text.replace("use crate::sys::tcp::{bind, listen, new_for_addr};", '#[cfg(not(' + NATIVE + '))]\nuse crate::sys::tcp::{bind, listen, new_for_addr};')
            text = native_body(text, "pub fn bind(addr: SocketAddr)", "net::TcpListener::bind(addr).map(TcpListener::from_std)")
            text = native_body(text, "pub fn accept(&self)", "self.inner.accept().map(|(stream, addr)| (TcpStream::from_std(stream), addr))")
            text = text.replace("use crate::{event, sys, Interest, Registry, Token};", "use crate::{event, Interest, Registry, Token};\n#[cfg(not(" + NATIVE + "))]\nuse crate::sys;")
        else:
            text = native_body(text, "pub fn bind(addr: SocketAddr)", "net::UdpSocket::bind(addr).map(UdpSocket::from_std)")
            text = native_body(text, "pub fn only_v6(&self)", "Err(io::Error::from(io::ErrorKind::Unsupported))")
            text = text.replace("use crate::{event, sys, Interest, Registry, Token};", "use crate::{event, Interest, Registry, Token};\n#[cfg(not(" + NATIVE + "))]\nuse crate::sys;")
        text = native_body(text, "fn from(" + variable + ": " + kind + ")", variable + ".inner.into_inner()")
        path.write_text(text)
    selector = (root / "kernel/runtime/http/selector.rs").read_text()
    selector = selector.replace("use crate::transport::Readiness;", "use super::Readiness;", 1)
    (destination / "src/sys/infinity_selector.rs").write_text(selector)
    library = destination / "src/lib.rs"
    source = library.read_text().replace("    mod io_source;", '''    #[cfg(not(all(target_os = "none", infinity_native)))]
    mod io_source;
    #[cfg(all(target_os = "none", infinity_native))]
    #[path = "infinity_io.rs"]
    mod io_source;''', 1)
    library.write_text(source + '''
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
