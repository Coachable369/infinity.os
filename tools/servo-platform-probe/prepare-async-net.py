"""Stage native TCP-only Tokio/Hyper connectors; preserve upstream host backends."""
import argparse
import hashlib
import importlib.util
import os
from pathlib import Path
import subprocess
import tarfile
import tomllib

# ------------------------=
# FUNC: stage
# DESC: Authenticates exact upstream archives and stages patches outside the Cargo registry cache.
# ------------------=
def stage(root, packages, name, version):
    package = next(p for p in packages if p["name"] == name and p["version"] == version)
    archives = list((root / "build/servo-cargo-home/registry/cache").glob("*/" + name + "-" + version + ".crate"))
    if len(archives) != 1 or hashlib.sha256(archives[0].read_bytes()).hexdigest() != package["checksum"]:
        raise SystemExit("Source checksum mismatch: " + name)
    directory = root / "build/servo-native-deps" / (name + "-" + version)
    with tarfile.open(archives[0]) as archive:
        if any(not m.name.startswith(directory.name + "/") or not (m.isfile() or m.isdir()) for m in archive.getmembers()):
            raise SystemExit("Unexpected archive entry")
        archive.extractall(directory.parent, filter="data")
    return directory, package

# ------------------------=
# FUNC: main
# DESC: Keeps unused raw-socket APIs out of the native dependency graph and connects Hyper through real Mio TCP.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", action="store_true")
    options = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("mio_staging", Path(__file__).with_name("prepare-mio.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    native = helper.NATIVE
    excluded = '#[cfg(not(' + native + '))]\n'
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    packages = tomllib.loads(original)["package"]
    tokio, tokio_package = stage(root, packages, "tokio", "1.53.1")
    hyper, hyper_package = stage(root, packages, "hyper-util", "0.1.20")
    path = tokio / "Cargo.toml"
    key = '''[target.'cfg(any(not(target_family = "wasm"), all(target_os = "wasi", not(target_env = "p1"))))'.dependencies.socket2]'''
    text = path.read_text()
    if text.count(key) != 1:
        raise SystemExit("Tokio socket dependency changed")
    path.write_text(text.replace(key, '''[target.'cfg(all(not(target_os = "none"), any(not(target_family = "wasm"), all(target_os = "wasi", not(target_env = "p1")))))'.dependencies.socket2]'''))
    path = tokio / "src/net/mod.rs"
    text = path.read_text()
    for item in ("pub use tcp::socket::TcpSocket;", "mod udp;", "pub use udp::UdpSocket;"):
        text = text.replace(item, excluded + item, 1)
    path.write_text(text)
    path = tokio / "src/net/tcp/mod.rs"
    path.write_text(path.read_text().replace("pub(crate) mod socket;", excluded + "pub(crate) mod socket;", 1))
    path = tokio / "src/net/tcp/stream.rs"
    text = path.read_text()
    for signature in ("pub fn linger(&self)", "pub fn set_linger(&self, dur: Option<Duration>)", "pub fn set_zero_linger(&self)"):
        body = "Err(io::Error::from(io::ErrorKind::Unsupported))"
        if "dur:" in signature:
            body = "let _ = dur; " + body
        text = helper.native_body(text, signature, body)
    text = helper.native_body(text, "pub fn into_std(self)", "self.io.into_inner().map(Into::into)")
    path.write_text(text)
    path = tokio / "src/net/tcp/listener.rs"
    path.write_text(helper.native_body(path.read_text(), "pub fn into_std(self)", "self.io.into_inner().map(Into::into)"))
    path = tokio / "src/signal/ctrl_c.rs"
    path.write_text(helper.native_body(path.read_text(), "pub async fn ctrl_c()", "Err(io::Error::from(io::ErrorKind::Unsupported))"))
    path = hyper / "Cargo.toml"
    path.write_text(path.read_text().replace("[dependencies.socket2]", '''[target.'cfg(not(target_os = "none"))'.dependencies.socket2]''', 1))
    path = hyper / "src/client/legacy/connect/http.rs"
    text = path.read_text().replace("use socket2::TcpKeepalive;", excluded + "use socket2::TcpKeepalive;", 1)
    text = text.replace("use tokio::net::{TcpSocket, TcpStream};", "use tokio::net::TcpStream;\n" + excluded + "use tokio::net::TcpSocket;", 1)
    text = text.replace("impl TcpKeepaliveConfig {", excluded + "impl TcpKeepaliveConfig {", 1)
    text = text.replace("fn bind_local_address(", excluded + "fn bind_local_address(", 1)
    text = helper.native_body(text, "fn connect(\n    addr: &SocketAddr,", '''
        // Native authority owns local addressing, buffer limits and lifetime.
        // Reject explicit unsupported tuning rather than silently ignoring it.
        if config.local_address_ipv4.is_some() || config.local_address_ipv6.is_some()
            || config.reuse_address || config.send_buffer_size.is_some() || config.recv_buffer_size.is_some()
            || config.tcp_keepalive_config.time.is_some() || config.tcp_keepalive_config.interval.is_some()
            || config.tcp_keepalive_config.retries.is_some() {
            return Err(ConnectError::new("unsupported native socket option", io::Error::from(io::ErrorKind::Unsupported)));
        }
        let addr = *addr;
        Ok(async move {
            let connect = TcpStream::connect(addr);
            match connect_timeout {
                Some(duration) => tokio::time::timeout(duration, connect).await
                    .map_err(|_| ConnectError::new("tcp timeout", io::Error::from(io::ErrorKind::TimedOut)))?
                    .map_err(ConnectError::m("tcp connect error")),
                None => connect.await.map_err(ConnectError::m("tcp connect error")),
            }
        })
    ''')
    path.write_text(text)
    if options.engine:
        lock = servo / "Cargo.lock"
        text = lock.read_text()
        for package in (tokio_package, hyper_package):
            header = 'name = "' + package["name"] + '"\nversion = "' + package["version"] + '"\n'
            entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
            if text.count(entry) > 1:
                raise SystemExit("Ambiguous dependency entry")
            text = text.replace(entry, header)
        lock.write_text(text)
    print(tokio)
    print(hyper)

if __name__ == "__main__":
    main()
