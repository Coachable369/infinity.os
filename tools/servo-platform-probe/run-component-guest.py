"""Execute the same isolated engine object intended for native desktop linkage."""
import json
import os
from pathlib import Path
import struct
import subprocess
import socket
import argparse

# ------------------------=
# FUNC: main
# DESC: Builds via the active kit and checks actual ABI lifecycle and pixel observations.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    output = root / "build/servo-platform-probe"
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace",action="store_true",help="Enable targeted diagnostics and selector regression")
    options=parser.parse_args()
    subprocess.run(["python3", str(Path(__file__).with_name("link-engine.py")), "--component"] +
                   (["--component-trace"] if options.trace else []), check=True)
    report = json.loads((output / "component-aarch64.json").read_text())
    codegen = json.loads((output / "servo-aarch64-codegen.json").read_text())
    native = []
    for name in ("core", "compiler_builtins"):
        native += ["--extern", name + "=" + codegen["native_archives"][name]]
    image = output / "component-boot.elf"
    subprocess.run(["rustc", "--edition=2021", "--target", "aarch64-unknown-none", "-C", "panic=abort",
        "-C", "linker=/opt/homebrew/opt/lld/bin/ld.lld", "-C", "link-arg=" + report["object"],
        "-C", "link-arg=-T" + str(Path(__file__).with_name("engine-boot.ld")),
        *native, "-L", "dependency=" + str(root / "build/cargo/aarch64-unknown-none/debug/deps"),
        str(Path(__file__).with_name("component-boot.rs")), "-o", str(image)], check=True)
    result = output / "component-boot.bin"
    result.unlink(missing_ok=True)
    monitor = output / "component-qmp.sock"
    monitor.unlink(missing_ok=True)
    process = subprocess.Popen(["qemu-system-aarch64", "-machine", "virt,highmem=off", "-accel", "tcg",
        "-cpu", "max", "-m", "2G", "-display", "none", "-serial", "file:" + str(result),
        "-monitor", "none", "-qmp", "unix:" + str(monitor) + ",server=on,wait=off", "-kernel", str(image)])
    timeout = False
    registers = None
    try:
        process.wait(timeout=90)
    except subprocess.TimeoutExpired:
        timeout = True
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
            connection.settimeout(3)
            connection.connect(str(monitor))
            stream = connection.makefile("rwb")
            stream.readline()
            for command in ({"execute":"qmp_capabilities"},
                            {"execute":"human-monitor-command","arguments":{"command-line":"info registers"}}):
                stream.write((json.dumps(command) + "\n").encode())
                stream.flush()
                while True:
                    response=json.loads(stream.readline())
                    if "return" in response:
                        registers=response["return"]
                        break
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
    data = result.read_bytes()
    records = [list(struct.unpack("<3Q", data[i:i+24])) for i in range(0, len(data)-23, 24)]
    passed = not timeout and bool(records) and records[-1] == [9, 0, 4]
    diagnostic = b"".join(struct.pack("<Q", row[2]) for row in records if row[1] == 4).decode(errors="replace")
    report = {"passed": passed, "records": [row for row in records if row[1] != 4],
              "diagnostic": diagnostic, "registers":registers, "timeout": timeout, "installed_os": False}
    (output / "component-boot.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    return 0 if passed else 1

if __name__ == "__main__":
    raise SystemExit(main())
