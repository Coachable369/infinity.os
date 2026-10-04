#!/usr/bin/env python3
"""Bounded native VirtualBox timer/capture diagnosis, never installed acceptance."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import shutil
import struct
import subprocess
import time
import uuid
import zlib

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "build/arm-capture-timer-probe"
MAGIC = 0x494E46434150544D


# ------------------------=
# FUNC: command
# DESC: Executes an explicit bounded command inside this serialized build-kit workflow.
# ------------------=
def command(args, **kwargs):
    print("+ " + " ".join(map(str, args)), flush=True)
    return subprocess.run(list(map(str, args)), cwd=ROOT, check=True, **kwargs)


# ------------------------=
# FUNC: disk_image
# DESC: Builds an isolated GPT/FAT EFI test disk without producing or changing any ISO.
# ------------------=
def disk_image():
    path = OUT / "probe.raw"
    size = 128 * 1024 * 1024
    sectors = size // 512
    entries = bytearray(128 * 128)
    struct.pack_into("<16s16sQQQ", entries, 0,
                     uuid.UUID("c12a7328-f81f-11d2-ba4b-00a0c93ec93b").bytes_le,
                     uuid.uuid4().bytes_le, 2048, sectors - 34, 0)
    entries[56:56 + 20] = "TimerProbe".encode("utf-16le")
    disk_id = uuid.uuid4().bytes_le
    with path.open("wb") as stream:
        stream.truncate(size)
        mbr = bytearray(512)
        struct.pack_into("<B3sB3sII", mbr, 446, 0, b"\0\2\0", 0xEE,
                         b"\xff\xff\xff", 1, sectors - 1)
        mbr[510:] = b"\x55\xaa"
        stream.write(mbr)
        for current, backup, table in ((1, sectors - 1, 2), (sectors - 1, 1, sectors - 33)):
            header = bytearray(512)
            struct.pack_into("<8sIIIIQQQQ16sQIII", header, 0, b"EFI PART", 0x10000,
                             92, 0, 0, current, backup, 34, sectors - 34, disk_id,
                             table, 128, 128, zlib.crc32(entries))
            struct.pack_into("<I", header, 16, zlib.crc32(header[:92]))
            stream.seek(current * 512)
            stream.write(header)
            stream.seek(table * 512)
            stream.write(entries)
    image = str(path) + "@@1048576"
    command(["mformat", "-i", image, "-F", "-T", sectors - 34 - 2048 + 1, "::"])
    command(["mmd", "-i", image, "::/EFI", "::/EFI/BOOT", "::/EFI/INFINITY"])
    command(["mcopy", "-i", image, OUT / "baseline-BOOTAA64.EFI", "::/EFI/BOOT/BOOTAA64.EFI"])
    command(["mcopy", "-i", image, OUT / "probe.elf", "::/EFI/INFINITY/KERNEL.ELF"])


# ------------------------=
# FUNC: build
# DESC: Preserves the existing production loader and links the small native capture probe.
# ------------------=
def build():
    OUT.mkdir(parents=True, exist_ok=True)
    loader = OUT / "baseline-BOOTAA64.EFI"
    if not loader.exists():
        shutil.copyfile(ROOT / "build/aarch64/BOOTAA64.EFI", loader)
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(OUT / "target"))
    command(["cargo", "build", "--manifest-path", "tools/arm-capture-timer-probe/Cargo.toml",
             "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat"], env=env)
    command(["/opt/homebrew/opt/lld/bin/ld.lld", "-nostdlib", "--gc-sections", "-T",
             "linker/aarch64.ld", "-o", OUT / "probe.elf",
             OUT / "target/aarch64-unknown-none-softfloat/release/libinfinity_capture_timer_probe.a"])
    disk_image()
    (OUT / "build-evidence.json").write_text(json.dumps(dict(
        baseline_loader_sha256=hashlib.sha256(loader.read_bytes()).hexdigest(),
        probe_sha256=hashlib.sha256((OUT / "probe.elf").read_bytes()).hexdigest(),
        baseline_loader_source="Production BOOTAA64.EFI captured at the first probe build",
        iso_generated=False), indent=2) + "\n")


# ------------------------=
# FUNC: decode
# DESC: Decodes binary register/capture measurements without using diagnostic prose as an oracle.
# ------------------=
def decode(data):
    marker = struct.pack("<Q", MAGIC).hex().encode() + b"\n"
    start = data.find(marker)
    assert start >= 0, "Guest emitted no binary result"
    data = bytes.fromhex(data[start:].decode())
    status, = struct.unpack_from("<Q", data, 8)
    assert status == 2, dict(guest_status=status, bytes=len(data))
    words = struct.unpack("<251Q", data[:251 * 8])
    assert words[:3] == (MAGIC, 2, 7), words[:5]
    assert words[-2:] == (MAGIC, 0), words[-2:]
    register_names = ["mpidr", "current_el", "daif", "cntp_ctl", "cntp_cval",
                      "cntv_ctl", "cntv_cval", "cntpct", "cntvct", "cntfrq",
                      "isr", "highest_pending_irq", "group1_enabled", "priority_mask"]
    registers = [dict(zip(register_names, words[i:i + 14])) for i in range(5, 117, 14)]
    states = [dict(zip(register_names, words[i:i + 14])) for i in range(117, 249, 22)]
    phases = []
    names = ["inherited", "disabled", "expired_masked", "disabled_recovery", "busy_yield", "wfe_recovery"]
    for i in range(6):
        row = words[131 + i * 22:139 + i * 22]
        assert row[0] == i and row[2] > row[1] and row[5] > 0, row
        phases.append(dict(phase=names[i], seconds=(row[2] - row[1]) / words[4],
                           capture_frames=row[3], max_no_samples_seconds=row[4] / words[4],
                           reads=row[5], initial_lpib=row[6], final_lpib=row[7]))
    assert sorted(r["mpidr"] & 255 for r in registers) == list(range(8)), registers
    assert states[2]["cntv_ctl"] & 5 == 5 and states[2]["daif"] & 128, states[2]
    assert states[1]["cntv_ctl"] & 1 == states[3]["cntv_ctl"] & 1 == 0
    evidence = dict(environment="disposable native VirtualBox ARM guest", installed_verified=False,
                    sample_rate=words[3], actual_inherited_registers=registers,
                    target_phase_registers=states, phases=phases,
                    mechanism_reproduced=phases[2]["max_no_samples_seconds"] > 2 and
                    phases[3]["max_no_samples_seconds"] < 0.5)
    (OUT / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2), flush=True)


# ------------------------=
# FUNC: run
# DESC: Runs only the explicitly named repository-local disposable VM without touching installed systems.
# ------------------=
def run(name):
    assert name == "infinity-audio-timer-test", "Only the approved isolated VM is allowed"
    assert (OUT / "probe.raw").is_file(), "Run --build-only first"
    existing = subprocess.run(["VBoxManage", "showvminfo", name, "--machinereadable"],
                              capture_output=True, text=True)
    if existing.returncode == 0:
        fields = dict(line.split("=", 1) for line in existing.stdout.splitlines() if "=" in line)
        assert fields.get("VMState") == '"poweroff"', "Existing test VM must be powered off"
        assert Path(fields["CfgFile"].strip('"')).is_relative_to(OUT), "VM is not owned by this probe"
    else:
        command(["VBoxManage", "createvm", "--name", name, "--platform-architecture", "arm",
                 "--ostype", "Other_arm64", "--basefolder", OUT / "vms", "--register"])
        command(["VBoxManage", "storagectl", name, "--name", "VirtioSCSI", "--add", "virtio-scsi", "--portcount", 1])
    serial = OUT / ("serial-" + str(time.time_ns()) + ".bin")
    disk = OUT / ("probe-" + str(time.time_ns()) + ".vdi")
    command(["VBoxManage", "convertfromraw", OUT / "probe.raw", disk, "--format", "VDI"])
    command(["VBoxManage", "modifyvm", name, "--cpus", 8, "--memory", 2048, "--cpu-profile", "host",
             "--firmware", "efi", "--chipset", "armv8virtual", "--graphicscontroller", "vmsvga", "--vram", 128,
             "--boot1", "disk", "--boot2", "none", "--nic1", "none", "--usb", "off", "--usb-xhci", "on",
             "--mouse", "usb", "--keyboard", "usb", "--audio-driver", "default", "--audio-controller", "hda",
             "--audio-enabled", "on", "--audio-in", "on", "--audio-out", "on",
             "--uart1", "0x3f8", 4, "--uart-mode1", "file", serial])
    command(["VBoxManage", "storageattach", name, "--storagectl", "VirtioSCSI", "--port", 0,
             "--device", 0, "--type", "hdd", "--medium", disk])
    command(["VBoxManage", "startvm", name, "--type", "headless"])
    deadline = time.monotonic() + 100
    footer = (struct.pack("<Q", MAGIC).hex() + "\n" + bytes(8).hex() + "\n").encode()
    while time.monotonic() < deadline:
        data = serial.read_bytes() if serial.exists() else b""
        if footer in data:
            state = subprocess.run(["VBoxManage", "showvminfo", name, "--machinereadable"],
                                   capture_output=True, text=True, check=True)
            fields = dict(line.split("=", 1) for line in state.stdout.splitlines() if "=" in line)
            if fields.get("VMState") == '"running"':
                command(["VBoxManage", "controlvm", name, "poweroff"])
            decode(data)
            return
        time.sleep(0.2)
    command(["VBoxManage", "controlvm", name, "poweroff"])
    decode(serial.read_bytes() if serial.exists() else b"")
    raise RuntimeError("Guest did not finish within the bounded watchdog")


# ------------------------=
# FUNC: main
# DESC: Separates build preparation from explicitly authorized isolated VM execution.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1", "Use ./build-kit run python3 " + __file__
    parser = argparse.ArgumentParser()
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--vm")
    args = parser.parse_args()
    assert args.build_only != args.run
    if args.build_only:
        build()
    else:
        run(args.vm)


if __name__ == "__main__":
    main()
