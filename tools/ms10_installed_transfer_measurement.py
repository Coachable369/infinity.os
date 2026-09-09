"""Bounded installed transfer measurements, not a substitute for byte verification."""
import struct
import time
import ms10_installed_fixture as fixture


# ------------------------=
# FUNC: measure
# DESC: Samples actual coordinator offsets and receipt-backed protection; captures bounded failure evidence rather than widening a stalled deadline.
# ------------------=
def measure(guest, symbol, object_id, timeout=600, clock=time.monotonic, pause=time.sleep):
    guest.launch("storage", 8, 8)
    started = clock()
    observations = []
    last_window = None
    windows = []
    while clock() - started < timeout:
        now = clock()
        pool = fixture.read_state(guest, symbol)
        assert not pool[25], {"projection_failed": pool[25]}
        active = struct.pack("<2Q", *pool[240:242]).hex()
        destination = struct.pack("<4Q", *pool[242:246]).hex()
        sample = {"elapsed_seconds": now - started, "phase": pool[26],
                  "offset": pool[27], "length": pool[28], "destination": destination,
                  "request": pool[29], "active_object": active}
        if active == object_id and pool[26]:
            if last_window and destination == last_window[0] and pool[27] > last_window[1]:
                windows.append({"bytes": pool[27] - last_window[1],
                                "seconds": now - last_window[2]})
            if last_window is None or destination != last_window[0] or pool[27] != last_window[1]:
                last_window = (destination, pool[27], now)
        observations.append(sample)
        for index in range(min(pool[18], 8)):
            row = pool[32 + index * 16:48 + index * 16]
            if struct.pack("<2Q", *row[:2]).hex() == object_id and row[5] == row[6] == 3:
                return {"status": "TESTED_PROTECTION_RECEIPTS", "elapsed_seconds": clock() - started,
                        "object": object_id, "bytes": row[4], "row": list(row),
                        "windows": windows, "samples": observations,
                        "full_persisted_byte_verification": False,
                        "interactive_performance_claim": False}
        pause(1)
    raise AssertionError({"transfer_measurement_deadline": object_id, "deadline_seconds": timeout,
                          "windows": windows, "samples": observations})
