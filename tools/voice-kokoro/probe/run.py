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
import wave

ROOT = Path(__file__).resolve().parents[3]


# ------------------------=
# FUNC: main
# DESC: Asserts binary synthesis results and writes actual guest PCM for listening review.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--accel", choices=("hvf", "tcg"), default="hvf")
    args = parser.parse_args()
    output = ROOT / "build/voice-kokoro/aarch64"
    target = ROOT / "build/voice-kokoro/probe-target"
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(target))
    subprocess.run(["cargo", "build", "--manifest-path", "tools/voice-kokoro/probe/Cargo.toml",
        "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat"], cwd=ROOT, env=env, check=True)
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
        str(ROOT / "tools/voice-kokoro/probe/link.ld"), "-o", str(output / "probe.elf"),
        str(target / "aarch64-unknown-none-softfloat/release/libinfinity_kokoro_probe.a"),
        str(output / "private-native.o")], check=True)
    result = output / ("probe-" + args.accel + ".bin")
    result.unlink(missing_ok=True)
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", args.accel, "-cpu",
        "host" if args.accel == "hvf" else "max", "-m", "2G", "-display", "none",
        "-serial", "file:" + str(result), "-monitor", "none", "-kernel", str(output / "probe.elf")],
        check=True, timeout=180)
    data = result.read_bytes()
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
        assert 0 < heap <= 1024*1024*1024 and failed_allocation == 0
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
                         pcm_sha256=hashlib.sha256(pcm).hexdigest()))
    assert rows[0]["pcm_sha256"] == rows[1]["pcm_sha256"] == rows[5]["pcm_sha256"] == rows[8]["pcm_sha256"]
    assert rows[7]["pcm_sha256"] == rows[9]["pcm_sha256"]
    assert rows[7]["heap_bytes"] == rows[9]["heap_bytes"]
    assert len(data) - offset >= 8
    notice_length, = struct.unpack_from("<Q", data, offset)
    offset += 8
    notice = data[offset:offset + notice_length]
    assert notice == (output / "THIRD-PARTY-NOTICES.txt").read_bytes()
    offset += notice_length
    profile = struct.unpack_from("<256Q", data, offset)
    offset += 256 * 8
    dot_cases, = struct.unpack_from("<Q", data, offset)
    offset += 8
    assert dot_cases == 66 * 8 * 8 * 2
    tile_cases, = struct.unpack_from("<Q", data, offset)
    offset += 8
    assert tile_cases == 5 * 3 * 11
    operations = [dict(operation=i, seconds=profile[i*2]/frequency, calls=profile[i*2+1])
                  for i in range(128) if profile[i*2+1]]
    assert offset==len(data)
    evidence=dict(environment="freestanding ARM64 guest",installed_verified=False,cases=rows,
                  dot_alignment_cases=dot_cases,
                  dot_tile_cases=tile_cases,
                  operation_profile=sorted(operations, key=lambda row: row["seconds"], reverse=True),
                  dependency_notices_sha256=hashlib.sha256(notice).hexdigest())
    (output / "probe-evidence.json").write_text(json.dumps(evidence,indent=2)+"\n")
    print(json.dumps(evidence,indent=2))


if __name__ == "__main__":
    main()
