#!/usr/bin/env python3
"""Run a native ARM64 speech probe; never report this as installed acceptance."""
from pathlib import Path
import argparse
import array
import hashlib
import json
import os
import struct
import subprocess
import shutil
import wave

ROOT = Path(__file__).resolve().parents[3]


# ------------------------=
# FUNC: longest_internal_silence
# DESC: Measures the longest non-edge low-energy interval in actual synthesized PCM.
# ------------------=
def longest_internal_silence(samples, rate, floor=96):
    edge = rate // 2
    longest = current = 0
    for sample in samples[edge:len(samples) - edge]:
        if abs(sample) <= floor:
            current += 1
            longest = max(longest, current)
        else:
            current = 0
    return longest / rate


# ------------------------=
# FUNC: main
# DESC: Asserts binary synthesis results and writes actual guest PCM for listening review.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--accel", choices=("hvf", "tcg"), default="hvf")
    parser.add_argument("--firmware", action="store_true")
    parser.add_argument("--recognition-warmup", action="store_true",
                        help="Isolate recognizer initialization with real firmware and the production high-memory speech arena")
    args = parser.parse_args()
    if args.recognition_warmup and not args.firmware:
        parser.error("--recognition-warmup requires --firmware")
    native = ROOT / "build/voice-kokoro/aarch64"
    output = native / "firmware-recognition" if args.recognition_warmup else native / "firmware" if args.firmware else native
    output.mkdir(parents=True, exist_ok=True)
    target = ROOT / "build/voice-kokoro/probe-target"
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(target))
    subprocess.run(["cargo", "build", "--manifest-path", "tools/voice-kokoro/probe/Cargo.toml",
        "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat",
        *(["--features", "recognition-warmup" if args.recognition_warmup else "firmware"] if args.firmware else [])], cwd=ROOT, env=env, check=True)
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
        str(ROOT / ("linker/aarch64-qemu.ld" if args.firmware else "tools/voice-kokoro/probe/link.ld")), "-o", str(output / "probe.elf"),
        str(target / "aarch64-unknown-none-softfloat/release/libinfinity_kokoro_probe.a"),
        str(native / "private-native.o"),
        *(["-e", "infinity_kernel_entry", str(ROOT / "build/aarch64/qwen-math.o")] if args.firmware else [])], check=True)
    result = output / ("probe-" + args.accel + ".bin")
    result.unlink(missing_ok=True)
    boot = ["-kernel", str(output / "probe.elf")]
    if args.firmware:
        esp = output / "esp"
        (esp / "EFI/BOOT").mkdir(parents=True, exist_ok=True)
        (esp / "EFI/INFINITY").mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / "build/aarch64/BOOTAA64.EFI", esp / "EFI/BOOT/BOOTAA64.EFI")
        shutil.copyfile(output / "probe.elf", esp / "EFI/INFINITY/KERNEL.ELF")
        shutil.copyfile("/opt/homebrew/share/qemu/edk2-arm-vars.fd", output / "vars.fd")
        boot = ["-bios", "/opt/homebrew/share/qemu/edk2-aarch64-code.fd", "-smp", "2", "-device", "ramfb",
                "-drive", "if=none,id=esp,format=raw,file=fat:rw:" + str(esp),
                "-device", "virtio-blk-pci,drive=esp"]
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", args.accel, "-cpu",
        "host" if args.accel == "hvf" else "max", "-m", "6G" if args.recognition_warmup else "2G", "-display", "none",
        "-serial", "file:" + str(result), "-monitor", "none", *boot],
        check=True, timeout=180)
    data = result.read_bytes()
    if args.firmware:
        marker = struct.pack("<Q", 0x4953504545434841)
        assert data.count(marker) == 1
        data = data.split(marker, 1)[1]
    if args.recognition_warmup:
        assert len(data) == 20 * 8, {"result_bytes": len(data), "fault_record": data[:64].hex()}
        version, status, heap, ticks, frequency, stack, level, masks, *diagnostics = struct.unpack("<20Q", data)
        assert version == 3 and status == 0 and frequency > 0
        assert 0 < heap <= 1536 * 1024 * 1024 and diagnostics[2] == 0
        assert level in (4, 8) and masks & 0x3c0 == 0x3c0 and stack % 16 == 0
        # Validate the ELF load segment itself, not a symbol name or log string.
        with (output / "probe.elf").open("rb") as stream:
            header = stream.read(64)
            phoff = struct.unpack_from("<Q", header, 32)[0]
            phsize, phcount = struct.unpack_from("<HH", header, 54)
            stream.seek(phoff)
            segments = [struct.unpack_from("<II6Q", stream.read(phsize)) for _ in range(phcount)]
        assert any(s[0] == 1 and s[1] == 6 and s[3] == s[4] == 0x100000000
                   and s[6] == 0x60000000 for s in segments)
        evidence = dict(environment="ARM64 production firmware worker", installed_verified=False,
                        kernel_activity="isolated speech warmup; no desktop or input polling", result=status,
                        heap_bytes=heap, seconds=ticks/frequency, worker_sp=stack, current_el=level,
                        daif=masks, speech_arena_address=0x100000000, speech_arena_bytes=0x60000000,
                        diagnostics=diagnostics)
        (output / "probe-evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
        print(json.dumps(evidence, indent=2))
        return
    offset = 0
    rows = []
    for case in range(10):
        if len(data)-offset < 144:
            raise RuntimeError("Incomplete guest result: " + data[offset:offset+48].hex())
        version, index, status, count, ticks, frequency, stage, heap, failed_allocation, fatal_address = struct.unpack_from("<10Q", data, offset)
        offset += 80
        callers=struct.unpack_from("<8Q",data,offset)
        offset += 64
        assert (version,index,status)==(2,case,[0,0,2,1,2,0,0,0,0,0][case]), dict(version=version,case=index,status=status,frames=count,
            seconds=ticks/frequency,stage=stage,heap=heap,failed_allocation=failed_allocation,fatal_address=hex(fatal_address),callers=[hex(v) for v in callers])
        assert frequency>0 and count<=720000 and len(data)-offset>=count*2
        assert 0 < heap <= 1536*1024*1024 and failed_allocation == 0
        pcm = data[offset:offset+count*2]
        offset += len(pcm)
        samples = array.array("h",pcm)
        if case not in (2,3,4):
            assert count>=2400 and any(samples)
            assert sum(abs(v)>=32767 for v in samples)<count//100
            with wave.open(str(output / f"native-{case}.wav"),"wb") as wav:
                wav.setparams((1,2,24000,count,"NONE","not compressed"));wav.writeframes(pcm)
        else:
            assert count==0
        rows.append(dict(case=case,status=status,frames=count,seconds=ticks/frequency,heap_bytes=heap,
                         longest_internal_silence_seconds=longest_internal_silence(samples, 24000) if count else 0,
                         pcm_sha256=hashlib.sha256(pcm).hexdigest()))
    assert rows[0]["pcm_sha256"] == rows[1]["pcm_sha256"] == rows[5]["pcm_sha256"] == rows[8]["pcm_sha256"]
    assert rows[7]["pcm_sha256"] == rows[9]["pcm_sha256"]
    assert rows[7]["heap_bytes"] == rows[9]["heap_bytes"]
    assert rows[7]["longest_internal_silence_seconds"] <= 0.4, rows[7]
    assert len(data) - offset >= 8
    notice_length, = struct.unpack_from("<Q", data, offset)
    offset += 8
    notice = data[offset:offset + notice_length]
    assert notice == (native / "THIRD-PARTY-NOTICES.txt").read_bytes()
    offset += notice_length
    profile = struct.unpack_from("<256Q", data, offset)
    offset += 256 * 8
    dot_cases, = struct.unpack_from("<Q", data, offset)
    offset += 8
    assert dot_cases == 66 * 8 * 8 * 2
    tile_cases, = struct.unpack_from("<Q", data, offset)
    offset += 8
    assert tile_cases == 5 * 3 * 11
    worker_stack_bytes, = struct.unpack_from("<Q", data, offset)
    offset += 8
    assert worker_stack_bytes < 1024 * 1024 - 4096, worker_stack_bytes
    operations = [dict(operation=i, seconds=profile[i*2]/frequency, calls=profile[i*2+1])
                  for i in range(128) if profile[i*2+1]]
    assert offset==len(data)
    evidence=dict(environment="freestanding ARM64 guest",installed_verified=False,cases=rows,
                  dot_alignment_cases=dot_cases,
                  dot_tile_cases=tile_cases,
                  worker_stack_bytes=worker_stack_bytes,
                  operation_profile=sorted(operations, key=lambda row: row["seconds"], reverse=True),
                  dependency_notices_sha256=hashlib.sha256(notice).hexdigest())
    (output / "probe-evidence.json").write_text(json.dumps(evidence,indent=2)+"\n")
    print(json.dumps(evidence,indent=2))


if __name__ == "__main__":
    main()
