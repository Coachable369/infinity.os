"""One bounded installed A/B ceremony on four preserved independently installed disks."""
import argparse
import importlib.util
import json
import pathlib
import struct
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
# FUNC: reset_peer
# DESC: Revokes existing authority and explicitly unblocks to untrusted through normal operator controls before a fresh ceremony.
# ------------------=
def reset_peer(guest, peer):
    guest.fast_commands = True
    guest.launch("command", 5)
    for command, expected in (("trust-revoke", 5), ("unblock", 1)):
        before = guest.state()[20]
        guest.command(f"node {command} node:{peer}")
        guest.wait(lambda state: state[20] > before and not state[22]
                   and peer_trust(state, peer) == expected and state[26] == 0,
                   "explicit peer authority reset", timeout=30)
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
    args = parser.parse_args()
    work = args.output.resolve()
    receipt = json.loads((work / "result.json").read_text())
    assert receipt["independent_installs"] == 4
    hub = EthernetHub(metadata_limit=8192).start()
    guests = []
    report = {"boundary": "installed pairing trace only", "paired": False,
              "cycles": [], "full_ms10_acceptance": False}
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
            if not args.reset_trust:
                assert state[25] == 0
        assert args.cycles == 1 or args.reset_trust
        transactions = set()
        for cycle in range(args.cycles):
            if args.reset_trust:
                with ThreadPoolExecutor(max_workers=2) as workers:
                    list(workers.map(lambda item: reset_peer(*item),
                         ((guests[0], DISTRIBUTION.identity(guests[1])),
                          (guests[1], DISTRIBUTION.identity(guests[0])))))
            DISTRIBUTION.pair(guests[0], guests[1])
            values = [guest.state() for guest in guests[:2]]
            transaction = tuple(values[0][44:48])
            assert transaction not in transactions and transaction == tuple(values[1][44:48])
            transactions.add(transaction)
            for guest, peer in ((guests[0], DISTRIBUTION.identity(guests[1])),
                                (guests[1], DISTRIBUTION.identity(guests[0]))):
                state = DISTRIBUTION.select(guest, peer)
                assert peer_trust(state, peer) == 3 and not state[22]
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
        name = "repeated-pairing-trace.json" if args.reset_trust else "focused-pairing-trace.json"
        (work / name).write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
