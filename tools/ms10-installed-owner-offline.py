"""Installed owner-loss/automatic-replacement gate; not full MS10 acceptance."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
import pathlib
import struct
import hashlib
from ms10_ethernet_hub import EthernetHub
import ms10_installed_fixture as fixture
from ms10_installed_closure_setup import establish_authority
from ms10_installed_metadata import invoke, read_path, read_path_ready
from ms10_installed_transfer_measurement import measure
import ms10_installed_owner_lifecycle as lifecycle
import ms10_installed_pool_parity as parity

SPEC = importlib.util.spec_from_file_location("distribution", pathlib.Path(__file__).with_name("ms10-installed-distribution.py"))
D = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(D)


# ------------------------=
# FUNC: validate_case
# DESC: Fences resume to the exact independently computed byte set and measured object, not merely a matching VM directory.
# ------------------=
def validate_case(prior, identities, length, seed):
    assert prior["stage"] == "measurement-complete-owner-loss-not-tested"
    assert prior["identities"] == identities and len(prior["persisted"]) == 3
    assert prior["length"] == length and prior["seed"] == seed
    assert prior["created"]["length"] == length
    assert prior["created"]["sha256"] == hashlib.sha256(fixture.expected_content(length, seed)).hexdigest()
    assert prior["measurement"]["object"] == prior["created"]["object_id"]
    assert prior["measurement"]["bytes"] == length


# ------------------------=
# FUNC: validate_mode
# DESC: Keeps exact-object resume separate from new-object configured reuse and rejects conflicting modes before touching guests.
# ------------------=
def validate_mode(resume, reuse, measurement_only):
    assert not (resume and reuse)
    assert not (resume and measurement_only)


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
    parser.add_argument("--resume-measured", action="store_true")
    parser.add_argument("--reuse-configured", action="store_true")
    parser.add_argument("--length", type=int, choices=(32768, 65536, 262144), default=32768)
    parser.add_argument("--seed", type=int, default=17)
    parser.add_argument("--lifecycle", action="store_true")
    parser.add_argument("--parity", action="store_true", help="Requires the final shared ObjectInspect runtime; do not use on older installed generations.")
    args = parser.parse_args()
    work = args.output.resolve()
    assert 0 <= args.seed <= 0xffffffff
    assert not (args.lifecycle and args.measurement_only)
    validate_mode(args.resume_measured, args.reuse_configured, args.measurement_only)
    provenance = json.loads((work / "result.json").read_text())
    assert provenance["independent_installs"] == 4
    identities = [entry["node_id"] for entry in provenance["nodes"]]
    assert len(set(identities)) == 4
    prior = None
    if args.resume_measured or args.reuse_configured:
        prior = json.loads((work / "owner-offline-gate-result.json").read_text())
        validate_case(prior, identities, args.length if args.resume_measured else prior["length"],
                      args.seed if args.resume_measured else prior["seed"])
        (work / f"owner-offline-measurement-{prior['length']}-{prior['seed']}.json").write_text(json.dumps(prior, indent=2))
    hub = EthernetHub().start()
    guests = []
    report = {"status": "INCOMPLETE", "full_ms10_acceptance": False,
              "boundary": "four installed media-detached QEMU nodes", "stage": "boot",
              "identities": identities, "resume_measured": args.resume_measured,
              "reuse_configured": args.reuse_configured,
              "length": args.length, "seed": args.seed,
              "remaining_gates": ["stale-owner-return", "ordinary-update-copy-delete",
                                  "cold-reboot-shared-state", "garbage-collection",
                                  "all-three-sizes", "UI-and-event-verification"]}
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
        if prior is None:
            with ThreadPoolExecutor(max_workers=4) as workers:
                list(workers.map(lambda guest: D.MESH.configure(guest, 4), guests))
        for guest in guests:
            guest.wait(lambda s: s[24] == 3 and s[22] == 0, "three discovered peers", timeout=120)
        if prior is None:
            report["stage"] = "pair-all-six"
            for left in range(4):
                for right in range(left + 1, 4):
                    D.pair(guests[left], guests[right])
            report["stage"] = "authority"
            report["authority"] = establish_authority(guests, D)
        else:
            report["stage"] = "validate-measured-trust"
            for guest in guests:
                state = guest.state()
                assert state[25] == 3 and state[29] == 3
                peers = {}
                for index in range(3):
                    row = struct.pack("<16Q", *state[128+index*16:144+index*16])
                    peers[row[:32].hex()] = row[85]
                assert peers == {peer: 3 for peer in identities if peer != D.identity(guest)}
            for left in range(4):
                for right in range(left + 1, 4):
                    D.open_session(guests[left], guests[right])
            for guest in guests:
                guest.wait(lambda s: s[496] == 3, "persisted authorized publishers", timeout=90)
            report["authority"] = prior["authority"]
        a, b, c, replacement = guests
        replacement.stop()
        a.wait(lambda s: s[496] == 2, "replacement offline", timeout=90)
        report["stage"] = "bounded-transfer-measurement"
        a.launch("command", 5)
        created = prior["created"] if args.resume_measured else fixture.create(a, D.API.symbol, length=args.length, seed=args.seed)
        if args.reuse_configured:
            assert created["object_id"] != prior["created"]["object_id"], {"fixture_reused_prior_object": created["object_id"]}
        report["created"] = created
        report["namespace_path"] = f"/Shared/MS10_{args.length}_{args.seed}_{created['object_id'][:8]}"
        report["measurement"] = prior["measurement"] if args.resume_measured else measure(
            a, D.API.symbol, created["object_id"], traffic=hub.traffic_snapshot,
            during_transfer=lambda: D.responsive_transfer(a, created["object_id"]))
        report["persisted"] = [D.persisted_hash(g, args.verifier.resolve(), identities[0], created)
                               for g in (a, b, c)]
        if args.parity:
            report["baseline_console_settings_parity"] = parity.verify(a, D, created["object_id"], "baseline-parity")
        if args.measurement_only:
            report["stage"] = "measurement-complete-owner-loss-not-tested"
            return
        report["stage"] = "share"
        path = report["namespace_path"]
        a.launch("command", 5)
        result = invoke(a, f"pool share obj:{created['object_id']} path={path} confirm=true",
                        lambda: fixture.read_state(a, D.API.symbol))
        report["share"] = {key: value for key, value in result.items() if key != "data"}
        expected = fixture.expected_content(64, args.seed)
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
        tail_offset = args.length - 64
        tail_bytes = fixture.expected_content(args.length, args.seed)[tail_offset:]
        tail = read_path(b, path, created["object_id"], tail_bytes,
                         lambda: fixture.read_state(b, D.API.symbol), offset=tail_offset)
        report["owner_offline_last_extent_read"] = {key: value for key, value in tail.items() if key != "data"}
        report["owner_offline_last_extent_read"].update({"offset": tail_offset, "length": 64,
                                                      "sha256": hashlib.sha256(tail["data"]).hexdigest()})
        degraded = D.object_state(b, created["object_id"],
                                  lambda r: r[5] == 3 and r[6] == 2 and r[7] == 1,
                                  "owner-offline-two-of-three-degraded", timeout=90)
        assert degraded[2] == created["version"]
        report["owner_offline_degraded_row"] = list(degraded)
        if args.parity:
            report["degraded_console_settings_parity"] = parity.verify(b, D, created["object_id"], "degraded-parity", shared=True)
        replacement.boot(False)
        replacement.authenticate()
        replacement.fast_commands = True
        D.open_session(b, replacement)
        D.open_session(c, replacement)
        report["stage"] = "automatic-replacement"
        row = D.object_state(b, created["object_id"], lambda r: r[5] == r[6] == 3,
                             "owner-offline-automatic-replacement", timeout=600)
        report["healed_row"] = list(row)
        if args.parity:
            report["healed_console_settings_parity"] = parity.verify(b, D, created["object_id"], "healed-parity", shared=True)
        report["replacement_persisted"] = D.persisted_hash(replacement, args.verifier.resolve(), identities[0], created)
        b.launch("command", 5)
        read_path(b, path, created["object_id"], expected, lambda: fixture.read_state(b, D.API.symbol))
        report["stage"] = "owner-loss-gate-passed-other-closure-gates-pending"
        report["owner_loss_gate"] = "TESTED"
        if args.lifecycle:
            report["stage"] = "owner-return-mutation-continuation"
            report["lifecycle"] = lifecycle.run(guests, D, args.verifier.resolve(), report)
            report["remaining_gates"] = [gate for gate in report["remaining_gates"]
                                         if gate not in ("stale-owner-return", "ordinary-update-copy-delete")]
            report["stage"] = "owner-lifecycle-gate-passed-other-closure-gates-pending"
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
        (work / f"owner-offline-gate-{args.length}-{args.seed}.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
