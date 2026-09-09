"""Installed owner-loss/automatic-replacement gate; not full MS10 acceptance."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
import pathlib
from ms10_ethernet_hub import EthernetHub
import ms10_installed_fixture as fixture
from ms10_installed_closure_setup import establish_authority
from ms10_installed_metadata import invoke, read_path, read_path_ready
from ms10_installed_transfer_measurement import measure

SPEC = importlib.util.spec_from_file_location("distribution", pathlib.Path(__file__).with_name("ms10-installed-distribution.py"))
D = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(D)


# ------------------------=
# FUNC: main
# DESC: Uses independently installed nodes and ordinary native operations; loses original A and never requests explicit repair.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    parser.add_argument("--verifier", type=pathlib.Path, required=True)
    parser.add_argument("--measurement-only", action="store_true")
    args = parser.parse_args()
    work = args.output.resolve()
    provenance = json.loads((work / "result.json").read_text())
    assert provenance["independent_installs"] == 4
    identities = [entry["node_id"] for entry in provenance["nodes"]]
    assert len(set(identities)) == 4
    hub = EthernetHub().start()
    guests = []
    report = {"status": "INCOMPLETE", "full_ms10_acceptance": False,
              "boundary": "four installed media-detached QEMU nodes", "stage": "boot"}
    try:
        for number in range(1, 5):
            guest = D.API.Guest(work, number, args.firmware, reuse=True)
            guests.append(guest)
            guest.mesh_port, guest.mesh_connect = hub.port, True
            guest.boot(False)
        with ThreadPoolExecutor(max_workers=4) as workers:
            list(workers.map(lambda guest: guest.authenticate(), guests))
        assert [D.identity(guest) for guest in guests] == identities
        for guest in guests:
            guest.launch("command", 5)
            guest.fast_input_probe()
            guest.key("esc")
            guest.fast_commands = True
        with ThreadPoolExecutor(max_workers=4) as workers:
            list(workers.map(lambda guest: D.MESH.configure(guest, 4), guests))
        for guest in guests:
            guest.wait(lambda s: s[24] == 3 and s[22] == 0, "three discovered peers", timeout=120)
        report["stage"] = "pair-all-six"
        for left in range(4):
            for right in range(left + 1, 4):
                D.pair(guests[left], guests[right])
        report["stage"] = "authority"
        report["authority"] = establish_authority(guests, D)
        a, b, c, replacement = guests
        replacement.stop()
        a.wait(lambda s: s[496] == 2, "replacement offline", timeout=90)
        report["stage"] = "32KiB-measurement"
        a.launch("command", 5)
        created = fixture.create(a, D.API.symbol, length=32768, seed=17)
        report["created"] = created
        report["measurement"] = measure(a, D.API.symbol, created["object_id"])
        report["persisted"] = [D.persisted_hash(g, args.verifier.resolve(), identities[0], created)
                               for g in (a, b, c)]
        if args.measurement_only:
            report["stage"] = "measurement-complete-owner-loss-not-tested"
            return
        report["stage"] = "share"
        path = "/Shared/MS10Acceptance"
        a.launch("command", 5)
        result = invoke(a, f"pool share obj:{created['object_id']} path={path} confirm=true",
                        lambda: fixture.read_state(a, D.API.symbol))
        report["share"] = {key: value for key, value in result.items() if key != "data"}
        expected = fixture.expected_content(64, 17)
        b.launch("command", 5)
        ready = read_path_ready(b, path, created["object_id"], expected, lambda: fixture.read_state(b, D.API.symbol))
        report["namespace_readiness"] = ready["readiness_attempts"]
        a.stop()
        report["stage"] = "original-owner-offline-normal-read"
        b.wait(lambda s: s[496] == 1, "original owner resource offline", timeout=90)
        b.launch("command", 5)
        result = read_path(b, path, created["object_id"], expected,
                           lambda: fixture.read_state(b, D.API.symbol))
        report["owner_offline_read"] = {key: value for key, value in result.items() if key != "data"}
        replacement.boot(False)
        replacement.authenticate()
        replacement.fast_commands = True
        D.open_session(b, replacement)
        D.open_session(c, replacement)
        report["stage"] = "automatic-replacement"
        row = D.object_state(b, created["object_id"], lambda r: r[5] == r[6] == 3,
                             "owner-offline-automatic-replacement", timeout=600)
        report["healed_row"] = list(row)
        report["replacement_persisted"] = D.persisted_hash(replacement, args.verifier.resolve(), identities[0], created)
        b.launch("command", 5)
        read_path(b, path, created["object_id"], expected, lambda: fixture.read_state(b, D.API.symbol))
        report["stage"] = "owner-loss-gate-passed-other-closure-gates-pending"
        report["owner_loss_gate"] = "TESTED"
    except BaseException as error:
        report["failure"] = repr(error)
        raise
    finally:
        for guest in guests:
            try:
                guest.stop()
            except Exception as error:
                report.setdefault("stop_errors", []).append(repr(error))
        hub.close()
        (work / "owner-offline-gate-result.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
