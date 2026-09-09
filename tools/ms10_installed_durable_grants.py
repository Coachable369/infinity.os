"""Explicit until-revoked Pool approval through native Console completion."""
import struct

OPERATIONS = {"resource-advertise": 0xe001, "replica-inspect": 0xe020,
              "replica-delete": 0xe024, "transfer-begin": 0xe021,
              "transfer-chunk": 0xe022, "transfer-commit": 0xe023,
              "object-read": 0x3002, "pool-metadata": 0xe050}


# ------------------------=
# FUNC: decode_approval
# DESC: Requires a fresh successful exact-peer native durable approval, not an ephemeral maximum-ID observation.
# ------------------=
def decode_approval(state, previous, peer, operation):
    assert state[73] != 0 and state[73] != previous
    assert state[76] == 1, {"request": state[73], "status": state[76]}
    assert state[74] != 0 and state[75] != 0
    payload = struct.pack("<10Q", *state[77:87])
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
def grant(guest, peer, operation):
    assert operation in OPERATIONS and len(bytes.fromhex(peer)) == 32
    previous = guest.state()[73]
    guest.command(f"node capability-grant node:{peer} name={operation} durable=true confirm=true")
    result = guest.wait(lambda state: state[73] != 0 and state[73] != previous,
                        "durable Pool approval completed", timeout=120)
    return decode_approval(result, previous, peer, operation)
