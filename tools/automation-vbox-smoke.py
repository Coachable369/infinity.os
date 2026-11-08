#!/usr/bin/env python3
"""Bounded canonical ARM boot/install smoke on a new private VirtualBox VM.

This probes VirtualBox core snapshots before using them to drive the installer.
It never modifies existing VMs, replaces a guest kernel, or configures audio.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("desktop_media", ROOT / "tools/servo-platform-probe/run-desktop-installed.py")
desktop = importlib.util.module_from_spec(spec)
spec.loader.exec_module(desktop)


# ------------------------=
# FUNC: command
# DESC: Runs one bounded management operation and retains diagnostics without parsing guest prose as evidence.
# ------------------=
def command(*arguments, timeout=45):
    return subprocess.run([str(item) for item in arguments], cwd=ROOT, check=True,
                          capture_output=True, text=True, timeout=timeout)


# ------------------------=
# FUNC: core_bytes
# DESC: Extracts a bounded physical range from an ELF core without loading or retaining the full guest memory image.
# ------------------=
def core_bytes(path, address, size):
    with path.open("rb") as stream:
        header = stream.read(64)
        assert header[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", header, 16)[0] == 4
        offset = struct.unpack_from("<Q", header, 32)[0]
        stride, count = struct.unpack_from("<HH", header, 54)
        assert stride >= 56 and count <= 4096
        for index in range(count):
            stream.seek(offset + index * stride)
            kind, _, file_offset, _, physical, file_size, _, _ = struct.unpack("<IIQQQQQQ", stream.read(56))
            if kind == 1 and physical <= address and address + size <= physical + file_size:
                stream.seek(file_offset + address - physical)
                result = stream.read(size)
                assert len(result) == size
                return result
    raise AssertionError({"core_missing_physical_range": [address, size]})


class Smoke:
    # ------------------------=
    # FUNC: __init__
    # DESC: Assigns a unique private VM identity and repository-owned artifact paths before any creation.
    # ------------------=
    def __init__(self, args):
        self.args = args
        suffix = str(time.time_ns()) + "-" + uuid.uuid4().hex[:8]
        self.work = ROOT / "build" / ("automation-vbox-" + suffix)
        self.name = "infinity-automation-" + suffix
        self.owned = False
        self.installer = True
        self.dumps = 0
        self.snapshots = {}
        self.proof = dict(vm=self.name, work=str(self.work), canonical_boot=False,
                          fresh_install=False, detached_installed_boot=False,
                          feature_acceptance=False, core_snapshot_viable=False,
                          screenshots=[], cores=[], states=[])

    # ------------------------=
    # FUNC: vbox
    # DESC: Targets only this run's uniquely created VirtualBox machine.
    # ------------------=
    def vbox(self, operation, *arguments, timeout=45):
        assert self.owned
        return command("VBoxManage", operation, self.name, *arguments, timeout=timeout)

    # ------------------------=
    # FUNC: prepare
    # DESC: Copies the exact canonical ISO and creates a blank 32 GiB ARM VM with the established native hardware layout.
    # ------------------=
    def prepare(self):
        iso = self.args.iso.resolve()
        assert iso == ROOT / "builds/InfinityOS-aarch64.iso" and iso.is_file()
        assert shutil.disk_usage(ROOT).free >= 60 * 1024**3
        artifacts = self.work / "artifacts"
        artifacts.mkdir(parents=True)
        desktop.copy_artifact(iso, artifacts / "installer.iso")
        with (artifacts / "installer.iso").open("rb") as source:
            self.proof["iso_sha256"] = hashlib.file_digest(source, "sha256").hexdigest()
        self.proof["source_iso"] = str(iso)
        self.proof["media_kernels"] = desktop.extract_media_kernels(artifacts / "installer.iso", artifacts, "aarch64")
        for name in ("kernel.elf", "installed-kernel.elf"):
            self.snapshots[name] = desktop.base.symbol(artifacts / name, "INFINITY_DIAGNOSTIC_SNAPSHOT")
            assert self.snapshots[name][1] == 4096
        command("VBoxManage", "createvm", "--name", self.name, "--platform-architecture", "arm",
                "--ostype", "Other_arm64", "--basefolder", self.work, "--register")
        self.owned = True
        self.vbox("modifyvm", "--memory", "12288", "--cpus", "4", "--cpu-profile", "host",
                  "--firmware", "efi", "--chipset", "armv8virtual", "--tpm-type", "2.0",
                  "--graphicscontroller", "vmsvga", "--vram", "128", "--boot1", "dvd",
                  "--boot2", "disk", "--boot3", "none", "--boot4", "none", "--nic1", "none",
                  "--usb", "off", "--usb-xhci", "on", "--mouse", "usb", "--keyboard", "usb",
                  "--rtc-use-utc", "on", "--uart1", "0x3f8", "4", "--uart-mode1", "file", self.work / "serial.log")
        self.vbox("setextradata", "VBoxInternal2/EfiGraphicsResolution", "1024x768")
        self.vbox("storagectl", "--name", "VirtioSCSI", "--add", "virtio-scsi", "--portcount", "2", "--bootable", "on")
        command("VBoxManage", "createmedium", "disk", "--filename", self.work / "installed.vdi",
                "--size", "32768", "--format", "VDI")
        self.vbox("storageattach", "--storagectl", "VirtioSCSI", "--port", "0", "--device", "0",
                  "--type", "hdd", "--medium", self.work / "installed.vdi")
        self.vbox("storageattach", "--storagectl", "VirtioSCSI", "--port", "1", "--device", "0",
                  "--type", "dvddrive", "--medium", artifacts / "installer.iso")

    # ------------------------=
    # FUNC: screenshot
    # DESC: Captures the actual private guest framebuffer for manual visual review.
    # ------------------=
    def screenshot(self, label):
        path = self.work / (label + ".png")
        self.vbox("controlvm", "screenshotpng", path)
        self.proof["screenshots"].append(str(path))

    # ------------------------=
    # FUNC: key
    # DESC: Sends one real keyboard make/break pair to the private guest.
    # ------------------=
    def key(self, code):
        self.vbox("controlvm", "keyboardputscancode", f"{code:02x}", f"{code | 0x80:02x}")
        time.sleep(.3)

    # ------------------------=
    # FUNC: state
    # DESC: Probes a bounded full-core operation, extracts only typed public diagnostics, and immediately removes the RAM dump.
    # ------------------=
    def state(self):
        assert self.dumps < 64, "Core snapshot budget exhausted"
        self.dumps += 1
        core = self.work / "snapshot.core"
        core.unlink(missing_ok=True)
        started = time.monotonic()
        sample = dict(number=self.dumps)
        self.proof["cores"].append(sample)
        try:
            self.vbox("debugvm", "dumpvmcore", "--filename=" + str(core), timeout=self.args.core_timeout)
            sample["bytes"] = core.stat().st_size
            address, size = self.snapshots["kernel.elf" if self.installer else "installed-kernel.elf"]
            values = struct.unpack("<512Q", core_bytes(core, address, size))
            sample["valid"] = values[:2] == (0x494e464449414731, 1) and values[2] == values[511] and not values[2] & 1
            if sample["valid"]:
                self.proof["core_snapshot_viable"] = True
                self.proof["states"].append(list(values[:16]))
                return values
            return None
        finally:
            sample["seconds"] = round(time.monotonic() - started, 3)
            core.unlink(missing_ok=True)

    # ------------------------=
    # FUNC: wait
    # DESC: Waits for typed UI transitions with finite wall-clock and total core-dump budgets.
    # ------------------=
    def wait(self, predicate, label, timeout=180):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            state = self.state()
            if state is not None and predicate(state):
                print(json.dumps(dict(stage=label, snapshot=state[2])), flush=True)
                return state
            time.sleep(2)
        raise AssertionError({"timeout": label})

    # ------------------------=
    # FUNC: run
    # DESC: Verifies canonical boot and keyboard navigation, then optionally installs only onto this run's blank disk.
    # ------------------=
    def run(self):
        self.vbox("startvm", "--type", "headless")
        time.sleep(12)
        self.screenshot("live-boot")
        self.wait(lambda value: value[3] == 0 and value[4] == 0, "canonical live boot")
        self.proof["canonical_boot"] = True
        self.key(0x02)
        self.key(0x1c)
        self.wait(lambda value: value[4] == 3 and value[5] == 0, "installer keyboard navigation")
        self.screenshot("installer-welcome")
        if not self.args.install:
            return
        for step in range(7):
            state = self.wait(lambda value: value[5] == step, "installer step " + str(step))
            if step == 6:
                assert state[6] == 0, "Destructive confirmation must default to Cancel"
                self.screenshot("private-disk-confirmation")
            for _ in range(5):
                if state[6] == 1:
                    break
                previous = state[6]
                self.key(0x0f)
                state = self.wait(lambda value: value[6] != previous, "installer focus")
            assert state[6] == 1
            self.key(0x1c)
            state = self.wait(lambda value: value[5] > step, "installer advance " + str(step),
                              timeout=900 if step == 6 else 180)
            assert state[5] != 9, {"installer_failure": list(state[:16])}
        self.wait(lambda value: value[5] == 8, "fresh installation complete", timeout=900)
        self.proof["fresh_install"] = True
        self.screenshot("installation-complete")
        self.vbox("controlvm", "poweroff")
        self.vbox("storageattach", "--storagectl", "VirtioSCSI", "--port", "1", "--device", "0",
                  "--type", "dvddrive", "--medium", "none")
        self.vbox("modifyvm", "--boot1", "disk", "--boot2", "none")
        self.installer = False
        self.vbox("startvm", "--type", "headless")
        time.sleep(12)
        self.wait(lambda value: value[3] == 1 and value[4] == 4, "detached installed onboarding")
        self.proof["detached_installed_boot"] = True
        self.screenshot("detached-installed-onboarding")

    # ------------------------=
    # FUNC: cleanup
    # DESC: Stops and unregisters only the newly created machine while retaining its disk, configuration and evidence.
    # ------------------=
    def cleanup(self):
        if not self.owned:
            return
        try:
            self.vbox("controlvm", "poweroff")
        except Exception as error:
            self.proof["poweroff_diagnostic"] = str(error)
        try:
            self.vbox("unregistervm")
            self.proof["unregistered"] = True
        except Exception as error:
            self.proof["cleanup_failure"] = str(error)


# ------------------------=
# FUNC: main
# DESC: Runs only under build-kit and always preserves a bounded smoke receipt without claiming feature acceptance.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/automation-vbox-smoke.py [--install]")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iso", type=Path, default=ROOT / "builds/InfinityOS-aarch64.iso")
    parser.add_argument("--install", action="store_true", help="Install onto this run's blank disk and boot with the ISO detached")
    parser.add_argument("--core-timeout", type=int, default=45, choices=range(10, 61), metavar="10..60")
    args = parser.parse_args()
    smoke = Smoke(args)
    try:
        smoke.prepare()
        smoke.run()
    except BaseException as error:
        smoke.proof["failure"] = str(error)
        if isinstance(error, subprocess.CalledProcessError):
            smoke.proof["command_stderr"] = error.stderr
        if smoke.owned:
            try:
                smoke.screenshot("failure")
            except Exception:
                pass
        raise
    finally:
        smoke.cleanup()
        if smoke.work.is_dir():
            path = smoke.work / "smoke.json"
            path.write_text(json.dumps(smoke.proof, indent=2) + "\n")
            print(str(path), flush=True)


if __name__ == "__main__":
    main()
