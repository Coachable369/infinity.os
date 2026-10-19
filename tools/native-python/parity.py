#!/usr/bin/env python3
"""Validate Python runtime bytes and the installer's ObjectStore seed payloads."""
from pathlib import Path
import hashlib
import json
import mmap
import os
import argparse
import contextlib
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: iso_live_kernel
# DESC: Reads the live kernel from the actual published ISO into repository-local disposable storage.
# ------------------=
@contextlib.contextmanager
def iso_live_kernel(iso):
    with tempfile.TemporaryDirectory(prefix="python-iso-parity-", dir=ROOT / "build/tmp") as directory:
        directory = Path(directory)
        efi = directory / "efi.img"
        kernel = directory / "live-kernel.elf"
        subprocess.run(["xorriso", "-osirrox", "on", "-indev", str(iso.resolve()),
                        "-extract", "/efi.img", str(efi)], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        subprocess.run(["mcopy", "-i", str(efi), "::/EFI/INFINITY/KERNEL.ELF", str(kernel)],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        yield kernel

# ------------------------=
# FUNC: main
# DESC: Verifies runtime bytes in both kernels and installer-only seed bytes in the live kernel without requiring redundant installed copies.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/parity.py")
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=("aarch64", "x86_64"))
    parser.add_argument("--iso", type=Path, help="Inspect this packaged live kernel instead of an intermediate kernel.elf")
    args = parser.parse_args()
    if args.iso and not args.arch:
        parser.error("--iso requires --arch")
    evidence = {}
    for arch in (args.arch,) if args.arch else ("aarch64", "x86_64"):
        package = ROOT / "builds/toolchains" / arch
        payloads = {name: (package / name).read_bytes() for name in ("python.elf", "infinity_native_test.so", "python-license.txt")}
        with contextlib.ExitStack() as stack:
            live = (stack.enter_context(iso_live_kernel(args.iso)) if args.iso
                    else ROOT / "build" / arch / "kernel.elf")
            for path, required in ((live, tuple(payloads)),
                                   (ROOT / "build" / arch / "installed-kernel.elf", ("python.elf",))):
                with path.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as image:
                    # The installer seeds the extension and license into native
                    # ObjectStore files; the installed kernel executes Python
                    # but deliberately does not retain duplicate seed blobs.
                    for name in required:
                        payload = payloads[name]
                        assert payload and image.find(payload) >= 0, (arch, path.name, name)
        evidence[arch] = {name: hashlib.sha256(payload).hexdigest() for name, payload in payloads.items()}
    destination = ROOT / "builds/validation/native-python" / (f"parity-{args.arch}.json" if args.arch else "parity.json")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(evidence, indent=2) + "\n")

if __name__ == "__main__":
    main()
