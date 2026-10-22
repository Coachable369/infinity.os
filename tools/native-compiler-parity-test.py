"""Artifact-level parity for the native compiler embedded in installed kernels."""
from pathlib import Path
import hashlib
import json
import mmap
import argparse
import contextlib
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: iso_live_kernel
# DESC: Extracts the published live kernel into bounded-lifetime repository-local storage for artifact checks, not guest acceptance.
# ------------------=
@contextlib.contextmanager
def iso_live_kernel(iso):
    with tempfile.TemporaryDirectory(prefix="camera-compiler-parity-", dir=ROOT / "build/tmp") as directory:
        efi = Path(directory) / "efi.img"
        kernel = Path(directory) / "kernel.elf"
        subprocess.run(["xorriso", "-osirrox", "on", "-indev", str(iso.resolve()),
                        "-extract", "/efi.img", str(efi)], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        subprocess.run(["mcopy", "-i", str(efi), "::/EFI/INFINITY/KERNEL.ELF", str(kernel)],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        yield kernel

# ------------------------=
# FUNC: main
# DESC: Verifies actual compiler, linker and isolated CRT bytes in both live and installed kernel artifacts on both architectures.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=("aarch64", "x86_64"))
    source = parser.add_mutually_exclusive_group()
    source.add_argument("--live-kernel", type=Path)
    source.add_argument("--iso", type=Path)
    parser.add_argument("--installed-kernel", type=Path)
    args = parser.parse_args()
    if (args.live_kernel or args.iso or args.installed_kernel) and not args.arch:
        parser.error("explicit artifacts require --arch")
    evidence = {}
    for arch in ((args.arch,) if args.arch else ("aarch64", "x86_64")):
        package = ROOT / "builds/toolchains" / arch
        assets = {name: (package / name).read_bytes() for name in ("clang.elf", "lld.elf", "application-crt.o")}
        for name in ("console.h", "camera.h", "syscall.h", "uikit.h", "interaction.h", "app.h", "app_api.h"):
            assets["infinity/" + name] = (ROOT / "sdk/c/include/infinity" / name).read_bytes()
        assets["camera_capture.c"] = (ROOT / "sdk/c/examples/camera_capture.c").read_bytes()
        results = {}
        with contextlib.ExitStack() as stack:
            live = (stack.enter_context(iso_live_kernel(args.iso)) if args.iso else
                    args.live_kernel or ROOT / "build" / arch / "kernel.elf")
            installed = args.installed_kernel or ROOT / "build" / arch / "installed-kernel.elf"
            for kernel, path in (("kernel.elf", live), ("installed-kernel.elf", installed)):
                # Match the loader's bounded 1.5 GiB reservation.
                assert path.stat().st_size <= 1536 * 1024 ** 2, (arch, kernel, "exceeds native kernel extent")
                with path.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as data:
                    for name, binary in assets.items():
                        assert binary and data.find(binary) >= 0, (arch, kernel, name)
                results[kernel] = {name: hashlib.sha256(binary).hexdigest() for name, binary in assets.items()}
        evidence[arch] = results
    destination = ROOT / "builds/validation/native-apps" / (f"compiler-parity-{args.arch}.json" if args.arch else "compiler-parity.json")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence))

if __name__ == "__main__":
    main()
