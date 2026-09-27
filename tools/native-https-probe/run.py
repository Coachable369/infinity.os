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
import argparse

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: run
# DESC: Builds a test-only trust fixture, boots the native HTTPS client and asserts its binary guest outcome.
# ------------------=
def run(arch="x86_64", rsa=False):
    with tempfile.TemporaryDirectory(prefix="infinity-https-") as temporary:
        work = Path(temporary)
        key, cert, der = (work / name for name in ("key.pem", "cert.pem", "root.der"))
        key_args = ["rsa:2048"] if rsa else ["ec", "-pkeyopt", "ec_paramgen_curve:prime256v1"]
        subprocess.run(["openssl", "req", "-x509", "-newkey"] + key_args + [
            "-nodes", "-keyout", str(key), "-out", str(cert), "-days", "1", "-subj", "/CN=localhost",
            "-addext", "subjectAltName=DNS:localhost", "-addext", "basicConstraints=critical,CA:FALSE",
            "-addext", "extendedKeyUsage=serverAuth"], check=True, capture_output=True)
        subprocess.run(["openssl", "x509", "-in", str(cert), "-outform", "DER", "-out", str(der)], check=True)
        listener = socket.socket(); listener.bind(("127.0.0.1", 0)); listener.listen(1); listener.settimeout(60)
        environment = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(ROOT / "build/native-https/cargo"),
            HTTPS_TEST_PORT=str(listener.getsockname()[1]), HTTPS_TEST_ROOT=str(der), HTTPS_TEST_TIME=str(int(time.time())))
        target = "aarch64-unknown-none" if arch == "aarch64" else "x86_64-unknown-none"
        linker = "linker/aarch64-qemu.ld" if arch == "aarch64" else "linker/x86_64.ld"
        subprocess.run(["cargo", "build", "--release", "-Z", "build-std=core", "--target", target,
            "--manifest-path", "tools/native-https-probe/Cargo.toml"], cwd=ROOT, env=environment, check=True)
        subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "-nostdlib", "-static", "-T", linker,
            "-o", "build/native-https/probe.elf", f"build/native-https/cargo/{target}/release/libinfinity_native_https_probe.a"], cwd=ROOT, check=True)
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
                    lines = bytes(request).split(b"\r\n")
                    assert lines[0].split() == [b"GET", b"/", b"HTTP/1.1"], bytes(request)
                    assert lines[-2:] == [b"", b""], bytes(request)
                    headers = dict((name.lower(), value.strip()) for name, value in
                                   (line.split(b":", 1) for line in lines[1:-2]))
                    assert headers[b"host"] == b"localhost"
                    assert headers[b"connection"] == b"close"
                    assert headers[b"accept-encoding"] == b"identity"
                    stream.sendall(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n")
                    outcomes.append(True)
            except Exception as error:
                outcomes.append(error)
        thread = threading.Thread(target=serve, daemon=True); thread.start()
        volume = work / "volume"; (volume / "EFI/BOOT").mkdir(parents=True); (volume / "EFI/INFINITY").mkdir(parents=True)
        boot = "BOOTAA64.EFI" if arch == "aarch64" else "BOOTX64.EFI"
        shutil.copyfile(ROOT / f"build/{arch}/{boot}", volume / f"EFI/BOOT/{boot}")
        shutil.copyfile(ROOT / "build/native-https/probe.elf", volume / "EFI/INFINITY/KERNEL.ELF")
        with (work / "qemu.log").open("wb") as log:
            machine = (["qemu-system-aarch64", "-machine", "virt", "-cpu", "cortex-a72", "-bios", "/opt/homebrew/share/qemu/edk2-aarch64-code.fd", "-semihosting-config", "enable=on,target=native"]
                if arch == "aarch64" else ["qemu-system-x86_64", "-machine", "pc", "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd", "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"])
            result = subprocess.run(machine + ["-m", "512M",
                "-drive", f"format=raw,file=fat:rw:{volume}", "-boot", "order=c", "-netdev", "user,id=net",
                "-device", "e1000,netdev=net",
                "-display", "none", "-serial", "stdio", "-no-reboot"], stdout=log, stderr=subprocess.STDOUT, timeout=55)
        thread.join(timeout=2); listener.close()
        assert result.returncode == (0 if arch == "aarch64" else 33), (result.returncode, outcomes, (work / "qemu.log").read_text(errors="replace")[-1000:])
        assert outcomes == [True], outcomes
        print({"architecture": arch, "native_https_guest_exit": result.returncode, "installed_acceptance": False})

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=["x86_64", "aarch64"], default="x86_64")
    parser.add_argument("--rsa", action="store_true")
    args = parser.parse_args()
    run(args.arch, args.rsa)
