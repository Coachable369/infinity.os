"""Installed three/four-node Ethernet discovery gate, not full Pool acceptance."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
import pathlib
import struct
from ms10_ethernet_hub import EthernetHub

SPEC = importlib.util.spec_from_file_location("installed", pathlib.Path(__file__).with_name("ms9-installed-acceptance.py"))
INSTALLED = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INSTALLED)


# ------------------------=
# FUNC: configure
# DESC: Configures independent static addressing and pair-specific native link slots through ordinary settings and Console input.
# ------------------=
def configure(guest, count):
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
        guest.command(f"node link-configure {slot} local=10.42.0.{guest.number} remote=10.42.0.{peer} local-port={49152 + peer} remote-port={49152 + guest.number}")
        guest.wait(lambda state: state[20] > before and state[29] == slot, "native endpoint committed")


# ------------------------=
# FUNC: main
# DESC: Cold-boots independently installed disks on a bounded Ethernet fixture and verifies all-peer discovery without granting trust.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=pathlib.Path)
    parser.add_argument("--nodes", type=int, choices=(3, 4), default=3)
    parser.add_argument("--firmware", default="/opt/homebrew/share/qemu/edk2-x86_64-code.fd")
    args = parser.parse_args()
    work = args.output.resolve()
    receipt = json.loads((work / "result.json").read_text())
    assert receipt["independent_installs"] >= args.nodes
    identities = [entry["node_id"] for entry in receipt["nodes"][:args.nodes]]
    assert len(set(identities)) == args.nodes
    guests = []
    hub = EthernetHub(args.nodes).start()
    try:
        for number in range(1, args.nodes + 1):
            guest = INSTALLED.Guest(work, number, args.firmware, reuse=True)
            guests.append(guest)
            guest.mesh_port, guest.mesh_connect = hub.port, True
            guest.boot(False)
        with ThreadPoolExecutor(max_workers=args.nodes) as workers:
            authenticated = list(workers.map(lambda guest: guest.authenticate(), guests))
            for index, state in enumerate(authenticated):
                assert struct.pack("<4Q", *state[16:20]).hex() == identities[index]
            list(workers.map(lambda guest: configure(guest, args.nodes), guests))
        for guest in guests:
            state = guest.wait(lambda state: state[24] == args.nodes - 1 and state[22] == 0,
                               "all independently installed peers discovered", timeout=120)
            discovered = {struct.pack("<4Q", *state[128 + index * 16:132 + index * 16]).hex()
                          for index in range(args.nodes - 1)}
            assert discovered == set(identities) - {identities[guest.number - 1]}
            assert all(error == 0 for error in state[50:54])
            assert state[109] == 0  # Discovery does not fabricate resource advertisements.
            guest.screenshot("all-peer-discovery")
        (work / "mesh-discovery-result.json").write_text(json.dumps({
            "boundary": "installed QEMU Ethernet and native node discovery",
            "nodes": identities, "iso_detached": True, "all_peer_discovery": True,
            "trust_acceptance": False, "distributed_pool_acceptance": False}, indent=2))
    finally:
        for guest in guests:
            guest.stop()
        hub.close()


if __name__ == "__main__":
    main()
