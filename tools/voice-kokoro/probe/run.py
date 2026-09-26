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
        str(output / "native.o")], check=True)
    result = output / ("probe-" + args.accel + ".bin")
    result.unlink(missing_ok=True)
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", args.accel, "-cpu",
        "host" if args.accel == "hvf" else "max", "-m", "2G", "-display", "none",
        "-serial", "file:" + str(result), "-monitor", "none", "-kernel", str(output / "probe.elf")],
        check=True, timeout=180)
    data = result.read_bytes()
    offset = 0
    rows = []
    for case in range(4):
        if len(data)-offset < 48:
            raise RuntimeError("Incomplete guest result: " + data[offset:offset+48].hex())
        version, index, status, count, ticks, frequency = struct.unpack_from("<6Q", data, offset)
        offset += 48
        assert (version,index,status)==(1,case,[0,0,2,1][case]), (version,index,status,count,ticks,frequency)
        assert frequency>0 and count<=720000 and len(data)-offset>=count*2
        pcm = data[offset:offset+count*2]
        offset += len(pcm)
        samples = array.array("h",pcm)
        if case<2:
            assert count>=2400 and any(samples)
            assert sum(abs(v)>=32767 for v in samples)<count//100
            with wave.open(str(output / f"native-{case}.wav"),"wb") as wav:
                wav.setparams((1,2,24000,count,"NONE","not compressed"));wav.writeframes(pcm)
        else:
            assert count==0
        rows.append(dict(case=case,status=status,frames=count,seconds=ticks/frequency,
                         pcm_sha256=hashlib.sha256(pcm).hexdigest()))
    assert offset==len(data)
    evidence=dict(environment="freestanding ARM64 guest",installed_verified=False,cases=rows)
    (output / "probe-evidence.json").write_text(json.dumps(evidence,indent=2)+"\n")
    print(json.dumps(evidence,indent=2))


if __name__ == "__main__":
    main()
