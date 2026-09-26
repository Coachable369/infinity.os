"""Execute the actual x86 speech engine on production native background workers."""
from pathlib import Path
import array
import argparse
import json
import os
import shutil
import struct
import subprocess
import wave

ROOT = Path(__file__).resolve().parents[3]

# ------------------------=
# FUNC: main
# DESC: Boots the production loader with an isolated synthesis kernel and verifies returned PCM.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dots-only", action="store_true")
    parser.add_argument("--clock", choices=("deterministic", "realtime"), default="deterministic")
    parser.add_argument("--cpu", choices=("qemu64", "max"), default="max")
    args = parser.parse_args()
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use the repository build kit")
    work = ROOT / "build/voice-kokoro/x86-runtime"
    esp = work / "esp"
    (esp / "EFI/BOOT").mkdir(parents=True, exist_ok=True)
    # A failed rerun must never leave a previous successful audio/evidence result.
    (work / "native-hi.wav").unlink(missing_ok=True)
    (work / "evidence.json").unlink(missing_ok=True)
    (esp / "EFI/INFINITY").mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(work / "target"))
    subprocess.run(["cargo", "build", "--release", "-Z", "build-std=core", "--target", "x86_64-unknown-none",
                    "--manifest-path", "tools/voice-kokoro/probe/x86/Cargo.toml",
                    *(["--features", "dot-only"] if args.dots_only else [])], cwd=ROOT, env=env, check=True)
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
                    str(ROOT / "linker/x86_64.ld"), "-o", str(esp / "EFI/INFINITY/KERNEL.ELF"),
                    str(work / "target/x86_64-unknown-none/release/libinfinity_kokoro_x86_probe.a"),
                    str(ROOT / "build/x86_64/qwen-math.o"),
                    str(ROOT / "build/voice-kokoro/x86_64/private-native.o")], check=True)
    shutil.copyfile(ROOT / "build/x86_64/BOOTX64.EFI", esp / "EFI/BOOT/BOOTX64.EFI")
    shutil.copyfile("/opt/homebrew/share/qemu/edk2-i386-vars.fd", work / "vars.fd")
    # Cross-ISA arithmetic correctness uses deterministic instruction time.
    # This is not native performance evidence: wall-clock TCG previously reached
    # the unchanged production 90-second cancellation deadline during inference.
    clock = (["-accel", "tcg,thread=single", "-icount", "shift=0,sleep=off"]
             if args.clock == "deterministic" else ["-accel", "tcg,thread=multi"])
    result = subprocess.run(["qemu-system-x86_64", "-machine", "q35", "-cpu", args.cpu, *clock, "-smp", "2",
                            "-m", "3G", "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
                            "-drive", "if=pflash,format=raw,file=" + str(work / "vars.fd"),
                            "-drive", "format=raw,file=fat:rw:" + str(esp), "-display", "none", "-monitor", "none",
                            "-serial", "file:" + str(work / "serial.log"), "-debugcon", "file:" + str(work / "result.bin"),
                            "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-no-reboot"], timeout=210)
    data = (work / "result.bin").read_bytes()
    if result.returncode != 33:
        values = list(struct.unpack("<" + "Q" * (len(data) // 8), data)) if len(data) % 8 == 0 else []
        profile = {i: values[8 + i*2:10 + i*2] for i in range(128)
                   if len(values) >= 264 and values[9 + i*2]}
        (work / "evidence.json").write_text(json.dumps(dict(
            completed=False, cpu=args.cpu, guest_exit=result.returncode, clock=args.clock,
            native_latency_verified=False, installed_verified=False,
            synthesis_result=values[:8], operation_profile=profile), indent=2) + "\n")
        raise RuntimeError(f"Native synthesis guest exit status {result.returncode}; binary result {values[:8]}; profile {profile}")
    if args.dots_only:
        assert data == struct.pack("<2Q", 205, int(args.cpu == "max"))
        print("Native x86 SIMD: 205 exact guest dot-product and rejection cases passed")
        return
    version, status, frames, elapsed, heap, allocation_failure, phase, heartbeat = struct.unpack_from("<8Q", data)
    assert version == 1 and status == 0 and 2400 <= frames <= 720000
    assert 0 < elapsed < 180_000_000_000 and heartbeat > 1000
    assert 0 < heap <= 1024**3 and allocation_failure == 0
    assert phase == 4 and len(data) == 2112 + frames * 2
    pcm = data[2112:]
    samples = array.array("h", pcm)
    assert any(samples) and sum(abs(v) >= 32767 for v in samples) < frames // 100
    with wave.open(str(work / "native-hi.wav"), "wb") as output:
        output.setparams((1, 2, 24000, frames, "NONE", "not compressed"))
        output.writeframes(pcm)
    evidence = dict(completed=True, cpu=args.cpu, environment=args.clock + " x86-64 TCG guest, production loader and AP scheduler",
                    installed_verified=False, native_latency_verified=False, dot_product_cases=205,
                    frames=frames, virtual_synthesis_ns=elapsed, heap_bytes=heap, bsp_heartbeat=heartbeat)
    (work / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence))

if __name__ == "__main__":
    main()
