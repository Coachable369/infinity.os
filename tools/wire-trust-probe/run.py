"""Phase 9A native-wire engineering/operator acceptance. No installed GUI claim."""
import json
import os
import pathlib
import select
import shutil
import socket
import struct
import subprocess
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
PREFIX = bytes([0x91, 0x9A, 0x4F, 0x50])

class Guest:
    # ------------------------=
    # FUNC: __init__
    # DESC: Boots one independent firmware-entropy identity with a native E1000 and a trusted external diagnostic UART.
    # ------------------=
    def __init__(self, work, node, port):
        volume = work / f"volume-{node}"
        (volume / "EFI/BOOT").mkdir(parents=True)
        (volume / "EFI/INFINITY").mkdir(parents=True)
        shutil.copyfile(ROOT / "build/x86_64/BOOTX64.EFI", volume / "EFI/BOOT/BOOTX64.EFI")
        shutil.copyfile(ROOT / "build/wire-trust-probe.elf", volume / "EFI/INFINITY/KERNEL.ELF")
        self.qmp_path = work / f"qmp-{node}"
        self.log = open(work / f"stderr-{node}.log", "wb")
        self.process = subprocess.Popen([
            "qemu-system-x86_64", "-machine", "pc", "-cpu", "max", "-m", "512M",
            "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
            "-drive", f"format=raw,file=fat:rw:{volume}", "-boot", "order=c",
            "-netdev", f"socket,id=link,{'listen' if node == 1 else 'connect'}=127.0.0.1:{port}",
            "-device", f"e1000,id=nic0,netdev=link,mac=02:00:00:00:00:0{node}",
            "-object", "rng-random,id=rng0,filename=/dev/urandom", "-device", "virtio-rng-pci,rng=rng0",
            "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
            "-qmp", f"unix:{self.qmp_path},server=on,wait=off", "-display", "none",
            "-serial", "stdio", "-no-reboot"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log)
        self.buffer = bytearray()

    # ------------------------=
    # FUNC: rpc
    # DESC: Exchanges binary operator records; assertions use structured fields, not terminal text.
    # ------------------=
    def rpc(self, command, payload=b"", allow_error=False, timeout=15):
        assert len(payload) <= 505
        self.process.stdin.write(PREFIX + bytes([command]) + struct.pack("<H", len(payload)) + payload)
        self.process.stdin.flush()
        return self.response(command, allow_error, timeout)

    # ------------------------=
    # FUNC: response
    # DESC: Reads a framed binary response or startup readiness record without sending premature UART input.
    # ------------------=
    def response(self, command, allow_error=False, timeout=45):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            if self.process.poll() is not None:
                raise AssertionError({"guest_exit": self.process.returncode, "command": command})
            at = self.buffer.find(PREFIX)
            if at >= 0 and len(self.buffer) >= at + 8:
                cmd, status, length = struct.unpack("<BBH", self.buffer[at+4:at+8])
                assert length <= 512
                if len(self.buffer) >= at + 8 + length:
                    data = bytes(self.buffer[at+8:at+8+length])
                    del self.buffer[:at+8+length]
                    assert cmd == command, (cmd, command)
                    if status:
                        if allow_error:
                            return None
                        raise AssertionError({"operator_command": command, "typed_node_error": data[0]})
                    return data
            ready, _, _ = select.select([self.process.stdout], [], [], 0.2)
            if ready:
                chunk = os.read(self.process.stdout.fileno(), 4096)
                if chunk:
                    self.buffer.extend(chunk)
                    assert len(self.buffer) < 2_000_000
        raise AssertionError({"operator_timeout": command})

    # ------------------------=
    # FUNC: qmp
    # DESC: Changes VM execution state through QEMU management only, never forwarding guest protocol payloads.
    # ------------------=
    def qmp(self, command):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
            sock.settimeout(5)
            sock.connect(str(self.qmp_path))
            channel = sock.makefile("rwb")
            json.loads(channel.readline())
            channel.write(b'{"execute":"qmp_capabilities"}\n'); channel.flush()
            while "return" not in json.loads(channel.readline()):
                pass
            channel.write(json.dumps({"execute": command}).encode() + b"\n"); channel.flush()
            while True:
                result = json.loads(channel.readline())
                assert "error" not in result, result
                if "return" in result:
                    return

    # ------------------------=
    # FUNC: close
    # DESC: Reaps only this test's QEMU guest and closes its diagnostic log.
    # ------------------=
    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.process.kill(); self.process.wait()
        self.log.close()

# ------------------------=
# FUNC: eventually
# DESC: Waits for a bounded observable state transition with no string-based acceptance.
# ------------------=
def eventually(action, timeout=45):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        value = action()
        if value is not None and value is not False:
            return value
        time.sleep(0.25)
    raise AssertionError({"transition_timeout_seconds": timeout})

# ------------------------=
# FUNC: received
# DESC: Asserts exact independently decrypted peer identity and payload bytes.
# ------------------=
def received(guest, peer, payload):
    value = eventually(lambda: guest.rpc(7, allow_error=True))
    assert value[:32] == peer
    length = struct.unpack("<H", value[48:50])[0]
    assert value[50:] == payload and length == len(payload)

# ------------------------=
# FUNC: rejected
# DESC: Injects a captured/mutated encrypted frame through native UDP and checks rejection without delivery.
# ------------------=
def rejected(sender, receiver, mode):
    before = struct.unpack("<Q", receiver.rpc(13)[:8])[0]
    sender.rpc(10, bytes([mode]))
    eventually(lambda: struct.unpack("<Q", receiver.rpc(13)[:8])[0] > before)
    assert receiver.rpc(7, allow_error=True) is None

# ------------------------=
# FUNC: run
# DESC: Proves the bounded phase-9A flow on two independently running native guests with external explicit confirmation.
# ------------------=
def run():
    with tempfile.TemporaryDirectory(prefix="infinity-wire-trust-") as temporary:
        work = pathlib.Path(temporary)
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0)); port = reservation.getsockname()[1]
        guests = []
        try:
            a = Guest(work, 1, port); guests.append(a)
            time.sleep(0.5)
            b = Guest(work, 2, port); guests.append(b)
            a.response(255); b.response(255)
            aid = a.rpc(0, timeout=45)[:32]; bid = b.rpc(0, timeout=45)[:32]
            assert aid != bid
            ad = eventually(lambda: a.rpc(1, bid, allow_error=True))
            bd = eventually(lambda: b.rpc(1, aid, allow_error=True))
            assert ad[0] == bd[0] == 1 and ad[10] == bd[10] == 1
            assert struct.unpack("<HH", ad[11:15]) == (49152, 49152)
            assert struct.unpack("<HH", bd[11:15]) == (49152, 49152)
            # A missing peer during PairBegin must clear pending local trust.
            b.qmp("stop")
            abandoned = a.rpc(2, bid + b"\x00")
            eventually(lambda: a.rpc(1, bid)[1] == 3, timeout=45)
            assert a.rpc(3, bid, allow_error=True) is None
            assert a.rpc(1, bid)[0] == 1
            b.qmp("cont")
            eventually(lambda: b.rpc(1, aid)[0] == 2)
            b.rpc(9, abandoned)
            eventually(lambda: a.rpc(1, bid)[1] == 1)
            # An explicit local-only abandoned transaction on B makes local IDs
            # diverge. It grants no trust and is not the wire pairing acceptance.
            b.rpc(11, aid)
            transaction = a.rpc(2, bid + b"\x00")
            av = eventually(lambda: a.rpc(3, bid, allow_error=True))
            bv = eventually(lambda: b.rpc(3, aid, allow_error=True))
            assert av[:68] == bv[:68] and av[:32] == transaction
            assert av[93:125] == aid and bv[93:125] == bid
            assert av[125:157] == bid and bv[125:157] == aid
            assert a.rpc(5, bid, allow_error=True) is None and b.rpc(5, aid, allow_error=True) is None
            code_a = struct.unpack("<I", av[64:68])[0]; code_b = struct.unpack("<I", bv[64:68])[0]
            assert a.rpc(4, transaction + struct.pack("<I", code_a) + b"\x00", allow_error=True) is None
            assert a.rpc(4, transaction + struct.pack("<I", (code_a+1)%1_000_000) + b"\x01", allow_error=True) is None
            # Explicit EXTERNAL operator actions follow comparison of values
            # independently produced inside separate guests; no guest auto-confirms.
            a.rpc(4, transaction + struct.pack("<I", code_a) + b"\x01")
            eventually(lambda: (b.rpc(3, aid, allow_error=True) or bytes(93))[92] == 5)
            assert a.rpc(1, bid)[0] != 3 and b.rpc(1, aid)[0] != 3
            b.rpc(4, transaction + struct.pack("<I", code_b) + b"\x01")
            eventually(lambda: a.rpc(1, bid)[0] == b.rpc(1, aid)[0] == 3)
            a.rpc(2, bid + b"\x01")
            sa = eventually(lambda: a.rpc(5, bid, allow_error=True))
            sb = eventually(lambda: b.rpc(5, aid, allow_error=True))
            assert sa[:8] != sb[:8] and sa[8:24] == sb[8:24]
            assert int.from_bytes(a.rpc(15), "little") & 0xC0 == 0xC0
            first_reference = sa[8:24]
            a.rpc(6, bid + bytes([1, 9, 0, 255])); received(b, aid, bytes([1, 9, 0, 255]))
            assert struct.unpack("<Q", a.rpc(5, bid)[24:32])[0] == 1
            assert struct.unpack("<Q", b.rpc(5, aid)[24:32])[0] == 0
            b.rpc(6, aid + bytes([2, 8])); received(a, bid, bytes([2, 8]))
            b.rpc(6, aid + bytes([3, 7, 5])); received(a, bid, bytes([3, 7, 5]))
            assert struct.unpack("<Q", b.rpc(5, aid)[24:32])[0] == 2
            a.rpc(6, bid + bytes([4, 6])); received(b, aid, bytes([4, 6]))
            a.rpc(12); b.rpc(12)
            rejected(a, b, 0); rejected(b, a, 0)
            for mode in [2, 3, 4, 5, 6]:
                rejected(a, b, mode)
            a.rpc(8, bid)
            eventually(lambda: b.rpc(5, aid, allow_error=True) is None)
            assert a.rpc(5, bid, allow_error=True) is None
            assert int.from_bytes(a.rpc(15), "little") & 0x20
            a.rpc(2, bid + b"\x01")
            sa2 = eventually(lambda: a.rpc(5, bid, allow_error=True)); sb2 = eventually(lambda: b.rpc(5, aid, allow_error=True))
            assert sa2[8:24] == sb2[8:24] and sa2[8:24] != first_reference
            assert sa2[:8] != sa[:8] and sb2[:8] != sb[:8]
            a.rpc(6, bid + bytes([5, 42])); received(b, aid, bytes([5, 42]))
            rejected(a, b, 1)
            b.qmp("stop")
            eventually(lambda: a.rpc(5, bid, allow_error=True) is None, timeout=45)
            assert a.rpc(1, bid)[1] == 3  # Reachability::Offline
            b.qmp("cont")
            eventually(lambda: a.rpc(1, bid)[1] == 1, timeout=45)
            assert a.rpc(5, bid, allow_error=True) is None
            a.rpc(2, bid + b"\x01")
            sa3 = eventually(lambda: a.rpc(5, bid, allow_error=True)); sb3 = eventually(lambda: b.rpc(5, aid, allow_error=True))
            assert sa3[8:24] == sb3[8:24] and sa3[8:24] != sa2[8:24]
            b.rpc(6, aid + bytes([6, 43])); received(a, bid, bytes([6, 43]))
            rejected(a, b, 1)
            report = {"phase": "9A", "boundary": "PRODUCTION WIRE / ENGINEERING GUEST",
                "independent_node_ids": [aid.hex(), bid.hex()], "independent_handles": [int.from_bytes(sa[:8], "little"), int.from_bytes(sb[:8], "little")],
                "verification_independently_matched": True, "external_confirmations": 2,
                "offline_during_pair_begin": True,
                "native_duplex": True, "fresh_session_references": [first_reference.hex(), sa2[8:24].hex(), sa3[8:24].hex()],
                "peer_return": "QEMU stop/cont; not an installed cold reboot", "installed_gui_acceptance": False}
            (ROOT / "build/milestone-9a-wire-proof.json").write_text(json.dumps(report, indent=2) + "\n")
            print(json.dumps(report))
        except Exception:
            for guest in guests:
                guest.log.flush()
                print({"guest_exit": guest.process.poll(), "operator_buffer_hex": guest.buffer[-128:].hex()})
            raise
        finally:
            for guest in guests:
                guest.close()

if __name__ == "__main__":
    run()
