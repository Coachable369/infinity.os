#!/usr/bin/env python3
"""Fresh-installed native Python acceptance through actual console input."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import shutil
import struct
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("native_editor", ROOT / "tools/native-editor-installed-test.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)

# ------------------------=
# FUNC: snapshot
# DESC: Reads seqlocked numeric Python execution evidence from the installed production kernel.
# ------------------=
def snapshot(guest):
    address, size = native.base.symbol(guest.work.parent / "artifacts/installed-kernel.elf", "INFINITY_PYTHON_SNAPSHOT")
    assert size == 80
    values = struct.unpack("<10Q", guest.memory(address, size))
    assert values[:2] == (0x494e46505954484e, 1)
    return values if values[2] == values[9] and not values[2] & 1 else None

# ------------------------=
# FUNC: execute
# DESC: Submits a real command and verifies completion, exit status and process-memory reclamation, not output text.
# ------------------=
def execute(guest, filename):
    before = snapshot(guest)
    assert before is not None
    guest.command("python /home/default/documents/" + filename)
    deadline = time.monotonic() + 150
    while time.monotonic() < deadline:
        value = snapshot(guest)
        if value and value[2] > before[2] and value[3] != 1:
            assert value[3] == 2 and value[4] == 0, value
            assert value[5] > 0 and value[6] == 0, value
            assert value[7] == value[8], value
            return value
        time.sleep(.1)
    raise AssertionError({"python_command_did_not_complete": filename, "state": snapshot(guest)})

# ------------------------=
# FUNC: main
# DESC: Installs unchanged media on a new disposable disk and verifies script and extension execution without installer media attached.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/installed.py --arch ARCH --iso PATH")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), required=True)
    parser.add_argument("--iso", type=Path, required=True)
    args = parser.parse_args()
    iso = args.iso.resolve()
    assert iso.is_relative_to(ROOT) and iso.is_file()
    work = ROOT / "build" / f"browser-installed-python-{args.arch}-{time.time_ns()}"
    artifacts = work / "artifacts"
    artifacts.mkdir(parents=True)
    shutil.copyfile(iso, artifacts / "installer.iso")
    receipt = native.desktop.extract_media_kernels(iso, artifacts, args.arch)
    guest = native.NativeGuest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd", width=1024, height=768, memory_mb=12288)
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" else "tcg"
    guest.patched = True  # Never replace the installed kernel offline.
    guest.install_timeout_seconds = 1800 if args.arch == "x86_64" else 900
    guest.boot_timeout_seconds = 300 if args.arch == "x86_64" else 120
    proof = {"architecture": args.arch, "installed_acceptance": False, "media_kernels": receipt}
    destination = ROOT / "builds/validation/native-python" / args.arch / "installed.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        guest.install()
        guest.stop()
        subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"), str(guest.disk),
                        str(artifacts / "installed-kernel.elf"), "--verify-only"], check=True)
        guest.onboard()
        assert not guest.installer and guest.state()[3] == 1
        guest.launch("command", 5)
        proof["hello"] = execute(guest, "hello.py")
        proof["native_extension"] = execute(guest, "native_demo.py")
        proof["repeat"] = execute(guest, "native_demo.py")
        guest.screenshot("python-console")
        proof["installed_acceptance"] = True
        with iso.open("rb") as stream:
            proof["iso_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
    finally:
        guest.stop()
        proof["work"] = str(work)
        destination.write_text(json.dumps(proof, indent=2) + "\n")

if __name__ == "__main__":
    main()
