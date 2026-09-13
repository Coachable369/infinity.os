"""Native TCP/TLS client proof against a local TLS server; no installed acceptance."""
import os
from pathlib import Path
import shutil
import socket
import ssl
import subprocess
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: run
# DESC: Builds a test-only trust fixture, boots the native HTTPS client and asserts its binary guest outcome.
# ------------------=
def run():
    with tempfile.TemporaryDirectory(prefix="infinity-https-") as temporary:
        work = Path(temporary)
        key, cert, der = (work / name for name in ("key.pem", "cert.pem", "root.der"))
        subprocess.run(["openssl", "req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1",
            "-nodes", "-keyout", str(key), "-out", str(cert), "-days", "1", "-subj", "/CN=localhost",
            "-addext", "subjectAltName=DNS:localhost", "-addext", "basicConstraints=critical,CA:FALSE",
            "-addext", "extendedKeyUsage=serverAuth"], check=True, capture_output=True)
        subprocess.run(["openssl", "x509", "-in", str(cert), "-outform", "DER", "-out", str(der)], check=True)
        listener = socket.socket(); listener.bind(("127.0.0.1", 0)); listener.listen(1); listener.settimeout(60)
        environment = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(ROOT / "build/native-https/cargo"),
            HTTPS_TEST_PORT=str(listener.getsockname()[1]), HTTPS_TEST_ROOT=str(der), HTTPS_TEST_TIME=str(int(time.time())))
        subprocess.run(["cargo", "build", "--release", "-Z", "build-std=core", "--target", "x86_64-unknown-none",
            "--manifest-path", "tools/native-https-probe/Cargo.toml"], cwd=ROOT, env=environment, check=True)
        subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "-nostdlib", "-static", "-T", "linker/x86_64.ld",
            "-o", "build/native-https/probe.elf", "build/native-https/cargo/x86_64-unknown-none/release/libinfinity_native_https_probe.a"], cwd=ROOT, check=True)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); context.minimum_version = ssl.TLSVersion.TLSv1_3
        context.load_cert_chain(cert, key); context.set_alpn_protocols(["http/1.1"])
        outcomes = []
        # ------------------------=
        # FUNC: serve
        # DESC: Acts only as the remote HTTPS server; guest performs all client transport and cryptographic work.
        # ------------------=
        def serve():
            try:
                connection, _ = listener.accept(); connection.settimeout(30)
                with context.wrap_socket(connection, server_side=True) as stream:
                    request = bytearray()
                    while not request.endswith(b"\r\n\r\n") and len(request) < 2048:
                        chunk = stream.recv(512)
                        if not chunk: break
                        request.extend(chunk)
                    assert request == b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n"
                    stream.sendall(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n")
                    outcomes.append(True)
            except Exception as error:
                outcomes.append(error)
        thread = threading.Thread(target=serve, daemon=True); thread.start()
        volume = work / "volume"; (volume / "EFI/BOOT").mkdir(parents=True); (volume / "EFI/INFINITY").mkdir(parents=True)
        shutil.copyfile(ROOT / "build/x86_64/BOOTX64.EFI", volume / "EFI/BOOT/BOOTX64.EFI")
        shutil.copyfile(ROOT / "build/native-https/probe.elf", volume / "EFI/INFINITY/KERNEL.ELF")
        with (work / "qemu.log").open("wb") as log:
            result = subprocess.run(["qemu-system-x86_64", "-machine", "pc", "-m", "512M",
                "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
                "-drive", f"format=raw,file=fat:rw:{volume}", "-boot", "order=c", "-netdev", "user,id=net",
                "-device", "e1000,netdev=net", "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
                "-display", "none", "-serial", "stdio", "-no-reboot"], stdout=log, stderr=subprocess.STDOUT, timeout=55)
        thread.join(timeout=2); listener.close()
        assert result.returncode == 33, (result.returncode, outcomes, (work / "qemu.log").read_text(errors="replace")[-1000:])
        assert outcomes == [True], outcomes
        print({"native_https_guest_exit": result.returncode, "installed_acceptance": False})

if __name__ == "__main__":
    run()
