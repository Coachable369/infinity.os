"""Decode guest binary diagnostics, not logs or simulated capture."""
import json
import struct
import sys
from pathlib import Path

# ------------------------=
# FUNC: main
# DESC: Reports guest USB outcome and measured call durations without claiming installed acceptance.
# ------------------=
def main():
    path = Path(sys.argv[1])
    state, size, elapsed, longest, calls, payload = struct.unpack("<6Q", path.read_bytes())
    states = ["idle", "running", "frame", "cancelled", "device_error", "timeout", "protocol_error"]
    assert state < len(states)
    assert size <= 1228800 and payload <= 65536
    report = {"state": states[state], "frame_bytes": size, "elapsed_ms": elapsed,
              "longest_bulk_call_ms": longest, "bulk_calls": calls,
              "negotiated_payload_bytes": payload, "requested_bulk_timeout_ms": 10,
              "isolated_delivery_verified": False, "installed_system_verified": False}
    path.with_suffix(".json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))

if __name__ == "__main__":
    main()
