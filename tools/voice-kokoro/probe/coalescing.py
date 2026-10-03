"""Compare sentence splitting and coherent synthesis in the actual native guest."""
from pathlib import Path
import hashlib
import json
import os
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[3]


# ------------------------=
# FUNC: main
# DESC: Builds the standalone probe using the production worker trampoline, asserts deterministic PCM, and records paired synthesis latency.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-kokoro/probe/coalescing.py")
    output = ROOT / "build/voice-kokoro/coalescing"
    output.mkdir(parents=True, exist_ok=True)
    source = ROOT / "tools/voice-kokoro/probe/coalescing-cases.rs"
    (output / "Cargo.toml").write_text('''[package]
name = "infinity-kokoro-coalescing-probe"
version = "0.1.0"
edition = "2021"
[lib]
path = ''' + json.dumps(str(source)) + '''
crate-type = ["staticlib"]
[profile.release]
opt-level = 3
panic = "abort"
lto = true
''')
    target = output / "target"
    environment = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(target))
    subprocess.run(["cargo", "build", "--quiet", "--manifest-path", str(output / "Cargo.toml"),
                    "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat"],
                   cwd=ROOT, env=environment, check=True)
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
                    str(ROOT / "tools/voice-kokoro/probe/link.ld"), "-o", str(output / "probe.elf"),
                    str(target / "aarch64-unknown-none-softfloat/release/libinfinity_kokoro_coalescing_probe.a"),
                    str(ROOT / "build/voice-kokoro/aarch64/private-native.o")], check=True)
    result = output / "result.bin"
    result.unlink(missing_ok=True)
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", "hvf", "-cpu", "host",
                    "-m", "2G", "-display", "none", "-serial", "file:" + str(result), "-monitor", "none",
                    "-kernel", str(output / "probe.elf")], check=True, timeout=120)
    data = result.read_bytes()
    offset = 0
    cases = []
    for index, split in enumerate((True, False, False, True)):
        magic, case, variant, status, frames, ticks, frequency = struct.unpack_from("<7Q", data, offset)
        offset += 56
        assert (magic, case, variant, status) == (0x494e46434f414c31, index, int(split), 0)
        assert frequency > 0 and 2400 <= frames <= 720000
        pcm = data[offset:offset + frames * 2]
        assert len(pcm) == frames * 2 and any(pcm)
        offset += len(pcm)
        cases.append(dict(split=split, seconds=ticks / frequency, frames=frames,
                          pcm_sha256=hashlib.sha256(pcm).hexdigest()))
    assert offset == len(data)
    assert cases[0]["pcm_sha256"] == cases[3]["pcm_sha256"]
    assert cases[1]["pcm_sha256"] == cases[2]["pcm_sha256"]
    split_seconds = (cases[0]["seconds"] + cases[3]["seconds"]) / 2
    joined_seconds = (cases[1]["seconds"] + cases[2]["seconds"]) / 2
    evidence = dict(environment="freestanding ARM64 guest", installed_verified=False, cases=cases,
                    split_seconds=split_seconds, joined_seconds=joined_seconds,
                    latency_reduction_percent=100 * (1 - joined_seconds / split_seconds))
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
