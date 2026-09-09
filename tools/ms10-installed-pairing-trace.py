"""One bounded installed A/B ceremony on four preserved independently installed disks."""
import argparse
import importlib.util
import json
import pathlib
from concurrent.futures import ThreadPoolExecutor
from ms10_ethernet_hub import EthernetHub

SPEC = importlib.util.spec_from_file_location("distribution", pathlib.Path(__file__).with_name("ms10-installed-distribution.py"))
DISTRIBUTION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DISTRIBUTION)


# ------------------------=
# FUNC: main
# DESC: Reproduces only discovery and protected pairing, retaining bounded public network metadata and native counters without installing, granting or storing Pool data.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    work = args.output.resolve()
    receipt = json.loads((work / "result.json").read_text())
    assert receipt["independent_installs"] == 4
    hub = EthernetHub(metadata_limit=8192).start()
    guests = []
    report = {"boundary": "installed pairing trace only", "paired": False, "full_ms10_acceptance": False}
    try:
        for number in range(1, 5):
            guest = DISTRIBUTION.API.Guest(work, number, "/opt/homebrew/share/qemu/edk2-x86_64-code.fd", reuse=True)
            guests.append(guest)
            guest.mesh_port, guest.mesh_connect = hub.port, True
            guest.boot(False)
        with ThreadPoolExecutor(max_workers=4) as workers:
            list(workers.map(lambda guest: guest.authenticate(), guests))
        for guest, installed in zip(guests, receipt["nodes"]):
            assert DISTRIBUTION.identity(guest) == installed["node_id"]
            state = guest.wait(lambda state: state[24] == 3 and state[29] == 3, "preserved endpoints discover three peers", timeout=90)
            assert state[25] == 0
        DISTRIBUTION.pair(guests[0], guests[1])
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
        (work / "focused-pairing-trace.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
