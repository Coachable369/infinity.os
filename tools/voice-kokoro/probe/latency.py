#!/usr/bin/env python3
"""Measure native Kokoro with production ARM worker adapters, never the desktop VM."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import wave

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("latency", Path(__file__).with_name("compare-latency.py"))
latency = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(latency)


# ------------------------=
# FUNC: baseline_object
# DESC: Limits historical native inputs to existing regular artifacts resolved inside the repository build directory.
# ------------------=
def baseline_object(path):
    resolved = path.resolve()
    if not resolved.is_relative_to(ROOT / "build") or not resolved.is_file():
        raise ValueError("The baseline native object must be an existing file inside the repository build directory")
    return resolved


# ------------------------=
# FUNC: lifetime_source
# DESC: Reuses the existing mixed-engine fixtures with real firmware workers while disabling stack walking outside an owned probe stack.
# ------------------=
def lifetime_source(output):
    fixture = output / "jfk-10s.raw"
    with wave.open(str(ROOT / "build/voice-whisper-src/samples/jfk.wav"), "rb") as recording:
        if (recording.getnchannels(), recording.getsampwidth(), recording.getframerate()) != (1, 2, 16000):
            raise ValueError("Unexpected native recognition fixture format")
        pcm = recording.readframes(160000)
    if hashlib.sha256(pcm).hexdigest() != "65ebc63b62dc5aa2a5c068e3d0939110065692db500494388746a1eba0a29d9e":
        raise ValueError("Unexpected native recognition fixture contents")
    fixture.write_bytes(pcm)
    base = Path(__file__).with_name("latency-cases.rs").read_text()
    boundary = "// ------------------------=\n// FUNC: cancel\n"
    if base.count(boundary) != 1:
        raise ValueError("Unreviewed firmware probe entry boundary")
    source = base.split(boundary)[0]
    for old, new in (("../../../kernel/core/boot_info.rs", str(ROOT / "kernel/core/boot_info.rs")),
                     ("../../../kernel/runtime/ai/qwen/workers.rs", str(ROOT / "kernel/runtime/ai/qwen/workers.rs"))):
        source = source.replace(json.dumps(old), json.dumps(new))
    source = source.replace("mod dot;", "#[path = " + json.dumps(str(Path(__file__).with_name("dot.rs"))) + "] mod dot;")
    source = source.replace('"b exception"', '"b lifetime_vector"')
    cases = Path(__file__).with_name("lifetime-cases.rs").read_text()
    start = "    let low = (&raw const WORKER_STACK.0).cast::<u8>() as u64;"
    end = "    finish()\n}"
    if cases.count(start) != 1 or cases.count(end) != 1:
        raise ValueError("Unreviewed standalone lifetime fault boundary")
    begin = cases.index(start)
    finish = cases.index(end, begin)
    cases = cases[:begin] + "    // Firmware owns this stack; never infer bounds or walk unrelated memory.\n    for _ in 0..8 { bytes(&0u64.to_le_bytes()); }\n" + cases[finish:]
    entry = 'unsafe extern "C" fn speech_cases(_: u64)'
    if cases.count(entry) != 1:
        raise ValueError("Unreviewed native lifetime entry boundary")
    cases = cases.replace(entry, 'unsafe extern "C" fn lifetime_cases(_: u64)')
    source += cases + '''
// ------------------------=
// FUNC: speech_cases
// DESC: Runs shared-heap Whisper and Kokoro lifetime fixtures on a real AP and emits the joined helper count.
// ------------------=
unsafe fn speech_cases() {
    core::arch::asm!("msr vbar_el1, {}", "isb", in(reg) exception_vectors as *const () as u64);
    bytes(&0x494e464c41544331u64.to_le_bytes());
    bytes(&(workers::online() as u64).to_le_bytes());
    lifetime_cases(0);
    bytes(&(workers::parallel_completed() as u64).to_le_bytes());
    finish()
}
'''
    path = output / "lifetime-probe.rs"
    path.write_text(source)
    return path, fixture


# ------------------------=
# FUNC: decode_lifetime
# DESC: Validates every mixed-engine transition, repeated full-sample hash, heap ownership, PCM erasure, and final native quarantine result.
# ------------------=
def decode_lifetime(data):
    expected = [(0, 10, 0)]
    for turn in range(2):
        expected.append((turn, 11, 0))
        for index in range(6):
            expected.extend(((turn * 6 + index, 0, 0), (turn * 6 + index, 1, 0)))
    expected.extend(((12, 12, 7), (13, 13, 7)))
    if len(data) != len(expected) * 216:
        raise ValueError("Incomplete native lifetime transitions or architectural fault")
    rows = []
    for index, transition in enumerate(expected):
        values = struct.unpack_from("<27Q", data, index * 216)
        if values[0] != 0x494e464c49464531 or tuple(values[1:4]) != transition or not values[6]:
            raise ValueError(f"Invalid native lifetime transition {index}")
        row = dict(case=values[1], kind=values[2], status=values[3], frames=values[4],
                   seconds=values[5] / values[6], diagnostics=values[7:19], pcm_fnv64=hex(values[19]),
                   memory=dict(zip(("committed", "live", "free", "top_free", "context_capacity",
                                    "context_used", "bad_pointer"), values[20:27])))
        if index < 27:
            memory = row["memory"]
            if not (0 < memory["committed"] < 1536 * 1024 * 1024
                    and memory["live"] + memory["free"] == memory["committed"]
                    and memory["context_used"] <= memory["context_capacity"]
                    and row["diagnostics"][2:4] == (0, 0) and memory["bad_pointer"] == 0):
                raise ValueError(f"Invalid native lifetime memory state {index}")
            if row["kind"] == 11 and not 0 < row["frames"] <= 512:
                raise ValueError("Recognition did not publish a bounded transcript")
        rows.append(row)
    finished = [row for row in rows if row["kind"] == 1]
    if any(not 2400 <= row["frames"] <= 720000 or not row["seconds"] for row in finished):
        raise ValueError("Invalid native lifetime speech result")
    for first, repeated in zip(finished[:6], finished[6:]):
        if (first["frames"], first["pcm_fnv64"]) != (repeated["frames"], repeated["pcm_fnv64"]):
            raise ValueError("Mixed recognition and synthesis changed repeated PCM")
    if (finished[0]["frames"], finished[0]["pcm_fnv64"]) != (18273, "0x5838af8f1e7ca150") or \
            (finished[1]["frames"], finished[1]["pcm_fnv64"]) != (43486, "0xbefdcb6522e7d813"):
        raise ValueError("Known native complete-PCM baseline changed")
    if any(row["memory"]["committed"] > finished[5]["memory"]["committed"] for row in finished[6:]):
        raise ValueError("Repeated native workload grew the bounded heap")
    invalid, quarantined = rows[-2:]
    if (invalid["frames"] != 0 or invalid["pcm_fnv64"] != "0x0"
            or invalid["memory"]["bad_pointer"] != 0xffffffff14000400
            or not invalid["diagnostics"][3] or quarantined["frames"] != 0
            or quarantined["memory"] != invalid["memory"]):
        raise ValueError("Native fatal ownership, full-buffer erasure, or quarantine failed")
    return rows


# ------------------------=
# FUNC: run
# DESC: Executes a bounded native build or probe within the enclosing build-kit run.
# ------------------=
def run(*command, env=None, timeout=240):
    subprocess.run([str(value) for value in command], cwd=ROOT, env=env, check=True, timeout=timeout)


# ------------------------=
# FUNC: main
# DESC: Builds one firmware-loaded native probe, verifies real helper use and full PCM, and records replayable binary evidence.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-kokoro/probe/latency.py")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cpus", type=int, choices=(2, 4, 8), default=8)
    parser.add_argument("--label", required=True)
    parser.add_argument("--serial", action="store_true", help="Leave the optional helper hook unbound in a test-only object copy")
    parser.add_argument("--baseline-object", type=Path, help="Measure a preserved pre-optimization object under build/ with the same firmware and ten PCM fixtures")
    parser.add_argument("--lifetime", action="store_true", help="Run shared-heap Whisper/Kokoro lifetime and quarantine acceptance")
    parser.add_argument("--profile", action="store_true", help="Capture explicitly instrumented diagnostic operation totals, not release latency")
    args = parser.parse_args()
    if args.lifetime and args.profile:
        parser.error("Lifetime and diagnostic profiling use distinct evidence formats")
    if args.baseline_object is not None and (args.serial or args.lifetime or args.profile):
        parser.error("The preserved baseline is a distinct uninstrumented ten-case latency measurement")
    baseline = None
    if args.baseline_object is not None:
        try:
            baseline = baseline_object(args.baseline_object)
        except ValueError as error:
            parser.error(str(error))
    if not args.label or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789-_" for c in args.label):
        parser.error("Label must use lowercase letters, digits, hyphens or underscores")
    output = ROOT / "build/voice-kokoro/native-latency" / args.label
    output.mkdir(parents=True, exist_ok=True)
    source = Path(__file__).with_name("latency-cases.rs")
    fixture = None
    if args.lifetime:
        source, fixture = lifetime_source(output)
    cargo = output / "Cargo.toml"
    cargo.write_text('''[package]
name = "infinity-native-latency-probe"
version = "0.1.0"
edition = "2021"
[lib]
path = ''' + json.dumps(str(source)) + '''
crate-type = ["staticlib"]
[features]
baseline-math = []
[profile.release]
opt-level = 3
panic = "abort"
lto = true
''')
    target = ROOT / "build/voice-kokoro/native-latency/target"
    helpers_expected = args.cpus > 2 and not args.serial and baseline is None
    environment = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(target),
                       INFINITY_PROBE_WORKERS=str(args.cpus - 1),
                       INFINITY_PROBE_HELPERS="1" if helpers_expected else "0")
    if fixture is not None:
        environment["INFINITY_STT_FIXTURE"] = str(fixture)
    features = ("--features", "baseline-math") if baseline is not None else ()
    run("cargo", "build", "--quiet", "--manifest-path", cargo, "--release", "-Z", "build-std=core",
        "--target", "aarch64-unknown-none-softfloat", *features, env=environment)
    math_object = output / "worker-math.o"
    run("/opt/homebrew/opt/llvm/bin/clang", "--target=aarch64-none-elf", "-O3", "-ffreestanding",
        "-mstrict-align", "-ffp-contract=off", "-fno-stack-protector", "-c",
        "kernel/runtime/ai/qwen/cpu_math.c", "-o", math_object)
    native = baseline if baseline is not None else ROOT / "build/voice-kokoro/aarch64/private-native.o"
    linked_native = native
    if args.serial:
        linked_native = output / "serial-native.o"
        # Same model, math and firmware/CPU count; only the optional kernel
        # helper hook is absent. Never modify the object used by ISO builds.
        run("/opt/homebrew/opt/llvm/bin/llvm-objcopy", "--redefine-sym",
            "infinity_speech_parallel=infinity_probe_unbound_parallel", native, linked_native)
    elf = output / "probe.elf"
    run("/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T", "linker/aarch64-qemu.ld",
        "-o", elf, target / "aarch64-unknown-none-softfloat/release/libinfinity_native_latency_probe.a",
        math_object, linked_native)
    esp = output / "esp"
    (esp / "EFI/BOOT").mkdir(parents=True, exist_ok=True)
    (esp / "EFI/INFINITY").mkdir(parents=True, exist_ok=True)
    loader = ROOT / "build/aarch64/BOOTAA64.EFI"
    shutil.copyfile(loader, esp / "EFI/BOOT/BOOTAA64.EFI")
    shutil.copyfile(elf, esp / "EFI/INFINITY/KERNEL.ELF")
    serial = output / "serial.bin"
    serial.unlink(missing_ok=True)
    run("qemu-system-aarch64", "-machine", "virt", "-accel", "hvf", "-cpu", "host", "-m", "3G",
        "-smp", args.cpus, "-display", "none", "-serial", "file:" + str(serial), "-monitor", "none",
        "-bios", "/opt/homebrew/share/qemu/edk2-aarch64-code.fd", "-device", "ramfb",
        "-drive", "if=none,id=esp,format=raw,file=fat:rw:" + str(esp), "-device", "virtio-blk-pci,drive=esp")
    marker = struct.pack("<Q", 0x494e464c41544331)
    data = serial.read_bytes()
    if data.count(marker) != 1:
        raise ValueError("Native probe did not publish a unique result header")
    payload = data.split(marker, 1)[1]
    online, = struct.unpack_from("<Q", payload)
    helpers, = struct.unpack_from("<Q", payload, len(payload) - 8)
    binary = payload[8:-8]
    cases = decode_lifetime(binary) if args.lifetime else latency.decode(binary, allow_profile=args.profile)[0]
    profile = None
    if args.profile:
        values = struct.unpack_from("<256Q", binary, len(binary) - 2072)
        if not any(values):
            raise ValueError("Diagnostic profiling requires an instrumented native object")
        frequency, = struct.unpack_from("<Q", binary, 40)
        profile = dict(frequency_hz=frequency, counters=list(values))
    if online != args.cpus - 1 or bool(helpers) != helpers_expected:
        raise ValueError(f"Unexpected native helper execution: online={online}, helpers={helpers}")
    (output / "probe-hvf.bin").write_bytes(binary)
    rows = []
    for case in cases:
        if args.lifetime:
            rows.append(case)
            continue
        pcm = case.pop("pcm")
        if pcm:
            with wave.open(str(output / f"native-{case['case']}.wav"), "wb") as wav:
                wav.setparams((1, 2, 24000, case["frames"], "NONE", "not compressed"))
                wav.writeframes(pcm)
        rows.append(dict(case, pcm_sha256=hashlib.sha256(pcm).hexdigest()))
    with linked_native.open("rb") as image:
        native_hash = hashlib.file_digest(image, "sha256").hexdigest()
    evidence = dict(environment="firmware-loaded native ARM64 workers", installed_verified=False,
                    cpus=args.cpus, helpers_enabled=helpers_expected, online_workers=online,
                    helper_completions=helpers, cases=rows,
                    mixed_engine_lifetime=args.lifetime,
                    diagnostic_profile=args.profile,
                    preserved_baseline=baseline is not None,
                    native_object_sha256=native_hash,
                    boot_loader_sha256=hashlib.sha256(loader.read_bytes()).hexdigest())
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    if profile is not None:
        (output / "profile-evidence.json").write_text(json.dumps(profile, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
