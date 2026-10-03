"""Bounded mixed-engine native lifetime regression; not installed playback proof."""
from pathlib import Path
import json
import hashlib
import os
import struct
import subprocess
import wave

ROOT = Path(__file__).resolve().parents[3]


# ------------------------=
# FUNC: main
# DESC: Reuses the committed worker harness to capture varied native speech transitions, original faults and deterministic repeat results.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-kokoro/probe/lifetime.py")
    output = ROOT / "build/voice-kokoro/lifetime"
    output.mkdir(parents=True, exist_ok=True)
    fixture = output / "jfk-10s.raw"
    with wave.open(str(ROOT / "build/voice-whisper-src/samples/jfk.wav"), "rb") as recording:
        assert (recording.getnchannels(), recording.getsampwidth(), recording.getframerate()) == (1, 2, 16000)
        pcm = recording.readframes(160000)
    assert hashlib.sha256(pcm).hexdigest() == "65ebc63b62dc5aa2a5c068e3d0939110065692db500494388746a1eba0a29d9e"
    fixture.write_bytes(pcm)
    template = (ROOT / "tools/voice-kokoro/probe/coalescing-cases.rs").read_text()
    boundary = "// ------------------------=\n// FUNC: speech_cases\n"
    assert template.count(boundary) == 1
    source = template.split(boundary)[0].replace('"b exception"', '"b lifetime_vector"')
    source = source.replace('"../../../boot/aarch64/handoff.S"', json.dumps(str(ROOT / "boot/aarch64/handoff.S")))
    source += (ROOT / "tools/voice-kokoro/probe/lifetime-cases.rs").read_text()
    (output / "probe.rs").write_text(source)
    (output / "Cargo.toml").write_text('''[package]
name = "infinity-kokoro-lifetime-probe"
version = "0.1.0"
edition = "2021"
[lib]
path = "probe.rs"
crate-type = ["staticlib"]
[profile.release]
opt-level = 3
panic = "abort"
lto = true
''')
    target = output / "target"
    environment = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(target), INFINITY_STT_FIXTURE=str(fixture))
    subprocess.run(["cargo", "build", "--quiet", "--manifest-path", str(output / "Cargo.toml"),
                    "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat"],
                   cwd=ROOT, env=environment, check=True)
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
                    str(ROOT / "tools/voice-kokoro/probe/link.ld"), "-o", str(output / "probe.elf"),
                    str(target / "aarch64-unknown-none-softfloat/release/libinfinity_kokoro_lifetime_probe.a"),
                    str(ROOT / "build/voice-kokoro/aarch64/private-native.o")], check=True)
    result = output / "result.bin"
    result.unlink(missing_ok=True)
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", "hvf", "-cpu", "host",
                    "-m", "2G", "-display", "none", "-serial", "file:" + str(result), "-monitor", "none",
                    "-kernel", str(output / "probe.elf")], check=True, timeout=180)
    data = result.read_bytes()
    rows = []
    offset = 0
    fault = None
    while offset < len(data):
        magic, = struct.unpack_from("<Q", data, offset)
        if magic == 0x494e464c49464546:
            values = struct.unpack_from("<26Q", data, offset)
            fault = dict(esr=hex(values[1]), pc=hex(values[2]), far=hex(values[3]), lr=hex(values[4]),
                         argument=hex(values[5]), diagnostics=values[6:18], frames=[hex(v) for v in values[18:]])
            offset += 208
            break
        assert magic == 0x494e464c49464531, data[offset:offset+32].hex()
        values = struct.unpack_from("<27Q", data, offset)
        offset += 216
        rows.append(dict(case=values[1], kind=values[2], status=values[3], frames=values[4],
                         seconds=values[5]/values[6], diagnostics=values[7:19], pcm_fnv64=hex(values[19]),
                         memory=dict(zip(("committed", "live", "free", "top_free", "context_capacity", "context_used", "bad_pointer"), values[20:27]))))
    evidence = dict(installed_verified=False, fault=fault, transitions=rows)
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(dict(evidence=str(output / "evidence.json"), fault=fault,
                          transitions=[dict(case=row["case"], kind=row["kind"], status=row["status"],
                                            frames=row["frames"], seconds=row["seconds"], memory=row["memory"])
                                       for row in rows]), indent=2), flush=True)
    assert fault is None and offset == len(data)
    assert len(rows) == 29 and all(row["status"] == 0 for row in rows[:27])
    finished = [row for row in rows if row["kind"] == 1]
    assert len(finished) == 12 and all(2400 <= row["frames"] <= 720000 for row in finished)
    for first, repeated in zip(finished[:6], finished[6:]):
        assert (first["frames"], first["pcm_fnv64"]) == (repeated["frames"], repeated["pcm_fnv64"])
    # These are full PCM hashes from the retained pre-fix native artifact, not
    # log/prose matching. The metadata-only fix must not change audible samples.
    assert (finished[0]["frames"], finished[0]["pcm_fnv64"]) == (18273, "0x5838af8f1e7ca150")
    assert (finished[1]["frames"], finished[1]["pcm_fnv64"]) == (43486, "0xbefdcb6522e7d813")
    for row in rows[:27]:
        memory = row["memory"]
        assert memory["live"] + memory["free"] == memory["committed"] < 1536 * 1024 * 1024
        assert memory["context_used"] <= memory["context_capacity"]
        assert row["diagnostics"][2:4] == (0, 0) and memory["bad_pointer"] == 0
    assert all(row["memory"]["committed"] <= finished[5]["memory"]["committed"] for row in finished[6:])
    invalid, quarantined = rows[-2:]
    assert invalid["kind"] == 12 and invalid["status"] == 7 and invalid["frames"] == 0
    assert invalid["pcm_fnv64"] == "0x0" and invalid["memory"]["bad_pointer"] == 0xffffffff14000400
    assert invalid["diagnostics"][3] != 0
    assert quarantined["kind"] == 13 and quarantined["status"] == 7 and quarantined["frames"] == 0
    assert quarantined["memory"] == invalid["memory"]


if __name__ == "__main__":
    main()
