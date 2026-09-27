"""Link the browser component with the real installed kernel, without publishing an ISO."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import struct


# ------------------------=
# FUNC: elf_footprint
# DESC: Measures actual ELF load segments and rejects malformed, overlapping or writable-executable mappings.
# ------------------=
def elf_footprint(path, arch):
    with path.open("rb") as source:
        header = source.read(64)
        if header[:6] != b"\x7fELF\x02\x01":
            raise ValueError("Expected little-endian ELF64")
        machine = struct.unpack_from("<H", header, 18)[0]
        if machine != (183 if arch == "aarch64" else 62):
            raise ValueError("Wrong kernel architecture")
        entry, table = struct.unpack_from("<QQ", header, 24)
        size, count = struct.unpack_from("<HH", header, 54)
        if size != 56 or not 0 < count < 128:
            raise ValueError("Invalid program header table")
        loads = []
        for index in range(count):
            source.seek(table + index * size)
            kind, flags, offset, address, _, disk, memory, _ = struct.unpack("<II6Q", source.read(size))
            if kind != 1:
                continue
            if memory < disk or flags & 3 == 3 or offset + disk > path.stat().st_size:
                raise ValueError("Invalid native load segment")
            loads.append((address, address + memory, disk, memory, flags))
    loads.sort()
    if not loads or any(left[1] > right[0] for left, right in zip(loads, loads[1:])):
        raise ValueError("Overlapping load segments")
    if not any(start <= entry < end and flags & 1 for start, end, _, _, flags in loads):
        raise ValueError("Kernel entry is not executable")
    return dict(load_file_bytes=sum(row[2] for row in loads),
                load_memory_bytes=sum(row[3] for row in loads),
                virtual_span_bytes=loads[-1][1]-loads[0][0], load_segments=len(loads))


# ------------------------=
# FUNC: main
# DESC: Verifies native component coexistence with production kernel and speech providers in an isolated build directory.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), default="aarch64")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    output = root / "build/servo-platform-probe"
    arch = args.arch
    target = "aarch64-unknown-none-softfloat" if arch == "aarch64" else "x86_64-unknown-none"
    work = output / ("kernel-" + arch)
    work.mkdir(parents=True, exist_ok=True)
    component = json.loads((output / ("component-" + arch + ".json")).read_text())
    if any(line.split()[0] == "U" for line in component["undefined"].splitlines() if line.split()):
        raise SystemExit("Component has unresolved required symbols")
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(work / "cargo"))
    with (work / "compile.log").open("w") as log:
        result = subprocess.run(["cargo", "build", "--release", "-Z", "build-std=core", "--target", target,
                        "--features", "native-browser", "--manifest-path", str(root / "Cargo.toml")],
                       cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
    if result.returncode:
        raise SystemExit("Kernel compilation failed; inspect " + str(work / "compile.log"))
    image = work / "installed-kernel.elf"
    sources = [work / "cargo" / target / "release/libinfinity_kernel.a",
               root / "build" / arch / "qwen-math.o",
               root / "build/voice-kokoro" / arch / "private-native.o",
               root / ("build/voice-pocketsphinx-arm/private-native.o" if arch == "aarch64"
                       else "build/voice-pocketsphinx-x86_64/private-native.o"),
               Path(component["object"])]
    if not all(path.is_file() for path in sources):
        raise SystemExit("Production native dependencies must be built through the build kit first")
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "-nostdlib", "-static",
                    "--undefined=infinity_browser_private_infinity_browser_run",
                    "-T", str(root / "linker" / (arch + ".ld")), "-o", str(image),
                    *map(str, sources)], check=True)
    report = dict(arch=arch, image=str(image), bytes=image.stat().st_size,
                  installed_configuration=True, executed=False, release_iso_updated=False,
                  **elf_footprint(image, arch))
    (work / "link.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
