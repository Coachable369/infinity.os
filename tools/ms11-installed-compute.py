"""Three-node detached-media Milestone 11 behavioral acceptance.

The fixture forwards Ethernet frames only. Guest commands are operator input;
all acceptance assertions read structured binary state, never rendered prose.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib.util
import json
import pathlib
import shutil
import struct
import time

from ms10_ethernet_hub import EthernetHub

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "installed", pathlib.Path(__file__).with_name("ms9-installed-acceptance.py"))
INSTALLED = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INSTALLED)


# ------------------------=
# FUNC: node_id
# DESC: Decodes the public node identity from the structured installed-runtime snapshot.
# ------------------=
def node_id(state):
    return struct.pack("<4Q", *state[16:20]).hex()


# ------------------------=
# FUNC: projected_peer
# DESC: Returns the deterministic Settings projection row for one complete peer identity.
# ------------------=
def projected_peer(state, peer):
    for index in range(min(state[24], 16)):
        at = 128 + index * 16
        if struct.pack("<4Q", *state[at:at + 4]).hex() == peer:
            return index, struct.pack("<16Q", *state[at:at + 16])
    raise AssertionError({"missing_peer": peer, "discovered": state[24]})


# ------------------------=
# FUNC: task
# DESC: Decodes one authoritative task slot from the fixed compute diagnostic ABI.
# ------------------=
def task(values, index):
    at = 16 + index * 26
    return {
        "id": values[at], "epoch": values[at + 1], "state": values[at + 2],
        "error": values[at + 3], "node": struct.pack("<4Q", *values[at + 4:at + 8]).hex(),
        "resource": struct.pack("<2Q", *values[at + 8:at + 10]).hex(),
        "correlation": values[at + 10], "kind": values[at + 11],
        "locality": values[at + 12], "durability": values[at + 13],
        "private": values[at + 14], "deadline": values[at + 15],
        "context": values[at + 16], "cpu_reserved": values[at + 17],
        "memory_reserved": values[at + 18], "cpu_ticks": values[at + 19],
        "started": values[at + 20], "finished": values[at + 21],
        "restarts": values[at + 22], "digest": values[at + 23:at + 25],
    }


# ------------------------=
# FUNC: tasks
# DESC: Returns every published authoritative task without inferring state from Console output.
# ------------------=
def tasks(values):
    return [task(values, index) for index in range(min(values[3], 6))]


# ------------------------=
# FUNC: task_by_id
# DESC: Finds an exact task identity in the bounded authoritative snapshot.
# ------------------=
def task_by_id(values, identity):
    return next((entry for entry in tasks(values) if entry["id"] == identity), None)


# ------------------------=
# FUNC: latest_task
# DESC: Selects the newest nonzero task identity after a submitted operator request.
# ------------------=
def latest_task(values):
    result = max(tasks(values), key=lambda entry: entry["id"], default=None)
    assert result is not None and result["id"] != 0
    return result


# ------------------------=
# FUNC: capability_for_target
# DESC: Reads an exact active operator capability from the fixed compute diagnostic ABI.
# ------------------=
def capability_for_target(values, target):
    matches = []
    for index in range(4):
        at = 226 + index * 7
        if values[at] and values[at + 2] == target and values[at + 5] == 0:
            matches.append(values[at])
    return max(matches, default=0)


# ------------------------=
# FUNC: wait_task
# DESC: Waits for an exact authoritative task predicate on one installed coordinator.
# ------------------=
def wait_task(guest, identity, predicate, label, timeout=180):
    state = guest.wait_compute(
        lambda values: (current := task_by_id(values, identity)) is not None and predicate(current, values),
        label, timeout)
    return task_by_id(state, identity), state


# ------------------------=
# FUNC: select_peer
# DESC: Selects one exact discovered peer through the native Settings UI.
# ------------------=
def select_peer(guest, peer):
    state = guest.state()
    projected_peer(state, peer)
    guest.launch("nodes", 8, 7)
    for _ in range(state[24] + 1):
        current = guest.state()
        if struct.pack("<4Q", *current[32:36]).hex() == peer:
            return current
        guest.key("ret")
    raise AssertionError({"unable_to_select_peer": peer, "state": guest.state()})


# ------------------------=
# FUNC: pair
# DESC: Performs mutual visual verification, dual trusted confirmation, and native session establishment for one pair.
# ------------------=
def pair(a, b):
    aid, bid = node_id(a.state()), node_id(b.state())
    _, a_projection = projected_peer(a.state(), bid)
    _, b_projection = projected_peer(b.state(), aid)
    if a_projection[85] == 3 and b_projection[85] == 3:
        a.launch("command", 5)
        before_a, before_b = a.state()[26], b.state()[26]
        a.command(f"node session-open node:{bid}")
        a.wait(lambda state: state[26] > before_a, "initiator secure session")
        b.wait(lambda state: state[26] > before_b, "peer secure session")
        return
    for guest, peer in [(a, bid), (b, aid)]:
        _, projection = projected_peer(guest.state(), peer)
        if projection[85] not in (0, 1):
            guest.launch("command", 5)
            before = guest.state()[20]
            guest.command(f"node trust-update node:{peer} name=state value=untrusted")
            guest.wait(lambda state: state[20] > before and projected_peer(state, peer)[1][85] == 1,
                       "peer returned to explicit untrusted state")
    select_peer(a, bid)
    select_peer(b, aid)
    a.key("right")
    b.key("right")
    a.key("down")
    a.key("ret")
    av = a.wait(lambda state: state[36] != 0, "local authenticated pairing transcript")
    bv = b.wait(lambda state: state[36] != 0, "peer authenticated pairing transcript")
    assert av[37] == bv[37] and av[40:48] == bv[40:48]
    with ThreadPoolExecutor(max_workers=2) as workers:
        list(workers.map(lambda item: item[0].confirm_peer(item[1], item[2]),
                         [(a, av, 2), (b, bv, 3)]))
    a.wait(lambda state: state[25] >= 1, "pair trusted locally")
    b.wait(lambda state: state[25] >= 1, "pair trusted remotely")
    a.key("esc")
    a.launch("command", 5)
    before_a, before_b = a.state()[26], b.state()[26]
    a.command(f"node session-open node:{bid}")
    a.wait(lambda state: state[26] > before_a, "initiator secure session")
    b.wait(lambda state: state[26] > before_b, "peer secure session")


# ------------------------=
# FUNC: policy
# DESC: Commits and verifies one exact scoped peer policy category through structured projection state.
# ------------------=
def policy(guest, peer, name, scope):
    category = {"object": 0, "compute": 2}[name]
    before = guest.state()[20]
    guest.command(f"node policy-update node:{peer} name={name} value=allow scope={scope}")
    return guest.wait(
        lambda state: state[20] > before and not state[22]
        and (projection := projected_peer(state, peer))[1][86 + category] == 1
        and struct.unpack_from("<Q", projection[1], 98)[0] == scope,
        "scoped peer policy committed")


# ------------------------=
# FUNC: grant
# DESC: Issues one exact expiring remote-operation grant and returns its structured handle.
# ------------------=
def grant(guest, peer, operation, scope):
    before = guest.state()[87]
    guest.command(
        f"node capability-grant node:{peer} name={operation} scope={scope} seconds=3600 confirm=true")
    return guest.wait(lambda state: state[87] > before and state[88] == 0
                      and state[89] > state[10], "scoped compute grant committed")[87]


# ------------------------=
# FUNC: authorize_compute
# DESC: Obtains an explicit session-owned Compute.Use authority and verifies its exact target.
# ------------------=
def authorize_compute(guest, scope):
    before = guest.compute_state()[8]
    guest.command(f"compute authorize scope={scope} seconds=3600 confirm=true")
    state = guest.wait_compute(lambda values: values[8] > before and capability_for_target(values, scope) != 0,
                               "compute use authority committed")
    return capability_for_target(state, scope)


# ------------------------=
# FUNC: authorize_cancel
# DESC: Obtains an explicit task-scoped Compute.Cancel authority and verifies its exact target.
# ------------------=
def authorize_cancel(guest, identity):
    before = guest.compute_state()[9]
    guest.command(f"compute authorize-cancel task={identity} seconds=3600 confirm=true")
    state = guest.wait_compute(lambda values: values[9] > before and capability_for_target(values, identity) != 0,
                               "compute cancel authority committed")
    return capability_for_target(state, identity)


# ------------------------=
# FUNC: submit
# DESC: Submits one explicit remote compute contract and returns its authoritative task identity.
# ------------------=
def submit(a, capability, kind, durability, work, deadline, primary, primary_grants,
           fallback=None, fallback_grants=None):
    before = a.compute_state()[3]
    command = (f"compute submit scope=11 capability={capability} kind={kind} locality=remote "
               f"durability={durability} work={work} memory=65536 cpu=2 deadline={deadline} "
               f"node={primary} request-grant={primary_grants[0]} cancel-grant={primary_grants[1]}")
    if fallback is not None:
        command += (f" fallback={fallback} fallback-request-grant={fallback_grants[0]} "
                    f"fallback-cancel-grant={fallback_grants[1]}")
    a.command(command)
    state = a.wait_compute(lambda values: values[3] > before, "distributed task admitted")
    return latest_task(state)["id"]


# ------------------------=
# FUNC: configure_network
# DESC: Configures all pair-specific native endpoints for one independently installed node.
# ------------------=
def configure_network(guest, count):
    if guest.state()[29] >= count - 1:
        return
    guest.launch("network", 8, 6)
    guest.key("right")
    guest.key("right")
    guest.key("down")
    guest.key("ret")
    guest.wait(lambda state: state[9] & 4, "static address editor")
    guest.text(f"10.42.0.{guest.number}")
    guest.key("ret")
    guest.wait(lambda state: not state[9] & 12, "static address committed")
    guest.key("esc")
    guest.launch("command", 5)
    slot = 0
    for peer in range(1, count + 1):
        if peer == guest.number:
            continue
        slot += 1
        before = guest.state()[20]
        guest.command(
            f"node link-configure {slot} local=10.42.0.{guest.number} remote=10.42.0.{peer} "
            f"local-port={49152 + peer} remote-port={49152 + guest.number}")
        guest.wait(lambda state: state[20] > before and state[29] == slot,
                   "native endpoint committed")


# ------------------------=
# FUNC: prepare
# DESC: Creates three independent fresh installs and proves each identity survives cold detached-media boot.
# ------------------=
def prepare(work, firmware):
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
    results = []
    for number in range(1, 4):
        guest = INSTALLED.Guest(work, number, firmware, memory_mb=4096)
        try:
            guest.install()
            guest.stop()
            results.append(guest.onboard())
        finally:
            guest.stop()
    assert len({result["node_id"] for result in results}) == 3
    (work / "result.json").write_text(json.dumps({
        "independent_installs": 3, "iso_detached": True, "nodes": results,
    }, indent=2))


# ------------------------=
# FUNC: installed_acceptance
# DESC: Executes the mandatory distributed CPU, authority, cancellation, deadline, recovery, fencing, and persistence transitions.
# ------------------=
def installed_acceptance(work, firmware):
    receipt = json.loads((work / "result.json").read_text())
    assert receipt["independent_installs"] == 3 and receipt["iso_detached"] is True
    expected = [entry["node_id"] for entry in receipt["nodes"]]
    hub = EthernetHub(3, metadata_limit=8192).start()
    guests = []
    evidence = {"boundary": "three independently installed detached-media QEMU guests"}
    try:
        for number in range(1, 4):
            guest = INSTALLED.Guest(work, number, firmware, reuse=True, memory_mb=1024)
            guest.mesh_port, guest.mesh_connect = hub.port, True
            guest.boot(False)
            guests.append(guest)
        with ThreadPoolExecutor(max_workers=3) as workers:
            authenticated = list(workers.map(lambda guest: guest.authenticate(), guests))
            for guest in guests:
                guest.fast_commands = True
            list(workers.map(lambda guest: configure_network(guest, 3), guests))
        assert [node_id(state) for state in authenticated] == expected
        for guest in guests:
            guest.wait(lambda state: state[24] == 2 and state[22] == 0,
                       "all installed peers discovered", timeout=180)
        a, b, c = guests
        aid, bid, cid = expected
        pair(a, b)
        pair(a, c)
        assert a.state()[25] == 2 and b.state()[25] >= 1 and c.state()[25] >= 1

        # A accepts B/C resource observations at scope zero before those peers
        # independently authorize A's scope-11 compute requests.
        a.launch("command", 5)
        policy(a, bid, "object", 0)
        advertise_b = grant(a, bid, "resource-advertise", 0)
        policy(a, cid, "object", 0)
        advertise_c = grant(a, cid, "resource-advertise", 0)
        for guest, peer, peer_grant in [(b, aid, advertise_b), (c, aid, advertise_c)]:
            guest.launch("command", 5)
            guest.command(f"pool advertise peer=node:{peer} grant={peer_grant}")
            publication = guest.wait(
                lambda values: values[494] > 0 or values[495] != 0,
                "authenticated resource publication result",
                timeout=90,
            )
            assert publication[495] == 0, {
                "stage": "authenticated resource publication result",
                "node": guest.number,
                "completed": publication[494],
                "error": publication[495],
            }
        a.wait_compute(lambda values: values[10] >= 3 and values[11] >= 3,
                       "remote compute and memory advertisements", timeout=180)

        grants = {}
        for guest, peer, label in [(b, aid, "b"), (c, aid, "c")]:
            policy(guest, peer, "compute", 11)
            grants[label] = (grant(guest, peer, "compute-request", 11),
                             grant(guest, peer, "compute-cancel", 11))

        a.launch("command", 5)
        compute_capability = authorize_compute(a, 11)
        first = submit(a, compute_capability, "checksum", "restartable", 4096, 600,
                       bid, grants["b"])
        wait_task(a, first, lambda current, _: current["node"] == bid and current["state"] in (2, 3),
                  "work placed on node B")
        completed, completed_state = wait_task(
            a, first, lambda current, _: current["state"] == 4 and any(current["digest"]),
            "typed remote result completed", timeout=300)
        assert completed["node"] == bid and completed["cpu_ticks"] == 4096
        assert completed_state[12] == 0 and completed_state[13] == 0
        evidence["remote_cpu"] = completed

        before_tasks = a.compute_state()[3]
        a.command(f"compute revoke {compute_capability}")
        a.command(f"compute submit scope=11 capability={compute_capability} kind=checksum locality=remote "
                  f"durability=restartable work=64 memory=65536 cpu=2 deadline=60 node={bid} "
                  f"request-grant={grants['b'][0]} cancel-grant={grants['b'][1]}")
        denied = a.wait_compute(lambda values: values[3] == before_tasks, "revoked compute denied")
        assert denied[12] == 0 and denied[13] == 0 and b.compute_state()[5] == 0
        evidence["revoked_authority_denied"] = True

        compute_capability = authorize_compute(a, 11)
        cancel_task = submit(a, compute_capability, "counter", "restartable", 1_000_000, 600,
                             bid, grants["b"])
        b.wait_compute(lambda values: values[5] > 0, "cancellable remote context active")
        cancel_capability = authorize_cancel(a, cancel_task)
        a.command(f"compute cancel {cancel_task} {cancel_capability}")
        cancelled, cancel_state = wait_task(
            a, cancel_task, lambda current, values: current["state"] == 6 and values[12] == 0 and values[13] == 0,
            "authoritative cancellation and reservation release")
        b.wait_compute(lambda values: values[5] == 0, "remote cancellation cleanup")
        evidence["cancelled"] = cancelled

        deadline_task = submit(a, compute_capability, "counter", "restartable", 1_000_000, 1,
                               bid, grants["b"])
        deadline, _ = wait_task(
            a, deadline_task, lambda current, values: current["state"] == 5 and current["error"] == 4
            and values[12] == 0 and values[13] == 0,
            "deadline failure and cleanup", timeout=180)
        evidence["deadline"] = deadline

        recovery_task = submit(a, compute_capability, "counter", "restartable", 32_768, 900,
                               bid, grants["b"], cid, grants["c"])
        wait_task(a, recovery_task, lambda current, _: current["node"] == bid and current["state"] == 3,
                  "restartable execution active on B")
        b.wait_compute(lambda values: values[5] > 0, "restartable remote context on B")
        b.stop()
        recovered, _ = wait_task(
            a, recovery_task, lambda current, _: current["epoch"] == 2 and current["node"] == cid
            and current["state"] in (2, 3, 4) and current["restarts"] == 1,
            "node loss re-placed on C", timeout=240)
        c.wait_compute(lambda values: values[5] > 0 or task_by_id(a.compute_state(), recovery_task)["state"] == 4,
                       "node C executes recovered task", timeout=180)
        recovered, _ = wait_task(a, recovery_task, lambda current, _: current["state"] == 4,
                                 "recovered task completed on C", timeout=240)
        assert recovered["epoch"] == 2 and recovered["node"] == cid and recovered["restarts"] == 1
        evidence["restartable_recovery"] = recovered

        b = INSTALLED.Guest(work, 2, firmware, reuse=True, memory_mb=1024)
        b.mesh_port, b.mesh_connect = hub.port, True
        b.boot(False)
        guests[1] = b
        b.authenticate()
        a.wait(lambda state: state[24] == 2, "returned B rediscovered", timeout=180)
        stable, _ = wait_task(a, recovery_task,
                              lambda current, _: current["epoch"] == 2 and current["node"] == cid
                              and current["state"] == 4,
                              "returned B cannot overwrite fenced result")
        assert stable["digest"] == recovered["digest"]
        evidence["stale_result_fenced_after_return"] = True

        a.screenshot("compute-console-authoritative")
        a.key("esc")
        a.launch("task", 5)
        a.screenshot("compute-task-manager-authoritative")
        gui_state = a.compute_state()
        assert task_by_id(gui_state, first) == task_by_id(completed_state, first)
        evidence["gui_console_shared_state"] = True

        persisted = {entry["id"]: (entry["epoch"], entry["state"], entry["node"], entry["digest"])
                     for entry in tasks(a.compute_state())}
        a.stop()
        a = INSTALLED.Guest(work, 1, firmware, reuse=True, memory_mb=1024)
        a.mesh_port, a.mesh_connect = hub.port, True
        a.boot(False)
        guests[0] = a
        a.authenticate()
        restored = {entry["id"]: (entry["epoch"], entry["state"], entry["node"], entry["digest"])
                    for entry in tasks(a.compute_state())}
        assert all(restored.get(identity) == value for identity, value in persisted.items())
        evidence["installed_reboot_persistence"] = True
        evidence["ethernet"] = hub.traffic_snapshot()
        evidence["iso_attached_during_runtime"] = False
        (work / "ms11-result.json").write_text(json.dumps(evidence, indent=2))
    finally:
        for guest in guests:
            guest.stop()
        hub.close()


# ------------------------=
# FUNC: main
# DESC: Runs fresh installation and/or detached-media three-node compute acceptance in explicit phases.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=pathlib.Path)
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--run-only", action="store_true")
    args = parser.parse_args()
    assert not (args.prepare_only and args.run_only)
    work = args.output.resolve()
    if not args.run_only:
        prepare(work, args.firmware)
    if not args.prepare_only:
        installed_acceptance(work, args.firmware)


if __name__ == "__main__":
    main()
