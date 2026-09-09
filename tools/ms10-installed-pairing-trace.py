"""One bounded installed A/B ceremony on four preserved independently installed disks."""
import argparse
import importlib.util
import json
import pathlib
import struct
import time
from concurrent.futures import ThreadPoolExecutor
from ms10_ethernet_hub import EthernetHub

SPEC = importlib.util.spec_from_file_location("distribution", pathlib.Path(__file__).with_name("ms10-installed-distribution.py"))
DISTRIBUTION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DISTRIBUTION)


# ------------------------=
# FUNC: peer_trust
# DESC: Reads the exact peer's typed trust state independently of rendered labels or row order.
# ------------------=
def peer_trust(state, peer):
    for row in range(min(state[24], 16)):
        data = struct.pack("<16Q", *state[128 + row * 16:144 + row * 16])
        if data[:32].hex() == peer:
            return data[85]
    return None


# ------------------------=
# FUNC: validate_pair
# DESC: Accepts exactly two distinct installed node indices without selecting additional authority targets.
# ------------------=
def validate_pair(pair):
    assert len(pair) == 2 and len(set(pair)) == 2 and all(1 <= value <= 4 for value in pair)
    return tuple(pair)


# ------------------------=
# FUNC: other_trust
# DESC: Captures all non-target peer trust values so focused resets cannot silently alter prior successful pairings.
# ------------------=
def other_trust(state, peer):
    result = {}
    for row in range(min(state[24], 16)):
        data = struct.pack("<16Q", *state[128+row*16:144+row*16])
        if data[:32].hex() != peer:
            result[data[:32].hex()] = data[85]
    return result


# ------------------------=
# FUNC: reset_peer
# DESC: Revokes existing authority and explicitly unblocks to untrusted through normal operator controls before a fresh ceremony.
# ------------------=
def reset_peer(guest, peer):
    selected = DISTRIBUTION.select(guest, peer)
    preserved = other_trust(selected, peer)
    guest.fast_commands = True
    guest.launch("command", 5)
    for command, expected in (("trust-revoke", 5), ("unblock", 1)):
        before = guest.state()[20]
        guest.command(f"node {command} node:{peer}")
        changed = guest.wait(lambda state: state[20] > before and not state[22]
                   and peer_trust(state, peer) == expected,
                   "explicit peer authority reset", timeout=30)
        assert other_trust(changed, peer) == preserved
        assert not DISTRIBUTION.session_ready(changed, peer), "Target session remains live after explicit revoke"
    refreshed = DISTRIBUTION.select(guest, peer)
    assert peer_trust(refreshed, peer) == 1 and not refreshed[22]


# ------------------------=
# FUNC: main
# DESC: Reproduces only discovery and protected pairing, retaining bounded public network metadata and native counters without installing, granting or storing Pool data.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--cycles", type=int, choices=range(1, 6), default=1)
    parser.add_argument("--reset-trust", action="store_true")
    parser.add_argument("--configure-network", action="store_true")
    parser.add_argument("--pair", type=int, nargs=2, default=(1, 2))
    args = parser.parse_args()
    pair = validate_pair(args.pair)
    work = args.output.resolve()
    receipt = json.loads((work / "result.json").read_text())
    assert receipt["independent_installs"] == 4
    hub = EthernetHub(metadata_limit=8192).start()
    guests = []
    report = {"boundary": "installed pairing trace only", "paired": False,
              "cycles": [], "pair": pair, "full_ms10_acceptance": False}
    try:
        for number in range(1, 5):
            guest = DISTRIBUTION.API.Guest(work, number, "/opt/homebrew/share/qemu/edk2-x86_64-code.fd", reuse=True)
            guests.append(guest)
            guest.mesh_port, guest.mesh_connect = hub.port, True
            guest.boot(False)
        with ThreadPoolExecutor(max_workers=4) as workers:
            list(workers.map(lambda guest: guest.authenticate(), guests))
        if args.configure_network:
            for guest in guests:
                guest.fast_commands = True
            with ThreadPoolExecutor(max_workers=4) as workers:
                list(workers.map(lambda guest: DISTRIBUTION.MESH.configure(guest, 4), guests))
        for guest, installed in zip(guests, receipt["nodes"]):
            assert DISTRIBUTION.identity(guest) == installed["node_id"]
            state = guest.wait(lambda state: state[24] == 3 and state[29] == 3, "preserved endpoints discover three peers", timeout=90)
        left, right = [guests[index-1] for index in pair]
        targets = ((left, DISTRIBUTION.identity(right)), (right, DISTRIBUTION.identity(left)))
        if not args.reset_trust:
            for guest, peer in targets:
                assert peer_trust(guest.state(), peer) in (0, 1), "Selected peer requires explicit reset"
        assert args.cycles == 1 or args.reset_trust
        transactions = set()
        for cycle in range(args.cycles):
            if args.reset_trust:
                with ThreadPoolExecutor(max_workers=2) as workers:
                    list(workers.map(lambda item: reset_peer(*item),
                         targets))
            preserved = [other_trust(guest.state(), peer) for guest, peer in targets]
            DISTRIBUTION.pair(left, right)
            values = [guest.state() for guest in (left, right)]
            transaction = tuple(values[0][44:48])
            assert transaction not in transactions and transaction == tuple(values[1][44:48])
            transactions.add(transaction)
            for index, (guest, peer) in enumerate(targets):
                state = DISTRIBUTION.select(guest, peer)
                assert peer_trust(state, peer) == 3 and not state[22]
                assert other_trust(state, peer) == preserved[index]
            report["cycles"].append({"cycle": cycle + 1, "trusted_both": True,
                                    "transaction": list(transaction),
                                    "clocks": [state[10] for state in values]})
            print(json.dumps(report["cycles"][-1]), flush=True)
        report["paired"] = True
    finally:
        report["observations"] = []
        for guest in guests:
            if guest.process is not None and guest.process.poll() is None:
                state = guest.state()
                report["observations"].append({"node": guest.number, "clock": state[10],
                    "trusted": state[25], "sessions": state[26], "wire_state": state[56],
                    "approvals": state[57], "expiry": state[58], "trust_error": state[60],
                    "transport_error": state[61], "ended": state[62], "site": state[63],
                    "poll_calls": state[68], "serviced_links": state[69], "received_packets": state[70]})
                guest.screenshot("focused-pairing-trace")
            guest.stop()
        hub.close()
        report["packets"] = hub.metadata
        report["metadata_dropped"] = hub.metadata_dropped
        name = f"pairing-trace-{pair[0]}-{pair[1]}-{time.time_ns()}.json"
        (work / name).write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
