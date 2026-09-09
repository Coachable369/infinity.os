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

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("installed_acceptance", ROOT / "tools/ms9-installed-acceptance.py")
installed = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installed)

# ------------------------=
# FUNC: provision
# DESC: Installs one blank owned disk, detaches media, configures through real UI and proves the mounted replica service survives cold authenticated boot.
# ------------------=
def provision(guest):
    try:
        guest.install()
        state = guest.wait(lambda state: state[108] == 3, "mounted native replica service")
        assert state[3] == 1
        guest.stop()
        result = guest.onboard()
        state = guest.wait(lambda state: state[108] == 3 and state[4] == 5, "cold installed recipient service")
        result["replica_service_ready_after_cold_boot"] = True
        result["installer_detached"] = not guest.installer
        assert result["installer_detached"]
        guest.screenshot("recipient-service-desktop")
        guest.frame_report("recipient-service-desktop")
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
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    args = parser.parse_args()
    work = args.output.resolve(); work.mkdir(parents=True, exist_ok=False)
    artifacts = work / "artifacts"; artifacts.mkdir()
    hashes = {}
    for source, name in [(ROOT / "builds/InfinityOS-x86_64.iso", "installer.iso"),
                         (ROOT / "build/x86_64/kernel.elf", "kernel.elf"),
                         (ROOT / "build/x86_64/installed-kernel.elf", "installed-kernel.elf")]:
        target = artifacts / name; shutil.copyfile(source, target)
        with target.open("rb") as stream:
            hashes[name] = hashlib.file_digest(stream, "sha256").hexdigest()
    (artifacts / "sha256.json").write_text(json.dumps(hashes, indent=2))
    guests = [installed.Guest(work, n, args.firmware) for n in range(1, args.nodes+1)]
    with ThreadPoolExecutor(max_workers=2) as workers:
        results = list(workers.map(provision, guests))
    assert len({result["node_id"] for result in results}) == args.nodes
    (work / "result.json").write_text(json.dumps({"boundary": "installed QEMU recipient-service bootstrap",
        "independent_installs": args.nodes, "nodes": results, "full_ms10_acceptance": False}, indent=2))

if __name__ == "__main__":
    main()
