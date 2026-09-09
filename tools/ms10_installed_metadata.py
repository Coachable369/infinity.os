"""Correlated installed metadata operations through ordinary Console input."""
import struct
import time


# ------------------------=
# FUNC: invoke
# DESC: Submits once, then collects only that admitted request; stale observations cannot satisfy completion.
# ------------------=
def invoke(guest, command, projection, timeout=180, clock=time.monotonic, allow_unadmitted=False):
    before = projection()[252]
    guest.command(command)
    admitted = projection()
    request = admitted[252]
    if request == before and allow_unadmitted:
        return None
    assert request != before and request >> 62 == 3, {"not_admitted": request, "previous": before}
    deadline = clock() + timeout
    collections = 0
    while clock() < deadline:
        state = guest.command(f"pool result request={request}")
        collections += 1
        observed = projection()
        if observed[253] != request:
            continue
        assert observed[254] == 0, {"metadata_error": observed[254], "request": request}
        assert state[110] == 1, {"missing_consumed_payload": request}
        raw = struct.pack("<17Q", *state[111:128])
        schema, length, operation = struct.unpack_from("<HHI", raw)
        assert schema == 1 and length <= 64
        return {"request": request, "collections": collections, "operation": operation,
                "object": raw[8:24].hex(), "generation": struct.unpack_from("<Q", raw, 32)[0],
                "version": struct.unpack_from("<Q", raw, 40)[0],
                "value": struct.unpack_from("<Q", raw, 64)[0], "data": raw[72:72+length]}
    raise AssertionError({"metadata_deadline": request, "collections": collections})


# ------------------------=
# FUNC: read_path
# DESC: Uses the normal shared namespace without selecting a physical replica and compares returned bytes.
# ------------------=
def read_path(guest, path, object_id, expected, projection, offset=0, timeout=180):
    result = invoke(guest, f"pool read {path} offset={offset} length={len(expected)}", projection, timeout)
    assert result["operation"] == 0x3002 and result["object"] == object_id
    assert result["data"] == expected
    return result


# ------------------------=
# FUNC: read_path_ready
# DESC: Polls only an eventual read-only namespace binding, records every non-admission, and accepts solely correlated completed bytes.
# ------------------=
def read_path_ready(guest, path, object_id, expected, projection, timeout=180, clock=time.monotonic):
    started = clock()
    attempts = []
    while clock() - started < timeout:
        remaining = timeout - (clock() - started)
        result = invoke(guest, f"pool read {path} offset=0 length={len(expected)}", projection,
                        remaining, clock, allow_unadmitted=True)
        attempts.append({"elapsed_seconds": clock() - started, "admitted": result is not None})
        if result is None:
            continue
        assert result["operation"] == 0x3002 and result["object"] == object_id
        assert result["data"] == expected
        result["readiness_attempts"] = attempts
        return result
    raise AssertionError({"namespace_readiness_deadline": path, "attempts": attempts})
