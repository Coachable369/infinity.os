"""Artifact-level parity for the native compiler embedded in installed kernels."""
from pathlib import Path
import hashlib
import json
import mmap
import argparse

ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: main
# DESC: Verifies actual compiler, linker and isolated CRT bytes in both live and installed kernel artifacts on both architectures.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=("aarch64", "x86_64"))
    args = parser.parse_args()
    evidence = {}
    for arch in ((args.arch,) if args.arch else ("aarch64", "x86_64")):
        package = ROOT / "builds/toolchains" / arch
        assets = {name: (package / name).read_bytes() for name in ("clang.elf", "lld.elf", "application-crt.o")}
        for name in ("console.h", "syscall.h", "uikit.h", "interaction.h", "app.h", "app_api.h"):
            assets["infinity/" + name] = (ROOT / "sdk/c/include/infinity" / name).read_bytes()
        results = {}
        for kernel in ("kernel.elf", "installed-kernel.elf"):
            path = ROOT / "build" / arch / kernel
            assert path.stat().st_size <= 1024 ** 3, (arch, kernel, "exceeds native kernel extent")
            with path.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as data:
                for name, binary in assets.items():
                    assert data.find(binary) >= 0, (arch, kernel, name)
            results[kernel] = {name: hashlib.sha256(binary).hexdigest() for name, binary in assets.items()}
        evidence[arch] = results
    destination = ROOT / "builds/validation/native-apps" / (f"compiler-parity-{args.arch}.json" if args.arch else "compiler-parity.json")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence))

if __name__ == "__main__":
    main()
