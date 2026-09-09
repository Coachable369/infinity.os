"""Final-source four-node native Pool distribution acceptance.

All setup is real operator input; the host forwards Ethernet and observes state.
"""
import argparse
import hashlib
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
import pathlib
import struct
import subprocess
import time
from ms10_ethernet_hub import EthernetHub
import ms10_installed_fixture as fixture
from ms10_installed_pool import call

SPEC = importlib.util.spec_from_file_location("mesh_discovery", pathlib.Path(__file__).with_name("ms10-installed-mesh-discovery.py"))
MESH = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MESH)
API = MESH.INSTALLED


# ------------------------=
# FUNC: identity
# DESC: Reads one installed persistent NodeId from the native diagnostic projection.
# ------------------=
def identity(guest):
    return struct.pack("<4Q", *guest.state()[16:20]).hex()


# ------------------------=
# FUNC: select
# DESC: Cycles the native peer selector to an exact discovered identity, never assumes list order.
# ------------------=
def select(guest, peer):
    guest.launch("nodes", 8, 7)
    for _ in range(5):
        guest.key("ret")
        state = guest.state()
        if struct.pack("<4Q", *state[32:36]).hex() == peer:
            return state
    raise AssertionError({"peer_not_selectable": peer})


# ------------------------=
# FUNC: pair
# DESC: Requires matching authenticated fingerprints and two protected confirmations; sessions are opened only after grant preparation.
# ------------------=
def pair(a, b):
    aid, bid = identity(a), identity(b)
    old_a, old_b = a.state()[25], b.state()[25]
    for guest, peer in ((a, bid), (b, aid)):
        select(guest, peer)
        guest.key("right")
    a.key("down")
    a.key("ret")
    av = a.wait(lambda s: s[36] != 0, "pairing transcript")
    bv = b.wait(lambda s: s[36] != 0, "peer pairing transcript")
    assert av[37] == bv[37] and av[40:48] == bv[40:48]
    assert av[32:36] == bv[16:20] and bv[32:36] == av[16:20]
    with ThreadPoolExecutor(max_workers=2) as workers:
        list(workers.map(lambda item: item[0].confirm_peer(item[1], item[2]), ((a, av, 2), (b, bv, 3))))
    a.wait(lambda s: s[25] == old_a + 1, "new trusted peer")
    b.wait(lambda s: s[25] == old_b + 1, "new trusted peer")


# ------------------------=
# FUNC: open_session
# DESC: Verifies the exact two-sided peer transaction, accepting an already established automatic session without requiring a count increase.
# ------------------=
def open_session(a, b):
    aid, bid = identity(a), identity(b)
    av, bv = select(a, bid), select(b, aid)
    if not session_ready(av, bid):
        a.launch("command", 5)
        a.command(f"node session-open node:{bid}")
    av = a.wait(lambda s: session_ready(s, bid), "requested peer secure session")
    bv = b.wait(lambda s: session_ready(s, aid), "reciprocal peer secure session")
    assert av[64:68] == bv[64:68], {"session_transaction_mismatch": (aid, bid)}


# ------------------------=
# FUNC: session_ready
# DESC: Requires a live established wire transaction for the selected exact peer, not an aggregate session count.
# ------------------=
def session_ready(state, peer):
    return (struct.pack("<4Q", *state[32:36]).hex() == peer
            and state[56] == 11 and state[58] > state[10]
            and any(state[64:68]) and state[26] > 0)


# ------------------------=
# FUNC: participate
# DESC: Grants exact recipient operations and publication direction through native peer policy and bounded leased authority.
# ------------------=
def participate(a, b, grants, publication):
    aid, bid = identity(a), identity(b)
    retirement = grants[5]
    a.launch("command", 5)
    a.command(f"pool participate peer=node:{bid} begin={grants[0]} chunk={grants[1]} commit={grants[2]} inspect={grants[3]} read={grants[4]} lease=3600 confirm=true")
    a.command(f"pool retire-authority peer=node:{bid} grant={retirement} lease=3600 confirm=true")
    b.command(f"pool advertise peer=node:{aid} grant={publication}")


# ------------------------=
# FUNC: recipient_grants
# DESC: Prepares one independent recipient's exact leased operation grants through its authenticated Console.
# ------------------=
def recipient_grants(b, aid):
    b.launch("command", 5)
    b.peer_policy(aid, "object", "allow")
    b.peer_policy(aid, "namespace", "allow")
    return [b.peer_grant(aid, operation) for operation in
            ("transfer-begin", "transfer-chunk", "transfer-commit", "replica-inspect", "object-read", "replica-delete")]


# ------------------------=
# FUNC: publication_grants
# DESC: Issues publication permissions sequentially on the one authority while recipients independently prepare their grants.
# ------------------=
def publication_grants(a, peers):
    return [a.peer_grant(identity(peer), "resource-advertise") for peer in peers]


# ------------------------=
# FUNC: object_state
# DESC: Waits for authoritative visible Storage projection conditions on the exact ObjectId.
# ------------------=
def object_state(guest, object_id, condition, label, timeout=180):
    guest.launch("storage", 8, 8)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = fixture.read_state(guest, API.symbol)
        assert not state[25], {"storage_projection_failed": state[:26]}
        if state[24]:
            for index in range(min(state[18], 8)):
                row = state[32 + index * 16:48 + index * 16]
                if struct.pack("<2Q", *row[:2]).hex() == object_id and condition(row):
                    guest.screenshot(label)
                    return row
        time.sleep(.1)
    raise AssertionError({"pool_transition_timeout": label, "last_state": state})


# ------------------------=
# FUNC: local_read
# DESC: Reads normal ObjectId bytes through the shared Pool service and verifies binary response content.
# ------------------=
def local_read(guest, object_id, row, expected):
    guest.launch("command", 5)
    state = guest.command(f"pool read obj:{object_id} generation={row[3]} version={row[2]} offset=0 length={len(expected)}")
    assert state[110] == 1
    reply = struct.pack("<17Q", *state[111:128])
    assert struct.unpack_from("<HHI", reply) == (1, len(expected), 0x3002)
    assert reply[72:72 + len(expected)] == expected
    assert reply[8:24].hex() == object_id
    assert struct.unpack_from("<QQ", reply, 32) == (row[3], row[2])


# ------------------------=
# FUNC: remote_read
# DESC: Requests a verified remote source by ObjectId without naming any physical host, then collects the same bounded native request.
# ------------------=
def remote_read(guest, object_id, row, expected):
    guest.launch("command", 5)
    before = fixture.read_state(guest, API.symbol)[16]
    guest.command(f"pool read obj:{object_id} generation={row[3]} version={row[2]} offset=0 length={len(expected)} source=remote")
    request = fixture.read_state(guest, API.symbol)[16]
    assert request != before and request & (1 << 63)
    admitted = guest.state()
    assert admitted[110] == 0, {"stale_read_observation_at_admission": request}
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        state = guest.command(f"pool result request={request}")
        completion = fixture.read_state(guest, API.symbol)
        if completion[30] == request:
            assert completion[251] == 0, {"remote_read_failed": request, "error": completion[251]}
            assert state[110] == 1, {"missing_consumed_read_payload": request}
            reply = struct.pack("<17Q", *state[111:128])
            assert struct.unpack_from("<HHI", reply) == (1, len(expected), 0x3002)
            assert reply[8:24].hex() == object_id and reply[72:72+len(expected)] == expected
            assert struct.unpack_from("<QQ", reply, 32) == (row[3], row[2])
            return
    raise AssertionError({"remote_read_timeout": request})


# ------------------------=
# FUNC: persisted_hash
# DESC: Pauses an already committed guest and invokes a strictly read-only native extent verifier; cold boot remains a separate gate.
# ------------------=
def persisted_hash(guest, verifier, owner, created):
    guest.qmp("stop")
    try:
        status = guest.qmp("query-status")
        assert status["status"] == "paused" and not status["running"]
        result = subprocess.run([str(verifier), str(guest.disk), owner, created["object_id"],
                                 created["sha256"], str(created["version"])],
                                check=True, capture_output=True, text=True, timeout=60)
        report = json.loads(result.stdout)
        (guest.work / "persisted-content-result.json").write_text(json.dumps(report, indent=2))
    finally:
        guest.qmp("cont")
    return report


# ------------------------=
# FUNC: responsive_transfer
# DESC: Moves a real Settings window while the actual coordinator owns a transfer, retaining input-to-state timing instead of inventing frame measurements.
# ------------------=
def responsive_transfer(guest, object_id):
    state = guest.state()
    assert state[4] == 8 and state[8] == 8, {"loaded_probe_requires_open_storage": state[4:9]}
    deadline = time.monotonic() + 90
    while True:
        pool = fixture.read_state(guest, API.symbol)
        if pool[26] and struct.pack("<2Q", *pool[240:242]).hex() == object_id:
            break
        assert time.monotonic() < deadline, {"no_active_transfer_for_input_probe": pool[:32]}
        time.sleep(.1)
    initial_rect = pool[246:250]
    target = (initial_rect[0] + initial_rect[2] // 2, initial_rect[1] + 20)
    for _ in range(60):
        state = guest.state()
        dx, dy = target[0] - state[13], target[1] - state[14]
        if abs(dx) < 6 and abs(dy) < 6:
            break
        events = [{"type": "rel", "data": {"axis": axis, "value": max(-60, min(60, int(delta / 3) or (1 if delta > 0 else -1)))}}
                  for axis, delta in (("x", dx), ("y", dy)) if abs(delta) >= 6]
        guest.qmp("input-send-event", {"events": events})
        guest.wait(lambda s: s[13:15] != state[13:15], "pointer moved toward window", timeout=2)
    else:
        raise AssertionError("pointer did not reach native window title bar")
    assert fixture.read_state(guest, API.symbol)[26] != 0
    latencies = []
    guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "left", "down": True}}]})
    try:
        for _ in range(6):
            before = fixture.read_state(guest, API.symbol)
            assert before[26] and struct.pack("<2Q", *before[240:242]).hex() == object_id, {"transfer_ended_before_drag": object_id}
            started = time.monotonic_ns()
            guest.qmp("input-send-event", {"events": [{"type": "rel", "data": {"axis": "x", "value": 4}}]})
            while True:
                after = fixture.read_state(guest, API.symbol)
                if after[246:250] != before[246:250]:
                    break
                assert time.monotonic_ns() - started < 2_000_000_000, {"drag_did_not_move_window": after[246:251]}
                time.sleep(.02)
            assert after[250] == 1
            assert after[26] and struct.pack("<2Q", *after[240:242]).hex() == object_id, {"transfer_ended_during_drag": object_id}
            latencies.append(time.monotonic_ns() - started)
    finally:
        guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "left", "down": False}}]})
    guest.wait(lambda s: s[15] == 0, "pointer button released", timeout=2)
    assert fixture.read_state(guest, API.symbol)[250] == 0
    assert max(latencies) < 2_000_000_000
    guest.screenshot("drag-during-transfer")
    navigation = []
    for query, mode, section in (("network", 8, 6), ("command", 5, None),
                                 ("storage", 8, 8)):
        before = fixture.read_state(guest, API.symbol)
        assert before[26] and struct.pack("<2Q", *before[240:242]).hex() == object_id
        started = time.monotonic_ns()
        guest.launch(query, mode, section)
        if query == "command":
            guest.fast_input_probe()
        after = fixture.read_state(guest, API.symbol)
        assert after[26] and struct.pack("<2Q", *after[240:242]).hex() == object_id
        navigation.append({"native_surface": query, "elapsed_ns": time.monotonic_ns() - started,
                           "transfer_active_before_and_after": True})
        guest.screenshot(f"{query}-during-transfer")
    report = {"boundary": "installed real pointer input during native coordinator transfer",
              "input_to_observed_window_move_ns": latencies, "limit_ns": 2_000_000_000,
              "loaded_navigation": navigation, "rapid_keys_accepted": 10,
              "functional_liveness_only": True, "ui_performance_acceptance": False,
              "renderer_fps_claim": False, "frame_measurement": guest.frame_report("drag-during-transfer")}
    (guest.work / "transfer-input-result.json").write_text(json.dumps(report, indent=2))
    return report


# ------------------------=
# FUNC: main
# DESC: Runs fresh-generation installed identity, discovery, trust, distribution, loss and replacement gates without inventing remaining acceptance.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    parser.add_argument("--verifier", type=pathlib.Path, required=True)
    parser.add_argument("--stop-before-remote-read", action="store_true",
                        help="Collect distribution evidence only, then exit incomplete before a known unavailable read-consumption diagnostic.")
    args = parser.parse_args()
    work = args.output.resolve()
    receipt = json.loads((work / "result.json").read_text())
    assert receipt["independent_installs"] == 4
    known = [entry["node_id"] for entry in receipt["nodes"]]
    assert len(set(known)) == 4
    hub = EthernetHub().start()
    guests = []
    report = {"boundary": "four independently installed QEMU nodes", "full_ms10_acceptance": False}
    try:
        for number in range(1, 5):
            guest = API.Guest(work, number, args.firmware, reuse=True)
            guests.append(guest)
            guest.mesh_port, guest.mesh_connect = hub.port, True
            guest.boot(False)
        with ThreadPoolExecutor(max_workers=4) as workers:
            list(workers.map(lambda guest: guest.authenticate(), guests))
        assert [identity(guest) for guest in guests] == known
        for guest in guests:
            guest.launch("command", 5)
            guest.fast_input_probe()
            guest.key("esc")
            guest.fast_commands = True
        with ThreadPoolExecutor(max_workers=4) as workers:
            list(workers.map(lambda guest: MESH.configure(guest, 4), guests))
        for guest in guests:
            state = guest.wait(lambda s: s[24] == 3 and s[22] == 0, "three exact discovered peers", timeout=120)
            peers = {struct.pack("<4Q", *state[128+i*16:132+i*16]).hex() for i in range(3)}
            assert peers == set(known) - {known[guest.number-1]}
        a, b, c, d = guests
        for peer in (b, c, d):
            pair(a, peer)
        a.launch("command", 5)
        for peer in (b, c, d):
            a.peer_policy(identity(peer), "object", "allow")
            a.peer_policy(identity(peer), "namespace", "allow")
        with ThreadPoolExecutor(max_workers=4) as workers:
            recipients = [workers.submit(recipient_grants, peer, known[0]) for peer in (b, c, d)]
            publications = workers.submit(publication_grants, a, (b, c, d))
            grants = [future.result() for future in recipients]
            publication = publications.result()
        for index, peer in enumerate((b, c, d)):
            participate(a, peer, grants[index], publication[index])
        for peer in (b, c, d):
            open_session(a, peer)
        a.wait(lambda s: s[496] == 3, "three actual advertised resources", timeout=90)
        d.stop()
        a.wait(lambda s: s[496] == 2 and s[109] == 3, "replacement host offline but known", timeout=90)
        a.launch("command", 5)
        created = fixture.create(a, API.symbol)
        oid = created["object_id"]
        input_report = responsive_transfer(a, oid)
        healthy = object_state(a, oid, lambda r: r[5] == 3 and r[6] == 3, "critical-three-verified")
        assert healthy[4] == 32768
        persisted = [persisted_hash(guest, args.verifier.resolve(), known[0], created) for guest in (a, b, c)]
        local_read(a, oid, healthy, fixture.expected_content(64, 17))
        if args.stop_before_remote_read:
            report.update({"status": "INCOMPLETE", "created": created,
                           "critical_three_verified": True, "persisted_content": persisted,
                           "transfer_input": input_report, "local_read": True,
                           "stop_reason": "Installed generation lacks correlated remote-read consumption diagnostics",
                           "remote_read": "NOT TESTED", "loss_heal_reboot": "NOT TESTED"})
            (work / "distribution-preflight-result.json").write_text(json.dumps(report, indent=2))
            raise SystemExit(2)
        remote_read(a, oid, healthy, fixture.expected_content(64, 17))
        b.stop()
        degraded = object_state(a, oid, lambda r: r[6] == 2 and r[7] >= 1, "replica-host-loss")
        local_read(a, oid, degraded, fixture.expected_content(64, 17))
        d.boot(False)
        d.authenticate()
        a.launch("command", 5)
        a.command(f"node session-open node:{known[3]}")
        healed = object_state(a, oid, lambda r: r[6] == 3 and not r[10], "replacement-healed", timeout=240)
        local_read(a, oid, healed, fixture.expected_content(64, 17))
        persisted.append(persisted_hash(d, args.verifier.resolve(), known[0], created))
        a.launch("command", 5)
        updated = call(a, f"pool write obj:{oid} generation={healed[3]} version={healed[2]} content=CurrentVersion", 0x3003)
        assert updated["object"] == oid and updated["version"] == 2
        current = object_state(a, oid, lambda r: r[2] == 2 and r[6] >= 3 and r[7] + r[8] >= 1,
                               "old-offline-replica-retained", timeout=240)
        b.boot(False)
        b.authenticate()
        assert identity(b) == known[1]
        a.launch("command", 5)
        a.command(f"node session-open node:{known[1]}")
        reconciled = object_state(a, oid, lambda r: r[2] == 2 and r[6] >= 3 and r[7] == 0 and r[8] == 0,
                                  "stale-host-reconciled", timeout=240)
        local_read(a, oid, reconciled, b"CurrentVersion")
        remote_read(a, oid, reconciled, b"CurrentVersion")
        a.launch("command", 5)
        copied = call(a, f"pool copy obj:{oid} generation={reconciled[3]} version=2 nonce=919", 0x3008)
        copy_id = copied["data"][:16].hex()
        assert copy_id not in (oid, "00" * 16)
        copy_row = object_state(a, copy_id, lambda r: r[6] >= 3, "distributed-copy", timeout=240)
        source_info = dict(created, version=2, sha256=hashlib.sha256(b"CurrentVersion").hexdigest())
        source_physical = persisted_hash(a, args.verifier.resolve(), known[0], source_info)
        copy_info = dict(source_info, object_id=copy_id, version=1)
        copy_physical = persisted_hash(a, args.verifier.resolve(), known[0], copy_info)
        assert source_physical["chunks"] == copy_physical["chunks"]
        a.launch("command", 5)
        call(a, f"pool write obj:{copy_id} generation={copy_row[3]} version=1 content=IndependentCopy", 0x3003)
        source_row = object_state(a, oid, lambda r: r[2] == 2 and r[6] >= 3, "source-unchanged-after-copy-edit")
        local_read(a, oid, source_row, b"CurrentVersion")
        copy_row = object_state(a, copy_id, lambda r: r[2] == 2 and r[6] >= 3, "copy-divergence-protected", timeout=240)
        local_read(a, copy_id, copy_row, b"IndependentCopy")
        a.launch("command", 5)
        call(a, f"pool write obj:{oid} generation={source_row[3]} version=2 content=SourceThird", 0x3003)
        source_row = object_state(a, oid, lambda r: r[2] == 3 and r[6] >= 3, "source-divergence-protected", timeout=240)
        copy_row = object_state(a, copy_id, lambda r: r[2] == 2 and r[6] >= 3, "copy-unchanged-after-source-edit")
        local_read(a, copy_id, copy_row, b"IndependentCopy")
        a.launch("command", 5)
        deleted = call(a, f"pool delete obj:{copy_id} generation={copy_row[3]} version=2 confirm=true", 0x3009)
        assert deleted["object"] == copy_id
        source_row = object_state(a, oid, lambda r: r[2] == 3 and r[6] >= 3, "source-survives-copy-deletion")
        local_read(a, oid, source_row, b"SourceThird")
        for guest in guests:
            guest.stop()
        for guest in guests:
            guest.boot(False)
        with ThreadPoolExecutor(max_workers=4) as workers:
            list(workers.map(lambda guest: guest.authenticate(), guests))
        assert [identity(guest) for guest in guests] == known
        a.launch("command", 5)
        for peer in known[1:]:
            a.command(f"node session-open node:{peer}")
        recovered = object_state(a, oid, lambda r: r[2] == 3 and r[6] >= 3, "cold-reboot-protected-object", timeout=240)
        local_read(a, oid, recovered, b"SourceThird")
        remote_read(a, oid, recovered, b"SourceThird")
        report.update({"identities": known, "all_peer_discovery": True, "critical_object": created,
                       "three_verified": True, "node_loss_degraded_read": True,
                       "replacement_healed": True, "persisted_hashes": persisted,
                       "stale_return_reconciled": True, "cow_independent": True,
                       "copy_deleted_source_readable": True, "cold_reboot_read": True,
                       "transfer_input": input_report,
                       "unverified_gates": ["failure injection", "durable audit", "final-dependent reclamation", "healing and remote-read responsiveness"]})
        (work / "distribution-result.json").write_text(json.dumps(report, indent=2))
    except BaseException:
        for guest in guests:
            if guest.process is not None:
                try:
                    guest.screenshot("distribution-failure")
                    guest.frame_report("distribution-failure")
                except (OSError, AssertionError, ValueError):
                    pass  # Preserve the original failure when its guest already exited.
        raise
    finally:
        for guest in guests:
            guest.stop()
        hub.close()


if __name__ == "__main__":
    main()
