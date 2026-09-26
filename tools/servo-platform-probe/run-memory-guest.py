"""Execute memory primitives in a disposable freestanding ARM64 guest, not the installed OS."""
import json
import os
from pathlib import Path
import struct
import subprocess


# ------------------------=
# FUNC: main
# DESC: Builds through the active kit and checks binary guest assertions rather than serial prose.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    output = root / "build/servo-memory-guest"
    output.mkdir(parents=True, exist_ok=True)
    target = output / "target"
    environment = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(target))
    subprocess.run(["cargo", "build", "--manifest-path", str(Path(__file__).with_name("guest") / "Cargo.toml"),
                    "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat"],
                   env=environment, check=True)
    executable = output / "probe.elf"
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
                    str(Path(__file__).with_name("guest") / "link.ld"), "-o", str(executable),
                    str(target / "aarch64-unknown-none-softfloat/release/libinfinity_servo_memory_probe.a")], check=True)
    result = output / "result.bin"
    result.unlink(missing_ok=True)
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", "tcg", "-cpu", "max",
                    "-m", "32M", "-display", "none", "-serial", "file:" + str(result),
                    "-monitor", "none", "-kernel", str(executable)], check=True, timeout=30)
    record = struct.unpack("<4Q", result.read_bytes())
    if record != (1, 0, 65, 4096):
        raise RuntimeError("Guest memory assertions failed: " + repr(record))
    evidence = dict(environment="freestanding ARM64 QEMU guest", allocations=record[2],
                    arena_bytes=record[3], passed=True, installed_os=False, servo_executed=False)
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
