#!/usr/bin/env python3
"""Execute production ARM worker entrypoints and assert binary stack/ABI results."""
from pathlib import Path
import json
import os
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]


# ------------------------=
# FUNC: main
# DESC: Builds and runs a bounded native guest through the enclosing build-kit workflow.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/arm-worker-stack-test.py")
    output = ROOT / "build/arm-worker-stack-test"
    output.mkdir(parents=True, exist_ok=True)
    objects = []
    for source in ["tools/arm-worker-stack-test.c", "tools/arm-worker-stack-test.S", "boot/aarch64/handoff.S"]:
        obj = output / (Path(source).name + ".o")
        subprocess.run(["clang", "--target=aarch64-none-elf", "-ffreestanding", "-fno-builtin",
                        "-fno-stack-protector", "-mgeneral-regs-only", "-O2", "-c", source, "-o", str(obj)],
                       cwd=ROOT, check=True)
        objects.append(str(obj))
    elf = output / "probe.elf"
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "-nostdlib", "-static", "-T",
                    str(ROOT / "tools/arm-worker-stack-test.ld"), "-o", str(elf), *objects], check=True)
    result = output / "result.bin"
    result.unlink(missing_ok=True)
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-cpu", "max", "-accel", "tcg",
                    "-m", "64M", "-display", "none", "-monitor", "none", "-serial", "file:" + str(result),
                    "-kernel", str(elf)], check=True, timeout=20)
    words = struct.unpack("<18Q", result.read_bytes())
    assert words[:3] == (0x494e465350303031, 0, 3), words
    evidence = dict(environment="isolated native ARM64 guest", installed_verified=False,
                    mp_stack_modes=[0, 1], psci_entry=True, original_exceptions=words[2],
                    abi_failures=words[1], stacks=[list(words[i:i+5]) for i in range(3,18,5)])
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
