#!/usr/bin/env python3
"""Synthetic native HDA output evidence; never records user speech or builds an ISO."""
from pathlib import Path
import argparse
import array
import hashlib
import importlib.util
import json
import os
import struct
import subprocess
import sys
import time
import wave
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "build/arm-capture-timer-probe"
OUT = BASE / "playback"
MAGIC = 0x494E46504C415942
LENGTHS = [79906, 98615, 119473, 116285]


# ------------------------=
# FUNC: command
# DESC: Executes only bounded explicit commands owned by this build-kit workflow.
# ------------------=
def command(args, **kwargs):
    print("+ " + " ".join(map(str, args)), flush=True)
    return subprocess.run(list(map(str, args)), cwd=ROOT, check=True, **kwargs)


# ------------------------=
# FUNC: build
# DESC: Links production HDA into a native synthetic diagnostic and reuses the current production loader.
# ------------------=
def build(short_periods=False, production_long=False):
    OUT.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(OUT / "target"))
    args = ["cargo", "build", "--manifest-path", "tools/arm-capture-timer-probe/playback/Cargo.toml",
            "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat"]
    if short_periods:
        args.extend(["--features", "short-periods"])
    if production_long:
        args.extend(["--features", "production-long"])
    command(args, env=env)
    command(["/opt/homebrew/opt/lld/bin/ld.lld", "-nostdlib", "--gc-sections", "-T", "linker/aarch64.ld",
             "-o", OUT / "probe.elf", OUT / "target/aarch64-unknown-none-softfloat/release/libinfinity_playback_timer_probe.a"])
    spec = importlib.util.spec_from_file_location("capture_runner", Path(__file__).with_name("run.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    helper.OUT = OUT
    loader = ROOT / "build/aarch64/BOOTAA64.EFI"
    helper.disk_image(loader)
    evidence = {"loader": str(loader.relative_to(ROOT)), "loader_sha256": hashlib.sha256(loader.read_bytes()).hexdigest(),
                "disk_sha256": hashlib.sha256((OUT / "probe.raw").read_bytes()).hexdigest(),
                "hda_sha256": hashlib.sha256((ROOT / "kernel/drivers/hda.rs").read_bytes()).hexdigest(),
                "iso_generated": False, "short_periods": short_periods, "production_long": production_long}
    (OUT / "build-evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")


# ------------------------=
# FUNC: fields
# DESC: Reads typed machine configuration properties before touching only the approved disposable VM.
# ------------------=
def fields(name):
    result = subprocess.run(["VBoxManage", "showvminfo", name, "--machinereadable"], capture_output=True, text=True)
    assert result.returncode == 0, "The previously approved disposable VM must already exist"
    return dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)


# ------------------------=
# FUNC: decode
# DESC: Decodes native binary words and asserts actual playback progress and reserved-worker policy.
# ------------------=
def decode(data, short_periods=False, production_long=False):
    marker = struct.pack("<Q", MAGIC).hex().encode() + b"\n"
    count = 9 if production_long else 8
    word_count = 12 + count * 9 + 2
    starts = []
    offset = data.find(marker)
    while offset >= 0:
        starts.append(offset)
        offset = data.find(marker, offset + len(marker))
    values = None
    for start in reversed(starts):
        try:
            raw = bytes.fromhex(data[start:].decode())
            packet = struct.unpack(f"<{word_count}Q", raw[:word_count * 8])
            if (packet[:2] == (MAGIC, 3 if production_long else 2 if short_periods else 1)
                    and packet[4] == 6 and packet[5:12] == tuple(range(7)) and packet[-2:] == (MAGIC, 0)):
                values = packet
                break
        except (ValueError, struct.error):
            continue
    assert values is not None, "No complete native structured packet; use independent WAV and typed-host evidence"
    rows = []
    for index in range(count):
        row = values[12 + index * 9:21 + index * 9]
        assert row[0] == index and row[1] == (values[2] * 32 if index == 8 else LENGTHS[index % 4])
        assert row[4] == row[1] * 4
        if not production_long:
            assert row[8] >= row[4] + 65536
        rows.append(dict(index=index, frames=row[1], seconds=(row[3] - row[2]) / values[3],
                         max_cursor_gap_seconds=row[5] / values[3], polls=row[6], capture_frames=row[7],
                         final_lpib=row[8]))
    return dict(sample_rate=values[2], phases=rows, requested_workers=7, started_workers=6,
                claimed_cpu_ids=list(values[5:12]), installed_verified=False, short_periods=short_periods,
                production_long=production_long)


# ------------------------=
# FUNC: counters
# DESC: Extracts typed numeric HDA counters from VirtualBox's structured statistics XML.
# ------------------=
def counters(path):
    root = ET.fromstring(path.read_bytes())
    result = {}
    for child in root:
        name = child.attrib.get("name")
        key = "c" if child.tag == "Counter" else "cPeriods" if child.tag == "Profile" else "val"
        if name and key in child.attrib:
            result[name] = int(child.attrib[key])
    return result


# ------------------------=
# FUNC: reference
# DESC: Recreates exactly the probe's synthetic integer triangle markers for a byte-level output oracle.
# ------------------=
def reference(index, rate, production_long=False):
    count = rate * 32 if index == 8 else LENGTHS[index % 4]
    samples = array.array("h")
    for frame in range(count):
        period = 32 + index * 2 if frame + rate // 5 >= count else 96 + index * 8
        phase = frame % period
        value = -4000 + phase * 16000 // period if phase < period // 2 else 12000 - phase * 16000 // period
        if production_long:
            x = frame ^ ((index + 1) * 0x9E3779B9 & 0xFFFFFFFF)
            x = (x ^ (x >> 16)) * 0x7FEB352D & 0xFFFFFFFF
            x = (x ^ (x >> 15)) * 0x846CA68B & 0xFFFFFFFF
            value += ((x ^ (x >> 16)) & 15) - 8
        samples.extend((value, value))
    if sys.byteorder != "little":
        samples.byteswap()
    return samples.tobytes()


# ------------------------=
# FUNC: recordings
# DESC: Compares synthesized guest samples against HDA DMA and mixer-delivered WAV bytes, never prose.
# ------------------=
def recordings(directory, rate, production_long=False):
    result = []
    expected = [reference(index, rate, production_long) for index in range(9 if production_long else 8)]
    for path in sorted(directory.glob("*.wav")):
        if not any(part in path.name for part in ("ReadSD", "DrvAudioPlay")):
            continue
        try:
            with wave.open(str(path), "rb") as stream:
                shape = (stream.getframerate(), stream.getnchannels(), stream.getsampwidth())
                pcm = stream.readframes(stream.getnframes())
        except (wave.Error, EOFError) as error:
            result.append(dict(file=path.name, decode_error=str(error)))
            continue
        row = dict(file=path.name, rate=shape[0], channels=shape[1], sample_bytes=shape[2], pcm_bytes=len(pcm))
        if shape == (rate, 2, 2):
            offsets = [pcm.find(value) for value in expected]
            row["complete_utterance_offsets"] = offsets
            row["complete_utterances"] = sum(offset >= 0 for offset in offsets)
            row["complete_utterance_counts"] = [pcm.count(value) for value in expected]
            row["last_marker_present"] = [pcm.find(value[-rate // 5 * 4:]) >= 0 for value in expected]
        result.append(row)
    return result


# ------------------------=
# FUNC: analyze
# DESC: Retains independent PCM and typed-host evidence even when diagnostic UART words were lost.
# ------------------=
def analyze(attempt, short_periods=False, production_long=False):
    attempt = Path(attempt).resolve()
    assert attempt.is_relative_to(BASE)
    stats = counters(attempt / "hda-statistics.txt")
    rate = stats["/Devices/hda/Stream4/Cfg/Hz"]
    try:
        evidence = decode((attempt / "serial.bin").read_bytes(), short_periods, production_long)
        evidence["native_telemetry_complete"] = True
    except (AssertionError, ValueError, struct.error) as error:
        evidence = dict(sample_rate=rate, native_telemetry_complete=False, native_decode_error=str(error),
                        installed_verified=False, short_periods=short_periods, production_long=production_long)
    evidence["recordings"] = recordings(attempt / "synthetic-output", rate, production_long)
    evidence["hda_counters"] = stats
    evidence["artifact_directory"] = str(attempt.relative_to(ROOT))
    evidence["host_microphone_enabled"] = False
    raw = next((row for row in evidence["recordings"] if row["file"].startswith("hdaDMARawReadSD4")), {})
    delivered = next((row for row in evidence["recordings"] if row["file"].startswith("hdaStreamReadSD4")), {})
    outputs = (raw, delivered)
    evidence["waveform_acceptance"] = all(
        row.get("complete_utterance_counts") == [1] * (9 if production_long else 8)
        and row["complete_utterance_offsets"] == sorted(row["complete_utterance_offsets"])
        for row in outputs
    ) and stats.get("/Devices/hda/Stream4/DMABufferOverflows", 0) == 0
    (attempt / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2), flush=True)
    if short_periods or production_long:
        assert evidence["waveform_acceptance"], "Missing, duplicated, or reordered synthetic PCM, or a host DMA discard"
    return evidence


# ------------------------=
# FUNC: run
# DESC: Records only synthetic playback from the explicitly approved isolated VM with host microphone disabled.
# ------------------=
def run(name, short_periods=False, production_long=False):
    assert name == "infinity-audio-timer-test"
    current = fields(name)
    assert current.get("VMState") == '"poweroff"'
    assert Path(current["CfgFile"].strip('"')).is_relative_to(BASE)
    built = json.loads((OUT / "build-evidence.json").read_text())
    assert built.get("short_periods", False) == short_periods
    assert built.get("production_long", False) == production_long
    assert hashlib.sha256((ROOT / built["loader"]).read_bytes()).hexdigest() == built["loader_sha256"]
    assert hashlib.sha256((ROOT / "kernel/drivers/hda.rs").read_bytes()).hexdigest() == built["hda_sha256"]
    assert hashlib.sha256((OUT / "probe.raw").read_bytes()).hexdigest() == built["disk_sha256"]
    attempt = OUT / str(time.time_ns())
    attempt.mkdir()
    wavs = attempt / "synthetic-output"
    wavs.mkdir()
    serial = attempt / "serial.bin"
    disk = attempt / "probe.vdi"
    config = ET.parse(current["CfgFile"].strip('"'))
    old_keys = {node.attrib["name"]: node.attrib["value"] for node in config.iter()
                if node.tag.rsplit("}", 1)[-1] == "ExtraDataItem"}
    command(["VBoxManage", "convertfromraw", OUT / "probe.raw", disk, "--format", "VDI"])
    complete = False
    try:
        command(["VBoxManage", "modifyvm", name, "--cpus", 8, "--memory", 2048, "--nic1", "none",
                 "--audio-driver", "default", "--audio-controller", "hda", "--audio-enabled", "on",
                 "--audio-in", "off", "--audio-out", "on", "--uart1", "0x3f8", 4, "--uart-mode1", "file", serial])
        # Both official VirtualBox device and connector layers expose these typed
        # configuration keys. The input backend is OFF before either is enabled.
        for prefix in ("VBoxInternal/Devices/hda/0/Config/", "VBoxInternal/Devices/hda/0/LUN#0/Config/"):
            command(["VBoxManage", "setextradata", name, prefix + "DebugEnabled", "1"])
            command(["VBoxManage", "setextradata", name, prefix + "DebugPathOut", wavs])
        command(["VBoxManage", "storageattach", name, "--storagectl", "VirtioSCSI", "--port", 0,
                 "--device", 0, "--type", "hdd", "--medium", disk])
        state = fields(name)
        assert state.get("audio_in") == '"off"', "Host microphone must remain disabled for synthetic recording"
        command(["VBoxManage", "startvm", name, "--type", "headless"])
        deadline = time.monotonic() + (110 if production_long else 70)
        footer = (struct.pack("<Q", MAGIC).hex() + "\n" + bytes(8).hex() + "\n").encode()
        while time.monotonic() < deadline:
            data = serial.read_bytes() if serial.exists() else b""
            if footer in data:
                complete = True
                break
            time.sleep(0.2)
        for args, file in ((["statistics", "--pattern=/Devices/hda*"], "hda-statistics.txt"),
                           (["info", "hda"], "hda-registers.txt"),
                           (["info", "hdamixer"], "hda-mixer.txt")):
            result = subprocess.run(["VBoxManage", "debugvm", name] + args, capture_output=True)
            (attempt / file).write_bytes(result.stdout + result.stderr)
    finally:
        original_error = sys.exc_info()[0] is not None
        cleanup_errors = []
        try:
            if fields(name).get("VMState") == '"running"':
                command(["VBoxManage", "controlvm", name, "poweroff"])
        except Exception as error:
            cleanup_errors.append(str(error))
        for prefix in ("VBoxInternal/Devices/hda/0/Config/", "VBoxInternal/Devices/hda/0/LUN#0/Config/"):
            for key in ("DebugEnabled", "DebugPathOut"):
                previous = old_keys.get(prefix + key)
                try:
                    command(["VBoxManage", "setextradata", name, prefix + key] + ([] if previous is None else [previous]))
                except Exception as error:
                    cleanup_errors.append(str(error))
        if cleanup_errors:
            message = "Disposable-VM cleanup errors: " + "; ".join(cleanup_errors)
            if original_error:
                print(message, file=sys.stderr, flush=True)
            else:
                raise RuntimeError(message)
    assert complete, "Native playback watchdog expired; diagnostic evidence retained at " + str(attempt)
    analyze(attempt, short_periods, production_long)


# ------------------------=
# FUNC: main
# DESC: Requires serialized build-kit authority and separates build from explicitly requested VM execution.
# ------------------=
def main():
    global OUT
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    parser = argparse.ArgumentParser()
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--vm")
    parser.add_argument("--short-periods", action="store_true")
    parser.add_argument("--analyze", type=Path)
    parser.add_argument("--production-long", action="store_true")
    args = parser.parse_args()
    assert sum((args.build_only, args.run, args.analyze is not None)) == 1
    assert not (args.short_periods and args.production_long)
    if args.short_periods:
        OUT = BASE / "playback-short"
    elif args.production_long:
        OUT = BASE / "playback-production-long"
    if args.analyze:
        analyze(args.analyze, args.short_periods, args.production_long)
    elif args.build_only:
        build(args.short_periods, args.production_long)
    else:
        run(args.vm, args.short_periods, args.production_long)


if __name__ == "__main__":
    main()
