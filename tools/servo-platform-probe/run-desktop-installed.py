"""Exercise the integrated browser in a disposable installed ARM desktop.

The installation uses existing media and receives the experimental kernel offline.
This is not fresh browser-ISO parity evidence.
"""
import importlib.util
import argparse
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("installed", ROOT / "tools/ms9-installed-acceptance.py")
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
net_spec = importlib.util.spec_from_file_location("geturl_installed", ROOT / "tools/geturl-installed-test.py")
network = importlib.util.module_from_spec(net_spec)
net_spec.loader.exec_module(network)

# ------------------------=
# FUNC: browser_symbols
# DESC: Locates read-only engine counters in ELF metadata; acceptance uses their guest values, not symbol strings.
# ------------------=
def browser_symbols(elf):
    output = subprocess.check_output(["/opt/homebrew/opt/llvm/bin/llvm-nm", "-S",
        "--defined-only", "--demangle", str(elf)], text=True)
    result = {}
    for line in output.splitlines():
        fields = line.split(maxsplit=3)
        if len(fields) != 4:
            continue
        for name in ("STATE", "FAILURE", "FRAME_REVISION", "PEAK"):
            if fields[3] in ("infinity_kernel::runtime::browser::" + name,
                "infinity_kernel::runtime::browser::" + name + " (.0)"):
                result[name] = (int(fields[0], 16), int(fields[1], 16))
    assert len(result) == 4
    return result

class Guest(base.Guest):
    # ------------------------=
    # FUNC: click
    # DESC: Requires observed pointer motion and paired button transitions before accepting a desktop click.
    # ------------------=
    def click(self, x, y):
        target = (x * 1000 // self.width, y * 1000 // self.height)
        for _ in range(80):
            state = self.state()
            assert state is not None
            deltas = (target[0] - state[13], target[1] - state[14])
            if max(map(abs, deltas)) <= 4:
                break
            events = [{"type": "rel", "data": {"axis": axis,
                "value": max(-40, min(40, int(delta / 3) or (1 if delta > 0 else -1)))}}
                for axis, delta in zip(("x", "y"), deltas) if abs(delta) > 4]
            self.qmp("input-send-event", {"events": events})
            self.wait(lambda s: s[13:15] != state[13:15], "browser pointer moved", timeout=30)
        else:
            raise AssertionError("Pointer did not reach browser control")
        for down in (True, False):
            self.qmp("input-send-event", {"events": [
                {"type": "btn", "data": {"button": "left", "down": down}}]})
            self.wait(lambda s: bool(s[15] & 1) == down, "browser pointer button", timeout=30)

    # ------------------------=
    # FUNC: boot
    # DESC: Boots actual UEFI media and patches only this disposable installation before its first detached boot.
    # ------------------=
    def boot(self, installer):
        assert self.process is None
        artifacts = self.work.parent / "artifacts"
        if not installer and not getattr(self, "patched", False):
            subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"),
                str(self.disk), str(artifacts / "installed-stripped.elf"), "--apply"], check=True)
            self.patched = True
        self.installer = installer
        elf = artifacts / ("kernel.elf" if installer else "installed-kernel.elf")
        self.address, self.length = base.symbol(elf, "INFINITY_DIAGNOSTIC_SNAPSHOT")
        self.frames_address, self.frames_length = base.symbol(elf, "INFINITY_DIAGNOSTIC_FRAMES")
        self.compute_address = None
        qmp = self.work / "qmp.sock"
        qmp.unlink(missing_ok=True)
        self.log = (self.work / ("installer.log" if installer else "installed.log")).open("ab")
        command = ["qemu-system-aarch64", "-machine", "virt", "-accel", "tcg", "-cpu", "max",
            "-smp", "4", "-m", "12G", "-bios", self.firmware, "-device", "ramfb",
            "-device", "qemu-xhci", "-device", "usb-kbd", "-device", "usb-mouse",
            "-device", "virtio-scsi-pci", "-drive", f"if=none,id=disk,format=raw,file={self.disk}",
            "-device", "scsi-hd,drive=disk,bootindex=1", "-netdev", "user,id=net",
            "-device", "e1000,netdev=net", "-qmp", f"unix:{qmp},server=on,wait=off",
            "-display", "none", "-serial", "stdio", "-monitor", "none", "-no-reboot"]
        if installer:
            command += ["-drive", f"if=none,id=cd,format=raw,media=cdrom,file={artifacts / 'installer.iso'}",
                "-device", "scsi-cd,drive=cd,bootindex=0"]
        self.process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=self.log, stderr=self.log)
        deadline = time.monotonic() + 20
        while not qmp.exists():
            if self.process.poll() is not None or time.monotonic() >= deadline:
                if self.process.poll() is None:
                    self.process.kill()
                self.process.wait()
                self.process = None
                self.log.close()
                raise RuntimeError("QEMU management startup failed; inspect guest log")
            time.sleep(.1)
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(20)
        self.sock.connect(str(qmp))
        self.channel = self.sock.makefile("rwb")
        json.loads(self.channel.readline())
        self.qmp("qmp_capabilities")

# ------------------------=
# FUNC: main
# DESC: Installs on a new private disk and captures native browser launch without claiming release packaging acceptance.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reuse-installed", type=Path)
    parser.add_argument("--update-kernel", type=Path, help="Update only this harness's disposable disk from a repository-local kernel")
    parser.add_argument("--navigation", action="store_true", help="Capture real link/history interaction for manual review; not an automatic navigation pass")
    args = parser.parse_args()
    reuse = args.reuse_installed is not None
    if args.update_kernel and (not reuse or not args.update_kernel.resolve().is_relative_to(ROOT / "build")):
        parser.error("Kernel updates require a reused disposable installation and a build-local image")
    work = args.reuse_installed.resolve() if reuse else ROOT / "build" / ("browser-installed-" + str(time.time_ns()))
    assert work.parent == ROOT / "build" and work.name.startswith("browser-installed-")
    artifacts = work / "artifacts"
    if not reuse:
        artifacts.mkdir(parents=True)
    for source, name in ([] if reuse else [
        ("builds/InfinityOS-aarch64-qemu-test.iso", "installer.iso"),
        ("build/aarch64/kernel-qemu.elf", "kernel.elf"),
        ("build/servo-platform-probe/kernel-aarch64/qemu-kernel.elf", "installed-kernel.elf")]):
        shutil.copyfile(ROOT / source, artifacts / name)
    if args.update_kernel:
        shutil.copyfile(args.update_kernel.resolve(), artifacts / "installed-kernel.elf")
    subprocess.run(["/opt/homebrew/opt/llvm/bin/llvm-objcopy", "--strip-debug",
        str(artifacts / "installed-kernel.elf"), str(artifacts / "installed-stripped.elf")], check=True)
    guest = Guest(work, 1, "/opt/homebrew/share/qemu/edk2-aarch64-code.fd", reuse=reuse, width=1024, height=768, memory_mb=12288)
    guest.patched = reuse and args.update_kernel is None
    receipt = dict(installed=reuse, browser_iso_parity=False, browser_interactive=False)
    try:
        if not reuse:
            guest.install()
            receipt["installed"] = True
            guest.stop()
        guest.onboard()
        if not reuse:
            network.configure_nat(guest)
        guest.launch("command", 5)
        counters = browser_symbols(artifacts / "installed-kernel.elf")
        guest.command("https authorize confirm=true")
        guest.command("browser https://example.com/")
        started = time.monotonic()
        deadline = started + 90
        while time.monotonic() < deadline:
            values = {name: int.from_bytes(guest.memory(address, size), "little")
                for name, (address, size) in counters.items()}
            receipt["engine"] = values
            if values["STATE"] == 3 or values["FAILURE"]:
                break
            if values["STATE"] == 2 and values["FRAME_REVISION"] >= 2 and time.monotonic() - started >= 15:
                break
            time.sleep(.25)
        guest.screenshot("browser-launch")
        receipt["observation_seconds"] = time.monotonic() - started
        assert values["STATE"] == 2 and values["FAILURE"] == 0 and values["FRAME_REVISION"] >= 2, receipt
        receipt["engine_running_with_frames"] = True
        receipt["launch_command_submitted"] = True
        if args.navigation:
            for label, x, y in (("link", 303, 368), ("back", 139, 161),
                               ("forward", 192, 161), ("reload", 242, 161)):
                guest.click(x, y)
                time.sleep(8)
                guest.screenshot("browser-" + label)
            receipt["navigation_captures_for_review"] = True
        print(json.dumps(dict(work=str(work), **receipt)), flush=True)
    finally:
        (work / "result.json").write_text(json.dumps(receipt, indent=2) + "\n")
        guest.stop()

if __name__ == "__main__":
    main()
