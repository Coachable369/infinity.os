"""Installed-OS acceptance through real input and read-only binary debugger state.

Never parses guest terminal text. Only fresh, harness-owned disks are modified.
"""
import argparse
import json
import pathlib
import socket
import struct
import subprocess
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: symbol
# DESC: Resolves debugger addresses from ELF metadata, not from guest prose or source assertions.
# ------------------=
def symbol(elf, name):
    data = elf.read_bytes()
    assert data[:6] == b"\x7fELF\x02\x01"
    offset = struct.unpack_from("<Q", data, 40)[0]
    size, count = struct.unpack_from("<HH", data, 58)
    sections = [struct.unpack_from("<IIQQQQIIQQ", data, offset + i * size) for i in range(count)]
    for section in sections:
        if section[1] != 2:
            continue
        strings = sections[section[6]]
        table = data[strings[4]:strings[4] + strings[5]]
        for at in range(section[4], section[4] + section[5], section[9]):
            label, _, _, _, address, length = struct.unpack_from("<IBBHQQ", data, at)
            if table[label:table.index(0, label)] == name.encode():
                return address, length
    raise AssertionError({"missing_debug_metadata": name, "elf": str(elf)})

class Guest:
    # ------------------------=
    # FUNC: __init__
    # DESC: Creates one isolated machine with a unique blank disk and independently booted installed generation.
    # ------------------=
    def __init__(self, work, number, firmware):
        self.work = work / f"node-{number}"
        self.work.mkdir()
        self.number = number
        self.firmware = firmware
        self.disk = self.work / "installed.raw"
        with self.disk.open("xb") as stream:
            stream.truncate(32 * 1024**3)
        self.process = None
        self.channel = None
        self.capture = 0

    # ------------------------=
    # FUNC: boot
    # DESC: Boots either original installer media or disk alone; cold boot always creates a new emulator process.
    # ------------------=
    def boot(self, installer):
        assert self.process is None
        self.installer = installer
        elf = ROOT / "build/x86_64" / ("kernel.elf" if installer else "installed-kernel.elf")
        self.address, self.length = symbol(elf, "INFINITY_DIAGNOSTIC_SNAPSHOT")
        self.frames_address, self.frames_length = symbol(elf, "INFINITY_DIAGNOSTIC_FRAMES")
        qmp = self.work / "qmp.sock"
        qmp.unlink(missing_ok=True)
        self.log = (self.work / ("installer.log" if installer else "installed.log")).open("ab")
        command = ["qemu-system-x86_64", "-machine", "pc", "-cpu", "max", "-m", "4096M",
                   "-drive", f"if=pflash,format=raw,readonly=on,file={self.firmware}",
                   "-drive", f"if=ide,index=0,format=raw,file={self.disk}",
                   "-netdev", "user,id=net", "-device", f"e1000,netdev=net,mac=02:00:00:00:09:{self.number:02x}",
                   "-object", "rng-random,id=rng0,filename=/dev/urandom", "-device", "virtio-rng-pci,rng=rng0",
                   "-qmp", f"unix:{qmp},server=on,wait=off", "-display", "none", "-serial", "stdio", "-no-reboot"]
        if installer:
            command += ["-cdrom", str(ROOT / "builds/InfinityOS-x86_64.iso"), "-boot", "order=d"]
        else:
            command += ["-boot", "order=c"]
        self.process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=self.log, stderr=self.log)
        deadline = time.monotonic() + 15
        while not qmp.exists():
            assert self.process.poll() is None and time.monotonic() < deadline
            time.sleep(.1)
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(20)
        self.sock.connect(str(qmp))
        self.channel = self.sock.makefile("rwb")
        json.loads(self.channel.readline())
        self.qmp("qmp_capabilities")

    # ------------------------=
    # FUNC: qmp
    # DESC: Uses structured QEMU management results for input and read-only memory capture.
    # ------------------=
    def qmp(self, command, arguments=None):
        self.channel.write(json.dumps({"execute": command, "arguments": arguments or {}}).encode() + b"\n")
        self.channel.flush()
        while True:
            response = json.loads(self.channel.readline())
            if "error" in response:
                raise AssertionError(response)
            if "return" in response:
                return response["return"]

    # ------------------------=
    # FUNC: memory
    # DESC: Reads a fixed exported diagnostic region without writing guest memory or granting guest authority.
    # ------------------=
    def memory(self, address, size):
        target = self.work / "capture.bin"
        self.qmp("pmemsave", {"val": address, "size": size, "filename": str(target)})
        result = target.read_bytes()
        assert len(result) == size
        return result

    # ------------------------=
    # FUNC: state
    # DESC: Rejects incomplete snapshots and decodes actual UI and service values.
    # ------------------=
    def state(self):
        values = struct.unpack("<512Q", self.memory(self.address, self.length))
        if values[0] != 0x494e464449414731 or values[1] != 1 or values[2] & 1 or values[2] != values[511]:
            return None
        return values

    # ------------------------=
    # FUNC: wait
    # DESC: Waits on a bounded behavioral predicate and records the exact final binary state on failure.
    # ------------------=
    def wait(self, predicate, label, timeout=120):
        deadline = time.monotonic() + timeout
        last = None
        while time.monotonic() < deadline:
            assert self.process.poll() is None, {"stage": label, "exit": self.process.returncode}
            last = self.state()
            if last is not None and predicate(last):
                return last
            time.sleep(.25)
        self.screenshot("failure")
        raise AssertionError({"stage": label, "state": last})

    # ------------------------=
    # FUNC: key
    # DESC: Sends a real make/break key event and waits beyond the guest keyboard repeat interval.
    # ------------------=
    def key(self, *codes):
        self.qmp("send-key", {"keys": [{"type": "qcode", "data": code} for code in codes], "hold-time": 80})
        time.sleep(.15)

    # ------------------------=
    # FUNC: text
    # DESC: Types a bounded ASCII operator command through the actual guest keyboard path.
    # ------------------=
    def text(self, value):
        aliases = {" ": "spc", "-": "minus", ".": "dot", "=": "equal", "/": "slash"}
        for character in value:
            if character == ":":
                self.key("shift", "semicolon")
            elif character.isupper():
                self.key("shift", character.lower())
            else:
                self.key(aliases.get(character, character))

    # ------------------------=
    # FUNC: screenshot
    # DESC: Captures the actual display for manual visual review, not a rendered-text success oracle.
    # ------------------=
    def screenshot(self, label):
        target = self.work / f"{label}.ppm"
        self.qmp("screendump", {"filename": str(target)})
        return target

    # ------------------------=
    # FUNC: install
    # DESC: Navigates the unmodified installer and verifies destructive focus and completion from actual state transitions.
    # ------------------=
    def install(self):
        self.boot(True)
        self.wait(lambda state: state[4] == 0 and state[3] == 0, "live startup")
        self.text("1")
        self.key("ret")
        self.wait(lambda state: state[4] == 3 and state[5] == 0, "wizard welcome")
        for step in range(7):
            state = self.wait(lambda state: state[5] == step, f"installer step {step}")
            if step == 6:
                assert state[6] == 0, "Destructive confirmation must default to Cancel"
                self.screenshot("erase-confirmation")
            for _ in range(5):
                if state[6] == 1:
                    break
                self.key("tab")
                state = self.wait(lambda value: value[6] != state[6], "focus advanced")
            assert state[6] == 1
            self.key("ret")
            self.wait(lambda state: state[5] > step and state[5] != 9, f"advance {step}", 300)
        self.wait(lambda state: state[5] == 8, "installation completed", 300)
        self.screenshot("installed-complete")
        self.stop()
        self.boot(False)
        self.wait(lambda state: state[3] == 1 and state[4] == 4, "detached-media installed onboarding", 180)
        self.screenshot("detached-onboarding")

    # ------------------------=
    # FUNC: stop
    # DESC: Powers off only this harness-owned machine and closes its management channel.
    # ------------------=
    def stop(self):
        if self.process is not None:
            if self.process.poll() is None:
                self.qmp("quit")
            self.process.wait(timeout=20)
            self.channel.close()
            self.sock.close()
            self.log.close()
            self.process = None

# ------------------------=
# FUNC: main
# DESC: Runs two independent fresh installs; artifacts and evidence remain in a newly created output directory.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    args = parser.parse_args()
    work = args.output.resolve()
    work.mkdir(parents=True, exist_ok=False)
    guests = []
    try:
        for number in [1, 2]:
            guest = Guest(work, number, args.firmware)
            guests.append(guest)
            guest.install()
            guest.stop()
        (work / "install-result.json").write_text(json.dumps({"independent_installs": 2, "detached_onboarding": True, "full_ms9_lifecycle": False}, indent=2))
    finally:
        for guest in guests:
            guest.stop()

if __name__ == "__main__":
    main()
