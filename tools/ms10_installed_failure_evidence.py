"""Bounded read-only failure evidence, excluding credentials and payload bytes."""
import json
import struct
import time
import ms10_installed_fixture as fixture


# ------------------------=
# FUNC: safe_state
# DESC: Projects public request progress and numeric transport state without copying input buffers, keys, or payload data.
# ------------------=
def safe_state(main, pool):
    return {"clock": main[10], "mode": main[4], "sessions": main[26],
            "selected_peer": struct.pack("<4Q", *main[32:36]).hex(),
            "wire_stage": main[56], "wire_expiry": main[58], "wire_seen": main[59],
            "trust_error": main[60], "transport_error": main[61],
            "rx_packets": main[70], "online_resources": main[496],
            "pool_generation": pool[2], "fixture_phase": pool[3],
            "fixture_error": pool[31], "coordinator_error": pool[15],
            "phase": pool[26], "offset": pool[27], "length": pool[28], "request": pool[29],
            "object": struct.pack("<2Q", *pool[240:242]).hex(),
            "destination": struct.pack("<4Q", *pool[242:246]).hex(),
            "ready": pool[24], "projection_error": pool[25]}


# ------------------------=
# FUNC: capture
# DESC: Captures one coherent Pool snapshot and a coherent main snapshot using only the existing guest controller.
# ------------------=
def capture(guest, symbol):
    pool = fixture.read_state(guest, symbol)
    main = guest.state()
    assert main is not None, "Main diagnostic generation unavailable"
    return safe_state(main, pool)


# ------------------------=
# FUNC: trace_action
# DESC: Persists a single bounded public observation before an action can fail; no background polling or second controller.
# ------------------=
def trace_action(guest, symbol, action):
    entry = {"action": action, "host_monotonic_ns": time.monotonic_ns()}
    try:
        entry.update(capture(guest, symbol))
    except Exception as error:
        entry["capture_error"] = repr(error)
    with (guest.work / "transfer-action-trace.jsonl").open("a") as output:
        output.write(json.dumps(entry) + "\n")


# ------------------------=
# FUNC: capture_final
# DESC: Retains final hub and at most four guest observations before shutdown, treating capture failures as missing evidence.
# ------------------=
def capture_final(guests, hub, symbol):
    result = {"hub": hub.traffic_snapshot(), "guests": []}
    for guest in guests[:4]:
        entry = {"node": guest.number}
        try:
            if guest.process is None or guest.process.poll() is not None:
                entry["stopped"] = True
            else:
                entry.update(capture(guest, symbol))
        except Exception as error:
            entry["capture_error"] = repr(error)
        result["guests"].append(entry)
    return result
