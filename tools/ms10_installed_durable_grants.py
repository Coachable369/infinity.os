"""Explicit until-revoked Pool approval through native Console completion."""
import struct
import time

OPERATIONS = {"resource-advertise": 0xe001, "replica-inspect": 0xe020,
              "replica-delete": 0xe024, "transfer-begin": 0xe021,
              "transfer-chunk": 0xe022, "transfer-commit": 0xe023,
              "object-read": 0x3002, "pool-metadata": 0xe050}


# ------------------------=
# FUNC: decode_approval
# DESC: Requires a fresh successful exact-peer native durable approval, not an ephemeral maximum-ID observation.
# ------------------=
def decode_approval(state, previous, peer, operation):
    assert state[3] != 0 and state[3] != previous
    assert state[4] == 1, {"request": state[3], "status": state[4]}
    payload = struct.pack("<10Q", *state[5:15])
    handle, scope, deadline, opcode, rights, value, flags, schema = struct.unpack_from("<QQQIIIIH", payload, 32)
    assert payload[:32] == bytes.fromhex(peer)
    assert opcode == 0xd022 and value == OPERATIONS[operation]
    assert rights == 1 and scope == 0 and flags == 3
    assert schema == 1 and payload[74:] == bytes(6)
    assert deadline == 0 and handle & (1 << 63)
    return handle


# ------------------------=
# FUNC: grant
# DESC: Submits one explicitly confirmed durable Pool operation and verifies its exact typed response without reissuing it.
# ------------------=
def grant(guest, peer, operation, read=None, clock=time.monotonic, pause=time.sleep):
    assert operation in OPERATIONS and len(bytes.fromhex(peer)) == 32
    read = read or (lambda: local_completion(guest))
    previous = read()[3]
    guest.command(f"node capability-grant node:{peer} name={operation} durable=true confirm=true")
    deadline = clock() + 120
    while True:
        result = read()
        if result[3] and result[3] != previous:
            break
        assert clock() < deadline, {"local_approval_not_completed": previous}
        pause(.1)
    return decode_approval(result, previous, peer, operation)


# ------------------------=
# FUNC: local_completion
# DESC: Reads the independent local broker snapshot; remote operator completion is never reused for local approval evidence.
# ------------------=
def local_completion(guest):
    if not hasattr(guest, "node_local_symbol"):
        resolve = guest.state.__func__.__globals__["symbol"]
        guest.node_local_symbol = resolve(guest.work.parent / "artifacts/installed-kernel.elf",
                                          "INFINITY_NODE_LOCAL_DIAGNOSTIC_SNAPSHOT")
    address, size = guest.node_local_symbol
    assert size == 256
    guest.qmp("stop")
    try:
        result = struct.unpack("<32Q", guest.memory(address, size))
    finally:
        guest.qmp("cont")
    if not any(result):
        return result
    assert result[0] == 0x494e464c4f434c31 and result[1] == 1
    assert result[2] == result[31] and result[2] % 2 == 0
    return result
