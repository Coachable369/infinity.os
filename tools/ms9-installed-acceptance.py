"""Installed-OS acceptance through real input and read-only binary debugger state.

Never parses guest terminal text. Only fresh, harness-owned disks are modified.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import hashlib
import pathlib
import shutil
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
    def __init__(self, work, number, firmware, reuse=False, width=2048, height=2048):
        self.work = work / f"node-{number}"
        if not reuse:
            self.work.mkdir()
        self.number = number
        self.firmware = firmware
        self.disk = self.work / "installed.raw"
        if reuse:
            assert self.disk.is_file() and self.disk.stat().st_size == 32 * 1024**3
        else:
            with self.disk.open("xb") as stream:
                stream.truncate(32 * 1024**3)
        self.process = None
        self.channel = None
        self.capture = 0
        self.mesh_port = None
        self.width, self.height = width, height
        self.last_pairing = None
        self.input_latency_ns = []

    # ------------------------=
    # FUNC: boot
    # DESC: Boots either original installer media or disk alone; cold boot always creates a new emulator process.
    # ------------------=
    def boot(self, installer):
        assert self.process is None
        self.installer = installer
        elf = self.work.parent / "artifacts" / ("kernel.elf" if installer else "installed-kernel.elf")
        self.address, self.length = symbol(elf, "INFINITY_DIAGNOSTIC_SNAPSHOT")
        self.frames_address, self.frames_length = symbol(elf, "INFINITY_DIAGNOSTIC_FRAMES")
        qmp = self.work / "qmp.sock"
        qmp.unlink(missing_ok=True)
        self.log = (self.work / ("installer.log" if installer else "installed.log")).open("ab")
        command = ["qemu-system-x86_64", "-machine", "pc", "-cpu", "max", "-m", "4096M",
                   "-vga", "none", "-device", f"VGA,xres={self.width},yres={self.height},xmax={self.width},ymax={self.height}",
                   "-drive", f"if=pflash,format=raw,readonly=on,file={self.firmware}",
                   "-drive", f"if=ide,index=0,format=raw,file={self.disk}",
                   "-netdev", (f"socket,id=net,{'listen' if self.number == 1 else 'connect'}=127.0.0.1:{self.mesh_port}" if self.mesh_port else "user,id=net"), "-device", f"e1000,netdev=net,mac=02:00:00:00:09:{self.number:02x}",
                   "-object", "rng-random,id=rng0,filename=/dev/urandom", "-device", "virtio-rng-pci,rng=rng0",
                   "-qmp", f"unix:{qmp},server=on,wait=off", "-display", "none", "-serial", "stdio", "-no-reboot"]
        if installer:
            command += ["-cdrom", str(self.work.parent / "artifacts/installer.iso"), "-boot", "order=d"]
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
        lifecycle = values[32:37] + values[56:68]
        if lifecycle != self.last_pairing:
            self.last_pairing = lifecycle
            with (self.work / "pairing-lifecycle.jsonl").open("a") as trace:
                trace.write(json.dumps({"clock": values[10], "flags": values[9], "editor": values[54:56], "view": values[32:48], "lifecycle": values[56:68]}) + "\n")
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
                if label != "guest input-loop progress":
                    print(json.dumps({"node": self.number, "stage": label, "snapshot": last[2]}), flush=True)
                return last
            time.sleep(.25)
        self.screenshot("failure")
        self.frame_report("failure")
        (self.work / "failure-registers.json").write_text(json.dumps(
            self.qmp("human-monitor-command", {"command-line": "info registers"})))
        raise AssertionError({"stage": label, "state": last})

    # ------------------------=
    # FUNC: frame_report
    # DESC: Captures measured guest frame samples without treating telemetry availability as performance acceptance.
    # ------------------=
    def frame_report(self, label):
        self.qmp("stop")
        try:
            values = struct.unpack("<10802Q", self.memory(self.frames_address, self.frames_length))
        finally:
            self.qmp("cont")
        if values[0] != 1:
            return {"sample_count": 0, "available": False, "performance_acceptance": False}
        latest = values[1]
        samples = []
        for index in range(3600):
            sequence, duration, timestamp = values[2 + index * 3:5 + index * 3]
            if max(1, latest - 3599) <= sequence <= latest and (sequence - 1) % 3600 == index and duration != 0xffffffffffffffff:
                samples.append(duration)
        samples.sort()
        report = {"boundary": "LIVE INSTALLER QEMU OBSERVATION" if self.installer else "INSTALLED QEMU OBSERVATION", "latest_sequence": latest,
                  "sample_count": len(samples), "performance_acceptance": False}
        if samples:
            report.update({"average_ns": sum(samples) // len(samples),
                           "p95_ns": samples[(len(samples) * 95 + 99) // 100 - 1],
                           "worst_ns": samples[-1]})
        (self.work / f"{label}-frames.json").write_text(json.dumps(report, indent=2))
        return report

    # ------------------------=
    # FUNC: key
    # DESC: Sends a real make/break event and applies backpressure against actual guest event-loop progress.
    # ------------------=
    def key(self, *codes):
        before = self.state()
        self.qmp("send-key", {"keys": [{"type": "qcode", "data": code} for code in codes], "hold-time": 150})
        if before is not None and not (before[4] == 3 and before[5] == 6 and codes == ("ret",)):
            self.wait(lambda state: state[2] >= before[2] + 4, "guest input-loop progress", timeout=30)
        else:
            time.sleep(.5)

    # ------------------------=
    # FUNC: text
    # DESC: Types a bounded ASCII operator command through the actual guest keyboard path.
    # ------------------=
    def text(self, value):
        aliases = {" ": "spc", "-": "minus", ".": "dot", "=": "equal", "/": "slash"}
        for character in value:
            if character == ":":
                codes = ("shift", "semicolon")
            elif character.isupper():
                codes = ("shift", character.lower())
            else:
                codes = (aliases.get(character, character),)
            before = self.state()
            if before is None or before[71] == 0:
                self.key(*codes)
                continue
            started = time.monotonic_ns()
            self.qmp("send-key", {"keys": [{"type": "qcode", "data": code} for code in codes], "hold-time": 150})
            accepted = self.wait(lambda state: state[71] > before[71], "accepted non-secret input", timeout=30)
            assert accepted[71] == before[71] + 1 and accepted[4] == before[4]
            self.input_latency_ns.append(time.monotonic_ns() - started)

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
            # TCG verification reads the complete installed kernel. This is a
            # host acceptance bound, not a guest protocol/authority deadline.
            self.wait(lambda state: state[5] > step and state[5] != 9, f"advance {step}", 900 if step == 6 else 300)
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
            if self.input_latency_ns:
                (self.work / "input-observations.json").write_text(json.dumps({
                    "boundary": "QEMU keyboard submission to accepted-length snapshot",
                    "poll_resolution_ms": 250, "samples_ns": self.input_latency_ns,
                    "performance_acceptance": False}))
            if self.process.poll() is None:
                self.qmp("quit")
            self.process.wait(timeout=20)
            self.channel.close()
            self.sock.close()
            self.log.close()
            self.process = None

    # ------------------------=
    # FUNC: onboard
    # DESC: Configures a synthetic local operator through real first-boot fields and verifies a cold-boot authenticated desktop.
    # ------------------=
    def onboard(self):
        self.boot(False)
        initial = self.wait(lambda state: state[3] == 1 and state[4] in (4, 9), "installed local UI")
        if initial[4] == 9:
            before = self.authenticate()
            return self.cold_boot_proof(before)
        assert initial[7] == 0, "Fresh account setup must begin at the welcome step"
        self.key("ret")
        for step in range(1, 5):
            self.wait(lambda state: state[7] == step, f"configuration input {step}")
            self.text("MeshProof901" if step == 4 else f"ms9node0{self.number}")
            self.key("ret")
            self.wait(lambda state: state[7] == step + 1 and not state[9] & 8, f"configuration commit {step}")
        self.key("ret")
        state = self.wait(lambda state: state[7] == 6, "network configuration")
        # NAT presents a wired adapter. Select the real wired operation, not Offline.
        while state[8] != 2:
            self.key("down")
            state = self.wait(lambda value: value[8] != state[8], "wired focus")
        self.key("ret")
        self.wait(lambda state: state[8] == 1 and not state[9] & 8, "wired selected")
        self.key("ret")
        self.wait(lambda state: state[7] == 7 and not state[9] & 8, "network committed")
        self.key("ret")
        before = self.wait(lambda state: state[4] == 5 and state[9] & 3 == 3, "authenticated desktop")
        assert any(before[16:20]), "Node identity must exist on the installed system"
        self.screenshot("first-desktop")
        return self.cold_boot_proof(before)

    # ------------------------=
    # FUNC: authenticate
    # DESC: Uses the actual focused password field rather than assuming an extra Tab is necessary.
    # ------------------=
    def authenticate(self):
        state = self.wait(lambda state: state[3] == 1 and state[4] == 9, "cold boot authentication")
        for _ in range(11):
            if state[8] == 1:
                break
            self.key("tab")
            state = self.wait(lambda value: value[8] != state[8], "password focus")
        assert state[8] == 1
        self.text("MeshProof901")
        self.key("ret")
        return self.wait(lambda state: state[4] == 5 and state[9] & 3 == 3, "authenticated desktop")

    # ------------------------=
    # FUNC: cold_boot_proof
    # DESC: Verifies authentication and identity preservation across termination and a new emulator process without media.
    # ------------------=
    def cold_boot_proof(self, before):
        self.stop()
        self.boot(False)
        after = self.authenticate()
        assert after[16:20] == before[16:20], "Cold boot must preserve the public node identity"
        self.screenshot("cold-boot-desktop")
        return {"node_id": struct.pack("<4Q", *after[16:20]).hex(), "cold_boot_identity": True, "authenticated_desktop": True}

    # ------------------------=
    # FUNC: launch
    # DESC: Uses the real searchable application launcher and verifies its resulting native surface.
    # ------------------=
    def launch(self, query, mode, section=None):
        self.key("slash")
        self.wait(lambda state: state[4] == 6, "launcher opened")
        self.text(query)
        self.key("ret")
        return self.wait(lambda state: state[4] == mode and (section is None or state[8] == section), f"launch {query}")

    # ------------------------=
    # FUNC: configure_peer
    # DESC: Uses native network Settings and Console IOP to provision one explicit endpoint on the installed system.
    # ------------------=
    def configure_peer(self, network_label):
        self.launch(network_label, 8, 6)
        self.key("right")
        self.key("right")
        self.key("down")
        self.key("ret")
        self.wait(lambda state: state[9] & 4, "static address editor")
        self.text(f"10.42.0.{self.number}")
        self.key("ret")
        self.wait(lambda state: not state[9] & 12, "static address committed")
        self.key("esc")
        self.wait(lambda state: state[4] == 5, "return to desktop")
        self.launch("command", 5)
        before = self.state()[20]
        self.text(f"node link-configure 1 local=10.42.0.{self.number} remote=10.42.0.{3-self.number} local-port=49152 remote-port=49152")
        self.key("ret")
        self.wait(lambda state: state[20] > before and state[29] == 1 and state[48] > 0, "durable native peer connection")
        self.key("esc")
        self.wait(lambda state: state[4] == 5, "command window closed")

    # ------------------------=
    # FUNC: select_peer
    # DESC: Opens the native node inspector and selects an actually discovered peer.
    # ------------------=
    def select_peer(self, nodes_label):
        self.launch(nodes_label, 8, 7)
        self.key("ret")
        return self.wait(lambda state: any(state[32:36]), "selected discovered node")

    # ------------------------=
    # FUNC: confirm_peer
    # DESC: Enters each public verification digit through normal key events and waits for its actual acceptance within unchanged security leases.
    # ------------------=
    def confirm_peer(self, state, moves, delay_last=0):
        for _ in range(moves):
            self.key("down")
        self.key("ret")
        ready = self.wait(lambda value: value[9] & 4, "trusted confirmation input")
        assert ready[54] == 0 and ready[55] > ready[10]
        for count, digit in enumerate(f"{state[37]:06d}", 1):
            self.qmp("send-key", {"keys": [{"type": "qcode", "data": digit}], "hold-time": 150})
            accepted = self.wait(lambda value: value[54] >= count or value[9] & 8, "accepted confirmation digit", timeout=15)
            assert accepted[54] == count and not accepted[9] & 8
            assert accepted[10] < accepted[55] and accepted[36] == state[36], {"lost_pairing_during_input": accepted[:68]}
            if count == 5 and delay_last:
                resumed = self.wait(lambda value: value[10] >= accepted[10] + delay_last or value[36] != state[36], "normal runtime before final digit", timeout=30)
                assert resumed[36] == state[36] and resumed[10] < resumed[55], {"lost_pairing_during_review": resumed[:68]}
        self.qmp("send-key", {"keys": [{"type": "qcode", "data": "ret"}], "hold-time": 150})
        result = self.wait(lambda value: not value[9] & 4 or value[9] & 8, "explicit operator confirmation", timeout=15)
        assert not result[9] & 12, {"confirmation_failure": result[:56]}

    # ------------------------=
    # FUNC: approve_membership
    # DESC: Grants node-control policy through existing Settings controls and submits explicit local join consent.
    # ------------------=
    def approve_membership(self, page):
        for _ in range(3 - page):
            self.key("right")
        self.key("down")
        for _ in range(2):
            before = self.state()[20]
            self.key("ret")
            changed = self.wait(lambda value: value[20] > before or value[9] & 8, "node-control policy commit")
            assert not changed[9] & 8
        self.key("left")
        self.key("down")
        self.key("ret")
        assert not self.state()[9] & 8

    # ------------------------=
    # FUNC: command
    # DESC: Submits a normal operator command and verifies the native input buffer was consumed, never its rendered output.
    # ------------------=
    def command(self, value):
        self.text(value)
        self.key("ret")
        return self.wait(lambda state: state[71] == 1, "operator command consumed")

    # ------------------------=
    # FUNC: peer_policy
    # DESC: Changes an explicit peer policy through native Console and checks the shared authoritative projection.
    # ------------------=
    def peer_policy(self, peer, category, choice):
        index = {"object": 0, "namespace": 1}[category]
        expected = {"deny": 0, "allow": 1}[choice]
        before = self.state()[20]
        self.command(f"node policy-update node:{peer} name={category} value={choice}")
        return self.wait(lambda state: state[20] > before and not state[22]
                         and struct.pack("<16Q", *state[128:144])[86 + index] == expected,
                         "committed peer policy projection")

    # ------------------------=
    # FUNC: peer_grant
    # DESC: Issues one explicitly consented expiring peer operation and reads its actual native grant handle.
    # ------------------=
    def peer_grant(self, peer, operation):
        before = self.state()[87]
        self.command(f"node capability-grant node:{peer} name={operation} seconds=3600 confirm=true")
        return self.wait(lambda state: state[87] > before and state[88] == 0
                         and state[89] > state[10], "scoped peer grant committed")[87]

    # ------------------------=
    # FUNC: remote_call
    # DESC: Exercises the installed asynchronous operator broker and asserts the actual correlated wire result code.
    # ------------------=
    def remote_call(self, peer, grant, expected, mutation=False, domain=False):
        before = self.state()[72]
        action = "remote-policy-update" if mutation else "remote-domain" if domain else "remote-read"
        suffix = " name=object value=deny" if mutation else ""
        self.command(f"node {action} node:{peer} grant={grant}{suffix}")
        request = self.wait(lambda state: state[72] > before, "remote request admitted")[72]
        # Collection may precede the asynchronous wire completion. Re-query
        # the same owned result; never retry or resubmit the remote operation.
        deadline = time.monotonic() + 120
        result = self.state()
        while result is None or result[73] != request:
            assert time.monotonic() < deadline, {"uncollected_remote_request": request}
            self.command(f"node remote-result {request}")
            result = self.state()
        assert result[76] == expected, {"remote_request": request, "actual": result[76], "expected": expected}
        assert result[74] != 0 and result[75] != 0
        return result

# ------------------------=
# FUNC: installed_remote_acceptance
# DESC: Requires genuine installed operator grants, peer-authorized remote inspection/mutation, scope denial, policy denial and live revocation.
# ------------------=
def installed_remote_acceptance(a, b, nodes_label):
    aid = struct.pack("<4Q", *a.state()[16:20]).hex()
    bid = struct.pack("<4Q", *b.state()[16:20]).hex()
    for guest in (a, b):
        guest.key("esc")
        guest.launch("command", 5)
    inspection = b.peer_grant(aid, "inspect")
    b.peer_policy(aid, "object", "allow")
    result = a.remote_call(bid, inspection, 1)
    response = struct.pack("<10Q", *result[77:87])
    assert response[:32].hex() == aid
    assert struct.unpack_from("<I", response, 56)[0] == 0xd002
    assert result[91] == 128
    detail = struct.pack("<16Q", *result[92:108])
    authoritative = struct.pack("<16Q", *b.state()[128:144])
    assert detail[:76] == authoritative[:76] and detail[84:118] == authoritative[84:118]
    # Diagnostic status 1 is success; error discriminants are encoded plus one.
    a.remote_call(bid, inspection, 13, mutation=True)  # CapabilityScopeDenied
    mutation = b.peer_grant(aid, "policy-update")
    b.peer_policy(aid, "namespace", "allow")
    before = b.state()[20]
    a.remote_call(bid, mutation, 1, mutation=True)
    b.wait(lambda state: state[20] > before and not state[22]
           and struct.pack("<16Q", *state[128:144])[86] == 0, "remote mutation reached peer state")
    a.remote_call(bid, inspection, 14)  # PolicyDenied
    b.peer_policy(aid, "object", "allow")
    b.command(f"node capability-revoke {inspection}")
    a.remote_call(bid, inspection, 12)  # CapabilityRevoked
    b.peer_policy(aid, "namespace", "deny")
    for guest in (a, b):
        guest.screenshot("remote-operator-result")
        guest.key("esc")
        guest.select_peer(nodes_label)

# ------------------------=
# FUNC: main
# DESC: Runs two independent fresh installs; artifacts and evidence remain in a newly created output directory.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    parser.add_argument("--resume-installed", action="store_true")
    parser.add_argument("--mesh-installed", action="store_true")
    parser.add_argument("--network-label", default="network")
    parser.add_argument("--nodes-label", default="nodes")
    parser.add_argument("--width", type=int, default=2048)
    parser.add_argument("--height", type=int, default=2048)
    parser.add_argument("--confirmation-delay", type=int, default=5)
    parser.add_argument("--remote-installed", action="store_true")
    args = parser.parse_args()
    assert 640 <= args.width <= 4096 and 480 <= args.height <= 4096
    assert 0 <= args.confirmation_delay <= 10
    work = args.output.resolve()
    if args.resume_installed or args.mesh_installed:
        assert json.loads((work / "install-result.json").read_text())["independent_installs"] == 2
    else:
        work.mkdir(parents=True, exist_ok=False)
        artifacts = work / "artifacts"
        artifacts.mkdir()
        hashes = {}
        for source, name in [(ROOT / "builds/InfinityOS-x86_64.iso", "installer.iso"),
                             (ROOT / "build/x86_64/kernel.elf", "kernel.elf"),
                             (ROOT / "build/x86_64/installed-kernel.elf", "installed-kernel.elf")]:
            target = artifacts / name
            shutil.copyfile(source, target)
            with target.open("rb") as stream:
                hashes[name] = hashlib.file_digest(stream, "sha256").hexdigest()
        (artifacts / "sha256.json").write_text(json.dumps(hashes, indent=2))
    guests = []
    results = []
    try:
        if args.mesh_installed:
            known = json.loads((work / "onboarding-result.json").read_text())
            with socket.socket() as reserve:
                reserve.bind(("127.0.0.1", 0))
                port = reserve.getsockname()[1]
            for number in [1, 2]:
                guest = Guest(work, number, args.firmware, reuse=True, width=args.width, height=args.height)
                guests.append(guest)
                guest.mesh_port = port
                guest.boot(False)
            with ThreadPoolExecutor(max_workers=2) as workers:
                authenticated = list(workers.map(lambda guest: guest.authenticate(), guests))
            for guest, current in zip(guests, authenticated):
                assert struct.pack("<4Q", *current[16:20]).hex() == known[guest.number - 1]["node_id"]
            with ThreadPoolExecutor(max_workers=2) as workers:
                list(workers.map(lambda guest: guest.configure_peer(args.network_label), guests))
            for guest in guests:
                guest.wait(lambda state: state[24] == 1 and state[22] == 0, "installed discovery")
                guest.select_peer(args.nodes_label)
                guest.key("right")
            a, b = guests
            a.key("down")
            a.key("ret")
            av = a.wait(lambda state: state[36] != 0, "local authenticated pairing transcript")
            bv = b.wait(lambda state: state[36] != 0, "peer authenticated pairing transcript")
            assert av[37] == bv[37] and av[40:48] == bv[40:48]
            assert av[32:36] == bv[16:20] and bv[32:36] == av[16:20]
            # Snapshot publication precedes rendering; allow the preceding refresh
            # to finish before capturing its visible result, without another UI action.
            for guest, state in [(a, av), (b, bv)]:
                guest.wait(lambda value: value[2] >= state[2] + 2, "pairing presentation refresh", timeout=15)
            a.screenshot("pairing-verification")
            b.screenshot("pairing-verification")
            with ThreadPoolExecutor(max_workers=2) as workers:
                list(workers.map(lambda item: item[0].confirm_peer(item[1], item[2], item[3]), [(a, av, 2, 0), (b, bv, 3, args.confirmation_delay)]))
            for guest in guests:
                guest.wait(lambda state: state[25] == 1, "dual-confirmed installed trust")
            a.key("left")
            for _ in range(3):
                a.key("down")
            a.key("ret")
            for guest in guests:
                guest.wait(lambda state: state[26] == 1, "installed secure session")
                guest.screenshot("secure-session")
                guest.frame_report("secure-session")
            report = {"installed_discovery": True, "installed_dual_confirmation": True, "installed_secure_session": True, "full_ms9_lifecycle": False}
            (work / "mesh-result.json").write_text(json.dumps(report, indent=2))
            if args.remote_installed:
                installed_remote_acceptance(a, b, args.nodes_label)
                report["installed_remote_allow_deny_revoke"] = True
                (work / "mesh-result.json").write_text(json.dumps(report, indent=2))
            with ThreadPoolExecutor(max_workers=2) as workers:
                list(workers.map(lambda item: item[0].approve_membership(item[1]), [(a, 0), (b, 0 if args.remote_installed else 1)]))
            joined = [guest.wait(lambda state: state[27] == 1 and state[28] == 0 and state[22] == 0, "installed synchronized join") for guest in guests]
            assert joined[0][384:394] == joined[1][384:394]
            for guest in guests:
                guest.screenshot("synchronized-join")
            for _ in range(3):
                a.key("down")
            a.key("ret")
            left = [guest.wait(lambda state: state[27] == 0 and state[28] == 0 and state[392] > joined[index][392], "installed synchronized leave") for index, guest in enumerate(guests)]
            assert left[0][384:394] == left[1][384:394]
            report["installed_join_leave"] = True
            (work / "mesh-result.json").write_text(json.dumps(report, indent=2))
            for guest in guests:
                guest.stop()
            for guest in guests:
                guest.boot(False)
            for index, guest in enumerate(guests):
                state = guest.authenticate()
                assert state[16:20] == joined[index][16:20]
                restored = guest.wait(lambda value: value[25] == 1 and value[29] == 1 and value[48] > 0 and value[22] == 0, "installed trusted state restored")
                assert restored[26] == 0 and restored[384:394] == left[index][384:394]
            report["installed_trust_membership_cold_boot"] = True
            (work / "mesh-result.json").write_text(json.dumps(report, indent=2))
            return
        for number in [1, 2]:
            guest = Guest(work, number, args.firmware, reuse=args.resume_installed, width=args.width, height=args.height)
            guests.append(guest)
        with ThreadPoolExecutor(max_workers=2) as workers:
            results = list(workers.map(lambda guest: guest.onboard() if args.resume_installed else guest.install(), guests))
        for guest in guests:
            guest.stop()
        if args.resume_installed:
            assert results[0]["node_id"] != results[1]["node_id"]
            (work / "onboarding-result.json").write_text(json.dumps(results, indent=2))
        else:
            (work / "install-result.json").write_text(json.dumps({"independent_installs": 2, "detached_onboarding": True, "full_ms9_lifecycle": False}, indent=2))
    except Exception:
        for guest in guests:
            if guest.process is not None and guest.process.poll() is None:
                try:
                    state = guest.state()
                    (guest.work / "failure-state.json").write_text(json.dumps(state))
                    guest.screenshot("failure")
                except Exception:
                    pass
        raise
    finally:
        for guest in guests:
            guest.stop()

if __name__ == "__main__":
    main()
