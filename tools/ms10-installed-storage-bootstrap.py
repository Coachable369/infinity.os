"""Focused installed recipient-service boot acceptance, NOT full MS10 closure.

Uses the ordinary installer, onboarding and cold login on independently created
disks. No guest-memory writes, injected grants, shell parsing or prose oracles.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib.util
import json
import pathlib
import shutil
import struct

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("installed_acceptance", ROOT / "tools/ms9-installed-acceptance.py")
installed = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installed)

# ------------------------=
# FUNC: provision
# DESC: Installs one blank owned disk, detaches media, configures through real UI and proves the mounted replica service survives cold authenticated boot.
# ------------------=
def provision(guest, resume=False):
    try:
        if resume:
            guest.boot(False)
            before = guest.authenticate()
            result = guest.cold_boot_proof(before)
        else:
            guest.install()
            state = guest.wait(lambda state: state[108] == 3, "mounted native replica service")
            assert state[3] == 1
            guest.stop()
            result = guest.onboard()
        state = guest.wait(lambda state: state[108] == 3 and state[4] == 5, "cold installed recipient service")
        result["replica_service_ready_after_cold_boot"] = True
        result["installer_detached"] = not guest.installer
        assert result["installer_detached"]
        guest.launch("command", 5)
        guest.command("storage status")
        state = guest.wait(lambda state: state[110] == 1, "authoritative local storage IOP observation")
        raw = struct.pack("<17Q", *state[111:128])
        assert struct.unpack_from("<HHI", raw) == (1, 48, 0xe002)
        resource, device = raw[72:88].hex(), raw[88:104].hex()
        assert resource != "00" * 16 and device != "00" * 16
        capacity, available = struct.unpack_from("<QQ", raw, 40)
        reserved = struct.unpack_from("<Q", raw, 104)[0]
        assert capacity > 0 and available <= capacity and reserved <= capacity - available
        with guest.disk.open("rb") as image:
            image.seek(512); header = image.read(512)
            assert header[56:72].hex() == device
            entries, count, size = struct.unpack_from("<QII", header, 72)
            assert count <= 128 and size == 128
            image.seek(entries * 512); partitions = image.read(count * size)
            assert any(partitions[at+16:at+32].hex() == resource for at in range(0, len(partitions), size))
        result["resource_id"] = resource
        result["device_id"] = device
        result["local_storage_iop"] = True
        result["capacity_bytes"] = capacity
        result["available_bytes"] = available
        result["reserved_bytes"] = reserved
        guest.screenshot("recipient-service-desktop")
        guest.frame_report("recipient-service-desktop")
        (guest.work / "recipient-result.json").write_text(json.dumps(result, indent=2))
        return result
    except Exception:
        if guest.process is not None and guest.process.poll() is None:
            (guest.work / "failure-state.json").write_text(json.dumps(guest.state()))
            guest.screenshot("recipient-service-failure")
        raise
    finally:
        guest.stop()

# ------------------------=
# FUNC: main
# DESC: Pins exact artifacts and performs a finite three-or-four-node boot test without claiming transfer, healing, remote-read or complete-milestone acceptance.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--nodes", type=int, choices=(3, 4), default=3)
    parser.add_argument("--resume", action="store_true", help="Reverify existing independently installed disks using their pinned artifacts; never reinstall or substitute current builds")
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    args = parser.parse_args()
    work = args.output.resolve()
    if not args.resume:
        work.mkdir(parents=True, exist_ok=False)
    artifacts = work / "artifacts"
    if not args.resume:
        artifacts.mkdir()
    hashes = {}
    for source, name in [(ROOT / "builds/InfinityOS-x86_64.iso", "installer.iso"),
                         (ROOT / "build/x86_64/kernel.elf", "kernel.elf"),
                         (ROOT / "build/x86_64/installed-kernel.elf", "installed-kernel.elf")]:
        target = artifacts / name
        if not args.resume:
            shutil.copyfile(source, target)
        with target.open("rb") as stream:
            hashes[name] = hashlib.file_digest(stream, "sha256").hexdigest()
    if args.resume:
        assert hashes == json.loads((artifacts / "sha256.json").read_text()), "Pinned acceptance artifacts changed"
    else:
        (artifacts / "sha256.json").write_text(json.dumps(hashes, indent=2))
    guests = [installed.Guest(work, n, args.firmware, reuse=args.resume) for n in range(1, args.nodes+1)]
    results = []
    with ThreadPoolExecutor(max_workers=2) as workers:
        # Do not launch another batch after a failed install or an identity
        # collision. Preserve each completed node's receipt for focused reuse.
        for start in range(0, len(guests), 2):
            batch = guests[start:start+2]
            results.extend(workers.map(provision, batch, [args.resume] * len(batch)))
            assert len({result["node_id"] for result in results}) == len(results)
            assert len({result["resource_id"] for result in results}) == len(results)
            assert len({result["device_id"] for result in results}) == len(results)
    (work / "result.json").write_text(json.dumps({"boundary": "installed QEMU recipient-service bootstrap",
        "independent_installs": args.nodes, "nodes": results, "full_ms10_acceptance": False}, indent=2))

if __name__ == "__main__":
    main()
