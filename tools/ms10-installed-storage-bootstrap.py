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
import ms10_installed_pool

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("installed_acceptance", ROOT / "tools/ms9-installed-acceptance.py")
installed = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installed)

# ------------------------=
# FUNC: prior_result
# DESC: Reuses only completed detached-install receipts on existing harness-owned disks, without rerunning successful setup.
# ------------------=
def prior_result(guest):
    path = guest.work / "recipient-result.json"
    if not path.exists():
        return None
    result = json.loads(path.read_text())
    for field in ("cold_boot_identity", "authenticated_desktop", "installer_detached",
                  "replica_service_ready_after_cold_boot", "local_storage_iop"):
        assert result.get(field) is True, {"incomplete_prior_receipt": str(path), "field": field}
    assert guest.disk.is_file() and guest.disk.stat().st_size == 32 * 1024**3
    assert len(bytes.fromhex(result["node_id"])) == 32
    assert len(bytes.fromhex(result["resource_id"])) == 16
    assert len(bytes.fromhex(result["device_id"])) == 16
    return result

# ------------------------=
# FUNC: capture_failure
# DESC: Preserves the first operation failure even when diagnostic storage itself is unavailable.
# ------------------=
def capture_failure(guest, error):
    report = {"node": guest.number, "first_failure": repr(error), "failure_errno": getattr(error, "errno", None)}
    try:
        if guest.process is not None and guest.process.poll() is None:
            report["state"] = guest.state()
            guest.screenshot("recipient-service-failure")
    except Exception as diagnostic:
        report["diagnostic_failure"] = repr(diagnostic)
        report["diagnostic_errno"] = getattr(diagnostic, "errno", None)
    print(json.dumps(report), flush=True)
    try:
        (guest.work / "bootstrap-failure.json").write_text(json.dumps(report, indent=2))
    except OSError:
        pass  # The original failure is still emitted and re-raised by the caller.

# ------------------------=
# FUNC: provision
# DESC: Installs one blank owned disk, detaches media, configures through real UI and proves the mounted replica service survives cold authenticated boot.
# ------------------=
def provision(guest, resume=False, pool=False):
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
        if pool:
            result["pool"] = ms10_installed_pool.verify(guest)
        guest.screenshot("recipient-service-desktop")
        guest.frame_report("recipient-service-desktop")
        (guest.work / "recipient-result.json").write_text(json.dumps(result, indent=2))
        return result
    except Exception as error:
        capture_failure(guest, error)
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
    parser.add_argument("--resume-pending", action="store_true", help="Preserve completed node receipts; install only unfinished harness nodes serially using pinned artifacts")
    parser.add_argument("--pool", action="store_true", help="Exercise native local Pool creation, read, update, policy and detached cold-reboot persistence")
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    args = parser.parse_args()
    assert not (args.resume and args.resume_pending)
    existing = args.resume or args.resume_pending
    work = args.output.resolve()
    if not existing:
        work.mkdir(parents=True, exist_ok=False)
    artifacts = work / "artifacts"
    if not existing:
        artifacts.mkdir()
    hashes = {}
    for source, name in [(ROOT / "builds/InfinityOS-x86_64.iso", "installer.iso"),
                         (ROOT / "build/x86_64/kernel.elf", "kernel.elf"),
                         (ROOT / "build/x86_64/installed-kernel.elf", "installed-kernel.elf")]:
        target = artifacts / name
        if not existing:
            shutil.copyfile(source, target)
        with target.open("rb") as stream:
            hashes[name] = hashlib.file_digest(stream, "sha256").hexdigest()
    if existing:
        assert hashes == json.loads((artifacts / "sha256.json").read_text()), "Pinned acceptance artifacts changed"
    else:
        (artifacts / "sha256.json").write_text(json.dumps(hashes, indent=2))
    guests = [installed.Guest(work, n, args.firmware, reuse=existing) for n in range(1, args.nodes+1)]
    results = []
    if args.resume_pending:
        for guest in guests:
            saved = prior_result(guest)
            results.append(saved if saved is not None else provision(guest, pool=args.pool))
            assert len({result["node_id"] for result in results}) == len(results)
            assert len({result["resource_id"] for result in results}) == len(results)
            assert len({result["device_id"] for result in results}) == len(results)
        (work / "result.json").write_text(json.dumps({"boundary": "installed QEMU recipient-service bootstrap",
            "independent_installs": args.nodes, "nodes": results, "full_ms10_acceptance": False}, indent=2))
        return
    with ThreadPoolExecutor(max_workers=2) as workers:
        # Do not launch another batch after a failed install or an identity
        # collision. Preserve each completed node's receipt for focused reuse.
        for start in range(0, len(guests), 2):
            batch = guests[start:start+2]
            results.extend(workers.map(provision, batch, [args.resume] * len(batch), [args.pool] * len(batch)))
            assert len({result["node_id"] for result in results}) == len(results)
            assert len({result["resource_id"] for result in results}) == len(results)
            assert len({result["device_id"] for result in results}) == len(results)
    (work / "result.json").write_text(json.dumps({"boundary": "installed QEMU recipient-service bootstrap",
        "independent_installs": args.nodes, "nodes": results, "full_ms10_acceptance": False}, indent=2))

if __name__ == "__main__":
    main()
